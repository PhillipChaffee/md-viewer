//! How the text of a document is set: lists, quotes, tasks, footnotes,
//! front matter, rules, inline styles and scripts other than Latin.

use super::pages::{between, each, left_of, page, row_of, NEAR};
use super::support::Reader;

#[test]
fn a_finished_task_is_ticked_and_an_open_one_is_not() {
    let reader = page("# Chores\n\n- [x] done thing\n- [ ] open thing");

    let done = reader.marks_in(left_of(&reader, "done thing"));
    let open = reader.marks_in(left_of(&reader, "open thing"));

    assert_eq!(open.len(), 1, "an open task has a box: {open:?}");
    assert_eq!(done.len(), 2, "a finished task has a box and a tick");
    assert!(
        (done[0].width - open[0].width).abs() < NEAR,
        "one size of box"
    );
    for markup in ["[x]", "[ ]"] {
        reader.assert_hides(markup);
    }
}

#[test]
fn a_task_box_on_the_page_is_not_a_switch() {
    let mut reader = page("# Chores\n\n- [ ] open thing");
    let boxes = reader.marks_in(left_of(&reader, "open thing"));
    let at = (
        boxes[0].left + boxes[0].width / 2.0,
        boxes[0].top + boxes[0].height / 2.0,
    );

    reader.click_point(at.0, at.1);

    assert_eq!(reader.marks_in(left_of(&reader, "open thing")).len(), 1);
    assert_eq!(reader.read_file("page.md"), "# Chores\n\n- [ ] open thing");
}

#[test]
fn tasks_line_up_with_the_plain_items_of_their_list() {
    let reader = page("# Chores\n\n- [x] done thing\n- [ ] open thing\n- plain thing");

    let lefts = ["done thing", "open thing", "plain thing"].map(|item| reader.text_area(item).left);

    assert!((lefts[0] - lefts[1]).abs() < NEAR, "{lefts:?}");
    assert!((lefts[1] - lefts[2]).abs() < NEAR, "{lefts:?}");
}

#[test]
fn each_level_of_a_list_steps_in_by_the_same_amount() {
    let reader = page("# Levels\n\n- top\n  - middle\n    - bottom\n      - deepest");

    let lefts = ["top", "middle", "bottom", "deepest"].map(|item| reader.text_area(item).left);
    let steps = [
        lefts[1] - lefts[0],
        lefts[2] - lefts[1],
        lefts[3] - lefts[2],
    ];

    assert!(steps[0] > 10.0, "a level steps in: {lefts:?}");
    assert!((steps[0] - steps[1]).abs() < NEAR, "{lefts:?}");
    assert!((steps[1] - steps[2]).abs() < NEAR, "{lefts:?}");
}

#[test]
fn every_bullet_has_a_dot_beside_it() {
    let reader = page("# Levels\n\n- top\n  - middle\n- last");

    for item in ["top", "middle", "last"] {
        let dots = reader.marks_in(left_of(&reader, item));
        assert_eq!(dots.len(), 1, "{item}: {dots:?}");
    }
}

#[test]
fn a_numbered_list_counts_from_the_number_it_starts_at() {
    let reader = page("# Steps\n\n5. five\n6. six\n7. seven");

    for number in ["5.", "6.", "7."] {
        reader.assert_sees(number);
    }
    reader.assert_hides("1.");
    let lefts = ["five", "six", "seven"].map(|item| reader.text_area(item).left);
    assert!((lefts[0] - lefts[2]).abs() < NEAR, "{lefts:?}");
}

#[test]
fn a_numbered_list_inside_a_numbered_list_starts_again() {
    let reader = page("# Steps\n\n1. outer one\n   1. inner one\n   2. inner two\n2. outer two");

    let lefts = ["outer one", "inner one", "inner two", "outer two"]
        .map(|item| reader.text_area(item).left);

    assert!(lefts[1] > lefts[0], "the inner list steps in: {lefts:?}");
    assert!((lefts[1] - lefts[2]).abs() < NEAR, "{lefts:?}");
    assert!((lefts[0] - lefts[3]).abs() < NEAR, "{lefts:?}");
    reader.assert_hides("3.");
}

