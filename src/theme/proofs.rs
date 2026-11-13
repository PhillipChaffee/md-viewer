//! Kani proofs for the theme cycle, the restore of a saved theme and the
//! hex color constructor. `PROOFS.md` lists each harness with its bounds.
//!
//! The contrast the palettes promise is not proven here: it takes a
//! fractional power, which the verifier does not model. `tests.rs` checks
//! it.

use super::{rgb, ThemeId};

fn any_theme() -> ThemeId {
    ThemeId::ALL[usize::from(kani::any::<u8>() % 4)]
}

/// Ctrl+D visits all four themes and comes back: `next` is one cycle of
/// length four in the order of `ALL`.
#[kani::proof]
fn next_cycles_through_every_theme() {
    let theme = any_theme();
    let (second, third, fourth) = (
        theme.next(),
        theme.next().next(),
        theme.next().next().next(),
    );

    assert!(fourth.next() == theme);
    assert!(theme != second && theme != third && theme != fourth);
    assert!(second != third && second != fourth && third != fourth);

    let position: usize = kani::any();
    kani::assume(position < ThemeId::ALL.len());
    assert!(ThemeId::ALL[position].next() == ThemeId::ALL[(position + 1) % ThemeId::ALL.len()]);

    kani::cover!(theme == ThemeId::Night && second == ThemeId::Paper);
}

/// Every saved session comes back in a theme: a saved theme wins, a
/// session from before the themes keeps its light or dark, and a fresh
/// install gets the default.
#[kani::proof]
fn restored_maps_every_saved_session_to_a_theme() {
    let saved: Option<ThemeId> = kani::any::<bool>().then(any_theme);
    let legacy_dark_mode: Option<bool> = kani::any();
    let theme = ThemeId::restored(saved, legacy_dark_mode);

    match (saved, legacy_dark_mode) {
        (Some(saved), _) => {
            assert!(theme == saved);
        }
        (None, Some(dark)) => {
            assert!(theme.palette().dark == dark);
        }
        (None, None) => {
            assert!(theme == ThemeId::default());
        }
    }

    kani::cover!(saved.is_some() && legacy_dark_mode.is_some());
    kani::cover!(saved.is_none() && legacy_dark_mode == Some(true));
    kani::cover!(saved.is_none() && legacy_dark_mode == Some(false));
}

/// Each theme has its own palette, and `ALL` lists each theme once.
#[kani::proof]
fn every_theme_has_its_own_palette() {
    let (one, other) = (any_theme(), any_theme());

    if one != other {
        assert!(one.palette().page != other.palette().page);
        assert!(!std::ptr::eq(one.palette(), other.palette()));
    }

    kani::cover!(one != other);
    kani::cover!(one == other);
}

/// A `0xRRGGBB` value becomes the opaque color with those three channels,
/// for every 24-bit value.
#[kani::proof]
fn rgb_reads_the_three_channels() {
    let hex: u32 = kani::any();
    kani::assume(hex <= 0x00FF_FFFF);
    let color = rgb(hex);

    assert!(u32::from(color.r()) == hex >> 16);
    assert!(u32::from(color.g()) == (hex >> 8) & 0xFF);
    assert!(u32::from(color.b()) == hex & 0xFF);
    assert!(color.a() == u8::MAX);

    kani::cover!(hex == 0x00FD_FDFB);
}
