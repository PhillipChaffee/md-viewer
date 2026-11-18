//! The whole app over a scratch directory, driven by the labels of its widgets.

use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use eframe::egui;
use egui_kittest::kittest::Queryable as _;

use crate::app::MarkdownApp;
use crate::test_support::{evaluate_log_arguments, ScratchDir};

/// A running app that tests click through by widget label, the way a user
/// reads the window. It dereferences to the `egui_kittest` harness, whose
/// `state` is the app.
pub(crate) struct LabelHarness {
    kit: egui_kittest::Harness<'static, MarkdownApp>,
    pub(crate) dir: PathBuf,
    /// Removes `dir` when the harness is dropped.
    _scratch: ScratchDir,
}

impl LabelHarness {
    /// Write each `(name, content)` document and start the app on the first
    /// one, the way `md-viewer <file>` does. A name may hold a subdirectory.
    /// File watching starts off.
    pub(crate) fn with_docs(docs: &[(&str, &str)]) -> Self {
        // Format log arguments as a run with `RUST_LOG` set does, so one that
        // panics fails a test.
        evaluate_log_arguments();
        let scratch = ScratchDir::new();
        let dir = scratch.path().to_path_buf();
        for (name, content) in docs {
            scratch.file(name, content);
        }
        let first = docs.first().map(|(name, _)| dir.join(name));

        let kit = egui_kittest::Harness::builder()
            .with_size(egui::vec2(1200.0, 800.0))
            .build_eframe(|cc| MarkdownApp::new(cc, first, false));
        let mut harness = Self {
            kit,
            dir,
            _scratch: scratch,
        };
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
        while self.kit.state().file_explorer.is_scanning() {
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
