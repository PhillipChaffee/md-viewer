//! The arithmetic of the title bar, the answer to a request to quit, and
//! what a window that is not a Mac's gets: nothing.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use raw_window_handle::{HandleError, HasWindowHandle, WindowHandle};

use super::*;

/// A window that has no handle to give, and counts how often it is asked.
#[derive(Default)]
struct NoWindow {
    asked: Cell<u32>,
}

impl HasWindowHandle for NoWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.asked.set(self.asked.get() + 1);
        Err(HandleError::Unavailable)
    }
}

#[test]
fn the_title_bar_is_the_row_at_the_top_of_the_window() {
    let placed = placement(800.0, 40.0, 16.0);

    assert_eq!(placed.bar_height.to_bits(), 40.0_f64.to_bits());
    assert_eq!(placed.bar_bottom.to_bits(), 760.0_f64.to_bits());
}

#[test]
fn a_button_sits_on_the_centre_line_of_the_row() {
    for (row, button) in [(40.0, 16.0), (20.0, 14.0), (120.0, 16.0), (50.0, 16.0)] {
        let placed = placement(800.0, row, button);

        let centre = placed.button_bottom + button / 2.0;
        assert_eq!(centre.to_bits(), (row / 2.0).to_bits(), "a row of {row}");
    }
}

#[test]
fn a_row_lower_than_a_button_gets_a_bar_that_holds_the_button() {
    for row in [10.0, 0.0, -5.0, f64::NAN] {
        let placed = placement(800.0, row, 16.0);

        assert_eq!(placed.bar_height.to_bits(), 16.0_f64.to_bits());
        assert_eq!(placed.button_bottom.to_bits(), 0.0_f64.to_bits());
        assert_eq!(placed.bar_bottom.to_bits(), 784.0_f64.to_bits());
    }
}

#[test]
fn a_length_is_off_only_past_the_rounding_to_a_pixel() {
    assert!(!is_off(40.0, 40.0));
    assert!(!is_off(40.5, 40.0));
    assert!(!is_off(39.5, 40.0));
    assert!(is_off(40.75, 40.0));
    assert!(is_off(28.0, 40.0));
    assert!(is_off(40.0, 28.0));
}

/// Tests run off the main thread and on a window without a handle, so on
/// a Mac too these reach nothing of the system's.
#[test]
fn a_window_that_is_not_a_macs_is_left_alone() {
    let window = NoWindow::default();

    center_window_buttons(&window, 40.0);
    hand_quit_key_to_app();

    assert!(window.asked.get() <= 1);
}

/// One test, because the asker is the process's and is given once. Tests
/// run off the main thread, so on a Mac too nothing of the system's is
/// reached.
#[test]
fn a_request_to_quit_gets_the_answer_of_the_first_asker_given() {
    assert_eq!(quit_answer(), QuitAnswer::Now, "nobody asked to be asked");

    let unsaved = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&unsaved);
    ask_before_quit(move || {
        if seen.load(Ordering::Relaxed) {
            QuitAnswer::NotYet
        } else {
            QuitAnswer::Now
        }
    });
    assert_eq!(quit_answer(), QuitAnswer::Now);
    unsaved.store(true, Ordering::Relaxed);
    assert_eq!(quit_answer(), QuitAnswer::NotYet);
    unsaved.store(false, Ordering::Relaxed);
    assert_eq!(quit_answer(), QuitAnswer::Now);

    ask_before_quit(|| QuitAnswer::NotYet);
    assert_eq!(
        quit_answer(),
        QuitAnswer::Now,
        "a second asker replaced the first"
    );
}
