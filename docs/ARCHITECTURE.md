# Architecture

Rust desktop application for reading markdown files, built on egui and a vendored egui_commonmark fork. `src/main.rs` only names the modules and starts the app; each module under `src/` owns one responsibility (`app/` the window and its panels, `theme/` the colors, `layout/` the reading column, `tab/`, `links/`, `explorer/`, `watcher/`, `system_fonts/`, `cli/`, `dialogs/`), with its tests in a sibling `tests.rs`.

## Core Components

- **MarkdownApp**: Main struct implementing `eframe::App`. Holds:
  - `tabs: Vec<Tab>` - list of open tabs
  - `active_tab: usize` - index of the currently active tab
  - `theme: ThemeId` - the reading theme (Paper, Sepia, Dusk, Night); its palette is every color the app paints
  - `reading_width: ReadingWidth` - width of the centered text column
  - `line_spacing: LineSpacing` - body line height (Compact 1.45, Comfortable 1.6, Airy 1.8)
  - `document_look: DocumentLook` - the document pane's fonts and block styling, rebuilt when the theme or the fonts change
  - `zoom_level: f32` - global zoom (0.5 to 3.0)
  - `show_outline: bool` - outline sidebar visibility; `outline_rule: OutlineRule` says whether the reader set it or it follows the room in the window
  - `heading_spy: HeadingSpy` - the heading the reader is in, which the outline marks
  - `show_explorer: bool` - toggle file explorer visibility (hidden on a fresh install)
  - `file_explorer: FileExplorer` - file explorer state
  - `watch_enabled: bool` - file watching state
  - `watcher` + `watcher_rx` - file watching via mpsc channel
  - `watched_paths: HashSet<PathBuf>` - unified watcher for all open tabs
  - `recent_files: Vec<RecentEntry>` - recently opened files for the welcome page
  - `welcome_show_all: bool` - whether the home page's recent list is expanded
  - `home_list: Option<HomeList>` - the home page's rows, built when the recent files change

- **Tab**: Per-tab state for a document. Each tab has:
  - `id: egui::Id` - unique identifier
  - `path: PathBuf` - file path
  - `content: String` - markdown text
  - `cache: CommonMarkCache` - **must persist across frames** (never recreate per-frame, only reset on file load)
  - `document_title: Option<String>` - first h1 used as sidebar title
  - `outline_headers: Vec<Header>` - parsed headers for outline
  - `scroll_offset`, `pending_scroll_offset`, `last_content_height`, `last_viewport_height` - scroll state
  - `local_links: Vec<String>` - cached local markdown links
  - `history_back`, `history_forward: Vec<PathBuf>` - per-tab navigation history

- **PersistedState**: Serializable struct for session persistence:
  - `theme: Option<ThemeId>` (`dark_mode: Option<bool>` is still read from sessions saved before the themes: dark maps to Dusk, light to Paper)
  - `reading_width: Option<f32>` (`full_width_content: Option<bool>` is still read from older sessions: `true` maps to fit-window)
  - `line_spacing: Option<LineSpacing>`
  - `zoom_level: Option<f32>`
  - `show_outline: Option<bool>` - `None` until the reader toggles the outline
  - `show_explorer: Option<bool>` - file explorer visibility
  - `explorer_root: Option<PathBuf>` - file explorer root directory
  - `expanded_dirs: Option<Vec<PathBuf>>` - expanded directories in explorer
  - `open_tabs: Option<Vec<PathBuf>>` - restore tabs on startup
  - `active_tab: Option<usize>` - restore active tab position
  - `recent_files: Option<Vec<RecentEntry>>` - recently opened files (home page); each carries the `title`, `excerpt` and `word_count` taken from the document when it was recorded, all absent in older sessions

- **FileExplorer**: Left sidebar showing markdown files in a directory tree:
  - `root: Option<PathBuf>` - root directory to display
  - `tree: Vec<FileTreeNode>` - hierarchical file tree
  - `expanded_dirs: HashSet<PathBuf>` - which directories are expanded

- **FileTreeNode**: Enum representing a node in the file explorer tree:
  - `File { path, name }` - a markdown file
  - `Directory { path, name, children }` - a directory containing markdown files

- **File Watching**: Uses `notify-debouncer-mini` with 200ms debounce. Watches all open tab paths. On change, reloads matching tabs. Auto-recovers up to 3 times on watcher failure.

