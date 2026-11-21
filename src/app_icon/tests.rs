use std::collections::HashMap;

use super::*;
use crate::persist::{PersistedState, APP_KEY};
use crate::test_support::ScratchDir;

/// The side of each picture in pixels.
const SIDE: u32 = 512;

/// The alpha of the pixel `x` right and `y` down from the top left corner.
fn alpha_at(image: &IconData, x: u32, y: u32) -> u8 {
    let index = usize::try_from((y * image.width + x) * 4 + 3).unwrap();
    image.rgba[index]
}

/// A settings folder as eframe leaves it after a session saved as `state`.
fn folder_after(state: &PersistedState) -> ScratchDir {
    let session = ron::to_string(state).unwrap();
    let stored = HashMap::from([(APP_KEY.to_owned(), session)]);
    let folder = ScratchDir::new();
    folder.file(SETTINGS_FILE, ron::to_string(&stored).unwrap());
    folder
}

#[test]
fn the_light_themes_take_the_sepia_icon_and_the_dark_ones_the_reversed() {
    assert_eq!(AppIcon::for_theme(ThemeId::Paper), AppIcon::Sepia);
    assert_eq!(AppIcon::for_theme(ThemeId::Sepia), AppIcon::Sepia);
    assert_eq!(AppIcon::for_theme(ThemeId::Dusk), AppIcon::Reversed);
    assert_eq!(AppIcon::for_theme(ThemeId::Night), AppIcon::Reversed);
}

#[test]
fn a_first_run_opens_with_the_sepia_icon() {
    let empty = ScratchDir::new();

    assert_eq!(AppIcon::at_launch(None), AppIcon::Sepia);
    assert_eq!(AppIcon::at_launch(Some(empty.path())), AppIcon::Sepia);
}

#[test]
fn the_window_opens_with_the_icon_of_the_theme_the_last_session_saved() {
    let saved_as = |theme| {
        let folder = folder_after(&PersistedState {
            theme: Some(theme),
            ..PersistedState::default()
        });
        AppIcon::at_launch(Some(folder.path()))
    };

    assert_eq!(saved_as(ThemeId::Paper), AppIcon::Sepia);
    assert_eq!(saved_as(ThemeId::Sepia), AppIcon::Sepia);
    assert_eq!(saved_as(ThemeId::Dusk), AppIcon::Reversed);
    assert_eq!(saved_as(ThemeId::Night), AppIcon::Reversed);
}

#[test]
fn a_settings_file_that_cannot_be_read_opens_with_the_sepia_icon() {
    let folder = ScratchDir::new();
    folder.file(SETTINGS_FILE, "not a settings file");

    assert_eq!(AppIcon::at_launch(Some(folder.path())), AppIcon::Sepia);
}

#[test]
fn each_icon_is_a_square_picture_of_a_tile_with_clear_corners() {
    for icon in [AppIcon::Sepia, AppIcon::Reversed] {
        let image = icon.image();

        assert_eq!((image.width, image.height), (SIDE, SIDE), "{icon:?}");
        let bytes = usize::try_from(SIDE * SIDE * 4).unwrap();
        assert_eq!(image.rgba.len(), bytes, "{icon:?}");
        for (x, y) in [(0, 0), (SIDE - 1, 0), (0, SIDE - 1), (SIDE - 1, SIDE - 1)] {
            assert_eq!(alpha_at(&image, x, y), 0, "{icon:?} at {x}, {y}");
        }
        assert_eq!(alpha_at(&image, SIDE / 2, SIDE / 2), 255, "{icon:?}");
        assert_eq!(alpha_at(&image, SIDE / 2, SIDE / 8), 255, "{icon:?}");
    }
}

#[test]
fn the_two_icons_are_different_pictures() {
    assert_ne!(AppIcon::Sepia.image(), AppIcon::Reversed.image());
}
