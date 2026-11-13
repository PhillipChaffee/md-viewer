//! Kani proofs for the active-tab index. `PROOFS.md` lists each harness
//! with its bounds.
//!
//! The three functions are loop-free integer arithmetic, so each harness
//! covers every `usize`. What they assume is the invariant the tab strip
//! keeps: with tabs open, the active index names one of them.

use super::{active_after_close, next_index, previous_index};

/// Closing any tab leaves the active index naming an open tab (or 0 with
/// none left), and keeps the same document active unless it was the one
/// closed.
#[kani::proof]
fn closing_a_tab_keeps_the_active_index_valid() {
    let (active, closed, len): (usize, usize, usize) = (kani::any(), kani::any(), kani::any());
    kani::assume(active < len && closed < len);
    let remaining = len - 1;

    let after = active_after_close(active, closed, remaining);

    if remaining == 0 {
        assert!(after == 0);
    } else {
        assert!(after < remaining);
    }
    // Tabs after the closed one move down by one; the rest stay put.
    if active < closed {
        assert!(after == active);
    }
    if active > closed {
        assert!(after == active - 1);
    }
    // Closing the active tab gives its place to the next tab, or to the
    // new last tab where there is no next.
    if active == closed && remaining > 0 {
        assert!(after == active.min(remaining - 1));
    }

    kani::cover!(remaining == 0);
    kani::cover!(active == closed && active == remaining && remaining > 0);
    kani::cover!(active > closed);
    kani::cover!(active < closed);
}

/// The close arithmetic never panics, even from an index that names no
/// tab, and still lands on an open tab.
#[kani::proof]
fn closing_a_tab_is_total() {
    let (active, closed, remaining): (usize, usize, usize) =
        (kani::any(), kani::any(), kani::any());

    let after = active_after_close(active, closed, remaining);

    if remaining == 0 {
        assert!(after == 0);
    } else {
        assert!(after < remaining);
    }

    kani::cover!(active > remaining && remaining > 0);
    kani::cover!(active == usize::MAX && closed == usize::MAX);
}

/// Ctrl+Tab and Ctrl+Shift+Tab stay on an open tab, move to a different
/// one, undo each other, and wrap at the ends.
#[kani::proof]
fn next_and_previous_tab_wrap_and_invert() {
    let (active, len): (usize, usize) = (kani::any(), kani::any());
    // The callers step only with two tabs or more.
    kani::assume(len > 1 && active < len);

    let (next, previous) = (next_index(active, len), previous_index(active, len));

    assert!(next < len && previous < len);
    assert!(next != active && previous != active);
    assert!(previous_index(next, len) == active);
    assert!(next_index(previous, len) == active);
    if active == len - 1 {
        assert!(next == 0);
    }
    if active == 0 {
        assert!(previous == len - 1);
    }

    kani::cover!(active == len - 1);
    kani::cover!(active == 0);
    kani::cover!(active > 0 && active < len - 1);
}
