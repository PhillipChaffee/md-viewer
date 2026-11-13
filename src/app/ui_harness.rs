//! Test support for the app shell: the real app behind `egui_kittest`, where
//! tests find widgets by what they say and click them, plus the scratch
//! files and in-memory settings store a hermetic launch needs.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::{self, accesskit};
use eframe::App as _;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::settings::FontPicker;
use super::tests::Harness;
use super::MarkdownApp;

/// The frames an interaction gets to come to rest before the harness starts
/// waiting on the clock for background work.
const QUICK_FRAMES: usize = 12;

/// The window size every harness frame is laid out in.
const SCREEN: egui::Vec2 = egui::vec2(1200.0, 800.0);

/// How long to wait for background work: `WAIT_FRAMES` frames, `WAIT_STEP`
/// apart, is ten seconds.
const WAIT_FRAMES: usize = 1000;
const WAIT_STEP: Duration = Duration::from_millis(10);

/// A directory of one test's files, removed when dropped: the crate's
/// scratch directory under the names the app-shell tests call it by.
pub(crate) struct ScratchDir(crate::test_support::ScratchDir);

impl ScratchDir {
    pub(crate) fn new() -> Self {
        Self(crate::test_support::ScratchDir::new())
    }

    /// Where `name` lives in this directory, whether or not it exists.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.0.path().join(name)
    }

    /// Write a file and return its path.
    pub(crate) fn write(&self, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
        self.0.file(name, content)
    }

    /// Create a subdirectory and return its path.
    pub(crate) fn subdir(&self, name: &str) -> PathBuf {
        self.0.dir(name)
    }

    pub(crate) fn root(&self) -> PathBuf {
        self.0.path().to_path_buf()
    }
}

/// The settings store eframe hands the app, kept in memory instead of in the
/// user's data directory.
#[derive(Default)]
pub(crate) struct MemoryStorage(HashMap<String, String>);

impl eframe::Storage for MemoryStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }

    fn flush(&mut self) {}
}

/// Start the app the way eframe does when `storage` holds the last
/// session's settings.
pub(crate) fn launch(storage: &MemoryStorage, file: Option<PathBuf>, watch: bool) -> MarkdownApp {
    let mut creation = eframe::CreationContext::_new_kittest(egui::Context::default());
    creation.storage = Some(storage);
    MarkdownApp::new(&creation, file, watch)
}

/// Run one frame of the key-driven harness on `input`, through the same
/// input hook and update call eframe makes.
pub(crate) fn frame_with(harness: &mut Harness, mut input: egui::RawInput) -> egui::FullOutput {
    input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN));
    harness.app.raw_input_hook(&harness.ctx, &mut input);
    let (app, frame) = (&mut harness.app, &mut harness.frame);
    harness.ctx.run(input, |ctx| app.update(ctx, frame))
}

/// Run one frame of the key-driven harness on `events`.
pub(crate) fn frame_on(harness: &mut Harness, events: Vec<egui::Event>) -> egui::FullOutput {
    let input = egui::RawInput {
        events,
        ..Default::default()
    };
    frame_with(harness, input)
}

/// Move the pointer of the key-driven harness to `pos` and click there, one
/// frame per event. Returns the output of the frame the button came up in.
pub(crate) fn click_on(harness: &mut Harness, pos: egui::Pos2) -> egui::FullOutput {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame_on(harness, vec![egui::Event::PointerMoved(pos)]);
    frame_on(harness, vec![button(true)]);
    frame_on(harness, vec![button(false)])
}

/// Run frames of the key-driven harness until `done`, for work that
/// finishes on another thread.
#[track_caller]
pub(crate) fn frames_until(harness: &mut Harness, what: &str, done: impl Fn(&MarkdownApp) -> bool) {
    let finished = (0..WAIT_FRAMES).any(|_| {
        std::thread::sleep(WAIT_STEP);
        harness.idle();
        done(&harness.app)
    });
    assert!(finished, "timed out waiting for {what}");
}

