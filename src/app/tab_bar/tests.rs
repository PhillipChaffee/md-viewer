//! The tabs of the top row, clicked through by label: a click goes in,
//! the open tabs and the active document come out.

use std::collections::HashSet;
use std::path::PathBuf;

use eframe::egui::accesskit::Toggled;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{CLOSE_SIDE, NAME_WIDTH, TAB_PADDING, UNDERLINE};
use crate::app::chrome::{BAR_HEIGHT, BUTTON_SIDE};
use crate::app::content::tests::label_harness::LabelHarness;
use crate::test_support::{assert_close, painted_shapes};

/// `a.md`, `b.md`, and `c.md`, each in its own tab with `c.md` active. None
/// has a heading, so each tab is named by its file, and the explorer is
/// hidden, so a file name labels its tab and nothing else.
pub(crate) fn three_tabs() -> LabelHarness {
    let mut harness =
        LabelHarness::with_docs(&[("a.md", "Alpha."), ("b.md", "Beta."), ("c.md", "Gamma.")]);
    let (b, c) = (harness.path("b.md"), harness.path("c.md"));
    let app = harness.state_mut();
    app.show_explorer = false;
    app.open_in_new_tab(b);
    app.open_in_new_tab(c);
    harness.frames(2);
    harness
}

/// One document in one tab, with the explorer hidden.
pub(crate) fn one_tab(content: &str) -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[("a.md", content)]);
    harness.state_mut().show_explorer = false;
    harness.frames(2);
    harness
}

/// Forty documents, each in a tab of its own with the last in front: more
/// tabs than the window is wide.
fn many_tabs() -> LabelHarness {
    let names: Vec<String> = (1..=40).map(|n| format!("doc-{n:02}.md")).collect();
    let docs: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), "Doc.")).collect();
    let mut harness = LabelHarness::with_docs(&docs);
    let rest: Vec<PathBuf> = names[1..].iter().map(|name| harness.path(name)).collect();
    let app = harness.state_mut();
    app.show_explorer = false;
    for path in rest {
        app.open_in_new_tab(path);
    }
    harness.frames(2);
    harness
}

fn open_paths(harness: &LabelHarness) -> Vec<PathBuf> {
    let tabs = &harness.state().tabs;
    tabs.iter().map(|tab| tab.path.clone()).collect()
}

fn active_path(harness: &LabelHarness) -> PathBuf {
    let app = harness.state();
    app.tabs[app.active_tab].path.clone()
}

fn open_context_menu(harness: &mut LabelHarness, tab: &str) {
    harness.get_by_label(tab).click_secondary();
    harness.frames(2);
}

#[test]
fn clicking_a_tab_shows_its_document() {
    let mut harness = three_tabs();
    assert_eq!(active_path(&harness), harness.path("c.md"));

    harness.click("a.md");

    assert_eq!(active_path(&harness), harness.path("a.md"));
    assert_eq!(open_paths(&harness).len(), 3);
}

#[test]
fn middle_clicking_a_tab_closes_it() {
    let mut harness = three_tabs();

    harness
        .get_by_label("b.md")
        .click_button(PointerButton::Middle);
    harness.frames(2);

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("c.md")]
    );
}

/// Whether the close button of the tab named `tab` is drawn: something
/// that is neither text nor a box is painted inside it.
fn close_is_drawn(harness: &LabelHarness, tab: &str) -> bool {
    let button = harness.get_by_label(&format!("Close {tab}")).rect();
    let room = button.expand(0.5);
    painted_shapes(harness.output()).iter().any(|shape| {
        !matches!(shape, egui::Shape::Text(_) | egui::Shape::Rect(_))
            && room.contains_rect(shape.visual_bounding_rect())
    })
}

#[test]
fn a_tab_under_the_pointer_shows_a_close_button_until_the_pointer_leaves() {
    let drawn =
        |harness: &LabelHarness| ["a.md", "b.md", "c.md"].map(|tab| close_is_drawn(harness, tab));
    let mut harness = three_tabs();
    assert_eq!(drawn(&harness), [false, false, false]);

    harness.get_by_label("a.md").hover();
    harness.frames(2);
    assert_eq!(drawn(&harness), [true, false, false]);

    harness.event(Event::PointerMoved(egui::pos2(600.0, 500.0)));
    harness.frames(2);
    assert_eq!(drawn(&harness), [false, false, false]);
}

