//! Native file and folder pickers, with a Tk fallback when no portal is available.

use std::ffi::OsString;
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(test)]
mod tests;

/// Check whether the desktop portal exposes the interface used by rfd's
/// Linux folder picker. Calling rfd without this interface fails like a user
/// cancellation, so checking first lets us distinguish that case and use a
/// local fallback without opening a second dialog after a real cancellation.
#[cfg(target_os = "linux")]
fn portal_file_chooser_available() -> bool {
    Command::new("gdbus")
        .args([
            "introspect",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|output| {
            output.status.success() && portal_introspection_has_file_chooser(&output.stdout)
        })
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn portal_introspection_has_file_chooser(stdout: &[u8]) -> bool {
    String::from_utf8_lossy(stdout).contains("org.freedesktop.portal.FileChooser")
}

#[cfg(target_os = "linux")]
fn path_from_picker_output(stdout: Vec<u8>) -> Option<PathBuf> {
    if stdout.is_empty() {
        None
    } else {
        // Python writes os.fsencode(path), so preserve arbitrary Unix path
        // bytes rather than requiring the selected directory to be UTF-8.
        Some(PathBuf::from(OsString::from_vec(stdout)))
    }
}

/// Use Python's Tk binding as a fallback file picker on Linux desktops where
/// xdg-desktop-portal is not available.
#[cfg(target_os = "linux")]
fn pick_file_with_tkinter(initial_dir: Option<&Path>) -> Result<Option<PathBuf>, String> {
    const SCRIPT: &str = r#"
import os
import sys
import tkinter as tk
from tkinter import filedialog

root = tk.Tk()
root.withdraw()
try:
    options = {
        "title": "Open Markdown File",
        "parent": root,
        "filetypes": [
            ("Markdown", ("*.md", "*.markdown")),
            ("Text", "*.txt"),
            ("All Files", "*"),
        ],
    }
    if len(sys.argv) > 1 and os.path.isdir(sys.argv[1]):
        options["initialdir"] = sys.argv[1]
    selected = filedialog.askopenfilename(**options)
    if selected:
        sys.stdout.buffer.write(os.fsencode(selected))
finally:
    root.destroy()
"#;

    let mut command = Command::new("python3");
    command
        .arg("-c")
        .arg(SCRIPT)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    if let Some(directory) = initial_dir.filter(|path| path.is_dir()) {
        command.arg(directory);
    }

    let output = command
        .output()
        .map_err(|error| format!("Tkinter file picker could not start: {error}"))?;

    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if details.is_empty() {
            "Tkinter file picker exited with an error".to_string()
        } else {
            format!("Tkinter file picker failed: {details}")
        });
    }

    let selected = path_from_picker_output(output.stdout);
    if let Some(path) = &selected {
        if !path.is_file() {
            return Err(format!(
                "File picker returned a file that does not exist: {}",
                path.display()
            ));
        }
    }
    Ok(selected)
}

pub(crate) fn pick_file(initial_dir: Option<&Path>) -> Result<Option<PathBuf>, String> {
    let mut dialog = rfd::FileDialog::new()
        .add_filter("Markdown", &["md", "markdown"])
        .add_filter("Text", &["txt"])
        .add_filter("All Files", &["*"]);
    if let Some(directory) = initial_dir.filter(|path| path.is_dir()) {
        dialog = dialog.set_directory(directory);
    }

    #[cfg(target_os = "linux")]
    {
        if portal_file_chooser_available() {
            return Ok(dialog.pick_file());
        }

        log::info!("XDG Desktop Portal FileChooser is unavailable; using the Tkinter file picker");
        pick_file_with_tkinter(initial_dir)
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(dialog.pick_file())
    }
}

/// Use Python's Tk binding as a fallback folder picker on Linux desktops where
/// xdg-desktop-portal is not available. The initial directory is passed as a
/// process argument, never interpolated into Python.
#[cfg(target_os = "linux")]
fn pick_folder_with_tkinter(initial_dir: Option<&Path>) -> Result<Option<PathBuf>, String> {
    const SCRIPT: &str = r#"
import os
import sys
import tkinter as tk
from tkinter import filedialog

root = tk.Tk()
root.withdraw()
try:
    options = {"title": "Open Folder", "mustexist": True, "parent": root}
    if len(sys.argv) > 1 and os.path.isdir(sys.argv[1]):
        options["initialdir"] = sys.argv[1]
    selected = filedialog.askdirectory(**options)
    if selected:
        sys.stdout.buffer.write(os.fsencode(selected))
finally:
    root.destroy()
"#;

    let mut command = Command::new("python3");
    command
        .arg("-c")
        .arg(SCRIPT)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    if let Some(directory) = initial_dir.filter(|path| path.is_dir()) {
        command.arg(directory);
    }

    let output = command
        .output()
        .map_err(|error| format!("Tkinter folder picker could not start: {error}"))?;

    if !output.status.success() {
        let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if details.is_empty() {
            "Tkinter folder picker exited with an error".to_string()
        } else {
            format!("Tkinter folder picker failed: {details}")
        });
    }

    let selected = path_from_picker_output(output.stdout);
    if let Some(path) = &selected {
        if !path.is_dir() {
            return Err(format!(
                "Folder picker returned a directory that does not exist: {}",
                path.display()
            ));
        }
    }
    Ok(selected)
}

pub(crate) fn pick_folder(initial_dir: Option<&Path>) -> Result<Option<PathBuf>, String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(directory) = initial_dir.filter(|path| path.is_dir()) {
        dialog = dialog.set_directory(directory);
    }

    #[cfg(target_os = "linux")]
    {
        if portal_file_chooser_available() {
            return Ok(dialog.pick_folder());
        }

        log::info!(
            "XDG Desktop Portal FileChooser is unavailable; using the Tkinter folder picker"
        );
        pick_folder_with_tkinter(initial_dir)
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(dialog.pick_folder())
    }
}
