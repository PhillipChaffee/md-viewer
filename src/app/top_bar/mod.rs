//! The top bar: a few line icons either side of the document's title.

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::{self, QuietButton, BAR_HEIGHT, BUTTON_SIDE};
use crate::app::mcp;
use crate::app::MarkdownApp;
use crate::icons::Icon;
use crate::tab::Tab;
use crate::theme::Palette;

#[cfg(kani)]
mod proofs;
#[cfg(test)]
mod tests;

/// The least space kept between the title and the buttons beside it.
const TITLE_GAP: f32 = 12.0;
/// Space between two buttons of the bar.
const BUTTON_GAP: f32 = 6.0;
/// Buttons at the right end of the bar: appearance, find, contents, more.
const TRAILING_BUTTONS: f32 = 4.0;
/// Diameter of the dot that stands for live reload in the title's hover text.
const LIVE_DOT: f32 = 6.0;

/// Record `action` as the bar's pick for this frame when `button` was
/// clicked.
fn pick(button: &egui::Response, action: AppAction, picked: &mut Option<AppAction>) {
    if button.clicked() {
        *picked = Some(action);
    }
}

/// Where the middle of a title `width` px wide goes: the middle of the bar,
/// unless the title would then run into the buttons at one end, in which
/// case it moves just clear of them.
fn title_center(bar_center: f32, free: egui::Rangef, width: f32) -> f32 {
    let half = width / 2.0 + TITLE_GAP;
    let (leftmost, rightmost) = (free.min + half, free.max - half);
    if leftmost > rightmost {
        return free.center();
    }
    bar_center.clamp(leftmost, rightmost)
}

/// What the bar calls a document: its first heading, else its file name.
fn document_name(tab: &Tab) -> std::borrow::Cow<'_, str> {
    tab.document_title
        .as_deref()
        .map_or_else(|| tab.title().into(), Into::into)
}

/// The hover text of the title: where the document is, and whether it is
/// reloaded when the file changes.
fn title_hover(ui: &mut egui::Ui, tab: &Tab, live: bool, palette: &Palette) {
    ui.style_mut().override_font_id = Some(chrome::label_font());
    ui.label(tab.path.display().to_string());
    if live {
        ui.horizontal(|ui| {
            let (dot, _) =
                ui.allocate_exact_size(egui::Vec2::splat(LIVE_DOT), egui::Sense::hover());
            ui.painter()
                .circle_filled(dot.center(), LIVE_DOT / 2.0, palette.muted);
            chrome::small_label(ui, palette, "Live: reloads when the file changes");
        });
    }
}