#[test]
fn the_close_button_takes_the_ink_of_the_text_under_the_pointer_and_is_muted_beside_it() {
    let ink = |harness: &LabelHarness| {
        let button = harness.get_by_label("Close a.md").rect().expand(0.5);
        let strokes = painted_shapes(harness.output()).into_iter();
        let mut inks = strokes.filter_map(|shape| match shape {
            egui::Shape::Path(path) if button.contains_rect(path.visual_bounding_rect()) => {
                Some(path.stroke.color)
            }
            _ => None,
        });
        inks.next().expect("the close button is drawn in strokes")
    };
    let mut harness = three_tabs();
    let palette = harness.state().theme.palette();

    harness.get_by_label("a.md").hover();
    harness.frames(2);
    let solid = egui::epaint::ColorMode::Solid;
    assert_eq!(ink(&harness), solid(palette.muted), "the tab is pointed at");

    harness.get_by_label("Close a.md").hover();
    harness.frames(2);
    assert_eq!(ink(&harness), solid(palette.text), "the button is");
}

#[test]
fn the_close_button_of_a_hovered_tab_closes_that_tab() {
    let mut harness = three_tabs();
    harness.get_by_label("a.md").hover();
    harness.frames(2);

    harness.click("Close a.md");

    assert_eq!(
        open_paths(&harness),
        [harness.path("b.md"), harness.path("c.md")]
    );
    assert_eq!(active_path(&harness), harness.path("c.md"));
}

#[test]
fn closing_the_active_tab_shows_the_one_before_it() {
    let mut harness = three_tabs();
    harness.get_by_label("c.md").hover();
    harness.frames(2);

    harness.click("Close c.md");

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("b.md")]
    );
    assert_eq!(active_path(&harness), harness.path("b.md"));
}

/// Whether the keyboard is on what is labelled `label`.
fn keyboard_is_on(harness: &LabelHarness, label: &str) -> bool {
    harness.get_by_label(label).accesskit_node().is_focused()
}

/// Press the Tab key until the keyboard is on what is labelled `label`.
#[track_caller]
fn tab_to(harness: &mut LabelHarness, label: &str) {
    for _ in 0..40 {
        if keyboard_is_on(harness, label) {
            return;
        }
        harness.key_press(Key::Tab);
        harness.frames(2);
    }
    panic!("the Tab key never reached {label}");
}

#[test]
fn the_tab_key_stops_on_a_tabs_close_button_right_after_the_tab() {
    let mut harness = three_tabs();
    tab_to(&mut harness, "a.md");
    assert!(close_is_drawn(&harness, "a.md"), "shown for the keyboard");

    harness.key_press(Key::Tab);
    harness.frames(2);
    assert!(keyboard_is_on(&harness, "Close a.md"));
    assert!(close_is_drawn(&harness, "a.md"), "and while it is on it");
    assert!(!close_is_drawn(&harness, "b.md"));

    harness.key_press(Key::Tab);
    harness.frames(2);
    assert!(keyboard_is_on(&harness, "b.md"), "then the next tab");
    assert!(!close_is_drawn(&harness, "a.md"));
}

#[test]
fn enter_on_a_close_button_the_keyboard_is_on_closes_its_tab() {
    let mut harness = three_tabs();
    tab_to(&mut harness, "Close b.md");

    harness.key_press(Key::Enter);
    harness.frames(2);

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("c.md")]
    );
    assert_eq!(active_path(&harness), harness.path("c.md"));
}

