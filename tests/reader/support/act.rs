//! What a person does to the window: point, click, type, press, drop.

use std::path::PathBuf;

use eframe::egui::{self, Modifiers, PointerButton, Pos2};

use super::keys::chord;
use super::Reader;

impl Reader {
    /// Press and release `button` at `at` with `modifiers` held, one frame
    /// per event, then let the app come to rest.
    #[track_caller]
    fn click_at(&mut self, at: Pos2, button: PointerButton, modifiers: Modifiers) {
        let at = self.to_points(at);
        self.frame(modifiers, vec![egui::Event::PointerMoved(at)]);
        for pressed in [true, false] {
            let event = egui::Event::PointerButton {
                pos: at,
                button,
                pressed,
                modifiers,
            };
            self.frame(modifiers, vec![event]);
        }
        self.settle();
    }

    /// Click a spot of the window, in pixels from its top left corner.
    #[track_caller]
    pub(crate) fn click_point(&mut self, x: f32, y: f32) {
        self.click_at(egui::pos2(x, y), PointerButton::Primary, Modifiers::NONE);
    }

    /// Click the one control or text labelled exactly `label`.
    #[track_caller]
    pub(crate) fn click(&mut self, label: &str) {
        let at = self.spot(label);
        self.click_at(at, PointerButton::Primary, Modifiers::NONE);
    }

    /// Click the `nth` (from zero, in reading order) of the controls
    /// labelled exactly `label`, for a label the window shows twice.
    #[track_caller]
    pub(crate) fn click_nth(&mut self, label: &str, nth: usize) {
        let at = self.nth_spot(label, nth);
        self.click_at(at, PointerButton::Primary, Modifiers::NONE);
    }

    /// The middle of the `nth` (from zero, in reading order) of the
    /// controls labelled exactly `label`.
    #[track_caller]
    fn nth_spot(&self, label: &str, nth: usize) -> Pos2 {
        let all = self.all(label);
        let Some(element) = all.get(nth) else {
            panic!(
                "there are {} `{label}`, no number {nth}\n{}",
                all.len(),
                self.describe()
            );
        };
        element.area.center()
    }

    /// Click `label` with Ctrl held.
    #[track_caller]
    pub(crate) fn ctrl_click(&mut self, label: &str) {
        let at = self.spot(label);
        self.click_at(at, PointerButton::Primary, chord("Ctrl+A").0);
    }

    /// Click the `nth` of the controls labelled `label` with the right
    /// mouse button.
    #[track_caller]
    pub(crate) fn right_click_nth(&mut self, label: &str, nth: usize) {
        let at = self.nth_spot(label, nth);
        self.click_at(at, PointerButton::Secondary, Modifiers::NONE);
    }

    /// Click the `nth` of the controls labelled `label` with the middle
    /// mouse button.
    #[track_caller]
    pub(crate) fn middle_click_nth(&mut self, label: &str, nth: usize) {
        let at = self.nth_spot(label, nth);
        self.click_at(at, PointerButton::Middle, Modifiers::NONE);
    }

    /// Click `label` with the right mouse button, which opens its menu.
    #[track_caller]
    pub(crate) fn right_click(&mut self, label: &str) {
        let at = self.spot(label);
        self.click_at(at, PointerButton::Secondary, Modifiers::NONE);
    }

    /// Click `label` with the middle mouse button.
    #[track_caller]
    pub(crate) fn middle_click(&mut self, label: &str) {
        let at = self.spot(label);
        self.click_at(at, PointerButton::Middle, Modifiers::NONE);
    }

    /// Click twice in quick succession on `label`.
    #[track_caller]
    pub(crate) fn double_click(&mut self, label: &str) {
        let at = self.to_points(self.spot(label));
        self.frame(Modifiers::NONE, vec![egui::Event::PointerMoved(at)]);
        // A press and its release share a frame, so the two clicks are one
        // frame apart: inside the time a double click allows.
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        for _ in 0..2 {
            self.frame(Modifiers::NONE, vec![button(true), button(false)]);
        }
        self.settle();
    }

    /// Rest the pointer on `label`.
    #[track_caller]
    pub(crate) fn hover(&mut self, label: &str) {
        let at = self.to_points(self.spot(label));
        self.frame(Modifiers::NONE, vec![egui::Event::PointerMoved(at)]);
        self.settle();
    }

