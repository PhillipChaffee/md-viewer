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

#[test]
fn live_reload_is_on_unless_no_watch_is_given() {
    let default = Args::try_parse_from(["md-viewer", "README.md"]).unwrap();
    assert!(default.watch());
    let no_watch = Args::try_parse_from(["md-viewer", "--no-watch", "README.md"]).unwrap();
    assert!(!no_watch.watch());
}

#[cfg(unix)]
mod terminal {
    use std::fs::{File, OpenOptions};

    use super::*;

    fn not_a_terminal() -> File {
        File::open("/dev/null").unwrap()
    }

    /// The controlling end of a fresh pseudo-terminal, which is a terminal
    /// without any terminal emulator attached to the test run.
    fn terminal() -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/ptmx")
            .unwrap()
    }

    #[test]
    fn redirected_streams_are_not_a_terminal_launch() {
        let (stdin, stdout, stderr) = (not_a_terminal(), not_a_terminal(), not_a_terminal());
        assert!(!launched_from_terminal([&stdin, &stdout, &stderr]));
    }

    #[test]
    fn one_terminal_stream_makes_it_a_terminal_launch() {
        let (plain, tty) = (not_a_terminal(), terminal());
        assert!(launched_from_terminal([&tty, &plain, &plain]));
        assert!(launched_from_terminal([&plain, &tty, &plain]));
        assert!(launched_from_terminal([&plain, &plain, &tty]));
    }
}

mod detach {
    use std::cell::Cell;

    use super::*;

    fn args(cli: &[&str]) -> Args {
        Args::try_parse_from(cli).unwrap()
    }

    /// What `launch` did: whether it spawned a copy, whether it opened the
    /// window in this process, and what it returned.
    struct Launched {
        spawned: bool,
        opened_window: bool,
        result: eframe::Result<()>,
    }

    fn launch_recording(
        cli: &[&str],
        from_terminal: bool,
        spawn_result: fn() -> io::Result<()>,
    ) -> Launched {
        let (spawned, opened_window) = (Cell::new(false), Cell::new(false));
        let result = launch(
            &args(cli),
            from_terminal,
            &|| {
                spawned.set(true);
                spawn_result()
            },
            &|_| {
                opened_window.set(true);
                Ok(())
            },
        );
        Launched {
            spawned: spawned.get(),
            opened_window: opened_window.get(),
            result,
        }
    }

    #[test]
    fn terminal_launch_hands_over_to_the_copy_and_opens_no_window() {
        let launched = launch_recording(&["md-viewer", "README.md"], true, || Ok(()));
        assert!(launched.spawned);
        assert!(!launched.opened_window);
        assert!(launched.result.is_ok());
    }

    #[test]
    fn failed_hand_over_opens_the_window_in_this_process() {
        crate::test_support::evaluate_log_arguments();
        let launched = launch_recording(&["md-viewer", "README.md"], true, || {
            Err(io::Error::other("no more processes"))
        });
        assert!(launched.spawned);
        assert!(launched.opened_window);
        assert!(launched.result.is_ok());
    }

    #[test]
    fn launcher_and_foreground_launches_open_the_window_without_spawning() {
        for (cli, from_terminal) in [
            (["md-viewer", "README.md"].as_slice(), false),
            (["md-viewer", "--foreground", "README.md"].as_slice(), true),
            (["md-viewer", "--no-detach", "README.md"].as_slice(), true),
        ] {
            let launched = launch_recording(cli, from_terminal, || Ok(()));
            assert!(!launched.spawned, "{cli:?} must not respawn");
            assert!(launched.opened_window, "{cli:?} must open the window");
        }
    }

    #[test]
    fn window_failure_is_what_launch_returns() {
        let result = launch(
            &args(&["md-viewer", "--foreground"]),
            true,
            &|| Ok(()),
            &|_| Err(eframe::Error::AppCreation("no display".into())),
        );
        assert!(matches!(result, Err(eframe::Error::AppCreation(_))));
    }

