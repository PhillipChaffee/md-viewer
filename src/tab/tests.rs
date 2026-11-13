use super::*;

use crate::recent::now_epoch_secs;

#[test]
fn tab_history_changes_only_after_successful_loads() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-history-safety-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(&root).unwrap();
    let first = root.join("first.md");
    let second = root.join("second.md");
    fs::write(&first, "# First").unwrap();
    fs::write(&second, "# Second").unwrap();

    let mut tab = Tab::new(first.clone()).unwrap();
    assert!(tab.navigate_to_link("second.md").unwrap());
    assert_eq!(tab.path, second.canonicalize().unwrap());
    assert!(tab.can_go_back());

    fs::remove_file(&first).unwrap();
    assert!(tab.navigate_back().is_err());
    assert_eq!(tab.path, second.canonicalize().unwrap());
    assert!(tab.can_go_back());
    assert!(!tab.can_go_forward());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unreadable_new_tab_returns_an_error() {
    let missing = std::env::temp_dir().join(format!(
        "md-viewer-missing-{}-{}.md",
        std::process::id(),
        now_epoch_secs()
    ));
    assert!(Tab::new(missing).is_err());
}

#[test]
fn tab_path_navigation_tracks_back_and_forward_history() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-navigation-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(&root).unwrap();
    let first = root.join("first.md");
    let second = root.join("second.md");
    fs::write(&first, "# First").unwrap();
    fs::write(&second, "# Second").unwrap();

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
    fs::remove_dir_all(root).unwrap();
}