#[test]
fn the_ring_of_a_tab_the_keyboard_is_on_is_as_tall_as_a_button_and_clear_of_the_underline() {
    let rings = |harness: &LabelHarness| -> Vec<egui::Rect> {
        let accent = harness.state().theme.palette().accent;
        let shapes = painted_shapes(harness.output()).into_iter();
        let outlines = shapes.filter_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if rect.fill == egui::Color32::TRANSPARENT && rect.stroke.color == accent =>
            {
                Some(rect.rect)
            }
            _ => None,
        });
        outlines.collect()
    };
    let mut harness = three_tabs();
    assert!(rings(&harness).is_empty(), "nothing has the keyboard");
    harness.get_by_label("c.md").hover();
    harness.frames(2);
    assert!(rings(&harness).is_empty(), "the pointer draws no ring");

    tab_to(&mut harness, "c.md");

    let [ring] = rings(&harness)[..] else {
        panic!("one ring: {:?}", rings(&harness));
    };
    let tab = harness.get_by_label("c.md").rect();
    assert_close(ring.height(), BUTTON_SIDE);
    assert_close(ring.center().y, tab.center().y);
    assert_close(ring.left(), tab.left());
    assert_close(ring.right(), tab.right());
    assert!(ring.bottom() < tab.bottom() - UNDERLINE, "above the line");
}

#[test]
fn the_tabs_keep_their_places_when_a_close_button_appears() {
    let lefts = |harness: &LabelHarness| -> Vec<f32> {
        ["a.md", "b.md", "c.md"]
            .map(|tab| harness.get_by_label(tab).rect().left())
            .into()
    };
    let mut harness = three_tabs();
    let at_rest = lefts(&harness);

    harness.get_by_label("a.md").hover();
    harness.frames(2);

    assert_eq!(lefts(&harness), at_rest);
    let (tab, close) = (
        harness.get_by_label("a.md").rect(),
        harness.get_by_label("Close a.md").rect(),
    );
    assert!(tab.contains_rect(close), "{close:?} inside {tab:?}");
}

#[test]
fn the_context_menu_closes_the_tab_it_was_opened_on() {
    let mut harness = three_tabs();

    open_context_menu(&mut harness, "a.md");
    harness.click("Close");

    assert_eq!(
        open_paths(&harness),
        [harness.path("b.md"), harness.path("c.md")]
    );
}

#[test]
fn close_others_keeps_only_the_tab_it_was_opened_on() {
    let mut harness = three_tabs();

    open_context_menu(&mut harness, "b.md");
    harness.click("Close Others");

    let kept = harness.path("b.md");
    assert_eq!(open_paths(&harness), std::slice::from_ref(&kept));
    assert_eq!(active_path(&harness), kept);
    assert_eq!(harness.state().open_tab_paths, HashSet::from([kept]));
}

#[test]
fn close_others_stops_watching_the_closed_documents() {
    let mut harness = three_tabs();
    harness.key_press(Key::F5);
    harness.frames(2);
    assert_eq!(harness.state().watched_files().len(), 3);

    open_context_menu(&mut harness, "b.md");
    harness.click("Close Others");

    assert_eq!(
        *harness.state().watched_files(),
        HashSet::from([harness.path("b.md")])
    );
}

/// Three tabs with the find bar open on the first: `a.md` holds the word
/// looked for once, `b.md` holds it twice and `c.md` not at all.
fn finding_needle() -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[
        ("a.md", "needle"),
        ("b.md", "needle needle"),
        ("c.md", "None here."),
    ]);
    let (b, c) = (harness.path("b.md"), harness.path("c.md"));
    let app = harness.state_mut();
    app.show_explorer = false;
    app.open_in_new_tab(b);
    app.open_in_new_tab(c);
    app.focus_tab(0);
    app.open_search();
    app.search.query = "needle".to_owned();
    harness.frames(2);
    assert!(harness.query_by_label("1 / 1").is_some());
    harness
}

/// Whether the last frame asked for another one at once.
fn asks_for_a_frame(harness: &LabelHarness) -> bool {
    harness.output().viewport_output[&egui::ViewportId::ROOT]
        .repaint_delay
        .is_zero()
}

/// Run the two frames egui asks for after a frame that had input, and no
/// more: the window draws nothing further unless something asks.
fn run_the_frames_asked_for(harness: &mut LabelHarness) {
    for _ in 0..2 {
        assert!(asks_for_a_frame(harness));
        harness.step();
    }
}

