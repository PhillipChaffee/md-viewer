use super::*;

/// The default column and its two margins.
const DEFAULT_PANE: f32 = 680.0 + 2.0 * 32.0;

#[test]
fn a_window_that_holds_the_column_and_the_outline_has_room() {
    let window = DEFAULT_PANE + OUTLINE_DEFAULT_WIDTH;

    assert!(has_room(window, None, ReadingWidth::DEFAULT));
    assert!(!has_room(window - 1.0, None, ReadingWidth::DEFAULT));
}

#[test]
fn an_open_explorer_takes_its_width_out_of_the_room() {
    let window = DEFAULT_PANE + OUTLINE_DEFAULT_WIDTH + 216.0;

    assert!(has_room(window, Some(216.0), ReadingWidth::DEFAULT));
    assert!(!has_room(window, Some(217.0), ReadingWidth::DEFAULT));
}

#[test]
fn a_wider_column_needs_a_wider_window() {
    let window = DEFAULT_PANE + OUTLINE_DEFAULT_WIDTH;

    assert!(!has_room(window, None, ReadingWidth::new(900.0)));
    assert!(has_room(window + 220.0, None, ReadingWidth::new(900.0)));
    assert!(has_room(window - 200.0, None, ReadingWidth::new(480.0)));
}

#[test]
fn a_column_that_fits_the_window_is_measured_at_the_default_width() {
    let window = DEFAULT_PANE + OUTLINE_DEFAULT_WIDTH;

    assert!(has_room(window, None, ReadingWidth::FIT_WINDOW));
    assert!(!has_room(window - 1.0, None, ReadingWidth::FIT_WINDOW));
}

#[test]
fn a_session_that_saved_the_outline_keeps_the_reader_in_charge() {
    assert_eq!(OutlineRule::restored(Some(false)), OutlineRule::Reader);
    assert_eq!(OutlineRule::restored(Some(true)), OutlineRule::Reader);
    assert_eq!(
        OutlineRule::restored(None),
        OutlineRule::Room { had_room: true }
    );
}

#[test]
fn only_the_readers_choice_is_saved() {
    assert_eq!(OutlineRule::Reader.saved(false), Some(false));
    assert_eq!(OutlineRule::Reader.saved(true), Some(true));
    assert_eq!(OutlineRule::Room { had_room: true }.saved(true), None);
}

#[test]
fn the_outline_follows_the_room_when_the_room_changes() {
    let mut rule = OutlineRule::restored(None);

    assert_eq!(rule.follow(true), None, "a session starts as if with room");
    assert_eq!(rule.follow(false), Some(false));
    assert_eq!(rule.follow(false), None);
    assert_eq!(rule.follow(true), Some(true));
}

#[test]
fn the_readers_choice_does_not_follow_the_room() {
    let mut rule = OutlineRule::Reader;

    assert_eq!(rule.follow(false), None);
    assert_eq!(rule.follow(true), None);
    assert_eq!(rule, OutlineRule::Reader);
}
