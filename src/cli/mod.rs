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

pub(crate) const fn should_detach(args: &Args, launched_from_terminal: bool) -> bool {
    launched_from_terminal && !args.foreground && !args.no_detach
}

pub(crate) fn launched_from_terminal() -> bool {
    io::stdin().is_terminal() || io::stdout().is_terminal() || io::stderr().is_terminal()
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

pub(crate) fn spawn_detached_child() -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let child_args = child_args_with_no_detach(std::env::args_os());
    let mut command = Command::new(exe);

    command
        .args(child_args)
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

/// Build the native options and hand the app to eframe. Called once per
/// process: if the event loop cannot be created, winit forbids a second
/// attempt (`RecreationAttempt`), which is why backend selection is settled
/// up front instead of via retry.
pub(crate) fn run_native_once(args: &Args) -> eframe::Result<()> {
    // Calculate optimal window width assuming both sidebars are shown (the default)
    let optimal_width =
        CONTENT_OPTIMAL_WIDTH + EXPLORER_DEFAULT_WIDTH + OUTLINE_DEFAULT_WIDTH + PANEL_SEPARATORS;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([optimal_width, OPTIMAL_WINDOW_HEIGHT])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Markdown Viewer")
            // Wayland app_id: GNOME matches running windows to pinned dock
            // icons via app_id (the Wayland WM_CLASS equivalent); without it
            // the launcher shows a second generic icon (#62). Must match the
            // StartupWMClass in the distributed .desktop file.
            .with_app_id("md-viewer")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    let file = args.file.clone();
    let watch = !args.no_watch;

    eframe::run_native(
        "md-viewer",
        options,
        Box::new(move |cc| Ok(Box::new(MarkdownApp::new(cc, file, watch)))),
    )
}

/// winit 0.30 selects exactly one windowing backend from the environment and
/// never falls back: a non-empty `WAYLAND_DISPLAY`/`WAYLAND_SOCKET` commits it to
/// Wayland, and an unusable socket then aborts startup even when Xwayland is
/// available (seen in strict snaps on Ubuntu 26.04, #65). Probe the socket
/// before handing control to eframe; if it cannot be connected, clear the
/// Wayland variables so winit picks X11 up front.
#[cfg(unix)]
pub(crate) fn fall_back_to_x11_if_wayland_socket_is_dead() {
    // An inherited WAYLAND_SOCKET fd cannot be probed without consuming it;
    // trust it and let libwayland report any problem.
    if std::env::var_os("WAYLAND_SOCKET").is_some_and(|value| !value.is_empty()) {
        return;
    }

    let display = std::env::var("WAYLAND_DISPLAY")
        .ok()
        .filter(|v| !v.is_empty());
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").unwrap_or_default();
    let Some(path) = wayland_display_socket_path(display.as_deref(), &runtime_dir) else {
        return;
    };

    if UnixStream::connect(&path).is_ok() {
        return;
    }

    log::warn!(
        "WAYLAND_DISPLAY points at {}, which cannot be connected to; falling back to X11",
        path.display()
    );
    std::env::remove_var("WAYLAND_DISPLAY");
    std::env::remove_var("WAYLAND_SOCKET");
}

#[cfg(not(unix))]
pub(crate) fn fall_back_to_x11_if_wayland_socket_is_dead() {}

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
