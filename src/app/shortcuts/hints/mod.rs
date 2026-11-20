//! How the shortcuts read on screen: "Ctrl+Shift+E" on Linux and Windows,
//! "⇧⌘E" on a Mac. Every hint in a menu, a tooltip or the home page is
//! written here from the binding table, so a hint cannot name a key the
//! app does not answer to.

use eframe::egui::{self, Key};

use super::{Chord, BINDINGS};
use crate::app::action::AppAction;
use crate::app::chrome;
use crate::app::MarkdownApp;
use crate::platform::Platform;

#[cfg(test)]
mod tests;

/// The signs a Mac's hints are written with. A font that lacks one of them
/// would draw an empty box, so hints fall back to names there.
const MAC_SIGNS: &str = "⌃⌥⇧⌘←→↑↓";

/// How a hint is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Notation {
    /// "Ctrl+Shift+E", off the Mac.
    Names,
    /// "⇧⌘E", on a Mac.
    MacSigns,
    /// "Shift+Cmd+E", on a Mac whose font has no modifier signs.
    MacNames,
}

impl Notation {
    /// How to write hints on `platform` in a font that has the Mac's signs
    /// or does not.
    pub(crate) const fn on(platform: Platform, font_has_signs: bool) -> Self {
        match (platform, font_has_signs) {
            (Platform::Other, _) => Self::Names,
            (Platform::Mac, true) => Self::MacSigns,
            (Platform::Mac, false) => Self::MacNames,
        }
    }

    const fn platform(self) -> Platform {
        match self {
            Self::Names => Platform::Other,
            Self::MacSigns | Self::MacNames => Platform::Mac,
        }
    }

    /// What goes between the parts of a hint.
    const fn joint(self) -> &'static str {
        match self {
            Self::Names | Self::MacNames => "+",
            Self::MacSigns => "",
        }
    }

    /// The modifier keys `chord` asks for, in the order the platform lists
    /// them: Ctrl, Alt, Shift off the Mac, and ⌃ ⌥ ⇧ ⌘ on one.
    fn modifiers(self, chord: Chord) -> impl Iterator<Item = &'static str> {
        let command = matches!(
            chord,
            Chord::Cmd | Chord::CmdNoShift | Chord::CmdShift | Chord::CmdAlt | Chord::MacCmd
        );
        let ctrl = matches!(chord, Chord::CtrlNoShift | Chord::CtrlShift);
        let alt = matches!(chord, Chord::CmdAlt | Chord::AltNoCtrl);
        let shift = matches!(chord, Chord::Shift | Chord::CmdShift | Chord::CtrlShift);
        let held = match self {
            Self::Names => [(ctrl || command, "Ctrl"), (alt, "Alt"), (shift, "Shift")].to_vec(),
            Self::MacSigns => [(ctrl, "⌃"), (alt, "⌥"), (shift, "⇧"), (command, "⌘")].to_vec(),
            Self::MacNames => [
                (ctrl, "Ctrl"),
                (alt, "Option"),
                (shift, "Shift"),
                (command, "Cmd"),
            ]
            .to_vec(),
        };
        held.into_iter().filter_map(|(is, name)| is.then_some(name))
    }

    /// What `key` is called.
    fn key(self, key: Key) -> &'static str {
        match (self, key) {
            (_, Key::Plus) => "+",
            (_, Key::Minus) => "-",
            (_, Key::Equals) => "=",
            (_, Key::OpenBracket) => "[",
            (_, Key::CloseBracket) => "]",
            (Self::MacSigns, Key::ArrowLeft) => "←",
            (Self::MacSigns, Key::ArrowRight) => "→",
            (Self::MacSigns, Key::ArrowUp) => "↑",
            (Self::MacSigns, Key::ArrowDown) => "↓",
            (Self::MacSigns | Self::MacNames, Key::Enter) => "Return",
            _ => key.name(),
        }
    }

    /// `key` pressed with `chord`, as a hint.
    fn hint(self, chord: Chord, key: Key) -> String {
        let mut parts: Vec<&str> = self.modifiers(chord).collect();
        parts.push(self.key(key));
        parts.join(self.joint())
    }
}

/// The hint of every binding the platform has, written once.
#[derive(Debug)]
pub(crate) struct Hints(Vec<(AppAction, String)>);

impl Hints {
    /// The hints in `notation`, one for each row of the binding table its
    /// platform's keyboard can press, in table order.
    pub(crate) fn new(notation: Notation) -> Self {
        let rows = BINDINGS.iter();
        let hinted = rows
            .filter(|(chord, ..)| chord.exists_on(notation.platform()))
            .map(|&(chord, key, _, action)| (action, notation.hint(chord, key)));
        Self(hinted.collect())
    }

    /// Every way to ask for `action` from the keyboard, the first in the
    /// table first.
    pub(crate) fn all(&self, action: AppAction) -> impl Iterator<Item = &str> {
        let hints = self.0.iter();
        hints
            .filter(move |(bound, _)| *bound == action)
            .map(|(_, hint)| hint.as_str())
    }

    /// The shortcut for `action` a menu shows beside it: the first in the
    /// table. Empty for an action with no shortcut.
    pub(crate) fn of(&self, action: AppAction) -> &str {
        self.all(action).next().unwrap_or_default()
    }

    /// Show `name`, with every shortcut for `action` in brackets after it,
    /// while the pointer rests on `response`. The text is put together
    /// only while it shows.
    pub(crate) fn tooltip(
        &self,
        response: egui::Response,
        name: &str,
        action: AppAction,
    ) -> egui::Response {
        response.on_hover_ui(|ui| {
            ui.set_max_width(ui.spacing().tooltip_width);
            let keys: Vec<&str> = self.all(action).collect();
            ui.label(format!("{name} ({})", keys.join(" / ")));
        })
    }
}

impl MarkdownApp {
    /// Write the hints again for the fonts now in force: a Mac's are in
    /// signs only where the chrome's font has them all.
    pub(crate) fn rewrite_hints(&mut self, ctx: &egui::Context) {
        let font_has_signs = matches!(self.platform, Platform::Mac)
            && ctx.fonts_mut(|fonts| fonts.has_glyphs(&chrome::small_font(), MAC_SIGNS));
        self.hints = Hints::new(Notation::on(self.platform, font_has_signs));
    }
}
