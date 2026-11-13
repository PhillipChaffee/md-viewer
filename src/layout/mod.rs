//! Window, sidebar, zoom, and keyboard-scroll geometry shared across the UI.

#[cfg(test)]
mod tests;

// Optimal widths for initial window sizing (based on typography research)
// Content: 600px optimal for 55-75 CPL readability
// Explorer: 200px default with 16px inner margins
// Outline: 200px default with 8px inner margins
// Plus ~16px for panel separators
pub(crate) const CONTENT_OPTIMAL_WIDTH: f32 = 600.0;
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

pub(crate) fn content_default_width(full_width_content: bool) -> Option<usize> {
    if full_width_content {
        None
    } else {
        Some(CONTENT_OPTIMAL_WIDTH as usize)
    }
}

pub(crate) fn content_width_limit(full_width_content: bool, available_width: f32) -> usize {
    let available_width = available_width.max(0.0) as usize;
    content_default_width(full_width_content)
        .map_or(available_width, |preferred| preferred.min(available_width))
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
