#!/usr/bin/env python3
"""Regenerate src/ from the base commit as a behavior-preserving module split.
Usage: split.py   (reads vis.json and extra.json next to it)"""
import collections, json, os, pathlib, re, shutil, subprocess
from lex import string_start_lines, chunks, effective

REPO = '/home/user/md-viewer'
BASE = 'c0bc88eb16437c7dd1e7134c11ec7aad58f99a07'
HERE = pathlib.Path(__file__).parent

def show(p):
    L = subprocess.run(['git', '-C', REPO, 'show', f'{BASE}:{p}'], capture_output=True, text=True, check=True).stdout.split('\n')
    if L[-1] == '':
        L.pop()
    return L

class Source:
    def __init__(self, path, method_impls=()):
        self.L = show(path)
        self.instr = string_start_lines(self.L)
        self.top, _ = chunks(self.L, 0, len(self.L), 0, self.instr)
        self.used = set()
        self.sub = {}
        for kind, name, a, b in self.top:
            if name in method_impls or name == 'tests':
                s = next(k for k in range(a, b) if self.L[k].startswith(('impl', 'mod')))
                self.sub[name], _ = chunks(self.L, s + 1, b - 1, 4, self.instr)
                self.used.update(range(a, s + 1)); self.used.add(b - 1)
    def take(self, table, names):
        out = []
        for want in names:
            hits = [c for c in table if c[1] == want]
            assert hits, want
            for _, _, a, b in hits:
                assert not (set(range(a, b)) & self.used), want
                self.used.update(range(a, b))
                out.append((a, b))
        out.sort()
        return out
    def lines(self, a, b):
        """1-based inclusive range."""
        r = range(a - 1, b)
        assert not (set(r) & self.used), (a, b)
        self.used.update(r)
        return self.L[a - 1:b]
    def text(self, spans, dedent=False):
        out = []
        for a, b in spans:
            for i in range(a, b):
                l = self.L[i]
                if dedent and i not in self.instr and l.startswith('    '):
                    l = l[4:]
                out.append(l)
        return out

M = Source('src/main.rs', ('impl MarkdownApp', 'impl eframe::App for MarkdownApp'))
M.lines(5121, 5121)
F = Source('src/system_fonts.rs')
APP = 'impl MarkdownApp'

def top(*n): return ('top', M, n)
def meth(*n): return ('meth', M, n)
def tests(*n): return ('tests', M, n)
def ftop(*n): return ('top', F, n)
def ftests(*n): return ('tests', F, n)
def raw(*l): return ('raw', None, l)
def rng(a, b): return ('rng', M, (a, b))

SPEC = collections.OrderedDict()
DOC = {}
def f(path, doc, *parts):
    SPEC[path] = parts; DOC[path] = doc

f('cli/mod.rs', 'Command-line arguments, terminal detach, and native window startup.',
  top('Args', 'should_detach', 'launched_from_terminal', 'child_args_with_no_detach', 'spawn_detached_child',
      'run_native_once', 'fall_back_to_x11_if_wayland_socket_is_dead', 'wayland_display_socket_path'))
f('cli/tests.rs', None,
  tests('wayland_socket_path_mirrors_libwayland_lookup', 'default_terminal_launch_detaches', 'non_terminal_launch_does_not_detach',
        'foreground_flag_disables_detach', 'hidden_no_detach_marker_disables_detach', 'child_args_preserve_user_args_and_append_marker',
        'child_args_insert_marker_before_double_dash', 'hidden_no_detach_marker_is_not_in_help', 'version_flag_reports_crate_version'))
f('layout/mod.rs', 'Window, sidebar, zoom, and keyboard-scroll geometry shared across the UI.',
  top('CONTENT_OPTIMAL_WIDTH', 'EXPLORER_DEFAULT_WIDTH', 'OUTLINE_DEFAULT_WIDTH', 'PANEL_SEPARATORS', 'OPTIMAL_WINDOW_HEIGHT',
      'SIDEBAR_MIN_WIDTH', 'SIDEBAR_RESIZE_GRAB_RADIUS', 'EXPLORER_RIGHT_RESIZE_GUTTER', 'CONTENT_RIGHT_RESIZE_GUTTER',
      'KEYBOARD_LINE_SCROLL_STEP', 'KEYBOARD_PAGE_SCROLL_RATIO', 'KeyboardScrollAction', 'keyboard_scroll_target',
      'content_default_width', 'content_width_limit', 'ZOOM_MIN', 'ZOOM_MAX', 'ZOOM_STEP', 'zoomed'))
