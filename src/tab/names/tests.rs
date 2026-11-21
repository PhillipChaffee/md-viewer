use super::*;

/// What `documents`, each a name and a path, are shown as.
fn shown(documents: &[(&str, &str)]) -> Vec<String> {
    let documents: Vec<(&str, &Path)> = documents
        .iter()
        .map(|(name, path)| (*name, Path::new(path)))
        .collect();
    let names = shown_names(&documents).into_iter();
    names.map(|name| name.as_str().to_owned()).collect()
}

#[test]
fn names_no_other_document_has_are_shown_alone() {
    let names = shown(&[
        ("Notes", "/a/guides/notes.md"),
        ("Plan", "/a/guides/plan.md"),
    ]);

    assert_eq!(names, ["Notes", "Plan"]);
    assert_eq!(shown(&[]), [""; 0]);
}

#[test]
fn documents_of_one_name_show_their_folders() {
    let names = shown(&[
        ("Notes", "/a/guides/notes.md"),
        ("Plan", "/a/guides/plan.md"),
        ("Notes", "/a/drafts/notes.md"),
        ("Notes", "/a/archive/notes.md"),
    ]);

    assert_eq!(
        names,
        [
            "Notes · guides",
            "Plan",
            "Notes · drafts",
            "Notes · archive"
        ]
    );
}

#[test]
fn folders_of_one_name_show_the_folders_above_them() {
    let names = shown(&[
        ("Notes", "/x/a/guides/notes.md"),
        ("Notes", "/y/a/guides/notes.md"),
        ("Notes", "/x/b/guides/notes.md"),
        ("Notes", "/x/drafts/notes.md"),
    ]);

    assert_eq!(
        names,
        [
            "Notes · x/a/guides",
            "Notes · y/a/guides",
            "Notes · b/guides",
            "Notes · drafts"
        ]
    );
}

#[test]
fn the_folders_stop_at_the_root() {
    let names = shown(&[
        ("Notes", "/guides/notes.md"),
        ("Notes", "/a/guides/notes.md"),
    ]);

    assert_eq!(names, ["Notes · /guides", "Notes · a/guides"]);
}

#[test]
fn namesakes_in_one_folder_show_their_files() {
    let names = shown(&[
        ("Notes", "/a/guides/monday.md"),
        ("Notes", "/a/guides/tuesday.md"),
        ("Notes", "/a/drafts/monday.md"),
    ]);

    assert_eq!(
        names,
        [
            "Notes · guides/monday.md",
            "Notes · guides/tuesday.md",
            "Notes · drafts"
        ]
    );
}

#[test]
fn one_document_open_twice_is_shown_the_same_both_times() {
    let twice = [("Notes", "/a/guides/notes.md"); 2];
    assert_eq!(shown(&twice), ["Notes", "Notes"]);

    let beside_another = [twice[0], twice[1], ("Notes", "/a/drafts/notes.md")];
    assert_eq!(
        shown(&beside_another),
        ["Notes · guides", "Notes · guides", "Notes · drafts"]
    );
}

#[test]
fn a_document_with_no_folder_shows_its_path() {
    let names = shown(&[("Notes", "notes.md"), ("Notes", "guides/notes.md")]);

    assert_eq!(names, ["Notes · notes.md", "Notes · guides"]);
}

#[test]
fn a_name_that_reads_as_another_with_its_folder_is_told_apart_too() {
    let names = shown(&[
        ("Notes · guides", "/a/misc/odd.md"),
        ("Notes", "/a/guides/notes.md"),
        ("Notes", "/a/drafts/notes.md"),
    ]);

    assert_eq!(
        names,
        [
            "Notes · guides · misc",
            "Notes · a/guides",
            "Notes · drafts"
        ]
    );
}

#[test]
fn a_file_and_a_folder_of_one_name_are_told_apart_further_up() {
    // `c` is a file in the first folder `b` and a folder in the second,
    // so both documents read `b/c` on the way.
    let names = shown(&[
        ("N", "/q/b/c"),
        ("N", "/q/b/d"),
        ("N", "/z/b/c/f.md"),
        ("N", "/w/x/c/g.md"),
    ]);

    assert_eq!(names, ["N · q/b/c", "N · b/d", "N · z/b/c", "N · x/c"]);
}

