//! The whole app over a scratch directory, driven by the labels of its widgets.

use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use eframe::egui;
use egui_kittest::kittest::Queryable as _;

use crate::app::MarkdownApp;

static NEXT_SCRATCH_DIR: AtomicUsize = AtomicUsize::new(0);

/// A running app that tests click through by widget label, the way a user
/// reads the window. It dereferences to the `egui_kittest` harness, whose
/// `state` is the app.
pub(crate) struct LabelHarness {
    kit: egui_kittest::Harness<'static, MarkdownApp>,
    pub(crate) dir: PathBuf,
}

impl LabelHarness {
    /// Write each `(name, content)` document and start the app on the first
    /// one, the way `md-viewer <file>` does. A name may hold a subdirectory.
    /// File watching starts off.
    pub(crate) fn with_docs(docs: &[(&str, &str)]) -> Self {
        // Format log arguments as a run with `RUST_LOG` set does, so one that
        // panics fails a test.
        log::set_max_level(log::LevelFilter::Trace);
        let dir = std::env::temp_dir().join(format!(
            "md-viewer-label-test-{}-{}",
            std::process::id(),
            NEXT_SCRATCH_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let dir = dir.canonicalize().unwrap();
        for (name, content) in docs {
            let path = dir.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        let first = docs.first().map(|(name, _)| dir.join(name));

        let kit = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1200.0, 800.0))
            .build_eframe(|cc| MarkdownApp::new(cc, first, false));
        let mut harness = Self { kit, dir };
        harness.wait_for_explorer_scan();
        harness
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Click the one widget labelled `label`, then let the click take effect.
    pub(crate) fn click(&mut self, label: &str) {
        self.kit.get_by_label(label).click();
        self.frames(2);
    }

    /// Run `count` rounds of frames. The first round feeds queued input to
    /// the app, one frame per event.
    pub(crate) fn frames(&mut self, count: usize) {
        self.kit.run_steps(count);
    }

    /// The explorer lists its root on a background thread; run frames until
    /// the listing has landed.
    pub(crate) fn wait_for_explorer_scan(&mut self) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while self.kit.state().file_explorer.pending_scan.is_some() {
            assert!(
                std::time::Instant::now() < deadline,
                "the explorer scan of {} never finished",
                self.dir.display()
            );
            std::thread::yield_now();
            self.frames(1);
        }
        self.frames(1);
    }
}

impl Deref for LabelHarness {
    type Target = egui_kittest::Harness<'static, MarkdownApp>;

    fn deref(&self) -> &Self::Target {
        &self.kit
    }
}

impl DerefMut for LabelHarness {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.kit
    }
}

impl Drop for LabelHarness {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
