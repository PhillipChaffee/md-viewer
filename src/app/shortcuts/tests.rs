use eframe::egui::Modifiers;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::app::tests::Harness;
use crate::app::ui_harness::{enlarged, frame_on, frame_with, UiHarness};
use crate::test_support::assert_close;

const CTRL_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: false,
    command: false,
};

/// What a Mac reports while ⌘ is held: egui sets `command` beside
/// `mac_cmd` there.
const MAC_CMD: Modifiers = Modifiers {
    alt: false,
    ctrl: false,
    shift: false,
    mac_cmd: true,
    command: true,
};

/// The fewest keys that hold `chord` on `platform`, as that platform's
/// keyboard reports them. `None` where it cannot hold the chord.
fn held_for(chord: Chord, platform: Platform) -> Option<Modifiers> {
    let command = match platform {
        Platform::Mac => MAC_CMD,
        Platform::Other => Modifiers::CTRL,
    };
    let held = match chord {
        Chord::Any | Chord::Unmodified | Chord::NoShift => Modifiers::NONE,
        Chord::Shift => Modifiers::SHIFT,
        Chord::Cmd | Chord::CmdNoShift | Chord::MacCmd => command,
        Chord::CmdShift => command | Modifiers::SHIFT,
        Chord::CmdAlt => command | Modifiers::ALT,
        Chord::CtrlNoShift => Modifiers::CTRL,
        Chord::CtrlShift => CTRL_SHIFT,
        Chord::AltNoCtrl => Modifiers::ALT,
    };
    chord.exists_on(platform).then_some(held)
}

/// The actions one frame asks for off the Mac when every key in `keys`
/// goes down with `modifiers` held.
fn actions_for(modifiers: Modifiers, keys: &[Key], focus: Focus) -> Vec<AppAction> {
    actions_on(Platform::Other, modifiers, keys, focus)
}

