use super::{BlockKind, BlockRhythm};

const RHYTHM: BlockRhythm = BlockRhythm {
    paragraph: 0.9,
    block: 1.25,
    heading_above: 1.6,
    heading_below: 0.5,
    tight_item: 0.35,
    display_math: 0.3,
};

#[test]
fn a_gap_is_the_larger_of_what_two_neighbors_ask_for() {
    use BlockKind::{Boxed, Text};
    let gap = |upper, lower| RHYTHM.gap(upper, lower, 20.0);

    assert_eq!(gap(Text, Text), 18.0);
    assert_eq!(gap(Boxed, Boxed), 25.0);
    assert_eq!(gap(Text, Boxed), 25.0);
    assert_eq!(gap(Boxed, Text), 25.0);
}

#[test]
fn a_heading_decides_the_gaps_around_it() {
    use BlockKind::{Boxed, Heading, Text};
    let gap = |upper, lower| RHYTHM.gap(upper, lower, 20.0);

    for above in [Text, Boxed] {
        assert_eq!(gap(above, Heading), 32.0, "above a heading, after {above:?}");
    }
    // A boxed block asks for more than the heading leaves below itself, and
    // still does not get it: the heading belongs to what follows.
    for below in [Text, Boxed, Heading] {
        assert_eq!(gap(Heading, below), 10.0, "below a heading, before {below:?}");
    }
}

#[test]
fn gaps_are_whole_pixels_of_the_body_font_size() {
    let gap = RHYTHM.gap(BlockKind::Text, BlockKind::Text, 17.0);
    assert_eq!(gap, 15.0, "0.9 x 17 = 15.3");
    assert_eq!(RHYTHM.item_gap(false, 17.0), 6.0, "0.35 x 17 = 5.95");
}

#[test]
fn loose_items_are_a_paragraph_apart_and_tight_ones_closer() {
    assert_eq!(RHYTHM.item_gap(true, 20.0), 18.0);
    assert_eq!(RHYTHM.item_gap(false, 20.0), 7.0);
}
