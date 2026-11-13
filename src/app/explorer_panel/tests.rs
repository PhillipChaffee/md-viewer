use std::time::Instant;

use eframe::egui::accesskit::{Role, Toggled};
use eframe::egui::{Key, Modifiers, OutputCommand, PointerButton};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::app::content::tests::label_harness::LabelHarness;
use crate::test_support::{assert_close, painted_shapes};
use crate::theme::ThemeId;

/// `a.md` open in the only tab, beside `b.md` and a folder `sub` that holds
/// `c.md` and, one level down, `deep/d.md`.
fn explorer() -> LabelHarness {
    let mut harness = LabelHarness::with_docs(&[
        ("a.md", "# A"),
        ("b.md", "# B"),
        ("sub/c.md", "# C"),
        ("sub/deep/d.md", "# D"),
    ]);
    // A fresh install starts with the explorer hidden.
    harness.state_mut().show_explorer = true;
    harness.frames(2);
    harness
}

/// Pick `item` from the menu behind the explorer's three dots.
fn pick(harness: &mut LabelHarness, item: &str) {
    harness.click("Explorer options");
    harness.click(item);
}

/// The color the explorer row `name` is written in.
fn ink(harness: &LabelHarness, name: &str) -> egui::Color32 {
    let row = explorer_row(harness, name).rect();
    let shapes = painted_shapes(harness.output());
    let mut inks = shapes.iter().filter_map(|shape| match shape {
        egui::Shape::Text(text) if text.galley.text() == name && row.contains(text.pos) => {
            Some(text.galley.job.sections[0].format.color)
        }
        _ => None,
    });
    inks.next_back().unwrap()
}

/// The explorer's row for `name`. The tab strip above the explorer carries
/// the same label while that document is open.
fn explorer_row<'a>(harness: &'a LabelHarness, name: &'a str) -> egui_kittest::Node<'a> {
    harness
        .get_all_by_label(name)
        .max_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .unwrap()
}

fn open_paths(harness: &LabelHarness) -> Vec<PathBuf> {
    let tabs = &harness.state().tabs;
    tabs.iter().map(|tab| tab.path.clone()).collect()
}

fn is_listed(harness: &LabelHarness, name: &str) -> bool {
    harness.query_by_label(name).is_some()
}

/// Picks `item` from the context menu of the explorer row `file`. Returns
/// the text the pick put on the clipboard.
fn copy_from_context_menu(harness: &mut LabelHarness, file: &str, item: &str) -> Vec<String> {
    explorer_row(harness, file).click_secondary();
    harness.frames(2);
    harness.get_by_label(item).click();
    // One round: the clipboard request belongs to the frame of the click.
    harness.frames(1);
    let commands = &harness.output().platform_output.commands;
    commands
        .iter()
        .filter_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Whether a translucent highlight is painted over the explorer row `name`.
fn is_highlighted(harness: &LabelHarness, name: &str) -> bool {
    let row = explorer_row(harness, name).rect().center();
    harness.output().shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Rect(rect)
            if rect.rect.contains(row) && (1..255).contains(&rect.fill.a()))
    })
}

#[test]
fn the_explorer_is_titled_with_its_folder() {
    let mut harness = explorer();
    let folder = harness
        .dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(is_listed(&harness, &format!("FILES {folder}")));

    harness.state_mut().file_explorer = crate::explorer::FileExplorer::default();
    harness.frames(1);

    assert!(is_listed(&harness, "FILES No folder"));
}

#[test]
fn hiding_the_explorer_removes_its_files_from_the_window() {
    let mut harness = explorer();
    assert!(is_listed(&harness, "b.md"));

    harness.state_mut().show_explorer = false;
    harness.frames(1);

    assert!(!is_listed(&harness, "b.md"));
}

#[test]
fn clicking_a_file_shows_it_in_the_active_tab() {
    let mut harness = explorer();

    harness.click("b.md");

    assert_eq!(open_paths(&harness), [harness.path("b.md")]);
}

#[test]
fn ctrl_clicking_a_file_opens_it_in_a_new_tab() {
    let mut harness = explorer();

    harness
        .get_by_label("b.md")
        .click_modifiers(Modifiers::CTRL);
    harness.frames(2);

    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md"), harness.path("b.md")]
    );
}

