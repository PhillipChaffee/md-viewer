//! Keyboard shortcuts and dropped files.

use std::mem;

use eframe::egui::{self, Key};

use crate::app::action::AppAction;
use crate::app::MarkdownApp;
use crate::layout::{KeyboardScrollAction, ZOOM_STEP};

#[cfg(test)]
mod tests;

/// The modifier keys a binding asks for. Each variant constrains only the
/// modifiers it names and ignores the rest.
#[derive(Clone, Copy, Debug)]
enum Chord {
    Any,
    /// No Ctrl, Alt, or Command, so the key cannot belong to another
    /// shortcut.
    Unmodified,
    Shift,
    NoShift,
    Ctrl,
    CtrlNoShift,
    CtrlShift,
    Alt,
}

impl Chord {
    const fn is_held(self, modifiers: egui::Modifiers) -> bool {
        match self {
            Self::Any => true,
            Self::Unmodified => !modifiers.ctrl && !modifiers.alt && !modifiers.command,
            Self::Shift => modifiers.shift,
            Self::NoShift => !modifiers.shift,
            Self::Ctrl => modifiers.ctrl,
            Self::CtrlNoShift => modifiers.ctrl && !modifiers.shift,
            Self::CtrlShift => modifiers.ctrl && modifiers.shift,
            Self::Alt => modifiers.alt,
        }
    }
}

/// What the keyboard is talking to this frame.
#[derive(Clone, Copy, Debug)]
enum Focus {
    /// The lightbox is open; it suppresses everything but its own zoom keys.
    Lightbox,
    /// The find bar is open over the document.
    FindBar,
    /// The document, with no overlay taking keys.
    Reading,
}

/// The focus states a binding is live in.
#[derive(Clone, Copy, Debug)]
enum Scope {
    Lightbox,
    /// The document, with or without the find bar.
    Document,
    FindBar,
    Reading,
}

impl Scope {
    const fn covers(self, focus: Focus) -> bool {
        matches!(
            (self, focus),
            (Self::Lightbox, Focus::Lightbox)
                | (Self::Document | Self::FindBar, Focus::FindBar)
                | (Self::Document | Self::Reading, Focus::Reading)
        )
    }
}

/// Every keyboard shortcut. Rows sit in the order their actions run when
/// several fire in one frame.
const BINDINGS: &[(Chord, Key, Scope, AppAction)] = &[
    // Ctrl+0 resets the lightbox zoom only; the document has its own reset.
    (
        Chord::Ctrl,
        Key::Num0,
        Scope::Lightbox,
        AppAction::ResetLightboxZoom,
    ),
    // Ctrl+/- zoom applies to the lightbox when open, the document otherwise.
    // Steps are multiplicative (×1.25) like browsers.
    (
        Chord::Ctrl,
        Key::Plus,
        Scope::Lightbox,
        AppAction::ZoomLightboxBy(ZOOM_STEP),
    ),
    (
        Chord::Ctrl,
        Key::Equals,
        Scope::Lightbox,
        AppAction::ZoomLightboxBy(ZOOM_STEP),
    ),
    (
        Chord::Ctrl,
        Key::Minus,
        Scope::Lightbox,
        AppAction::ZoomLightboxBy(1.0 / ZOOM_STEP),
    ),
    (
        Chord::Ctrl,
        Key::Plus,
        Scope::Document,
        AppAction::ZoomBy(ZOOM_STEP),
    ),
    (
        Chord::Ctrl,
        Key::Equals,
        Scope::Document,
        AppAction::ZoomBy(ZOOM_STEP),
    ),
    (
        Chord::Ctrl,
        Key::Minus,
        Scope::Document,
        AppAction::ZoomBy(1.0 / ZOOM_STEP),
    ),
    (
        Chord::Ctrl,
        Key::Num0,
        Scope::Document,
        AppAction::ResetZoom,
    ),
    (
        Chord::CtrlNoShift,
        Key::O,
        Scope::Document,
        AppAction::OpenFile,
    ),
    (Chord::Ctrl, Key::T, Scope::Document, AppAction::OpenFile),
    (Chord::Any, Key::F5, Scope::Document, AppAction::ToggleWatch),
    (
        Chord::Ctrl,
        Key::D,
        Scope::Document,
        AppAction::ToggleDarkMode,
    ),
    (
        Chord::CtrlShift,
        Key::O,
        Scope::Document,
        AppAction::ToggleOutline,
    ),
    (
        Chord::CtrlShift,
        Key::E,
        Scope::Document,
        AppAction::ToggleExplorer,
    ),
    (Chord::Ctrl, Key::Q, Scope::Document, AppAction::Quit),
    (Chord::CtrlNoShift, Key::S, Scope::Document, AppAction::Save),
    (Chord::Ctrl, Key::W, Scope::Document, AppAction::CloseTab),
    (
        Chord::CtrlNoShift,
        Key::Tab,
        Scope::Document,
        AppAction::NextTab,
    ),
    (
        Chord::CtrlShift,
        Key::Tab,
        Scope::Document,
        AppAction::PreviousTab,
    ),
    (
        Chord::Ctrl,
        Key::Num1,
        Scope::Document,
        AppAction::FocusTab(0),
    ),
    (
        Chord::Ctrl,
        Key::Num2,
        Scope::Document,
        AppAction::FocusTab(1),
    ),
    (
        Chord::Ctrl,
        Key::Num3,
        Scope::Document,
        AppAction::FocusTab(2),
    ),
    (
        Chord::Ctrl,
        Key::Num4,
        Scope::Document,
        AppAction::FocusTab(3),
    ),
    (
        Chord::Ctrl,
        Key::Num5,
        Scope::Document,
        AppAction::FocusTab(4),
    ),
    (
        Chord::Ctrl,
        Key::Num6,
        Scope::Document,
        AppAction::FocusTab(5),
    ),
    (
        Chord::Ctrl,
        Key::Num7,
        Scope::Document,
        AppAction::FocusTab(6),
    ),
    (
        Chord::Ctrl,
        Key::Num8,
        Scope::Document,
        AppAction::FocusTab(7),
    ),
    (
        Chord::Ctrl,
        Key::Num9,
        Scope::Document,
        AppAction::FocusTab(8),
    ),
    (
        Chord::Alt,
        Key::ArrowLeft,
        Scope::Document,
        AppAction::NavigateBack,
    ),
    (
        Chord::Alt,
        Key::ArrowRight,
        Scope::Document,
        AppAction::NavigateForward,
    ),
    // Opens the find bar, or refocuses it if already open.
    (
        Chord::CtrlNoShift,
        Key::F,
        Scope::Document,
        AppAction::OpenSearch,
    ),
    // While the find bar is open it takes Enter / Shift+Enter / ↑↓ / Esc.
    // Up/Down are safe to bind even when the singleline TextEdit has focus
    // because it doesn't use vertical arrows for cursor movement.
    (
        Chord::NoShift,
        Key::Enter,
        Scope::FindBar,
        AppAction::NextMatch,
    ),
    (
        Chord::Any,
        Key::ArrowDown,
        Scope::FindBar,
        AppAction::NextMatch,
    ),
    (
        Chord::Shift,
        Key::Enter,
        Scope::FindBar,
        AppAction::PreviousMatch,
    ),
    (
        Chord::Any,
        Key::ArrowUp,
        Scope::FindBar,
        AppAction::PreviousMatch,
    ),
    (
        Chord::Any,
        Key::Escape,
        Scope::FindBar,
        AppAction::CloseSearch,
    ),
    // Plain document scroll keys. The arrows stay with search navigation
    // while the find bar is open; the page keys scroll either way.
    (
        Chord::Unmodified,
        Key::ArrowUp,
        Scope::Reading,
        AppAction::Scroll(KeyboardScrollAction::LineUp),
    ),
    (
        Chord::Unmodified,
        Key::ArrowDown,
        Scope::Reading,
        AppAction::Scroll(KeyboardScrollAction::LineDown),
    ),
    (
        Chord::Unmodified,
        Key::PageUp,
        Scope::Document,
        AppAction::Scroll(KeyboardScrollAction::PageUp),
    ),
    (
        Chord::Unmodified,
        Key::PageDown,
        Scope::Document,
        AppAction::Scroll(KeyboardScrollAction::PageDown),
    ),
];

