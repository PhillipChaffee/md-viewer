//! Kani proofs for the shortcut table: which bindings are live under each
//! focus and each set of modifier keys, on a Mac and off one. `PROOFS.md`
//! lists each harness with its bounds.
//!
//! The harnesses read the real `BINDINGS` table, every row of it, under
//! every focus, all 32 combinations of the five modifier flags, and both
//! platforms. What
//! they do not run is `shortcut_actions`, which reads and consumes egui's input state:
//! `tests.rs` covers it.

use eframe::egui;

use super::{Chord, Focus, Scope, BINDINGS};
use crate::platform::Platform;

fn any_focus() -> Focus {
    match kani::any::<u8>() % 5 {
        0 => Focus::Prompt,
        1 => Focus::Lightbox,
        2 => Focus::Popup,
        3 => Focus::FindBar,
        _ => Focus::Reading,
    }
}

fn any_modifiers() -> egui::Modifiers {
    egui::Modifiers {
        alt: kani::any(),
        ctrl: kani::any(),
        shift: kani::any(),
        mac_cmd: kani::any(),
        command: kani::any(),
    }
}

fn any_platform() -> Platform {
    if kani::any() {
        Platform::Mac
    } else {
        Platform::Other
    }
}

/// The keys held when a key goes down, and the machine they are held on.
/// Every set of flags is tried on both platforms, the sets a real
/// keyboard of that platform never reports among them.
#[derive(Clone, Copy)]
struct Held {
    modifiers: egui::Modifiers,
    platform: Platform,
}

fn any_held() -> Held {
    Held {
        modifiers: any_modifiers(),
        platform: any_platform(),
    }
}

/// Any row of the table.
fn any_binding() -> usize {
    let row: usize = kani::any();
    kani::assume(row < BINDINGS.len());
    row
}

/// Whether the binding in `row` fires for its key under `focus` with
/// `held`: the test `shortcut_actions` applies to each row.
fn is_live(row: usize, focus: Focus, held: Held) -> bool {
    let (chord, _, scope, _) = BINDINGS[row];
    scope.covers(focus) && chord.is_held(held.modifiers, held.platform)
}

/// One key press runs at most one binding: no two rows of the table share
/// a key and are live under the same focus with the same modifiers held.
#[kani::proof]
fn no_key_press_fires_two_bindings() {
    let (one, other) = (any_binding(), any_binding());
    kani::assume(one != other);
    let (focus, held) = (any_focus(), any_held());

    if BINDINGS[one].1 == BINDINGS[other].1 {
        assert!(!(is_live(one, focus, held) && is_live(other, focus, held)));
    }

    kani::cover!(BINDINGS[one].1 == BINDINGS[other].1 && is_live(one, focus, held));
    kani::cover!(one == BINDINGS.len() - 1 && other == 0);
}

/// While the lightbox is open, only the lightbox's own bindings are live;
/// with it closed, none of them is.
#[kani::proof]
fn the_lightbox_takes_the_keyboard() {
    let row = any_binding();
    let (focus, held) = (any_focus(), any_held());
    let for_lightbox = matches!(BINDINGS[row].2, Scope::Lightbox);

    if is_live(row, focus, held) {
        assert!(for_lightbox == matches!(focus, Focus::Lightbox));
    }

    kani::cover!(is_live(row, focus, held) && for_lightbox);
    kani::cover!(is_live(row, focus, held) && !for_lightbox);
}

/// While the prompt about unsaved edits asks, no binding is live: a key
/// answers the prompt or does nothing.
#[kani::proof]
fn the_prompt_takes_every_key() {
    let row = any_binding();
    let (focus, held) = (any_focus(), any_held());

    if matches!(focus, Focus::Prompt) {
        assert!(!is_live(row, focus, held));
    }

    kani::cover!(matches!(focus, Focus::Prompt));
    kani::cover!(is_live(row, focus, held));
}

