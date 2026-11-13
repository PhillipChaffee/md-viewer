//! Kani proofs for a tab's back and forward history and for the reading
//! time. `PROOFS.md` lists each harness with its bounds.
//!
//! The history harnesses run the two helpers `Tab` itself calls, on
//! stacks of at most `MAX_HISTORY` entries. The helpers are generic, so
//! what holds for the `u8` documents here holds for paths. What the
//! harnesses do not run is the loading of a file between the two halves of
//! a step: `Tab::navigate_back` and its siblings are covered by
//! `tests.rs`.

use super::{minutes_for_words, move_history_entry, record_visit, WORDS_PER_MINUTE};

/// Longest back or forward stack a harness explores.
const MAX_HISTORY: usize = 3;

/// A tab's history as the proofs see it: where Back goes, the document
/// shown, and where Forward goes.
struct History {
    back: Vec<u8>,
    current: u8,
    forward: Vec<u8>,
}

impl History {
    /// Any history with at most `MAX_HISTORY` entries each way.
    fn any() -> Self {
        Self {
            back: any_stack(),
            current: kani::any(),
            forward: any_stack(),
        }
    }

    /// What `Tab::navigate_back` does once the earlier document loaded.
    fn go_back(&mut self) -> bool {
        let Some(&previous) = self.back.last() else {
            return false;
        };
        let left = std::mem::replace(&mut self.current, previous);
        move_history_entry(&mut self.back, &mut self.forward, left);
        true
    }

    /// What `Tab::navigate_forward` does once the later document loaded.
    fn go_forward(&mut self) -> bool {
        let Some(&next) = self.forward.last() else {
            return false;
        };
        let left = std::mem::replace(&mut self.current, next);
        move_history_entry(&mut self.forward, &mut self.back, left);
        true
    }

    /// What `Tab::navigate_to_path` does once the new document loaded.
    fn visit(&mut self, document: u8) {
        let left = std::mem::replace(&mut self.current, document);
        record_visit(&mut self.back, &mut self.forward, left);
    }

    fn snapshot(&self) -> (Vec<u8>, u8, Vec<u8>) {
        (self.back.clone(), self.current, self.forward.clone())
    }
}

fn any_stack() -> Vec<u8> {
    let length: usize = kani::any();
    kani::assume(length <= MAX_HISTORY);
    let mut stack = Vec::with_capacity(length);
    for _ in 0..length {
        stack.push(kani::any());
    }
    stack
}

/// Back then Forward shows the same document with the same history, and
/// so does Forward then Back.
#[kani::proof]
#[kani::unwind(5)]
fn back_and_forward_undo_each_other() {
    let mut history = History::any();
    let before = history.snapshot();

    if history.go_back() {
        assert!(history.go_forward());
        assert!(history.snapshot() == before);
        kani::cover!(before.0.len() == MAX_HISTORY && before.2.len() == MAX_HISTORY);
    } else {
        assert!(before.0.is_empty());
        assert!(history.snapshot() == before);
    }

    if history.go_forward() {
        assert!(history.go_back());
        assert!(history.snapshot() == before);
    } else {
        assert!(before.2.is_empty());
    }

    kani::cover!(before.0.is_empty() && before.2.is_empty());
}

/// A step through history loses no document and invents none: Back moves
/// exactly one entry from the back stack to the forward stack and shows
/// the entry it took.
#[kani::proof]
#[kani::unwind(5)]
fn a_step_back_moves_exactly_one_entry() {
    let mut history = History::any();
    let (back, current, forward) = history.snapshot();
    kani::assume(!back.is_empty());

    assert!(history.go_back());

    assert!(history.current == back[back.len() - 1]);
    assert!(history.back[..] == back[..back.len() - 1]);
    assert!(history.forward[..forward.len()] == forward[..]);
    assert!(history.forward.len() == forward.len() + 1);
    assert!(history.forward[forward.len()] == current);

    kani::cover!(back.len() == MAX_HISTORY);
}

/// Opening a new document from a tab makes Back return to the document
/// that was shown, keeps the older history under it, and leaves nothing
/// to go forward to.
#[kani::proof]
#[kani::unwind(5)]
fn a_visit_can_be_undone_and_clears_forward() {
    let mut history = History::any();
    let (back, current, _) = history.snapshot();
    let document: u8 = kani::any();

    history.visit(document);

    assert!(history.current == document);
    assert!(history.forward.is_empty());
    assert!(history.back.len() == back.len() + 1);
    assert!(history.go_back());
    assert!(history.current == current);
    assert!(history.back == back);
    assert!(history.forward.len() == 1 && history.forward[0] == document);

    kani::cover!(back.len() == MAX_HISTORY);
}

/// Reading time is at least a minute and never overflows, for every
/// `usize` of words, and a document of up to a minute's words takes one.
#[kani::proof]
fn reading_minutes_never_overflow() {
    let words: usize = kani::any();
    let minutes = minutes_for_words(words);

    assert!(minutes >= 1);
    if words <= WORDS_PER_MINUTE {
        assert!(minutes == 1);
    }

    kani::cover!(words == 0);
    kani::cover!(words == usize::MAX);
    kani::cover!(minutes > 1);
}

/// For a document of up to `u32::MAX` words, the most the recent list
/// records, the reading time is the least whole number of minutes that
/// covers the words.
#[kani::proof]
fn reading_minutes_round_up() {
    let words: usize = kani::any();
    kani::assume(words > 0 && words <= u32::MAX as usize);

    let minutes = minutes_for_words(words);

    assert!((minutes - 1) * WORDS_PER_MINUTE < words);
    assert!(words <= minutes * WORDS_PER_MINUTE);

    kani::cover!(words % WORDS_PER_MINUTE == 0);
    kani::cover!(words % WORDS_PER_MINUTE == 1 && words > WORDS_PER_MINUTE);
}