f('layout/tests.rs', None,
  tests('document_zoom_steps_are_multiplicative_and_clamped', 'zoom_bounds_match_the_persisted_value_clamp',
        'capped_content_width_uses_optimal_width', 'full_width_content_uses_available_width',
        'capped_content_applies_the_same_limit_to_tables', 'full_width_tables_use_the_available_content_width',
        'keyboard_scroll_target_moves_by_line_step', 'keyboard_scroll_target_moves_by_page_step',
        'keyboard_scroll_target_clamps_to_document_bounds', 'keyboard_scroll_target_handles_short_documents'))
f('icons.rs', 'Hand-painted icons and affordances for the sidebars.',
  top('paint_sidebar_resize_affordance', 'paint_file_icon', 'paint_folder_icon'))
f('dialogs/mod.rs', 'Native file and folder pickers, with a Tk fallback when no portal is available.',
  top('portal_file_chooser_available', 'portal_introspection_has_file_chooser', 'path_from_picker_output',
      'pick_file_with_tkinter', 'pick_file', 'pick_folder_with_tkinter', 'pick_folder'))
f('dialogs/tests.rs', None,
  tests('portal_introspection_detects_file_chooser_interface', 'empty_folder_picker_output_means_cancelled',
        'folder_picker_output_preserves_path_bytes'))
f('recent/mod.rs', 'Recently opened files shown on the welcome page.',
  top('RECENT_FILES_CAP', 'RECENT_SHOWN', 'RecentEntry', 'now_epoch_secs', 'push_recent', 'format_relative_time'))
f('recent/tests.rs', None,
  tests('push_recent_dedupes_and_moves_to_front', 'push_recent_caps_length_keeping_newest', 'relative_time_buckets'))
f('persist/mod.rs', 'Session state persisted between runs.',
  top('APP_KEY', 'PersistedState', 'restored_sidebar_width'))
f('persist/tests.rs', None,
  tests('MemoryStorage', 'impl eframe::Storage for MemoryStorage', 'font_preset_survives_a_persistence_roundtrip',
        'persisted_sidebar_widths_are_sanitized'))
f('headers/mod.rs', 'Heading extraction for the document title and the outline sidebar.',
  top('Header', 'ParsedHeaders', 'parse_headers', 'header_is_hidden', 'header_has_children', 'any_header_has_children'))
f('headers/tests.rs', None,
  tests('heading_parser_keeps_raw_shortcode_and_source_identity', 'unknown_shortcode_heading_stays_raw',
        'heading_parser_uses_commonmark_for_formatting_links_and_setext'))
f('search/mod.rs', 'Find-in-document state and match scanning.',
  top('SearchMatch', 'SearchBarOutcome', 'SearchState', 'find_matches', 'take_search_correction_request'))
f('search/tests.rs', None,
  tests('search_correction_request_is_consumed_even_without_a_rendered_match', 'find_matches_empty_query_returns_none',
        'find_matches_empty_content_returns_none', 'find_matches_single_ascii', 'find_matches_case_insensitive_ascii',
        'find_matches_multiple_per_line', 'find_matches_line_numbers_one_based', 'find_matches_preserves_byte_offsets_with_utf8',
        'find_matches_skips_cross_newline_matches', 'find_matches_overlapping_uses_match_indices_semantics',
        'find_matches_skips_image_alt_text', 'find_matches_skips_image_url', 'find_matches_skips_link_url_keeps_link_text',
        'find_matches_multiple_alt_text_images', 'shortcode_search_range_uses_raw_source_bytes',
        'rendered_emoji_does_not_match_shortcode_only_source', 'literal_unicode_emoji_still_matches_raw_source'))
f('explorer/mod.rs', 'File explorer tree model: lazy directory scanning, sorting, and expansion state.',
  top('ExplorerAction', 'SortOrder', 'impl SortOrder', 'FileTreeNode', 'impl FileTreeNode', 'ExplorerScanResult',
      'FileExplorer', 'impl FileExplorer'))
