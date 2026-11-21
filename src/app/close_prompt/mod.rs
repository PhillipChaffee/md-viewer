//! The prompt that asks before unsaved edits are thrown away, by closing
//! their document or by showing another in its tab: Save, Discard or
//! Cancel, for one unsaved document at a time.
//!
//! It asks a second question before a save replaces what another program
//! wrote to the file while it was edited here: Overwrite or Cancel. That
//! one follows Save in the first, and is all a plain save asks.
//!
//! A request that would lose nothing never comes here in any way the
//! reader can see: it is carried out in the frame it was asked for. One
//! that would waits in [`CloseState::Asking`] until every unsaved document
//! it covers has an answer, and only then closes or replaces anything,
//! through the same `close_tab`, `close_other_tabs` and `take_route` an
//! unasked request uses. Cancel at any question therefore leaves every tab
//! as it is, with its edits; what a Save wrote before it stays written.

use std::borrow::Cow;

use eframe::egui;

use crate::app::action::AppAction;
use crate::app::chrome::{self, QuietButton};
use crate::app::MarkdownApp;
use crate::tab::{Route, Saved, Tab, Theirs};
use crate::theme::{self, Palette};

#[cfg(test)]
mod tests;

/// Width of the prompt's sheet, which its question wraps in.
const WIDTH: f32 = 340.0;
const PADDING: i8 = 18;
/// The least width of an answer, so they read as a row of buttons.
const ANSWER_WIDTH: f32 = 76.0;
/// The most characters of a document's heading the question quotes: two
/// lines of the sheet. The tab shortens the same name to its own width.
const NAME_CHARS: usize = 60;

/// What the reader asked for that takes documents out of their tabs. Tabs
/// are named by their place in the strip, which holds while the prompt
/// asks: it is modal and the app's shortcuts wait, so no tab closes or
/// moves under it.
#[derive(Debug, PartialEq, Eq)]
enum Goal {
    /// Close the tab at this place.
    Tab(usize),
    /// Close every tab but the one at this place.
    OtherTabs(usize),
    /// Close the app.
    Quit,
    /// Show another document in the tab at this place: the one the route
    /// leads to.
    Leave(usize, Route),
}

impl Goal {
    /// Whether reaching the goal takes its document out of `tab`, the one
    /// at `index`: by closing the tab, or by showing another document
    /// in it.
    fn replaces(&self, index: usize, tab: &Tab) -> bool {
        match self {
            Self::Tab(closed) => index == *closed,
            Self::OtherTabs(kept) => index != *kept,
            Self::Quit => true,
            Self::Leave(left, route) => index == *left && tab.leads_away(route),
        }
    }
}

/// What the prompt asks about a document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Question {
    /// Whether to save its unsaved edits before they are lost.
    Unsaved,
    /// Whether to save them over what another program wrote to its file.
    Overwrite,
}

impl Question {
    /// The question as it is put about `tab`.
    fn put_to(self, tab: &Tab) -> String {
        let document = named(tab);
        match self {
            Self::Unsaved => format!("Save changes to {document}?"),
            Self::Overwrite => format!("{document} changed on disk. Overwrite it?"),
        }
    }

    /// What an answer costs, under the question.
    const fn warning(self) -> &'static str {
        match self {
            Self::Unsaved => "Edits that are not saved will be lost.",
            Self::Overwrite => "Saving replaces what another program wrote there.",
        }
    }

    /// The answers the reader can give, in the order of their buttons.
    const fn answers(self) -> &'static [(&'static str, CloseAnswer)] {
        match self {
            Self::Unsaved => &[
                ("Save", CloseAnswer::Save),
                ("Discard", CloseAnswer::Discard),
                ("Cancel", CloseAnswer::Cancel),
            ],
            Self::Overwrite => &[
                ("Overwrite", CloseAnswer::Save),
                ("Cancel", CloseAnswer::Cancel),
            ],
        }
    }

    /// The answer the keyboard starts on: the one that loses no text, so
    /// Enter pressed too soon does no harm.
    const fn safe_answer(self) -> CloseAnswer {
        match self {
            Self::Unsaved => CloseAnswer::Save,
            Self::Overwrite => CloseAnswer::Cancel,
        }
    }

    /// What a save under the question does about text another program
    /// wrote to the file: the first question has not asked about it, and
    /// Save under the second is the reader's word to replace it.
    const fn theirs(self) -> Theirs {
        match self {
            Self::Unsaved => Theirs::Ask,
            Self::Overwrite => Theirs::Replace,
        }
    }
}

