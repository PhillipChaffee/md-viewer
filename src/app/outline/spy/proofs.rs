//! Kani proofs for the search that finds the heading the reader is in.
//! `PROOFS.md` lists each harness with its bounds.
//!
//! The harnesses run the real binary search on documents of at most
//! `MAX_HEADINGS` headings, with any `f32` for every position and any
//! finite view. They are bounded proofs: a longer outline is not covered. The
//! search halves its range, so `MAX_HEADINGS` headings take at most three
//! rounds, and the unwind bound of five leaves the verifier room to show
//! that no fourth round exists.

use super::{heading_in_view, View, READING_LINE};

/// Most headings a harness explores.
const MAX_HEADINGS: usize = 6;

/// Any view whose scroll offset and height are finite; the document's
/// height is any `f32`. An infinite offset and an infinite height can add
/// up to NaN, which the verifier reports as an error where it arises.
fn any_view() -> View {
    let view = View {
        top: kani::any(),
        height: kani::any(),
        content_height: kani::any(),
    };
    kani::assume(view.top.is_finite() && view.height.is_finite());
    view
}

/// Whatever the positions and the view, sorted or not, NaN or not, the
/// search ends, does not overflow, and names one of the headings or none.
#[kani::proof]
#[kani::unwind(5)]
fn heading_in_view_names_a_heading_or_none() {
    let positions: [f32; MAX_HEADINGS] = kani::any();
    let count: usize = kani::any();
    kani::assume(count <= MAX_HEADINGS);
    let view = any_view();

    let current = heading_in_view(count, |index| positions[index], view);

    if let Some(index) = current {
        assert!(index < count);
    }
    if count == 0 {
        assert!(current.is_none());
    }

    kani::cover!(count == MAX_HEADINGS && current == Some(MAX_HEADINGS - 1));
    kani::cover!(count == MAX_HEADINGS && current == Some(0));
    kani::cover!(count > 0 && current.is_none());
}

/// With headings in document order and a view that is laid out and not at
/// the end, the heading named is the last one at or above the reading
/// line. Above the first heading it is the first one where that is on
/// screen, and none otherwise.
#[kani::proof]
#[kani::unwind(7)]
fn heading_in_view_is_the_last_one_reached() {
    let positions: [f32; MAX_HEADINGS] = kani::any();
    let count: usize = kani::any();
    kani::assume(count > 0 && count <= MAX_HEADINGS);
    // What `heading_in_view` asks of `position`: it grows with the index.
    for index in 1..count {
        kani::assume(positions[index - 1] <= positions[index]);
    }
    let view = any_view();
    kani::assume(view.content_height > 0.0 && !view.is_at_the_end());
    let reading_line = view.top + READING_LINE;

    let current = heading_in_view(count, |index| positions[index], view);

    match current {
        Some(index) if positions[index] <= reading_line => {
            assert!(index + 1 == count || positions[index + 1] > reading_line);
        }
        Some(index) => {
            assert!(index == 0);
            assert!(positions[0] < view.top + view.height);
        }
        None => {
            assert!(!(positions[0] <= reading_line));
            assert!(!(positions[0] < view.top + view.height));
        }
    }

    kani::cover!(count == MAX_HEADINGS && current == Some(3));
    kani::cover!(matches!(current, Some(0)) && positions[0] > reading_line);
    kani::cover!(current.is_none());
}

/// At the end of a laid-out document the reader is in its last heading.
#[kani::proof]
#[kani::unwind(5)]
fn the_end_of_the_document_is_the_last_heading() {
    let positions: [f32; MAX_HEADINGS] = kani::any();
    let count: usize = kani::any();
    kani::assume(count > 0 && count <= MAX_HEADINGS);
    let view = any_view();
    kani::assume(view.content_height > 0.0 && view.is_at_the_end());

    assert!(heading_in_view(count, |index| positions[index], view) == Some(count - 1));

    kani::cover!(count == MAX_HEADINGS);
}
