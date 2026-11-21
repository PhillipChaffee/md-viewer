//! The lines the document is set in: which words share a line, and where
//! each line of one piece of text lies.

use eframe::egui::Rect;

use super::paint::{painted_lines, Stretch};
use super::{Area, Reader};

/// How far left of the text column, in pixels, a piece of text may start
/// and still be on the page.
const EDGE: f32 = 1.0;

/// One line of the document: everything set on it, read left to right.
#[derive(Clone, Debug)]
pub(crate) struct Line {
    pub(crate) text: String,
    pub(crate) area: Area,
}

fn area(rect: Rect) -> Area {
    Area {
        left: rect.left(),
        top: rect.top(),
        width: rect.width(),
        height: rect.height(),
    }
}

/// `stretches` gathered into lines, top to bottom. Two stretches share a
/// line when the middle of one lies within the height of the other: code
/// is set smaller than the prose it stands in.
fn gathered(mut stretches: Vec<Stretch>) -> Vec<Vec<Stretch>> {
    stretches.sort_by(|a, b| a.rect.center().y.total_cmp(&b.rect.center().y));
    let mut lines: Vec<Vec<Stretch>> = Vec::new();
    for stretch in stretches {
        let middle = stretch.rect.center().y;
        let shares = |line: &Vec<Stretch>| {
            line.iter()
                .any(|set| (set.rect.top()..set.rect.bottom()).contains(&middle))
        };
        match lines.last_mut().filter(|line| shares(line)) {
            Some(line) => line.push(stretch),
            None => lines.push(vec![stretch]),
        }
    }
    lines
}

impl Reader {
    /// The lines of the document as the last frame set them, top to
    /// bottom: every piece of text painted in the text column, joined along
    /// its line with nothing put between the pieces.
    #[track_caller]
    pub(crate) fn lines(&self) -> Vec<Line> {
        let scale = self.scale();
        let column = self.column();
        let painted = painted_lines(&self.shapes()).into_iter().flatten();
        let on_page = painted
            .map(|stretch| Stretch {
                rect: stretch.rect * scale,
                ..stretch
            })
            .filter(|stretch| {
                let rect = stretch.rect;
                rect.left() >= column.left - EDGE
                    && rect.left() < column.right()
                    && rect.top() >= column.top
            });
        let lines = gathered(on_page.collect()).into_iter();
        lines
            .map(|mut line| {
                line.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
                let around = line
                    .iter()
                    .fold(Rect::NOTHING, |around, set| around.union(set.rect));
                Line {
                    text: line.iter().map(|set| set.text.as_str()).collect(),
                    area: area(around),
                }
            })
            .collect()
    }

    /// Where each line of the first painted text that contains `text` lies,
    /// top to bottom: one area for a text on one line, more for one that
    /// wraps.
    #[track_caller]
    pub(crate) fn lines_of(&self, text: &str) -> Vec<Area> {
        let scale = self.scale();
        let mut painted = painted_lines(&self.shapes()).into_iter();
        let found = painted.find(|lines| {
            let whole: String = lines.iter().map(|line| line.text.as_str()).collect();
            whole.contains(text)
        });
        let Some(found) = found else {
            panic!("no painted text contains `{text}`\n{}", self.describe());
        };
        found.iter().map(|line| area(line.rect * scale)).collect()
    }
}
