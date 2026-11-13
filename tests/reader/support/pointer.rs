//! Finer work with the mouse: gestures on a spot of the window, for what
//! has no name to click, such as a picture or the empty page.

use eframe::egui::{self, Modifiers, PointerButton, Pos2};

use super::Reader;

/// The primary button going down or coming up at `at`.
const fn button(at: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

impl Reader {
    /// Rest the pointer on a spot of the window, in pixels from its top
    /// left corner.
    pub(crate) fn hover_point(&mut self, x: f32, y: f32) {
        let at = self.to_points(egui::pos2(x, y));
        self.frame(Modifiers::NONE, vec![egui::Event::PointerMoved(at)]);
        self.settle();
    }

    /// Click twice in quick succession on a spot of the window.
    pub(crate) fn double_click_point(&mut self, x: f32, y: f32) {
        let at = self.to_points(egui::pos2(x, y));
        self.frame(Modifiers::NONE, vec![egui::Event::PointerMoved(at)]);
        for _ in 0..2 {
            self.frame(Modifiers::NONE, vec![button(at, true), button(at, false)]);
        }
        self.settle();
    }

    /// Press on a spot of the window, move the pointer `dx` pixels right
    /// and `dy` down, and keep the button down.
    pub(crate) fn press_and_drag(&mut self, x: f32, y: f32, dx: f32, dy: f32) {
        let from = self.to_points(egui::pos2(x, y));
        let by = egui::vec2(dx, dy) / self.scale();
        self.frame(Modifiers::NONE, vec![egui::Event::PointerMoved(from)]);
        self.frame(Modifiers::NONE, vec![button(from, true)]);
        // Two steps: the first tells a drag from a click, the second lands.
        for part in [0.5, 1.0] {
            let at = from + by * part;
            self.frame(Modifiers::NONE, vec![egui::Event::PointerMoved(at)]);
        }
        self.held = Some(from + by);
        self.settle();
    }

    /// Let go of the button [`Self::press_and_drag`] left down.
    #[track_caller]
    pub(crate) fn let_go(&mut self) {
        let at = self.held.take().expect("no button is held down");
        self.frame(Modifiers::NONE, vec![button(at, false)]);
        self.settle();
    }

    /// Press on a spot of the window, move the pointer `dx` pixels right
    /// and `dy` down, and let go.
    pub(crate) fn drag_point(&mut self, x: f32, y: f32, dx: f32, dy: f32) {
        self.press_and_drag(x, y, dx, dy);
        self.let_go();
    }

    /// Turn the mouse wheel by `turn` pixels (up and to the left are
    /// positive) with the pointer on a spot of the window and `modifiers`
    /// held.
    fn turn_wheel(&mut self, x: f32, y: f32, turn: egui::Vec2, modifiers: Modifiers) {
        let at = self.to_points(egui::pos2(x, y));
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: turn / self.scale(),
            modifiers,
        };
        self.frame(modifiers, vec![egui::Event::PointerMoved(at)]);
        self.frame(modifiers, vec![wheel]);
        self.settle();
    }

    /// Turn the mouse wheel away from the reader with the pointer on a
    /// spot of the window, which moves a page `pixels` toward its start.
    pub(crate) fn wheel_up_at(&mut self, x: f32, y: f32, pixels: f32) {
        self.turn_wheel(x, y, egui::vec2(0.0, pixels), Modifiers::NONE);
    }

    /// Turn the mouse wheel toward the reader with the pointer on a spot
    /// of the window, which moves a page `pixels` toward its end.
    pub(crate) fn wheel_down_at(&mut self, x: f32, y: f32, pixels: f32) {
        self.turn_wheel(x, y, egui::vec2(0.0, -pixels), Modifiers::NONE);
    }

    /// Tilt the mouse wheel, or sweep two fingers, with the pointer on a
    /// spot of the window, which moves what is under it `pixels` toward
    /// its right end.
    pub(crate) fn sideways_wheel_at(&mut self, x: f32, y: f32, pixels: f32) {
        self.turn_wheel(x, y, egui::vec2(-pixels, 0.0), Modifiers::NONE);
    }
}
