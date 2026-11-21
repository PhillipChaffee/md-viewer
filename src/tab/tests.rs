use super::*;

use crate::test_support::ScratchDir;

#[test]
fn tab_history_changes_only_after_successful_loads() {
    let dir = ScratchDir::new();
    let first = dir.file("first.md", "# First");
    let second = dir.file("second.md", "# Second");

    let mut tab = Tab::new(first.clone()).unwrap();
    assert!(tab.navigate_to_link("second.md").unwrap());
    assert_eq!(tab.path, second.canonicalize().unwrap());
    assert!(tab.can_go_back());

    fs::remove_file(&first).unwrap();
    assert!(tab.navigate_back().is_err());
    assert_eq!(tab.path, second.canonicalize().unwrap());
    assert!(tab.can_go_back());
    assert!(!tab.can_go_forward());
}

#[test]
fn unreadable_new_tab_returns_an_error() {
    let dir = ScratchDir::new();
    assert!(Tab::new(dir.path().join("missing.md")).is_err());
}

#[test]
fn tab_path_navigation_tracks_back_and_forward_history() {
    let dir = ScratchDir::new();
    let first = dir.file("first.md", "# First");
    let second = dir.file("second.md", "# Second");

    let mut tab = Tab::new(first.canonicalize().unwrap()).unwrap();
    assert!(tab.navigate_to_path(&second).unwrap());
    assert_eq!(tab.path, second.canonicalize().unwrap());
    assert!(tab.can_go_back());
    assert!(!tab.can_go_forward());

    tab.navigate_back().unwrap();
    assert_eq!(tab.path, first.canonicalize().unwrap());
    assert!(tab.can_go_forward());

    tab.navigate_forward().unwrap();
    assert_eq!(tab.path, second.canonicalize().unwrap());
}

/// A scratch directory holding `docs`, with a tab on the first of them.
fn tab_on(docs: &[(&str, &str)]) -> (ScratchDir, Tab) {
    let dir = ScratchDir::new();
    for (name, content) in docs {
        dir.file(name, content);
    }
    let tab = Tab::new(dir.path().join(docs[0].0)).unwrap();
    (dir, tab)
}

/// A tab on a lone `notes.md` holding `content`.
fn tab_with(content: &str) -> (ScratchDir, Tab) {
    tab_on(&[("notes.md", content)])
}

mod document {
    use super::*;
    use crate::test_support::assert_close;

    #[test]
    fn tab_is_titled_after_the_file_it_shows() {
        let (dir, mut tab) = tab_on(&[("notes.md", "# Notes"), ("other.md", "# Other")]);
        assert_eq!(tab.title(), "notes.md");

        assert!(tab.navigate_to_path(&dir.path().join("other.md")).unwrap());
        assert_eq!(tab.title(), "other.md");
        tab.navigate_back().unwrap();
        assert_eq!(tab.title(), "notes.md");

        assert_eq!(file_title(Path::new("/")), "Unknown");
    }

    #[test]
    fn tab_is_named_by_the_first_heading_else_by_the_file_and_follows_an_edit() {
        let (dir, mut tab) = tab_on(&[("notes.md", "# Field Notes"), ("plain.md", "No heading.")]);
        assert_eq!(tab.name(), "Field Notes");

        tab.content = "# Renamed\n".to_owned();
        tab.content_edited();
        assert_eq!(tab.name(), "Renamed");
        tab.content = "No heading any more.".to_owned();
        tab.content_edited();
        assert_eq!(tab.name(), "notes.md");

        assert!(tab.navigate_to_path(&dir.path().join("plain.md")).unwrap());
        assert_eq!(tab.name(), "plain.md");
    }

    #[test]
    fn images_and_links_resolve_against_the_folder_of_the_document() {
        let (dir, tab) = tab_on(&[("notes.md", "# Notes")]);
        assert_eq!(tab.base_uri, format!("file://{}/", dir.path().display()));
        assert_eq!(Tab::compute_base_uri(Path::new("/")), "file://");
    }

