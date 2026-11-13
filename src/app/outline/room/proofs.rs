//! Kani proofs for the rule that shows the outline while there is room.
//! `PROOFS.md` lists each harness with its bounds.

use super::{has_room, OutlineRule};
use crate::layout::ReadingWidth;

fn any_rule() -> OutlineRule {
    if kani::any() {
        OutlineRule::Reader
    } else {
        OutlineRule::Room {
            had_room: kani::any(),
        }
    }
}

/// What a session saves about the outline comes back as the same rule: a
/// reader's choice is saved and stays the reader's, and a session where
/// nobody chose saves nothing and stays with the room rule.
#[kani::proof]
fn the_outline_rule_survives_a_restart() {
    let rule = any_rule();
    let shown: bool = kani::any();

    let saved = rule.saved(shown);
    let restored = OutlineRule::restored(saved);

    match rule {
        OutlineRule::Reader => {
            assert!(saved == Some(shown));
            assert!(restored == OutlineRule::Reader);
        }
        OutlineRule::Room { .. } => {
            assert!(saved.is_none());
            assert!(matches!(restored, OutlineRule::Room { .. }));
        }
    }
    assert!(restored.saved(shown) == saved);

    kani::cover!(rule == OutlineRule::Reader && !shown);
    kani::cover!(matches!(rule, OutlineRule::Room { had_room: false }));
}

/// The window's width never overrides the reader: once they have chosen,
/// `follow` asks for nothing. Under the room rule it asks for a change
/// exactly when the room changed, and not again while it stays the same.
#[kani::proof]
fn follow_reports_each_change_of_room_once() {
    let mut rule = any_rule();
    let before = rule;
    let has_room: bool = kani::any();

    let change = rule.follow(has_room);

    match before {
        OutlineRule::Reader => {
            assert!(change.is_none());
            assert!(rule == OutlineRule::Reader);
        }
        OutlineRule::Room { had_room } => {
            assert!(change == (had_room != has_room).then_some(has_room));
            assert!(rule == OutlineRule::Room { had_room: has_room });
        }
    }
    assert!(rule.follow(has_room).is_none());

    kani::cover!(change == Some(true));
    kani::cover!(change == Some(false));
}

/// A wider window never loses the room a narrower one had, and a window
/// whose width is NaN has none.
#[kani::proof]
fn a_wider_window_keeps_its_room() {
    let (narrow, wide): (f32, f32) = (kani::any(), kani::any());
    let explorer_width: Option<f32> = kani::any();
    let reading_width = ReadingWidth::new(kani::any());
    kani::assume(narrow <= wide && wide.is_finite() && narrow.is_finite());
    kani::assume(explorer_width.is_none_or(f32::is_finite));

    let narrow_has_room = has_room(narrow, explorer_width, reading_width);
    let wide_has_room = has_room(wide, explorer_width, reading_width);

    if narrow_has_room {
        assert!(wide_has_room);
    }
    assert!(!has_room(f32::NAN, explorer_width, reading_width));

    kani::cover!(narrow_has_room);
    kani::cover!(!narrow_has_room && wide_has_room);
}

/// Opening the explorer, or widening it, never makes room for the
/// outline.
#[kani::proof]
fn the_explorer_only_takes_room() {
    let window: f32 = kani::any();
    let explorer_width: f32 = kani::any();
    let reading_width = ReadingWidth::new(kani::any());
    kani::assume(window.is_finite() && explorer_width.is_finite() && explorer_width >= 0.0);

    let with_explorer = has_room(window, Some(explorer_width), reading_width);
    let without_explorer = has_room(window, None, reading_width);

    if with_explorer {
        assert!(without_explorer);
    }

    kani::cover!(with_explorer);
    kani::cover!(!with_explorer && without_explorer);
}
