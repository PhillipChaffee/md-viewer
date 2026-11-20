//! Keyboard shortcuts.

use std::mem;

use eframe::egui::{self, Key};

use crate::app::action::AppAction;
use crate::app::MarkdownApp;
use crate::layout::{KeyboardScrollAction, ReadingWidth, ZOOM_STEP};
use crate::platform::Platform;

pub(crate) mod hints;
#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

impl Platform {
    /// Whether the command key is among `modifiers`: ⌘ on a Mac, where
    /// Control is another key, and Ctrl elsewhere.
    const fn command_held(self, modifiers: egui::Modifiers) -> bool {
        match self {
            Self::Mac => modifiers.mac_cmd,
            Self::Other => modifiers.ctrl,
        }
    }
}

/// The modifier keys a binding asks for. Each variant constrains only the
/// modifiers it names and ignores the rest. `Cmd` is the command key: ⌘ on
/// a Mac and Ctrl elsewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Chord {
    Any,
    /// No Ctrl, Alt, or Command, so the key cannot belong to another
    /// shortcut.
    Unmodified,
    Shift,
    NoShift,
    Cmd,
    CmdNoShift,
    CmdShift,
    CmdAlt,
    /// The Control key itself, on a Mac too: there it and not ⌘ switches
    /// tabs, since ⌘ with Tab switches apps.
    CtrlNoShift,
    CtrlShift,
    /// Alt without Ctrl, which with Alt is a chord of its own. Not on a
    /// Mac, where Option with an arrow moves a text cursor by a word.
    AltNoCtrl,
    /// ⌘, for a shortcut only a Mac has.
    MacCmd,
}

impl Chord {
    const fn is_held(self, modifiers: egui::Modifiers, platform: Platform) -> bool {
        let command = platform.command_held(modifiers);
        let mac = matches!(platform, Platform::Mac);
        match self {
            Self::Any => true,
            Self::Unmodified => !modifiers.ctrl && !modifiers.alt && !modifiers.command,
            Self::Shift => modifiers.shift,
            Self::NoShift => !modifiers.shift,
            Self::Cmd => command,
            Self::CmdNoShift => command && !modifiers.shift,
            Self::CmdShift => command && modifiers.shift,
            Self::CmdAlt => command && modifiers.alt,
            Self::CtrlNoShift => modifiers.ctrl && !modifiers.shift,
            Self::CtrlShift => modifiers.ctrl && modifiers.shift,
            Self::AltNoCtrl => !mac && modifiers.alt && !modifiers.ctrl,
            Self::MacCmd => mac && modifiers.mac_cmd,
        }
    }

    /// Whether a keyboard on `platform` can hold the chord at all.
    const fn exists_on(self, platform: Platform) -> bool {
        !matches!(
            (self, platform),
            (Self::AltNoCtrl, Platform::Mac) | (Self::MacCmd, Platform::Other)
        )
    }

    /// Whether the chord is a command: a key pressed with ⌘, Ctrl or Alt.
    /// A command is the app's alone. A plain key, such as Enter or an
    /// arrow, is shared with whatever the keyboard is in.
    const fn is_command(self) -> bool {
        !matches!(
            self,
            Self::Any | Self::Unmodified | Self::Shift | Self::NoShift
        )
    }
}

/// What the keyboard is talking to this frame.
#[derive(Clone, Copy, Debug)]
enum Focus {
    /// The prompt about unsaved edits is asking; it suppresses every
    /// binding, and its own keys are its buttons'.
    Prompt,
    /// The lightbox is open; it suppresses everything but its own zoom keys.
    Lightbox,
    /// A menu or popover is open; it takes the keys the find bar and the
    /// document would, and Escape closes it alone.
    Popup,
    /// The find bar is open over the document.
    FindBar,
    /// The document, with no overlay taking keys.
    Reading,
}

/// The focus states a binding is live in.
#[derive(Clone, Copy, Debug)]
enum Scope {
    Lightbox,
    /// The document, with or without the find bar or a menu over it.
    Document,
    FindBar,
    Reading,
}

impl Scope {
    const fn covers(self, focus: Focus) -> bool {
        matches!(
            (self, focus),
            (Self::Lightbox, Focus::Lightbox)
                | (Self::Document, Focus::Popup)
                | (Self::Document | Self::FindBar, Focus::FindBar)
                | (Self::Document | Self::Reading, Focus::Reading)
        )
    }
}