    /// Press on `label`, move the pointer `dx` pixels right and `dy` down,
    /// and let go.
    #[track_caller]
    pub(crate) fn drag(&mut self, label: &str, dx: f32, dy: f32) {
        let from = self.spot(label);
        self.drag_point(from.x, from.y, dx, dy);
    }

    /// Press on the slider `label` at `fraction` of its length (0 is its
    /// left end, 1 its right end) and let go.
    #[track_caller]
    pub(crate) fn slide(&mut self, label: &str, fraction: f32) {
        let area = self.the(label).area;
        let at = egui::pos2(area.width.mul_add(fraction, area.left), area.center().y);
        self.click_at(at, PointerButton::Primary, Modifiers::NONE);
    }

    /// Press and release a shortcut written the way the README writes it:
    /// `Ctrl+D`, `Alt+Left`, `Ctrl+Shift+Tab`, `Ctrl++`, `F5`, `Escape`.
    #[track_caller]
    pub(crate) fn press(&mut self, shortcut: &str) {
        let (modifiers, key) = chord(shortcut);
        for pressed in [true, false] {
            let event = egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            };
            self.frame(modifiers, vec![event]);
        }
        self.settle();
    }

    /// Press and release a shortcut so fast that the whole of it, the
    /// modifiers going down and coming up included, lands in one frame, as
    /// a quick tap does while the app is busy drawing.
    #[track_caller]
    pub(crate) fn tap(&mut self, shortcut: &str) {
        let (modifiers, key) = chord(shortcut);
        let stroke = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        // By the end of the frame nothing is held any more.
        self.frame(Modifiers::NONE, vec![stroke(true), stroke(false)]);
        self.settle();
    }

    /// Type `text` into whatever has the keyboard.
    pub(crate) fn type_text(&mut self, text: &str) {
        self.frame(Modifiers::NONE, vec![egui::Event::Text(text.to_owned())]);
        self.settle();
    }

    /// Turn the mouse wheel over the middle of the window, moving the page
    /// `pixels` toward its end.
    pub(crate) fn scroll_down(&mut self, pixels: f32) {
        let middle = self.to_points(egui::pos2(self.window.0 / 2.0, self.window.1 / 2.0));
        let points = pixels / self.scale();
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -points),
            modifiers: Modifiers::NONE,
        };
        self.frame(Modifiers::NONE, vec![egui::Event::PointerMoved(middle)]);
        self.frame(Modifiers::NONE, vec![wheel]);
        self.settle();
    }

    /// Turn the mouse wheel the other way, moving the page `pixels` toward
    /// its start.
    pub(crate) fn scroll_up(&mut self, pixels: f32) {
        self.scroll_down(-pixels);
    }

    /// Paste `text` from the clipboard into whatever has the keyboard.
    pub(crate) fn paste(&mut self, text: &str) {
        self.frame(Modifiers::NONE, vec![egui::Event::Paste(text.to_owned())]);
        self.settle();
    }

    /// Hold files over the window without letting go.
    pub(crate) fn drag_files_over(&mut self, names: &[&str]) {
        let files = names.iter().map(|name| egui::HoveredFile {
            path: Some(self.path(name)),
            ..Default::default()
        });
        self.kit.input_mut().hovered_files = files.collect();
        self.frame(Modifiers::NONE, Vec::new());
    }

    /// Drop the file `name` from the folder onto the window.
    pub(crate) fn drop_file(&mut self, name: &str) {
        self.drop_path(self.path(name));
    }

    /// Drop several files from the folder onto the window in one go.
    pub(crate) fn drop_files(&mut self, names: &[&str]) {
        let paths = names.iter().map(|name| self.path(name)).collect();
        self.drop_paths(paths);
    }

    /// Drop a file from anywhere on disk onto the window.
    pub(crate) fn drop_path(&mut self, path: PathBuf) {
        self.drop_paths(vec![path]);
    }

    fn drop_paths(&mut self, paths: Vec<PathBuf>) {
        let dropped = paths.into_iter().map(|path| egui::DroppedFile {
            path: Some(path),
            ..Default::default()
        });
        let input = self.kit.input_mut();
        input.hovered_files.clear();
        input.dropped_files = dropped.collect();
        self.frame(Modifiers::NONE, Vec::new());
        self.settle();
    }

    /// Give the window a new size in pixels.
    pub(crate) fn resize_window(&mut self, width: f32, height: f32) {
        self.window = (width, height);
        self.settle();
    }
}