    #[test]
    fn reload_shows_the_new_content_and_keeps_the_scroll_position() {
        let (dir, mut tab) = tab_on(&[("notes.md", "# Notes\n\n## Old\n")]);
        tab.scroll_offset = 120.0;
        let loaded_version = tab.content_version;

        dir.file("notes.md", "# Notes\n\n## New\n\nSee [other](other.md).\n");
        tab.reload().unwrap();

        assert_eq!(tab.outline.headers()[1].title, "New");
        assert_eq!(tab.content_lines, 5);
        assert_eq!(tab.saved_content, tab.content);
        assert_close(tab.scroll_offset, 120.0);
        assert_ne!(tab.content_version, loaded_version);
    }

    #[test]
    fn reload_reports_a_file_that_went_away() {
        let (dir, mut tab) = tab_on(&[("notes.md", "# Notes")]);
        fs::remove_file(dir.path().join("notes.md")).unwrap();

        assert!(tab.reload().is_err());
        assert_eq!(tab.content, "# Notes");
    }

    #[test]
    fn search_matches_follow_the_query() {
        let (_dir, mut tab) = tab_on(&[("notes.md", "alpha beta\nalpha\n")]);

        tab.rebuild_search("alpha");
        assert_eq!(tab.search_matches.len(), 2);

        tab.rebuild_search("");
        assert!(tab.search_matches.is_empty());
    }

    #[test]
    fn clicked_link_is_the_one_whose_hook_fired() {
        let (_dir, mut tab) = tab_on(&[
            ("notes.md", "[one](one.md) and [two](two.md)"),
            ("one.md", "# One"),
            ("two.md", "# Two"),
        ]);
        assert_eq!(tab.check_link_hooks(), None);

        // The renderer flips a link's hook when the link is clicked.
        tab.cache.link_hooks_mut().insert("two.md".to_owned(), true);
        assert_eq!(tab.check_link_hooks().as_deref(), Some("two.md"));
    }
}

mod editing {
    use super::*;
    use crate::test_support::assert_close;

    #[test]
    fn an_edit_brings_the_contents_the_meta_line_and_the_links_up_to_date() {
        let (_dir, mut tab) = tab_on(&[
            ("notes.md", "# Notes\n\n## One\n\n## Two\n"),
            ("other.md", "# Other"),
        ]);
        tab.outline.toggle(1);

        tab.content.push_str("\nA line of prose.");
        tab.content_edited();
        assert_eq!(tab.content_lines, 7);
        assert!(
            tab.outline.is_collapsed(1),
            "typing prose folds nothing open"
        );

        tab.content = format!(
            "# Renamed\n\nSee [other](other.md).\n\n{}",
            "word ".repeat(700)
        );
        tab.content_edited();

        let titles: Vec<&str> = tab
            .outline
            .headers()
            .iter()
            .map(|header| header.title.as_str())
            .collect();
        assert_eq!(titles, ["Renamed"]);
        assert_eq!(tab.document_title.as_deref(), Some("Renamed"));
        assert_eq!(tab.kicker, "notes.md \u{b7} 4 min read");
        assert!(
            !tab.outline.is_collapsed(1),
            "a fold is kept by position, which a removed heading shifts"
        );
        // The new link is one the renderer reports clicks on.
        tab.cache
            .link_hooks_mut()
            .insert("other.md".to_owned(), true);
        assert_eq!(tab.check_link_hooks().as_deref(), Some("other.md"));
    }

    #[test]
    fn switching_to_the_editor_and_back_keeps_the_place_in_the_document() {
        let (_dir, mut tab) = tab_with("# Notes");
        tab.scroll_offset = 200.0;
        tab.last_content_height = 1000.0;

        tab.set_edit_mode(true);
        assert!(tab.is_editing);
        assert_close(tab.pending_edit_scroll.unwrap(), 0.2);

        // The editor surface has its own height; half way down it is half
        // way down the rendered view.
        tab.scroll_offset = 250.0;
        tab.last_content_height = 500.0;
        tab.set_edit_mode(false);
        assert!(!tab.is_editing);
        assert_close(tab.pending_scroll_offset.unwrap(), 500.0);
    }