/// An open menu or popover takes the keys the find bar and the reading
/// view would: under it no find-bar binding and no reading binding is
/// live, so Escape closes the menu and leaves the find bar open.
#[kani::proof]
fn a_popup_takes_the_find_bar_and_reading_keys() {
    let row = any_binding();
    let held = any_held();

    if is_live(row, Focus::Popup, held) {
        assert!(matches!(BINDINGS[row].2, Scope::Document));
        assert!(BINDINGS[row].1 != egui::Key::Escape);
    }
    // The find bar's keys and the plain arrows do not overlap either: a
    // binding live with the find bar open is not a reading-only one.
    if is_live(row, Focus::FindBar, held) {
        assert!(!matches!(BINDINGS[row].2, Scope::Reading | Scope::Lightbox));
    }

    kani::cover!(is_live(row, Focus::Popup, held));
    kani::cover!(is_live(row, Focus::FindBar, held) && BINDINGS[row].1 == egui::Key::Escape);
}

/// The command key is ⌘ on a Mac and Ctrl elsewhere: a `Cmd` binding
/// that is live has that key held, so Control with a letter is never a
/// command on a Mac, nor ⌘ one off it.
#[kani::proof]
fn the_command_key_is_cmd_on_a_mac_and_ctrl_elsewhere() {
    let row = any_binding();
    let (focus, held) = (any_focus(), any_held());
    let for_command = matches!(
        BINDINGS[row].0,
        Chord::Cmd | Chord::CmdNoShift | Chord::CmdShift | Chord::CmdAlt
    );

    if for_command && is_live(row, focus, held) {
        match held.platform {
            Platform::Mac => assert!(held.modifiers.mac_cmd),
            Platform::Other => assert!(held.modifiers.ctrl),
        }
    }

    kani::cover!(for_command && is_live(row, focus, held) && !held.modifiers.ctrl);
    kani::cover!(for_command && is_live(row, focus, held) && !held.modifiers.mac_cmd);
}

/// Every binding can fire, on each platform whose keyboard can hold its
/// chord: for each row there is a focus and a set of modifiers under which
/// it is live there. On a platform that cannot hold the chord the row
/// never fires, and no row is left without a platform.
#[kani::proof]
fn every_binding_can_fire() {
    let row = any_binding();
    let platform = any_platform();
    let (chord, _, scope, _) = BINDINGS[row];
    let focus = match scope {
        Scope::Lightbox => Focus::Lightbox,
        Scope::FindBar => Focus::FindBar,
        Scope::Document | Scope::Reading => Focus::Reading,
    };
    // What each platform's keyboard reports with the command key held.
    let mac = matches!(platform, Platform::Mac);
    let keys = |command: bool, ctrl: bool, alt: bool, shift: bool| egui::Modifiers {
        alt,
        ctrl: ctrl || (command && !mac),
        shift,
        mac_cmd: command && mac,
        command,
    };
    let modifiers = match chord {
        Chord::Any | Chord::Unmodified | Chord::NoShift => keys(false, false, false, false),
        Chord::Shift => keys(false, false, false, true),
        Chord::Cmd | Chord::CmdNoShift | Chord::MacCmd => keys(true, false, false, false),
        Chord::CmdShift => keys(true, false, false, true),
        Chord::CmdAlt => keys(true, false, true, false),
        Chord::CtrlNoShift => keys(false, true, false, false),
        Chord::CtrlShift => keys(false, true, false, true),
        Chord::AltNoCtrl => keys(false, false, true, false),
    };

    if chord.exists_on(platform) {
        assert!(is_live(
            row,
            focus,
            Held {
                modifiers,
                platform
            }
        ));
    } else {
        let modifiers = any_modifiers();
        assert!(!is_live(
            row,
            focus,
            Held {
                modifiers,
                platform
            }
        ));
    }
    assert!(chord.exists_on(Platform::Mac) || chord.exists_on(Platform::Other));

    kani::cover!(row == 0);
    kani::cover!(row == BINDINGS.len() - 1);
    kani::cover!(!chord.exists_on(Platform::Mac));
    kani::cover!(!chord.exists_on(Platform::Other));
}
