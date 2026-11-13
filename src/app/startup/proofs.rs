//! Kani proofs for what a saved session comes back with. `PROOFS.md`
//! lists each harness with its bounds.
//!
//! Each harness is loop-free and covers every value the settings file can
//! hold for the field.

use super::{restored_active_tab, restored_math_scale, restored_zoom};

/// Whatever index a session saved, the restored one names an open tab,
/// and it is the saved one wherever that tab still exists.
#[kani::proof]
fn restored_active_tab_names_an_open_tab() {
    let saved: Option<usize> = kani::any();
    let tab_count: usize = kani::any();

    let active = restored_active_tab(saved, tab_count);

    if tab_count == 0 {
        assert!(active == 0);
    } else {
        assert!(active < tab_count);
    }
    if let Some(saved) = saved {
        if saved < tab_count {
            assert!(active == saved);
        }
    } else {
        assert!(active == 0);
    }

    kani::cover!(saved.is_some_and(|saved| saved >= tab_count) && tab_count > 0);
    kani::cover!(saved.is_none() && tab_count > 0);
    kani::cover!(saved == Some(usize::MAX));
}

/// A saved zoom that is a number comes back inside the zoom range, a zoom
/// already in range comes back as saved, and no saved zoom is 100%.
///
/// A saved NaN is not covered: see the finding in `PROOFS.md`.
#[kani::proof]
fn restored_zoom_is_in_range() {
    let saved: Option<f32> = kani::any();
    kani::assume(!saved.is_some_and(f32::is_nan));

    let zoom = restored_zoom(saved);

    assert!((0.5..=3.0).contains(&zoom));
    match saved {
        Some(saved) if (0.5..=3.0).contains(&saved) => {
            assert!(zoom == saved);
        }
        Some(_) => {}
        None => {
            assert!(zoom == 1.0);
        }
    }

    kani::cover!(saved.is_some_and(f32::is_infinite));
    kani::cover!(saved.is_some_and(|saved| saved < 0.0));
}

/// A saved formula size that is a number comes back between 1 and 2.
///
/// A saved NaN is not covered: see the finding in `PROOFS.md`.
#[kani::proof]
fn restored_math_scale_is_in_range() {
    let saved: Option<f32> = kani::any();
    kani::assume(!saved.is_some_and(f32::is_nan));

    let scale = restored_math_scale(saved);

    assert!((1.0..=2.0).contains(&scale));
    match saved {
        Some(saved) if (1.0..=2.0).contains(&saved) => {
            assert!(scale == saved);
        }
        Some(_) => {}
        None => {
            assert!(scale == 1.0);
        }
    }

    kani::cover!(saved.is_some_and(|saved| saved > 2.0));
}

/// The gap the two harnesses above leave: a settings file that holds NaN
/// for the zoom or the formula size starts the app with NaN, because
/// `f32::clamp` passes NaN through. This harness pins that behavior so the
/// finding in `PROOFS.md` stays true until the code changes.
#[kani::proof]
fn restored_zoom_passes_nan_through() {
    assert!(restored_zoom(Some(f32::NAN)).is_nan());
    assert!(restored_math_scale(Some(f32::NAN)).is_nan());
}
