//! A private X display for one test, and the windows on it.

use std::fmt::Write as _;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use super::{wait_for, Home};

/// What the real-window tests run: a virtual display, a way to press keys
/// on it, a way to photograph it, and a pseudo-terminal.
const TOOLS: [&str; 5] = ["Xvfb", "xdotool", "import", "convert", "script"];

/// Set to run the suite on a machine without `TOOLS`.
const SKIP: &str = "MD_VIEWER_SKIP_WINDOW_TESTS";

fn is_installed(tool: &str) -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path).any(|dir| dir.join(tool).is_file())
}

/// `md-viewer <args>` typed at a terminal: `script` gives the process a
/// pseudo-terminal for its standard streams, and exits with its status.
pub(crate) fn in_a_terminal(home: &Home, args: &[&str]) -> Command {
    let mut typed = format!("'{}'", env!("CARGO_BIN_EXE_md-viewer"));
    for arg in args {
        let _ = write!(typed, " '{arg}'");
    }
    let mut command = Command::new("script");
    command.args(["-qec", &typed, "/dev/null"]);
    home.settle_in(&mut command);
    command.stdout(Stdio::null()).stderr(Stdio::null());
    command
}

/// End the process `pid`, which the test did not start itself.
#[track_caller]
pub(crate) fn kill(pid: &str) {
    let killed = Command::new("kill").arg(pid).status();
    assert!(
        killed.is_ok_and(|status| status.success()),
        "could not kill {pid}"
    );
}

/// A virtual X display of this test's own, stopped when dropped. Every
/// window on it goes with it.
pub(crate) struct Screen {
    xvfb: Child,
    display: String,
}

impl Screen {
    /// Start a display on a number nobody else uses.
    ///
    /// Without the tools this fails, and says what to install. A machine
    /// that cannot have them sets `MD_VIEWER_SKIP_WINDOW_TESTS`, and then
    /// this returns `None` for the test to return on.
    #[track_caller]
    pub(crate) fn start() -> Option<Self> {
        let missing: Vec<_> = TOOLS.iter().filter(|tool| !is_installed(tool)).collect();
        if !missing.is_empty() {
            let need = format!(
                "the real-window tests need {missing:?} on PATH \
                 (Debian and Ubuntu: apt-get install xvfb xdotool imagemagick)"
            );
            assert!(
                std::env::var_os(SKIP).is_some(),
                "{need}; set {SKIP}=1 to skip them"
            );
            let _ = writeln!(std::io::stderr(), "SKIPPED, not passed: {need}");
            return None;
        }

        // `-displayfd` has Xvfb pick a free display and report it once the
        // display accepts connections. `-noreset` keeps the display as it
        // is when its last window closes: a server that resets itself then
        // can refuse the app a test starts next.
        let mut xvfb = Command::new("Xvfb")
            .args(["-displayfd", "1", "-screen", "0", "1600x1000x24"])
            .args(["-nolisten", "tcp", "-noreset"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut number = String::new();
        BufReader::new(xvfb.stdout.take().unwrap())
            .read_line(&mut number)
            .unwrap();
        let number = number.trim();
        assert!(!number.is_empty(), "Xvfb did not start a display");
        Some(Self {
            xvfb,
            display: format!(":{number}"),
        })
    }

    /// Have `command` open its windows on this display, over X11 and with
    /// software rendering.
    pub(crate) fn show(&self, command: &mut Command) {
        command
            .env("DISPLAY", &self.display)
            .env("WINIT_UNIX_BACKEND", "x11")
            .env("LIBGL_ALWAYS_SOFTWARE", "1");
    }

    fn tool(&self, name: &str) -> Command {
        let mut command = Command::new(name);
        command.env("DISPLAY", &self.display).stdin(Stdio::null());
        command
    }

    /// What `xdotool <args>` prints, or nothing if it fails.
    fn xdotool(&self, args: &[&str]) -> String {
        let output = self.tool("xdotool").args(args).output().unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    /// The md-viewer windows on this display.
    fn windows(&self) -> Vec<Window<'_>> {
        let ids = self.xdotool(&["search", "--class", "md-viewer"]);
        let windows = ids.lines().map(|id| Window {
            screen: self,
            id: id.to_owned(),
        });
        windows.collect()
    }

    /// Wait for an md-viewer window whose title is exactly `title`.
    #[track_caller]
    pub(crate) fn window_titled(&self, title: &str) -> Window<'_> {
        let found = wait_for(|| {
            let mut windows = self.windows().into_iter();
            windows.find(|window| window.title() == title)
        });
        found.unwrap_or_else(|| {
            let titles: Vec<_> = self.windows().iter().map(Window::title).collect();
            panic!("no window titled {title:?}; the windows are {titles:?}")
        })
    }

    /// Wait until no md-viewer window is left on this display.
    pub(crate) fn has_no_window(&self) -> bool {
        wait_for(|| self.windows().is_empty().then_some(())).is_some()
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = self.xvfb.kill();
        let _ = self.xvfb.wait();
    }
}

/// One window on a [`Screen`].
pub(crate) struct Window<'screen> {
    screen: &'screen Screen,
    id: String,
}

