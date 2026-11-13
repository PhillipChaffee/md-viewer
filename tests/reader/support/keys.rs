//! Keyboard shortcuts written the way the README writes them.

use eframe::egui::{Key, Modifiers};

/// The modifier prefixes a shortcut may carry, and what each one holds
/// down. Ctrl also sets `command`, as the windowing layer does on Linux.
const PREFIXES: [(&str, Modifiers); 3] = [
    (
        "Ctrl+",
        Modifiers {
            alt: false,
            ctrl: true,
            shift: false,
            mac_cmd: false,
            command: true,
        },
    ),
    ("Shift+", Modifiers::SHIFT),
    ("Alt+", Modifiers::ALT),
];

/// Names the README uses that egui spells differently.
const ALIASES: [(&str, Key); 6] = [
    ("Left", Key::ArrowLeft),
    ("Right", Key::ArrowRight),
    ("Up", Key::ArrowUp),
    ("Down", Key::ArrowDown),
    ("Esc", Key::Escape),
    ("Del", Key::Delete),
];

/// Read a shortcut such as `Ctrl+Shift+Tab`, `Alt+Left`, `Ctrl++` or `F5`.
#[track_caller]
pub(crate) fn chord(shortcut: &str) -> (Modifiers, Key) {
    let mut modifiers = Modifiers::NONE;
    let mut rest = shortcut;
    while let Some((held, after)) = PREFIXES
        .iter()
        .find_map(|(prefix, held)| Some((*held, rest.strip_prefix(prefix)?)))
    {
        modifiers |= held;
        rest = after;
    }
    let alias = ALIASES.iter().find(|(name, _)| *name == rest);
    let key = alias
        .map(|(_, key)| *key)
        .or_else(|| Key::from_name(rest))
        .unwrap_or_else(|| panic!("`{shortcut}` names no key the driver knows"));
    (modifiers, key)
}
