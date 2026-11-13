//! Kani proofs for the folding of the outline. `PROOFS.md` lists each
//! harness with its bounds.
//!
//! The harnesses run the real functions on outlines of at most
//! `MAX_HEADERS` headings with any level a `u8` can hold, and any set of
//! collapsed headings. They are bounded proofs: a longer outline is not
//! covered. The set is a table of flags here, because the verifier cannot
//! run a `HashSet`; `header_is_hidden` only looks the index up in one.
//! `parse_headers` runs the Markdown parser and is covered by `tests.rs`.

use std::mem::ManuallyDrop;

use pulldown_cmark::HeadingLevel;

use super::{
    any_header_has_children, has_collapsed_ancestor, header_has_children, level_number, Header,
};

/// Most headings a harness explores.
const MAX_HEADERS: usize = 5;

/// `MAX_HEADERS` headings with any levels, of which a harness takes any
/// number from the front. They are never dropped, which keeps the freeing
/// of their empty titles out of what the verifier explores.
fn any_headers() -> ManuallyDrop<[Header; MAX_HEADERS]> {
    ManuallyDrop::new(std::array::from_fn(|_| Header {
        level: kani::any(),
        title: String::new(),
        source_start: 0,
        line_number: 0,
    }))
}

fn any_count() -> usize {
    let count: usize = kani::any();
    kani::assume(count <= MAX_HEADERS);
    count
}

/// The nearest heading before `index` with a lower level: its parent.
fn parent_of(headers: &[Header], index: usize) -> Option<usize> {
    (0..index)
        .rev()
        .find(|&before| headers[before].level < headers[index].level)
}

/// Whether a heading has children never reads outside the outline, for
/// any index a `usize` can hold, and agrees with the next heading's
/// level; the outline has a fold column exactly when some heading has a
/// child.
#[kani::proof]
#[kani::unwind(7)]
fn has_children_reads_only_the_next_heading() {
    let all = any_headers();
    let headers = &all[..any_count()];
    let index: usize = kani::any();

    let has_children = header_has_children(headers, index);

    if index < headers.len() && index + 1 < headers.len() {
        assert!(has_children == (headers[index + 1].level > headers[index].level));
    } else {
        assert!(!has_children);
    }
    if has_children {
        assert!(any_header_has_children(headers));
    }

    kani::cover!(has_children);
    kani::cover!(index == usize::MAX);
    kani::cover!(headers.len() == MAX_HEADERS && !any_header_has_children(headers));
}

/// The fold test never reads outside the outline, for any index. A
/// heading is hidden when its parent is collapsed, and when its parent is
/// hidden; it is never hidden with nothing collapsed, and never by
/// anything but a collapsed heading above it in the tree.
#[kani::proof]
#[kani::unwind(7)]
fn a_heading_is_hidden_exactly_under_a_collapsed_ancestor() {
    let all = any_headers();
    let headers = &all[..any_count()];
    let collapsed: [bool; MAX_HEADERS] = kani::any();
    let index: usize = kani::any();
    let is_collapsed = |heading: usize| collapsed[heading];

    let hidden = has_collapsed_ancestor(headers, index, is_collapsed);

    if index >= headers.len() {
        assert!(!hidden);
    } else {
        match parent_of(headers, index) {
            // A heading folds away with its parent, and with whatever
            // hides its parent.
            Some(parent) => {
                let parent_hidden = has_collapsed_ancestor(headers, parent, is_collapsed);
                assert!(hidden == (collapsed[parent] || parent_hidden));
            }
            // A heading with no parent is at the top of the tree.
            None => {
                assert!(!hidden);
            }
        }
    }
    if !collapsed.contains(&true) {
        assert!(!hidden);
    }

    kani::cover!(hidden && index == MAX_HEADERS - 1);
    kani::cover!(hidden && parent_of(headers, index).is_some_and(|parent| !collapsed[parent]));
    kani::cover!(!hidden && index > 0 && index < headers.len() && collapsed[0]);
    kani::cover!(index == usize::MAX);
}

/// Every Markdown heading level has a number from 1 to 6, in order.
#[kani::proof]
fn heading_levels_are_one_to_six() {
    let levels = [
        HeadingLevel::H1,
        HeadingLevel::H2,
        HeadingLevel::H3,
        HeadingLevel::H4,
        HeadingLevel::H5,
        HeadingLevel::H6,
    ];
    let position: usize = kani::any();
    kani::assume(position < levels.len());

    assert!(usize::from(level_number(levels[position])) == position + 1);

    kani::cover!(position == 5);
}
