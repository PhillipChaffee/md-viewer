//! The top row: line icons at both ends, and between them the tabs of the
//! open documents, the new-tab button and the edit controls. The row is
//! there with any number of documents, so nothing under it moves when one
//! opens or closes.

use eframe::egui::{self, Rangef};

use crate::app::action::AppAction;
use crate::app::chrome::{self, QuietButton, BAR_HEIGHT, BUTTON_SIDE};
use crate::app::tab_bar::TabPicks;
use crate::app::MarkdownApp;
use crate::icons::Icon;

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

/// Space between two buttons of the bar.
const BUTTON_GAP: f32 = 6.0;
/// Buttons at the left end of the bar: files, home, back, forward.
const LEADING_BUTTONS: f32 = 4.0;
/// Buttons at the right end of the bar: appearance, find, contents, more.
const TRAILING_BUTTONS: f32 = 4.0;
/// Room the row keeps empty before its first button. A platform that draws
/// the window's own buttons inside the row keeps their room here.
const LEADING_INSET: f32 = 0.0;
/// The least room between the buttons at both ends at which the edit
/// controls join the row. In less, they would leave no room for a tab and
/// the new-tab button; the menu and Ctrl+E still switch modes.
pub(super) const EDIT_CONTROLS_ROOM: f32 = 300.0;

/// Record `action` as the bar's pick for this frame when `button` was
/// clicked.
pub(super) fn pick(button: &egui::Response, action: AppAction, picked: &mut Option<AppAction>) {
    if button.clicked() {
        *picked = Some(action);
    }
}

/// The width of `count` buttons side by side.
fn buttons_width(count: f32) -> f32 {
    count.mul_add(BUTTON_SIDE, (count - 1.0) * BUTTON_GAP)
}

/// Where the parts of the row go, from left to right.
#[derive(Clone, Copy, Debug)]
struct Row {
    /// The room kept empty before the first button.
    inset: Rangef,
    leading: Rangef,
    /// What the buttons at both ends leave: the tabs, the new-tab button,
    /// empty room, and the edit controls. Empty in a window too narrow
    /// for more than the buttons.
    middle: Rangef,
    trailing: Rangef,
}

impl Row {
    /// The row of a bar that spans `bar`, with `inset` px kept empty at
    /// its left end.
    fn new(bar: Rangef, inset: f32) -> Self {
        let first = bar.min + inset;
        let leading = Rangef::new(first, first + buttons_width(LEADING_BUTTONS));
        let trailing = Rangef::new(bar.max - buttons_width(TRAILING_BUTTONS), bar.max);
        let left = leading.max + BUTTON_GAP;
        Self {
            inset: Rangef::new(bar.min, first),
            leading,
            middle: Rangef::new(left, (trailing.min - BUTTON_GAP).max(left)),
            trailing,
        }
    }

    /// Whether the middle has room for the edit controls beside a tab.
    fn fits_edit_controls(self) -> bool {
        self.middle.span() >= EDIT_CONTROLS_ROOM
    }

    /// Where the tabs and the new-tab button go: the middle, up to the
    /// edit controls where those start at `edit_left`.
    fn tab_room(self, edit_left: Option<f32>) -> Rangef {
        let right = edit_left.map_or(self.middle.max, |left| left - BUTTON_GAP);
        Rangef::new(
            self.middle.min,
            right.clamp(self.middle.min, self.middle.max),
        )
    }

    /// The parts of the row that hold nothing: the inset, and what the
    /// tabs and the new-tab button, which reach to `used`, leave of
    /// `room`.
    fn background(self, room: Rangef, used: f32) -> [Rangef; 2] {
        [
            self.inset,
            Rangef::new(used.clamp(room.min, room.max), room.max),
        ]
    }
}

/// The width the tabs may show in: `room` less the new-tab button's place.
pub(super) fn tabs_width(room: Rangef) -> f32 {
    (room.span() - BUTTON_SIDE).max(0.0)
}

/// Where the new-tab button goes: right after tabs that show `shown` px
/// wide, and inside `room` however wide they are. `None` where the room
/// cannot hold a button.
pub(super) fn new_tab_span(room: Rangef, shown: f32) -> Option<Rangef> {
    if room.span() < BUTTON_SIDE {
        return None;
    }
    let right = (room.min + shown.max(0.0) + BUTTON_SIDE).min(room.max);
    Some(Rangef::new((right - BUTTON_SIDE).max(room.min), right))
}

/// A part of the bar: its full height over `span`, laid out by `layout`.
pub(super) fn part(bar: egui::Rect, span: Rangef, layout: egui::Layout) -> egui::UiBuilder {
    let rect = egui::Rect::from_x_y_ranges(span, bar.y_range());
    egui::UiBuilder::new().max_rect(rect).layout(layout)
}

