use eframe::egui::Modifiers;

use super::*;

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
