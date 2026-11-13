//! Per-tab document state: content, caches, scroll position, and navigation history.

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

/// A document to open, and the `#fragment` in it to land on.
pub(crate) type LinkTarget = (PathBuf, Option<String>);

/// Per-tab state for a document.
pub(crate) struct Tab {
    pub(crate) id: egui::Id,
    pub(crate) path: PathBuf,
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
    /// Snapshot of the on-disk content at last load/save. The edit buffer is
    /// dirty when `content != saved_content`.
    pub(crate) saved_content: String,
    /// "Saved ✓" / "Save failed" flash trigger for the edit toolbar
    /// (true = success, false = failure).
    pub(crate) save_flash: Option<(bool, Instant)>,
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
            path,
            saved_content: content.clone(),
            content,
            cache: CommonMarkCache::default(),
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

    pub(crate) fn title(&self) -> String {
        file_title(&self.path)
    }

    pub(crate) fn reload(&mut self) -> io::Result<()> {
        let content = read_document(&self.path)?;
        self.apply_loaded_content(self.path.clone(), content);
        Ok(())
    }

    /// Atomic save: write a temp file next to the target, then rename over
    /// it (mirrors the dsh-better-sidebar editor's tmp+rename save path).
    /// On success `saved_content` is refreshed so the buffer is no longer
    /// dirty.
    pub(crate) fn save_to_disk(&mut self) -> io::Result<()> {
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

    fn apply_loaded_content(&mut self, path: PathBuf, content: String) {
        self.content = content;
        self.saved_content = self.content.clone();
        self.path = path;
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
        if target == self.path || !target.is_file() {
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
