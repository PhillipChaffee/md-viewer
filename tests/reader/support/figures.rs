//! What a picture on the page looks like, how a text is set, and what the
//! app asked of the desktop around it.

use super::paint::{marks, pictures, Painted, Picture};
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
    /// quote's bar, a check box and its tick, the chip behind inline code.
    pub(crate) fn marks_in(&self, area: Area) -> Vec<Area> {
        let scale = self.scale();
        let drawn = marks(&self.shapes()).into_iter();
        let mut inside: Vec<Area> = drawn
            .map(|rect| rect * scale)
            .map(|rect| Area {
                left: rect.left(),
                top: rect.top(),
                width: rect.width(),
                height: rect.height(),
            })
            .filter(|mark| {
                mark.left >= area.left
                    && mark.top >= area.top
                    && mark.right() <= area.right()
                    && mark.bottom() <= area.bottom()
            })
            .collect();
        inside.sort_by(|a, b| a.top.total_cmp(&b.top));
        inside
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
