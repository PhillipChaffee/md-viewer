//! A scratch folder of documents: the reader's files for one test.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FOLDER: AtomicUsize = AtomicUsize::new(0);

/// A folder of its own for one test, removed when the test ends.
pub(crate) struct Folder(PathBuf);

impl Folder {
    pub(crate) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "md-viewer-reader-test-{}-{}",
            std::process::id(),
            NEXT_FOLDER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        // Canonical, so a path built here equals the one the app reports.
        Self(path.canonicalize().unwrap())
    }

    pub(crate) fn root(&self) -> &Path {
        &self.0
    }

    /// Where `name` lives in this folder, whether or not it exists.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Write `content` to `name`, creating the folders above it.
    pub(crate) fn write(&self, name: &str, content: impl AsRef<[u8]>) {
        let path = self.path(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Make the folder `name` and the folders above it.
    pub(crate) fn make_folder(&self, name: &str) {
        fs::create_dir_all(self.path(name)).unwrap();
    }

    /// Date `name` as last changed `age` ago.
    #[track_caller]
    pub(crate) fn age(&self, name: &str, age: std::time::Duration) {
        let file = fs::File::options().write(true).open(self.path(name));
        let then = std::time::SystemTime::now() - age;
        file.unwrap().set_modified(then).unwrap();
    }

    /// Make `name` a symbolic link to `target`.
    #[cfg(unix)]
    pub(crate) fn link(&self, name: &str, target: &str) {
        std::os::unix::fs::symlink(self.path(target), self.path(name)).unwrap();
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
