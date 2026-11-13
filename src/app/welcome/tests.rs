//! The home page: what it shows of the recently read documents, and what
//! a click on it asks for.

use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

use eframe::egui::accesskit::Role;
use eframe::egui::{Color32, Rect, Shape};
use eframe::Storage as _;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::app::content::tests::label_harness::LabelHarness;
use crate::app::ui_harness::{launch, MemoryStorage};
use crate::layout::READING_COLUMN_MARGIN;
use crate::persist::APP_KEY;
use crate::recent::RecentEntry;
use crate::test_support::{assert_close, painted_shapes};
use crate::theme::ThemeId;

const ESSAY: &str = "# On Reading Slowly\n\nA page read twice is worth two pages read once. \
                     This is a note on why, and on how to make the time for it.\n\n\
                     ## Why\n\nBecause attention is the whole of it.\n";

/// The app with every document closed, so the home page shows. Each
/// `(name, content)` document was opened and closed again, first to last,
/// so the last one leads the list.
fn home(docs: &[(&str, &str)]) -> LabelHarness {
    let mut harness = LabelHarness::with_docs(docs);
    let paths: Vec<_> = docs.iter().map(|(name, _)| harness.path(name)).collect();
    let app = harness.state_mut();
    for path in paths {
        app.open_in_new_tab(path);
    }
    while !app.tabs.is_empty() {
        app.close_active_tab();
    }
    harness.frames(2);
    harness
}

/// The home page of an app that has read nothing.
fn empty_home() -> LabelHarness {
    home(&[])
}

/// Every text the last frame painted, with its color and where it is.
fn painted_texts(harness: &LabelHarness) -> Vec<(String, Color32, Rect)> {
    let shapes = painted_shapes(harness.output());
    let texts = shapes.iter().filter_map(|shape| match shape {
        Shape::Text(text) => Some(text),
        _ => None,
    });
    texts
        .map(|text| {
            let color = text.galley.job.sections[0].format.color;
            let rect = Rect::from_min_size(text.pos, text.galley.size());
            (text.galley.text().to_owned(), color, rect)
        })
        .collect()
}

/// The color and place of the one painted text that starts with `prefix`.
#[track_caller]
fn painted(harness: &LabelHarness, prefix: &str) -> (Color32, Rect) {
    let texts = painted_texts(harness);
    let mut found = texts.iter().filter(|(text, _, _)| text.starts_with(prefix));
    let (_, color, rect) = found
        .next()
        .unwrap_or_else(|| panic!("nothing painted starts with {prefix:?}"));
    assert!(found.next().is_none(), "{prefix:?} is painted twice");
    (*color, *rect)
}

/// The color and place of the wordmark. The scratch folder's name starts
/// like it, so it is looked up whole.
#[track_caller]
fn painted_wordmark(harness: &LabelHarness) -> (Color32, Rect) {
    let texts = painted_texts(harness);
    let found = texts.iter().find(|(text, _, _)| text == WORDMARK);
    let (_, color, rect) = found.expect("the wordmark is not painted");
    (*color, *rect)
}

/// Where on screen the baseline of the painted text `text` is.
fn first_baseline(harness: &LabelHarness, text: &str) -> f32 {
    let shapes = painted_shapes(harness.output());
    let found = shapes.iter().find_map(|shape| match shape {
        Shape::Text(painted) if painted.galley.text() == text => Some(painted),
        _ => None,
    });
    let painted = found.unwrap();
    let line = &painted.galley.rows[0];
    painted.pos.y + line.pos.y + line.glyphs[0].pos.y
}

fn is_painted(harness: &LabelHarness, prefix: &str) -> bool {
    let texts = painted_texts(harness);
    texts.iter().any(|(text, _, _)| text.starts_with(prefix))
}

/// The filled rectangles of the last frame that have `fill`.
fn washes(harness: &LabelHarness, fill: Color32) -> Vec<Rect> {
    let shapes = painted_shapes(harness.output());
    let rects = shapes.iter().filter_map(|shape| match shape {
        Shape::Rect(rect) if rect.fill == fill => Some(rect.rect),
        _ => None,
    });
    rects.collect()
}

