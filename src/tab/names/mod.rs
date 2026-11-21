//! What the open tabs are called beside each other. A tab is named by its
//! document. Where two documents share a name, each shows where it is
//! kept as well: its folder, and as many folders above that as it takes
//! to tell them apart.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

#[cfg(test)]
mod tests;

/// What stands between a name and the place after it.
const SEPARATOR: &str = " · ";

/// The most characters of a place the tab row draws. A longer place is
/// drawn without some of what it shares with the places it is told from.
const PLACE_CHARS: usize = 11;

/// What one tab is called: its document's name and, where another open
/// document has that name too, the place that tells the two apart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShownName {
    /// The name, then the separator and the whole place where there is
    /// one: what a screen reader says.
    whole: String,
    name: String,
    /// What the tab row draws after the name: the separator and the
    /// place, short enough for a tab, or nothing.
    after: String,
}

impl ShownName {
    /// A name that needs no place: no other tab has it.
    pub(crate) fn plain(name: &str) -> Self {
        Self {
            whole: name.to_owned(),
            name: name.to_owned(),
            after: String::new(),
        }
    }

    /// A name with the place its document is kept in.
    fn placed(name: &str, place: &Path) -> Self {
        let after = format!("{SEPARATOR}{}", place.display());
        Self {
            whole: format!("{name}{after}"),
            name: name.to_owned(),
            after,
        }
    }

    /// The same name, with its place drawn as `place` in the tab row.
    fn drawn_as(mut self, place: &str) -> Self {
        self.after = format!("{SEPARATOR}{place}");
        self
    }

    /// The whole of it, as a screen reader says it: `Notes · guides`.
    pub(crate) fn as_str(&self) -> &str {
        &self.whole
    }

    /// The whole of it, to keep where only the whole is read.
    pub(crate) fn into_string(self) -> String {
        self.whole
    }

    /// The place, where there is one.
    fn place(&self) -> Option<&str> {
        self.after.strip_prefix(SEPARATOR)
    }

    /// What the tab row draws: the name, and after it the separator and
    /// the place, or nothing. A row short of room cuts the name and
    /// keeps the rest, which tells the tab from its namesakes.
    pub(crate) fn parts(&self) -> (&str, &str) {
        (&self.name, &self.after)
    }
}

/// One document while its name is worked out: what it shows now, and
/// how to show more of where it is kept when another shows the same.
struct Candidate<'a> {
    name: &'a str,
    path: &'a Path,
    /// The folders the document is in, from the root down.
    folders: Vec<Component<'a>>,
    /// The file, where the folders tell nothing apart without it.
    file: Option<&'a OsStr>,
    /// How many of the nearest folders `shown` names.
    depth: usize,
    shown: ShownName,
    /// The place as the tab row draws it, where the whole place is too
    /// long for a tab.
    short: Option<String>,
}

impl<'a> Candidate<'a> {
    /// `name` alone to start with. `beside_namesake` is whether a
    /// document of the same name is kept in the same folder: the folder
    /// then tells nothing apart without the file.
    fn new(name: &'a str, path: &'a Path, beside_namesake: bool) -> Self {
        let folders = path.parent().map(Path::components);
        Self {
            name,
            path,
            folders: folders.into_iter().flatten().collect(),
            file: path.file_name().filter(|_| beside_namesake),
            depth: 0,
            shown: ShownName::plain(name),
            short: None,
        }
    }

    /// Show more of where the document is kept: one more folder, and
    /// after the last of them the whole path, which no two documents
    /// share. Returns whether there was more to show.
    fn lengthen(&mut self) -> bool {
        let longer = if self.depth < self.folders.len() {
            self.depth += 1;
            let nearest = self.folders.iter().skip(self.folders.len() - self.depth);
            let mut place: PathBuf = nearest.collect();
            place.extend(self.file);
            ShownName::placed(self.name, &place)
        } else {
            ShownName::placed(self.name, self.path)
        };
        let is_longer = longer != self.shown;
        self.shown = longer;
        is_longer
    }

    /// What the tab row draws for the document: its name and its place,
    /// the short one where it has one.
    fn drawn(&self) -> String {
        let place = self.short.as_deref().or_else(|| self.shown.place());
        place.map_or_else(
            || self.name.to_owned(),
            |place| format!("{}{SEPARATOR}{place}", self.name),
        )
    }

    /// What the document is called, once that is settled.
    fn named(self) -> ShownName {
        match self.short {
            Some(short) => self.shown.drawn_as(&short),
            None => self.shown,
        }
    }
}