f('explorer/tests.rs', None,
  tests('asynchronous_explorer_result_restores_expanded_children', 'local_root_scan_runs_asynchronously_and_restores_expansion',
        'targeted_explorer_refresh_preserves_unrelated_subtrees'))
f('tab/mod.rs', 'Per-tab document state: content, caches, scroll position, and navigation history.',
  top('Tab', 'impl Tab'))
f('tab/tests.rs', None,
  tests('tab_history_changes_only_after_successful_loads', 'unreadable_new_tab_returns_an_error',
        'tab_path_navigation_tracks_back_and_forward_history'))
f('links/mod.rs', 'Local link classification, resolution, and fragment navigation.',
  top('split_link_fragment', 'slugify_heading', 'FragmentTarget', 'find_fragment_target', 'apply_fragment_navigation',
      'handle_local_link_click', 'parse_local_links', 'register_bare_markdown_paths', 'register_existing_markdown_path',
      'DestinationShape', 'classify_destination', 'has_markdown_extension', 'is_local_markdown_link', 'explicit_uri_scheme',
      'resolve_local_link_path'))
f('links/tests.rs', None,
  tests('local_destination_resolution_is_characterized_for_every_uri_shape', 'nothing_the_gate_accepts_can_resolve_to_silence',
        'shared_resolver_keeps_absolute_paths_and_literal_percent_names', 'plain_click_and_ctrl_click_resolve_the_same_link_identically',
        'slugify_heading_matches_github_anchor_conventions', 'find_fragment_target_prefers_document_title_then_outline_order',
        'same_document_fragment_scrolls_without_touching_history', 'cross_document_fragment_opens_and_lands_on_heading',
        'bare_existing_markdown_path_is_registered_as_local_link', 'inline_code_markdown_path_can_touch_chinese_prose_and_punctuation',
        'nonexistent_bare_markdown_path_is_not_registered', 'missing_explicit_link_surfaces_errors_without_changing_tab_or_history',
        'fenced_and_indented_code_never_register_bare_markdown_paths', 'only_file_uris_are_classified_as_local_links',
        'windows_drive_prefixes_are_not_treated_as_uri_schemes', 'explicit_non_file_schemes_are_not_registered_as_link_hooks',
        'absolute_path_and_file_uri_are_registered_as_local_links', 'commonmark_reference_with_encoded_path_is_registered'))
f('watcher/mod.rs', 'File-watcher backends and retry policy.',
  top('MAX_WATCHER_RETRIES', 'FLASH_DURATION_MS', 'next_watcher_retry', 'is_gvfs_path', 'FileWatcher', 'impl FileWatcher'))
f('watcher/tests.rs', None, tests('watcher_retry_sequence_stops_at_configured_limit'))

f('app/mod.rs', 'The application state and its construction.',
  top('MarkdownApp'), raw('', APP + ' {'), meth('new', 'window_title', 'is_markdown_file'), raw('}'))
def app(path, doc, *names):
    f(path, doc, raw(APP + ' {'), meth(*names), raw('}'))
app('app/tabs.rs', 'Opening, closing, switching, and navigating tabs.',
    'open_file_dialog', 'open_folder_dialog', 'open_in_new_tab', 'open_in_new_tab_with_fragment', 'navigate_active_tab_to',
    'navigate_active_history', 'record_recent', 'close_tab', 'close_active_tab', 'next_tab', 'prev_tab', 'focus_tab',
    'get_open_tab_paths', 'refresh_open_tab_paths')
app('app/watching.rs', 'Live reload: watcher lifecycle, change polling, and tab reloads.',
    'start_watching', 'start_watching_attempt', 'stop_watching', 'update_watched_paths', 'reconcile_explorer_watches',
    'check_file_changes', 'reload_changed_tabs')
app('app/search_bar.rs', 'The find bar and match navigation.',
    'render_search_bar', 'jump_match', 'scroll_to_active_match', 'maybe_rebuild_search', 'close_search')