fn palette(harness: &LabelHarness) -> &'static Palette {
    harness.state().theme.palette()
}

#[test]
fn a_read_document_is_listed_by_its_title_with_where_it_is_and_how_long_it_reads() {
    let harness = home(&[("essay.md", ESSAY)]);
    let folder = harness.dir.file_name().unwrap().to_str().unwrap();

    assert!(harness
        .query_by_role_and_label(Role::Button, "On Reading Slowly")
        .is_some());
    assert!(is_painted(&harness, "RECENTLY READ"));
    assert!(is_painted(
        &harness,
        &format!("{folder} · 1 min read · opened just now")
    ));
    assert!(is_painted(
        &harness,
        "A page read twice is worth two pages read once."
    ));
}

#[test]
fn a_row_is_set_in_the_document_serif_with_a_sans_meta_line() {
    let harness = home(&[("essay.md", ESSAY)]);
    let palette = palette(&harness);
    let font_of = |prefix: &str| {
        let shapes = painted_shapes(harness.output());
        let font = shapes.iter().find_map(|shape| match shape {
            Shape::Text(text) if text.galley.text().starts_with(prefix) => {
                Some(text.galley.job.sections[0].format.font_id.clone())
            }
            _ => None,
        });
        font.unwrap()
    };
    let body = harness.state().document_look.style.text_styles[&TextStyle::Body].clone();
    let heading = harness.state().document_look.style.text_styles[&TextStyle::Heading].clone();

    let (title, excerpt, meta) = (
        font_of("On Reading Slowly"),
        font_of("A page read twice"),
        font_of(harness.dir.file_name().unwrap().to_str().unwrap()),
    );

    assert_eq!(title.family, body.family);
    assert_eq!(excerpt.family, body.family);
    assert_eq!(meta.family, FontFamily::Proportional);
    let shapes = painted_shapes(harness.output());
    let wordmark_font = shapes.iter().find_map(|shape| match shape {
        Shape::Text(text) if text.galley.text() == WORDMARK => {
            Some(text.galley.job.sections[0].format.font_id.clone())
        }
        _ => None,
    });
    assert_eq!(wordmark_font.unwrap().family, heading.family);
    assert!(title.size > excerpt.size && excerpt.size > meta.size);
    assert_eq!(painted(&harness, "On Reading Slowly").0, palette.heading);
    assert_eq!(painted(&harness, "A page read twice").0, palette.text);
    assert_eq!(painted(&harness, "RECENTLY READ").0, palette.muted);
}

#[test]
fn an_excerpt_is_held_to_two_lines() {
    let long = format!("# Long\n\n{}\n", "word ".repeat(200));
    let harness = home(&[("long.md", &long)]);

    let shapes = painted_shapes(harness.output());
    let excerpt = shapes.iter().find_map(|shape| match shape {
        Shape::Text(text) if text.galley.text().starts_with("word word") => Some(text),
        _ => None,
    });
    let excerpt = excerpt.unwrap();

    assert_eq!(excerpt.galley.rows.len(), 2);
    assert!(excerpt.galley.elided);
}

#[test]
fn a_document_without_a_title_or_a_paragraph_is_listed_by_its_file_name() {
    let harness = home(&[("shopping-list.md", "- eggs\n- flour\n")]);

    assert!(harness
        .query_by_role_and_label(Role::Button, "shopping-list")
        .is_some());
}

#[test]
fn the_most_recently_read_document_leads_the_list() {
    let harness = home(&[("a.md", "# First"), ("b.md", "# Second")]);

    let first = painted(&harness, "First").1;
    let second = painted(&harness, "Second").1;

    assert!(second.top() < first.top());
}

#[test]
fn rows_are_parted_by_hairlines_across_the_column() {
    let harness = home(&[("a.md", "# First"), ("b.md", "# Second")]);
    let rule = palette(&harness).rule;
    let wordmark = painted_wordmark(&harness).1;

    let shapes = painted_shapes(harness.output());
    let hairlines: Vec<_> = shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { points, stroke } if stroke.color == rule => Some(*points),
            _ => None,
        })
        .filter(|points| (points[0].x - wordmark.left()).abs() < 0.5)
        .collect();

    assert_eq!(hairlines.len(), 2, "one above each row");
    for [from, to] in hairlines {
        assert_close(from.x, wordmark.left());
        assert_close(to.x - from.x, 680.0);
    }
}

