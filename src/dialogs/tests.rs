use super::*;
use crate::test_support::ScratchDir;

#[test]
fn native_file_dialog_filters_documents_and_starts_in_the_given_directory() {
    let dir = ScratchDir::new();
    let dialog = format!("{:?}", Picker::File.native_dialog(Some(dir.path())));
    for extension in ["\"md\"", "\"markdown\"", "\"txt\"", "\"*\""] {
        assert!(dialog.contains(extension), "{extension} missing: {dialog}");
    }
    assert!(dialog.contains(&format!("{:?}", dir.path())), "{dialog}");
}

/// A dialog nothing was set on, as rfd describes it.
fn untouched_dialog() -> String {
    format!("{:?}", rfd::FileDialog::new())
}

#[test]
fn native_folder_dialog_has_no_filters_and_no_starting_point_of_its_own() {
    let dialog = format!("{:?}", Picker::Folder.native_dialog(None));
    assert_eq!(dialog, untouched_dialog());
}

#[test]
fn native_dialog_ignores_a_starting_point_that_is_not_a_directory() {
    let dir = ScratchDir::new();
    let file = dir.file("notes.md", "# Notes");
    let dialog = format!("{:?}", Picker::Folder.native_dialog(Some(&file)));
    assert_eq!(dialog, untouched_dialog());

    let in_folder = format!("{:?}", Picker::Folder.native_dialog(Some(dir.path())));
    assert_ne!(in_folder, untouched_dialog());
}

#[cfg(target_os = "linux")]
mod linux {
    use std::cell::RefCell;

    use super::*;

    /// Native dialogs that record what they were asked to show, as rfd
    /// describes it, and answer with a fixed path each.
    struct NativeDialogs {
        file_requests: RefCell<Vec<String>>,
        folder_requests: RefCell<Vec<String>>,
    }

    impl NativeDialogs {
        const PICKED_FILE: &'static str = "/native/picked.md";
        const PICKED_FOLDER: &'static str = "/native/folder";

        const fn new() -> Self {
            Self {
                file_requests: RefCell::new(Vec::new()),
                folder_requests: RefCell::new(Vec::new()),
            }
        }

        fn show_file(&self, dialog: &rfd::FileDialog) -> PathBuf {
            self.file_requests.borrow_mut().push(format!("{dialog:?}"));
            PathBuf::from(Self::PICKED_FILE)
        }

        fn show_folder(&self, dialog: &rfd::FileDialog) -> PathBuf {
            self.folder_requests
                .borrow_mut()
                .push(format!("{dialog:?}"));
            PathBuf::from(Self::PICKED_FOLDER)
        }
    }

