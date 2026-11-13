//! Kani proofs for stepping through search matches. `PROOFS.md` lists
//! each harness with its bounds.
//!
//! `find_matches` is not proven: it runs a regular expression and a
//! substring search over the whole document, which the verifier cannot
//! explore. `tests.rs` covers it.

use super::stepped_match_index;

/// Next and Previous keep the active match inside the list, undo each
/// other, and wrap at the ends. Holds for every count of matches.
#[kani::proof]
fn stepping_matches_wraps_and_inverts() {
    let (current, count): (usize, usize) = (kani::any(), kani::any());
    let (forward, backward): (i32, i32) = (kani::any(), kani::any());
    // The find bar steps only with a match to step to, and keeps the
    // active index inside the list it rebuilt.
    kani::assume(current < count);
    kani::assume(forward >= 0 && backward < 0);

    let next = stepped_match_index(current, count, forward);
    let previous = stepped_match_index(current, count, backward);

    assert!(next < count && previous < count);
    assert!(stepped_match_index(next, count, backward) == current);
    assert!(stepped_match_index(previous, count, forward) == current);
    if current == count - 1 {
        assert!(next == 0);
    }
    if current == 0 {
        assert!(previous == count - 1);
    }
    if count == 1 {
        assert!(next == 0 && previous == 0);
    }

    kani::cover!(count == 1);
    kani::cover!(current == count - 1 && count > 1);
    kani::cover!(current == 0 && count > 1);
}

/// Stepping forward from an index left over from a longer list still
/// lands inside the list, short of the one index that cannot be stepped
/// past.
#[kani::proof]
fn stepping_forward_recovers_a_stale_index() {
    let (current, count): (usize, usize) = (kani::any(), kani::any());
    let forward: i32 = kani::any();
    kani::assume(count > 0 && current < usize::MAX && forward >= 0);

    assert!(stepped_match_index(current, count, forward) < count);

    kani::cover!(current > count);
}
