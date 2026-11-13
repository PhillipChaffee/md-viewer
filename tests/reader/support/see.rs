//! What a person, or a screen reader, can tell from the window.

use std::fmt::Write as _;

use eframe::egui::accesskit::{Role, Toggled};
use eframe::egui::{self, Pos2};
use egui_kittest::kittest::{By, NodeT as _, Queryable as _};

use super::paint::{color_at, painted_text, pictures, Painted, Rgb};
use super::Reader;

/// Where something is in the window, in pixels from its top left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Area {
    pub(crate) left: f32,
    pub(crate) top: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl Area {
    pub(crate) fn right(self) -> f32 {
        self.left + self.width
    }

    pub(crate) fn bottom(self) -> f32 {
        self.top + self.height
    }

    pub(super) fn center(self) -> Pos2 {
        egui::pos2(
            self.width.mul_add(0.5, self.left),
            self.height.mul_add(0.5, self.top),
        )
    }
}

/// One thing the window exposes to assistive tools.
#[derive(Clone, Debug)]
pub(super) struct Element {
    /// What it is called: a control's label, or a text's words.
    pub(super) name: String,
    /// What it holds, for a text field.
    value: Option<String>,
    number: Option<f64>,
    role: Role,
    pub(super) area: Area,
    /// Chosen, for one of a group; switched on, for a toggle.
    on: bool,
    enabled: bool,
    focused: bool,
}

impl Element {
    /// Whether this is something to operate, not something to read.
    fn is_control(&self) -> bool {
        !matches!(self.role, Role::Label | Role::TextRun)
    }
}