#[test]
fn a_paragraph_inside_a_list_item_keeps_the_items_indent() {
    let reader = page("# Steps\n\n- item\n\n  more about the item\n\nback at the margin");

    let item = reader.text_area("item").left;
    let more = reader.text_area("more about the item").left;
    let after = reader.text_area("back at the margin").left;

    assert!((item - more).abs() < NEAR, "{item} then {more}");
    assert!((after - reader.column().left).abs() < NEAR, "{after}");
}

#[test]
fn a_quote_has_a_bar_down_its_left_side() {
    let reader = page("# Said\n\nbefore\n\n> A quoted remark.\n\nafter");

    let bars = reader.marks_in(left_of(&reader, "A quoted remark."));

    let [bar] = bars[..] else {
        panic!("one bar beside the quote, not {bars:?}");
    };
    assert!(bar.height > bar.width * 3.0, "a bar is tall: {bar:?}");
    assert_ne!(
        reader.color_at(bar.left + bar.width / 2.0, bar.top + bar.height / 2.0),
        reader.page_color()
    );
}

#[test]
fn a_quote_keeps_the_lists_and_paragraphs_inside_it_together() {
    let reader = page("# Said\n\n> first line\n>\n> - quoted item\n>\n> last line\n\nafter");

    let edge = reader.column().left;
    let lefts = ["first line", "quoted item", "last line"].map(|text| reader.text_area(text).left);

    assert!(lefts[0] > edge && lefts[2] > edge, "{lefts:?} from {edge}");
    assert!((lefts[0] - lefts[2]).abs() < NEAR, "{lefts:?}");
    assert!(lefts[1] > lefts[0], "the item steps in again: {lefts:?}");
    assert!((reader.text_area("after").left - edge).abs() < NEAR);
}

/// Known bug, listed in `tests/README.md`: a quote inside a quote is set
/// at the same indent as the quote around it, so the two read as one.
#[test]
#[should_panic(expected = "a quote inside a quote does not step in")]
fn a_quote_inside_a_quote_steps_in_again() {
    let reader = page("# Said\n\n> Outer quote\n>\n> > Inner quote\n\nafter");

    let outer = reader.text_area("Outer quote").left;
    let inner = reader.text_area("Inner quote").left;

    assert!(
        inner > outer + NEAR,
        "a quote inside a quote does not step in"
    );
}

/// Known bug, listed in `tests/README.md`: the first item of a list that
/// follows a quote inside a quote is pushed far to the right of the items
/// under it.
#[test]
#[should_panic(expected = "the first item is out of line")]
fn a_list_after_a_quote_inside_a_quote_is_set_straight() {
    let reader = page("# Said\n\n> Outer\n>\n> > Inner\n\n- first item\n- second item");

    let first = reader.text_area("first item").left;
    let second = reader.text_area("second item").left;

    assert!(
        (first - second).abs() < NEAR,
        "the first item is out of line"
    );
}

#[test]
fn struck_out_text_has_a_line_through_it_and_its_neighbors_do_not() {
    let reader = page("# Edits\n\nkept words ~~dropped words~~ more kept");

    assert!(reader.is_struck_out("dropped words"));
    assert!(!reader.is_struck_out("kept words"));
    assert!(!reader.is_struck_out("more kept"));
    reader.assert_hides("~~");
}

#[test]
fn emoji_shortcodes_become_the_emoji() {
    let cases = [
        (":pushpin:", "\u{1f4cc}"),
        (":rocket:", "\u{1f680}"),
        (":tada:", "\u{1f389}"),
        (":bulb:", "\u{1f4a1}"),
        (":books:", "\u{1f4da}"),
    ];
    let text = each(cases, |(code, _)| {
        format!("A sign {code} in a sentence.\n\n")
    });
    let reader = page(&format!("# Signs\n\n{text}"));

    for (code, emoji) in cases {
        reader.assert_sees(emoji);
        reader.assert_hides(code);
    }
}