const ZOOM_IN: f32 = ZOOM_STEP;
const ZOOM_OUT: f32 = 1.0 / ZOOM_STEP;

/// Writes the binding table one shortcut per line, as
/// `Chord + Key in Scope => action`. It is a macro only because rustfmt
/// would otherwise spread every row over six lines.
macro_rules! bindings {
    ($($chord:ident + $key:ident in $scope:ident => $action:expr),* $(,)?) => {
        &[$((Chord::$chord, Key::$key, Scope::$scope, $action)),*]
    };
}

/// Every keyboard shortcut. Rows sit in the order their actions run when
/// several fire in one frame.
const BINDINGS: &[(Chord, Key, Scope, AppAction)] = bindings![
    // The lightbox takes over the zoom keys. Its Cmd+0 resets the lightbox
    // zoom only; the document has its own reset.
    Cmd + Num0 in Lightbox => AppAction::ResetLightboxZoom,
    Cmd + Plus in Lightbox => AppAction::ZoomLightboxBy(ZOOM_IN),
    Cmd + Equals in Lightbox => AppAction::ZoomLightboxBy(ZOOM_IN),
    Cmd + Minus in Lightbox => AppAction::ZoomLightboxBy(ZOOM_OUT),

    // Zoom steps are multiplicative (×1.25) like browsers and the lightbox.
    Cmd + Plus in Document => AppAction::ZoomBy(ZOOM_IN),
    Cmd + Equals in Document => AppAction::ZoomBy(ZOOM_IN),
    Cmd + Minus in Document => AppAction::ZoomBy(ZOOM_OUT),
    Cmd + Num0 in Document => AppAction::ResetZoom,

    CmdNoShift + O in Document => AppAction::OpenFile,
    Cmd + T in Document => AppAction::NewTab,
    CmdShift + H in Document => AppAction::GoHome,
    Any + F5 in Document => AppAction::ToggleWatch,
    Cmd + D in Document => AppAction::NextTheme,
    CmdShift + O in Document => AppAction::ToggleOutline,
    CmdShift + E in Document => AppAction::ToggleExplorer,
    Cmd + Q in Document => AppAction::Quit,
    CmdNoShift + S in Document => AppAction::Save,
    CmdNoShift + E in Document => AppAction::ToggleEditMode,

    Cmd + W in Document => AppAction::CloseTab,
    CtrlNoShift + Tab in Document => AppAction::NextTab,
    CtrlShift + Tab in Document => AppAction::PreviousTab,
    Cmd + Num1 in Document => AppAction::FocusTab(0),
    Cmd + Num2 in Document => AppAction::FocusTab(1),
    Cmd + Num3 in Document => AppAction::FocusTab(2),
    Cmd + Num4 in Document => AppAction::FocusTab(3),
    Cmd + Num5 in Document => AppAction::FocusTab(4),
    Cmd + Num6 in Document => AppAction::FocusTab(5),
    Cmd + Num7 in Document => AppAction::FocusTab(6),
    Cmd + Num8 in Document => AppAction::FocusTab(7),
    Cmd + Num9 in Document => AppAction::FocusTab(8),
    AltNoCtrl + ArrowLeft in Document => AppAction::NavigateBack,
    AltNoCtrl + ArrowRight in Document => AppAction::NavigateForward,
    // A Mac goes through history with the browsers' keys.
    MacCmd + OpenBracket in Document => AppAction::NavigateBack,
    MacCmd + CloseBracket in Document => AppAction::NavigateForward,
    // The text column, one step per press.
    CmdAlt + ArrowLeft in Document => AppAction::StepReadingWidth(-ReadingWidth::STEP),
    CmdAlt + ArrowRight in Document => AppAction::StepReadingWidth(ReadingWidth::STEP),

    // Opens the find bar, or refocuses it if already open.
    CmdNoShift + F in Document => AppAction::OpenSearch,
    // While the find bar is open it takes Enter / Shift+Enter / ↑↓ / Esc.
    // Up/Down are safe to bind even when the singleline TextEdit has focus
    // because it doesn't use vertical arrows for cursor movement.
    NoShift + Enter in FindBar => AppAction::NextMatch,
    Any + ArrowDown in FindBar => AppAction::NextMatch,
    Shift + Enter in FindBar => AppAction::PreviousMatch,
    Any + ArrowUp in FindBar => AppAction::PreviousMatch,
    Any + Escape in FindBar => AppAction::CloseSearch,

    // Plain document scroll keys. The arrows stay with search navigation
    // while the find bar is open; the page keys scroll either way.
    Unmodified + ArrowUp in Reading => AppAction::Scroll(KeyboardScrollAction::LineUp),
    Unmodified + ArrowDown in Reading => AppAction::Scroll(KeyboardScrollAction::LineDown),
    Unmodified + PageUp in Document => AppAction::Scroll(KeyboardScrollAction::PageUp),
    Unmodified + PageDown in Document => AppAction::Scroll(KeyboardScrollAction::PageDown),
];

