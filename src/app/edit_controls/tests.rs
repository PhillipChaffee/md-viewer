//! The edit controls, clicked through by label, in the strip and in the
//! top bar of a lone document.

use std::time::Instant;

use eframe::egui::accesskit::Toggled;
use eframe::egui::{self, Key, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::app::content::tests::label_harness::LabelHarness;
use crate::app::tab_bar::tests::{one_tab, shows_strip, strip_text, three_tabs};
use crate::tab::SaveOutcome;
use crate::test_support::{assert_close, painted_shapes};

#[test]
fn the_mode_toggle_switches_between_preview_and_edit() {
    let mut harness = three_tabs();
    let editing = |harness: &LabelHarness| harness.state().tabs[2].is_editing;
    let chosen = |harness: &LabelHarness, mode| {
        harness.get_by_label(mode).accesskit_node().toggled() == Some(Toggled::True)
    };
    assert!(chosen(&harness, "Preview"));

    harness.click("Edit");
    assert!(editing(&harness));
    assert!(chosen(&harness, "Edit"));
    assert!(!chosen(&harness, "Preview"));

    harness.click("Preview");
    assert!(!editing(&harness));
}

/// Where the document pane starts: the top of the contents sidebar, which
/// stands beside the document in it.
fn pane_top(harness: &LabelHarness) -> f32 {
    harness.get_by_label("CONTENTS").rect().top()
}

#[test]
fn editing_a_lone_document_leaves_the_page_where_it_is() {
    let mut harness = one_tab("# A\n\n## B");
    harness.state_mut().show_outline = true;
    harness.frames(2);
    let at_rest = pane_top(&harness);

    harness.key_press_modifiers(Modifiers::CTRL, Key::E);
    harness.frames(2);
    assert!(harness.state().tabs[0].is_editing);
    assert_close(pane_top(&harness), at_rest);

    harness.state_mut().tabs[0].content = "# A, edited".to_owned();
    harness.frames(2);
    assert!(harness.query_by_label("Unsaved changes").is_some());
    assert_close(pane_top(&harness), at_rest);

    harness.click("Preview");
    assert!(!harness.state().tabs[0].is_editing);
    assert_close(pane_top(&harness), at_rest);
}

#[test]
fn a_lone_document_has_its_edit_controls_in_the_top_bar() {
    let mut harness = one_tab("# A");
    assert!(harness.query_by_label("Preview").is_none());

    harness.state_mut().tabs[0].set_edit_mode(true);
    harness.state_mut().tabs[0].content = "# A, edited".to_owned();
    harness.state_mut().tabs[0].save_flash = Some((SaveOutcome::Failed, Instant::now()));
    harness.frames(2);

    assert!(!shows_strip(&harness));
    let bar_bottom = crate::app::chrome::BAR_HEIGHT;
    let appearance = harness.get_by_label("Appearance").rect();
    for control in ["Save failed", "Unsaved changes", "Preview", "Edit"] {
        let rect = harness.get_by_label(control).rect();
        assert!(rect.bottom() <= bar_bottom, "{control} at {rect:?}");
        assert!(rect.right() < appearance.left(), "{control} at {rect:?}");
    }
}

#[test]
fn unsaved_edits_are_marked_beside_the_mode_toggle() {
    let mut harness = one_tab("# A");
    harness.state_mut().tabs[0].set_edit_mode(true);
    harness.frames(2);
    assert!(harness.query_by_label("Unsaved changes").is_none());

    harness.state_mut().tabs[0].content = "# A, edited".to_owned();
    harness.frames(1);

    let dot = harness.get_by_label("Unsaved changes").rect();
    let toggle = harness.get_by_label("Preview").rect();
    assert!(dot.right() <= toggle.left(), "{dot:?} before {toggle:?}");
    assert_close(dot.center().y, toggle.center().y);
    let warning = harness.state().theme.palette().warning;
    let dots = painted_shapes(harness.output())
        .into_iter()
        .filter(|shape| matches!(shape, egui::Shape::Circle(dot) if dot.fill == warning))
        .count();
    assert_eq!(dots, 1);
}

#[test]
fn unsaved_edits_of_a_lone_document_stay_marked_after_leaving_edit_mode() {
    let mut harness = one_tab("# A");
    harness.state_mut().tabs[0].content = "# A, edited".to_owned();
    harness.frames(2);

    assert!(!shows_strip(&harness));
    assert!(harness.query_by_label("Unsaved changes").is_some());
    assert!(harness.query_by_label("Preview").is_some());
}

#[test]
fn a_save_reports_its_outcome_beside_the_mode_toggle() {
    let mut harness = three_tabs();
    assert!(harness.query_by_label("Saved").is_none());

    harness.state_mut().tabs[2].save_flash = Some((SaveOutcome::Saved, Instant::now()));
    harness.frames(1);
    assert!(harness.query_by_label("Saved").is_some());
    let palette = harness.state().theme.palette();
    assert_eq!(
        strip_text(&harness, "Saved").galley.job.sections[0]
            .format
            .color,
        palette.positive
    );

    harness.state_mut().tabs[2].save_flash = Some((SaveOutcome::Failed, Instant::now()));
    harness.frames(1);
    assert!(harness.query_by_label("Save failed").is_some());
    assert!(harness.query_by_label("Saved").is_none());
    assert_eq!(
        strip_text(&harness, "Save failed").galley.job.sections[0]
            .format
            .color,
        palette.warning
    );
}

#[test]
fn a_save_outcome_disappears_after_a_moment() {
    let mut harness = one_tab("# A");
    let expired = Instant::now().checked_sub(super::SAVE_FLASH).unwrap();

    harness.state_mut().tabs[0].save_flash = Some((SaveOutcome::Saved, expired));
    harness.frames(2);

    assert!(harness.query_by_label("Saved").is_none());
    assert_eq!(harness.state().tabs[0].save_flash, None);
    assert!(!shows_strip(&harness), "nothing is left to report");
}
