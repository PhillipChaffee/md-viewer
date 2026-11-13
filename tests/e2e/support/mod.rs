//! The driver for the real binary: a scratch home to run it in, and, for
//! the tests that need a window, a private X display to open it on.
//!
//! Nothing here links the app. A test sees what a person at a terminal and
//! a screen sees: exit codes, output, window titles, pixels, files.

#[cfg(target_os = "linux")]
mod screen;

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
pub(crate) use self::screen::{in_a_terminal, kill, Screen};

/// How long anything the binary does may take before a test gives up. A
/// cold start on a loaded CI machine paints its first frame in seconds.
pub(crate) const PATIENCE: Duration = Duration::from_secs(60);

static NEXT_HOME: AtomicUsize = AtomicUsize::new(0);

/// Where md-viewer keeps its settings under the user's data folder, and
/// the name its session goes by in that file.
const SETTINGS_FOLDER: &str = "md-viewer";
const SETTINGS_FILE: &str = "app.ron";
const SESSION_KEY: &str = "md-viewer-state";

/// Poll `seen` until it gives a value or `PATIENCE` runs out.
pub(crate) fn wait_for<T>(mut seen: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + PATIENCE;
    loop {
        if let Some(value) = seen() {
            return Some(value);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A user's home for one test: a folder of documents to run `md-viewer`
/// in, and the place the app keeps its settings. Removed when dropped.
pub(crate) struct Home(PathBuf);

impl Home {
    pub(crate) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "md-viewer-e2e-{}-{}",
            std::process::id(),
            NEXT_HOME.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("docs")).unwrap();
        Self(path.canonicalize().unwrap())
    }

    /// The top of this home, for files a test wants to keep out of the
    /// documents folder.
    pub(crate) fn root(&self) -> &Path {
        &self.0
    }

    /// Write a document into the folder `md-viewer` is run in.
    pub(crate) fn doc(&self, name: &str, content: &str) {
        let path = self.0.join("docs").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Where the document `name` is on disk.
    pub(crate) fn doc_path(&self, name: &str) -> PathBuf {
        self.0.join("docs").join(name)
    }

    /// Put `text` where the app keeps its settings between runs, as the
    /// whole settings file an earlier run left behind.
    pub(crate) fn left_settings_file(&self, text: &str) {
        let folder = self.0.join("data").join(SETTINGS_FOLDER);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(SETTINGS_FILE), text).unwrap();
    }

    /// Leave `session` as the session an earlier run saved, in the
    /// settings file the app reads at its next start.
    pub(crate) fn left_session(&self, session: &str) {
        self.left_settings_file(&format!("{{{SESSION_KEY:?}: {session:?}}}"));
    }

    /// Delete a document from the folder `md-viewer` is run in.
    #[track_caller]
    pub(crate) fn delete(&self, name: &str) {
        fs::remove_file(self.0.join("docs").join(name)).unwrap();
    }

    /// Every file the app has written to its settings folders, with its
    /// text.
    pub(crate) fn saved_settings(&self) -> Vec<(PathBuf, String)> {
        fn walk(dir: &Path, found: &mut Vec<(PathBuf, String)>) {
            for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, found);
                } else if let Ok(text) = fs::read_to_string(&path) {
                    found.push((path, text));
                }
            }
        }
        let mut found = Vec::new();
        walk(&self.0.join("data"), &mut found);
        walk(&self.0.join("config"), &mut found);
        found
    }

    /// `md-viewer <args>`, typed in the documents folder, with this home's
    /// settings and no display.
    pub(crate) fn md_viewer(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_md-viewer"));
        command.args(args);
        self.settle_in(&mut command);
        command
    }

    /// Point `command` at this home: its working directory, its settings
    /// folders, and no display of the machine's own.
    pub(crate) fn settle_in(&self, command: &mut Command) {
        command
            .current_dir(self.0.join("docs"))
            .env("HOME", &self.0)
            .env("XDG_DATA_HOME", self.0.join("data"))
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .env_remove("WAYLAND_SOCKET")
            .stdin(Stdio::null());
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A process the test started, killed when dropped.
pub(crate) struct Running(Child);

impl Running {
    #[track_caller]
    pub(crate) fn start(command: &mut Command) -> Self {
        Self(command.spawn().unwrap())
    }

    /// Whether the process is still running.
    pub(crate) fn is_alive(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }

    /// Wait for the process to end by itself and return its exit code, or
    /// `None` if it outlives `PATIENCE` or is killed by a signal.
    pub(crate) fn exit_code(&mut self) -> Option<i32> {
        wait_for(|| self.0.try_wait().ok().flatten())?.code()
    }

    /// Wait for the process to end by itself and return what it printed.
    /// Fails if it outlives `PATIENCE`. The output has to fit a pipe, as a
    /// usage message does.
    #[track_caller]
    pub(crate) fn finish(mut self) -> Finished {
        let status = wait_for(|| self.0.try_wait().ok().flatten());
        let status = status.expect("md-viewer did not exit");
        let (mut stdout, mut stderr) = (String::new(), String::new());
        if let Some(stream) = &mut self.0.stdout {
            stream.read_to_string(&mut stdout).unwrap();
        }
        if let Some(stream) = &mut self.0.stderr {
            stream.read_to_string(&mut stderr).unwrap();
        }
        Finished {
            code: status.code(),
            stdout,
            stderr,
        }
    }
}

/// How a run of `md-viewer` ended.
#[derive(Debug)]
pub(crate) struct Finished {
    pub(crate) code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Run `md-viewer <args>` to its end with its output captured. Fails if it
/// does not end.
#[track_caller]
pub(crate) fn run_to_end(home: &Home, args: &[&str]) -> Finished {
    let mut command = home.md_viewer(args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    Running::start(&mut command).finish()
}
