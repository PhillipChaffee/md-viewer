//! Per-tab document state: content, caches, scroll position, and navigation history.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui;
use egui_commonmark_extended::CommonMarkCache;

use crate::headers::{parse_headers, Outline};
use crate::links::{parse_local_links, resolve_local_link_path, split_link_fragment};
use crate::search::{find_matches, SearchMatch};
use crate::tab::scroll::ScrollCorrections;

#[cfg(kani)]
mod proofs;
pub(crate) mod scroll;
#[cfg(test)]
mod tests;

/// Words a reader gets through in a minute: the usual figure for prose on
/// a screen.
const WORDS_PER_MINUTE: usize = 230;

/// Minutes it takes to read `content`: at least one, rounded up.
pub(crate) fn reading_minutes(content: &str) -> usize {
    minutes_for_words(content.split_whitespace().count())
}

/// Minutes it takes to read `words` words: at least one, rounded up.
pub(crate) fn minutes_for_words(words: usize) -> usize {
    words.div_ceil(WORDS_PER_MINUTE).max(1)
}

/// The file name of `path` as shown to the user.
fn file_title(path: &Path) -> String {
    path.file_name().map_or_else(
        || "Unknown".to_string(),
        |n| n.to_string_lossy().to_string(),
    )
}

/// The line shown above a document: its file name and how long it takes
/// to read.
fn kicker_line(path: &Path, content: &str) -> String {
    format!(
        "{} · {} min read",
        file_title(path),
        reading_minutes(content)
    )
}

/// The text of the document at `path`, with any bytes that are not UTF-8
/// replaced.
fn read_document(path: &Path) -> io::Result<String> {
    Ok(String::from_utf8_lossy(&fs::read(path)?).into_owned())
}

/// Finish a step through history that went to the newest entry of `from`:
/// that entry leaves `from`, and `left`, the document the step came from,
/// becomes the newest entry of `to`.
fn move_history_entry<T>(from: &mut Vec<T>, to: &mut Vec<T>, left: T) {
    from.pop();
    to.push(left);
}

/// Record a visit to a new document from `left`: `left` is where Back now
/// goes, and there is no longer anywhere forward to go.
fn record_visit<T>(back: &mut Vec<T>, forward: &mut Vec<T>, left: T) {
    back.push(left);
    forward.clear();
}

/// How the last save of a tab's text went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    Saved,
    Failed,
}

/// What a save does about text another program wrote to the tab's file
/// and the reader has not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Theirs {
    /// Leave it there and write nothing: the reader is asked first.
    Ask,
    /// Write over it: the reader said to.
    Replace,
}

/// What a save that did not fail came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Saved {
    /// The file holds the tab's text.
    Written,
    /// Nothing was written, for it would replace what another program
    /// wrote.
    WouldOverwrite,
}

/// What another program did to a tab's file while the tab had unsaved
/// edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiskChange {
    /// The file holds other text now, or can no longer be read.
    Changed,
    /// The file is not there.
    Gone,
}

/// A change to a tab's file that its unsaved edits kept it from following,
/// until the reader reloads the file or saves over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Unfollowed {
    change: DiskChange,
    /// Whether `saved_content` is text another program wrote, which the
    /// reader has not taken: a file that goes and comes back with it is
    /// still not theirs to save over unasked.
    holds_theirs: bool,
    /// Whether the notice of it is up: it is until the reader keeps their
    /// own text.
    notice_shows: bool,
}

/// What a tab did when its file changed on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Followed {
    /// It shows the file's text now.
    Reloaded,
    /// It has unsaved edits and kept them.
    KeptEdits,
}

/// A way to show another document in a tab, in place of the one it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    /// A link in the document, as it is written there.
    Link(String),
    /// A file the reader picked.
    File(PathBuf),
    /// A step back through the tab's history.
    Back,
    /// A step forward through the tab's history.
    Forward,
}

/// A document to open, and the `#fragment` in it to land on.
pub(crate) type LinkTarget = (PathBuf, Option<String>);