#[test]
fn a_shown_name_is_its_name_and_what_follows_it() {
    let documents = [
        ("Notes", Path::new("/a/guides/notes.md")),
        ("Notes", Path::new("/a/drafts/notes.md")),
        ("Plan", Path::new("/a/plan.md")),
    ];

    let names = shown_names(&documents);
    let parts: Vec<(&str, &str)> = names.iter().map(ShownName::parts).collect();

    assert_eq!(
        parts,
        [("Notes", " · guides"), ("Notes", " · drafts"), ("Plan", "")]
    );
    assert_eq!(ShownName::plain("Plan").parts(), ("Plan", ""));
    assert_eq!(ShownName::default().parts(), ("", ""));
}

/// The names and paths the property below draws its documents from:
/// namesakes, folders of one name at several depths, files in one folder,
/// a path that is a prefix of another, and names that hold the separator.
const NAMES: [&str; 4] = ["Notes", "Plan", "Notes · guides", "Notes · a/guides"];
const PATHS: [&str; 12] = [
    "/a/guides/notes.md",
    "/b/guides/notes.md",
    "/a/guides/plan.md",
    "/a/drafts/notes.md",
    "/x/a/guides/notes.md",
    "/guides/notes.md",
    "/notes.md",
    "notes.md",
    "guides/notes.md",
    "/a/guides",
    "/a/guides/notes.md/deep.md",
    "/a/b/../guides/notes.md",
];

/// The next number of a fixed sequence that visits every residue: enough
/// to draw documents without a dependency, and the same on every run.
fn step(state: &mut usize) -> usize {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

/// Up to six documents drawn from `NAMES` and `PATHS`.
fn drawn(state: &mut usize) -> Vec<(&'static str, &'static Path)> {
    let count = 1 + step(state) % 6;
    let pick = |state: &mut usize| {
        let name = NAMES.get(step(state) % NAMES.len()).copied();
        let path = PATHS.get(step(state) % PATHS.len()).copied();
        (
            name.unwrap_or_default(),
            Path::new(path.unwrap_or_default()),
        )
    };
    (0..count).map(|_| pick(state)).collect()
}

#[test]
fn documents_at_different_paths_are_never_shown_the_same() {
    let mut state = 7;
    for _ in 0..20_000 {
        let documents = drawn(&mut state);

        let names = shown_names(&documents);

        assert_eq!(names.len(), documents.len());
        let shown = documents.iter().zip(&names);
        for ((name, path), shown_as) in shown.clone() {
            assert_eq!(shown_as.parts().0, *name, "{documents:?}");
            for ((other, at), other_as) in shown.clone() {
                let told_apart = shown_as != other_as;
                assert!(told_apart || path == at, "{documents:?} as {names:?}");
                assert!(!told_apart || (name, path) != (other, at), "{documents:?}");
            }
        }
    }
}

#[test]
fn what_a_document_is_shown_as_does_not_depend_on_the_order_of_the_tabs() {
    let mut state = 11;
    for _ in 0..2_000 {
        let documents = drawn(&mut state);
        let mut reversed = documents.clone();
        reversed.reverse();

        let mut backwards = shown_names(&reversed);
        backwards.reverse();

        assert_eq!(shown_names(&documents), backwards, "{documents:?}");
    }
}

#[test]
fn a_name_is_shown_alone_while_no_other_document_has_it() {
    let mut state = 13;
    for _ in 0..2_000 {
        let documents = drawn(&mut state);

        let names = shown_names(&documents);

        for ((name, path), shown_as) in documents.iter().zip(&names) {
            let namesake = |(other, at): &(&str, &Path)| other == name && at != path;
            // A name that holds the separator can read as another name
            // with its folder, and is then told apart as a namesake is.
            if !name.contains(SEPARATOR) && !documents.iter().any(namesake) {
                assert_eq!(shown_as.as_str(), *name, "{documents:?}");
            }
        }
    }
}