/// Take the row's empty parts for the bar itself, so nothing under the
/// bar is pointed at through them.
fn row_background(ui: &egui::Ui, bar: egui::Rect, empty: [Rangef; 2]) {
    for (index, span) in empty.into_iter().enumerate() {
        let rect = egui::Rect::from_x_y_ranges(span, bar.y_range());
        ui.interact(
            rect,
            ui.id().with(("background", index)),
            egui::Sense::hover(),
        );
    }
}

impl MarkdownApp {
    /// Render the top bar, then carry out the action picked from it.
    pub(crate) fn render_top_bar(&mut self, ctx: &egui::Context) {
        let palette = self.theme.palette();
        let mut picked = None;
        let mut tab_picks = TabPicks::default();
        egui::TopBottomPanel::top("top_bar")
            .exact_height(BAR_HEIGHT)
            .frame(chrome::bar_frame(palette))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.x = BUTTON_GAP;
                let bar = ui.max_rect();
                let row = Row::new(bar.x_range(), LEADING_INSET);
                let from_left = egui::Layout::left_to_right(egui::Align::Center);
                let from_right = egui::Layout::right_to_left(egui::Align::Center);
                ui.scope_builder(part(bar, row.leading, from_left), |ui| {
                    self.leading_buttons(ui, &mut picked);
                });
                ui.scope_builder(part(bar, row.trailing, from_left), |ui| {
                    self.trailing_buttons(ui, &mut picked);
                });
                // The edit controls are as wide as what they have to say,
                // so they are laid out before the tabs, which take the rest.
                let edit_left = if row.fits_edit_controls() {
                    self.edit_controls(ui, part(bar, row.middle, from_right))
                } else {
                    None
                };
                let room = row.tab_room(edit_left);
                let used = self.tabs_in_row(ui, bar, room, &mut picked, &mut tab_picks);
                row_background(ui, bar, row.background(room, used.unwrap_or(room.min)));
            });

        // The pick runs after the bar is laid out, so the bar still shows
        // the state from before the action; repaint to catch up.
        if let Some(action) = picked {
            self.apply_action(ctx, action);
        }
        if self.apply_tab_picks(&tab_picks) || picked.is_some() {
            ctx.request_repaint();
        }
    }

    /// Whether there is a way back and a way forward: through the
    /// history of the tab in front, or from the home page back to the tab
    /// it shows over.
    pub(crate) fn history_availability(&self) -> (bool, bool) {
        self.front_tab().map_or((self.home_shows, false), |tab| {
            (tab.can_go_back(), tab.can_go_forward())
        })
    }

    /// The file explorer toggle, home, and the history arrows.
    fn leading_buttons(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();
        let (can_back, can_forward) = self.history_availability();

        let files = QuietButton::icon(Icon::Sidebar, "Files sidebar").show(ui, palette);
        let files = self
            .hints
            .tooltip(files, "Files", AppAction::ToggleExplorer);
        pick(&files, AppAction::ToggleExplorer, picked);

        let home = QuietButton::icon(Icon::Home, "Home")
            .lit(self.front().is_none())
            .show(ui, palette);
        let home = self.hints.tooltip(home, "Home", AppAction::GoHome);
        pick(&home, AppAction::GoHome, picked);

        let back = QuietButton::icon(Icon::Back, "Go back")
            .enabled(can_back)
            .show(ui, palette);
        let back = self.hints.tooltip(back, "Back", AppAction::NavigateBack);
        pick(&back, AppAction::NavigateBack, picked);

        let forward = QuietButton::icon(Icon::Forward, "Go forward")
            .enabled(can_forward)
            .show(ui, palette);
        let forward = self
            .hints
            .tooltip(forward, "Forward", AppAction::NavigateForward);
        pick(&forward, AppAction::NavigateForward, picked);
    }

    /// Appearance, find, the outline toggle, and everything else.
    fn trailing_buttons(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();

        let appearance = QuietButton::text_sized("Aa", 15.0)
            .labelled("Appearance")
            .snug()
            .lit(chrome::next_widget_popup_is_open(ui))
            .show(ui, palette);
        self.appearance_popover(&appearance, picked);
        appearance.on_hover_text("Theme, text size, width and spacing");

        let find = QuietButton::icon(Icon::Search, "Find in document").show(ui, palette);
        let find = self.hints.tooltip(find, "Find", AppAction::OpenSearch);
        pick(&find, AppAction::OpenSearch, picked);

        let contents = QuietButton::icon(Icon::Outline, "Contents sidebar").show(ui, palette);
        let contents = self
            .hints
            .tooltip(contents, "Contents", AppAction::ToggleOutline);
        pick(&contents, AppAction::ToggleOutline, picked);

        let more = QuietButton::icon(Icon::More, "More")
            .lit(chrome::next_widget_popup_is_open(ui))
            .show(ui, palette);
        self.overflow_menu(&more, picked);
    }
}
