//! Kani proofs for the fade of the explorer's change highlight.
//! `PROOFS.md` lists each harness with its bounds.

use std::time::Duration;

use super::flash_alpha;
use crate::watcher::FLASH_DURATION_MS;

/// For every elapsed time the highlight's opacity is computed without
/// overflow; while the flash lasts it is between the floor of 60 and the
/// start of 180 and never brightens as time passes, and once the flash is
/// over there is none.
#[kani::proof]
fn the_flash_fades_to_its_floor_and_ends() {
    let (earlier, later): (Duration, Duration) = (kani::any(), kani::any());
    kani::assume(earlier <= later);
    let total = Duration::from_millis(FLASH_DURATION_MS);

    let (first, second) = (flash_alpha(earlier), flash_alpha(later));

    match first {
        Some(alpha) => {
            assert!(earlier < total);
            assert!((60..=180).contains(&alpha));
        }
        None => {
            assert!(earlier.as_millis() >= total.as_millis());
        }
    }
    match (first, second) {
        (Some(first), Some(second)) => {
            assert!(second <= first);
        }
        (None, Some(_)) => {
            panic!("a flash that ended came back");
        }
        _ => {}
    }

    kani::cover!(first == Some(180));
    kani::cover!(first == Some(60));
    kani::cover!(first.is_some() && second.is_none());
    kani::cover!(earlier == Duration::MAX);
}