app('app/tab_bar.rs', 'The tab strip.', 'render_tab_bar')
app('app/outline.rs', 'The outline sidebar.', 'render_outline')
app('app/welcome.rs', 'The welcome page shown when no tab is open.', 'render_welcome')
app('app/content.rs', 'The document pane: rendered markdown, edit mode, and link handling.', 'render_tab_content')
app('app/explorer_panel.rs', 'The file explorer sidebar.', 'render_file_explorer', 'get_flash_intensity', 'render_tree_node')
app('app/settings.rs', 'The color and font settings dialogs.', 'render_color_settings', 'render_font_settings')
f('app/lightbox.rs', 'The zoomable lightbox overlay for diagrams and images.',
  top('LightboxTexture', 'impl LightboxTexture', 'LightboxState'), raw('', APP + ' {'), meth('render_lightbox'), raw('}'))

EFR = 'impl eframe::App for MarkdownApp'
U = '        '
f('app/frame.rs', 'The eframe integration: input hook, persistence, and the per-frame update sequence.',
  raw(EFR + ' {'),
  ('sub', M, (EFR, ('raw_input_hook', 'persist_egui_memory', 'save'))),
  raw('', '    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {',
      U + 'self.begin_frame(ctx);',
      U + 'let (go_back, go_forward) = self.handle_keyboard_shortcuts(ctx);',
      U + 'self.handle_dropped_files(ctx);', ''),
  rng(5567, 5569),
  raw(U + 'self.render_menu_bar(ctx, go_back, go_forward);',
      U + 'self.render_top_bars(ctx);',
      U + 'self.render_panels(ctx, ctrl_held);',
      U + 'self.render_overlays(ctx);', ''),
  rng(6102, 6105),
  raw('}', '', APP + ' {',
      '    /// Per-frame housekeeping that runs before any input or UI: file',
      '    /// changes, async polls, and theme, font, zoom, and title application.',
      '    fn begin_frame(&mut self, ctx: &egui::Context) {'),
  rng(5122, 5271),
  raw('    }', '',
      '    /// The error bar, the find bar, and the tab strip.',
      '    fn render_top_bars(&mut self, ctx: &egui::Context) {'),
  rng(5949, 5993),
  raw('    }', '',
      '    /// The explorer, the outline, and the document pane, with the',
      '    /// navigation each of them requested.',
      '    fn render_panels(&mut self, ctx: &egui::Context, ctrl_held: bool) {'),
  rng(5995, 6044),
  raw('    }', '',
      '    /// The lightbox, the settings dialogs, and the drag-and-drop overlay.',
      '    fn render_overlays(&mut self, ctx: &egui::Context) {'),
  rng(6046, 6100),
  raw('    }', '}'))
f('app/shortcuts.rs', 'Keyboard shortcuts and dropped files.',
  raw(APP + ' {',
      '    /// Read this frame\'s keyboard shortcuts and act on them. Returns the',
      '    /// history back/forward requests, which the menu bar acts on again.',
      '    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) -> (bool, bool) {'),
  rng(5273, 5546),
  raw(U + '(go_back, go_forward)', '    }', '',
      '    /// Track drag hover and open dropped markdown files.',
      '    fn handle_dropped_files(&mut self, ctx: &egui::Context) {'),
  rng(5547, 5565),
  raw('    }', '}'))
f('app/menu_bar.rs', 'The menu bar, its navigation buttons, and the status readout.',
  raw(APP + ' {',
      '    /// Render the menu bar, then act on history navigation requested by',
      '    /// its buttons or carried in from the keyboard shortcuts.',
      '    fn render_menu_bar(', U + '&mut self,', U + 'ctx: &egui::Context,', U + 'mut go_back: bool,', U + 'mut go_forward: bool,', '    ) {'),
  rng(5570, 5947),
  raw('    }', '}'))

f('system_fonts/mod.rs', 'Font presets, text sizes, and system font installation for egui.',
  ftop('FontPreset', 'impl FontPreset', 'TextSizeClass', 'impl TextSizeClass', 'GITHUB_BODY_FAMILIES', 'GITHUB_MONO_FAMILIES',
       'VSCODE_BODY_FAMILIES', 'VSCODE_MONO_FAMILIES', 'apply_text_size_class', 'setup_fonts', 'scan_pickable_font_families',
       'pickable_family_names'))
f('system_fonts/tests.rs', None,
  ftests('pickable_family_list_is_deduplicated_and_body_capable', 'installed_fonts_cover_reported_scripts',
         'preset_family_lists_follow_the_emulated_css_stacks', 'preset_mono_lists_lead_with_linux_generic_monospace_resolutions',
         'github_preset_resolves_its_body_stack_when_fonts_exist'))