/// The actions this frame's input asks for, in the order they run. Each kind
/// of action is queued once per frame: when two bindings for the same kind
/// fire together, the later row of [`BINDINGS`] replaces the earlier one.
fn shortcut_actions(input: &egui::InputState, focus: Focus) -> Vec<AppAction> {
    let mut actions: Vec<AppAction> = Vec::new();
    for &(chord, key, scope, action) in BINDINGS {
        if scope.covers(focus) && chord.is_held(input.modifiers) && input.key_pressed(key) {
            let kind = mem::discriminant(&action);
            match actions
                .iter_mut()
                .find(|queued| mem::discriminant(*queued) == kind)
            {
                Some(queued) => *queued = action,
                None => actions.push(action),
            }
        }
    }

    // Ctrl + scroll wheel / pinch to zoom (multiplicative, like browsers).
    // The lightbox captures wheel input itself in raw_input_hook, so this
    // only runs for the document.
    let wheel_zoom_factor = input.zoom_delta();
    if !matches!(focus, Focus::Lightbox) && (wheel_zoom_factor - 1.0).abs() > f32::EPSILON {
        actions.push(AppAction::ZoomBy(wheel_zoom_factor));
    }
    actions
}

impl MarkdownApp {
    /// Carry out the actions this frame's keyboard and wheel input ask for.
    pub(crate) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let focus = if self.lightbox.is_some() {
            Focus::Lightbox
        } else if self.search.is_open {
            Focus::FindBar
        } else {
            Focus::Reading
        };
        let actions = ctx.input(|input| shortcut_actions(input, focus));
        for action in actions {
            self.apply_action(ctx, action);
        }
    }

    /// Track drag hover and open dropped markdown files.
    pub(crate) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        // Handle drag and drop
        self.is_dragging = false;
        ctx.input(|i| {
            if !i.raw.hovered_files.is_empty() {
                self.is_dragging = true;
            }

            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    if Self::is_markdown_file(path) {
                        self.open_in_new_tab(path.clone());
                    } else {
                        self.error_message = Some(
                            "Unsupported file type. Please drop a markdown file (.md, .markdown, .txt)".to_string(),
                        );
                    }
                }
            }
        });
    }
}