#[test]
fn the_find_bar_counts_again_in_the_frames_a_close_is_followed_by() {
    let mut harness = finding_needle();

    harness
        .get_by_label("a.md")
        .click_button(PointerButton::Middle);
    // One frame for each event of the click; the last one closes the tab.
    harness.step();
    assert_eq!(open_paths(&harness).len(), 2);
    run_the_frames_asked_for(&mut harness);
    assert!(harness.query_by_label("1 / 2").is_some(), "after a close");

    harness.state_mut().focus_tab(1);
    harness.frames(6);
    assert!(
        harness.query_by_label("1 / 2").is_none(),
        "c.md is in front"
    );
    open_context_menu(&mut harness, "b.md");
    harness.get_by_label("Close Others").click();
    harness.step();
    assert_eq!(open_paths(&harness), [harness.path("b.md")]);
    run_the_frames_asked_for(&mut harness);
    assert!(
        harness.query_by_label("1 / 2").is_some(),
        "after Close Others"
    );
}

#[test]
fn close_others_counts_the_matches_of_the_document_it_keeps() {
    let mut harness = finding_needle();

    open_context_menu(&mut harness, "b.md");
    harness.click("Close Others");
    // The matches are found again in the frame after the close and
    // counted in the one after that.
    harness.frames(1);

    assert!(harness.query_by_label("1 / 2").is_some());
}

#[test]
fn every_tab_fills_the_height_of_the_bar() {
    let harness = three_tabs();

    let open = harness.get_by_label("New tab").rect();
    for tab in ["a.md", "b.md", "c.md"] {
        let rect = harness.get_by_label(tab).rect();
        assert_close(rect.height(), BAR_HEIGHT);
        assert_close(rect.top(), 0.0);
        assert_close(rect.center().y, open.center().y);
    }
}

/// The text the last frame painted that reads `text`, in the top row.
#[track_caller]
pub(crate) fn strip_text(harness: &LabelHarness, text: &str) -> egui::epaint::TextShape {
    let strip = 0.0..=BAR_HEIGHT;
    let found = painted_shapes(harness.output())
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(shape)
                if shape.galley.text() == text && strip.contains(&shape.pos.y) =>
            {
                Some(shape)
            }
            _ => None,
        });
    found.expect("the strip did not paint the text")
}

#[test]
fn the_active_tab_is_dark_and_underlined_in_the_accent_and_the_rest_are_muted() {
    let harness = three_tabs();
    let palette = harness.state().theme.palette();

    assert_eq!(strip_text(&harness, "c.md").fallback_color, palette.text);
    assert_eq!(strip_text(&harness, "a.md").fallback_color, palette.muted);
    assert_eq!(strip_text(&harness, "b.md").fallback_color, palette.muted);

    let underlines: Vec<egui::Rect> = painted_shapes(harness.output())
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.fill == palette.accent => Some(rect.rect),
            _ => None,
        })
        .collect();
    let [line] = underlines[..] else {
        panic!("one underline: {underlines:?}");
    };
    let title = strip_text(&harness, "c.md");
    assert_close(line.height(), UNDERLINE);
    assert_close(line.left(), title.pos.x);
    assert_close(line.width(), title.galley.size().x);
    assert_close(line.bottom(), BAR_HEIGHT);
}

#[test]
fn a_muted_tab_darkens_under_the_pointer() {
    let mut harness = three_tabs();
    let palette = harness.state().theme.palette();

    harness.get_by_label("a.md").hover();
    harness.frames(2);

    assert_eq!(strip_text(&harness, "a.md").fallback_color, palette.text);
    assert_eq!(strip_text(&harness, "b.md").fallback_color, palette.muted);
}

#[test]
fn a_tab_tells_assistive_technology_whether_it_is_the_one_shown() {
    let harness = three_tabs();
    let shown = |tab| harness.get_by_label(tab).accesskit_node().toggled();

    assert_eq!(shown("c.md"), Some(Toggled::True));
    assert_eq!(shown("a.md"), Some(Toggled::False));
}

#[test]
fn the_plus_button_shows_the_home_page_and_leaves_no_tab_in_front() {
    let mut harness = three_tabs();
    harness.state_mut().file_picker = Box::new(|_| panic!("no dialog is asked for"));

    harness.click("New tab");

    assert_eq!(open_paths(&harness).len(), 3);
    assert_eq!(harness.state().front(), None);
    for tab in ["a.md", "b.md", "c.md"] {
        let shown = harness.get_by_label(tab).accesskit_node().toggled();
        assert_eq!(shown, Some(Toggled::False), "{tab}");
    }
    let accent = harness.state().theme.palette().accent;
    let underlined = painted_shapes(harness.output())
        .into_iter()
        .any(|shape| matches!(shape, egui::Shape::Rect(rect) if rect.fill == accent));
    assert!(!underlined, "no tab is underlined");
    assert!(
        harness.query_by_label("Preview").is_none(),
        "no edit switch"
    );

    harness.click("b.md");
    assert_eq!(active_path(&harness), harness.path("b.md"));
    assert_eq!(harness.state().front(), Some(1));
}

