use super::*;

#[test]
fn a_fresh_change_flashes_at_full_strength() {
    assert_eq!(flash_alpha(Duration::ZERO), Some(180));
}

#[test]
fn the_flash_fades_in_proportion_to_the_time_left() {
    // Half of the 600ms flash is left, so half of the 180 alpha.
    assert_eq!(flash_alpha(Duration::from_millis(300)), Some(90));
    assert_eq!(flash_alpha(Duration::from_millis(150)), Some(135));
}

#[test]
fn the_flash_stays_visible_until_it_ends() {
    // A proportional fade would reach 45 and then 0 here.
    assert_eq!(flash_alpha(Duration::from_millis(450)), Some(60));
    assert_eq!(flash_alpha(Duration::from_millis(599)), Some(60));
}

#[test]
fn the_flash_is_over_after_its_duration() {
    assert_eq!(flash_alpha(Duration::from_millis(600)), None);
    assert_eq!(flash_alpha(Duration::from_secs(5)), None);
}

#[test]
fn the_explorer_title_is_the_folder_name() {
    assert_eq!(folder_name(Path::new("/home/me/notes")), "notes");
}

#[test]
fn a_root_without_a_name_is_titled_by_its_path() {
    assert_eq!(folder_name(Path::new("/")), "/");
}
