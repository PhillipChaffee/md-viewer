use super::*;

#[cfg(target_os = "linux")]
#[test]
fn portal_introspection_detects_file_chooser_interface() {
    let output = b"interface org.freedesktop.portal.FileChooser {";
    assert!(portal_introspection_has_file_chooser(output));
    assert!(!portal_introspection_has_file_chooser(
        b"interface org.freedesktop.portal.OpenURI {"
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn empty_folder_picker_output_means_cancelled() {
    assert_eq!(path_from_picker_output(Vec::new()), None);
}

#[cfg(target_os = "linux")]
#[test]
fn folder_picker_output_preserves_path_bytes() {
    let bytes = b"/tmp/example folder".to_vec();
    assert_eq!(
        path_from_picker_output(bytes),
        Some(PathBuf::from("/tmp/example folder"))
    );
}

#[cfg(target_os = "linux")]
#[test]
fn tk_picker_accepts_only_a_selection_of_its_own_kind() {
    let directory = std::env::temp_dir();
    assert_eq!(TkPicker::Folder.check_selection(&directory), Ok(()));
    assert_eq!(
        TkPicker::File.check_selection(&directory),
        Err(format!(
            "File picker returned a file that does not exist: {}",
            directory.display()
        ))
    );

    let missing = directory.join("md-viewer-no-such-folder");
    assert_eq!(
        TkPicker::Folder.check_selection(&missing),
        Err(format!(
            "Folder picker returned a directory that does not exist: {}",
            missing.display()
        ))
    );
}
