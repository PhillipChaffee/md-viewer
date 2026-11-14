use std::time::Duration;

use egui::FrameDurations;

use super::frame_at;

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
