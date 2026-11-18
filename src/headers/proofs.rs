//! Kani proofs for the folding of the outline. `PROOFS.md` lists each
//! harness with its bounds.
//!
//! The harnesses run the real functions on outlines of at most
//! `MAX_HEADERS` headings with any level a `u8` can hold, and any set of
//! collapsed headings. They are bounded proofs: a longer outline is not
//! covered. The set is a table of flags here, because the verifier cannot
//! run a `HashSet`; `Outline` only looks the index up in one.
//!
//! The app folds the outline with `visible_rows`, one pass over the
//! headings. What that pass must compute is written here a second way,
//! as `has_collapsed_ancestor`: a walk back from one heading through its
//! ancestors. That walk is the specification, not shipped code.
//! `parse_headers` runs the Markdown parser and is covered by `tests.rs`.

use std::mem::ManuallyDrop;

use pulldown_cmark::HeadingLevel;

use super::{any_header_has_children, header_has_children, level_number, visible_rows, Header};

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

/// Whether some ancestor of the header at `index` is one `is_collapsed`
/// names. A header's parent is the nearest header before it with a lower
/// level, and its ancestors are its parent and that parent's ancestors.
fn has_collapsed_ancestor(
    headers: &[Header],
    index: usize,
    is_collapsed: impl Fn(usize) -> bool,
) -> bool {
    if index == 0 || index >= headers.len() {
        return false;
    }
    let mut search_level = headers[index].level;
    // Walk backwards to find ancestors
    for i in (0..index).rev() {
        let h = &headers[i];
        // Only consider headers with lower level than what we're searching for
        if h.level < search_level {
            // Found an ancestor
            if is_collapsed(i) {
                return true;
            }
            // This ancestor is not collapsed, but check its ancestors too
            // Update search_level to only look for even lower level headers
            search_level = h.level;
        }
        // Headers at same or higher level are siblings/cousins, skip them
    }
    false
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

/// Folding in one pass lists exactly the headings that the walk back
/// through the ancestors does not hide, each once and in document order,
/// and nothing outside the outline. The walk is held to the rule in turn:
/// a heading is hidden when its parent is collapsed, and when its parent
/// is hidden; it is never hidden with nothing collapsed, and never by
/// anything but a collapsed heading above it in the tree. Neither reads
/// outside the outline, for any index.
#[kani::proof]
#[kani::unwind(7)]
fn a_heading_is_hidden_exactly_under_a_collapsed_ancestor() {
    let all = any_headers();
    let headers = &all[..any_count()];
    let collapsed: [bool; MAX_HEADERS] = kani::any();
    let index: usize = kani::any();
    let is_collapsed = |heading: usize| collapsed[heading];

    // The rows of the one pass: how many, whether `index` is one, and
    // that each is past the one before.
    let mut rows = 0;
    let mut listed = false;
    let mut last_row = None;
    for row in visible_rows(headers.iter().map(|header| header.level), is_collapsed) {
        assert!(last_row.is_none_or(|last| last < row));
        last_row = Some(row);
        listed |= row == index;
        rows += 1;
    }
    let hidden = has_collapsed_ancestor(headers, index, is_collapsed);

    // The one pass against the walk: a position is a row exactly when it
    // is a heading the walk does not hide.
    assert!(listed == (index < headers.len() && !hidden));

    // The walk against the rule.
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
        assert!(rows == headers.len());
    }

    // A hidden row, a row under an expanded parent, and each of them
    // where the outline skips a level between parent and child.
    let skips_a_level = |child: usize| {
        parent_of(headers, child)
            .is_some_and(|parent| headers[child].level - headers[parent].level > 1)
    };
    kani::cover!(index < headers.len() && !listed);
    kani::cover!(listed && parent_of(headers, index).is_some());
    kani::cover!(listed && skips_a_level(index));
    kani::cover!(index < headers.len() && !listed && skips_a_level(index));
    kani::cover!(listed && index == MAX_HEADERS - 1 && rows < headers.len());
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
