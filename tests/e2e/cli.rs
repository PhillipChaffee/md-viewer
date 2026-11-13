//! The command line: what `md-viewer` prints and how it exits.

use super::support::{run_to_end, Home};

#[test]
fn help_lists_what_a_user_may_pass() {
    let run = run_to_end(&Home::new(), &["--help"]);

    assert_eq!(run.code, Some(0), "{run:?}");
    for part in [
        "Usage: md-viewer",
        "[FILE]",
        "--no-watch",
        "--foreground",
        "--version",
    ] {
        assert!(run.stdout.contains(part), "no {part} in {}", run.stdout);
    }
    assert!(
        !run.stdout.contains("--no-detach"),
        "the marker the app passes to itself is not for users: {}",
        run.stdout
    );
}

#[test]
fn version_prints_the_name_and_the_package_version() {
    let run = run_to_end(&Home::new(), &["--version"]);

    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(
        run.stdout.trim(),
        concat!("md-viewer ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn an_unknown_flag_is_a_usage_error() {
    let run = run_to_end(&Home::new(), &["--bogus"]);

    assert_eq!(run.code, Some(2), "{run:?}");
    assert!(run.stderr.contains("--bogus"), "{run:?}");
    assert!(run.stderr.contains("Usage: md-viewer"), "{run:?}");
    assert!(run.stdout.is_empty(), "{run:?}");
}

#[test]
fn a_second_file_is_a_usage_error() {
    let home = Home::new();
    home.doc("a.md", "# A");
    home.doc("b.md", "# B");

    let run = run_to_end(&home, &["a.md", "b.md"]);

    assert_eq!(run.code, Some(2), "{run:?}");
    assert!(run.stderr.contains("b.md"), "{run:?}");
}

#[cfg(target_os = "linux")]
#[test]
fn without_a_display_it_fails_and_does_not_hang() {
    let home = Home::new();
    home.doc("notes.md", "# Notes");

    let run = run_to_end(&home, &["--foreground", "notes.md"]);

    assert_eq!(run.code, Some(1), "{run:?}");
    assert!(run.stderr.contains("DISPLAY"), "{run:?}");
}