f('system_fonts/install/mod.rs', 'Regular, bold, and monospace face selection with script fallbacks.',
  ftop('ScriptFallback', 'SCRIPT_FALLBACKS', 'GENERIC_FALLBACKS', 'SelectedFont', 'impl SelectedFont', 'InstalledFont',
       'font_has_glyphs', 'source_description', 'select_from_families', 'fallback_family_ids', 'installed_fonts_cover',
       'install_regular_font', 'install_regular_fonts', 'push_unique', 'build_strong_family', 'install_strong_font_family',
       'install_preset_mono_font'))
f('system_fonts/install/tests.rs', None,
  ftests('reported_scripts_use_full_coverage_samples', 'default_latin_precedes_script_bold_without_primary_system_sans',
         'primary_regular_and_bold_stay_together', 'unknown_preferred_family_falls_back_to_auto_detect',
         'known_preferred_family_becomes_primary'))
f('system_fonts/emoji/mod.rs', 'Color and monochrome emoji fallback faces.',
  ftop('EMOJI_FONT_PATH', 'install_emoji_fonts'))
f('system_fonts/emoji/tests.rs', None,
  ftests('emoji_fallback_face_resolves_for_doc_emoji', 'color_emoji_face_never_claims_ui_symbols'))
f('main.rs', None, rng(1, 3), raw('', '@MODS@'), top('GLOBAL', 'main'))

# ---------------------------------------------------------------- bodies
def is_test(path): return path.endswith('tests.rs')
def strip_blank(ls):
    while ls and not ls[0].strip(): ls = ls[1:]
    while ls and not ls[-1].strip(): ls = ls[:-1]
    return ls

BODY = {}
for path, parts in SPEC.items():
    out = []
    for kind, src, arg in parts:
        if kind == 'raw':
            out += list(arg)
        elif kind == 'rng':
            out += src.lines(*arg)
        elif kind == 'top':
            out += strip_blank(src.text(src.take(src.top, arg))) + ['']
        elif kind == 'meth':
            out += strip_blank(src.text(src.take(src.sub[APP], arg)))
        elif kind == 'sub':
            out += strip_blank(src.text(src.take(src.sub[arg[0]], arg[1])))
        elif kind == 'tests':
            out += strip_blank(src.text(src.take(src.sub['tests'], arg), dedent=True)) + ['']
    BODY[path] = strip_blank(out)

# every source line must be placed, except `use`/`mod` framing and blanks
for name, src in (('main', M), ('fonts', F)):
    for kind, n, a, b in src.top + [c for s in src.sub.values() for c in s]:
        if kind in ('use',) or n == 'system_fonts':
            src.used.update(range(a, b))
    left = [i + 1 for i in range(len(src.L)) if i not in src.used and src.L[i].strip()]
    assert not left, (name, left[:40])

# ---------------------------------------------------------------- visibility
VIS = json.loads((HERE / 'vis.json').read_text()) if (HERE / 'vis.json').exists() else {'fields': [], 'methods': []}
EXTRA = json.loads((HERE / 'extra.json').read_text()) if (HERE / 'extra.json').exists() else {}
PUB = 'pub(crate) '
DEF = re.compile(r'^(?:pub(?:\(crate\))? )?(const|static|fn|struct|enum|type|trait) ([A-Za-z_]\w*)')

def modpath(path):
    p = path[:-3].split('/')
    if p[-1] == 'mod': p.pop()
    if p == ['main']: p = []
    return p

DEFS = {}
for path, body in BODY.items():
    if is_test(path): continue
    for l in body:
        m = DEF.match(l)
        if m and m.group(2) not in ('main', 'GLOBAL'):
            assert DEFS.get(m.group(2), path) == path, m.group(2)
            DEFS[m.group(2)] = path

