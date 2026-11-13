use super::*;

#[test]
fn wayland_socket_path_mirrors_libwayland_lookup() {
    assert_eq!(
        wayland_display_socket_path(Some("wayland-0"), OsStr::new("/run/user/1000")),
        Some(PathBuf::from("/run/user/1000/wayland-0"))
    );
    assert_eq!(
        wayland_display_socket_path(Some("/custom/wayland-1"), OsStr::new("/run/user/1000")),
        Some(PathBuf::from("/custom/wayland-1"))
    );
    // Nothing usable to probe when the variables are missing or empty.
    assert_eq!(
        wayland_display_socket_path(None, OsStr::new("/run/user/1000")),
        None
    );
    assert_eq!(
        wayland_display_socket_path(Some(""), OsStr::new("/run/user/1000")),
        None
    );
    assert_eq!(
        wayland_display_socket_path(Some("wayland-0"), OsStr::new("")),
        None
    );
}

#[test]
fn default_terminal_launch_detaches() {
    let args = Args::try_parse_from(["md-viewer", "README.md"]).unwrap();
    assert!(should_detach(&args, true));
}

#[test]
fn non_terminal_launch_does_not_detach() {
    let args = Args::try_parse_from(["md-viewer", "README.md"]).unwrap();
    assert!(!should_detach(&args, false));
}

#[test]
fn foreground_flag_disables_detach() {
    let args = Args::try_parse_from(["md-viewer", "--foreground", "README.md"]).unwrap();
    assert!(!should_detach(&args, true));
}

#[test]
fn hidden_no_detach_marker_disables_detach() {
    let args = Args::try_parse_from(["md-viewer", "--no-detach", "README.md"]).unwrap();
    assert!(!should_detach(&args, true));
}

#[test]
fn child_args_preserve_user_args_and_append_marker() {
    let child_args = child_args_with_no_detach([
        OsString::from("md-viewer"),
        OsString::from("README.md"),
        OsString::from("--no-watch"),
    ]);

    assert_eq!(
        child_args,
        vec![
            OsString::from("README.md"),
            OsString::from("--no-watch"),
            OsString::from("--no-detach"),
        ]
    );
}

#[test]
fn child_args_insert_marker_before_double_dash() {
    let child_args = child_args_with_no_detach([
        OsString::from("md-viewer"),
        OsString::from("--"),
        OsString::from("README.md"),
    ]);

    assert_eq!(
        child_args,
        vec![
            OsString::from("--no-detach"),
            OsString::from("--"),
            OsString::from("README.md"),
        ]
    );
}

#[test]
fn hidden_no_detach_marker_is_not_in_help() {
    use clap::CommandFactory;

    let help = Args::command().render_long_help().to_string();
    assert!(help.contains("--foreground"));
    assert!(!help.contains("--no-detach"));
}

#[test]
fn version_flag_reports_crate_version() {
    use clap::CommandFactory;

    let version = Args::command()
        .get_version()
        .expect("--version should be wired up")
        .to_string();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
}