- **Header Outline**: `parse_headers()` returns a `ParsedHeaders` struct containing `document_title` (first h1) and `outline_headers` (remaining headers). Rendered as a resizable right sidebar headed "CONTENTS". `HeadingSpy` (`src/app/outline/spy/`) finds the heading the reader is in from the scroll position and the heading positions the renderer recorded, only when the view or the document changed; the outline inks that entry and scrolls its list to keep it on screen. Until the reader toggles the outline, `OutlineRule::Room` shows it while `room::has_room` says the window holds it beside the text column (window width, less an open explorer and the outline's default width, at least the column and its two margins).

- **Sidebar parts**: `src/app/sidebar/` draws what the explorer and the outline share: the `surface` frame with no stroke, one hairline on the edge next to the page, the spaced-capitals header with an optional "⋯" menu button, and 28 px rows (current row in `text`, others `muted`, a rounded `raised` wash under the pointer, an optional chevron, elided text with the whole text on hover).

- **Link Navigation**: Uses egui_commonmark's link hook mechanism. Ctrl+Click opens links in new tabs, regular click navigates within the current tab.

- **Search (Ctrl+F)**: Current-document find bar with inline highlights. `SearchState` lives on `MarkdownApp`; per-tab `search_matches: Vec<SearchMatch>` cache match byte ranges. Highlights are painted by the vendored `egui_commonmark` renderer via a new `CommonMarkCache::set_search_ranges` API; the renderer splits `Event::Text` and (non-wrapped) `Event::Code` at range boundaries and applies a background color to the matching segments. Enter/Shift+Enter cycle matches with line-ratio scroll-into-view; Esc closes the bar and clears highlights on all tabs.

- **Markdown strong text**: md-viewer registers the renderer's `STRONG_FONT_FAMILY` (`MarkdownStrong`) during `setup_fonts`, then enables `CommonMarkViewer::use_strong_font_family(true)` so `**strong**` spans can use a real bold face while generic `egui_commonmark` consumers remain opt-in.

- **Keyboard document scrolling**: Plain document scroll keys are handled in `MarkdownApp::update` after mode-specific shortcuts are checked. `KeyboardScrollAction` maps Up/Down to fixed line steps and Page Up/Page Down to viewport-relative page steps through `keyboard_scroll_target`, which clamps against the active tab's `last_content_height`. The chosen target is assigned to the tab's `pending_scroll_offset`, so keyboard scrolling uses the same renderer-owned `ScrollArea` pipeline as outline and search jumps. Arrow keys are reserved for search-result navigation while the find bar is open, and document scrolling ignores Ctrl/Alt/Command-modified keypresses so it does not steal existing shortcuts.

- **Wide table scrolling**: Wide markdown / HTML tables are wrapped in a nested `egui::ScrollArea::horizontal()` so columns wider than the content area can still be reached. Plain vertical wheel stays with the outer document scroller; table horizontal movement uses the bottom scrollbar, native horizontal input, or `Shift+vertical-wheel` (routed via `forward_shift_wheel_to_horizontal_scroll` in `vendor/egui_commonmark/egui_commonmark/src/parsers/pulldown.rs`) so the cursor crossing a wide table during normal scrolling does not change its horizontal offset.

- **Themes**: `src/theme/` owns every color. `ThemeId::palette()` returns a `Palette` (page, surface, raised, text, heading, muted, rule, link, selection, accent, positive, warning, and the syntect theme for code). `theme::visuals(palette, overrides)` is the one function that turns a palette and the user's highlight/link overrides into `egui::Visuals`; `apply_theme` calls it when the theme or an override changes. Chrome code reads `self.theme.palette()` or `ui.visuals()` and never names a color. A test asserts body text and headings reach 7:1 contrast on every surface of every theme.

- **Reading column**: `layout::ReadingWidth` is the text column's width in px (280 to 4000, default 680; 4000 means fit the window). `resolve(pane_width)` gives the column actually used, which keeps a `READING_COLUMN_MARGIN` on each side. The renderer centers the column (`BlockStyle::center_content`) and tables, code blocks and images keep to it; a diagram wider than the column may leave it (see Document blocks). `src/app/content/column/` lays a resize strip in the margin beside each edge: hover shows a hairline and the resize cursor, a drag moves both edges about the pane center, a double click resets, and a pill shows the width while dragging. `Ctrl+Alt+←/→` step by 40 px. All of them end in `AppAction::SetReadingWidth` or `StepReadingWidth`.

- **Document look**: the document is laid out in its own fonts, the chrome around it in the system sans. `system_fonts::document_fonts(family, preset, size)` names them; `setup_fonts` registers the families they refer to (`MarkdownBody`, `MarkdownStrong`, `MarkdownHeading`, `MarkdownEmphasis`) for every preset. The default **Reader** preset leads those families with the bundled Source Serif 4 faces (`src/system_fonts/bundled/`, compiled in from `assets/fonts/`). `DocumentLook` (`src/app/content/look/`) holds the resulting `egui::Style` and the renderer's `BlockStyle` (heading and italic families, code block box, blockquote bar, task marker, short rules); it is built in `begin_frame` only when the theme or the font configuration changed, and `preview_pane` hands it to the renderer every frame without allocating. A `muted` meta line (`file-name · N min read`, computed in `Tab` when content loads) is painted through the renderer's `preamble` hook so it scrolls with the document.

- **Chrome**: `src/app/chrome/` holds what the bars, menus and popovers are built from: `QuietButton` (an icon or a word, `muted` at rest and `text` under the pointer), `MenuRow`, `segmented`, `hairline`, `bar_frame` and `popup_style`, all in fixed sizes so the controls that change the text do not move while they are used. `src/app/top_bar/` lays out the one bar (`render_top_bar`): explorer, back and forward on the left, the document's title centered between the button groups, and Aa, find, outline and ⋯ on the right. `src/app/appearance/` is the Aa popover (theme swatches, text size, width slider, line spacing, Fonts…) and `src/app/overflow_menu/` the ⋯ menu that carries every other command. All of them only pick an `AppAction`; the bar applies it after it is laid out. Icons are vector strokes from `icons::Icon`, painted in whatever ink the caller passes.

- **Document blocks**: `DocumentLook` also sets how the renderer spaces and draws blocks (all opt-in fields of the fork's `BlockStyle`). `rhythm` (`BlockRhythm`) replaces the blank line between blocks with one gap per pair of neighbors: a paragraph gap, more above a heading than below it, a block gap around code, tables, quotes and diagrams, a tight gap between the items of a tight list. `table` (`TableStyle`) drops the box, stripes and grid for hairlines between rows, sets the header strong, honors column alignment, widens a compact table to the column, and gives the table its own column separators, shown under the pointer. `list` (`ListStyle`) hangs small muted markers in a slot and draws a task item's box in place of its bullet. `inline_code_radius` puts inline code on a chip. `figure_max_width` is set per frame in `preview_pane` from `Column::figure_room()`: a mermaid diagram wider than the column is shown centered on it in up to the pane's width less its margins.

- **Global Allocator**: mimalloc for performance

## Key Libraries

| Crate | Purpose |
| ------- | --------- |
| eframe/egui 0.33 | GUI framework (glow backend for Wayland) |
| egui_commonmark 0.22 | Markdown rendering with syntax highlighting |
| notify 6.1 + notify-debouncer-mini 0.4 | File watching |
| rfd | Native file dialogs |
| clap | CLI argument parsing |
| regex | Header parsing for outline |

## Rendering Flow

```text
update() → check_file_changes() → reload affected tabs
         → request_repaint_after(100ms) if watching
         → Apply theme, zoom settings
         → TopBottomPanel (top bar) → render_top_bar()
           → appearance_popover() / overflow_menu(), while open
         → TopBottomPanel (error bar, if any)
         → TopBottomPanel (find bar, while open)
         → TopBottomPanel (tab strip, with two tabs or an edit) → render_tab_bar()
         → SidePanel::left (file explorer) → render_file_explorer()
           → render_tree_node() (recursive)
         → CentralPanel → render_tab_content()
           → SidePanel::right (outline, if show_outline && headers exist)
           → ScrollArea::show_viewport → CommonMarkViewer
           → check_link_hooks() → handle navigation
         → Drag-and-drop overlay
```

Uses `show_viewport` for optimized rendering - egui clips content outside the visible area.

## Custom Tab System

The tab system uses a simple `Vec<Tab>` with an `active_tab` index:

- The tab strip is hidden while one document is open and nothing is being edited (`shows_tab_bar`)
- Flat text tabs in a horizontal scroll area: the active one in `text` with a 2 px `accent` underline, the others `muted`
- Close button (×) shown while the pointer is over a tab
- A Preview | Edit segmented control at the right end, with the unsaved dot and the save outcome beside it
- Context menu with "Close" and "Close Others" options
- Middle-click to close tabs
- Ctrl+Click on links opens in a new tab
- Regular click navigates within the current tab
- Each tab maintains independent navigation history (Alt+Left/Right)
- Session restore opens previously open tabs and restores active tab
- Closing the last tab is allowed; `tabs` may be empty, which renders the **home page** (issue #28). `render_welcome` (shown from `render_tab_content` when no tab is active) draws, in the column a document would have, the wordmark, Open File / Open Folder as quiet text buttons, and the recently read documents: a serif title, a sans meta line (`folder · N min read · opened 2h ago`), a two-line excerpt, hairlines between rows, a wash and a click on the whole row. `record_recent` summarizes the document from its loaded tab (`DocumentSummary::of`: first h1, first paragraph of body text as plain text cut to 220 characters, word count) and `push_recent` stores it in `recent_files` (deduped, capped at `RECENT_FILES_CAP`, persisted). `HomeList::current` turns the entries into rows when they change, never while painting. A fresh launch with no file shows the page too — the old built-in sample document was removed.

## File Explorer

Left sidebar showing all markdown files in a hierarchical directory tree:

- Root directory determined by: CLI file → persisted state → first open tab → cwd
- Can be re-pointed at runtime via File → Open Folder… (`open_folder_dialog` → `FileExplorer::set_root`)
- Recursive scanning with 10 level depth limit
- Filters: .md, .markdown, .txt files only
- Skip hidden files (starting with .)
- Only shows directories containing markdown files
- Sorted: directories first, then files, alphabetically
- Hidden on a fresh install; toggle visibility with Ctrl+Shift+E
- Click a file to show it in the active tab; Ctrl+Click opens it in a new tab
- Click a directory's row to expand or collapse it
- The "⋯" menu in the header holds Expand All Folders, Collapse All Folders, Refresh and the sort order
- Session persistence for: visibility, root directory, expanded directories
