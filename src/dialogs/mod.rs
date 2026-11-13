//! Native file and folder pickers, with a Tk fallback when no portal is available.

#[cfg(target_os = "linux")]
use std::ffi::OsString;
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};

#[cfg(test)]
mod tests;

/// A program and the arguments it is always started with.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
struct CommandLine<'a> {
    program: &'a str,
    arguments: &'a [&'a str],
}

#[cfg(target_os = "linux")]
impl CommandLine<'_> {
    fn command(self) -> Command {
        let mut command = Command::new(self.program);
        command.args(self.arguments);
        command
    }
}

/// What a picker reaches outside this process: the programs it runs and the
/// native dialogs it opens.
struct Desktop<'a> {
    /// Prints the D-Bus introspection of the desktop portal.
    #[cfg(target_os = "linux")]
    portal_probe: CommandLine<'a>,
    /// Runs the Python source given as its next argument.
    #[cfg(target_os = "linux")]
    python: CommandLine<'a>,
    /// Opens a native dialog and waits for the file the user picks.
    show_file_dialog: &'a dyn Fn(rfd::FileDialog) -> Option<PathBuf>,
    /// Opens a native dialog and waits for the folder the user picks.
    show_folder_dialog: &'a dyn Fn(rfd::FileDialog) -> Option<PathBuf>,
}

/// The desktop this process runs on.
const SYSTEM_DESKTOP: Desktop<'static> = Desktop {
    #[cfg(target_os = "linux")]
    portal_probe: CommandLine {
        program: "gdbus",
        arguments: &[
            "introspect",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
        ],
    },
    #[cfg(target_os = "linux")]
    python: CommandLine {
        program: "python3",
        arguments: &["-c"],
    },
    show_file_dialog: &rfd::FileDialog::pick_file,
    show_folder_dialog: &rfd::FileDialog::pick_folder,
};

/// Check whether the desktop portal exposes the interface used by rfd's
/// Linux folder picker. Calling rfd without this interface fails like a user
/// cancellation, so checking first lets us distinguish that case and use a
/// local fallback without opening a second dialog after a real cancellation.
#[cfg(target_os = "linux")]
fn portal_file_chooser_available(probe: CommandLine<'_>) -> bool {
    probe
        .command()
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| {
            output.status.success() && portal_introspection_has_file_chooser(&output.stdout)
        })
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

#[cfg(target_os = "linux")]
const TK_FILE_PICKER_SCRIPT: &str = r#"
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

#[cfg(target_os = "linux")]
const TK_FOLDER_PICKER_SCRIPT: &str = r#"
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

/// What the user is asked to pick.
#[derive(Clone, Copy)]
enum Picker {
    File,
    Folder,
}

impl Picker {
    /// The native dialog, opened in `initial_dir` when that is a directory.
    fn native_dialog(self, initial_dir: Option<&Path>) -> rfd::FileDialog {
        let dialog = match self {
            Self::File => rfd::FileDialog::new()
                .add_filter("Markdown", &["md", "markdown"])
                .add_filter("Text", &["txt"])
                .add_filter("All Files", &["*"]),
            Self::Folder => rfd::FileDialog::new(),
        };
        match initial_dir.filter(|path| path.is_dir()) {
            Some(directory) => dialog.set_directory(directory),
            None => dialog,
        }
    }

    /// Ask the user through `desktop`: its native dialog where the portal can
    /// serve one, Python's Tk dialog otherwise. `Ok(None)` means the user
    /// cancelled.
    fn pick(
        self,
        initial_dir: Option<&Path>,
        desktop: &Desktop<'_>,
    ) -> Result<Option<PathBuf>, String> {
        #[cfg(target_os = "linux")]
        if !portal_file_chooser_available(desktop.portal_probe) {
            log::info!(
                "XDG Desktop Portal FileChooser is unavailable; using the Tkinter {} picker",
                self.noun()
            );
            return self.pick_with_tk(desktop.python, initial_dir);
        }

        let show = match self {
            Self::File => desktop.show_file_dialog,
            Self::Folder => desktop.show_folder_dialog,
        };
        Ok(show(self.native_dialog(initial_dir)))
    }
}

/// Python's Tk binding as a fallback picker on Linux desktops where
/// xdg-desktop-portal is not available.
#[cfg(target_os = "linux")]
impl Picker {
    const fn script(self) -> &'static str {
        match self {
            Self::File => TK_FILE_PICKER_SCRIPT,
            Self::Folder => TK_FOLDER_PICKER_SCRIPT,
        }
    }

    /// What the picker selects, as its error messages name it.
    const fn noun(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
        }
    }

    /// Reject a selection that is not on disk as the kind this picker hands out.
    fn check_selection(self, path: &Path) -> Result<(), String> {
        let (exists, complaint) = match self {
            Self::File => (
                path.is_file(),
                "File picker returned a file that does not exist",
            ),
            Self::Folder => (
                path.is_dir(),
                "Folder picker returned a directory that does not exist",
            ),
        };
        if exists {
            Ok(())
        } else {
            Err(format!("{complaint}: {}", path.display()))
        }
    }

    /// Show the Tk picker through the `python` interpreter and wait for it.
    /// `Ok(None)` means the user cancelled. The initial directory is passed
    /// as a process argument, never interpolated into Python.
    fn pick_with_tk(
        self,
        python: CommandLine<'_>,
        initial_dir: Option<&Path>,
    ) -> Result<Option<PathBuf>, String> {
        let noun = self.noun();
        let mut command = python.command();
        command
            .arg(self.script())
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());
        if let Some(directory) = initial_dir.filter(|path| path.is_dir()) {
            command.arg(directory);
        }

        let output = command
            .output()
            .map_err(|error| format!("Tkinter {noun} picker could not start: {error}"))?;

        if !output.status.success() {
            let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if details.is_empty() {
                format!("Tkinter {noun} picker exited with an error")
            } else {
                format!("Tkinter {noun} picker failed: {details}")
            });
        }

        let selected = path_from_picker_output(output.stdout);
        if let Some(path) = &selected {
            self.check_selection(path)?;
        }
        Ok(selected)
    }
}

/// Ask the user for a Markdown file to open. `Ok(None)` means they cancelled.
pub(crate) fn pick_file(initial_dir: Option<&Path>) -> Result<Option<PathBuf>, String> {
    Picker::File.pick(initial_dir, &SYSTEM_DESKTOP)
}

/// Ask the user for a folder to browse. `Ok(None)` means they cancelled.
pub(crate) fn pick_folder(initial_dir: Option<&Path>) -> Result<Option<PathBuf>, String> {
    Picker::Folder.pick(initial_dir, &SYSTEM_DESKTOP)
}
