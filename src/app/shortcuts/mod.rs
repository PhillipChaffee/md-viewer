//! Keyboard shortcuts and dropped files.

use std::mem;
use std::path::Path;

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
    // The lightbox takes over the zoom keys. Its Ctrl+0 resets the lightbox
    // zoom only; the document has its own reset.
    Ctrl + Num0 in Lightbox => AppAction::ResetLightboxZoom,
    Ctrl + Plus in Lightbox => AppAction::ZoomLightboxBy(ZOOM_IN),
    Ctrl + Equals in Lightbox => AppAction::ZoomLightboxBy(ZOOM_IN),
    Ctrl + Minus in Lightbox => AppAction::ZoomLightboxBy(ZOOM_OUT),

    // Zoom steps are multiplicative (×1.25) like browsers and the lightbox.
    Ctrl + Plus in Document => AppAction::ZoomBy(ZOOM_IN),
    Ctrl + Equals in Document => AppAction::ZoomBy(ZOOM_IN),
    Ctrl + Minus in Document => AppAction::ZoomBy(ZOOM_OUT),
    Ctrl + Num0 in Document => AppAction::ResetZoom,

    CtrlNoShift + O in Document => AppAction::OpenFile,
    Ctrl + T in Document => AppAction::OpenFile,
    Any + F5 in Document => AppAction::ToggleWatch,
    Ctrl + D in Document => AppAction::ToggleDarkMode,
    CtrlShift + O in Document => AppAction::ToggleOutline,
    CtrlShift + E in Document => AppAction::ToggleExplorer,
    Ctrl + Q in Document => AppAction::Quit,
    CtrlNoShift + S in Document => AppAction::Save,

    Ctrl + W in Document => AppAction::CloseTab,
    CtrlNoShift + Tab in Document => AppAction::NextTab,
    CtrlShift + Tab in Document => AppAction::PreviousTab,
    Ctrl + Num1 in Document => AppAction::FocusTab(0),
    Ctrl + Num2 in Document => AppAction::FocusTab(1),
    Ctrl + Num3 in Document => AppAction::FocusTab(2),
    Ctrl + Num4 in Document => AppAction::FocusTab(3),
    Ctrl + Num5 in Document => AppAction::FocusTab(4),
    Ctrl + Num6 in Document => AppAction::FocusTab(5),
    Ctrl + Num7 in Document => AppAction::FocusTab(6),
    Ctrl + Num8 in Document => AppAction::FocusTab(7),
    Ctrl + Num9 in Document => AppAction::FocusTab(8),
    Alt + ArrowLeft in Document => AppAction::NavigateBack,
    Alt + ArrowRight in Document => AppAction::NavigateForward,

    // Opens the find bar, or refocuses it if already open.
    CtrlNoShift + F in Document => AppAction::OpenSearch,
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

/// Whether a dropped file is one the viewer opens, judged by its extension.
fn is_markdown_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        let ext = ext.to_string_lossy().to_lowercase();
        ext == "md" || ext == "markdown" || ext == "txt"
    })
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
                    if is_markdown_file(path) {
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
