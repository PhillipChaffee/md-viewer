use eframe::egui::Modifiers;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::app::lightbox::{LightboxState, LightboxTexture};
use crate::app::tests::Harness;
use crate::app::ui_harness::{frame_on, UiHarness};
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
    shortcut_actions(&mut input, focus)
}

#[test]
fn a_shortcut_counts_by_what_was_held_when_its_key_went_down() {
    let press = |modifiers| egui::Event::Key {
        key: Key::D,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    let mut input = egui::InputState::default();

    // A quick tap: Ctrl is up again before the frame ends.
    input.modifiers = Modifiers::NONE;
    input.events = vec![press(Modifiers::CTRL)];
    let tapped = shortcut_actions(&mut input, Focus::Reading);
    assert!(matches!(tapped[..], [AppAction::NextTheme]));

    // Ctrl went down after the key did, inside the same frame.
    input.modifiers = Modifiers::CTRL;
    input.events = vec![press(Modifiers::NONE)];
    assert!(shortcut_actions(&mut input, Focus::Reading).is_empty());
}

#[test]
fn a_command_that_fired_is_taken_from_the_input_and_a_plain_key_is_left() {
    let press = |modifiers, key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    let mut input = egui::InputState::default();
    input.events = vec![
        press(Modifiers::CTRL, Key::W),
        press(Modifiers::NONE, Key::Enter),
        press(Modifiers::CTRL, Key::K),
        egui::Event::Text("w".to_owned()),
    ];

    let actions = shortcut_actions(&mut input, Focus::FindBar);

    assert!(matches!(
        actions[..],
        [AppAction::CloseTab, AppAction::NextMatch]
    ));
    assert_eq!(
        input.events,
        [
            press(Modifiers::NONE, Key::Enter),
            press(Modifiers::CTRL, Key::K),
            egui::Event::Text("w".to_owned()),
        ],
        "Enter is the field's too, and Ctrl+K is no shortcut of the app"
    );
}

#[test]
fn ctrl_w_in_the_find_bar_closes_the_tab_and_leaves_the_query() {
    let mut harness = UiHarness::with_three_docs();
    harness.press(Modifiers::CTRL, Key::F);
    harness
        .ui
        .get_by_role(egui::accesskit::Role::TextInput)
        .type_text("one two");
    harness.settle();
    assert_eq!(harness.app().search.query, "one two");

    harness.press(Modifiers::CTRL, Key::W);

    assert!(harness.app().tabs.is_empty());
    assert_eq!(harness.app().search.query, "one two");
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

    harness.app.lightbox.open = Some(LightboxState {
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
fn ctrl_e_edits_the_document_and_shift_makes_it_the_explorer_toggle() {
    let plain = actions_for(Modifiers::CTRL, &[Key::E], Focus::Reading);
    assert!(matches!(plain[..], [AppAction::ToggleEditMode]));

    let with_shift = actions_for(CTRL_SHIFT, &[Key::E], Focus::Reading);
    assert!(matches!(with_shift[..], [AppAction::ToggleExplorer]));

    let under_a_lightbox = actions_for(Modifiers::CTRL, &[Key::E], Focus::Lightbox);
    assert!(under_a_lightbox.is_empty());
}

#[test]
fn a_menu_takes_the_keys_the_find_bar_and_the_document_would() {
    let escape = actions_for(Modifiers::NONE, &[Key::Escape], Focus::Popup);
    assert!(escape.is_empty(), "Escape only closes the menu");
    let enter = actions_for(Modifiers::NONE, &[Key::Enter], Focus::Popup);
    assert!(enter.is_empty(), "Enter only presses the menu's control");
    let arrow = actions_for(Modifiers::NONE, &[Key::ArrowDown], Focus::Popup);
    assert!(arrow.is_empty());

    let theme = actions_for(Modifiers::CTRL, &[Key::D], Focus::Popup);
    assert!(matches!(theme[..], [AppAction::NextTheme]));
}

#[test]
fn escape_closes_an_open_menu_first_and_the_find_bar_on_the_next_press() {
    for (button, row) in [("Appearance", "Line spacing"), ("More", "Quit")] {
        let mut harness = UiHarness::with_three_docs();
        harness.press(Modifiers::CTRL, Key::F);
        harness.click_exact(button);
        assert!(harness.shows(row) && harness.shows("Close find bar"));

        harness.press(Modifiers::NONE, Key::Escape);
        assert!(!harness.shows(row), "Escape closes the {button} menu");
        assert!(harness.shows("Close find bar"), "{button}: the bar stays");

        harness.press(Modifiers::NONE, Key::Escape);
        assert!(!harness.shows("Close find bar"));
    }
}