/// The actions one frame asks for on `platform` when every key in `keys`
/// goes down with `modifiers` held.
fn actions_on(
    platform: Platform,
    modifiers: Modifiers,
    keys: &[Key],
    focus: Focus,
) -> Vec<AppAction> {
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
    shortcut_actions(&mut input, focus, platform)
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
    let tapped = shortcut_actions(&mut input, Focus::Reading, Platform::Other);
    assert!(matches!(tapped[..], [AppAction::NextTheme]));

    // Ctrl went down after the key did, inside the same frame.
    input.modifiers = Modifiers::CTRL;
    input.events = vec![press(Modifiers::NONE)];
    assert!(shortcut_actions(&mut input, Focus::Reading, Platform::Other).is_empty());
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

    let actions = shortcut_actions(&mut input, Focus::FindBar, Platform::Other);

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
fn ctrl_o_opens_a_file_and_ctrl_t_a_new_tab() {
    let ctrl_o = actions_for(Modifiers::CTRL, &[Key::O], Focus::Reading);
    assert!(matches!(ctrl_o[..], [AppAction::OpenFile]));

    let ctrl_t = actions_for(Modifiers::CTRL, &[Key::T], Focus::Reading);
    assert!(matches!(ctrl_t[..], [AppAction::NewTab]));
}

#[test]
fn ctrl_shift_h_goes_home_and_plain_ctrl_h_does_nothing() {
    let home = actions_for(
        Modifiers::CTRL | Modifiers::SHIFT,
        &[Key::H],
        Focus::Reading,
    );
    assert!(matches!(home[..], [AppAction::GoHome]));

    assert!(actions_for(Modifiers::CTRL, &[Key::H], Focus::Reading).is_empty());
    let under_a_picture = actions_for(
        Modifiers::CTRL | Modifiers::SHIFT,
        &[Key::H],
        Focus::Lightbox,
    );
    assert!(under_a_picture.is_empty());
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

    harness.app.lightbox.open = Some(enlarged());
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
fn the_prompt_about_unsaved_edits_takes_every_key_and_the_wheel() {
    for platform in [Platform::Mac, Platform::Other] {
        for &(chord, key, _, _) in BINDINGS {
            let Some(modifiers) = held_for(chord, platform) else {
                continue;
            };
            let asked = actions_on(platform, modifiers, &[key], Focus::Prompt);
            assert!(asked.is_empty(), "{platform:?}: {chord:?} + {key:?}");
        }
    }

    let mut harness = Harness::with_three_docs();
    harness.app.tabs[0].content.push_str(" edited");
    harness.app.close_active_tab();
    frame_on(&mut harness, vec![egui::Event::Zoom(1.5)]);
    assert_close(harness.app.zoom_level, 1.0);
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

/// The focus a binding of `scope` is live in.
const fn focus_of(scope: Scope) -> Focus {
    match scope {
        Scope::Lightbox => Focus::Lightbox,
        Scope::FindBar => Focus::FindBar,
        Scope::Document | Scope::Reading => Focus::Reading,
    }
}

#[test]
fn every_binding_fires_with_the_keys_its_platform_holds_it_with() {
    for platform in [Platform::Mac, Platform::Other] {
        for &(chord, key, scope, action) in BINDINGS {
            let Some(modifiers) = held_for(chord, platform) else {
                continue;
            };
            let asked = actions_on(platform, modifiers, &[key], focus_of(scope));
            assert_eq!(asked, [action], "{platform:?}: {chord:?} + {key:?}");
        }
    }
}

#[test]
fn a_command_is_held_with_cmd_on_a_mac_and_with_ctrl_elsewhere() {
    let commands = [
        (Key::W, AppAction::CloseTab),
        (Key::T, AppAction::NewTab),
        (Key::O, AppAction::OpenFile),
        (Key::F, AppAction::OpenSearch),
        (Key::S, AppAction::Save),
        (Key::E, AppAction::ToggleEditMode),
        (Key::D, AppAction::NextTheme),
        (Key::Q, AppAction::Quit),
        (Key::Num0, AppAction::ResetZoom),
        (Key::Num3, AppAction::FocusTab(2)),
    ];
    for (key, action) in commands {
        let cmd = actions_on(Platform::Mac, MAC_CMD, &[key], Focus::Reading);
        assert_eq!(cmd, [action], "⌘ + {key:?} on a Mac");
        let control = actions_on(Platform::Mac, Modifiers::CTRL, &[key], Focus::Reading);
        assert_eq!(control, [], "Control + {key:?} on a Mac");

        let ctrl = actions_on(Platform::Other, Modifiers::CTRL, &[key], Focus::Reading);
        assert_eq!(ctrl, [action], "Ctrl + {key:?} off the Mac");
    }

    let sidebar = actions_on(
        Platform::Mac,
        MAC_CMD | Modifiers::SHIFT,
        &[Key::E],
        Focus::Reading,
    );
    assert_eq!(sidebar, [AppAction::ToggleExplorer]);
}

#[test]
fn control_with_tab_switches_tabs_on_a_mac_too_and_cmd_with_tab_does_not() {
    for platform in [Platform::Mac, Platform::Other] {
        let next = actions_on(platform, Modifiers::CTRL, &[Key::Tab], Focus::Reading);
        assert_eq!(next, [AppAction::NextTab], "{platform:?}");
        let previous = actions_on(platform, CTRL_SHIFT, &[Key::Tab], Focus::Reading);
        assert_eq!(previous, [AppAction::PreviousTab], "{platform:?}");
    }
    let cmd_tab = actions_on(Platform::Mac, MAC_CMD, &[Key::Tab], Focus::Reading);
    assert_eq!(cmd_tab, [], "⌘ with Tab is the system's app switcher");
}

#[test]
fn a_mac_steps_through_history_with_cmd_and_a_bracket() {
    let on_mac = |modifiers, key| actions_on(Platform::Mac, modifiers, &[key], Focus::Reading);
    assert_eq!(on_mac(MAC_CMD, Key::OpenBracket), [AppAction::NavigateBack]);
    assert_eq!(
        on_mac(MAC_CMD, Key::CloseBracket),
        [AppAction::NavigateForward]
    );
    assert_eq!(
        on_mac(Modifiers::ALT, Key::ArrowLeft),
        [],
        "Option with an arrow moves the text cursor by a word"
    );

    let elsewhere = |modifiers, key| actions_on(Platform::Other, modifiers, &[key], Focus::Reading);
    assert_eq!(
        elsewhere(Modifiers::ALT, Key::ArrowLeft),
        [AppAction::NavigateBack]
    );
    assert_eq!(elsewhere(Modifiers::CTRL, Key::OpenBracket), []);
}

#[test]
fn the_column_is_set_with_the_command_key_alt_and_an_arrow() {
    let narrower = [AppAction::StepReadingWidth(-ReadingWidth::STEP)];
    let ctrl_alt = Modifiers::CTRL | Modifiers::ALT;
    let cmd_option = MAC_CMD | Modifiers::ALT;

    let off_the_mac = actions_on(Platform::Other, ctrl_alt, &[Key::ArrowLeft], Focus::Reading);
    assert_eq!(off_the_mac, narrower);
    let on_a_mac = actions_on(Platform::Mac, cmd_option, &[Key::ArrowLeft], Focus::Reading);
    assert_eq!(on_a_mac, narrower, "and no history step with it");
    let control_option = actions_on(Platform::Mac, ctrl_alt, &[Key::ArrowLeft], Focus::Reading);
    assert_eq!(control_option, []);
}

#[test]
fn on_a_mac_a_key_the_app_does_not_bind_stays_for_the_text_field() {
    let press = |modifiers, key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    let mut input = egui::InputState::default();
    // Control with a letter and Option with an arrow move the text cursor
    // on a Mac; ⌘A, ⌘Z and ⌘← are the text field's as everywhere.
    let left_alone = vec![
        press(Modifiers::CTRL, Key::E),
        press(Modifiers::CTRL, Key::W),
        press(Modifiers::ALT, Key::ArrowLeft),
        press(MAC_CMD, Key::A),
        press(MAC_CMD, Key::Z),
        press(MAC_CMD, Key::ArrowLeft),
    ];
    input.events = left_alone.clone();
    input.events.push(press(MAC_CMD, Key::W));

    let actions = shortcut_actions(&mut input, Focus::Reading, Platform::Mac);

    assert_eq!(actions, [AppAction::CloseTab]);
    assert_eq!(input.events, left_alone, "only ⌘W was the app's");
}

#[test]
fn a_chord_exists_exactly_where_some_keys_hold_it() {
    let every_set = (0..32_u8).map(|bits| Modifiers {
        alt: bits & 1 != 0,
        ctrl: bits & 2 != 0,
        shift: bits & 4 != 0,
        mac_cmd: bits & 8 != 0,
        command: bits & 16 != 0,
    });
    for platform in [Platform::Mac, Platform::Other] {
        for &(chord, ..) in BINDINGS {
            let can_be_held = every_set
                .clone()
                .any(|modifiers| chord.is_held(modifiers, platform));
            assert_eq!(
                chord.exists_on(platform),
                can_be_held,
                "{platform:?}: {chord:?}"
            );
        }
    }
}

/// A frame of the app as a Mac, in which the wheel turns with `modifiers`
/// held. Returns the zoom after it.
fn zoom_after_a_wheel_turn_on_a_mac(modifiers: Modifiers) -> f32 {
    let mut harness = Harness::with_three_docs();
    harness.app.platform = Platform::Mac;
    let wheel = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        modifiers,
    };
    let input = egui::RawInput {
        modifiers,
        events: vec![wheel],
        ..Default::default()
    };
    frame_with(&mut harness, input);
    harness.app.zoom_level
}

#[test]
fn the_wheel_zooms_with_cmd_held_on_a_mac() {
    assert!(zoom_after_a_wheel_turn_on_a_mac(MAC_CMD) > 1.0);
    assert_close(zoom_after_a_wheel_turn_on_a_mac(Modifiers::NONE), 1.0);
}

#[test]
fn the_app_as_a_mac_closes_a_tab_on_cmd_w_and_not_on_control_w() {
    let mut harness = Harness::with_three_docs();
    harness.app.platform = Platform::Mac;
    harness.app.open_in_new_tab(harness.path("b.md"));

    harness.press(Modifiers::CTRL, Key::W);
    assert_eq!(harness.app.tabs.len(), 2, "Control+W is not a command");
    harness.press(MAC_CMD, Key::W);
    assert_eq!(harness.app.tabs.len(), 1);
}

#[test]
fn the_host_is_a_mac_only_in_a_mac_build() {
    assert_eq!(Platform::HOST == Platform::Mac, cfg!(target_os = "macos"));
}
