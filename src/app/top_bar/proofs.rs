//! Kani proofs for where the top row puts its tabs and its new-tab
//! button. `PROOFS.md` lists each harness with its bounds.

use eframe::egui::Rangef;

use super::{new_tab_span, tabs_width, Row};
use crate::app::chrome::BUTTON_SIDE;

/// The edges of a bar and its inset are kept within this, far past any
/// screen, so that no sum of them overflows to infinity.
const FAR: f32 = 1.0e30;

/// For a bar and any inset kept at its left end, the tabs and the
/// new-tab button stay between the buttons at both ends: their room
/// starts after the buttons at the left, ends before those at the right
/// and before the edit controls, and the new-tab button lies inside it
/// however wide the tabs are.
#[kani::proof]
fn the_tabs_and_the_new_tab_button_keep_clear_of_the_buttons() {
    let (left, right, inset): (f32, f32, f32) = (kani::any(), kani::any(), kani::any());
    kani::assume(left >= -FAR && right <= FAR && left <= right);
    kani::assume((0.0..=FAR).contains(&inset));
    let (edit_left, shown): (Option<f32>, f32) = (kani::any(), kani::any());
    kani::assume(edit_left.is_none_or(f32::is_finite) && shown.is_finite());

    let row = Row::new(Rangef::new(left, right), inset);
    let room = row.tab_room(edit_left);

    assert!(row.leading.min >= left);
    assert!(room.min >= row.leading.max && room.min <= room.max);
    if room.max > room.min {
        assert!(room.max <= row.trailing.min);
        assert!(edit_left.is_none_or(|edit| room.max <= edit));
    }
    assert!(tabs_width(room) >= 0.0);
    match new_tab_span(room, shown) {
        Some(button) => {
            assert!(room.max - room.min >= BUTTON_SIDE);
            assert!(button.min >= room.min && button.max <= room.max);
            assert!(button.min <= button.max);
        }
        None => {
            assert!(room.max - room.min < BUTTON_SIDE);
        }
    }

    kani::cover!(room.max > room.min && new_tab_span(room, shown).is_none());
    kani::cover!(new_tab_span(room, shown).is_some_and(|button| button.max == room.max));
    kani::cover!(new_tab_span(room, shown).is_some_and(|button| button.max < room.max));
    kani::cover!(edit_left.is_some_and(|edit| edit < row.middle.min));
}