#[test]
fn with_more_tabs_than_fit_the_open_button_stops_short_of_the_mode_toggle() {
    let harness = many_tabs();

    let open = harness.get_by_label("New tab").rect();
    let toggle = harness.get_by_label("Preview").rect();

    assert!(
        open.right() <= toggle.left(),
        "the button at {open:?} runs into the toggle at {toggle:?}"
    );
}

/// Whether the tab named `name` is wholly on screen, before the open-file
/// button that ends the strip.
fn in_strip(harness: &LabelHarness, name: &str) -> bool {
    let tab = harness.get_by_label(name).rect();
    let strip_end = harness.get_by_label("New tab").rect().left();
    tab.left() >= 0.0 && tab.right() <= strip_end
}

#[test]
fn a_tab_that_comes_to_the_front_is_scrolled_into_the_strip() {
    let mut harness = many_tabs();
    harness.frames(30);
    assert!(in_strip(&harness, "doc-40.md"), "the document just opened");

    harness.state_mut().focus_tab(0);
    harness.frames(30);
    assert!(in_strip(&harness, "doc-01.md"), "the first, switched to");
    assert!(!in_strip(&harness, "doc-40.md"));
}

#[test]
fn a_tab_that_takes_the_place_of_a_closed_one_is_scrolled_into_the_strip() {
    let mut harness = many_tabs();
    harness.state_mut().focus_tab(0);
    harness.frames(30);
    let over_the_strip = harness.get_by_label("doc-01.md").rect().center();
    harness.event(Event::PointerMoved(over_the_strip));
    harness.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(-4000.0, 0.0),
        modifiers: Modifiers::NONE,
    });
    harness.frames(30);
    assert!(
        !in_strip(&harness, "doc-02.md"),
        "scrolled out of the strip"
    );

    harness.state_mut().close_tab(0);
    harness.frames(30);

    assert_eq!(active_path(&harness), harness.path("doc-02.md"));
    assert!(in_strip(&harness, "doc-02.md"), "the document now in front");
}

#[test]
fn a_second_tab_on_the_same_document_is_scrolled_into_the_strip_too() {
    let mut harness = many_tabs();
    let first = harness.path("doc-01.md");
    let last = &mut harness.state_mut().tabs[39];
    assert!(last.navigate_to_path(&first).unwrap());
    harness.frames(30);

    harness.state_mut().focus_tab(0);
    harness.frames(30);

    let strip_end = harness.get_by_label("New tab").rect().left();
    let tab = harness
        .query_all_by_label("doc-01.md")
        .next()
        .unwrap()
        .rect();
    assert!(
        tab.left() >= 0.0 && tab.right() <= strip_end,
        "the first tab is at {tab:?}"
    );
}

#[test]
fn the_strip_stays_where_the_reader_scrolled_it() {
    let mut harness = many_tabs();
    harness.frames(30);
    let over_the_strip = harness.get_by_label("doc-40.md").rect().center();

    harness.event(Event::PointerMoved(over_the_strip));
    harness.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(2000.0, 0.0),
        modifiers: Modifiers::NONE,
    });
    harness.frames(30);

    assert!(
        !in_strip(&harness, "doc-40.md"),
        "the tab in front is not pulled back into view"
    );
}

#[test]
fn one_document_has_a_tab_and_the_new_tab_button_right_after_it() {
    let harness = one_tab("Body.");

    let tab = harness.get_by_label("a.md").rect();
    let open = harness.get_by_label("New tab").rect();
    let forward = harness.get_by_label("Go forward").rect();
    assert!(forward.right() <= tab.left(), "{forward:?} then {tab:?}");
    assert_close(
        open.left(),
        tab.right() + harness.ctx.pixels_per_point().recip(),
    );
}