/// A request that waits on the reader's answers.
#[derive(Debug)]
pub(super) struct PendingClose {
    /// What the answers lead to. A plain save leads to nothing more: its
    /// document stays in its tab.
    goal: Option<Goal>,
    /// The unsaved tab the prompt asks about now. Those before it that
    /// the goal replaces have been answered for.
    asked: usize,
    /// What it asks about that tab.
    about: Question,
    /// The question, written when the prompt turned to this tab.
    question: String,
    /// Whether the keyboard has still to be put on the safe answer to
    /// this question.
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

/// One of the prompt's answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CloseAnswer {
    /// Write the document, then go on closing. Under the question about
    /// unsaved edits, ask first if that would replace what another program
    /// wrote. Under the question about that, write over it: its button
    /// reads Overwrite.
    Save,
    /// Go on closing without writing it.
    Discard,
    /// Close nothing after all.
    Cancel,
}

/// The document of `tab` as a question names it: as its tab does, by its
/// first heading, and by its file, which tells two documents with one
/// heading apart. A tab named by its file is named once.
fn named(tab: &Tab) -> String {
    let (name, file) = (tab.name(), tab.title());
    if name == file {
        file.to_owned()
    } else {
        format!("“{}” ({file})", shortened(name))
    }
}

/// `name` as the question quotes it: whole, or cut after a word and
/// closed with an ellipsis when it is longer than `NAME_CHARS`.
fn shortened(name: &str) -> Cow<'_, str> {
    let Some((end, _)) = name.char_indices().nth(NAME_CHARS) else {
        return Cow::Borrowed(name);
    };
    let kept = &name[..end];
    let words = kept.rfind(char::is_whitespace).filter(|space| *space > 0);
    let kept = words.map_or(kept, |space| &kept[..space]);
    Cow::Owned(format!("{}…", kept.trim_end()))
}

/// The answers to the question `about` as a row of buttons, with the
/// keyboard put on the safe one where that is `focus_due`. Returns the
/// answer the reader gave this frame.
fn answers(
    ui: &mut egui::Ui,
    palette: &Palette,
    about: Question,
    focus_due: bool,
) -> Option<CloseAnswer> {
    let mut answer = None;
    for &(label, given) in about.answers() {
        let button = QuietButton::text(label)
            .min_width(ANSWER_WIDTH)
            .show(ui, palette);
        if focus_due && given == about.safe_answer() {
            button.request_focus();
        }
        if button.clicked() {
            answer = Some(given);
        }
    }
    answer
}

impl MarkdownApp {
    /// Close the tab at `index`, asking first when it has unsaved edits.
    pub(crate) fn request_close_tab(&mut self, index: usize) {
        self.request(Goal::Tab(index));
    }

    /// Close every tab but the one at `keep`, asking first about each of
    /// them that has unsaved edits.
    pub(crate) fn request_close_other_tabs(&mut self, keep: usize) {
        self.request(Goal::OtherTabs(keep));
    }

    /// Take the tab in front where `route` leads, asking first when that
    /// shows another document in place of one with unsaved edits.
    pub(crate) fn request_route(&mut self, route: Route) {
        if let Some(front) = self.front() {
            self.request(Goal::Leave(front, route));
        }
    }

    /// Save the tab at `index`, asking first when that would replace what
    /// another program wrote to its file.
    pub(crate) fn request_save(&mut self, index: usize) {
        if self.save_tab(index, Theirs::Ask) == Some(Saved::WouldOverwrite) {
            self.ask(None, index, Question::Overwrite);
        }
    }

