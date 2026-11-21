//! What a picture on the page looks like, how a text is set, and what the
//! app asked of the desktop around it.

use super::paint::{dots, marks, pictures, Drawing, Painted, Picture};
use super::{Area, Reader, Rgb};

/// How far apart, in pixels, two areas may be and still be one picture.
const SAME_PLACE: f32 = 0.5;

impl Reader {
    /// The picture the last frame painted in `area`, one of
    /// [`Self::pictures`].
    #[track_caller]
    fn picture_in(&self, area: Area) -> Picture {
        let scale = self.scale();
        let painted = pictures(&self.shapes());
        let found = painted.into_iter().find(|picture| {
            let rect = picture.rect * scale;
            (rect.left() - area.left).abs() < SAME_PLACE
                && (rect.top() - area.top).abs() < SAME_PLACE
                && (rect.width() - area.width).abs() < SAME_PLACE
        });
        found.unwrap_or_else(|| panic!("no picture is painted in {area:?}"))
    }

    /// The ink of the picture in `area`: the average of the colors it
    /// paints. A formula's is the color of its strokes, a photo's is its
    /// overall tint.
    #[track_caller]
    pub(crate) fn picture_ink(&self, area: Area) -> Rgb {
        let ink = self.pixels.ink(self.picture_in(area));
        ink.unwrap_or_else(|| panic!("the picture in {area:?} paints nothing"))
    }

    /// How much of its rectangle the picture in `area` paints: near 0 for
    /// thin strokes on the bare page, 1 for a filled box.
    #[track_caller]
    pub(crate) fn picture_cover(&self, area: Area) -> f64 {
        self.pixels.cover(self.picture_in(area))
    }

    /// The first painted text that contains `text`.
    #[track_caller]
    fn painted_with(&self, text: &str) -> Painted {
        let mut painted = self.painted().into_iter();
        let found = painted.find(|painted| painted.text.contains(text));
        found.unwrap_or_else(|| panic!("no painted text contains `{text}`\n{}", self.describe()))
    }

    /// Every color the first painted text that contains `text` is set in,
    /// in reading order: one for plain prose, several for highlighted code.
    #[track_caller]
    pub(crate) fn inks_of(&self, text: &str) -> Vec<Rgb> {
        self.painted_with(text).inks
    }

    /// How wide the first line of the first painted text that contains
    /// `text` is, in pixels.
    #[track_caller]
    pub(crate) fn width_of(&self, text: &str) -> f32 {
        self.painted_with(text).first_line_width * self.scale()
    }

    /// How `character` is drawn on the first line of the first painted text
    /// that contains `text`: its picture, before the ink of the text.
    #[track_caller]
    pub(crate) fn drawing_of(&self, text: &str, character: char) -> Drawing {
        let drawings = self.drawings_of(text, character);
        let first = drawings.into_iter().next();
        first.unwrap_or_else(|| panic!("no painted `{text}` has `{character}` on its first line"))
    }

    /// How `character` is drawn in every painted text that contains `text`
    /// and has it on its first line, in the order painted: a heading is on
    /// the page, in the top bar and in the contents.
    pub(crate) fn drawings_of(&self, text: &str, character: char) -> Vec<Drawing> {
        let mut painted = self.painted();
        painted.retain(|painted| painted.text.contains(text));
        self.kit.ctx.fonts(|fonts| {
            // The sheet the frame keeps the picture of every character on.
            let sheet = fonts.image();
            let drawings = painted.iter();
            drawings
                .filter_map(|painted| painted.drawing_of(character, &sheet))
                .collect()
        })
    }

    /// The box the picture of `character` is painted in, on the first line
    /// of the first painted text that contains `text`: its bottom is where
    /// a letter such as `x` stands, its height how tall the letter is.
    #[track_caller]
    pub(crate) fn letter_of(&self, text: &str, character: char) -> Area {
        let painted = self.painted_with(text);
        let found = painted.letters.iter().find(|(set, _)| *set == character);
        let Some((_, drawn)) = found else {
            panic!("the painted `{text}` has no `{character}` on its first line");
        };
        let drawn = *drawn * self.scale();
        Area {
            left: drawn.left(),
            top: drawn.top(),
            width: drawn.width(),
            height: drawn.height(),
        }
    }