#[test]
fn clicking_a_row_opens_its_document_in_a_tab() {
    let mut harness = home(&[("a.md", "# First"), ("b.md", "# Second")]);
    assert!(harness.state().tabs.is_empty());

    harness.click("First");

    let tabs = &harness.state().tabs;
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].path, harness.path("a.md"));
}

#[test]
fn the_row_under_the_pointer_is_washed_from_edge_to_edge() {
    let mut harness = home(&[("a.md", "# First"), ("b.md", "# Second")]);
    let raised = palette(&harness).raised;
    assert!(washes(&harness, raised).is_empty());

    harness.get_by_label("First").hover();
    harness.frames(2);

    let row = harness.get_by_label("First").rect();
    let washes = washes(&harness, raised);
    assert_eq!(washes.len(), 1);
    assert!(washes[0].left() < row.left() && washes[0].right() > row.right());
    assert!(washes[0].contains(row.center()));
    assert!(!washes[0].contains(harness.get_by_label("Second").rect().center()));
}

#[test]
fn resting_the_pointer_on_a_row_says_where_its_file_is() {
    let mut harness = home(&[("a.md", "# First")]);
    let path = harness.path("a.md").display().to_string();
    assert!(harness.query_by_label(&path).is_none());

    harness.get_by_label("First").hover();
    harness.frames(40);

    assert!(harness.query_by_label(&path).is_some());
}

#[test]
fn a_document_that_is_gone_is_listed_muted_and_cannot_be_opened() {
    let mut harness = home(&[
        ("a.md", "# First\n\nStill here."),
        ("b.md", "# Gone\n\nIt was here."),
    ]);
    fs::remove_file(harness.path("b.md")).unwrap();
    harness.state_mut().home_list = None;
    harness.frames(2);
    let palette = palette(&harness);
    let folder = harness
        .dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    assert!(is_painted(
        &harness,
        &format!("{folder} · 1 min read · file not found")
    ));
    assert_eq!(painted(&harness, "Gone").0, palette.muted);
    assert_eq!(painted(&harness, "It was here.").0, palette.muted);
    assert_eq!(painted(&harness, "First").0, palette.heading);
    assert!(harness.get_by_label("Gone").accesskit_node().is_disabled());

    harness.get_by_label("Gone").hover();
    harness.frames(2);
    assert!(washes(&harness, palette.raised).is_empty());
    harness.click("Gone");

    assert!(harness.state().tabs.is_empty());
    assert_eq!(harness.state().error_message, None);
}

#[test]
fn the_list_shows_six_documents_until_it_is_expanded() {
    let docs: Vec<(String, String)> = (1..=7)
        .map(|n| (format!("{n}.md"), format!("# Doc {n}")))
        .collect();
    let docs: Vec<(&str, &str)> = docs
        .iter()
        .map(|(name, content)| (name.as_str(), content.as_str()))
        .collect();
    let mut harness = home(&docs);
    // Tall enough to show the whole list and the control under it.
    harness.set_size(egui::vec2(1200.0, 1400.0));
    harness.frames(2);
    // The first one opened is the seventh in the list.
    assert!(harness.query_by_label("Doc 2").is_some());
    assert!(harness.query_by_label("Doc 1").is_none());

    harness.click("Show more…");
    assert!(harness.query_by_label("Doc 1").is_some());

    harness.click("Show less");
    assert!(harness.query_by_label("Doc 1").is_none());
}

#[test]
fn a_list_that_fits_has_nothing_to_expand() {
    let harness = home(&[("a.md", "# First")]);

    assert!(harness.query_by_label("Show more…").is_none());
}

#[test]
fn with_nothing_read_the_page_invites_the_reader_to_open_something() {
    let harness = empty_home();

    assert!(is_painted(&harness, "Nothing read yet."));
    assert!(is_painted(
        &harness,
        "Open a Markdown file, or a folder of them"
    ));
    assert!(is_painted(&harness, "Ctrl+O opens a file."));
    assert!(!is_painted(&harness, "RECENTLY READ"));
    assert_eq!(painted_wordmark(&harness).0, palette(&harness).heading);
}