impl MarkdownApp {
    /// Render the top bar, then carry out the action picked from it.
    pub(crate) fn render_top_bar(&mut self, ctx: &egui::Context) {
        let palette = self.theme.palette();
        let mut picked = None;
        egui::TopBottomPanel::top("top_bar")
            .exact_height(BAR_HEIGHT)
            .frame(chrome::bar_frame(palette))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.x = BUTTON_GAP;
                let bar = ui.max_rect();
                let leading = ui
                    .scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(bar)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        |ui| self.leading_buttons(ui, &mut picked),
                    )
                    .response
                    .rect;
                // Laid out left to right from a known left edge, so the Tab
                // key walks the buttons in reading order.
                let trailing_width =
                    TRAILING_BUTTONS.mul_add(BUTTON_SIDE, (TRAILING_BUTTONS - 1.0) * BUTTON_GAP);
                let trailing = bar.with_min_x(bar.right() - trailing_width);
                let beside_trailing = bar.with_max_x(trailing.left() - BUTTON_GAP);
                let edit_left = self.lone_document_edit_controls(ui, beside_trailing);
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(trailing)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| self.trailing_buttons(ui, &mut picked),
                );
                let free = egui::Rangef::new(leading.right(), edit_left.unwrap_or(trailing.min.x));
                self.document_title(ui, bar, free);
            });

        // The pick runs after the bar is laid out, so the bar still shows
        // the state from before the action; repaint to catch up.
        if let Some(action) = picked {
            self.apply_action(ctx, action);
            ctx.request_repaint();
        }
    }

    /// Whether the active tab has history to step back and forward through.
    pub(crate) fn history_availability(&self) -> (bool, bool) {
        self.tabs
            .get(self.active_tab)
            .map_or((false, false), |tab| {
                (tab.can_go_back(), tab.can_go_forward())
            })
    }

    /// The file explorer toggle and the history arrows.
    fn leading_buttons(&self, ui: &mut egui::Ui, picked: &mut Option<AppAction>) {
        let palette = self.theme.palette();
        let (can_back, can_forward) = self.history_availability();

        let files = QuietButton::icon(Icon::Sidebar, "Files sidebar")
            .show(ui, palette)
            .on_hover_text("Files (Ctrl+Shift+E)");
        mcp::widget(
            &self.mcp,
            format_args!("Top Bar: Files"),
            "button",
            &files,
            None,
        );
        pick(&files, AppAction::ToggleExplorer, picked);

        let back = QuietButton::icon(Icon::Back, "Go back")
            .enabled(can_back)
            .show(ui, palette)
            .on_hover_text("Back (Alt+Left)");
        mcp::widget(
            &self.mcp,
            format_args!("Navigate Back"),
            "button",
            &back,
            None,
        );
        pick(&back, AppAction::NavigateBack, picked);

        let forward = QuietButton::icon(Icon::Forward, "Go forward")
            .enabled(can_forward)
            .show(ui, palette)
            .on_hover_text("Forward (Alt+Right)");
        mcp::widget(
            &self.mcp,
            format_args!("Navigate Forward"),
            "button",
            &forward,
            None,
        );
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
        mcp::widget(
            &self.mcp,
            format_args!("Top Bar: Appearance"),
            "button",
            &appearance,
            None,
        );
        self.appearance_popover(&appearance, picked);
        appearance.on_hover_text("Theme, text size, width and spacing");

        let find = QuietButton::icon(Icon::Search, "Find in document")
            .show(ui, palette)
            .on_hover_text("Find (Ctrl+F)");
        mcp::widget(
            &self.mcp,
            format_args!("Top Bar: Find"),
            "button",
            &find,
            None,
        );
        pick(&find, AppAction::OpenSearch, picked);

        let contents = QuietButton::icon(Icon::Outline, "Contents sidebar")
            .show(ui, palette)
            .on_hover_text("Contents (Ctrl+Shift+O)");
        mcp::widget(
            &self.mcp,
            format_args!("Top Bar: Contents"),
            "button",
            &contents,
            None,
        );
        pick(&contents, AppAction::ToggleOutline, picked);

        let more = QuietButton::icon(Icon::More, "More")
            .lit(chrome::next_widget_popup_is_open(ui))
            .show(ui, palette);
        mcp::widget(
            &self.mcp,
            format_args!("Top Bar: More"),
            "button",
            &more,
            None,
        );
        self.overflow_menu(&more, picked);
    }

    /// The active document's name, in the middle of the bar where there is
    /// room for it there, cut short where there is not.
    fn document_title(&self, ui: &mut egui::Ui, bar: egui::Rect, free: egui::Rangef) {
        let Some(tab) = self.tabs.get(self.active_tab) else {
            return;
        };
        let palette = self.theme.palette();
        let name = document_name(tab);
        let mut job = egui::text::LayoutJob::simple_singleline(
            name.clone().into_owned(),
            chrome::label_font(),
            egui::Color32::PLACEHOLDER,
        );
        let room = 2.0_f32.mul_add(-TITLE_GAP, free.span()).max(0.0);
        job.wrap = egui::text::TextWrapping::truncate_at_width(room);
        let galley = ui.painter().layout_job(job);
        let center = egui::pos2(
            title_center(bar.center().x, free, galley.size().x),
            bar.center().y,
        );
        let rect = egui::Rect::from_center_size(center, galley.size());

        let response = ui.allocate_rect(rect, egui::Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, name.as_ref())
        });
        let color = if response.hovered() {
            palette.text
        } else {
            palette.muted
        };
        ui.painter().galley(rect.min, galley, color);
        let live = self.watcher.is_some();
        response.on_hover_ui(|ui| title_hover(ui, tab, live, palette));
    }
}