    /// The size, in pixels, of every painted text that reads exactly
    /// `text`, in the order painted: a row of the sidebar, then the hover
    /// text that repeats it.
    pub(crate) fn sizes_of(&self, text: &str) -> Vec<f32> {
        let painted = self.painted().into_iter();
        painted
            .filter(|painted| painted.text == text)
            .map(|painted| painted.size * self.scale())
            .collect()
    }

    /// The color laid behind the first painted text that contains `text`,
    /// as find lays one behind a match. `None` when nothing is.
    #[track_caller]
    pub(crate) fn marker_of(&self, text: &str) -> Option<Rgb> {
        self.painted_with(text).marker
    }

    /// Whether a line is drawn through the painted text that contains
    /// `text`.
    #[track_caller]
    pub(crate) fn is_struck_out(&self, text: &str) -> bool {
        self.painted_with(text).struck_out
    }

    /// Whether a line is drawn under the painted text that contains `text`.
    #[track_caller]
    pub(crate) fn is_underlined(&self, text: &str) -> bool {
        self.painted_with(text).underlined
    }

    /// Where the text `label` is on the page itself, for words the bars and
    /// sidebars around the page repeat: the document's title is one.
    #[track_caller]
    pub(crate) fn on_page(&self, label: &str) -> Area {
        let column = self.column();
        let mut found = self.all(label);
        found.retain(|element| {
            let area = element.area;
            area.left >= column.left - SAME_PLACE
                && area.right() <= column.right() + SAME_PLACE
                && area.top >= column.top
        });
        assert!(
            found.len() == 1,
            "expected one `{label}` on the page, found {}\n{}",
            found.len(),
            self.describe()
        );
        found[0].area
    }

    /// The lines, boxes and dots the last frame drew wholly inside `area`,
    /// each by the rectangle that bounds it, top to bottom: a rule, a
    /// quote's bar, a check box and its tick.
    pub(crate) fn marks_in(&self, area: Area) -> Vec<Area> {
        let scale = self.scale();
        let drawn = marks(&self.shapes()).into_iter();
        let mut inside: Vec<Area> = drawn.map(|rect| Area::of(rect * scale)).collect();
        inside.retain(|mark| area.holds(*mark));
        inside.sort_by(|a, b| a.top.total_cmp(&b.top));
        inside
    }

    /// The dots the last frame drew wholly inside `area`, each with its
    /// color, left to right: the mark on a file that has a tab.
    pub(crate) fn dots_in(&self, area: Area) -> Vec<(Area, Rgb)> {
        let scale = self.scale();
        let drawn = dots(&self.shapes()).into_iter();
        let mut inside: Vec<(Area, Rgb)> = drawn
            .map(|(rect, ink)| (Area::of(rect * scale), ink))
            .collect();
        inside.retain(|(dot, _)| area.holds(*dot));
        inside.sort_by(|(a, _), (b, _)| a.left.total_cmp(&b.left));
        inside
    }

    /// Where the first line of the first painted text that contains `text`
    /// ends, in pixels from the left edge of the window.
    #[track_caller]
    pub(crate) fn end_of(&self, text: &str) -> f32 {
        let painted = self.painted_with(text);
        (painted.first_line.x + painted.first_line_width / 2.0) * self.scale()
    }

    /// The addresses the app has asked the desktop to open in a browser,
    /// oldest first.
    pub(crate) fn opened_in_browser(&self) -> &[String] {
        &self.opened
    }

    /// What the app last put on the clipboard.
    pub(crate) fn clipboard(&self) -> Option<&str> {
        self.copied.as_deref()
    }
}
