//! The command line, in depth: every flag in every spelling, and what is
//! and is not a usage error.

use super::support::{run_to_end, Home};

/// A home with two documents to name on the command line.
fn home() -> Home {
    let home = Home::new();
    home.doc("a.md", "# A");
    home.doc("b.md", "# B");
    home.doc("c.md", "# C");
    home
}

#[test]
fn the_short_flags_answer_as_the_long_ones_do() {
    let home = home();

    for (short, long) in [("-h", "--help"), ("-V", "--version")] {
        let (short, long) = (run_to_end(&home, &[short]), run_to_end(&home, &[long]));

        assert_eq!(short.code, Some(0), "{short:?}");
        assert_eq!(short.stdout, long.stdout);
        assert!(!short.stdout.is_empty());
    }
}

#[test]
fn help_and_version_print_to_standard_output_only() {
    let home = home();

    for flag in ["--help", "--version"] {
        let run = run_to_end(&home, &[flag]);

        assert!(run.stderr.is_empty(), "{run:?}");
        assert!(run.stdout.ends_with('\n'), "{run:?}");
    }
}

#[test]
fn help_is_given_whatever_else_is_on_the_line() {
    let home = home();

    for args in [
        vec!["a.md", "--help"],
        vec!["--help", "a.md"],
        vec!["--no-watch", "--foreground", "--help"],
        vec!["missing.md", "-h"],
    ] {
        let run = run_to_end(&home, &args);

        assert_eq!(run.code, Some(0), "{args:?}: {run:?}");
        assert!(run.stdout.contains("Usage: md-viewer"), "{args:?}: {run:?}");
    }
}

#[test]
fn help_says_what_each_flag_is_for() {
    let run = run_to_end(&home(), &["--help"]);

    let line_of = |flag: &str| run.stdout.lines().find(|line| line.contains(flag));
    for flag in ["--no-watch", "--foreground", "[FILE]"] {
        let line = line_of(flag).unwrap_or_else(|| panic!("no {flag} in {}", run.stdout));
        let explained = line.trim().len() > flag.len() + 8;
        assert!(explained, "{flag} is listed without a word on it: {line}");
    }
}

#[test]
fn what_md_viewer_does_not_accept_is_a_usage_error() {
    let home = home();

    for (args, named) in [
        (vec!["-x"], "-x"),
        (vec!["--watch"], "--watch"),
        (vec!["--no-watch=yes", "a.md"], "--no-watch"),
        (vec!["--foreground=1"], "--foreground"),
        (vec!["a.md", "b.md", "c.md"], "b.md"),
        (vec!["--", "a.md", "b.md"], "b.md"),
        (vec!["a.md", "--", "--no-watch"], "--no-watch"),
    ] {
        let run = run_to_end(&home, &args);

        assert_eq!(run.code, Some(2), "{args:?}: {run:?}");
        assert!(run.stderr.contains(named), "{args:?}: {run:?}");
        assert!(run.stderr.contains("Usage: md-viewer"), "{args:?}: {run:?}");
        assert!(run.stderr.contains("--help"), "{args:?}: {run:?}");
        assert!(run.stdout.is_empty(), "{args:?}: {run:?}");
    }
}

#[test]
fn a_usage_error_leaves_no_settings_behind() {
    let home = home();

    let run = run_to_end(&home, &["--bogus"]);

    assert_eq!(run.code, Some(2));
    assert!(home.saved_settings().is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn a_command_line_that_is_accepted_gets_as_far_as_needing_a_display() {
    let home = home();

    for args in [
        vec!["--foreground"],
        vec!["--foreground", "a.md"],
        vec!["a.md", "--foreground", "--no-watch"],
        vec!["--no-watch", "--foreground", "missing.md"],
        vec!["--foreground", "--", "a.md"],
        vec!["--foreground", "--", "--help"],
        vec!["--foreground", "--", "-dash.md"],
        vec!["--no-detach", "a.md"],
    ] {
        let run = run_to_end(&home, &args);

        assert_eq!(run.code, Some(1), "{args:?}: {run:?}");
        assert!(run.stderr.contains("DISPLAY"), "{args:?}: {run:?}");
        assert!(!run.stderr.contains("Usage:"), "{args:?}: {run:?}");
        assert!(run.stdout.is_empty(), "{args:?}: {run:?}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn started_from_a_script_it_stays_attached_without_being_told_to() {
    let home = home();

    let run = run_to_end(&home, &["a.md"]);

    assert_eq!(run.code, Some(1), "it ran here and failed here: {run:?}");
    assert!(run.stderr.contains("DISPLAY"), "{run:?}");
}
