use super::*;

const NOW: u64 = 1_000_000;

fn entry(path: &str, opened_secs_ago: u64) -> RecentEntry {
    RecentEntry {
        path: PathBuf::from(path),
        last_opened: NOW - opened_secs_ago,
        title: None,
        excerpt: None,
        word_count: None,
    }
}

fn list_of(recents: &[RecentEntry], show_all: bool) -> HomeList {
    HomeList::build(recents, show_all, NOW, |_| true)
}

#[test]
fn a_row_says_what_the_document_is_where_it_is_and_when_it_was_read() {
    let read = RecentEntry {
        title: Some("On Reading".to_owned()),
        excerpt: Some("Read slowly, and twice.".to_owned()),
        word_count: Some(1000),
        ..entry("/home/me/notes/reading.md", 2 * 60 * 60)
    };

    let list = list_of(&[read], false);

    assert_eq!(
        list.rows,
        [HomeRow {
            path: PathBuf::from("/home/me/notes/reading.md"),
            title: "On Reading".to_owned(),
            // 1000 words at 230 a minute are a little over four minutes.
            meta: "notes · 5 min read · opened 2h ago".to_owned(),
            excerpt: Some("Read slowly, and twice.".to_owned()),
            exists: true,
        }]
    );
}

#[test]
fn a_document_without_a_title_goes_by_its_file_name() {
    let list = list_of(&[entry("/notes/shopping-list.md", 60)], false);

    assert_eq!(list.rows[0].title, "shopping-list");
    assert_eq!(list.rows[0].excerpt, None);
}

#[test]
fn an_entry_saved_before_documents_were_summarized_leaves_the_length_out() {
    let list = list_of(&[entry("/notes/old.md", 180)], false);

    assert_eq!(list.rows[0].meta, "notes · opened 3m ago");
}

#[test]
fn a_file_at_the_root_has_no_folder_to_name() {
    let list = list_of(&[entry("/top.md", 0)], false);

    assert_eq!(list.rows[0].meta, "opened just now");
    assert_eq!(list_of(&[entry("/", 0)], false).rows[0].title, "/");
}

#[test]
fn a_file_that_is_gone_says_so_in_place_of_when_it_was_read() {
    let recents = [entry("/notes/here.md", 60), entry("/notes/gone.md", 60)];

    let list = HomeList::build(&recents, false, NOW, |path| path.ends_with("here.md"));

    assert!(list.rows[0].exists);
    assert_eq!(list.rows[0].meta, "notes · opened 1m ago");
    assert!(!list.rows[1].exists);
    assert_eq!(list.rows[1].meta, "notes · file not found");
}

#[test]
fn the_short_list_stops_at_six_and_says_there_are_more() {
    let recents: Vec<RecentEntry> = (0..9).map(|n| entry(&format!("/n/{n}.md"), n)).collect();

    let short = list_of(&recents, false);
    let whole = list_of(&recents, true);

    assert_eq!(short.rows.len(), RECENT_SHOWN);
    assert_eq!(short.rows[0].title, "0", "most recent first");
    assert!(short.has_more);
    assert!(!short.show_all);
    assert_eq!(whole.rows.len(), 9);
    assert!(whole.has_more);
    assert!(whole.show_all);
}

#[test]
fn a_list_that_fits_has_no_more_to_show() {
    let recents: Vec<RecentEntry> = (0..6).map(|n| entry(&format!("/n/{n}.md"), n)).collect();

    assert!(!list_of(&recents, false).has_more);
    assert!(list_of(&[], false).rows.is_empty());
}

#[test]
fn a_list_is_built_again_after_half_a_minute() {
    let list = list_of(&[], false);

    assert!(!list.is_stale(NOW));
    assert!(!list.is_stale(NOW + 29));
    assert!(list.is_stale(NOW + 30));
    assert!(!list.is_stale(NOW - 5), "a clock set back is not an age");
}

#[test]
fn the_page_keeps_one_list_until_the_files_change_or_it_ages() {
    let recents = [entry("/notes/a.md", 60)];
    let there = std::cell::Cell::new(true);
    let exists = |_: &Path| there.get();
    let mut slot = None;

    let built = HomeList::current(&mut slot, &recents, false, NOW, exists).clone();
    assert!(built.rows[0].exists);

    // The file goes away, and the list is not looked at again at once.
    there.set(false);
    assert_eq!(
        HomeList::current(&mut slot, &recents, false, NOW + 5, exists),
        &built
    );

    let aged = HomeList::current(&mut slot, &recents, false, NOW + 30, exists);
    assert!(!aged.rows[0].exists);
    assert_eq!(aged.rows[0].meta, "notes · file not found");

    // A change to the recent files empties the slot.
    slot = None;
    let rebuilt = HomeList::current(&mut slot, &[], false, NOW + 31, exists);
    assert!(rebuilt.rows.is_empty());
}