/// Whether `event` is `key` going down while `chord` is held on
/// `platform`. The chord is read off the key press itself and not off the
/// modifiers held when the frame ends: a quick tap lets go of Ctrl inside
/// the frame the key went down in.
fn is_press(event: &egui::Event, chord: Chord, key: Key, platform: Platform) -> bool {
    matches!(
        event,
        egui::Event::Key { key: went_down, pressed: true, modifiers, .. }
            if *went_down == key && chord.is_held(*modifiers, platform)
    )
}

/// The actions this frame's input asks for, in the order they run. Each kind
/// of action is queued once per frame: when two bindings for the same kind
/// fire together, the later row of [`BINDINGS`] replaces the earlier one.
///
/// The key press of a command that fired is taken out of `input`, so no
/// widget drawn later in the frame acts on it a second time: Ctrl+W closes
/// the tab and does not also delete a word in the text field the keyboard
/// is in. A key press that fired nothing stays: on a Mac, Control with a
/// letter is the text field's, and so is Option with an arrow.
fn shortcut_actions(
    input: &mut egui::InputState,
    focus: Focus,
    platform: Platform,
) -> Vec<AppAction> {
    let mut actions: Vec<AppAction> = Vec::new();
    for &(chord, key, scope, action) in BINDINGS {
        let pressed = |event: &egui::Event| is_press(event, chord, key, platform);
        if !scope.covers(focus) || !input.events.iter().any(pressed) {
            continue;
        }
        if chord.is_command() {
            input.events.retain(|event| !pressed(event));
        }
        let kind = mem::discriminant(&action);
        match actions
            .iter_mut()
            .find(|queued| mem::discriminant(*queued) == kind)
        {
            Some(queued) => *queued = action,
            None => actions.push(action),
        }
    }

    // The wheel with the command key held, or a pinch, zooms
    // (multiplicative, like browsers). egui decides which key that is and
    // takes ⌘ and Ctrl alike, so on a Mac Control with the wheel zooms too.
    // The lightbox captures wheel input itself in raw_input_hook, and
    // nothing zooms under the prompt, so this only runs for the document.
    let wheel_zoom_factor = input.zoom_delta();
    let zooms_document = !matches!(focus, Focus::Lightbox | Focus::Prompt);
    if zooms_document && (wheel_zoom_factor - 1.0).abs() > f32::EPSILON {
        actions.push(AppAction::ZoomBy(wheel_zoom_factor));
    }
    actions
}

/// egui has read the frame's Tab presses already, as steps of the keyboard
/// focus. One that switched tabs is the app's alone: the focus stays.
fn keep_the_focus_through_a_tab_switch(ctx: &egui::Context, actions: &[AppAction]) {
    let switches_tabs =
        |action: &AppAction| matches!(action, AppAction::NextTab | AppAction::PreviousTab);
    if actions.iter().any(switches_tabs) {
        ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
    }
}

impl MarkdownApp {
    /// Carry out the actions this frame's keyboard and wheel input ask for.
    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let focus = if self.closing.is_asking() {
            Focus::Prompt
        } else if self.lightbox.open.is_some() {
            Focus::Lightbox
        } else if egui::Popup::is_any_open(ctx) {
            Focus::Popup
        } else if self.search.is_open {
            Focus::FindBar
        } else {
            Focus::Reading
        };
        let platform = self.platform;
        let actions = ctx.input_mut(|input| shortcut_actions(input, focus, platform));
        keep_the_focus_through_a_tab_switch(ctx, &actions);
        for action in actions {
            self.apply_action(ctx, action);
        }
    }
}