/// How many characters all of `places` start with, and how many of the
/// rest they all end with.
fn shared(places: &BTreeSet<&str>) -> (usize, usize) {
    let Some(first) = places.first() else {
        return (0, 0);
    };
    let alike = |left: &mut dyn Iterator<Item = char>, right: &mut dyn Iterator<Item = char>| {
        left.zip(right)
            .take_while(|(left, right)| left == right)
            .count()
    };
    let shortest = places.iter().map(|place| place.chars().count()).min();
    let others = || places.iter().skip(1);
    let start = others()
        .map(|other| alike(&mut first.chars(), &mut other.chars()))
        .min()
        .or(shortest)
        .unwrap_or(0);
    let end = others()
        .map(|other| alike(&mut first.chars().rev(), &mut other.chars().rev()))
        .min()
        .unwrap_or(0);
    (start, end.min(shortest.unwrap_or(0) - start))
}

/// `place` as the tab row draws it where it is too long for a tab: with
/// an ellipsis for some of the `start` and `end` characters it shares
/// with the places it is told from, and all of what is its own. The end
/// is kept before the start: it is nearer the document. `None` for a
/// place that fits, and for one whose own part alone does not.
fn elided(place: &str, (start, end): (usize, usize)) -> Option<String> {
    let length = place.chars().count();
    let own = length - start - end;
    let room = PLACE_CHARS
        .checked_sub(own)
        .filter(|_| length > PLACE_CHARS)?;
    let after = end.min(room);
    let before = start.min(room - after);
    let kept = place
        .chars()
        .skip(start - before)
        .take(before + own + after);
    let (lead, trail) = (
        if before < start { "…" } else { "" },
        if after < end { "…" } else { "" },
    );
    Some(format!("{lead}{}{trail}", kept.collect::<String>()))
}

/// Give each candidate whose place is too long for a tab a short one to
/// draw, which leaves out what the places of its namesakes share and
/// keeps what differs. A short place that would have two documents draw
/// the same words is not used: those keep the whole place, which the row
/// cuts where it must.
fn shorten_places(candidates: &mut [Candidate<'_>]) {
    let mut places: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for candidate in &*candidates {
        if let Some(place) = candidate.shown.place() {
            places.entry(candidate.name).or_default().insert(place);
        }
    }
    let short = |candidate: &Candidate<'_>| {
        let shared = shared(places.get(candidate.name)?);
        elided(candidate.shown.place()?, shared)
    };
    let short: Vec<Option<String>> = candidates.iter().map(short).collect();
    for (candidate, short) in candidates.iter_mut().zip(short) {
        candidate.short = short;
    }
    loop {
        let drawn: Vec<String> = candidates.iter().map(Candidate::drawn).collect();
        let paths = candidates.iter().map(|candidate| candidate.path);
        let drawn: Vec<(&str, &Path)> = drawn.iter().map(String::as_str).zip(paths).collect();
        let mut restored = false;
        for (candidate, clashes) in candidates.iter_mut().zip(clashes(&drawn)) {
            restored |= clashes && candidate.short.take().is_some();
        }
        if !restored {
            break;
        }
    }
}

/// For each of `shown`, the words a document shows and its path, whether
/// another document shows the same words. One document open twice is one
/// document: its two tabs show the same.
fn clashes(shown: &[(&str, &Path)]) -> Vec<bool> {
    let mut shown_by: BTreeMap<&str, BTreeSet<&Path>> = BTreeMap::new();
    for (words, path) in shown {
        shown_by.entry(words).or_default().insert(path);
    }
    let clashes = |(words, _): &(&str, &Path)| {
        let documents = shown_by.get(words);
        documents.is_some_and(|documents| documents.len() > 1)
    };
    shown.iter().map(clashes).collect()
}

/// What to call each of `documents`, given as its name and its path, so
/// that no two documents are called the same: the name alone where no
/// other has it, else the name and the folder (`Notes · guides`), with
/// the folders above it while those are the same too (`Notes ·
/// a/guides`), and with the file where two namesakes share a folder
/// (`Notes · guides/monday.md`).
///
/// A place too long for a tab is drawn short in the tab row, with what
/// it shares with its namesakes' places left out (`Notes · …board-1998`),
/// while two documents still draw different words.
pub(crate) fn shown_names(documents: &[(&str, &Path)]) -> Vec<ShownName> {
    let beside_namesake = |name: &str, path: &Path| {
        documents
            .iter()
            .any(|(other, at)| *other == name && *at != path && at.parent() == path.parent())
    };
    let mut candidates: Vec<Candidate<'_>> = documents
        .iter()
        .map(|(name, path)| Candidate::new(name, path, beside_namesake(name, path)))
        .collect();
    loop {
        let mut lengthened = false;
        let shown: Vec<(&str, &Path)> = candidates
            .iter()
            .map(|candidate| (candidate.shown.as_str(), candidate.path))
            .collect();
        let clashing = clashes(&shown);
        for (candidate, clashes) in candidates.iter_mut().zip(clashing) {
            lengthened |= clashes && candidate.lengthen();
        }
        if !lengthened {
            break;
        }
    }
    shorten_places(&mut candidates);
    candidates.into_iter().map(Candidate::named).collect()
}