    /// Reach `goal`: now if that loses no unsaved edits, and otherwise
    /// once the reader has answered for them.
    fn request(&mut self, goal: Goal) {
        if let Err(goal) = self.ask_from(goal, 0) {
            self.reach(goal);
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
        if self.ask_from(Goal::Quit, 0).is_ok() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }

    /// Whether a request to quit has to ask the reader first: edits are
    /// unsaved, and the reader has not already answered for them on the
    /// way to a quit.
    pub(crate) fn quit_has_to_ask(&self) -> bool {
        !matches!(self.closing, CloseState::Quitting) && self.tabs.iter().any(Tab::is_dirty)
    }

    /// Turn the prompt to the first unsaved tab at or after the place
    /// `from` that `goal` would take its document out of. With no such
    /// tab the goal comes back, for nothing stands in its way.
    fn ask_from(&mut self, goal: Goal, from: usize) -> Result<(), Goal> {
        let mut tabs = self.tabs.iter().enumerate().skip(from);
        let unsaved = tabs.find(|(index, tab)| tab.is_dirty() && goal.replaces(*index, tab));
        let Some((asked, _)) = unsaved else {
            return Err(goal);
        };
        self.ask(Some(goal), asked, Question::Unsaved);
        Ok(())
    }

    /// Turn the prompt to the tab at `asked`, with what it asks `about`
    /// that tab.
    fn ask(&mut self, goal: Option<Goal>, asked: usize, about: Question) {
        let Some(tab) = self.tabs.get(asked) else {
            return;
        };
        let question = about.put_to(tab);
        // The prompt has to be seen, and the lightbox draws over it.
        self.lightbox.open = None;
        self.closing = CloseState::Asking(PendingClose {
            goal,
            asked,
            about,
            question,
            focus_due: true,
        });
    }

    /// Write the unsaved edits of the tab at `index` to its file. Returns
    /// what that came to, or `None` where the write failed, which the
    /// error bar then says.
    fn save_tab(&mut self, index: usize, theirs: Theirs) -> Option<Saved> {
        let Some(tab) = self.tabs.get_mut(index) else {
            return Some(Saved::Written);
        };
        let saved = tab.save(theirs);
        if let Err(error) = &saved {
            log::error!("Failed to save {}: {error}", tab.path.display());
            self.error_message = Some(format!("Unable to save {}: {error}", tab.path.display()));
        }
        saved.ok()
    }

    /// Carry out the reader's answer to the prompt.
    pub(crate) fn answer_close_prompt(&mut self, answer: CloseAnswer) {
        let CloseState::Asking(pending) = std::mem::take(&mut self.closing) else {
            return;
        };
        let PendingClose {
            goal, asked, about, ..
        } = pending;
        match answer {
            CloseAnswer::Save => match self.save_tab(asked, about.theirs()) {
                Some(Saved::Written) => {}
                Some(Saved::WouldOverwrite) => {
                    self.ask(goal, asked, Question::Overwrite);
                    return;
                }
                None => return,
            },
            CloseAnswer::Discard => {}
            CloseAnswer::Cancel => return,
        }
        // A plain save is done once its document is written.
        let Some(goal) = goal else {
            return;
        };
        if let Err(goal) = self.ask_from(goal, asked + 1) {
            self.reach(goal);
        }
    }

    /// Carry `goal` out now, with whatever the tabs hold unsaved.
    fn reach(&mut self, goal: Goal) {
        match goal {
            Goal::Tab(index) => self.close_tab(index),
            Goal::OtherTabs(keep) => self.close_other_tabs(keep),
            Goal::Quit => {
                self.closing = CloseState::Quitting;
                let window = egui::ViewportCommand::Close;
                self.egui_ctx.send_viewport_cmd(window);
            }
            Goal::Leave(index, route) => self.take_route(index, &route),
        }
    }

    /// Draw the prompt while a request waits on an answer. Returns the answer
    /// the reader gave this frame.
    pub(crate) fn render_close_prompt(&mut self, ctx: &egui::Context) -> Option<AppAction> {
        let CloseState::Asking(pending) = &mut self.closing else {
            return None;
        };
        let palette = self.theme.palette();
        let focus_due = std::mem::take(&mut pending.focus_due);
        let about = pending.about;
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
                chrome::small_label(ui, palette, about.warning());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    // The answers to one question are not those to the
                    // other: a key still down on Save must not find the
                    // keyboard on Overwrite.
                    let buttons = |ui: &mut egui::Ui| answers(ui, palette, about, focus_due);
                    answer = ui.push_id(about, buttons).inner;
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