#[test]
fn a_shortcode_that_names_no_emoji_stays_as_typed() {
    let reader = page("# Signs\n\nSee :not_an_emoji: and 10:30:45 here.");

    reader.assert_sees(":not_an_emoji:");
    reader.assert_sees("10:30:45");
}

#[test]
fn a_shortcode_in_code_stays_as_typed() {
    let reader = page("# Signs\n\nType `:rocket:` to get one.\n\n```\n:tada:\n```");

    reader.assert_sees(":rocket:");
    reader.assert_sees(":tada:");
    reader.assert_hides("\u{1f680}");
    reader.assert_hides("\u{1f389}");
}

#[test]
fn a_footnote_is_shown_with_its_mark_beside_it() {
    let reader = page("# Notes\n\nA claim with a note.[^1]\n\n[^1]: The note itself.");

    let note = reader.text_area("The note itself.");
    let mark = reader.text_area("1.");

    assert!(mark.right() <= note.left, "{mark:?} before {note:?}");
    assert!(mark.top < note.bottom() && mark.bottom() > note.top);
    assert!(note.top > reader.text_area("A claim with a note.").bottom());
    reader.assert_hides("[^1]");
}

#[test]
fn a_footnotes_mark_sits_right_after_the_words_it_belongs_to() {
    let reader = page("# Notes\n\nA claim with a note.[^1] And more.\n\n[^1]: The note itself.");

    let claim = reader.text_area("A claim with a note.");
    let mark = reader.text_area("1");

    assert!((mark.left - claim.right()).abs() < 3.0, "{mark:?}");
    assert!(mark.top < claim.bottom() && mark.bottom() > claim.top);
}

#[test]
fn each_footnote_keeps_the_mark_it_was_cited_with() {
    let reader = page(
        "# Notes\n\nFirst.[^a] Second.[^b]\n\n[^a]: Note for first.\n\n[^b]: Note for second.",
    );

    for (mark, note) in [("a.", "Note for first."), ("b.", "Note for second.")] {
        let (mark, note) = (reader.text_area(mark), reader.text_area(note));
        assert!(mark.top < note.bottom() && mark.bottom() > note.top);
    }
}

#[test]
fn every_field_of_the_front_matter_is_shown_in_order() {
    let reader =
        page("---\ntitle: Field notes\nauthor: R. Carson\ndraft: true\n---\n\n# Top\n\nbody");

    let rows = ["title", "author", "draft"].map(|name| reader.text_area(name));
    let values = ["Field notes", "R. Carson", "true"].map(|value| reader.text_area(value));

    assert!(rows[0].top < rows[1].top && rows[1].top < rows[2].top);
    for (name, value) in rows.iter().zip(values) {
        assert!(value.left > name.right(), "{value:?} beside {name:?}");
        assert!(value.top < name.bottom() && value.bottom() > name.top);
    }
    assert!((values[0].left - values[2].left).abs() < NEAR, "one column");
    assert!(rows[2].bottom() < reader.text_area("body").top);
}

#[test]
fn front_matter_is_not_part_of_the_title_or_the_contents() {
    let mut reader = page("---\ntitle: From the fields\n---\n\n# Real heading\n\n## Part");
    reader.press("Ctrl+Shift+O");

    assert_eq!(reader.count("Real heading"), 2, "the bar and the page");
    assert_eq!(reader.count("From the fields"), 1, "only the field");
}

#[test]
fn a_document_without_front_matter_shows_no_fields() {
    let reader = page("# Plain\n\ntitle: not a field");

    reader.assert_sees("title: not a field");
}

#[test]
fn dashes_further_down_are_a_rule_and_not_front_matter() {
    let reader = page("# Plain\n\nabove\n\n---\n\nname: value\n\n---\n\nbelow");

    reader.assert_sees("name: value");
    assert_eq!(
        reader
            .marks_in(between(&reader, "above", "name: value"))
            .len(),
        1
    );
}