/// Per-tab state for a document.
pub(crate) struct Tab {
    pub(crate) id: egui::Id,
    pub(crate) path: PathBuf,
    /// The file name of `path`, worked out when the path is set.
    title: String,
    pub(crate) content: String,
    pub(crate) cache: CommonMarkCache,
    pub(crate) document_title: Option<String>,
    /// The meta line above the document (`file-name · N min read`),
    /// worked out when the content is loaded.
    pub(crate) kicker: String,
    /// The headings of the document and which of them are folded.
    pub(crate) outline: Outline,
    pub(crate) scroll_offset: f32,
    pending_scroll_offset: Option<f32>,
    /// The jumps made from an estimate since the pane last took them.
    corrections: ScrollCorrections,
    pub(crate) last_content_height: f32,
    pub(crate) last_viewport_height: f32,
    pub(crate) content_lines: usize,
    local_links: Vec<String>,
    /// Cached base URI for markdown image/link resolution (e.g. `file:///path/to/dir/`).
    pub(crate) base_uri: String,
    /// The addresses of the pictures shown by documents this tab has left
    /// or reloaded. What egui keeps of them is given back when the tab is
    /// closed.
    pictures_left: BTreeSet<String>,
    pub(crate) history_back: Vec<PathBuf>,
    pub(crate) history_forward: Vec<PathBuf>,
    /// Cached matches for the current search query; empty when bar is closed or query is empty.
    pub(crate) search_matches: Vec<SearchMatch>,
    /// Monotonic counter bumped on every content load/reload. Used as the
    /// invalidation key for the renderer's per-document scroll cache so
    /// parsed events and `split_points` can survive across frames without
    /// re-hashing the entire content.
    pub(crate) content_version: u64,
    /// When true, the content area shows a `TextEdit` instead of `CommonMarkViewer`.
    pub(crate) is_editing: bool,
    /// The file's text as the tab last knew it: loaded, saved, or seen
    /// changed under unsaved edits. `is_dirty` compares the edit buffer
    /// with it.
    saved_content: String,
    /// The change to the file that the unsaved edits kept the tab from
    /// following. There is one only while there are unsaved edits: a tab
    /// without them reloads, and `follow_disk` decides which it is.
    unfollowed: Option<Unfollowed>,
    /// The outcome of the last save and when it was, for the edit
    /// toolbar to show until it has had its time.
    pub(crate) save_flash: Option<(SaveOutcome, Instant)>,
    /// Scroll ratio (0..=1) captured when entering edit mode, applied to the
    /// editor's `ScrollArea` on its first frame.
    pub(crate) pending_edit_scroll: Option<f32>,
    /// View-mode content height captured when entering edit mode; converts
    /// the edit scroll ratio back to an absolute offset when leaving edit.
    pre_edit_view_content_height: f32,
}

impl Tab {
    fn compute_base_uri(path: &Path) -> String {
        path.parent().map_or_else(
            || "file://".to_string(),
            |p| format!("file://{}/", p.display()),
        )
    }

    pub(crate) fn new(path: PathBuf) -> io::Result<Self> {
        // Canonicalize path for consistent comparison with watcher events
        let path = path.canonicalize().unwrap_or(path);
        let content = read_document(&path)?;
        let mut tab = Self {
            id: egui::Id::new(&path),
            base_uri: Self::compute_base_uri(&path),
            title: file_title(&path),
            path,
            saved_content: content.clone(),
            unfollowed: None,
            content,
            cache: CommonMarkCache::default(),
            pictures_left: BTreeSet::new(),
            document_title: None,
            kicker: String::new(),
            outline: Outline::default(),
            scroll_offset: 0.0,
            pending_scroll_offset: None,
            corrections: ScrollCorrections::default(),
            last_content_height: 0.0,
            last_viewport_height: 0.0,
            content_lines: 0,
            local_links: Vec::new(),
            history_back: Vec::new(),
            history_forward: Vec::new(),
            search_matches: Vec::new(),
            content_version: 1,
            is_editing: false,
            save_flash: None,
            pending_edit_scroll: None,
            pre_edit_view_content_height: 0.0,
        };
        tab.index_content(false);
        Ok(tab)
    }

    /// Switch between rendered view and raw-edit mode, preserving the scroll
    /// ratio across the switch (DSH-better-sidebar-style mode toggle).
    pub(crate) fn set_edit_mode(&mut self, to_edit: bool) {
        if self.is_editing == to_edit {
            return;
        }
        // Scroll mapping across the mode switch: capture the current ratio
        // and restore it in the other surface (ratio * that surface's
        // content height).
        let ratio = self.scroll_offset / self.last_content_height.max(1.0);
        if to_edit {
            self.pending_edit_scroll = Some(ratio);
            self.pre_edit_view_content_height = self.last_content_height;
        } else {
            // Leaving edit: first-frame estimate for the view; the
            // renderer's own correctives refine it after.
            self.pending_scroll_offset = Some(ratio * self.pre_edit_view_content_height.max(1.0));
        }
        self.is_editing = to_edit;
        self.cache.clear_scrollable();
        self.content_version = self.content_version.wrapping_add(1);
    }