#[test]
fn middle_clicking_an_open_file_closes_its_tab() {
    let mut harness = explorer();

    explorer_row(&harness, "b.md").click_button(PointerButton::Middle);
    harness.frames(2);
    assert_eq!(
        open_paths(&harness),
        [harness.path("a.md")],
        "b.md is not open, so there is nothing to close"
    );

    explorer_row(&harness, "a.md").click_button(PointerButton::Middle);
    harness.frames(2);
    assert!(open_paths(&harness).is_empty());
}

#[test]
fn a_folder_opens_and_closes_from_its_row() {
    let mut harness = explorer();
    assert!(!is_listed(&harness, "c.md"));

    harness.click("sub");
    assert!(is_listed(&harness, "c.md"));
    harness.click("sub");
    assert!(!is_listed(&harness, "c.md"));
}

#[test]
fn an_opened_folder_is_watched_while_live_reload_is_on() {
    let mut harness = explorer();
    harness.key_press(Key::F5);
    harness.frames(2);
    let sub = harness.path("sub");
    assert!(!harness.state().watched_explorer_dirs().contains(&sub));

    harness.click("sub");
    assert!(harness.state().watched_explorer_dirs().contains(&sub));

    harness.click("sub");
    assert!(!harness.state().watched_explorer_dirs().contains(&sub));
}

#[test]
fn a_folder_left_open_in_the_last_session_lists_its_files() {
    let mut harness = explorer();
    let sub = harness.path("sub");

    let last_session = crate::explorer::FileExplorer::restored(
        SortOrder::default(),
        HashSet::from([sub]),
        Some(harness.dir.clone()),
    );
    harness.state_mut().file_explorer = last_session;
    harness.wait_for_explorer_scan();

    assert!(is_listed(&harness, "c.md"));
}

#[test]
fn expand_all_and_collapse_all_reach_every_folder() {
    let mut harness = explorer();

    pick(&mut harness, "Expand All Folders");
    assert!(is_listed(&harness, "c.md"));
    assert!(is_listed(&harness, "d.md"));

    pick(&mut harness, "Collapse All Folders");
    assert!(!is_listed(&harness, "c.md"));
    assert!(!is_listed(&harness, "d.md"));
}

#[test]
fn expand_all_and_collapse_all_keep_the_watched_folders_in_step() {
    let mut harness = explorer();
    harness.key_press(Key::F5);
    harness.frames(2);

    pick(&mut harness, "Expand All Folders");
    let watched = harness.state().watched_explorer_dirs();
    assert!(watched.contains(&harness.path("sub/deep")));

    pick(&mut harness, "Collapse All Folders");
    let watched = harness.state().watched_explorer_dirs();
    assert_eq!(watched, &HashSet::from([harness.dir.clone()]));
}

#[test]
fn refresh_lists_a_file_created_since_the_last_scan() {
    let mut harness = explorer();
    fs::write(harness.path("new.md"), "# New").unwrap();
    harness.frames(1);
    assert!(!is_listed(&harness, "new.md"));

    pick(&mut harness, "Refresh");
    harness.wait_for_explorer_scan();

    assert!(is_listed(&harness, "new.md"));
}

#[test]
fn picking_a_sort_order_reorders_the_files() {
    let mut harness = explorer();
    let top = |harness: &LabelHarness, name| explorer_row(harness, name).rect().top();
    assert!(top(&harness, "a.md") < top(&harness, "b.md"));

    pick(&mut harness, "Name Z-A");

    assert!(top(&harness, "b.md") < top(&harness, "a.md"));
}

#[test]
fn the_menu_marks_the_order_the_files_are_in() {
    let mut harness = explorer();
    let checked = |harness: &LabelHarness, order: &str| {
        let radio = harness.get_by_role_and_label(Role::RadioButton, order);
        radio.accesskit_node().toggled() == Some(Toggled::True)
    };

    harness.click("Explorer options");
    assert!(checked(&harness, "Name A-Z"));
    assert!(!checked(&harness, "Newest First"));
    harness.click("Newest First");
    assert!(
        harness.query_by_label("Newest First").is_none(),
        "a pick closes the menu"
    );

    harness.click("Explorer options");
    assert!(checked(&harness, "Newest First"));
    assert!(!checked(&harness, "Name A-Z"));
}

#[test]
fn the_file_being_read_is_inked_and_the_others_are_muted() {
    let mut harness = explorer();
    let palette = harness.state().theme.palette();
    assert_eq!(ink(&harness, "a.md"), palette.text);
    assert_eq!(ink(&harness, "b.md"), palette.muted);
    assert_eq!(ink(&harness, "sub"), palette.muted);

    harness.click("b.md");

    assert_eq!(ink(&harness, "b.md"), palette.text);
    assert_eq!(ink(&harness, "a.md"), palette.muted);
}

