//! The calls into macOS. Compiled for macOS only, and run by no test.

use std::ffi::CStr;

use objc2::ffi::class_addMethod;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use objc2::{sel, AllocAnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSApplication, NSApplicationTerminateReply, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage,
    NSView, NSWindow, NSWindowButton, NSWindowStyleMask,
};
use objc2_foundation::{ns_string, NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{is_off, pixel_rows, placement, quit_answer, QuitAnswer};

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
    reason = "the first audited exception in GATES.md: macOS hands out the view behind a raw window handle, and the view above another, only through unsafe calls"
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

/// What `applicationShouldTerminate:` takes and returns, as the
/// Objective-C runtime writes it: an unsigned 64-bit integer (the reply)
/// from an object (the delegate), a selector and an object (the
/// application). Every Mac this builds for is 64-bit.
const SHOULD_TERMINATE_TYPES: &CStr = c"Q@:@";

/// `applicationShouldTerminate:`, as the delegate of the application
/// answers it: with what the app says at this moment.
///
/// macOS calls this on the main thread from `terminate:`, which Quit in
/// the menu bar and in the Dock, a log-out, a restart and a shut-down all
/// go through. None of the three arguments is read.
extern "C-unwind" fn should_terminate(
    _delegate: *mut AnyObject,
    _selector: Sel,
    _application: *mut AnyObject,
) -> NSApplicationTerminateReply {
    match quit_answer() {
        QuitAnswer::Now => NSApplicationTerminateReply::TerminateNow,
        QuitAnswer::NotYet => NSApplicationTerminateReply::TerminateCancel,
    }
}

/// Give `class`, the class of the application's delegate, the method
/// `applicationShouldTerminate:`, unless it has one of its own. Returns
/// whether the method was added.
#[expect(
    unsafe_code,
    reason = "the second audited exception in GATES.md: a method is added to a class that exists only through the Objective-C runtime's own, unsafe, function"
)]
fn add_should_terminate(class: &'static AnyClass, _main: MainThreadMarker) -> bool {
    let method: extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> _ = should_terminate;
    let class = std::ptr::from_ref(class).cast_mut();
    let selector = sel!(applicationShouldTerminate:);
    let types = SHOULD_TERMINATE_TYPES.as_ptr();
    // SAFETY: the block does two unsafe things, a transmute and a call.
    //
    // The transmute only forgets the signature of `should_terminate`:
    // `Imp` is the runtime's name for "some function with the C calling
    // convention", both are plain function pointers of one size, and the
    // runtime never calls an `Imp` as written. It calls it with the
    // arguments of the method it is registered as, which
    // `SHOULD_TERMINATE_TYPES` and the selector above state and which
    // are exactly `should_terminate`'s: the receiver, the selector and
    // one object in, one `NSUInteger` out, which
    // `NSApplicationTerminateReply` is a transparent wrapper of. The
    // arguments are taken as raw pointers and never dereferenced, so
    // nothing is assumed of what they point at.
    //
    // `class_addMethod` wants a registered class, a selector, a function
    // that outlives the class and a NUL-terminated type string it may
    // read during the call. `class` is a `&'static AnyClass`, which only
    // the runtime hands out, for a class it has registered; a class may
    // be given a method while its instances are live, the runtime taking
    // its own lock to do so. The function is a `fn` item and the string a
    // `&'static CStr`, both of which live as long as the process. The
    // call replaces nothing: where the class already has the method it
    // leaves it and returns `NO`. `_main` proves this is the main thread,
    // the only one that sends `applicationShouldTerminate:`, so no call
    // of the method can be in flight while it is added.
    let added = unsafe {
        let method = std::mem::transmute::<
            extern "C-unwind" fn(
                *mut AnyObject,
                Sel,
                *mut AnyObject,
            ) -> NSApplicationTerminateReply,
            Imp,
        >(method);
        class_addMethod(class, selector, method, types)
    };
    added.as_bool()
}

