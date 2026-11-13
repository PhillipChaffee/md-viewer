use std::path::PathBuf;

use eframe::egui::Modifiers;

use super::*;
use crate::app::lightbox::{LightboxState, LightboxTexture};
use crate::app::tests::Harness;
use crate::app::ui_harness::{frame_on, frame_with};
use crate::test_support::assert_close;

const CTRL_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: false,
    command: false,
};

/// The actions one frame asks for when every key in `keys` goes down with
/// `modifiers` held.
fn actions_for(modifiers: Modifiers, keys: &[Key], focus: Focus) -> Vec<AppAction> {
    let mut input = egui::InputState::default();
    input.modifiers = modifiers;
    input.events = keys
        .iter()
        .map(|&key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        })
        .collect();
    shortcut_actions(&input, focus)
}

#[test]
fn a_frame_without_keys_asks_for_nothing() {
    for focus in [Focus::Reading, Focus::FindBar, Focus::Lightbox] {
        assert!(actions_for(Modifiers::CTRL, &[], focus).is_empty());
    }
}

#[test]
fn both_open_file_shortcuts_in_one_frame_open_one_dialog() {
    let together = actions_for(Modifiers::CTRL, &[Key::O, Key::T], Focus::Reading);
    assert!(matches!(together[..], [AppAction::OpenFile]));

    let ctrl_t = actions_for(Modifiers::CTRL, &[Key::T], Focus::Reading);
    assert!(matches!(ctrl_t[..], [AppAction::OpenFile]));
}

#[test]
fn shift_turns_ctrl_o_into_the_outline_toggle() {
    let with_shift = actions_for(CTRL_SHIFT, &[Key::O], Focus::Reading);
    assert!(matches!(with_shift[..], [AppAction::ToggleOutline]));

    let without_shift = actions_for(Modifiers::CTRL, &[Key::O], Focus::Reading);
    assert!(matches!(without_shift[..], [AppAction::OpenFile]));
}

#[test]
fn alt_arrow_asks_for_a_single_history_step() {
    let back = actions_for(Modifiers::ALT, &[Key::ArrowLeft], Focus::Reading);
    assert!(matches!(back[..], [AppAction::NavigateBack]));

    let forward = actions_for(Modifiers::ALT, &[Key::ArrowRight], Focus::FindBar);
    assert!(matches!(forward[..], [AppAction::NavigateForward]));
}

#[test]
fn arrows_scroll_the_document_or_step_through_matches() {
    let reading = actions_for(Modifiers::NONE, &[Key::ArrowDown], Focus::Reading);
    assert!(matches!(
        reading[..],
        [AppAction::Scroll(KeyboardScrollAction::LineDown)]
    ));

    let finding = actions_for(Modifiers::NONE, &[Key::ArrowDown], Focus::FindBar);
    assert!(matches!(finding[..], [AppAction::NextMatch]));

    let page_while_finding = actions_for(Modifiers::NONE, &[Key::PageDown], Focus::FindBar);
    assert!(matches!(
        page_while_finding[..],
        [AppAction::Scroll(KeyboardScrollAction::PageDown)]
    ));

    let with_ctrl = actions_for(Modifiers::CTRL, &[Key::ArrowDown], Focus::Reading);
    assert!(with_ctrl.is_empty(), "a modified arrow is not a scroll key");
}

#[test]
fn the_lightbox_keeps_only_its_zoom_keys() {
    let zoom = actions_for(Modifiers::CTRL, &[Key::Plus], Focus::Lightbox);
    assert!(
        matches!(zoom[..], [AppAction::ZoomLightboxBy(factor)] if (factor - ZOOM_STEP).abs() < 1e-6)
    );

    let reset = actions_for(Modifiers::CTRL, &[Key::Num0], Focus::Lightbox);
    assert!(matches!(reset[..], [AppAction::ResetLightboxZoom]));

    for key in [Key::W, Key::D, Key::Q, Key::F, Key::PageDown, Key::F5] {
        let suppressed = actions_for(Modifiers::CTRL, &[key], Focus::Lightbox);
        assert!(suppressed.is_empty(), "{key:?} reached the document");
    }
}

#[test]
fn the_later_binding_wins_when_one_kind_fires_twice() {
    let zoom = actions_for(Modifiers::CTRL, &[Key::Plus, Key::Minus], Focus::Reading);
    assert!(
        matches!(zoom[..], [AppAction::ZoomBy(factor)] if (factor - 1.0 / ZOOM_STEP).abs() < 1e-6)
    );

    let tabs = actions_for(Modifiers::CTRL, &[Key::Num3, Key::Num1], Focus::Reading);
    assert!(matches!(tabs[..], [AppAction::FocusTab(2)]));
}

#[test]
fn a_pinch_zooms_the_document_but_not_under_a_lightbox() {
    let mut harness = Harness::with_three_docs();

    frame_on(&mut harness, vec![egui::Event::Zoom(1.5)]);
    assert_close(harness.app.zoom_level, 1.5);

    harness.app.lightbox = Some(LightboxState {
        texture: LightboxTexture::Loaded(egui::TextureId::default()),
        base_size: egui::vec2(100.0, 100.0),
        zoom: 1.0,
        open_id: 1,
        scroll_offset: egui::Vec2::ZERO,
    });
    frame_on(&mut harness, vec![egui::Event::Zoom(1.5)]);
    assert_close(harness.app.zoom_level, 1.5);
}

#[test]
fn markdown_and_text_files_are_recognized_by_extension_in_any_case() {
    for name in ["notes.md", "NOTES.MD", "guide.markdown", "todo.txt"] {
        assert!(is_markdown_file(Path::new(name)), "{name}");
    }
    for name in ["paper.pdf", "md", "archive.md.zip", ".md"] {
        assert!(!is_markdown_file(Path::new(name)), "{name}");
    }
}

fn dropping(paths: Vec<Option<PathBuf>>) -> egui::RawInput {
    let dropped_files = paths.into_iter().map(|path| egui::DroppedFile {
        path,
        ..Default::default()
    });
    egui::RawInput {
        dropped_files: dropped_files.collect(),
        ..Default::default()
    }
}

#[test]
fn a_dropped_markdown_file_opens_in_a_new_tab() {
    let mut harness = Harness::with_three_docs();
    let b = harness.path("b.md");

    frame_with(&mut harness, dropping(vec![None, Some(b.clone())]));

    assert_eq!(harness.app.tabs.len(), 2);
    assert_eq!(harness.active_path(), b);
    assert!(harness.app.error_message.is_none());
    assert!(!harness.app.is_dragging);
}

#[test]
fn a_dropped_file_of_another_kind_is_refused_with_a_message() {
    let mut harness = Harness::with_three_docs();
    let picture = harness.path("picture.png");

    frame_with(&mut harness, dropping(vec![Some(picture)]));

    assert_eq!(harness.app.tabs.len(), 1);
    assert!(harness.app.error_message.is_some());
}

#[test]
fn the_drag_state_lasts_only_while_files_hover() {
    let mut harness = Harness::with_three_docs();
    let hovering = egui::RawInput {
        hovered_files: vec![egui::HoveredFile::default()],
        ..Default::default()
    };

    frame_with(&mut harness, hovering);
    assert!(harness.app.is_dragging);

    harness.idle();
    assert!(!harness.app.is_dragging);
}
