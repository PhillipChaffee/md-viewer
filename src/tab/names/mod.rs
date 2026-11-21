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

/// What one tab is called: its document's name and, where another open
/// document has that name too, the place that tells the two apart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShownName {
    /// The name, then the separator and the place where there is one.
    whole: String,
    /// How many bytes of `whole` are the name.
    name_len: usize,
}

impl ShownName {
    /// A name that needs no place: no other tab has it.
    pub(crate) fn plain(name: &str) -> Self {
        Self {
            whole: name.to_owned(),
            name_len: name.len(),
        }
    }

    /// A name with the place its document is kept in.
    fn placed(name: &str, place: &Path) -> Self {
        Self {
            whole: format!("{name}{SEPARATOR}{}", place.display()),
            name_len: name.len(),
        }
    }

    /// The whole of it, as a screen reader says it: `Notes · guides`.
    pub(crate) fn as_str(&self) -> &str {
        &self.whole
    }

    /// The whole of it, to keep where only the whole is read.
    pub(crate) fn into_string(self) -> String {
        self.whole
    }

    /// The name, and what follows it: the separator and the place, or
    /// nothing. A row short of room cuts the name and keeps the rest.
    pub(crate) fn parts(&self) -> (&str, &str) {
        let parts = self.whole.split_at_checked(self.name_len);
        parts.unwrap_or((&self.whole, ""))
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
}

/// For each candidate, whether another document shows the same words.
/// One document open twice is one document: its two tabs show the same.
fn clashes(candidates: &[Candidate<'_>]) -> Vec<bool> {
    let mut shown_by: BTreeMap<&str, BTreeSet<&Path>> = BTreeMap::new();
    for candidate in candidates {
        let documents = shown_by.entry(candidate.shown.as_str()).or_default();
        documents.insert(candidate.path);
    }
    let alone = |candidate: &Candidate<'_>| {
        let documents = shown_by.get(candidate.shown.as_str());
        documents.is_none_or(|documents| documents.len() < 2)
    };
    candidates
        .iter()
        .map(|candidate| !alone(candidate))
        .collect()
}

/// What to call each of `documents`, given as its name and its path, so
/// that no two documents are called the same: the name alone where no
/// other has it, else the name and the folder (`Notes · guides`), with
/// the folders above it while those are the same too (`Notes ·
/// a/guides`), and with the file where two namesakes share a folder
/// (`Notes · guides/monday.md`).
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
        let clashing = clashes(&candidates);
        for (candidate, clashes) in candidates.iter_mut().zip(clashing) {
            lengthened |= clashes && candidate.lengthen();
        }
        if !lengthened {
            break;
        }
    }
    candidates
        .into_iter()
        .map(|candidate| candidate.shown)
        .collect()
}
