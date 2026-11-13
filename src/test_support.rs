//! Assertions shared by the unit tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

mod tests;

static NEXT_SCRATCH_DIR: AtomicUsize = AtomicUsize::new(0);

/// A directory of its own for one test, removed again when the test ends.
pub(crate) struct ScratchDir(PathBuf);

impl ScratchDir {
    pub(crate) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "md-viewer-scratch-{}-{}",
            std::process::id(),
            NEXT_SCRATCH_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        // Canonical, so paths built from it compare equal to the canonical
        // paths tabs and the watcher report.
        Self(path.canonicalize().unwrap())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    /// Create the directory `name` (and its parents) and return its path.
    pub(crate) fn dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// Write `content` to the file `name`, creating its parent directories,
    /// and return its path.
    pub(crate) fn file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Poll `condition` until it holds or `timeout` runs out; returns whether it
/// held. For effects that arrive from another thread or process.
pub(crate) fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Every shape a frame painted, in paint order, with grouped shapes taken
/// apart into their members.
pub(crate) fn painted_shapes(output: &egui::FullOutput) -> Vec<egui::Shape> {
    fn collect(shape: &egui::Shape, shapes: &mut Vec<egui::Shape>) {
        if let egui::Shape::Vec(members) = shape {
            for member in members {
                collect(member, shapes);
            }
        } else {
            shapes.push(shape.clone());
        }
    }

    let mut shapes = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut shapes);
    }
    shapes
}

/// Turn every log level on for the rest of the test process, so the
/// arguments of log calls are evaluated the way a `RUST_LOG=trace` run
/// evaluates them. No logger is installed; the records go nowhere.
pub(crate) fn evaluate_log_arguments() {
    log::set_max_level(log::LevelFilter::Trace);
}

/// Asserts two pixel or zoom values are equal to within float rounding.
///
/// The values compared in this crate are small exact constants and clamped
/// results, so the tolerance only absorbs representation error.
#[track_caller]
pub(crate) fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= f32::EPSILON * expected.abs().max(1.0),
        "expected {expected}, got {actual}"
    );
}
