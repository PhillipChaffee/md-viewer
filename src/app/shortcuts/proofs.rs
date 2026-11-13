//! Kani proofs for the shortcut table: which bindings are live under each
//! focus and each set of modifier keys. `PROOFS.md` lists each harness
//! with its bounds.
//!
//! The harnesses read the real `BINDINGS` table, every row of it, under
//! every focus and all 32 combinations of the five modifier flags. What
//! they do not run is `shortcut_actions`, which reads and consumes egui's input state:
//! `tests.rs` covers it.

use eframe::egui;

use super::{Chord, Focus, Scope, BINDINGS};

fn any_focus() -> Focus {
    match kani::any::<u8>() % 4 {
        0 => Focus::Lightbox,
        1 => Focus::Popup,
        2 => Focus::FindBar,
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

/// Any row of the table.
fn any_binding() -> usize {
    let row: usize = kani::any();
    kani::assume(row < BINDINGS.len());
    row
}

/// Whether the binding in `row` fires for its key under `focus` with
/// `modifiers` held: the test `shortcut_actions` applies to each row.
fn is_live(row: usize, focus: Focus, modifiers: egui::Modifiers) -> bool {
    let (chord, _, scope, _) = BINDINGS[row];
    scope.covers(focus) && chord.is_held(modifiers)
}

/// One key press runs at most one binding: no two rows of the table share
/// a key and are live under the same focus with the same modifiers held.
#[kani::proof]
fn no_key_press_fires_two_bindings() {
    let (one, other) = (any_binding(), any_binding());
    kani::assume(one != other);
    let (focus, modifiers) = (any_focus(), any_modifiers());

    if BINDINGS[one].1 == BINDINGS[other].1 {
        assert!(!(is_live(one, focus, modifiers) && is_live(other, focus, modifiers)));
    }

    kani::cover!(BINDINGS[one].1 == BINDINGS[other].1 && is_live(one, focus, modifiers));
    kani::cover!(one == BINDINGS.len() - 1 && other == 0);
}

/// While the lightbox is open, only the lightbox's own bindings are live;
/// with it closed, none of them is.
#[kani::proof]
fn the_lightbox_takes_the_keyboard() {
    let row = any_binding();
    let (focus, modifiers) = (any_focus(), any_modifiers());
    let for_lightbox = matches!(BINDINGS[row].2, Scope::Lightbox);

    if is_live(row, focus, modifiers) {
        assert!(for_lightbox == matches!(focus, Focus::Lightbox));
    }

    kani::cover!(is_live(row, focus, modifiers) && for_lightbox);
    kani::cover!(is_live(row, focus, modifiers) && !for_lightbox);
}

/// An open menu or popover takes the keys the find bar and the reading
/// view would: under it no find-bar binding and no reading binding is
/// live, so Escape closes the menu and leaves the find bar open.
#[kani::proof]
fn a_popup_takes_the_find_bar_and_reading_keys() {
    let row = any_binding();
    let modifiers = any_modifiers();

    if is_live(row, Focus::Popup, modifiers) {
        assert!(matches!(BINDINGS[row].2, Scope::Document));
        assert!(BINDINGS[row].1 != egui::Key::Escape);
    }
    // The find bar's keys and the plain arrows do not overlap either: a
    // binding live with the find bar open is not a reading-only one.
    if is_live(row, Focus::FindBar, modifiers) {
        assert!(!matches!(BINDINGS[row].2, Scope::Reading | Scope::Lightbox));
    }

    kani::cover!(is_live(row, Focus::Popup, modifiers));
    kani::cover!(is_live(row, Focus::FindBar, modifiers) && BINDINGS[row].1 == egui::Key::Escape);
}

/// Every binding can fire: for each row there is a focus and a set of
/// modifiers under which it is live.
#[kani::proof]
fn every_binding_can_fire() {
    let row = any_binding();
    let (chord, _, scope, _) = BINDINGS[row];
    let focus = match scope {
        Scope::Lightbox => Focus::Lightbox,
        Scope::FindBar => Focus::FindBar,
        Scope::Document | Scope::Reading => Focus::Reading,
    };
    let held = |ctrl: bool, alt: bool, shift: bool| egui::Modifiers {
        alt,
        ctrl,
        shift,
        mac_cmd: false,
        command: ctrl,
    };
    let modifiers = match chord {
        Chord::Any | Chord::Unmodified | Chord::NoShift => held(false, false, false),
        Chord::Shift => held(false, false, true),
        Chord::Ctrl | Chord::CtrlNoShift => held(true, false, false),
        Chord::CtrlShift => held(true, false, true),
        Chord::CtrlAlt => held(true, true, false),
        Chord::AltNoCtrl => held(false, true, false),
    };

    assert!(is_live(row, focus, modifiers));

    kani::cover!(row == 0);
    kani::cover!(row == BINDINGS.len() - 1);
}
