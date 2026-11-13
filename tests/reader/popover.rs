//! The appearance popover in depth: every text size, the zoom scale, and
//! how the width slider, Fit window and Reset play with the window.

use super::pages::{page, NEAR};
use super::support::Reader;

const SIZES: [&str; 5] = ["Small", "Normal", "Large", "Extra large", "Huge"];
const SPACINGS: [&str; 3] = ["Compact", "Comfortable", "Airy"];
const PARAGRAPH: &str = "The sea keeps its own hours.";

/// A page with a paragraph long enough to wrap, a heading and a list.
fn article() -> Reader {
    let body = "A long sentence that will wrap in any column. ".repeat(12);
    page(&format!(
        "# Tides\n\n{PARAGRAPH}\n\n{body}\n\n## Slack water\n\n- ebb\n- flow"
    ))
}

/// The same page with the appearance popover open.
fn popover() -> Reader {
    let mut reader = article();
    reader.click("Appearance");
    reader
}

/// The zoom the menu reports, such as `125%`.
fn zoom(reader: &mut Reader) -> String {
    reader.click("More");
    let names = reader.names();
    let readout = names
        .iter()
        .find_map(|name| name.strip_prefix("Reset Zoom, "));
    let zoom = readout.unwrap().to_owned();
    reader.press("Escape");
    zoom
}

/// Step the text size from Small up, and return what `measure` reads at
/// each of the five sizes.
fn at_each_size<T>(reader: &mut Reader, measure: impl Fn(&Reader) -> T) -> Vec<T> {
    for _ in 1..SIZES.len() {
        if reader.is_enabled("Smaller text") {
            reader.click("Smaller text");
        }
    }
    assert!(
        !reader.is_enabled("Smaller text"),
        "no size is the smallest"
    );
    let mut read = vec![measure(reader)];
    for _ in 1..SIZES.len() {
        reader.click("Larger text");
        read.push(measure(reader));
    }
    read
}

#[track_caller]
fn assert_width(reader: &Reader, expected: f32) {
    let width = reader.column().width;
    assert!(
        (width - expected).abs() < NEAR,
        "the column is {width} wide, not {expected}"
    );
}

#[test]
fn the_popover_holds_the_themes_the_size_the_width_and_the_spacing() {
    let reader = popover();

    for control in [
        "Paper",
        "Sepia",
        "Dusk",
        "Night",
        "Smaller text",
        "Larger text",
        "Text column width",
        "Fit window",
        "Reset width",
        "Compact",
        "Comfortable",
        "Airy",
        "Fonts\u{2026}",
    ] {
        assert_eq!(reader.count(control), 1, "{control}");
    }
    for label in ["Text size", "Width", "Line spacing", "Normal", "680 px"] {
        reader.assert_sees(label);
    }
}

#[test]
fn every_control_acts_at_once_and_leaves_the_popover_open() {
    let mut reader = popover();

    for control in ["Night", "Larger text", "Airy", "Fit window", "Reset width"] {
        reader.click(control);
        assert_eq!(
            reader.count("Fit window"),
            1,
            "{control} closed the popover"
        );
    }

    reader.slide("Text column width", 0.3);
    assert_eq!(
        reader.count("Fit window"),
        1,
        "the slider closed the popover"
    );
    assert!(reader.is_on("Night") && reader.is_on("Airy"));
    assert_eq!(reader.count("Large"), 1, "the size named is Large");
}

#[test]
fn clicking_the_button_again_closes_the_popover() {
    let mut reader = popover();

    reader.click("Appearance");

    assert_eq!(reader.count("Fit window"), 0);
}

#[test]
fn the_fonts_row_opens_the_fonts_dialog_and_closes_the_popover() {
    let mut reader = popover();

    reader.click("Fonts\u{2026}");

    assert_eq!(reader.count("Fit window"), 0);
    reader.assert_sees("Preset");
    assert_eq!(reader.count("System Default"), 1);
}

#[test]
fn the_popover_works_on_the_home_page_too() {
    let mut reader = Reader::first_run();
    let sepia = reader.page_color();

    reader.click("Appearance");
    reader.click("Night");

    assert!(reader.is_on("Night"));
    assert_ne!(reader.page_color(), sepia);
}

#[test]
fn each_of_the_five_text_sizes_is_larger_than_the_one_before() {
    let mut reader = popover();

    let heights = at_each_size(&mut reader, |reader| reader.text_area(PARAGRAPH).height);
    let widths = at_each_size(&mut reader, |reader| reader.text_area(PARAGRAPH).width);

    assert!(
        heights.windows(2).all(|pair| pair[1] > pair[0]),
        "{heights:?}"
    );
    assert!(
        widths.windows(2).all(|pair| pair[1] > pair[0]),
        "{widths:?}"
    );
}

#[test]
fn the_popover_names_each_text_size_as_it_is_reached() {
    let mut reader = popover();

    let names = at_each_size(&mut reader, |reader| {
        let shown: Vec<&str> = SIZES
            .into_iter()
            .filter(|size| reader.count(size) == 1)
            .collect();
        shown
    });

    assert_eq!(names, SIZES.map(|size| vec![size]));
}