    #[test]
    fn asking_for_the_current_mode_changes_nothing() {
        let (_dir, mut tab) = tab_with("# Notes");
        let version = tab.content_version;

        tab.set_edit_mode(false);

        assert_eq!(tab.content_version, version);
        assert_eq!(tab.pending_scroll_offset, None);
        tab.set_edit_mode(true);
        assert_ne!(tab.content_version, version, "a real switch re-renders");
    }

    #[test]
    fn a_tab_is_dirty_while_its_text_differs_from_the_file() {
        let (_dir, mut tab) = tab_with("# Notes");
        assert!(!tab.is_dirty());

        tab.content.push('!');
        assert!(tab.is_dirty());

        tab.content.pop();
        assert!(!tab.is_dirty(), "typed back to what is on disk");

        tab.content.push('!');
        tab.save_to_disk().unwrap();
        assert!(!tab.is_dirty());
    }

    #[test]
    fn a_save_notes_how_it_went() {
        let (dir, mut tab) = tab_with("# Notes");
        tab.content.push('!');

        tab.save().unwrap();
        assert!(matches!(tab.save_flash, Some((SaveOutcome::Saved, _))));
        assert!(!tab.is_dirty());

        tab.content.push('!');
        fs::remove_dir_all(dir.path()).unwrap();
        assert!(tab.save().is_err());
        assert!(matches!(tab.save_flash, Some((SaveOutcome::Failed, _))));
        assert!(tab.is_dirty());
    }

    #[test]
    fn save_writes_the_buffer_and_leaves_no_temporary_file() {
        let (dir, mut tab) = tab_with("# Notes");
        tab.content = "# Notes\n\nEdited.\n".to_owned();

        tab.save_to_disk().unwrap();

        assert_eq!(
            fs::read_to_string(dir.path().join("notes.md")).unwrap(),
            "# Notes\n\nEdited.\n"
        );
        assert_eq!(tab.saved_content, tab.content);
        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, ["notes.md"]);
    }

    #[test]
    fn failed_save_keeps_the_buffer_unsaved_and_cleans_up() {
        let (dir, mut tab) = tab_with("# Notes");
        tab.content = "# Edited".to_owned();
        // A folder with something in it cannot be replaced by a file.
        fs::remove_file(dir.path().join("notes.md")).unwrap();
        dir.file("notes.md/blocker", "in the way");

        assert!(tab.save_to_disk().is_err());

        assert_eq!(tab.saved_content, "# Notes");
        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, ["notes.md"], "the temporary file is removed");
    }

    #[test]
    fn save_into_a_folder_that_is_gone_reports_the_error() {
        let (dir, mut tab) = tab_with("# Notes");
        tab.path = dir.path().join("gone").join("notes.md");

        assert!(tab.save_to_disk().is_err());
        assert_eq!(tab.saved_content, "# Notes");
    }
}

mod disk {
    use super::*;

    /// A tab on `notes.md` with an unsaved `!` typed at its end.
    fn edited_tab() -> (ScratchDir, Tab) {
        let (dir, mut tab) = tab_with("# Notes");
        tab.content.push('!');
        tab.content_edited();
        (dir, tab)
    }

    #[test]
    fn a_tab_without_unsaved_edits_reloads_and_has_no_notice() {
        let (dir, mut tab) = tab_with("# Notes");
        dir.file("notes.md", "# Theirs");

        assert_eq!(tab.follow_disk().unwrap(), Followed::Reloaded);

        assert_eq!(tab.content, "# Theirs");
        assert_eq!(tab.disk_notice(), None);
        fs::remove_file(&tab.path).unwrap();
        assert!(tab.follow_disk().is_err());
        assert_eq!(tab.disk_notice(), None);
    }

    #[test]
    fn unsaved_edits_are_kept_and_measured_against_the_files_new_text() {
        let (dir, mut tab) = edited_tab();
        dir.file("notes.md", "# Theirs");

        assert_eq!(tab.follow_disk().unwrap(), Followed::KeptEdits);

        assert_eq!(tab.content, "# Notes!");
        assert_eq!(tab.disk_notice(), Some(DiskChange::Changed));
        tab.content.pop();
        tab.content_edited();
        assert!(tab.is_dirty(), "the file no longer holds this text");
        assert_eq!(tab.disk_notice(), Some(DiskChange::Changed));

        tab.content = "# Theirs".to_owned();
        tab.content_edited();
        assert!(!tab.is_dirty());
        assert_eq!(tab.disk_notice(), None, "no notice without unsaved edits");
    }

