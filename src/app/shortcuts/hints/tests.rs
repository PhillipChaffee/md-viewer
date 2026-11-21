use eframe::egui::{Key, Modifiers};

use super::*;
use crate::app::tests::Harness;
use crate::app::ui_harness::UiHarness;
use crate::layout::{ReadingWidth, ZOOM_STEP};
use crate::system_fonts::TextSizeClass;

/// The hint for `action` in each notation: off the Mac, on a Mac in signs,
/// and on a Mac in names.
fn hints_of(action: AppAction) -> [String; 3] {
    [Notation::Names, Notation::MacSigns, Notation::MacNames]
        .map(|notation| Hints::new(notation).of(action).to_owned())
}

#[test]
fn the_notation_follows_the_platform_and_then_the_font() {
    for has_signs in [false, true] {
        let off_the_mac = Notation::on(Platform::Other, has_signs);
        assert_eq!(off_the_mac, Notation::Names);
    }
    assert_eq!(Notation::on(Platform::Mac, true), Notation::MacSigns);
    assert_eq!(Notation::on(Platform::Mac, false), Notation::MacNames);
}

#[test]
fn a_command_reads_ctrl_off_the_mac_and_cmd_on_one() {
    let cases = [
        (AppAction::CloseTab, ["Ctrl+W", "⌘W", "Cmd+W"]),
        (AppAction::OpenFile, ["Ctrl+O", "⌘O", "Cmd+O"]),
        (AppAction::OpenSearch, ["Ctrl+F", "⌘F", "Cmd+F"]),
        (AppAction::Save, ["Ctrl+S", "⌘S", "Cmd+S"]),
        (AppAction::ToggleEditMode, ["Ctrl+E", "⌘E", "Cmd+E"]),
        (AppAction::NextTheme, ["Ctrl+D", "⌘D", "Cmd+D"]),
        (AppAction::Quit, ["Ctrl+Q", "⌘Q", "Cmd+Q"]),
        (AppAction::FocusTab(0), ["Ctrl+1", "⌘1", "Cmd+1"]),
        (AppAction::ResetZoom, ["Ctrl+0", "⌘0", "Cmd+0"]),
        (AppAction::ZoomBy(ZOOM_STEP), ["Ctrl++", "⌘+", "Cmd++"]),
        (
            AppAction::ZoomBy(1.0 / ZOOM_STEP),
            ["Ctrl+-", "⌘-", "Cmd+-"],
        ),
    ];
    for (action, expected) in cases {
        assert_eq!(hints_of(action), expected, "{action:?}");
    }
}

#[test]
fn modifiers_are_listed_in_the_order_each_platform_lists_them() {
    let cases = [
        (
            AppAction::ToggleExplorer,
            ["Ctrl+Shift+E", "⇧⌘E", "Shift+Cmd+E"],
        ),
        (
            AppAction::ToggleOutline,
            ["Ctrl+Shift+O", "⇧⌘O", "Shift+Cmd+O"],
        ),
        (
            AppAction::StepReadingWidth(-ReadingWidth::STEP),
            ["Ctrl+Alt+Left", "⌥⌘←", "Option+Cmd+Left"],
        ),
        (
            AppAction::StepReadingWidth(ReadingWidth::STEP),
            ["Ctrl+Alt+Right", "⌥⌘→", "Option+Cmd+Right"],
        ),
        (
            AppAction::PreviousTab,
            ["Ctrl+Shift+Tab", "⌃⇧Tab", "Ctrl+Shift+Tab"],
        ),
    ];
    for (action, expected) in cases {
        assert_eq!(hints_of(action), expected, "{action:?}");
    }
}

#[test]
fn control_stays_control_on_a_mac_for_the_tab_switch() {
    assert_eq!(
        hints_of(AppAction::NextTab),
        ["Ctrl+Tab", "⌃Tab", "Ctrl+Tab"]
    );
}

#[test]
fn history_reads_alt_and_an_arrow_off_the_mac_and_cmd_and_a_bracket_on_one() {
    assert_eq!(
        hints_of(AppAction::NavigateBack),
        ["Alt+Left", "⌘[", "Cmd+["]
    );
    assert_eq!(
        hints_of(AppAction::NavigateForward),
        ["Alt+Right", "⌘]", "Cmd+]"]
    );
}

#[test]
fn a_key_without_a_modifier_reads_as_the_key() {
    assert_eq!(hints_of(AppAction::ToggleWatch), ["F5", "F5", "F5"]);
}

#[test]
fn every_way_to_ask_is_listed_in_table_order() {
    let all = |notation, action| -> Vec<String> {
        let hints = Hints::new(notation);
        hints.all(action).map(str::to_owned).collect()
    };

    assert_eq!(
        all(Notation::Names, AppAction::PreviousMatch),
        ["Shift+Enter", "Up"]
    );
    assert_eq!(
        all(Notation::MacSigns, AppAction::PreviousMatch),
        ["⇧Return", "↑"]
    );
    assert_eq!(
        all(Notation::MacNames, AppAction::NextMatch),
        ["Return", "Down"]
    );
    assert_eq!(all(Notation::Names, AppAction::OpenFile), ["Ctrl+O"]);
    assert_eq!(all(Notation::MacSigns, AppAction::OpenFile), ["⌘O"]);
}