impl Window<'_> {
    pub(crate) fn title(&self) -> String {
        self.screen.xdotool(&["getwindowname", &self.id])
    }

    /// The process that owns the window.
    pub(crate) fn pid(&self) -> String {
        self.screen.xdotool(&["getwindowpid", &self.id])
    }

    /// Give the window the keyboard, hold Ctrl, press `key`, and let go of
    /// Ctrl once `done` holds. Returns whether it ever did.
    ///
    /// A person holds Ctrl for many frames. Letting go at once can put the
    /// key and the release in one frame of a busy machine, and the app
    /// reads the modifiers once per frame.
    pub(crate) fn press_ctrl(&self, key: &str, mut done: impl FnMut() -> bool) -> bool {
        // No window manager runs here to hand out the keyboard, so the test
        // does, and waits until the display agrees.
        let focused = wait_for(|| {
            self.screen.xdotool(&["windowfocus", &self.id]);
            (self.screen.xdotool(&["getwindowfocus"]) == self.id).then_some(())
        });
        assert!(focused.is_some(), "the window never took the keyboard");
        self.screen.xdotool(&["keydown", "Control_L", "key", key]);
        let happened = wait_for(|| done().then_some(())).is_some();
        self.screen.xdotool(&["keyup", "Control_L"]);
        happened
    }

    /// The color of the pixel `x` right and `y` down from the window's top
    /// left corner, as `RRGGBB`, read from a screenshot saved in `scratch`.
    #[track_caller]
    pub(crate) fn pixel(&self, scratch: &Path, x: u32, y: u32) -> String {
        let shot = scratch.join(format!("window-{}.png", self.id));
        // A window can be titled a moment before it can be photographed.
        let taken = wait_for(|| {
            let mut import = self.screen.tool("import");
            import.args(["-window", &self.id]).arg(&shot);
            let taken = import.stderr(Stdio::null()).status();
            taken.is_ok_and(|status| status.success()).then_some(())
        });
        assert!(taken.is_some(), "no screenshot");
        let output = self
            .screen
            .tool("convert")
            .arg(&shot)
            .args(["-depth", "8", "-format"])
            .arg(format!("%[hex:p{{{x},{y}}}]"))
            .arg("info:")
            .output()
            .unwrap();
        let hex = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_uppercase();
        hex.chars().take(6).collect()
    }

    /// Wait until the pixel at `x`, `y` is `color`. Returns whether it
    /// ever was.
    pub(crate) fn pixel_becomes(&self, scratch: &Path, x: u32, y: u32, color: &str) -> bool {
        wait_for(|| (self.pixel(scratch, x, y) == color).then_some(())).is_some()
    }
}
