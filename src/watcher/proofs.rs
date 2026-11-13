//! Kani proofs for the watcher's retry counter. `PROOFS.md` lists each
//! harness with its bounds.
//!
//! The rest of the watcher works on paths and on the operating system's
//! change notifications, and is covered by `tests.rs`.

use super::{next_watcher_retry, MAX_WATCHER_RETRIES};

/// The retry counter never overflows, goes up by one while retries are
/// left, and stops at the limit: from zero it allows exactly
/// `MAX_WATCHER_RETRIES` retries.
#[kani::proof]
fn watcher_retries_are_bounded() {
    let current: u32 = kani::any();

    match next_watcher_retry(current) {
        Some(next) => {
            assert!(current < MAX_WATCHER_RETRIES);
            assert!(next == current + 1 && next <= MAX_WATCHER_RETRIES);
        }
        None => {
            assert!(current >= MAX_WATCHER_RETRIES);
        }
    }

    let mut attempt = 0;
    let mut retries = 0_u32;
    while let Some(next) = next_watcher_retry(attempt) {
        attempt = next;
        retries += 1;
    }
    assert!(retries == MAX_WATCHER_RETRIES);

    kani::cover!(current == u32::MAX);
    kani::cover!(current == MAX_WATCHER_RETRIES - 1);
}