impl Reader {
    /// Everything in the accessibility tree that has a name or a value, in
    /// reading order.
    fn elements(&self) -> Vec<Element> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "window coordinates are a few thousand at most"
        )]
        let pixels = |pixels: f64| pixels as f32;
        self.kit
            .query_all(By::new())
            .filter_map(|node| {
                let node = node.accesskit_node();
                // The window itself has no box; nothing in it lacks one.
                let rect = node.bounding_box()?;
                let role = node.role();
                let (name, value) = if role == Role::Label {
                    (node.value(), None)
                } else {
                    (node.label(), node.value())
                };
                if name.is_none() && value.is_none() {
                    return None;
                }
                Some(Element {
                    name: name.unwrap_or_default(),
                    value,
                    number: node.numeric_value(),
                    role,
                    area: Area {
                        left: pixels(rect.x0),
                        top: pixels(rect.y0),
                        width: pixels(rect.width()),
                        height: pixels(rect.height()),
                    },
                    on: node.is_selected() == Some(true) || node.toggled() == Some(Toggled::True),
                    enabled: !node.is_disabled(),
                    focused: node.is_focused(),
                })
            })
            .collect()
    }

    /// Every element named exactly `label`, in reading order.
    pub(super) fn all(&self, label: &str) -> Vec<Element> {
        let mut found = self.elements();
        found.retain(|element| element.name == label);
        found
    }

    /// The one element named exactly `label`. Where a button and a text
    /// share the name, it is the button.
    #[track_caller]
    pub(super) fn the(&self, label: &str) -> Element {
        let mut found = self.all(label);
        if found.len() > 1 && found.iter().any(Element::is_control) {
            found.retain(Element::is_control);
        }
        assert!(
            found.len() == 1,
            "expected one `{label}`, found {}\n{}",
            found.len(),
            self.describe()
        );
        found.remove(0)
    }

    /// The one text named exactly `label`: words to read, not a control
    /// that carries the same words.
    #[track_caller]
    fn the_text(&self, label: &str) -> Element {
        let mut found = self.all(label);
        found.retain(|element| !element.is_control());
        assert!(
            found.len() == 1,
            "expected one text `{label}`, found {}\n{}",
            found.len(),
            self.describe()
        );
        found.remove(0)
    }

    /// What the last frame painted, without the clipping.
    pub(super) fn shapes(&self) -> Vec<egui::Shape> {
        let shapes = self.kit.output().shapes.iter();
        shapes.map(|clipped| clipped.shape.clone()).collect()
    }

    /// Every text the last frame painted.
    pub(super) fn painted(&self) -> Vec<Painted> {
        painted_text(&self.shapes())
    }

    /// Where a person would click on the one thing named `label`: on the
    /// first line of its words when they are painted, else in its middle.
    #[track_caller]
    pub(super) fn spot(&self, label: &str) -> Pos2 {
        let area = self.the(label).area;
        let within = |at: Pos2| {
            (area.left..=area.right()).contains(&at.x) && (area.top..=area.bottom()).contains(&at.y)
        };
        let scale = self.scale();
        let painted = self.painted().into_iter();
        let mut lines = painted
            .filter(|painted| painted.text == label)
            .map(|painted| (painted.first_line.to_vec2() * scale).to_pos2());
        let spot = lines
            .find(|line| within(*line))
            .unwrap_or_else(|| area.center());
        assert!(
            (0.0..=self.window.0).contains(&spot.x) && (0.0..=self.window.1).contains(&spot.y),
            "`{label}` is outside the window; scroll to it first\n{}",
            self.describe()
        );
        spot
    }

    /// Where the last frame painted pictures: images, formulas and
    /// diagrams. Top to bottom.
    pub(crate) fn pictures(&self) -> Vec<Area> {
        let scale = self.scale();
        let mut found: Vec<Area> = pictures(&self.shapes())
            .into_iter()
            .map(|picture| picture.rect * scale)
            .map(|rect| Area {
                left: rect.left(),
                top: rect.top(),
                width: rect.width(),
                height: rect.height(),
            })
            .collect();
        found.sort_by(|a, b| a.top.total_cmp(&b.top));
        found
    }

    /// Whether `text` is in the window: in the name or the content of
    /// something a screen reader would read, or in text that is painted.
    pub(crate) fn sees(&self, text: &str) -> bool {
        let read = self.elements().into_iter().any(|element| {
            element.name.contains(text) || element.value.is_some_and(|value| value.contains(text))
        });
        read || self
            .painted()
            .iter()
            .any(|painted| painted.text.contains(text))
    }

    /// Fail unless `text` is in the window.
    #[track_caller]
    pub(crate) fn assert_sees(&self, text: &str) {
        assert!(
            self.sees(text),
            "`{text}` is not in the window\n{}",
            self.describe()
        );
    }

    /// Fail if `text` is in the window.
    #[track_caller]
    pub(crate) fn assert_hides(&self, text: &str) {
        assert!(
            !self.sees(text),
            "`{text}` is in the window\n{}",
            self.describe()
        );
    }

    /// How many controls or texts are named exactly `label`.
    pub(crate) fn count(&self, label: &str) -> usize {
        self.all(label).len()
    }

    /// The names of everything a screen reader would read, in reading
    /// order.
    pub(crate) fn names(&self) -> Vec<String> {
        let elements = self.elements().into_iter();
        elements
            .map(|element| element.name)
            .filter(|name| !name.is_empty())
            .collect()
    }

    /// Whether `label` is on: a toggle that is switched on, or the chosen
    /// one of a group, such as the active tab or the theme in use.
    #[track_caller]
    pub(crate) fn is_on(&self, label: &str) -> bool {
        self.the(label).on
    }

    /// Whether `label` can be used, or is grayed out.
    #[track_caller]
    pub(crate) fn is_enabled(&self, label: &str) -> bool {
        self.the(label).enabled
    }

    /// The number the slider `label` stands at.
    #[track_caller]
    pub(crate) fn number_of(&self, label: &str) -> f64 {
        let number = self.the(label).number;
        number.unwrap_or_else(|| panic!("`{label}` holds no number"))
    }

    /// Where the control `label` is in the window.
    #[track_caller]
    pub(crate) fn area_of(&self, label: &str) -> Area {
        self.the(label).area
    }

    /// Where the text `label` is in the window: a heading, a paragraph, a
    /// table cell.
    #[track_caller]
    pub(crate) fn text_area(&self, label: &str) -> Area {
        self.the_text(label).area
    }

    /// Whether the text `label` is laid out and lies wholly inside the
    /// window. A long document lays out only what is near the part shown.
    pub(crate) fn in_view(&self, label: &str) -> bool {
        let mut texts = self.all(label);
        texts.retain(|element| !element.is_control());
        texts.iter().any(|text| {
            let area = text.area;
            area.left >= 0.0
                && area.top >= 0.0
                && area.right() <= self.window.0
                && area.bottom() <= self.window.1
        })
    }

    /// What the text fields hold, in reading order: the find bar's query,
    /// the source in edit mode.
    pub(crate) fn typed(&self) -> Vec<String> {
        let fields = self
            .elements()
            .into_iter()
            .filter(|element| matches!(element.role, Role::TextInput | Role::MultilineTextInput));
        fields
            .map(|field| field.value.unwrap_or_default())
            .collect()
    }

    /// Click into the `nth` text field (from zero, in reading order), so
    /// it has the keyboard.
    #[track_caller]
    pub(crate) fn click_text_field(&mut self, nth: usize) {
        let fields: Vec<_> = self
            .elements()
            .into_iter()
            .filter(|element| matches!(element.role, Role::TextInput | Role::MultilineTextInput))
            .collect();
        let Some(field) = fields.get(nth) else {
            panic!("there is no text field number {nth}\n{}", self.describe());
        };
        let at = field.area.center();
        self.click_point(at.x, at.y);
    }

    /// The title the app gave its window.
    #[track_caller]
    pub(crate) fn title(&self) -> &str {
        let title = self.title.as_deref();
        title.unwrap_or_else(|| panic!("the app never titled its window"))
    }

    /// Whether the app has asked its window to close.
    pub(crate) const fn has_quit(&self) -> bool {
        self.quit_asked
    }

    /// The flat color at a point of the window, under any text there.
    pub(crate) fn color_at(&self, x: f32, y: f32) -> Rgb {
        let ctx = &self.kit.ctx;
        let frame = ctx.tessellate(self.kit.output().shapes.clone(), ctx.pixels_per_point());
        color_at(&frame, self.to_points(egui::pos2(x, y)))
    }

    /// The color of the page the document is set on, taken from the top
    /// margin of the text column, or from the middle of the window when no
    /// document is open.
    pub(crate) fn page_color(&self) -> Rgb {
        let edges = self.column_edges();
        let (x, y) = edges.map_or(
            (self.window.0 / 2.0, self.window.1 - 8.0),
            |(left, right)| (left.right().midpoint(right.left), left.top + 8.0),
        );
        self.color_at(x, y)
    }

    /// The ink of the first painted text that contains `text`.
    #[track_caller]
    pub(crate) fn ink_of(&self, text: &str) -> Rgb {
        let painted = self.painted();
        let found = painted.iter().find(|painted| painted.text.contains(text));
        let Some(found) = found else {
            panic!("no painted text contains `{text}`\n{}", self.describe());
        };
        found.ink
    }

    /// The two handles at the edges of the text column, left then right.
    fn column_edges(&self) -> Option<(Area, Area)> {
        let mut edges = self.elements().into_iter().filter_map(|element| {
            element
                .name
                .starts_with("Resize text column")
                .then_some(element.area)
        });
        let (first, second) = (edges.next()?, edges.next()?);
        Some(if first.left <= second.left {
            (first, second)
        } else {
            (second, first)
        })
    }

    /// The text column: the room between its two resize handles.
    #[track_caller]
    pub(crate) fn column(&self) -> Area {
        let Some((left, right)) = self.column_edges() else {
            panic!("no document is open\n{}", self.describe());
        };
        Area {
            left: left.right(),
            top: left.top,
            width: right.left - left.right(),
            height: left.height,
        }
    }

    /// Everything the window exposes, for a failure message.
    pub(crate) fn describe(&self) -> String {
        let mut out = format!("window titled {:?} shows:\n", self.title);
        // A text run repeats the label it belongs to.
        let elements = self.elements().into_iter();
        for element in elements.filter(|element| element.role != Role::TextRun) {
            let Element {
                name, role, area, ..
            } = &element;
            let _ = write!(
                out,
                "  {role:?} {name:?} at ({:.0}, {:.0}) {:.0}x{:.0}",
                area.left, area.top, area.width, area.height,
            );
            if let Some(value) = &element.value {
                let _ = write!(out, " holds {value:?}");
            }
            if let Some(number) = element.number {
                let _ = write!(out, " stands at {number}");
            }
            for (holds, state) in [
                (element.on, " on"),
                (element.focused, " focused"),
                (!element.enabled, " disabled"),
            ] {
                out.push_str(if holds { state } else { "" });
            }
            out.push('\n');
        }
        out
    }
}