#[test]
fn the_open_buttons_ask_for_a_file_and_a_folder() {
    let mut harness = empty_home();
    let asked = Rc::new(RefCell::new(Vec::new()));
    let picker = |what: &'static str| {
        let asked = Rc::clone(&asked);
        Box::new(move |_: Option<&Path>| {
            asked.borrow_mut().push(what);
            Ok(None)
        })
    };
    harness.state_mut().file_picker = picker("file");
    harness.state_mut().folder_picker = picker("folder");

    harness.click("Open File…");
    harness.click("Open Folder…");

    assert_eq!(*asked.borrow(), ["file", "folder"]);
}

#[test]
fn an_open_button_is_muted_text_until_the_pointer_is_on_it() {
    let mut harness = empty_home();
    let palette = palette(&harness);
    assert_eq!(painted(&harness, "Open File…").0, palette.muted);
    assert!(washes(&harness, palette.raised).is_empty());

    harness.get_by_label("Open File…").hover();
    harness.frames(2);

    assert_eq!(painted(&harness, "Open File…").0, palette.text);
    assert_eq!(painted(&harness, "Open Folder…").0, palette.muted);
    let button = harness.get_by_label("Open File…").rect();
    assert_eq!(washes(&harness, palette.raised), [button]);
}

#[test]
fn the_page_keeps_to_the_column_a_document_would_have() {
    let mut harness = home(&[("a.md", "# First")]);
    harness.state_mut().show_explorer = false;
    harness.frames(2);
    let window = harness.ctx.content_rect();
    let left_of = |harness: &LabelHarness, text| painted(harness, text).1.left();

    // The default column is 680 px wide, centered in the window.
    let centered = ((window.width() - 680.0) / 2.0).floor();
    assert_close(painted_wordmark(&harness).1.left(), centered);
    assert_close(left_of(&harness, "First"), centered);
    assert_close(left_of(&harness, "RECENTLY READ"), centered);
    // The last label ends where the column does; its wash reaches past.
    let last_label = painted(&harness, "Open Folder…").1;
    assert!((last_label.right() - (centered + 680.0)).abs() < 1.0);

    harness.state_mut().reading_width = ReadingWidth::new(400.0);
    harness.frames(2);
    assert_close(
        painted_wordmark(&harness).1.left(),
        ((window.width() - 400.0) / 2.0).floor(),
    );

    // A column that fits the window has no width of its own.
    harness.state_mut().reading_width = ReadingWidth::FIT_WINDOW;
    harness.frames(2);
    assert_close(painted_wordmark(&harness).1.left(), centered);
}

#[test]
fn a_window_narrower_than_the_column_keeps_the_page_inside_its_margins() {
    let mut harness = home(&[("a.md", "# First")]);
    harness.state_mut().show_explorer = false;
    harness.set_size(egui::vec2(500.0, 700.0));
    harness.frames(2);

    let wordmark = painted_wordmark(&harness).1;
    let last_label = painted(&harness, "Open Folder…").1;

    assert_close(wordmark.left(), READING_COLUMN_MARGIN);
    assert!((last_label.right() - (500.0 - READING_COLUMN_MARGIN)).abs() < 1.0);
}

#[test]
fn the_open_buttons_sit_on_the_wordmarks_line_where_there_is_room() {
    let mut harness = empty_home();
    let wordmark = painted_wordmark(&harness).1;
    let button = harness.get_by_label("Open File…").rect();
    assert!(wordmark.y_range().contains(button.center().y));
    assert!(button.left() > wordmark.right());
    assert_close(
        first_baseline(&harness, "Open File…"),
        first_baseline(&harness, WORDMARK),
    );

    harness.state_mut().reading_width = ReadingWidth::new(ReadingWidth::MIN);
    harness.frames(2);

    let wordmark = painted_wordmark(&harness).1;
    let button = harness.get_by_label("Open File…").rect();
    assert!(button.top() >= wordmark.bottom(), "under the wordmark");
    // The label, not the button's padding, lines up with the wordmark.
    assert_close(painted(&harness, "Open File…").1.left(), wordmark.left());
}