    /// `sh -c <script>`, standing in for a program on the desktop. The
    /// arguments the picker adds arrive in the script as `$0`, `$1`, ...
    const fn shell(script: &'static [&'static str; 2]) -> CommandLine<'static> {
        CommandLine {
            program: "sh",
            arguments: script,
        }
    }

    const PORTAL_WITH_FILE_CHOOSER: CommandLine<'static> = shell(&[
        "-c",
        "echo 'interface org.freedesktop.portal.FileChooser {'",
    ]);
    const NO_PORTAL: CommandLine<'static> = shell(&["-c", "exit 1"]);

    /// A Python stand-in that "picks" `picked.md` or `sub` inside the
    /// initial directory, depending on which Tk dialog the source it was
    /// handed would open.
    const TK_PICKS_INSIDE_INITIAL_DIR: CommandLine<'static> = shell(&[
        "-c",
        concat!(
            "case \"$0\" in\n",
            "  *askdirectory*) printf %s \"$1/sub\" ;;\n",
            "  *askopenfilename*) printf %s \"$1/picked.md\" ;;\n",
            "esac",
        ),
    ]);

    fn tk(script: &'static [&'static str; 2], picker: Picker) -> Result<Option<PathBuf>, String> {
        picker.pick_with_tk(shell(script), None)
    }

    #[test]
    fn portal_introspection_detects_file_chooser_interface() {
        let output = b"interface org.freedesktop.portal.FileChooser {";
        assert!(portal_introspection_has_file_chooser(output));
        assert!(!portal_introspection_has_file_chooser(
            b"interface org.freedesktop.portal.OpenURI {"
        ));
    }

    #[test]
    fn portal_is_available_only_when_the_probe_succeeds_and_lists_the_file_chooser() {
        assert!(portal_file_chooser_available(PORTAL_WITH_FILE_CHOOSER));
        assert!(!portal_file_chooser_available(shell(&[
            "-c",
            "echo 'interface org.freedesktop.portal.OpenURI {'"
        ])));
        assert!(!portal_file_chooser_available(shell(&[
            "-c",
            "echo 'interface org.freedesktop.portal.FileChooser {'; exit 1"
        ])));
        assert!(!portal_file_chooser_available(CommandLine {
            program: "md-viewer-no-such-gdbus",
            arguments: &[],
        }));
    }

    #[test]
    fn empty_folder_picker_output_means_cancelled() {
        assert_eq!(path_from_picker_output(Vec::new()), None);
    }

    #[test]
    fn folder_picker_output_preserves_path_bytes() {
        let bytes = b"/tmp/example folder".to_vec();
        assert_eq!(
            path_from_picker_output(bytes),
            Some(PathBuf::from("/tmp/example folder"))
        );
    }

    #[test]
    fn tk_picker_accepts_only_a_selection_of_its_own_kind() {
        let directory = std::env::temp_dir();
        assert_eq!(Picker::Folder.check_selection(&directory), Ok(()));
        assert_eq!(
            Picker::File.check_selection(&directory),
            Err(format!(
                "File picker returned a file that does not exist: {}",
                directory.display()
            ))
        );

        let missing = directory.join("md-viewer-no-such-folder");
        assert_eq!(
            Picker::Folder.check_selection(&missing),
            Err(format!(
                "Folder picker returned a directory that does not exist: {}",
                missing.display()
            ))
        );
    }

    #[test]
    fn tk_picker_returns_the_selection_made_in_the_initial_directory() {
        let dir = ScratchDir::new();
        let file = dir.file("picked.md", "# Picked");
        let folder = dir.dir("sub");

        assert_eq!(
            Picker::File.pick_with_tk(TK_PICKS_INSIDE_INITIAL_DIR, Some(dir.path())),
            Ok(Some(file))
        );
        assert_eq!(
            Picker::Folder.pick_with_tk(TK_PICKS_INSIDE_INITIAL_DIR, Some(dir.path())),
            Ok(Some(folder))
        );
    }

    #[test]
    fn tk_picker_is_not_pointed_at_a_starting_point_that_is_not_a_directory() {
        let dir = ScratchDir::new();
        let file = dir.file("notes.md", "# Notes");
        // Echoes its first argument: handed the file, it would "pick" it.
        let echo_initial_dir = shell(&["-c", "printf %s \"${1-}\""]);
        assert_eq!(
            Picker::File.pick_with_tk(echo_initial_dir, Some(&file)),
            Ok(None)
        );
    }

    #[test]
    fn tk_picker_without_output_was_cancelled() {
        assert_eq!(tk(&["-c", "exit 0"], Picker::File), Ok(None));
    }

    #[test]
    fn tk_picker_failure_carries_what_python_printed() {
        let error = tk(&["-c", "echo 'no display name' >&2; exit 1"], Picker::File).unwrap_err();
        assert_eq!(error, "Tkinter file picker failed: no display name");

        let silent = tk(&["-c", "exit 3"], Picker::Folder).unwrap_err();
        assert_eq!(silent, "Tkinter folder picker exited with an error");
    }

    #[test]
    fn tk_picker_reports_a_missing_interpreter() {
        let no_python = CommandLine {
            program: "md-viewer-no-such-python",
            arguments: &[],
        };
        let error = Picker::File.pick_with_tk(no_python, None).unwrap_err();
        assert!(
            error.starts_with("Tkinter file picker could not start"),
            "{error}"
        );
    }

    #[test]
    fn tk_picker_rejects_a_selection_that_is_not_on_disk() {
        let error = tk(&["-c", "printf %s /md-viewer/ghost.md"], Picker::File).unwrap_err();
        assert_eq!(
            error,
            "File picker returned a file that does not exist: /md-viewer/ghost.md"
        );
    }

    #[test]
    fn portal_desktop_gets_the_native_dialog_of_the_picked_kind() {
        let dir = ScratchDir::new();
        let native = NativeDialogs::new();
        let desktop = Desktop {
            portal_probe: PORTAL_WITH_FILE_CHOOSER,
            python: NO_PORTAL,
            show_file_dialog: &|dialog| Some(native.show_file(&dialog)),
            show_folder_dialog: &|dialog| Some(native.show_folder(&dialog)),
        };

        assert_eq!(
            Picker::File.pick(Some(dir.path()), &desktop),
            Ok(Some(PathBuf::from(NativeDialogs::PICKED_FILE)))
        );
        assert_eq!(
            Picker::Folder.pick(None, &desktop),
            Ok(Some(PathBuf::from(NativeDialogs::PICKED_FOLDER)))
        );

        let file_requests = native.file_requests.borrow();
        assert_eq!(file_requests.len(), 1);
        assert!(file_requests[0].contains(&format!("{:?}", dir.path())));
        assert_eq!(native.folder_requests.borrow().len(), 1);
    }

    #[test]
    fn desktop_without_portal_falls_back_to_the_tk_picker() {
        crate::test_support::evaluate_log_arguments();
        let dir = ScratchDir::new();
        let file = dir.file("picked.md", "# Picked");
        let native = NativeDialogs::new();
        let desktop = Desktop {
            portal_probe: NO_PORTAL,
            python: TK_PICKS_INSIDE_INITIAL_DIR,
            show_file_dialog: &|dialog| Some(native.show_file(&dialog)),
            show_folder_dialog: &|dialog| Some(native.show_folder(&dialog)),
        };

        assert_eq!(
            Picker::File.pick(Some(dir.path()), &desktop),
            Ok(Some(file))
        );
        assert!(native.file_requests.borrow().is_empty());
    }
}
