//! Kani proofs for the text-size steps. `PROOFS.md` lists each harness
//! with its bounds.
//!
//! Font discovery and installation read the system's fonts and are covered
//! by `tests.rs`.

use super::TextSizeClass;

fn any_size() -> TextSizeClass {
    TextSizeClass::ALL[usize::from(kani::any::<u8>() % 5)]
}

/// A step up and a step down undo each other, a step up is a larger text
/// in px and a step down a smaller one, and only the two ends of the
/// scale have nowhere further to go.
#[kani::proof]
fn text_size_steps_are_ordered_and_reversible() {
    let size = any_size();

    match size.larger() {
        Some(larger) => {
            assert!(larger.px() > size.px());
            assert!(larger.smaller() == Some(size));
        }
        None => {
            assert!(size == TextSizeClass::Huge);
        }
    }
    match size.smaller() {
        Some(smaller) => {
            assert!(smaller.px() < size.px());
            assert!(smaller.larger() == Some(size));
        }
        None => {
            assert!(size == TextSizeClass::Small);
        }
    }

    kani::cover!(size == TextSizeClass::Huge);
    kani::cover!(size == TextSizeClass::Small);
    kani::cover!(size == TextSizeClass::default());
}

/// `ALL` lists the sizes from smallest to largest, one step apart, so the
/// popover's two buttons walk the whole scale.
#[kani::proof]
fn all_lists_the_sizes_in_step_order() {
    let position: usize = kani::any();
    kani::assume(position < TextSizeClass::ALL.len());
    let size = TextSizeClass::ALL[position];

    assert!(size.larger() == TextSizeClass::ALL.get(position + 1).copied());
    if position == 0 {
        assert!(size.smaller().is_none());
    } else {
        assert!(size.smaller() == Some(TextSizeClass::ALL[position - 1]));
    }

    kani::cover!(position == TextSizeClass::ALL.len() - 1);
}
