//! The kind of machine the app runs on.

/// The kind of machine the app runs on, where that changes what it does:
/// which key makes a command, and whose title bar the window has. It is a
/// value the code is handed, so both kinds are tested on either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Platform {
    /// macOS, where ⌘ makes a command of a key.
    Mac,
    /// Linux and Windows, where Ctrl does.
    Other,
}

impl Platform {
    /// The machine this build is for: the one place the app asks.
    pub(crate) const HOST: Self = if cfg!(target_os = "macos") {
        Self::Mac
    } else {
        Self::Other
    };
}
