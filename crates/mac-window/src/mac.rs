//! The calls into macOS. Compiled for macOS only, and run by no test.

use objc2::rc::Retained;
use objc2::{sel, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSView, NSWindow, NSWindowButton, NSWindowStyleMask};
use objc2_foundation::{ns_string, NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{is_off, placement};

/// The window's own buttons, from the left.
const BUTTONS: [NSWindowButton; 3] = [
    NSWindowButton::CloseButton,
    NSWindowButton::MiniaturizeButton,
    NSWindowButton::ZoomButton,
];

/// The window behind `handle` and the view that holds its title bar, for
/// a window that has one. `_main` is the proof that this is the main
/// thread.
#[expect(
    unsafe_code,
    reason = "the one audited exception in GATES.md: macOS hands out the view behind a raw window handle, and the view above another, only through unsafe calls"
)]
fn title_bar(
    handle: &impl HasWindowHandle,
    _main: MainThreadMarker,
) -> Option<(Retained<NSWindow>, Retained<NSView>)> {
    let handle = handle.window_handle().ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: `handle` borrows the window, and raw-window-handle's contract
    // for a borrowed handle is that `ns_view` points at a live `NSView` for
    // as long as the borrow lasts, which is longer than this reference
    // does. An `NSView` may only be touched on the main thread, which
    // `_main` proves this is.
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    let window = view.window()?;
    let close = window.standardWindowButton(NSWindowButton::CloseButton)?;
    // SAFETY: `superview` returns a view that its subview does not keep
    // alive. Both views asked here belong to `window`, which is retained
    // above and is changed only on this thread, so neither can go away
    // during the two calls; `Retained` then keeps the one returned alive
    // on its own.
    let container = unsafe { close.superview()?.superview()? };
    Some((window, container))
}

/// Put the close, minimize and zoom buttons of `window` on the centre
/// line of a row `row_height` points tall at its top edge.
///
/// The title bar is made as tall as the row and the buttons are put in
/// its middle. macOS lays them out again when the window is resized,
/// leaves full screen or changes its title bar, and nothing tells the app
/// that it did. So this is for calling every frame: it reads where the
/// title bar and the buttons are and moves only what is not where it
/// belongs. A window in full screen, whose buttons are hidden, is left
/// alone.
///
/// Does nothing off the main thread, and for a window that is not one of
/// this system's.
pub fn center_window_buttons(window: &impl HasWindowHandle, row_height: f64) {
    let Some(main) = MainThreadMarker::new() else {
        return;
    };
    let Some((window, container)) = title_bar(window, main) else {
        return;
    };
    // In full screen the buttons live in another window, which slides in
    // with the menu bar.
    if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return;
    }
    let buttons = BUTTONS.map(|button| window.standardWindowButton(button));
    let mut heights = buttons.iter().flatten().map(|b| b.frame().size.height);
    let Some(button_height) = heights.next() else {
        return;
    };
    let wanted = placement(window.frame().size.height, row_height, button_height);

    let bar = container.frame();
    if is_off(bar.origin.y, wanted.bar_bottom) || is_off(bar.size.height, wanted.bar_height) {
        container.setFrame(NSRect::new(
            NSPoint::new(bar.origin.x, wanted.bar_bottom),
            NSSize::new(bar.size.width, wanted.bar_height),
        ));
    }
    for button in buttons.iter().flatten() {
        let origin = button.frame().origin;
        if is_off(origin.y, wanted.button_bottom) {
            button.setFrameOrigin(NSPoint::new(origin.x, wanted.button_bottom));
        }
    }
}

/// Take ⌘Q off the menu bar's Quit item, so the key press reaches the
/// window and the app answers it as it answers its other shortcuts.
///
/// The item quits at once, without the window being asked to close, so an
/// app that asks about unsaved edits when its window closes never gets
/// to. The item itself stays in the menu and still quits at once.
///
/// Does nothing off the main thread.
pub fn hand_quit_key_to_app() {
    let Some(main) = MainThreadMarker::new() else {
        return;
    };
    let Some(menu_bar) = NSApplication::sharedApplication(main).mainMenu() else {
        return;
    };
    let menus = menu_bar.itemArray();
    for menu in menus.iter().filter_map(|title| title.submenu()) {
        for item in menu.itemArray() {
            if item.action() == Some(sel!(terminate:)) {
                item.setKeyEquivalent(ns_string!(""));
            }
        }
    }
}