    #[test]
    fn spawning_a_missing_executable_reports_the_error() {
        let missing = std::env::temp_dir().join("md-viewer-no-such-executable");
        let error = spawn_detached(missing, vec![OsString::from("md-viewer")]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    /// Runs a shell script as the "copy" and reads back what the script saw
    /// of itself: its arguments, its process group, and its standard streams.
    #[cfg(target_os = "linux")]
    #[test]
    fn detached_copy_gets_the_marker_its_own_process_group_and_no_streams() {
        use crate::test_support::{wait_until, ScratchDir};
        use std::time::Duration;

        let dir = ScratchDir::new();
        let report = dir.path().join("report");
        let script = dir.file(
            "copy.sh",
            concat!(
                "read -r pid _ _ _ group _ < /proc/$$/stat\n",
                "streams=$(for fd in 0 1 2; do readlink \"/proc/$$/fd/$fd\"; done)\n",
                "{\n",
                "  printf 'args=%s\\n' \"$*\"\n",
                "  [ \"$pid\" = \"$group\" ] && printf 'leads-own-group\\n'\n",
                "  printf '%s\\n' \"$streams\"\n",
                "} > \"$1.partial\"\n",
                "mv \"$1.partial\" \"$1\"\n",
            ),
        );

        spawn_detached(
            PathBuf::from("/bin/sh"),
            vec![
                OsString::from("md-viewer"),
                script.into_os_string(),
                report.clone().into_os_string(),
                OsString::from("README.md"),
            ],
        )
        .unwrap();

        assert!(wait_until(Duration::from_secs(20), || report.is_file()));
        let seen = std::fs::read_to_string(&report).unwrap();
        let expected = format!(
            "args={} README.md --no-detach\nleads-own-group\n/dev/null\n/dev/null\n/dev/null\n",
            report.display()
        );
        assert_eq!(seen, expected);
    }
}

mod window {
    use std::time::Duration;

    use egui_kittest::kittest::Queryable as _;

    use super::*;
    use crate::test_support::{wait_until, ScratchDir};

    #[test]
    fn window_fits_the_reading_column_and_both_sidebars() {
        let viewport = native_options().viewport;
        let width = CONTENT_OPTIMAL_WIDTH
            + EXPLORER_DEFAULT_WIDTH
            + OUTLINE_DEFAULT_WIDTH
            + PANEL_SEPARATORS;
        assert_eq!(
            viewport.inner_size,
            Some(egui::vec2(width, OPTIMAL_WINDOW_HEIGHT))
        );
        assert_eq!(viewport.min_inner_size, Some(egui::vec2(400.0, 300.0)));
        assert_eq!(viewport.title.as_deref(), Some("Markdown Viewer"));
        assert_eq!(viewport.drag_and_drop, Some(true));
    }

    /// GNOME pairs a running window with its launcher icon only when the
    /// window's app id equals the launcher's `StartupWMClass` (#62).
    #[test]
    fn window_app_id_matches_the_desktop_launcher() {
        let launcher = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/data/md-viewer.desktop"
        ));
        let class = launcher
            .lines()
            .find_map(|line| line.strip_prefix("StartupWMClass="))
            .expect("launcher declares StartupWMClass");
        assert_eq!(native_options().viewport.app_id.as_deref(), Some(class));
    }

    /// The boxed app `app_creator` hands eframe, as a type of its own for
    /// the test harness.
    struct Launched(Box<dyn eframe::App>);