EXT = [  # (name, detection regex, use path, cfg)
    ('HashMap', None, 'std::collections', None), ('HashSet', None, 'std::collections', None),
    ('OsStr', None, 'std::ffi', None), ('OsString', None, 'std::ffi', None),
    ('fs', r'(?<![:\w])fs::', 'std', None), ('self', r'(?<![:\w])io::', 'std::io', None),
    ('IsTerminal', r'\.is_terminal\(', 'std::io', None),
    ('OsStringExt', r'OsString::from_vec|\.into_vec\(', 'std::os::unix::ffi', 'target_os = "linux"'),
    ('UnixStream', None, 'std::os::unix::net', 'unix'),
    ('Path', None, 'std::path', None), ('PathBuf', None, 'std::path', None),
    ('Command', None, 'std::process', None), ('Stdio', None, 'std::process', None),
    ('self', r'(?<![:\w])mpsc::', 'std::sync::mpsc', None), ('Receiver', None, 'std::sync::mpsc', None),
    ('TryRecvError', None, 'std::sync::mpsc', None),
    ('Duration', None, 'std::time', None), ('Instant', None, 'std::time', None),
    ('Parser', r'\bParser\b|Args::(parse|try_parse)', 'clap', None),
    ('egui', r'(?<![:\w])egui::', 'eframe', None),
    ('Widget', r'\.ui\(ui\)', 'egui', None),
    ('header_position_key', None, 'egui_commonmark_extended', None),
    ('CommonMarkCache', None, 'egui_commonmark_extended', None),
    ('CommonMarkViewer', None, 'egui_commonmark_extended', None),
    ('PollWatcher', None, 'notify', None), ('RecommendedWatcher', None, 'notify', None),
    ('new_debouncer', None, 'notify_debouncer_mini', None), ('new_debouncer_opt', None, 'notify_debouncer_mini', None),
    ('DebouncedEventKind', None, 'notify_debouncer_mini', None), ('Debouncer', None, 'notify_debouncer_mini', None),
    ('Regex', None, 'regex', None), ('Deserialize', None, 'serde', None), ('Serialize', None, 'serde', None),
    ('McpBridge', None, 'egui_mcp_bridge', 'feature = "mcp"'),
    ('McpUiExt', r'(?<![\w:])McpBridge\b', 'egui_mcp_bridge', 'feature = "mcp"'),
]
FONT_EXT = [(n, None, p, None) for n, p in [
    ('HashSet', 'std::collections'), ('FontData', 'egui'), ('FontDefinitions', 'egui'), ('FontFamily', 'egui'),
    ('STRONG_FONT_FAMILY', 'egui_commonmark_extended'), ('Blob', 'fontique'), ('Collection', 'fontique'),
    ('CollectionOptions', 'fontique'), ('FamilyId', 'fontique'), ('FontStyle', 'fontique'), ('FontWeight', 'fontique'),
    ('GenericFamily', 'fontique'), ('Script', 'fontique'), ('SourceCache', 'fontique'), ('SourceKind', 'fontique'),
    ('Deserialize', 'serde'), ('Serialize', 'serde')]]

def code_text(body):
    return '\n'.join(re.sub(r'(^|\s)//.*$', '', l) for l in body)

IMPORTS = {}   # path -> {(cfg, usepath): set(names)}
NEEDS_PUB = set(VIS.get('types', []))
PUB_MODS = set()
def within(user, owner):   # is module `user` inside module `owner`
    return user[:len(owner)] == owner

order = sorted(BODY, key=is_test)
for path in order:
    body = BODY[path]
    text = code_text(body)
    imp = collections.defaultdict(set)
    mine = modpath(path)
    local = {m.group(2) for l in body for m in [DEF.match(l)] if m}
    inherited = set()
    if is_test(path):
        sup = path.replace('tests.rs', 'mod.rs')
        inherited = {n for names in IMPORTS[sup].values() for n in names} | {n for n, p in DEFS.items() if p == sup}
        mine = modpath(sup)
    exts = FONT_EXT if path.startswith('system_fonts/') else EXT
    for name, pat, usepath, cfg in exts:
        if name in inherited or name in EXTRA.get(path, {}).get('skip', []): continue
        if re.search(pat or r'(?<![\w:])' + name + r'\b', text):
            imp[(cfg, usepath)].add(name)
    for name, dpath in DEFS.items():
        if name in local or name in inherited or dpath == path or name in EXTRA.get(path, {}).get('skip', []): continue
        if is_test(path) and dpath == sup: continue
        if re.search(r'(?<![\w.:])' + name + r'\b', text):
            owner = modpath(dpath)
            imp[(None, '::'.join(['crate'] + owner))].add(name)
            if not within(mine, owner):
                NEEDS_PUB.add(name)
            # every module on the way down must be nameable from `mine`
            for depth in range(2, len(owner) + 1):
                if not within(mine, owner[:depth - 1]):
                    PUB_MODS.add('/'.join(owner[:depth]))
    for line in EXTRA.get(path, {}).get('use', []):
        imp[('raw', line)].add('')
    IMPORTS[path] = imp