#[test]
fn headings_stay_larger_than_the_text_at_every_size() {
    let mut reader = popover();

    let pairs = at_each_size(&mut reader, |reader| {
        (
            reader.text_area("Slack water").height,
            reader.text_area(PARAGRAPH).height,
        )
    });

    for (heading, body) in &pairs {
        assert!(heading > &(body * 1.1), "{pairs:?}");
    }
    assert!(pairs[4].0 > pairs[0].0, "headings grow too: {pairs:?}");
}

#[test]
fn the_text_size_leaves_the_column_as_wide_as_it_was() {
    let mut reader = popover();

    let widths = at_each_size(&mut reader, |reader| reader.column().width);

    assert!(
        widths.iter().all(|width| (width - 680.0).abs() < NEAR),
        "{widths:?}"
    );
}

#[test]
fn a_larger_text_size_wraps_the_same_words_onto_more_lines() {
    let mut reader = popover();
    let wrapped = "A long sentence that will wrap in any column. ".repeat(12);

    let heights = at_each_size(&mut reader, |reader| {
        reader.text_area(wrapped.trim_end()).height
    });

    assert!(heights[4] > heights[0] * 1.5, "{heights:?}");
    let column = reader.column();
    assert!(reader.text_area(wrapped.trim_end()).right() <= column.right() + NEAR);
}

#[test]
fn the_fonts_dialog_picks_from_the_same_five_sizes() {
    let mut reader = article();
    reader.click("More");
    reader.click("Fonts\u{2026}");
    let normal = reader.text_area(PARAGRAPH).height;

    reader.click("Huge");
    let huge = reader.text_area(PARAGRAPH).height;
    reader.click("Small");
    let small = reader.text_area(PARAGRAPH).height;

    assert!(small < normal && normal < huge, "{small} {normal} {huge}");
    reader.click("Close window");
    reader.click("Appearance");
    reader.assert_sees("Small");
    assert!(!reader.is_enabled("Smaller text"));
}

#[test]
fn zooming_in_goes_up_by_a_quarter_each_step_to_three_times() {
    let mut reader = article();
    let mut steps = Vec::new();

    for _ in 0..6 {
        reader.press("Ctrl++");
        steps.push(zoom(&mut reader));
    }

    assert_eq!(steps, ["125%", "156%", "195%", "244%", "300%", "300%"]);
}

#[test]
fn zooming_out_goes_down_by_a_fifth_each_step_to_a_half() {
    let mut reader = article();
    let mut steps = Vec::new();

    for _ in 0..5 {
        reader.press("Ctrl+-");
        steps.push(zoom(&mut reader));
    }

    assert_eq!(steps, ["80%", "64%", "51%", "50%", "50%"]);
}

#[test]
fn a_step_in_and_a_step_out_cancel_each_other() {
    let mut reader = article();
    let before = reader.text_area(PARAGRAPH);

    reader.press("Ctrl+-");
    reader.press("Ctrl++");

    assert_eq!(zoom(&mut reader), "100%");
    assert_eq!(reader.text_area(PARAGRAPH), before);
}

#[test]
fn zoom_scales_the_bars_and_buttons_with_the_page() {
    let mut reader = article();
    let (button, text) = (reader.area_of("More"), reader.text_area(PARAGRAPH));

    reader.press("Ctrl++");

    let (zoomed_button, zoomed_text) = (reader.area_of("More"), reader.text_area(PARAGRAPH));
    assert!((zoomed_button.width / button.width - 1.25).abs() < 0.02);
    assert!((zoomed_text.width / text.width - 1.25).abs() < 0.03);
    assert!(zoomed_button.right() <= 1200.0, "still inside the window");
}

#[test]
fn ctrl_and_the_wheel_zoom_in_and_out() {
    let mut reader = article();
    let before = reader.text_area(PARAGRAPH).width;

    reader.ctrl_wheel(120.0);
    let closer = reader.text_area(PARAGRAPH).width;
    reader.ctrl_wheel(-240.0);
    let further = reader.text_area(PARAGRAPH).width;

    assert!(closer > before + 2.0, "{before} then {closer}");
    assert!(further < before - 2.0, "{before} then {further}");
}

#[test]
fn the_zoom_row_of_the_menu_stays_open_while_it_is_pressed() {
    let mut reader = article();
    reader.click("More");

    reader.click("Zoom In");
    reader.click("Zoom In");
    assert_eq!(reader.count("Reset Zoom, 156%"), 1);

    reader.click("Zoom Out");
    assert_eq!(reader.count("Reset Zoom, 125%"), 1);

    reader.click("Reset Zoom, 125%");
    assert_eq!(reader.count("Reset Zoom, 100%"), 1);
}

#[test]
fn the_slider_runs_from_the_narrowest_column_to_the_one_that_fits_the_window() {
    let mut reader = popover();

    reader.slide("Text column width", 0.0);
    assert_width(&reader, 280.0);

    reader.slide("Text column width", 1.0);
    let widest = reader.column().width;
    reader.click("Fit window");
    assert_width(&reader, widest);
}