    #[test]
    fn a_file_left_as_it_was_is_not_announced() {
        let (_dir, mut tab) = edited_tab();

        assert_eq!(tab.follow_disk().unwrap(), Followed::KeptEdits);

        assert_eq!(tab.disk_notice(), None);
    }

    #[test]
    fn keep_mine_takes_the_notice_down_until_the_file_changes_again() {
        let (dir, mut tab) = edited_tab();
        dir.file("notes.md", "# Theirs");
        tab.follow_disk().unwrap();

        tab.keep_mine();
        assert_eq!(tab.disk_notice(), None);
        tab.follow_disk().unwrap();
        assert_eq!(tab.disk_notice(), None, "the same text is no news");

        dir.file("notes.md", "# Theirs again");
        tab.follow_disk().unwrap();
        assert_eq!(tab.disk_notice(), Some(DiskChange::Changed));
    }

    #[test]
    fn a_file_that_is_gone_is_announced_once_and_a_save_settles_it() {
        let (_dir, mut tab) = edited_tab();
        fs::remove_file(&tab.path).unwrap();

        tab.follow_disk().unwrap();
        assert_eq!(tab.disk_notice(), Some(DiskChange::Gone));
        tab.keep_mine();
        tab.follow_disk().unwrap();
        assert_eq!(tab.disk_notice(), None, "it was gone already");

        tab.save().unwrap();
        tab.content.push('?');
        tab.follow_disk().unwrap();
        assert_eq!(tab.disk_notice(), None);
    }
}

mod navigation {
    use super::*;

    #[test]
    fn links_that_name_no_other_document_do_not_navigate() {
        let (dir, mut tab) = tab_on(&[("notes.md", "# Notes")]);
        dir.dir("folder");
        let here = tab.path.clone();

        for link in ["https://example.com/guide.md", "notes.md", "folder"] {
            assert!(!tab.navigate_to_link(link).unwrap(), "{link}");
        }

        assert_eq!(tab.path, here);
        assert!(!tab.can_go_back());
    }

    #[test]
    fn link_to_a_missing_document_is_an_error() {
        let (_dir, mut tab) = tab_on(&[("notes.md", "# Notes")]);
        assert!(tab.navigate_to_link("missing.md").is_err());
        assert!(!tab.can_go_back());
    }

    #[test]
    fn path_navigation_ignores_missing_files_folders_and_the_open_document() {
        let (dir, mut tab) = tab_on(&[("notes.md", "# Notes")]);
        let folder = dir.dir("folder");
        let here = tab.path.clone();

        assert!(!tab
            .navigate_to_path(&dir.path().join("missing.md"))
            .unwrap());
        assert!(!tab.navigate_to_path(&folder).unwrap());
        assert!(!tab.navigate_to_path(&here).unwrap());

        assert_eq!(tab.path, here);
        assert!(!tab.can_go_back());
    }

    #[test]
    fn forward_step_to_a_deleted_document_fails_and_keeps_the_history() {
        let (dir, mut tab) = tab_on(&[("notes.md", "# Notes"), ("other.md", "# Other")]);
        let (notes, other) = (dir.path().join("notes.md"), dir.path().join("other.md"));
        assert!(tab.navigate_to_path(&other).unwrap());
        assert!(tab.navigate_back().unwrap());

        fs::remove_file(&other).unwrap();

        assert!(tab.navigate_forward().is_err());
        assert_eq!(tab.path, notes);
        assert!(tab.can_go_forward());
    }

    #[test]
    fn history_steps_with_nowhere_to_go_do_nothing() {
        let (_dir, mut tab) = tab_on(&[("notes.md", "# Notes")]);
        assert!(!tab.navigate_back().unwrap());
        assert!(!tab.navigate_forward().unwrap());
    }