    /// Whether the text differs from the file as the tab last knew it.
    /// Text typed back to what is on disk is not dirty.
    pub(crate) fn is_dirty(&self) -> bool {
        self.content != self.saved_content
    }

    /// Answer a change to the tab's file. Without unsaved edits the tab
    /// reloads. With them it keeps them, and notes what became of the file
    /// for the notice to show.
    pub(crate) fn follow_disk(&mut self) -> io::Result<Followed> {
        if self.is_dirty() {
            self.look_at_disk();
            Ok(Followed::KeptEdits)
        } else {
            self.reload().map(|()| Followed::Reloaded)
        }
    }

    /// Set the unsaved edits against the file as it is now. Other text in
    /// it is what the edits are measured against from here on, so taking
    /// them out by hand does not make the tab look saved. The notice goes
    /// up for other text and for another kind of change, not for a file
    /// that stays as it was last found.
    fn look_at_disk(&mut self) {
        let before = self.unfollowed;
        let held_theirs = before.is_some_and(|before| before.holds_theirs);
        let (change, holds_theirs, other_text) = match read_document(&self.path) {
            Ok(text) if text == self.saved_content && !held_theirs => {
                // The file is as the tab read or wrote it: one that was
                // gone is back, and a save replaces nothing.
                self.unfollowed = None;
                return;
            }
            Ok(text) if text == self.saved_content => (DiskChange::Changed, true, false),
            Ok(text) => {
                self.saved_content = text;
                (DiskChange::Changed, true, true)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                (DiskChange::Gone, held_theirs, false)
            }
            Err(_) => (DiskChange::Changed, held_theirs, false),
        };
        let notice_shows = match before {
            Some(before) if before.change == change && !other_text => before.notice_shows,
            _ => true,
        };
        // The other program may have written just what was typed here,
        // and then nothing is unsaved.
        self.unfollowed = self.is_dirty().then_some(Unfollowed {
            change,
            holds_theirs,
            notice_shows,
        });
    }

    /// The change to the file the reader has still to be told of.
    pub(crate) fn disk_notice(&self) -> Option<DiskChange> {
        let unfollowed = self.unfollowed?;
        unfollowed.notice_shows.then_some(unfollowed.change)
    }