#[test]
fn the_page_follows_the_theme() {
    let mut harness = home(&[("a.md", "# First")]);

    for theme in ThemeId::ALL {
        harness.state_mut().theme = theme;
        harness.frames(2);
        let palette = theme.palette();

        assert_eq!(painted(&harness, "First").0, palette.heading, "{theme:?}");
        assert_eq!(painted_wordmark(&harness).0, palette.heading);
        assert_eq!(painted(&harness, "Open File…").0, palette.muted);
    }
}

#[test]
fn a_recent_entry_from_an_older_session_is_listed_without_a_summary() {
    let mut harness = empty_home();
    let path = harness.dir.join("old-notes.md");
    fs::write(&path, "# Old notes").unwrap();
    let app = harness.state_mut();
    app.recent_files = vec![RecentEntry {
        path,
        last_opened: now_epoch_secs() - 2 * 60 * 60,
        title: None,
        excerpt: None,
        word_count: None,
    }];
    app.home_list = None;
    harness.frames(2);
    let folder = harness
        .dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    assert!(harness.query_by_label("old-notes").is_some());
    assert!(is_painted(&harness, &format!("{folder} · opened 2h ago")));
}

#[test]
fn what_the_list_says_of_a_document_survives_a_restart() {
    let mut harness = home(&[("essay.md", ESSAY)]);
    let mut storage = MemoryStorage::default();

    eframe::App::save(harness.state_mut(), &mut storage);
    let relaunched = launch(&storage, None, false);

    assert_eq!(relaunched.recent_files, harness.state().recent_files);
    let entry = &relaunched.recent_files[0];
    assert_eq!(entry.title.as_deref(), Some("On Reading Slowly"));
    assert!(entry.excerpt.as_ref().unwrap().starts_with("A page read"));
    assert_eq!(entry.word_count, Some(38));
}

#[test]
fn a_session_saved_before_documents_were_summarized_keeps_its_recent_files() {
    let mut storage = MemoryStorage::default();
    let saved = r#"(recent_files: Some([(path: "/notes/old.md", last_opened: 1700000000)]))"#;
    storage.set_string(APP_KEY, saved.to_owned());

    let app = launch(&storage, None, false);

    assert_eq!(
        app.recent_files,
        [RecentEntry {
            path: "/notes/old.md".into(),
            last_opened: 1_700_000_000,
            title: None,
            excerpt: None,
            word_count: None,
        }]
    );
}

#[test]
fn a_document_named_on_the_command_line_is_summarized_as_it_is_recorded() {
    let harness = LabelHarness::with_docs(&[("essay.md", ESSAY)]);

    let recents = &harness.state().recent_files;

    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0].title.as_deref(), Some("On Reading Slowly"));
    assert_eq!(recents[0].word_count, Some(38));
}

#[test]
fn following_a_link_records_the_document_it_leads_to() {
    let mut harness =
        LabelHarness::with_docs(&[("a.md", "# First\n\nOne."), ("b.md", "# Second\n\nTwo.")]);
    let b = harness.path("b.md");

    harness.state_mut().navigate_active_tab_to(&b);

    let recents = &harness.state().recent_files;
    assert_eq!(recents[0].title.as_deref(), Some("Second"));
    assert_eq!(recents[0].excerpt.as_deref(), Some("Two."));
    assert_eq!(recents[1].title.as_deref(), Some("First"));
}

#[test]
fn a_file_recorded_without_being_open_keeps_what_was_known_of_it() {
    let mut harness = home(&[("essay.md", ESSAY)]);
    let path = harness.path("essay.md");
    assert!(harness.state().home_list.is_some());

    harness.state_mut().record_recent(&path);

    let app = harness.state();
    assert_eq!(app.recent_files.len(), 1);
    assert_eq!(
        app.recent_files[0].title.as_deref(),
        Some("On Reading Slowly")
    );
    assert!(app.home_list.is_none(), "the page lists the files afresh");
}