/// Every piece of text a frame painted, in paint order.
pub(crate) fn painted_text(output: &egui::FullOutput) -> Vec<String> {
    let shapes = output.shapes.iter();
    shapes
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

/// A running app over a scratch directory of documents, driven through its
/// accessibility tree.
pub(crate) struct UiHarness {
    pub(crate) ui: egui_kittest::Harness<'static, MarkdownApp>,
    pub(crate) dir: ScratchDir,
    /// Every viewport command the app sent since the harness started.
    pub(crate) sent: Vec<egui::ViewportCommand>,
}

impl UiHarness {
    /// Write each `(name, content)` document and start the app on the first
    /// one. Without documents the app starts on the welcome page.
    pub(crate) fn with_docs(docs: &[(&str, &str)]) -> Self {
        let dir = ScratchDir::new();
        for (name, content) in docs {
            dir.write(name, content);
        }
        Self::open(dir, docs.first().map(|(name, _)| *name))
    }

    /// Start the app over an already filled directory, on the file `name`
    /// when there is one.
    ///
    /// The scan of installed fonts is cut loose, so no test depends on which
    /// fonts the machine has or on when the scan lands. A test of the family
    /// list feeds the font picker's `scan` itself.
    pub(crate) fn open(dir: ScratchDir, name: Option<&str>) -> Self {
        let file = name.map(|name| dir.path(name));
        let mut ui = egui_kittest::Harness::builder()
            .with_size(SCREEN)
            .build_eframe(|creation| MarkdownApp::new(creation, file, false));
        let app = ui.state_mut();
        app.font_picker = FontPicker::default();
        Self {
            ui,
            dir,
            sent: Vec::new(),
        }
    }

    /// Three one-line documents: `a.md`, `b.md`, and `c.md`.
    pub(crate) fn with_three_docs() -> Self {
        Self::with_docs(&[("a.md", "# A"), ("b.md", "# B"), ("c.md", "# C")])
    }

    pub(crate) fn app(&self) -> &MarkdownApp {
        self.ui.state()
    }

    pub(crate) fn app_mut(&mut self) -> &mut MarkdownApp {
        self.ui.state_mut()
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.dir.path(name)
    }

    /// The document the active tab shows.
    pub(crate) fn active_path(&self) -> PathBuf {
        let app = self.app();
        app.tabs[app.active_tab].path.clone()
    }

    /// Run the queued input, one frame per event, and note what the last
    /// of those frames asked of the window.
    pub(crate) fn step(&mut self) {
        self.ui.step();
        let viewports = self.ui.output().viewport_output.values();
        self.sent
            .extend(viewports.flat_map(|viewport| viewport.commands.iter().cloned()));
    }

    /// Run `count` frames without waiting for the app to come to rest, for
    /// screens that animate.
    pub(crate) fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.step();
        }
    }

    /// Run frames until the app stops asking for an immediate repaint.
    ///
    /// Under the harness clock a frame is longer than the 100 ms the app
    /// waits between polls of a folder scan, so a poll counts as immediate
    /// and the app is at rest only once the scan has landed. That takes
    /// time rather than frames: past the first few frames, each one waits.
    pub(crate) fn settle(&mut self) {
        let settled = (0..WAIT_FRAMES).any(|frame| {
            std::thread::sleep(WAIT_STEP * u32::from(frame >= QUICK_FRAMES));
            self.step();
            let root = &self.ui.output().viewport_output[&egui::ViewportId::ROOT];
            !root.repaint_delay.is_zero()
        });
        let causes = self.ui.ctx.repaint_causes();
        assert!(settled, "the app still asks for repaints: {causes:?}");
    }

    /// Run frames until `done`, for work that finishes on another thread,
    /// then settle.
    #[track_caller]
    pub(crate) fn wait_until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let finished = (0..WAIT_FRAMES).any(|_| {
            std::thread::sleep(WAIT_STEP);
            self.step();
            done(self)
        });
        assert!(finished, "timed out waiting for {what}");
        self.settle();
    }

    /// The accessibility tree reports physical pixels; pointer events take
    /// points. The two differ once the document zoom leaves 100%.
    fn to_points(&self, physical: egui::Pos2) -> egui::Pos2 {
        (physical.to_vec2() / self.ui.ctx.pixels_per_point()).to_pos2()
    }

    /// The center, in points, of the one widget whose label contains
    /// `label`.
    pub(crate) fn center_of(&self, label: &str) -> egui::Pos2 {
        self.to_points(self.ui.get_by_label_contains(label).rect().center())
    }

    /// Click the one widget whose label contains `label`, then settle.
    pub(crate) fn click(&mut self, label: &str) {
        self.click_at(self.center_of(label));
    }

    /// Click the one widget labelled exactly `label`, then settle.
    pub(crate) fn click_exact(&mut self, label: &str) {
        let rect = self.ui.get_by_label(label).rect();
        self.click_at(self.to_points(rect.center()));
    }

    /// Open the overflow menu behind the top bar's last button.
    pub(crate) fn open_more(&mut self) {
        self.click_exact("More");
    }

    /// Open the overflow menu and click the row whose label contains
    /// `item`.
    pub(crate) fn pick(&mut self, item: &str) {
        self.open_more();
        self.click(item);
    }

    /// Whether the one toggle labelled exactly `label` is on.
    pub(crate) fn is_on(&self, label: &str) -> bool {
        let node = self.ui.get_by_label(label);
        node.accesskit_node().toggled() == Some(accesskit::Toggled::True)
    }

    /// Move the pointer to `pos` and let what it points at react.
    pub(crate) fn hover_at(&mut self, pos: egui::Pos2) {
        self.ui.hover_at(pos);
        self.settle();
    }

    /// Press and release `key` with `modifiers` held, then settle.
    pub(crate) fn press(&mut self, modifiers: egui::Modifiers, key: egui::Key) {
        self.ui.key_press_modifiers(modifiers, key);
        self.settle();
    }

    /// Whether a widget whose label contains `label` is on screen.
    pub(crate) fn shows(&self, label: &str) -> bool {
        self.ui.query_all_by_label_contains(label).next().is_some()
    }

    /// Every widget labelled exactly `label`, in tree order: its center in
    /// points, and whether it can be clicked.
    pub(crate) fn labelled(&self, label: &str) -> Vec<(egui::Pos2, bool)> {
        self.ui
            .query_all_by_label(label)
            .map(|node| {
                let enabled = !node.accesskit_node().is_disabled();
                (self.to_points(node.rect().center()), enabled)
            })
            .collect()
    }

    /// Whether each widget labelled exactly `label` can be clicked, in tree
    /// order.
    pub(crate) fn enabled(&self, label: &str) -> Vec<bool> {
        let labelled = self.labelled(label);
        labelled.into_iter().map(|(_, enabled)| enabled).collect()
    }

    /// The centers, in points, of every widget with `role`, in tree order.
    pub(crate) fn centers_with_role(&self, role: accesskit::Role) -> Vec<egui::Pos2> {
        self.ui
            .query_all_by_role(role)
            .map(|node| self.to_points(node.rect().center()))
            .collect()
    }

    /// Press and release `button` at `pos` with `modifiers` held, then
    /// settle.
    pub(crate) fn click_with(
        &mut self,
        pos: egui::Pos2,
        button: egui::PointerButton,
        modifiers: egui::Modifiers,
    ) {
        self.ui.hover_at(pos);
        for pressed in [true, false] {
            let event = egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            };
            // A modified event is followed by a frame that lets the keys go,
            // so only an unmodified click leaves its own frame's output last.
            if modifiers.is_none() {
                self.ui.event(event);
            } else {
                self.ui.event_modifiers(event, modifiers);
            }
        }
        self.settle();
    }

    /// Press and release the primary button at `pos`, then settle.
    pub(crate) fn click_at(&mut self, pos: egui::Pos2) {
        self.click_with(pos, egui::PointerButton::Primary, egui::Modifiers::NONE);
    }

    /// Whether the app has asked the window to close.
    pub(crate) fn close_requested(&self) -> bool {
        self.sent
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::Close))
    }
}
