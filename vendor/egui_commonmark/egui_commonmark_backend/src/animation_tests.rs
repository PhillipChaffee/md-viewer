use std::time::Duration;

use egui::FrameDurations;

use super::{frame_at, FrameBudget, Kept, Pictures};

fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

fn delays(each: &[u64]) -> FrameDurations {
    FrameDurations::new(each.iter().copied().map(ms).collect())
}

#[test]
fn each_frame_is_on_for_its_own_delay_and_the_loop_starts_over() {
    let delays = delays(&[500, 200, 300]);

    assert_eq!(frame_at(&delays, ms(0)), (0, Some(ms(500))));
    assert_eq!(frame_at(&delays, ms(499)), (0, Some(ms(1))));
    assert_eq!(frame_at(&delays, ms(500)), (1, Some(ms(200))));
    assert_eq!(frame_at(&delays, ms(650)), (1, Some(ms(50))));
    assert_eq!(frame_at(&delays, ms(700)), (2, Some(ms(300))));
    assert_eq!(frame_at(&delays, ms(1000)), (0, Some(ms(500))));
    assert_eq!(frame_at(&delays, ms(3_600_700)), (2, Some(ms(300))));
}

#[test]
fn a_picture_of_one_frame_has_no_next_frame() {
    assert_eq!(frame_at(&delays(&[500]), ms(1234)), (0, None));
    assert_eq!(frame_at(&delays(&[0]), ms(1234)), (0, None));
    assert_eq!(frame_at(&delays(&[]), ms(1234)), (0, None));
}

#[test]
fn a_frame_that_asks_for_no_time_is_on_for_a_tenth_of_a_second() {
    let no_delay = delays(&[0, 0, 0]);

    assert_eq!(frame_at(&no_delay, ms(0)), (0, Some(ms(100))));
    assert_eq!(frame_at(&no_delay, ms(150)), (1, Some(ms(50))));
    assert_eq!(frame_at(&no_delay, ms(299)), (2, Some(ms(1))));

    let one_fast = delays(&[10, 400]);

    assert_eq!(frame_at(&one_fast, ms(99)), (0, Some(ms(1))));
    assert_eq!(frame_at(&one_fast, ms(100)), (1, Some(ms(400))));
}

/// A frame of 10 by 10 pixels takes 400 bytes.
const SIDE: egui::Vec2 = egui::vec2(10.0, 10.0);

fn pictures(one_picture: u64, all_pictures: u64) -> Pictures {
    Pictures {
        budget: FrameBudget {
            one_picture,
            all_pictures,
        },
        ..Default::default()
    }
}

fn kept(pictures: &Pictures, uri: &str) -> Kept {
    pictures.known[uri].kept
}

#[test]
fn a_picture_within_the_budget_keeps_every_frame() {
    let mut pictures = pictures(1200, 1200);

    let let_go = pictures.admit("a", SIDE, 3, 0.0);

    assert_eq!(let_go, []);
    assert_eq!(kept(&pictures, "a"), Kept::Every);
    assert_eq!(pictures.known["a"].kept_bytes(), 1200);
}

#[test]
fn a_picture_too_large_to_play_keeps_its_first_frame() {
    let mut pictures = pictures(1199, 10_000);

    pictures.admit("a", SIDE, 3, 0.0);

    assert_eq!(kept(&pictures, "a"), Kept::First);
    assert_eq!(pictures.known["a"].kept_bytes(), 400);
}

#[test]
fn over_the_budget_the_picture_seen_longest_ago_is_let_go_first() {
    let mut pictures = pictures(1200, 2400);
    pictures.admit("old", SIDE, 3, 0.0);
    pictures.admit("older", SIDE, 2, 0.0);
    pictures.known.get_mut("old").unwrap().last_shown = 5.0;
    pictures.known.get_mut("older").unwrap().last_shown = 4.0;

    let first = pictures.admit("new", SIDE, 3, 10.0);
    let second = pictures.admit("newer", SIDE, 3, 10.0);

    assert_eq!(first, [("older".to_owned(), 2)]);
    assert_eq!(second, [("old".to_owned(), 3)]);
    assert_eq!(kept(&pictures, "older"), Kept::Nothing);
    assert_eq!(kept(&pictures, "old"), Kept::Nothing);
    assert_eq!(kept(&pictures, "new"), Kept::Every);
    assert_eq!(kept(&pictures, "newer"), Kept::Every);
}

#[test]
fn a_picture_seen_a_moment_ago_is_not_let_go() {
    let mut pictures = pictures(1200, 1200);
    pictures.admit("seen", SIDE, 3, 0.0);

    let at_once = pictures.admit("new", SIDE, 3, 1.9);
    let later = pictures.let_go_down_to(1200, 2.0);

    assert_eq!(at_once, [], "both are kept: the budget gives way");
    assert_eq!(later, [("seen".to_owned(), 3)]);
    assert_eq!(kept(&pictures, "new"), Kept::Every);
}

#[test]
fn room_is_made_for_a_picture_before_it_is_read_again() {
    let mut pictures = pictures(1200, 2000);
    pictures.admit("back", SIDE, 3, 0.0);
    pictures.admit("small", SIDE, 1, 4.0);
    pictures.admit("large", SIDE, 3, 5.0);
    assert_eq!(kept(&pictures, "back"), Kept::Nothing);

    let made = pictures.make_room_for("back", 10.0);
    let again = pictures.make_room_for("back", 10.0);

    assert_eq!(made, [("small".to_owned(), 1), ("large".to_owned(), 3)]);
    assert_eq!(again, [], "the room is there");
    assert_eq!(pictures.make_room_for("unknown", 10.0), []);
    assert_eq!(pictures.admit("back", SIDE, 3, 10.1), []);
}

#[test]
fn a_picture_that_was_let_go_is_taken_in_again() {
    let mut pictures = pictures(1200, 1200);
    pictures.admit("a", SIDE, 3, 0.0);
    pictures.admit("b", SIDE, 3, 5.0);

    let let_go = pictures.admit("a", SIDE, 3, 10.0);

    assert_eq!(let_go, [("b".to_owned(), 3)]);
    assert_eq!(kept(&pictures, "a"), Kept::Every);
}