    impl eframe::App for Launched {
        fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
            self.0.update(ctx, frame);
        }
    }

    /// The app eframe would run for a command line, driven without a window.
    struct Window(egui_kittest::Harness<'static, Launched>);

    impl Window {
        fn open(cli: &[&OsStr]) -> Self {
            let args = Args::try_parse_from(cli).unwrap();
            let harness = egui_kittest::Harness::builder()
                .with_size(egui::vec2(1200.0, 800.0))
                .build_eframe(|creation| {
                    Launched(app_creator(&args)(creation).expect("the app starts"))
                });
            Self(harness)
        }

        /// Run one frame and report whether a label on it reads `text`.
        fn shows(&mut self, text: &str) -> bool {
            self.0.step();
            self.0.query_all_by_label(text).next().is_some()
        }

        fn eventually_shows(&mut self, text: &str, timeout: Duration) -> bool {
            wait_until(timeout, || self.shows(text))
        }
    }

    #[test]
    fn window_shows_the_named_file_and_follows_its_changes() {
        let dir = ScratchDir::new();
        let doc = dir.file("notes.md", "# First draft\n");
        let mut window = Window::open(&[OsStr::new("md-viewer"), doc.as_os_str()]);
        assert!(window.eventually_shows("First draft", Duration::from_secs(20)));

        std::fs::write(&doc, "# Second draft\n").unwrap();
        assert!(window.eventually_shows("Second draft", Duration::from_secs(20)));
    }

    #[test]
    fn no_watch_window_keeps_the_text_it_loaded() {
        let dir = ScratchDir::new();
        let doc = dir.file("notes.md", "# First draft\n");
        let mut window = Window::open(&[
            OsStr::new("md-viewer"),
            OsStr::new("--no-watch"),
            doc.as_os_str(),
        ]);
        assert!(window.eventually_shows("First draft", Duration::from_secs(20)));

        std::fs::write(&doc, "# Second draft\n").unwrap();
        // Longer than the watcher's debounce, so a reload would have landed.
        assert!(!window.eventually_shows("Second draft", Duration::from_millis(800)));
        assert!(window.shows("First draft"));
    }
}

#[cfg(unix)]
mod wayland {
    use std::collections::HashMap;
    use std::os::unix::net::UnixListener;

    use super::*;
    use crate::test_support::ScratchDir;

    /// The variables to clear for a made-up environment.
    fn cleared(environment: &[(&str, &OsStr)]) -> &'static [&'static str] {
        let environment: HashMap<&str, &OsStr> = environment.iter().copied().collect();
        dead_wayland_variables(&|name| environment.get(name).map(|value| value.to_os_string()))
    }

    #[test]
    fn dead_wayland_display_is_cleared_so_x11_is_picked() {
        crate::test_support::evaluate_log_arguments();
        let dir = ScratchDir::new();
        assert_eq!(
            cleared(&[
                ("WAYLAND_DISPLAY", OsStr::new("wayland-0")),
                ("XDG_RUNTIME_DIR", dir.path().as_os_str()),
                ("WAYLAND_SOCKET", OsStr::new("")),
            ]),
            ["WAYLAND_DISPLAY", "WAYLAND_SOCKET"]
        );
    }

    #[test]
    fn listening_wayland_display_is_kept() {
        let dir = ScratchDir::new();
        let _compositor = UnixListener::bind(dir.path().join("wayland-0")).unwrap();
        assert!(cleared(&[
            ("WAYLAND_DISPLAY", OsStr::new("wayland-0")),
            ("XDG_RUNTIME_DIR", dir.path().as_os_str()),
        ])
        .is_empty());
    }

    #[test]
    fn inherited_wayland_socket_is_trusted_without_probing() {
        let dir = ScratchDir::new();
        assert!(cleared(&[
            ("WAYLAND_SOCKET", OsStr::new("5")),
            ("WAYLAND_DISPLAY", OsStr::new("wayland-0")),
            ("XDG_RUNTIME_DIR", dir.path().as_os_str()),
        ])
        .is_empty());
    }

    #[test]
    fn nothing_is_cleared_when_wayland_was_never_configured() {
        assert!(cleared(&[("XDG_RUNTIME_DIR", OsStr::new("/run/user/1000"))]).is_empty());
        assert!(cleared(&[("WAYLAND_DISPLAY", OsStr::new(""))]).is_empty());
    }
}