#[test]
fn the_explorer_is_a_surface_with_one_hairline_and_no_frame() {
    let harness = explorer();
    let palette = harness.state().theme.palette();
    let panel = egui::containers::panel::PanelState::load(&harness.ctx, "file_explorer".into());
    let panel = panel.unwrap().rect;

    let shapes = painted_shapes(harness.output());
    let fill = shapes.iter().find_map(|shape| match shape {
        egui::Shape::Rect(rect) if rect.rect == panel => Some((rect.fill, rect.stroke)),
        _ => None,
    });
    let edges: Vec<_> = shapes
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == palette.rule && (points[0].x - panel.right()).abs() < 1.0 =>
            {
                Some(points[1].x)
            }
            _ => None,
        })
        .collect();

    assert_eq!(fill, Some((palette.surface, egui::Stroke::NONE)));
    assert_eq!(edges, [panel.right() - 0.5]);
}

#[test]
fn nested_files_are_indented_under_their_folder() {
    let mut harness = explorer();
    pick(&mut harness, "Expand All Folders");
    let left = |name| {
        let shapes = painted_shapes(harness.output());
        let text = shapes.iter().find_map(|shape| match shape {
            egui::Shape::Text(text) if text.galley.text() == name => Some(text.pos.x),
            _ => None,
        });
        text.unwrap()
    };

    assert_close(left("c.md") - left("b.md"), sidebar::INDENT);
    assert_close(left("d.md") - left("c.md"), sidebar::INDENT);
    // A file's name lines up with the folder's beside it.
    assert_close(left("sub"), left("b.md"));
}

#[test]
fn the_context_menu_copies_the_path_the_uri_and_the_contents() {
    let mut harness = explorer();
    let path = harness.path("b.md").display().to_string();

    assert_eq!(
        copy_from_context_menu(&mut harness, "b.md", "Copy Path"),
        std::slice::from_ref(&path)
    );
    assert_eq!(
        copy_from_context_menu(&mut harness, "b.md", "Copy File URI"),
        [format!("file://{path}")]
    );
    assert_eq!(
        copy_from_context_menu(&mut harness, "b.md", "Copy Contents"),
        ["# B"]
    );
}

#[test]
fn copying_the_contents_of_a_file_that_is_gone_copies_nothing() {
    let mut harness = explorer();
    fs::remove_file(harness.path("b.md")).unwrap();

    let copied = copy_from_context_menu(&mut harness, "b.md", "Copy Contents");

    assert!(copied.is_empty());
}

#[test]
fn a_file_that_just_changed_is_highlighted_in_every_theme() {
    let mut harness = explorer();
    assert!(!is_highlighted(&harness, "b.md"));

    for theme in ThemeId::ALL {
        let changed = harness.path("b.md");
        let app = harness.state_mut();
        app.theme = theme;
        app.flashing_paths.insert(changed, Instant::now());
        harness.frames(1);

        assert!(is_highlighted(&harness, "b.md"), "{theme:?}");
        assert!(!is_highlighted(&harness, "a.md"));
    }
}

#[test]
fn a_fresh_change_flashes_at_full_strength() {
    assert_eq!(flash_alpha(Duration::ZERO), Some(180));
}

#[test]
fn the_flash_fades_in_proportion_to_the_time_left() {
    // Half of the 600ms flash is left, so half of the 180 alpha.
    assert_eq!(flash_alpha(Duration::from_millis(300)), Some(90));
    assert_eq!(flash_alpha(Duration::from_millis(150)), Some(135));
}

#[test]
fn the_flash_stays_visible_until_it_ends() {
    // A proportional fade would reach 45 and then 0 here.
    assert_eq!(flash_alpha(Duration::from_millis(450)), Some(60));
    assert_eq!(flash_alpha(Duration::from_millis(599)), Some(60));
}

#[test]
fn the_flash_is_over_after_its_duration() {
    assert_eq!(flash_alpha(Duration::from_millis(600)), None);
    assert_eq!(flash_alpha(Duration::from_secs(5)), None);
}

#[test]
fn the_explorer_title_is_the_folder_name() {
    assert_eq!(folder_name(Path::new("/home/me/notes")), "notes");
}

#[test]
fn a_root_without_a_name_is_titled_by_its_path() {
    assert_eq!(folder_name(Path::new("/")), "/");
}