#[test]
fn front_matter_alone_is_still_a_page() {
    let reader = page("---\ntitle: Only fields\n---\n");

    let name = reader.text_area("title");
    let value = reader.text_area("Only fields");

    assert!(value.left > name.right());
}

#[test]
fn a_rule_is_a_thin_line_centered_in_the_column() {
    let reader = page("# Parts\n\nabove the rule\n\n---\n\nbelow the rule");

    let rules = reader.marks_in(between(&reader, "above the rule", "below the rule"));

    let [rule] = rules[..] else {
        panic!("one rule between the paragraphs, not {rules:?}");
    };
    let column = reader.column();
    assert!(rule.height < 4.0, "thin: {rule:?}");
    assert!(rule.width > 40.0, "a line, not a dot: {rule:?}");
    let middle = rule.left + rule.width / 2.0;
    assert!((middle - (column.left + column.width / 2.0)).abs() < NEAR);
    reader.assert_hides("---");
}

#[test]
fn the_three_ways_to_write_a_rule_all_draw_one() {
    let reader = page("# Parts\n\none\n\n---\n\ntwo\n\n***\n\nthree\n\n___\n\nfour");

    for (above, below) in [("one", "two"), ("two", "three"), ("three", "four")] {
        let rules = reader.marks_in(between(&reader, above, below));
        assert_eq!(rules.len(), 1, "between {above} and {below}");
    }
}

#[test]
fn inline_code_sits_on_a_chip_of_its_own_color() {
    let reader = page("# Code\n\nRun `cargo test` before you push.");

    let code = reader.text_area("cargo test");
    let chip = reader.color_at(code.left + 2.0, code.top + code.height / 2.0);
    let prose = reader.text_area("Run ");
    let paper = reader.color_at(prose.left + 2.0, prose.top + prose.height / 2.0);

    assert_eq!(paper, reader.page_color());
    assert_ne!(chip, paper, "the chip is the color of the page");
    reader.assert_hides("`");
}

#[test]
fn code_is_set_in_a_face_whose_letters_are_all_one_width() {
    let reader = page("# Code\n\n`iiiiii` and `mmmmmm` in code, iiiiii and mmmmmm in prose.");

    let narrow = reader.text_area("iiiiii").width;
    let wide = reader.text_area("mmmmmm").width;

    assert!((narrow - wide).abs() < NEAR, "{narrow} and {wide}");
}

#[test]
fn letters_of_prose_have_widths_of_their_own() {
    let reader = page("# Prose\n\niiiiii\n\nmmmmmm");

    let narrow = reader.text_area("iiiiii").width;
    let wide = reader.text_area("mmmmmm").width;

    assert!(wide > narrow * 1.5, "{narrow} and {wide}");
}

#[test]
fn headings_get_smaller_level_by_level() {
    let reader = page("# One\n\n## Two\n\n### Three\n\n#### Four\n\nbody text");

    let heights =
        ["One", "Two", "Three", "Four", "body text"].map(|text| reader.on_page(text).height);

    assert!(heights[0] > heights[1], "{heights:?}");
    assert!(heights[1] > heights[2], "{heights:?}");
    assert!(heights[2] > heights[3], "{heights:?}");
    assert!(heights[0] > heights[4] * 1.3, "{heights:?}");
}

#[test]
fn text_in_other_scripts_is_on_the_page() {
    let lines = [
        "\u{6f22}\u{5b57}\u{304b}\u{306a}\u{4ea4}\u{3058}\u{308a}",
        "\u{d55c}\u{ad6d}\u{c5b4} \u{bb38}\u{c7a5}",
        "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}, \u{43c}\u{438}\u{440}",
        "\u{393}\u{3b5}\u{3b9}\u{3b1} \u{3c3}\u{3bf}\u{3c5}",
        "\u{5e9}\u{5dc}\u{5d5}\u{5dd}",
    ];
    let reader = page(&format!("# Scripts\n\n{}", lines.join("\n\n")));

    let mut tops = Vec::new();
    for line in lines {
        let area = reader.text_area(line);
        assert!(area.width > 10.0, "{line} takes room: {area:?}");
        tops.push(area.top);
    }
    assert!(tops.is_sorted(), "in the order written: {tops:?}");
}

