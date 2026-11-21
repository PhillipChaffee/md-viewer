//! md-viewer patch: the running text of a paragraph, a list item, a quote
//! or a heading, kept until what follows each piece is known.

use egui::text::LayoutJob;
use egui::Ui;

/// A piece of running text, kept until what follows it is known. A row may
/// end between two pieces only where it could end inside one: a piece that
/// touches the next, with no place to break between them, keeps room for
/// the start of that one on its row.
pub(super) struct Run {
    pub(super) job: LayoutJob,
    /// Where it leads, when it is a link.
    pub(super) link: Option<String>,
    /// It holds find's current match: where it is set is where find
    /// scrolls to.
    pub(super) active: bool,
    /// No row may end between the run before it and this one.
    touches: bool,
}

/// The pieces that wait, in the order they are set.
#[derive(Default)]
pub(super) struct Runs(Vec<Run>);

impl Runs {
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Add a piece. It touches the piece before it when no row may end
    /// between the last character of that one and its first.
    pub(super) fn push(&mut self, job: LayoutJob, link: Option<String>, active: bool) {
        let end = self.0.last().and_then(|run| run.job.text.chars().next_back());
        let touches = match (end, job.text.chars().next()) {
            (Some(end), Some(start)) => !egui::epaint::text::is_word_boundary(end, Some(start)),
            _ => false,
        };
        self.0.push(Run {
            job,
            link,
            active,
            touches,
        });
    }

    /// Hand out the pieces, each with the room that those touching it need
    /// after it. The buffer is emptied and keeps the room it has.
    pub(super) fn drain_with_room(&mut self, ui: &Ui) -> std::vec::Drain<'_, Run> {
        keep_room_for_touching_runs(ui, &mut self.0);
        self.0.drain(..)
    }
}

/// The widths of pieces are measured apart from the row they are set on,
/// each rounded to the pixel on its own: the room kept for a piece is this
/// much more than its measure.
const RUN_ROUNDING: f32 = 1.0;

/// How wide the first `len` bytes of `job` are on one row.
fn width_of_start(ui: &Ui, job: &LayoutJob, len: usize) -> f32 {
    if len == 0 {
        return 0.0;
    }
    let sections = job
        .sections
        .iter()
        .filter(|section| section.byte_range.start < len);
    let start = LayoutJob {
        text: job.text[..len].to_owned(),
        sections: sections
            .map(|section| egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: section.byte_range.start..section.byte_range.end.min(len),
                format: section.format.clone(),
            })
            .collect(),
        round_output_to_gui: false,
        ..Default::default()
    };
    ui.fonts_mut(|fonts| fonts.layout_job(start)).rect.width()
}

/// Give each of `runs` the room that the runs touching it need after it:
/// what they hold up to the first place a row may break.
fn keep_room_for_touching_runs(ui: &Ui, runs: &mut [Run]) {
    let mut room = 0.0;
    for run in runs.iter_mut().rev() {
        run.job.wrap.room_after = room;
        room = if run.touches {
            let (word, ends) = egui::epaint::text::first_word(&run.job.text);
            let width = width_of_start(ui, &run.job, word.len()) + RUN_ROUNDING;
            if ends { width } else { width + room }
        } else {
            0.0
        };
    }
}