#[test]
fn a_new_tab_and_the_home_page_have_hints_of_their_own() {
    assert_eq!(hints_of(AppAction::NewTab), ["Ctrl+T", "⌘T", "Cmd+T"]);
    assert_eq!(
        hints_of(AppAction::GoHome),
        ["Ctrl+Shift+H", "⇧⌘H", "Shift+Cmd+H"]
    );
}

#[test]
fn an_action_with_no_shortcut_has_an_empty_hint() {
    for hint in hints_of(AppAction::OpenFolder) {
        assert_eq!(hint, "");
    }
}

#[test]
fn every_sign_a_mac_hint_uses_is_one_the_font_is_asked_about() {
    let hints = Hints::new(Notation::MacSigns);
    let signs = hints.keys.iter().flat_map(|(_, hint)| hint.chars());
    for sign in signs.filter(|sign| !sign.is_ascii()) {
        assert!(MAC_SIGNS.contains(sign), "{sign} is not checked for");
    }
}

#[test]
fn a_hint_never_names_a_key_the_platform_cannot_press() {
    let off_the_mac = Hints::new(Notation::Names);
    assert!(off_the_mac.keys.iter().all(|(_, hint)| !hint.contains('[')));
    let mac = Hints::new(Notation::MacNames);
    assert!(mac.keys.iter().all(|(_, hint)| !hint.contains("Alt")));
    let unmodified_arrow = |(_, hint): &(AppAction, String)| hint == "Option+Left";
    assert!(!mac.keys.iter().any(unmodified_arrow));
}

/// The app as a Mac, with its hints written again for that.
fn as_a_mac(docs: &[(&str, &str)]) -> UiHarness {
    let mut harness = UiHarness::with_docs(docs);
    harness.app_mut().platform = Platform::Mac;
    harness.app_mut().hints_stale = true;
    harness.settle();
    harness
}

#[test]
fn the_app_as_a_mac_writes_its_hints_for_a_mac() {
    let harness = as_a_mac(&[("a.md", "# A")]);
    let font = chrome::small_font();
    let has_signs = harness
        .ui
        .ctx
        .fonts_mut(|fonts| fonts.has_glyphs(&font, MAC_SIGNS));

    let close = harness.app().hints.of(AppAction::CloseTab);

    assert_eq!(close, if has_signs { "⌘W" } else { "Cmd+W" });
}

#[test]
fn the_menu_and_the_tooltips_of_the_app_as_a_mac_name_cmd() {
    let mut harness = as_a_mac(&[("a.md", "# A")]);
    let hints = |harness: &UiHarness, action| harness.app().hints.of(action).to_owned();
    let close = hints(&harness, AppAction::CloseTab);
    let find = hints(&harness, AppAction::OpenSearch);
    assert!(close.ends_with('W') && !close.contains("Ctrl"), "{close}");

    harness.hover_at(harness.center_of("Find in document"));
    assert!(harness.shows(&format!("Find ({find})")));

    harness.open_more();
    let painted = crate::app::ui_harness::painted_text(harness.ui.output());
    assert!(painted.contains(&close), "{painted:?}");
    assert!(!painted.iter().any(|text| text.starts_with("Ctrl+")));
}

#[test]
fn off_the_mac_the_hints_are_written_once_and_read_ctrl() {
    let mut harness = Harness::with_docs(&[("a.md", "# A")]);
    assert!(harness.app.hints_stale, "nothing is written before a frame");

    harness.idle();
    harness.idle();

    assert!(!harness.app.hints_stale);
    assert_eq!(harness.app.hints.of(AppAction::CloseTab), "Ctrl+W");
    assert_eq!(harness.app.platform, Platform::HOST);
}

#[test]
fn a_tooltip_lists_every_shortcut_after_the_name() {
    let mut harness = UiHarness::with_three_docs();
    harness.press(Modifiers::CTRL, Key::F);

    harness.hover_at(harness.center_of("Previous match"));

    assert!(harness.shows("Previous match (Shift+Enter / Up)"));
}

#[test]
fn the_hints_are_written_again_once_new_fonts_are_in_force() {
    let mut harness = Harness::with_docs(&[("a.md", "# A")]);
    harness.idle();
    harness.idle();
    // Stands in for new fonts turning out to have, or to lack, the signs.
    harness.app.platform = Platform::Mac;
    harness.idle();
    let close = |harness: &Harness| harness.app.hints.of(AppAction::CloseTab).to_owned();
    assert_eq!(close(&harness), "Ctrl+W", "no font changed");

    harness.apply(AppAction::SetTextSize(TextSizeClass::Large));
    harness.idle();
    assert_eq!(
        close(&harness),
        "Ctrl+W",
        "the new fonts are not in force yet"
    );
    harness.idle();

    assert!(close(&harness).ends_with('W') && !close(&harness).contains("Ctrl"));
}

#[test]
fn the_click_for_a_new_tab_is_named_after_the_command_key() {
    let clicks = [Notation::Names, Notation::MacSigns, Notation::MacNames]
        .map(|notation| Hints::new(notation).command_click().to_owned());

    assert_eq!(clicks, ["Ctrl-click", "⌘-click", "Cmd-click"]);
}