#[test]
fn the_slider_stands_where_the_column_is() {
    let mut reader = popover();

    for _ in 0..3 {
        reader.press("Ctrl+Alt+Left");
    }

    assert!((reader.number_of("Text column width") - 560.0).abs() < 1.0);
    reader.assert_sees("560 px");
    assert_width(&reader, 560.0);
}

#[test]
fn a_wider_window_gives_the_slider_more_to_offer() {
    let mut reader = popover();
    reader.slide("Text column width", 1.0);
    let widest = reader.column().width;

    reader.press("Escape");
    reader.resize_window(1600.0, 800.0);
    reader.click("Appearance");
    reader.slide("Text column width", 1.0);

    assert!(
        reader.column().width > widest + 200.0,
        "{:?}",
        reader.column()
    );
}

#[test]
fn fit_window_and_reset_take_turns() {
    let mut reader = popover();

    for _ in 0..2 {
        reader.click("Fit window");
        reader.assert_sees("Fits the window");
        reader.assert_hides("680 px");
        assert!(reader.column().width > 900.0);

        reader.click("Reset width");
        reader.assert_sees("680 px");
        reader.assert_hides("Fits the window");
        assert_width(&reader, 680.0);
    }
}

#[test]
fn a_fitted_column_follows_the_window_wider_and_narrower() {
    let mut reader = popover();
    reader.click("Fit window");
    reader.press("Escape");
    let at_1200 = reader.column().width;

    reader.resize_window(1600.0, 800.0);
    let at_1600 = reader.column().width;
    reader.resize_window(900.0, 800.0);
    let at_900 = reader.column();

    assert!(at_1600 > at_1200 + 200.0, "{at_1200} then {at_1600}");
    assert!(
        at_900.width < at_1600 - 200.0 && at_900.right() <= 900.0,
        "{at_900:?}"
    );
}

#[test]
fn a_chosen_width_is_kept_when_the_window_has_room_for_it() {
    let mut reader = article();

    reader.resize_window(1600.0, 900.0);
    assert_width(&reader, 680.0);

    reader.resize_window(1000.0, 700.0);
    assert_width(&reader, 680.0);
}

#[test]
fn a_window_too_narrow_for_the_chosen_width_gets_it_back_when_it_grows() {
    let mut reader = article();

    reader.resize_window(500.0, 700.0);
    let squeezed = reader.column();
    assert!(
        squeezed.width < 500.0 && squeezed.right() <= 500.0,
        "{squeezed:?}"
    );

    reader.resize_window(1200.0, 800.0);
    assert_width(&reader, 680.0);
}

#[test]
fn stepping_narrower_from_a_fitted_column_leaves_fit_window() {
    let mut reader = popover();
    reader.click("Fit window");
    let fitted = reader.column().width;

    reader.press("Ctrl+Alt+Left");

    reader.assert_hides("Fits the window");
    let stepped = reader.column().width;
    assert!(
        stepped < fitted && stepped > fitted - 80.0,
        "{fitted} then {stepped}"
    );
    reader.press("Escape");
    reader.click("More");
    assert!(!reader.is_on("Full Width"));
}

#[test]
fn the_readout_follows_an_edge_that_was_dragged() {
    let mut reader = article();

    reader.drag("Resize text column, right edge", 50.0, 0.0);
    reader.click("Appearance");

    reader.assert_sees("780 px");
    assert!((reader.number_of("Text column width") - 780.0).abs() < 1.0);
}

#[test]
fn exactly_one_line_spacing_is_chosen_at_a_time() {
    let mut reader = popover();

    for spacing in SPACINGS {
        reader.click(spacing);

        let chosen: Vec<&str> = SPACINGS
            .into_iter()
            .filter(|name| reader.is_on(name))
            .collect();
        assert_eq!(chosen, [spacing]);
    }
}

#[test]
fn a_wrapped_paragraph_grows_taller_with_each_line_spacing() {
    let mut reader = popover();
    let wrapped = "A long sentence that will wrap in any column. ".repeat(12);
    let mut heights = Vec::new();

    for spacing in SPACINGS {
        reader.click(spacing);
        heights.push(reader.text_area(wrapped.trim_end()).height);
    }

    assert!(
        heights[0] < heights[1] && heights[1] < heights[2],
        "{heights:?}"
    );
    assert_width(&reader, 680.0);
}

#[test]
fn the_line_spacing_in_the_menu_is_the_same_setting() {
    let mut reader = article();
    reader.click("More");
    reader.click("Line Spacing");
    assert!(reader.is_on("Comfortable"));

    reader.click("Airy");

    reader.click("Appearance");
    assert!(reader.is_on("Airy") && !reader.is_on("Comfortable"));
    reader.click("Compact");
    reader.press("Escape");
    reader.click("More");
    reader.click("Line Spacing");
    assert!(reader.is_on("Compact") && !reader.is_on("Airy"));
}

#[test]
fn the_width_shortcuts_work_while_the_popover_is_open() {
    let mut reader = popover();

    reader.press("Ctrl+Alt+Left");

    reader.assert_sees("640 px");
    assert_width(&reader, 640.0);
    assert_eq!(reader.count("Fit window"), 1, "the popover stayed open");
}
