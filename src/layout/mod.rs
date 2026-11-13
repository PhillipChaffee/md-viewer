//! Window, sidebar, reading-column, zoom, and keyboard-scroll geometry
//! shared across the UI.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// The least space kept between the text column and each edge of the
/// document pane. It also holds the column's resize strips and the
/// scrollbar, which is why it is wider than both together.
pub(crate) const READING_COLUMN_MARGIN: f32 = 32.0;

// Widths for initial window sizing.
// Content: the default reading column plus its two margins
// Explorer: 200px default with 16px inner margins
// Outline: 200px default with 8px inner margins
// Plus ~16px for panel separators
pub(crate) const CONTENT_OPTIMAL_WIDTH: f32 = ReadingWidth::DEFAULT.0 + 2.0 * READING_COLUMN_MARGIN;
pub(crate) const EXPLORER_DEFAULT_WIDTH: f32 = 216.0; // 200 + 16 margins
pub(crate) const OUTLINE_DEFAULT_WIDTH: f32 = 208.0; // 200 + 8 margins
pub(crate) const PANEL_SEPARATORS: f32 = 16.0;
pub(crate) const OPTIMAL_WINDOW_HEIGHT: f32 = 750.0;
pub(crate) const SIDEBAR_MIN_WIDTH: f32 = 70.0;
pub(crate) const SIDEBAR_RESIZE_GRAB_RADIUS: f32 = 3.0;
pub(crate) const EXPLORER_RIGHT_RESIZE_GUTTER: i8 = 12;
pub(crate) const CONTENT_RIGHT_RESIZE_GUTTER: i8 = 10;

// Keyboard document scroll deltas are centralized so shortcut wiring and tests
// share the same line/page behavior.
const KEYBOARD_LINE_SCROLL_STEP: f32 = 48.0;
const KEYBOARD_PAGE_SCROLL_RATIO: f32 = 0.9;

// App-level keyboard scroll actions; kept private until shortcut wiring needs
// to pass them through `MarkdownApp::update`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum KeyboardScrollAction {
    LineUp,
    LineDown,
    PageUp,
    PageDown,
}

// Compute the next clamped document scroll offset without touching egui state.
pub(crate) fn keyboard_scroll_target(
    current_offset: f32,
    viewport_height: f32,
    content_height: f32,
    action: KeyboardScrollAction,
) -> f32 {
    let page_step = viewport_height * KEYBOARD_PAGE_SCROLL_RATIO;
    let delta = match action {
        KeyboardScrollAction::LineUp => -KEYBOARD_LINE_SCROLL_STEP,
        KeyboardScrollAction::LineDown => KEYBOARD_LINE_SCROLL_STEP,
        KeyboardScrollAction::PageUp => -page_step,
        KeyboardScrollAction::PageDown => page_step,
    };
    let max_scroll = (content_height - viewport_height).max(0.0);

    (current_offset + delta).clamp(0.0, max_scroll)
}

/// Whole pixels in `width`: the fraction is dropped, and a negative or NaN
/// width counts as zero.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to be non-negative first and `as` saturates at usize::MAX; dropping the fraction is the point"
)]
pub(crate) const fn whole_pixels(width: f32) -> usize {
    width.max(0.0) as usize
}

/// Fraction of a document that lies above `line_number`, for estimating a
/// scroll offset before the renderer has measured the real one. An empty
/// document has nothing above any line.
#[expect(
    clippy::cast_precision_loss,
    reason = "line counts stay far below f32's 2^24 exact-integer range, and the result is only a scroll estimate"
)]
pub(crate) fn line_fraction(line_number: usize, content_lines: usize) -> f32 {
    if content_lines == 0 {
        return 0.0;
    }
    line_number as f32 / content_lines as f32
}

/// Width of the text column in logical px at zoom 1.0. Zoom scales the
/// column with the text, so a line keeps its length in characters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ReadingWidth(f32);