    #[test]
    fn only_destinations_that_name_a_file_resolve_to_a_path() {
        let (dir, mut tab) = tab_on(&[("notes.md", "# Notes"), ("other.md", "# Other")]);

        assert_eq!(
            tab.resolve_link("other.md").unwrap(),
            Some(dir.path().join("other.md"))
        );
        assert_eq!(tab.resolve_link("#section").unwrap(), None);
        assert_eq!(tab.resolve_link("https://example.com/a.md").unwrap(), None);
        assert!(tab.resolve_link("missing.md").is_err());
        assert!(
            tab.resolve_link("bad%FF.md").is_err(),
            "a name that is not text once decoded is reported, not ignored"
        );

        // A document without a containing folder has nothing to resolve
        // relative links against.
        tab.path = PathBuf::from("/");
        assert_eq!(tab.resolve_link("other.md").unwrap(), None);
    }

    #[test]
    fn a_route_leads_away_only_when_it_ends_at_another_document() {
        let docs = [("notes.md", "# Notes"), ("other.md", "# Other")];
        let (dir, mut tab) = tab_on(&docs);
        let folder = dir.dir("folder");
        let link = |link: &str| Route::Link(link.to_owned());
        let file = |name: &str| Route::File(dir.path().join(name));

        for away in [link("other.md"), link("other.md#top"), file("other.md")] {
            assert!(tab.leads_away(&away), "{away:?}");
        }
        let stays = [
            link("#notes"),
            link("notes.md"),
            link("notes.md#notes"),
            link("missing.md"),
            link("bad%FF.md"),
            link("https://example.com"),
            link("folder"),
            file("notes.md"),
            file("missing.md"),
            Route::File(folder),
            Route::Back,
            Route::Forward,
        ];
        for stay in stays {
            assert!(!tab.leads_away(&stay), "{stay:?}");
        }

        assert!(tab.take(&file("other.md")).unwrap());
        assert!(tab.leads_away(&Route::Back));
        assert!(!tab.leads_away(&Route::Forward));
        assert!(tab.take(&Route::Back).unwrap());
        assert!(tab.leads_away(&Route::Forward));
        assert!(!tab.leads_away(&Route::Back));
    }

    #[test]
    fn each_route_is_taken_as_its_own_step_would_be() {
        let docs = [("notes.md", "# Notes"), ("other.md", "# Other")];
        let (dir, mut tab) = tab_on(&docs);
        let (notes, other) = (tab.path.clone(), dir.path().join("other.md"));

        assert!(tab.take(&Route::Link("other.md".to_owned())).unwrap());
        assert_eq!(tab.path, other);
        assert!(tab.take(&Route::Back).unwrap());
        assert_eq!(tab.path, notes);
        assert!(tab.take(&Route::Forward).unwrap());
        assert_eq!(tab.path, other);
        assert!(tab.take(&Route::File(notes.clone())).unwrap());
        assert_eq!(tab.path, notes);
        assert_eq!(tab.history_back, [notes.clone(), other]);

        let missing = Route::File(dir.path().join("missing.md"));
        assert!(tab.take(&missing).is_err(), "a file picked by name");
        assert_eq!(tab.path, notes);
    }
}

#[test]
fn reading_time_is_a_minute_per_230_words_rounded_up_and_never_zero() {
    assert_eq!(reading_minutes(""), 1);
    assert_eq!(reading_minutes("one two three"), 1);
    assert_eq!(reading_minutes(&"word ".repeat(230)), 1);
    assert_eq!(reading_minutes(&"word ".repeat(231)), 2);
    assert_eq!(reading_minutes(&"word\n\n".repeat(2_300)), 10);
}

#[test]
fn the_meta_line_names_the_file_and_follows_the_content_that_is_loaded() {
    let dir = ScratchDir::new();
    let short = dir.file("short.md", "# Short\n\nA few words.");
    let long = dir.file("long read.md", "word ".repeat(700));

    let mut tab = Tab::new(short).unwrap();
    assert_eq!(tab.kicker, "short.md \u{b7} 1 min read");

    // Navigating loads another document into the same tab.
    assert!(tab.navigate_to_path(&long).unwrap());
    assert_eq!(tab.kicker, "long read.md \u{b7} 4 min read");

    // A reload picks up what is on disk now.
    fs::write(&long, "Cut down to a line.").unwrap();
    tab.reload().unwrap();
    assert_eq!(tab.kicker, "long read.md \u{b7} 1 min read");
}
