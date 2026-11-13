//! Kani proofs for the decision to detach from the terminal and for what
//! `launch` does with it. `PROOFS.md` lists each harness with its bounds.
//!
//! The harnesses run the real `should_detach` and `launch` with stand-ins
//! for the two things `launch` is handed: the respawn and the window. The
//! respawn itself, the Wayland probe and the argument parsing touch the
//! operating system and are covered by `tests.rs`.

use std::cell::Cell;
use std::io;

use super::{launch, should_detach, Args};

fn any_args() -> Args {
    Args {
        file: None,
        no_watch: kani::any(),
        foreground: kani::any(),
        no_detach: kani::any(),
    }
}

/// The process detaches exactly when it was started from a terminal
/// without `--foreground` and is not itself the detached copy. The copy
/// carries `--no-detach`, so it never detaches again: there is no respawn
/// loop.
#[kani::proof]
fn only_a_terminal_launch_detaches_and_only_once() {
    let args = any_args();
    let from_terminal: bool = kani::any();

    let detach = should_detach(&args, from_terminal);

    assert!(detach == (from_terminal && !args.foreground && !args.no_detach));
    if args.no_detach || args.foreground || !from_terminal {
        assert!(!detach);
    }

    kani::cover!(detach);
    kani::cover!(!detach && from_terminal);
}

/// Every launch ends in exactly one of two ways: a detached copy was
/// started and this process opens no window, or this process opens the
/// window once. A respawn that fails falls back to the window, and the
/// window's own result is what `launch` returns.
#[kani::proof]
fn every_launch_opens_one_window_or_hands_over() {
    let args = any_args();
    let from_terminal: bool = kani::any();
    let respawn_works: bool = kani::any();
    let window_works: bool = kani::any();
    let respawns = Cell::new(0_u8);
    let windows = Cell::new(0_u8);

    let result = launch(
        &args,
        from_terminal,
        &|| {
            respawns.set(respawns.get() + 1);
            if respawn_works {
                Ok(())
            } else {
                Err(io::Error::from(io::ErrorKind::NotFound))
            }
        },
        &|_| {
            windows.set(windows.get() + 1);
            if window_works {
                Ok(())
            } else {
                Err(eframe::Error::AppCreation("no window".into()))
            }
        },
    );

    let detach = should_detach(&args, from_terminal);
    assert!(respawns.get() == u8::from(detach));
    if detach && respawn_works {
        assert!(windows.get() == 0);
        assert!(result.is_ok());
    } else {
        assert!(windows.get() == 1);
        assert!(result.is_ok() == window_works);
    }

    kani::cover!(detach && respawn_works);
    kani::cover!(detach && !respawn_works && window_works);
    kani::cover!(!detach && !window_works);
}