impl ReadingWidth {
    pub(crate) const MIN: f32 = 280.0;
    pub(crate) const MAX: f32 = 4000.0;
    pub(crate) const DEFAULT: Self = Self(680.0);
    /// Wider than any pane, so the column takes all the room there is.
    pub(crate) const FIT_WINDOW: Self = Self(Self::MAX);
    /// What one press of the widen or narrow shortcut changes.
    pub(crate) const STEP: f32 = 40.0;

    /// `px` clamped to the allowed range. A width that is not a number
    /// (a corrupt settings file) gives the default.
    pub(crate) const fn new(px: f32) -> Self {
        if px.is_nan() {
            Self::DEFAULT
        } else {
            Self(px.clamp(Self::MIN, Self::MAX))
        }
    }

    pub(crate) const fn get(self) -> f32 {
        self.0
    }

    /// Whether the column takes the whole pane whatever its size.
    pub(crate) fn fits_window(self) -> bool {
        self.0 >= Self::MAX
    }

    /// The column actually used inside a pane of `available` px: the pane
    /// keeps a margin on each side, and a narrow pane narrows the column.
    pub(crate) fn resolve(self, available: f32) -> f32 {
        let room = 2.0_f32.mul_add(-READING_COLUMN_MARGIN, available).max(0.0);
        self.0.min(room).floor()
    }

    /// The width `delta` px away from the column shown in a pane of
    /// `available` px. Stepping from what is shown, not from what is
    /// stored, makes the first step out of fit-window visible.
    pub(crate) fn stepped(self, delta: f32, available: f32) -> Self {
        Self::new(self.resolve(available) + delta)
    }

    /// The width whose column has an edge at `edge_x` in a pane centered
    /// on `center_x`. The column stays centered, so moving one edge moves
    /// the other the same distance the other way.
    pub(crate) fn from_edge(edge_x: f32, center_x: f32) -> Self {
        Self::new(2.0 * (edge_x - center_x).abs())
    }

    /// The width a saved session comes back with. Versions before the
    /// resizable column saved only whether the text filled the window.
    pub(crate) const fn restored(width: Option<f32>, legacy_full_width: Option<bool>) -> Self {
        match (width, legacy_full_width) {
            (Some(width), _) => Self::new(width),
            (None, Some(true)) => Self::FIT_WINDOW,
            (None, _) => Self::DEFAULT,
        }
    }
}

/// The left edge of a column `width` px wide centered between `left` and
/// `right`, on a whole pixel. The renderer centers the same way, so the
/// resize strips line up with the text.
pub(crate) fn centered_column_left(left: f32, right: f32, width: f32) -> f32 {
    left + ((right - left - width) / 2.0).max(0.0).floor()
}

/// How far apart the lines of body text sit.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LineSpacing {
    Compact,
    #[default]
    Comfortable,
    Airy,
}

impl LineSpacing {
    pub(crate) const ALL: [Self; 3] = [Self::Compact, Self::Comfortable, Self::Airy];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Comfortable => "Comfortable",
            Self::Airy => "Airy",
        }
    }

    /// Line height of body text as a multiple of its font size.
    pub(crate) const fn line_height(self) -> f32 {
        match self {
            Self::Compact => 1.45,
            Self::Comfortable => 1.6,
            Self::Airy => 1.8,
        }
    }
}

/// Display-zoom bounds and step, shared by keyboard, menu, and Ctrl+wheel
/// zooming. Steps are multiplicative so the perceived change stays uniform
/// across the whole range (like browsers and the lightbox already do).
const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 3.0;
pub(crate) const ZOOM_STEP: f32 = 1.25;

/// One multiplicative zoom step applied to `current`, clamped to the app's
/// zoom bounds. Neutral, non-finite, or non-positive factors are no-ops —
/// egui reports `zoom_delta() == 1.0` on frames without a zoom gesture.
pub(crate) fn zoomed(current: f32, factor: f32) -> f32 {
    if !factor.is_finite() || factor <= 0.0 || (factor - 1.0).abs() < 1e-3 {
        return current.clamp(ZOOM_MIN, ZOOM_MAX);
    }
    (current * factor).clamp(ZOOM_MIN, ZOOM_MAX)
}
