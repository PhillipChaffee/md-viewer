//! Kani proof for the tab a saved session comes back on. `PROOFS.md`
//! lists the harness with its bounds.
//!
//! The harness is loop-free and covers every value the settings file can
//! hold for the field.

use super::restored_active_tab;

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