#[test]
fn with_no_document_the_new_tab_button_follows_the_history_arrows() {
    let mut harness = one_tab("Body.");
    harness.state_mut().close_tab(0);
    harness.frames(2);

    assert!(harness.query_by_label("a.md").is_none());
    let open = harness.get_by_label("New tab").rect();
    let forward = harness.get_by_label("Go forward").rect();
    assert!(open.left() >= forward.right());
    assert!(open.left() - forward.right() < 10.0, "{open:?}");
}

#[test]
fn a_second_document_moves_nothing_under_the_bar() {
    let mut harness = one_tab("# A\n\n## B");
    harness.state_mut().show_outline = true;
    harness.frames(2);
    let top = |harness: &LabelHarness| harness.get_by_label("CONTENTS").rect().top();
    let at_rest = top(&harness);

    let other = harness.dir.join("b.md");
    std::fs::write(&other, "# Other\n\n## More").unwrap();
    harness.state_mut().open_in_new_tab(other);
    harness.frames(2);

    assert_eq!(open_paths(&harness).len(), 2);
    assert_close(top(&harness), at_rest);
}

#[test]
fn a_short_name_gets_a_tab_wide_enough_to_hit_and_a_long_one_is_cut_short() {
    let long = format!("{}.md", "a-very-long-file-name".repeat(6));
    let mut harness = LabelHarness::with_docs(&[("i", "Body."), (&long, "Body.")]);
    let other = harness.path(&long);
    let app = harness.state_mut();
    app.show_explorer = false;
    app.open_in_new_tab(other);
    harness.frames(2);

    let chrome = 2.0_f32.mul_add(TAB_PADDING, CLOSE_SIDE);
    let short = harness.get_by_label("i").rect();
    assert_close(short.width(), chrome + NAME_WIDTH.min);
    let cut = harness.get_by_label(&long).rect();
    assert!(cut.width() <= chrome + NAME_WIDTH.max, "{cut:?}");
    let painted = painted_shapes(harness.output())
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if text.galley.text().starts_with("a-very") => Some(text),
            _ => None,
        })
        .unwrap();
    assert!(painted.galley.elided, "it ends in an ellipsis");
}

#[test]
fn pointing_at_a_tab_says_where_the_file_is_and_whether_it_is_live() {
    let mut harness = three_tabs();
    let path = harness.path("a.md").display().to_string();
    assert!(harness.query_by_label(&path).is_none());

    harness.get_by_label("a.md").hover();
    harness.frames(8);
    assert!(harness.query_by_label(&path).is_some());
    assert!(harness.query_by_label_contains("Live").is_none());

    harness.key_press(Key::F5);
    harness.frames(8);
    assert!(harness.query_by_label(&path).is_some());
    assert!(harness
        .query_by_label("Live: reloads when the file changes")
        .is_some());
}

#[test]
fn a_frame_that_picks_a_tab_and_closes_another_does_both() {
    let mut harness = three_tabs();
    let picks = super::TabPicks {
        activate: Some(0),
        close: Some(1),
        close_others: None,
    };

    assert!(harness.state_mut().apply_tab_picks(&picks));

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("c.md")]
    );
    assert_eq!(active_path(&harness), harness.path("a.md"));
    let nothing = super::TabPicks::default();
    assert!(!harness.state_mut().apply_tab_picks(&nothing));
    assert_eq!(open_paths(&harness).len(), 2);
}

#[test]
fn a_tab_is_named_by_its_documents_first_heading_and_has_the_role_of_a_tab() {
    let mut harness = LabelHarness::with_docs(&[
        ("notes.md", "# Field Notes\n\nBody."),
        ("plain.md", "No heading here."),
    ]);
    let plain = harness.path("plain.md");
    let app = harness.state_mut();
    app.show_explorer = false;
    app.show_outline = false;
    app.open_in_new_tab(plain);
    harness.frames(2);

    let tabs: Vec<String> = harness
        .query_all_by_role(egui::accesskit::Role::Tab)
        .filter_map(|tab| tab.accesskit_node().label())
        .collect();
    assert_eq!(tabs, ["Field Notes", "plain.md"]);
    assert_eq!(
        strip_text(&harness, "Field Notes").galley.text(),
        "Field Notes"
    );
}