#[test]
fn a_heading_in_another_script_names_the_document() {
    let mut reader = page(
        "# \u{65e5}\u{672c}\u{8a9e}\u{306e}\u{6587}\u{66f8}\n\n## \u{7b2c}\u{4e00}\u{7ae0}\n\nbody",
    );
    reader.press("Ctrl+Shift+O");

    assert_eq!(
        reader.count("\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{6587}\u{66f8}"),
        2
    );
    reader.press("Ctrl+Shift+O");
    assert_eq!(
        reader.count("\u{7b2c}\u{4e00}\u{7ae0}"),
        2,
        "page and contents"
    );
}

#[test]
fn a_backslash_keeps_markup_characters_as_typed() {
    let reader = page("# Escapes\n\n\\*not slanted\\* and \\# not a heading");

    reader.assert_sees("*not slanted");
    let hash = reader.text_area("# not a heading");
    assert!((hash.height - reader.text_area("* and ").height).abs() < NEAR);
}

#[test]
fn character_entities_are_shown_as_their_characters() {
    let reader = page("# Entities\n\nFish &amp; chips &copy; 2024 &lt;tag&gt;");

    for (character, entity) in [
        ("&", "&amp;"),
        ("\u{a9}", "&copy;"),
        ("<", "&lt;"),
        (">", "&gt;"),
    ] {
        assert_eq!(reader.count(character), 1, "{entity}");
        reader.assert_hides(entity);
    }
}

#[test]
fn a_hard_break_starts_a_new_line_and_a_soft_one_does_not() {
    let reader = page("# Lines\n\nsoft one\nsoft two\n\nhard one  \nhard two");

    let (soft_one, soft_two) = (reader.text_area("soft one"), reader.text_area("soft two"));
    assert!((soft_one.top - soft_two.top).abs() < NEAR, "one line");
    assert!(soft_two.left > soft_one.right());
    let first = reader.text_area("hard one");
    let second = reader.text_area("hard two");
    assert!(second.top >= first.bottom() - NEAR, "{first:?} {second:?}");
    assert!((second.left - first.left).abs() < NEAR);
}

#[test]
fn paragraphs_are_set_apart_from_each_other() {
    let reader = page("# Gaps\n\nfirst paragraph\n\nsecond paragraph");

    let first = reader.text_area("first paragraph");
    let second = reader.text_area("second paragraph");

    assert!(second.top - first.bottom() > 4.0, "{first:?} {second:?}");
    assert!(row_of(&reader, "first paragraph").width > first.width);
}

fn in_view_after(reader: &mut Reader, key: &str, presses: usize, text: &str) -> bool {
    for _ in 0..presses {
        reader.press(key);
    }
    reader.in_view(text)
}

#[test]
fn a_very_long_document_scrolls_to_its_end_and_back() {
    let text = super::docs::long_document(400);
    let mut reader = page(&text);
    let (opening, closing) = (
        "Paragraph 1 mentions the lighthouse once.",
        "Paragraph 400 mentions the lighthouse once.",
    );
    assert!(reader.in_view(opening) && !reader.in_view(closing));

    assert!(in_view_after(&mut reader, "PageDown", 80, closing));
    assert!(!reader.in_view(opening));
    assert!(
        in_view_after(&mut reader, "PageDown", 3, closing),
        "the end is the end"
    );

    assert!(in_view_after(&mut reader, "PageUp", 80, opening));
}

#[test]
fn the_arrow_keys_move_a_long_document_a_line_at_a_time() {
    let mut reader = page(&super::docs::long_document(60));
    let heading = "Section 2";
    let before = reader.text_area(heading).top;

    reader.press("Down");
    let after = reader.text_area(heading).top;
    reader.press("Up");

    assert!(after < before, "{before} then {after}");
    assert!(
        before - after < 200.0,
        "a line, not a page: {before} {after}"
    );
    assert!((reader.text_area(heading).top - before).abs() < NEAR);
}
