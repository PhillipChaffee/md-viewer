//! Command-line arguments, terminal detach, and native window startup.

use std::ffi::{OsStr, OsString};
use std::io::{self, IsTerminal};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Parser;
use eframe::egui;

use crate::app::MarkdownApp;
use crate::layout::{
    CONTENT_OPTIMAL_WIDTH, EXPLORER_DEFAULT_WIDTH, OPTIMAL_WINDOW_HEIGHT, OUTLINE_DEFAULT_WIDTH,
    PANEL_SEPARATORS,
};

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

#[derive(Parser, Debug)]
#[command(name = "md-viewer")]
#[command(version)]
#[command(about = "A lightweight markdown viewer", long_about = None)]
pub(crate) struct Args {
    /// Markdown file to open.
    file: Option<PathBuf>,

    /// Disable live reload (watching is enabled by default).
    #[arg(long)]
    no_watch: bool,

    /// Keep the GUI process attached to the terminal for debugging/logs.
    #[arg(long)]
    foreground: bool,

    /// Internal marker used by the detached child process to avoid respawn loops.
    #[arg(long, hide = true)]
    no_detach: bool,
}

impl Args {
    /// Whether the opened documents reload when their files change.
    const fn watch(&self) -> bool {
        !self.no_watch
    }
}

/// Wayland `app_id` and X11 class of the window. GNOME matches running
/// windows to pinned dock icons through it; without it the launcher shows a
/// second generic icon (#62). Must match the `StartupWMClass` in the
/// distributed .desktop file.
pub(crate) const APP_ID: &str = "md-viewer";

pub(crate) const fn should_detach(args: &Args, launched_from_terminal: bool) -> bool {
    launched_from_terminal && !args.foreground && !args.no_detach
}

/// Whether any of the three standard streams is attached to a terminal, which
/// is how a launch from a shell differs from one from a desktop launcher.
pub(crate) fn launched_from_terminal(standard_streams: [&dyn IsTerminal; 3]) -> bool {
    standard_streams.iter().any(|stream| stream.is_terminal())
}

fn child_args_with_no_detach<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut child_args: Vec<OsString> = args.into_iter().skip(1).collect();
    let marker_position = child_args
        .iter()
        .position(|arg| arg == OsStr::new("--"))
        .unwrap_or(child_args.len());
    child_args.insert(marker_position, OsString::from("--no-detach"));
    child_args
}

/// Start `exe` again with this process's `args` (program name first), cut
/// loose from the terminal: no standard streams, its own process group, and
/// the marker that stops the copy from detaching once more.
pub(crate) fn spawn_detached(exe: PathBuf, args: Vec<OsString>) -> io::Result<()> {
    let mut command = Command::new(exe);

    command
        .args(child_args_with_no_detach(args))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        // A new process group keeps the child out of the launching shell's
        // job, so it survives after the terminal hands control back.
        command.process_group(0);
    }

    command.spawn()?;
    Ok(())
}

/// Start the viewer. A launch from a terminal hands over to a detached copy
/// and returns at once, so the shell gets its prompt back; any other launch,
/// and one whose hand-over fails, opens the window in this process.
pub(crate) fn launch(
    args: &Args,
    from_terminal: bool,
    spawn_detached_copy: &dyn Fn() -> io::Result<()>,
    open_window: &dyn Fn(&Args) -> eframe::Result<()>,
) -> eframe::Result<()> {
    if should_detach(args, from_terminal) {
        match spawn_detached_copy() {
            Ok(()) => return Ok(()),
            // `error` is the one level env_logger prints when `RUST_LOG` is
            // unset, so whoever launched from the terminal still sees why
            // the window stayed attached to it.
            Err(err) => {
                log::error!("Failed to detach md-viewer process: {err}. Running in foreground.");
            }
        }
    }
    open_window(args)
}

/// Window settings handed to eframe. eframe is started once per process: if
/// the event loop cannot be created, winit forbids a second attempt
/// (`RecreationAttempt`), which is why backend selection is settled up front
/// instead of via retry.
pub(crate) fn native_options() -> eframe::NativeOptions {
    // Calculate optimal window width assuming both sidebars are shown (the default)
    let optimal_width =
        CONTENT_OPTIMAL_WIDTH + EXPLORER_DEFAULT_WIDTH + OUTLINE_DEFAULT_WIDTH + PANEL_SEPARATORS;

    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([optimal_width, OPTIMAL_WINDOW_HEIGHT])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Markdown Viewer")
            .with_app_id(APP_ID)
            .with_drag_and_drop(true),
        ..Default::default()
    }
}

/// The constructor eframe calls once the window exists: the app opens the
/// file named on the command line and watches it unless told not to.
pub(crate) fn app_creator(args: &Args) -> eframe::AppCreator<'static> {
    let file = args.file.clone();
    let watch = args.watch();
    Box::new(move |cc| Ok(Box::new(MarkdownApp::new(cc, file, watch))))
}

/// winit 0.30 selects exactly one windowing backend from the environment and
/// never falls back: a non-empty `WAYLAND_DISPLAY`/`WAYLAND_SOCKET` commits it to
/// Wayland, and an unusable socket then aborts startup even when Xwayland is
/// available (seen in strict snaps on Ubuntu 26.04, #65). Probe the socket
/// before handing control to eframe; if it cannot be connected, the Wayland
/// variables have to go so winit picks X11 up front.
///
/// Returns the environment variables to clear, which is none while Wayland
/// is usable or was never configured. `var` reads an environment variable;
/// clearing is left to the caller that owns the process environment.
#[cfg(unix)]
pub(crate) fn dead_wayland_variables(
    var: &dyn Fn(&'static str) -> Option<OsString>,
) -> &'static [&'static str] {
    // An inherited WAYLAND_SOCKET fd cannot be probed without consuming it;
    // trust it and let libwayland report any problem.
    if var("WAYLAND_SOCKET").is_some_and(|value| !value.is_empty()) {
        return &[];
    }

    let display = var("WAYLAND_DISPLAY")
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty());
    let runtime_dir = var("XDG_RUNTIME_DIR").unwrap_or_default();
    let Some(path) = wayland_display_socket_path(display.as_deref(), &runtime_dir) else {
        return &[];
    };

    if UnixStream::connect(&path).is_ok() {
        return &[];
    }

    log::warn!(
        "WAYLAND_DISPLAY points at {}, which cannot be connected to; falling back to X11",
        path.display()
    );
    &["WAYLAND_DISPLAY", "WAYLAND_SOCKET"]
}

#[cfg(not(unix))]
pub(crate) fn dead_wayland_variables(
    _var: &dyn Fn(&'static str) -> Option<OsString>,
) -> &'static [&'static str] {
    &[]
}

/// Resolve the filesystem path of the Wayland socket from `WAYLAND_DISPLAY`
/// and `XDG_RUNTIME_DIR`, mirroring libwayland's lookup rules: an absolute
/// display name passes through, otherwise it is joined onto the runtime
/// directory. Returns `None` when the pair does not describe a concrete
/// socket — Wayland was never usable in that case, so there is nothing to
/// probe or clear.
fn wayland_display_socket_path(display: Option<&str>, runtime_dir: &OsStr) -> Option<PathBuf> {
    let name = Path::new(display?);
    if name.as_os_str().is_empty() {
        return None;
    }
    if name.is_absolute() {
        return Some(name.to_path_buf());
    }
    if runtime_dir.is_empty() {
        return None;
    }
    Some(Path::new(runtime_dir).join(name))
}