/// Have the application's delegate answer `applicationShouldTerminate:`
/// with [`quit_answer`].
///
/// The delegate is winit's, which has no such method, so macOS takes the
/// answer to be yes. The method is added to its class. The delegate is
/// then taken off the application and put back: macOS is free to note
/// which questions a delegate answers at the moment it is given one, and
/// this delegate was given before it could answer this one.
///
/// Does nothing off the main thread, for an application without a
/// delegate, and for a delegate that already answers the question.
pub(crate) fn answer_quit_requests() {
    let Some(main) = MainThreadMarker::new() else {
        return;
    };
    let application = NSApplication::sharedApplication(main);
    let Some(delegate) = application.delegate() else {
        return;
    };
    let object: &AnyObject = (*delegate).as_ref();
    if add_should_terminate(object.class(), main) {
        application.setDelegate(None);
        application.setDelegate(Some(&delegate));
    }
}

/// Show the picture `rgba`, `width` pixels across and `height` down, four
/// bytes to a pixel, as the app's icon in the Dock and the app switcher.
///
/// On a Mac the Dock's icon is not the window's, and winit leaves it as
/// it was at launch. This changes it at once, for as long as the app runs:
/// the next launch starts from the icon the window is built with. macOS
/// keeps a copy of the pixels, so `rgba` may go once this returns.
///
/// Does nothing off the main thread, and for bytes that are not such a
/// picture.
#[expect(
    unsafe_code,
    reason = "the third audited exception in GATES.md: AppKit takes a picture's pixels, and a new icon for the Dock, only through unsafe calls"
)]
pub fn set_dock_icon(rgba: &[u8], width: u32, height: u32) {
    let Some(main) = MainThreadMarker::new() else {
        return;
    };
    let Some(rows) = pixel_rows(rgba, width, height) else {
        return;
    };
    let application = NSApplication::sharedApplication(main);
    let size = NSSize::new(f64::from(width), f64::from(height));
    let image = NSImage::initWithSize(NSImage::alloc(), size);
    // SAFETY: the block does four unsafe things, and nothing else in it is
    // unsafe.
    //
    // Reading `NSDeviceRGBColorSpace`: an extern static, a constant
    // `NSString` that AppKit defines and never changes.
    //
    // Making the bitmap: `planes` must be a valid pointer or null, and it
    // is null. AppKit then allocates the bitmap's memory itself and owns
    // it; it neither reads nor keeps a pointer of ours. The other
    // arguments describe one plane of 8-bit red, green, blue and alpha,
    // which is how eframe builds the icon it gives the Dock at launch.
    //
    // Copying the pixels: the copy writes `rows.bytes` bytes to `pixels`
    // and reads as many from `rgba`. `pixel_rows` gave `rows` only
    // because `rgba` holds exactly `rows.bytes` bytes. `pixels` is not
    // null, and `rows.fit` has checked that the bitmap's one plane holds
    // exactly `rows.bytes` bytes in rows as long as ours, so the write
    // stays inside the memory AppKit allocated for this bitmap, which
    // `bitmap` keeps alive and which no other pointer reaches. That memory
    // is new, so it cannot overlap `rgba`, which the caller holds.
    //
    // Setting the icon: `setApplicationIconImage` may not take `None`, and
    // it is given `Some`. It is sent on the main thread, which `main`
    // proves this is; the app calls this from inside a frame, which
    // eframe runs on the main thread on a Mac. The image retains the
    // bitmap and the application retains the image, so neither goes when
    // the `Retained` values here are dropped.
    unsafe {
        let Some(bitmap) = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            rows.width,
            rows.height,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            rows.bytes_per_row,
            32,
        ) else {
            return;
        };
        let pixels = bitmap.bitmapData();
        if pixels.is_null() || !rows.fit(bitmap.bytesPerRow(), bitmap.bytesPerPlane()) {
            return;
        }
        std::ptr::copy_nonoverlapping(rgba.as_ptr(), pixels, rows.bytes);
        image.addRepresentation(&bitmap);
        application.setApplicationIconImage(Some(&image));
    }
}
