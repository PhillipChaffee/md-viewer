//! The prompt that asks before a close throws unsaved edits away: Save,
//! Discard or Cancel, for one unsaved document at a time.
//!
//! A close that would lose nothing never comes here in any way the reader
//! can see: it is carried out in the frame it was asked for. One that would
//! waits in [`CloseState::Asking`] until every unsaved document it covers
//! has an answer, and only then closes anything, through the same
//! `close_tab` and `close_other_tabs` an unasked close uses. Cancel at any
//! question therefore leaves every tab open with its edits; what a Save
//! wrote before it stays written.

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::{self, QuietButton};
use crate::app::MarkdownApp;
use crate::tab::Tab;
use crate::theme;

#[cfg(test)]
mod tests;

/// Width of the prompt's sheet, which its question wraps in.
const WIDTH: f32 = 340.0;
const PADDING: i8 = 18;
/// The least width of an answer, so the three read as a row of buttons.
const ANSWER_WIDTH: f32 = 76.0;

/// What the reader asked to close. Tabs are named by their place in the
/// strip, which holds while the prompt asks: it is modal and the app's
/// shortcuts wait, so no tab closes or moves under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Goal {
    /// The tab at this place.
    Tab(usize),
    /// Every tab but the one at this place.
    OtherTabs(usize),
    /// The app.
    Quit,
}

impl Goal {
    /// Whether reaching the goal closes the tab at `index`.
    const fn closes(self, index: usize) -> bool {
        match self {
            Self::Tab(closed) => index == closed,
            Self::OtherTabs(kept) => index != kept,
            Self::Quit => true,
        }
    }
}

/// A close that waits on the reader's answers.
#[derive(Debug)]
pub(super) struct PendingClose {
    goal: Goal,
    /// The unsaved tab the prompt asks about now. Those before it that
    /// the goal closes have been answered for.
    asked: usize,
    /// The question, written when the prompt turned to this tab.
    question: String,
    /// Whether the keyboard has still to be put on Save for this question.
    focus_due: bool,
}

/// Where the app is with closing.
#[derive(Debug, Default)]
pub(super) enum CloseState {
    #[default]
    Idle,
    Asking(PendingClose),
    /// Every unsaved document was answered for and the window was told to
    /// close: its next request to close is let through unasked.
    Quitting,
}

impl CloseState {
    pub(super) const fn is_asking(&self) -> bool {
        matches!(self, Self::Asking(_))
    }
}

/// One of the prompt's three answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CloseAnswer {
    /// Write the document, then go on closing.
    Save,
    /// Go on closing without writing it.
    Discard,
    /// Close nothing after all.
    Cancel,
}

fn question(tab: &Tab) -> String {
    format!("Save changes to {}?", tab.title())
}

impl MarkdownApp {
    /// Close the tab at `index`, asking first when it has unsaved edits.
    pub(crate) fn request_close_tab(&mut self, index: usize) {
        if !self.ask_before(Goal::Tab(index)) {
            self.close_tab(index);
        }
    }

    /// Close every tab but the one at `keep`, asking first about each of
    /// them that has unsaved edits.
    pub(crate) fn request_close_other_tabs(&mut self, keep: usize) {
        if !self.ask_before(Goal::OtherTabs(keep)) {
            self.close_other_tabs(keep);
        }
    }

    /// Answer the window when it asks to close, whether its close button
    /// or the app's own Quit sent the request: with unsaved edits open the
    /// close is refused and the prompt asks about them.
    pub(crate) fn answer_close_request(&mut self, ctx: &egui::Context) {
        let requested = ctx.input(|input| input.viewport().close_requested());
        if !requested || matches!(self.closing, CloseState::Quitting) {
            return;
        }
        if self.ask_before(Goal::Quit) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }

    /// Start asking about the unsaved tabs `goal` would close. Returns
    /// whether there is any, and so whether the goal has to wait.
    fn ask_before(&mut self, goal: Goal) -> bool {
        self.ask_from(goal, 0)
    }

    /// Turn the prompt to the first unsaved tab at or after the place
    /// `from` that `goal` would close. Returns whether there is one.
    fn ask_from(&mut self, goal: Goal, from: usize) -> bool {
        let mut tabs = self.tabs.iter().enumerate().skip(from);
        let unsaved = tabs.find(|(index, tab)| goal.closes(*index) && tab.is_dirty());
        let Some((asked, tab)) = unsaved else {
            return false;
        };
        let question = question(tab);
        // The prompt has to be seen, and the lightbox draws over it.
        self.lightbox.open = None;
        self.closing = CloseState::Asking(PendingClose {
            goal,
            asked,
            question,
            focus_due: true,
        });
        true
    }

    /// Carry out the reader's answer to the prompt.
    pub(crate) fn answer_close_prompt(&mut self, ctx: &egui::Context, answer: CloseAnswer) {
        let CloseState::Asking(pending) = std::mem::take(&mut self.closing) else {
            return;
        };
        let go_on = match answer {
            CloseAnswer::Save => self.save_tab(pending.asked),
            CloseAnswer::Discard => true,
            CloseAnswer::Cancel => false,
        };
        if !go_on || self.ask_from(pending.goal, pending.asked + 1) {
            return;
        }
        match pending.goal {
            Goal::Tab(index) => self.close_tab(index),
            Goal::OtherTabs(keep) => self.close_other_tabs(keep),
            Goal::Quit => {
                self.closing = CloseState::Quitting;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Draw the prompt while a close waits on an answer. Returns the answer
    /// the reader gave this frame.
    pub(crate) fn render_close_prompt(&mut self, ctx: &egui::Context) -> Option<AppAction> {
        let CloseState::Asking(pending) = &mut self.closing else {
            return None;
        };
        let palette = self.theme.palette();
        let focus_save = std::mem::take(&mut pending.focus_due);
        let frame = egui::Frame::popup(&ctx.style())
            .fill(palette.surface)
            .stroke(egui::Stroke::new(1.0_f32, palette.rule))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(PADDING));
        let mut answer = None;
        egui::Modal::new(egui::Id::new("close_prompt"))
            .backdrop_color(theme::SCRIM)
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(WIDTH);
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                ui.label(
                    egui::RichText::new(pending.question.as_str())
                        .font(chrome::label_font())
                        .color(palette.text),
                );
                chrome::small_label(ui, palette, "Edits that are not saved will be lost.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let answers = [
                        ("Save", CloseAnswer::Save),
                        ("Discard", CloseAnswer::Discard),
                        ("Cancel", CloseAnswer::Cancel),
                    ];
                    for (label, given) in answers {
                        let button = QuietButton::text(label)
                            .min_width(ANSWER_WIDTH)
                            .show(ui, palette);
                        if focus_save && given == CloseAnswer::Save {
                            button.request_focus();
                        }
                        if button.clicked() {
                            answer = Some(given);
                        }
                    }
                });
            });
        // Escape is Cancel. A click beside the sheet is not: the app's
        // other dialogs do not close on one either.
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            answer = Some(CloseAnswer::Cancel);
        }
        answer.map(AppAction::AnswerClosePrompt)
    }
}