    /// Take the notice down and keep the edits. A save still has to ask.
    pub(crate) const fn keep_mine(&mut self) {
        if let Some(unfollowed) = &mut self.unfollowed {
            unfollowed.notice_shows = false;
        }
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// What the tab is called: the document's first heading, else its file
    /// name. Both are worked out when the text is loaded or edited.
    pub(crate) fn name(&self) -> &str {
        self.document_title.as_deref().unwrap_or(&self.title)
    }

    pub(crate) fn reload(&mut self) -> io::Result<()> {
        let content = read_document(&self.path)?;
        self.apply_loaded_content(self.path.clone(), content);
        Ok(())
    }

    /// Write the unsaved edits to the tab's file, and note how that went
    /// for the edit toolbar to show. With nothing unsaved it does nothing.
    ///
    /// Under [`Theirs::Ask`] nothing is written over text another program
    /// wrote and the reader has not taken. The file is read to find out,
    /// so a change no watcher reported counts too. A file that is gone has
    /// no text to replace.
    pub(crate) fn save(&mut self, theirs: Theirs) -> io::Result<Saved> {
        if theirs == Theirs::Ask && self.is_dirty() {
            self.look_at_disk();
            let unfollowed = self.unfollowed.map(|unfollowed| unfollowed.change);
            if unfollowed == Some(DiskChange::Changed) {
                return Ok(Saved::WouldOverwrite);
            }
        }
        // Looking may have found the file holding just what was typed.
        if !self.is_dirty() {
            return Ok(Saved::Written);
        }
        let result = self.save_to_disk();
        let outcome = if result.is_ok() {
            SaveOutcome::Saved
        } else {
            SaveOutcome::Failed
        };
        self.save_flash = Some((outcome, Instant::now()));
        result.map(|()| Saved::Written)
    }

    /// Atomic save: write a temp file next to the target, then rename over
    /// it (mirrors the dsh-better-sidebar editor's tmp+rename save path).
    /// On success `saved_content` is refreshed so the buffer is no longer
    /// dirty.
    fn save_to_disk(&mut self) -> io::Result<()> {
        let tmp = PathBuf::from(format!(
            "{}.mdv-tmp-{}",
            self.path.display(),
            std::process::id()
        ));
        fs::write(&tmp, &self.content)?;
        if let Err(error) = fs::rename(&tmp, &self.path) {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
        self.saved_content = self.content.clone();
        self.unfollowed = None;
        Ok(())
    }

    /// Rebuild `search_matches` for `query`. Empty query clears matches.
    pub(crate) fn rebuild_search(&mut self, query: &str) {
        self.search_matches = find_matches(&self.content, query);
    }

    fn load_file(&mut self, path: &Path) -> io::Result<()> {
        let content = read_document(path)?;
        self.apply_loaded_content(path.to_path_buf(), content);
        self.scroll_offset = 0.0;
        self.pending_scroll_offset = None;
        Ok(())
    }

    /// The addresses of the pictures this tab has shown, in its document
    /// and in the documents it showed before.
    pub(crate) fn pictures(&self) -> impl Iterator<Item = &str> {
        let left = self.pictures_left.iter().map(String::as_str);
        left.chain(self.cache.pictures_shown())
    }

    fn apply_loaded_content(&mut self, path: PathBuf, content: String) {
        let shown = self.cache.pictures_shown().map(str::to_owned);
        self.pictures_left.extend(shown);
        self.content = content;
        self.saved_content = self.content.clone();
        self.unfollowed = None;
        self.path = path;
        self.title = file_title(&self.path);
        self.id = egui::Id::new(&self.path);
        self.cache = CommonMarkCache::default();
        self.content_version = self.content_version.wrapping_add(1);
        self.base_uri = Self::compute_base_uri(&self.path);
        self.index_content(false);

        // Stale byte ranges; caller rebuilds if search bar is open.
        self.search_matches.clear();
    }

    /// Bring what is worked out from the text back in line with it after
    /// the editor changed it: the contents, the meta line and the links.
    /// Called once per edit, never per frame.
    pub(crate) fn content_edited(&mut self) {
        self.index_content(true);
        if !self.is_dirty() {
            // The text is the file's again: there is nothing left to
            // reload or to save over.
            self.unfollowed = None;
        }
    }

    /// Work out everything that is read off `content`: its line count,
    /// meta line, title, headings and local links. `keep_folds` is for
    /// the same document after an edit; another document starts unfolded.
    fn index_content(&mut self, keep_folds: bool) {
        self.content_lines = self.content.lines().count();
        self.kicker = kicker_line(&self.path, &self.content);

        let parsed = parse_headers(&self.content);
        self.document_title = parsed.document_title;
        self.outline
            .replace_headers(parsed.outline_headers, keep_folds);

        self.local_links = parse_local_links(&self.content, &self.path);
        for link in &self.local_links {
            self.cache.add_link_hook(link);
        }
    }

    /// Go where `route` leads. Returns whether the tab went anywhere, a
    /// heading of its own document included.
    pub(crate) fn take(&mut self, route: &Route) -> io::Result<bool> {
        match route {
            Route::Link(link) => self.navigate_to_link(link),
            // A file that is not there is an error here, where the reader
            // picked it by name.
            Route::File(path) => self.navigate_to_path(&path.canonicalize()?),
            Route::Back => self.navigate_back(),
            Route::Forward => self.navigate_forward(),
        }
    }

    /// Whether `route` leads to another document, which would take the
    /// place of this one and of any edits to it that are not saved. A
    /// heading of this document does not, nor does a way that leads
    /// nowhere.
    pub(crate) fn leads_away(&self, route: &Route) -> bool {
        match route {
            Route::Link(link) if link.starts_with('#') => false,
            Route::Link(link) => {
                let (destination, _) = split_link_fragment(link);
                let target = self.resolve_link(destination);
                target.is_ok_and(|target| target.is_some_and(|file| self.is_another_file(&file)))
            }
            Route::File(path) => {
                let target = path.canonicalize();
                target.is_ok_and(|file| self.is_another_file(&file))
            }
            Route::Back => self.can_go_back(),
            Route::Forward => self.can_go_forward(),
        }
    }

    /// Whether `target` is a file, and not the one this tab shows.
    fn is_another_file(&self, target: &Path) -> bool {
        target != self.path && target.is_file()
    }

    pub(crate) fn navigate_to_link(&mut self, link: &str) -> io::Result<bool> {
        // A same-document fragment scrolls this tab — no path resolution, no
        // history entry, the same semantics as clicking the heading in the
        // outline. Unmatched fragments do nothing rather than error: the
        // document opened fine, the anchor simply does not exist.
        if let Some(fragment) = link.strip_prefix('#') {
            return Ok(self.go_to_fragment(fragment));
        }

        // Share resolution and error handling with the ctrl+click path. These
        // used to differ: #78 taught percent-decoding and `file://` handling
        // to `resolve_local_link_path`, but this function still did a raw
        // `join`, so a link with an encoded space opened in a new tab on
        // ctrl+click and silently did nothing on a plain click.
        let (destination, fragment) = split_link_fragment(link);
        let Some(target_path) = self.resolve_link(destination)? else {
            return Ok(false);
        };

        let visited = self.visit(&target_path)?;
        // `guide.md#section` opens the document *and* lands on the heading —
        // the #141 decision that an anchor selects a place in the document,
        // not just the document. Applied after the load, which resets the
        // scroll and would otherwise clear it.
        if let (true, Some(fragment)) = (visited, fragment) {
            self.go_to_fragment(fragment);
        }
        Ok(visited)
    }

    /// Replace this tab's document and record a browser-like history entry.
    /// Returns whether navigation happened; failures leave history untouched.
    pub(crate) fn navigate_to_path(&mut self, target_path: &Path) -> io::Result<bool> {
        let Ok(target_path) = target_path.canonicalize() else {
            return Ok(false);
        };
        self.visit(&target_path)
    }

    /// Show the document at `target` in place of this one, and make this
    /// one where Back goes. Returns whether that happened: it does not for
    /// the document already shown, nor for anything but a file.
    fn visit(&mut self, target: &Path) -> io::Result<bool> {
        if !self.is_another_file(target) {
            return Ok(false);
        }
        let previous_path = self.path.clone();
        self.load_file(target)?;
        record_visit(
            &mut self.history_back,
            &mut self.history_forward,
            previous_path,
        );
        Ok(true)
    }

    pub(crate) fn check_link_hooks(&self) -> Option<String> {
        self.local_links
            .iter()
            .find(|link| self.cache.get_link_hook(link) == Some(true))
            .cloned()
    }

    pub(crate) const fn can_go_back(&self) -> bool {
        !self.history_back.is_empty()
    }

    pub(crate) const fn can_go_forward(&self) -> bool {
        !self.history_forward.is_empty()
    }

    pub(crate) fn navigate_back(&mut self) -> io::Result<bool> {
        let Some(prev_path) = self.history_back.last().cloned() else {
            return Ok(false);
        };
        let current_path = self.path.clone();
        self.load_file(&prev_path)?;
        move_history_entry(
            &mut self.history_back,
            &mut self.history_forward,
            current_path,
        );
        Ok(true)
    }

    pub(crate) fn navigate_forward(&mut self) -> io::Result<bool> {
        let Some(next_path) = self.history_forward.last().cloned() else {
            return Ok(false);
        };
        let current_path = self.path.clone();
        self.load_file(&next_path)?;
        move_history_entry(
            &mut self.history_forward,
            &mut self.history_back,
            current_path,
        );
        Ok(true)
    }

    /// The document `link` names and the `#fragment` after it, for opening
    /// in another tab. `None` for a link that names no local document.
    pub(crate) fn link_target(&self, link: &str) -> io::Result<Option<LinkTarget>> {
        let (destination, fragment) = split_link_fragment(link);
        let path = self.resolve_link(destination)?;
        Ok(path.map(|path| (path, fragment.map(str::to_owned))))
    }

    pub(crate) fn resolve_link(&self, link: &str) -> io::Result<Option<PathBuf>> {
        let Some(current_dir) = self.path.parent() else {
            return Ok(None);
        };
        // `?` rather than a discard: a destination that cannot be decoded is
        // reported through the same error bar as a missing file (#181), instead
        // of rendering as an internal link that does nothing when clicked.
        let Some(target_path) = resolve_local_link_path(link, current_dir)? else {
            return Ok(None);
        };
        target_path.canonicalize().map(Some)
    }
}