def fmt_imports(path):
    groups = {0: [], 1: [], 2: []}
    for (cfg, usepath), names in IMPORTS[path].items():
        if cfg == 'raw':
            g = 2 if 'crate::' in usepath else (0 if 'std::' in usepath else 1)
            groups[g].append((usepath, [usepath])); continue
        names = sorted(names, key=lambda s: (s[0].isupper() and not s.isupper(), s.lower()))
        names = sorted(names, key=lambda n: (n != 'self', n.lower()))
        if usepath == 'std' or usepath == 'eframe':
            lines = [f'use {usepath}::{n};' for n in names]
        elif names == ['self']:
            lines = [f'use {usepath};']
        elif len(names) == 1:
            lines = [f'use {usepath}::{names[0]};']
        else:
            lines = [f'use {usepath}::{{{", ".join(names)}}};']
        if cfg: lines = [x for l in lines for x in (f'#[cfg({cfg})]', l)]
        g = 0 if usepath.startswith('std') else 2 if usepath.startswith('crate') else 1
        groups[g].append((usepath, lines))
    out = []
    for g in (0, 1, 2):
        ls = [l for _, lines in sorted(groups[g]) for l in lines]
        if ls: out += ls + ['']
    if not out: return out
    r = subprocess.run(['rustfmt', '--edition', '2021', '--emit', 'stdout'], input='\n'.join(out), capture_output=True, text=True, check=True, cwd=REPO)
    return r.stdout.rstrip('\n').split('\n') + ['']

def child_mods(path):
    if not (path.endswith('mod.rs') or path == 'main.rs'): return []
    d = os.path.dirname(path)
    kids = set()
    for p in BODY:
        if p == path: continue
        rel = os.path.relpath(p, d) if d else p
        if rel.startswith('..'): continue
        parts = rel.split('/')
        if len(parts) == 1: kids.add(parts[0][:-3])
        elif len(parts) == 2 and parts[1] == 'mod.rs': kids.add(parts[0])
    out = []
    for k in sorted(kids):
        full = '/'.join(modpath(path) + [k])
        if k == 'tests': out += ['#[cfg(test)]', 'mod tests;']
        else: out.append(('pub(crate) ' if full in PUB_MODS else '') + f'mod {k};')
    return out

def apply_vis(path, body):
    if is_test(path): return body
    out, struct = [], None
    for l in body:
        m = DEF.match(l)
        if m and not l.startswith('pub') and m.group(2) in NEEDS_PUB:
            l = PUB + l
        m2 = re.match(r'^(?:pub(?:\(crate\))? )?struct (\w+)', l)
        if m2 and l.rstrip().endswith('{'): struct = m2.group(1)
        elif l.startswith('}'): struct = None
        elif struct:
            m3 = re.match(r'^    ([a-z_]\w*):', l)
            if m3 and f'{struct}.{m3.group(1)}' in VIS['fields']: l = '    ' + PUB + l[4:]
        m4 = re.match(r'^    (?:const )?fn (\w+)', l)
        if m4 and f'{path}:{m4.group(1)}' in VIS['methods']: l = '    ' + PUB + l[4:]
        out.append(l)
    return out

src = pathlib.Path(REPO) / 'src'
shutil.rmtree(src)
report = []
for path, body in BODY.items():
    body = apply_vis(path, body)
    head = []
    if DOC[path]: head += ['//! ' + DOC[path], '']
    if is_test(path):
        head += ['use super::*;', '']
    imports = fmt_imports(path)
    mods = child_mods(path)
    if path == 'main.rs':
        i = body.index('@MODS@')
        body = body[:i] + imports + mods + [''] + body[i + 1:]
        lines = body
    else:
        lines = head + imports + (mods + [''] if mods else []) + body
    p = src / path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text('\n'.join(lines).rstrip('\n') + '\n')
    report.append((effective(lines), path))
for n, p in sorted(report, reverse=True):
    print(f'{n:5} {p}')
