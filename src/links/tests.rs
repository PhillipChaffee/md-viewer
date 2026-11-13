use super::*;

use std::fs;

use crate::recent::now_epoch_secs;

/// Characterization table for issue #141: one place that says what a
/// Markdown destination resolves to.
///
/// #188 landed this as a *baseline*, with rows marked `#141-N` recording
/// behavior that contradicted the semantics the issue proposes. Those rows
/// are the ones this change flips, and the markers now name what was
/// decided rather than what was merely observed.
#[test]
#[cfg(unix)]
fn local_destination_resolution_is_characterized_for_every_uri_shape() {
    // (destination, is_local_markdown_link, resolve_local_link_path)
    // Err(()) = reports rather than resolving; Ok(None) = nothing to open.
    // One row: the destination, whether the gate accepts it, and what the
    // resolver answers.
    type Row<'a> = (&'a str, bool, Result<Option<&'a str>, ()>);
    let dir = Path::new("/base/doc");
    let table: &[Row<'_>] = &[
        // 1. relative destinations resolve against the containing document
        ("guide.md", true, Ok(Some("/base/doc/guide.md"))),
        ("sub/guide.md", true, Ok(Some("/base/doc/sub/guide.md"))),
        ("../up.md", true, Ok(Some("/base/doc/../up.md"))),
        // 2. a fragment selects the document; the anchor is dropped
        ("guide.md#section", true, Ok(Some("/base/doc/guide.md"))),
        // #141-2 DECIDED: a query string is stripped like a fragment, and
        // the gate agrees because it tests the extension of the stripped
        // spelling. Before, the gate rejected `md?v=2` while the resolver
        // put `?v=2` into the file name.
        ("guide.md?v=2", true, Ok(Some("/base/doc/guide.md"))),
        ("guide.md?v=2#s", true, Ok(Some("/base/doc/guide.md"))),
        // 3. percent escapes are decoded exactly once
        ("a%20b.md", true, Ok(Some("/base/doc/a b.md"))),
        ("100%25.md", true, Ok(Some("/base/doc/100%.md"))),
        ("bad%zz.md", true, Ok(Some("/base/doc/bad%zz.md"))),
        // an unencoded `#` delimits a fragment, `%23` is a literal
        // character — a relative destination and a file:// URI agree on
        // both, which RFC 3986 requires
        ("a%23b.md", true, Ok(Some("/base/doc/a#b.md"))),
        (
            "file:///base/doc/a%23b.md",
            true,
            Ok(Some("/base/doc/a#b.md")),
        ),
        // `a#b.md` names the file `a` with fragment `b.md`, so it is
        // correctly not a Markdown link and correctly resolves to `a`
        ("a#b.md", false, Ok(Some("/base/doc/a"))),
        // #141-5 DECIDED: a destination that does not decode to UTF-8 is
        // reported instead of resolving to nothing. It still passes the
        // gate, because passing is what makes the error reachable.
        ("bad%FF.md", true, Err(())),
        // 4. platform paths
        ("/abs/guide.md", true, Ok(Some("/abs/guide.md"))),
        ("file:///tmp/guide.md", true, Ok(Some("/tmp/guide.md"))),
        (
            "file://localhost/tmp/guide.md",
            true,
            Ok(Some("/tmp/guide.md")),
        ),
        ("file://server/share/guide.md", false, Ok(None)),
        ("C:/tmp/guide.md", false, Ok(None)),
        // 8. other schemes stay external and are never reinterpreted
        ("https://example.com/guide.md", false, Ok(None)),
        ("http://example.com/guide.md", false, Ok(None)),
        ("data:text/plain,x", false, Ok(None)),
        ("mailto:a@b.c", false, Ok(None)),
        ("weird+scheme:guide.md", false, Ok(None)),
        // #141-8 DECIDED: a protocol-relative URL is external. It carries
        // no scheme, so it used to be read as an absolute filesystem path
        // and was even classified as a local Markdown link.
        ("//evil.com/guide.md", false, Ok(None)),
        // #141 DECIDED: nothing left to name a file with is external,
        // rather than resolving to the containing directory.
        ("#section", false, Ok(None)),
        ("", false, Ok(None)),
        ("?v=2", false, Ok(None)),
    ];

    for (destination, expect_local, expect_resolved) in table {
        assert_eq!(
            is_local_markdown_link(destination),
            *expect_local,
            "is_local_markdown_link({destination:?})"
        );
        let resolved = resolve_local_link_path(destination, dir)
            .map(|path| path.map(|p| p.display().to_string()))
            .map_err(|_| ());
        assert_eq!(
            resolved,
            expect_resolved.map(|p| p.map(str::to_owned)),
            "resolve_local_link_path({destination:?})"
        );
    }
}

/// The invariant that closes the dead-click class: whatever the gate lets
/// through must be something the resolver either turns into a path or
/// complains about. It must never answer "nothing to open", because such a
/// destination renders as an internal link and then does nothing at all
/// when clicked — the state #188 pinned for `bad%FF.md`.
///
/// This replaces #188's `…disagree_on_four_shapes`, which asserted the two
/// functions *did* disagree and told its reader to drop the row once #141
/// decided. They now share one `classify_destination`, so the disagreement
/// is unrepresentable rather than merely fixed.
#[test]
#[cfg(unix)]
fn nothing_the_gate_accepts_can_resolve_to_silence() {
    let dir = Path::new("/base/doc");
    for destination in [
        "guide.md",
        "guide.md?v=2",
        "guide.md#section",
        "a%20b.md",
        "100%25.md",
        "bad%zz.md",
        "bad%FF.md",
        "/abs/guide.md",
        "file:///tmp/guide.md",
        "file://localhost/tmp/guide.md",
        "a%23b.md",
        "",
        "#section",
        "//evil.com/guide.md",
    ] {
        if !is_local_markdown_link(destination) {
            continue;
        }
        match resolve_local_link_path(destination, dir) {
            Ok(Some(_)) | Err(_) => {}
            Ok(None) => panic!(
                "{destination:?} passes the gate but resolves to nothing: it \
                     would render as an internal link and do nothing on click"
            ),
        }
    }
}

#[test]
fn shared_resolver_keeps_absolute_paths_and_literal_percent_names() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-141-edges-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(&root).unwrap();
    let home = root.join("home.md");
    let abs = root.join("absolute.md");
    // `%` not followed by two hex digits is not an escape and must survive.
    let literal = root.join("100%done.md");
    fs::write(&home, "# Home").unwrap();
    fs::write(&abs, "# Abs").unwrap();
    fs::write(&literal, "# Literal").unwrap();

    let mut tab = Tab::new(home).unwrap();

    let abs_link = abs.to_string_lossy().to_string();
    assert!(tab.navigate_to_link(&abs_link).unwrap(), "absolute path");
    assert_eq!(tab.path, abs.canonicalize().unwrap());

    assert!(
        tab.navigate_to_link("100%done.md").unwrap(),
        "literal percent"
    );
    assert_eq!(tab.path, literal.canonicalize().unwrap());

    // an anchor-only destination is still not a path
    assert!(!tab.navigate_to_link("#section").unwrap());

    fs::remove_dir_all(&root).ok();
}

#[test]
fn plain_click_and_ctrl_click_resolve_the_same_link_identically() {
    // #141: `resolve_link` (ctrl+click) and `navigate_to_link` (plain click)
    // must resolve a destination identically. #78 taught percent-decoding
    // and `file://` to the former only, so an encoded space opened in a new
    // tab on ctrl+click and silently did nothing on a plain click.
    let root = std::env::temp_dir().join(format!(
        "md-viewer-141-baseline-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    let spaced = root.join("docs with spaces");
    fs::create_dir_all(&spaced).unwrap();
    let home = root.join("home.md");
    let guide = spaced.join("guide.md");
    fs::write(&home, "# Home").unwrap();
    fs::write(&guide, "# Guide").unwrap();

    let mut tab = Tab::new(home.clone()).unwrap();
    let encoded = "docs%20with%20spaces/guide.md";

    // ctrl+click path: decodes, resolves
    assert_eq!(
        tab.resolve_link(encoded).unwrap(),
        Some(guide.canonicalize().unwrap()),
        "resolve_link should decode percent escapes"
    );

    // plain-click path must reach the same file
    assert!(
        tab.navigate_to_link(encoded).unwrap(),
        "navigate_to_link must decode percent escapes too"
    );
    assert_eq!(tab.path, guide.canonicalize().unwrap());
    assert!(tab.navigate_to_path(&home).unwrap());

    // same divergence for file:// destinations
    let uri = format!("file://{}", guide.canonicalize().unwrap().display());
    assert_eq!(
        tab.resolve_link(&uri).unwrap(),
        Some(guide.canonicalize().unwrap()),
        "resolve_link should accept file:// URIs"
    );
    assert!(
        tab.navigate_to_link(&uri).unwrap(),
        "navigate_to_link must accept file:// URIs too"
    );
    assert_eq!(tab.path, guide.canonicalize().unwrap());

    fs::remove_dir_all(&root).ok();
}

#[test]
fn slugify_heading_matches_github_anchor_conventions() {
    let cases = [
        ("Hello World", "hello-world"),
        ("Settings & Config", "settings--config"),
        ("C++ & Rust?", "c--rust"),
        ("42. The Meta", "42-the-meta"),
        // Each space becomes its own hyphen; existing hyphens are kept.
        ("A -- B_c", "a----b_c"),
        ("Ünïcode Tïtle", "ünïcode-tïtle"),
    ];
    for (title, expected) in cases {
        assert_eq!(
            slugify_heading(title),
            expected,
            "slug of {title:?} must match the GitHub anchor convention"
        );
    }
}

#[test]
fn find_fragment_target_prefers_document_title_then_outline_order() {
    let headers = vec![
        Header {
            level: 2,
            title: "Installation".to_owned(),
            source_start: 10,
            line_number: 4,
        },
        Header {
            level: 2,
            title: "Settings & Config".to_owned(),
            source_start: 90,
            line_number: 20,
        },
    ];

    // Slug semantics: case, punctuation and spacing differences all
    // collapse to the same anchor.
    assert!(matches!(
        find_fragment_target(Some("My Manual"), &headers, "installation"),
        FragmentTarget::Header(_)
    ));
    assert!(matches!(
        find_fragment_target(Some("My Manual"), &headers, "settings--config"),
        FragmentTarget::Header(_)
    ));

    // The first h1 is excluded from the outline but is still the page's
    // primary anchor; it navigates to the top.
    assert!(matches!(
        find_fragment_target(Some("My Manual"), &headers, "my-manual"),
        FragmentTarget::DocumentTop
    ));

    // An unmatched or empty fragment is a no-op, not an error.
    assert!(matches!(
        find_fragment_target(Some("My Manual"), &headers, "no-such-section"),
        FragmentTarget::Missing
    ));
    assert!(matches!(
        find_fragment_target(Some("My Manual"), &headers, "#"),
        FragmentTarget::Missing
    ));
}

#[test]
fn same_document_fragment_scrolls_without_touching_history() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-141-same-doc-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(&root).unwrap();
    let document = root.join("single.md");
    fs::write(&document, "# Title\n\ntext\n\n## A Section\n\ntext\n").unwrap();

    let mut tab = Tab::new(document).unwrap();
    let original_path = tab.path.clone();

    assert!(
        tab.navigate_to_link("#a-section").unwrap(),
        "a matching same-document fragment must navigate"
    );
    assert_eq!(
        tab.path, original_path,
        "same-document navigation stays put"
    );
    assert!(tab.history_back.is_empty(), "no history entry for a scroll");
    assert!(
        tab.pending_header_click_key.is_some() || tab.pending_scroll_offset.is_some(),
        "the click must schedule the scroll"
    );

    // An unmatched fragment does nothing at all — no navigation, and the
    // scroll scheduled by the earlier match is left alone.
    let key_before = tab.pending_header_click_key.clone();
    assert!(!tab.navigate_to_link("#no-such-anchor").unwrap());
    assert_eq!(tab.pending_header_click_key, key_before);
    assert_eq!(tab.path, original_path);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn cross_document_fragment_opens_and_lands_on_heading() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-141-cross-doc-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(&root).unwrap();
    let home = root.join("home.md");
    let guide = root.join("guide.md");
    fs::write(&home, "# Home\n\n[Setup](guide.md#setup)\n").unwrap();
    fs::write(&guide, "# Guide\n\nintro\n\n## Setup\n\nsteps\n").unwrap();

    let mut tab = Tab::new(home).unwrap();
    assert!(tab.navigate_to_link("guide.md#setup").unwrap());
    assert_eq!(tab.path, guide.canonicalize().unwrap());
    assert!(
        tab.pending_header_click_key.is_some(),
        "a fresh document has no cached heading positions, so the \
             corrective pass must be armed to land on the heading"
    );

    // ctrl+click carries the fragment through to the new-tab request.
    let (new_tab, error_message) = handle_local_link_click(&mut tab, "guide.md#setup", true);
    assert!(error_message.is_none());
    let (path, fragment) = new_tab.expect("ctrl+click resolves to a path");
    assert_eq!(path, guide.canonicalize().unwrap());
    assert_eq!(fragment.as_deref(), Some("setup"));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn bare_existing_markdown_path_is_registered_as_local_link() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-auto-link-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(root.join("docs")).unwrap();
    let document = root.join("index.md");
    fs::write(&document, "See docs/guide.md for details.").unwrap();
    fs::write(root.join("docs/guide.md"), "# Guide").unwrap();

    let links = parse_local_links("See docs/guide.md for details.", &document);
    assert_eq!(links, vec!["docs/guide.md"]);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inline_code_markdown_path_can_touch_chinese_prose_and_punctuation() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-inline-link-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(root.join("github/research/docs")).unwrap();
    let document = root.join("index.md");
    fs::write(&document, "# Index").unwrap();
    fs::write(root.join("github/research/docs/audit.md"), "# Audit").unwrap();

    let content = "触发文档：`github/research/docs/audit.md`：详细说明";
    assert_eq!(
        parse_local_links(content, &document),
        vec!["github/research/docs/audit.md"]
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn nonexistent_bare_markdown_path_is_not_registered() {
    let document = std::env::temp_dir().join("md-viewer-auto-link-missing/index.md");
    assert!(parse_local_links("See missing.md for details.", &document).is_empty());
}

#[test]
fn missing_explicit_link_surfaces_errors_without_changing_tab_or_history() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-missing-link-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(&root).unwrap();
    let document = root.join("index.md");
    fs::write(&document, "[Missing](missing.md)").unwrap();

    assert_eq!(
        parse_local_links("[Missing](missing.md)", &document),
        ["missing.md"]
    );

    let mut tab = Tab::new(document).unwrap();
    let original_path = tab.path.clone();
    let canonicalize_error = root.join("missing.md").canonicalize().unwrap_err();
    let expected_error = format!("Unable to open missing.md: {canonicalize_error}");

    // Ctrl+click resolves before asking the app to create a new tab. This
    // is the same helper whose error result is assigned to the error bar.
    let (new_tab, error_message) = handle_local_link_click(&mut tab, "missing.md", true);
    assert!(new_tab.is_none());
    assert_eq!(error_message, Some(expected_error.clone()));
    assert_eq!(tab.path, original_path);
    assert!(tab.history_back.is_empty());
    assert!(tab.history_forward.is_empty());

    // Plain click goes through the same production result handler and
    // leaves the current tab intact.
    let (new_tab, error_message) = handle_local_link_click(&mut tab, "missing.md", false);
    assert!(new_tab.is_none());
    assert_eq!(error_message, Some(expected_error));
    assert_eq!(tab.path, original_path);
    assert!(tab.history_back.is_empty());
    assert!(tab.history_forward.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fenced_and_indented_code_never_register_bare_markdown_paths() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-fenced-link-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(&root).unwrap();
    let document = root.join("index.md");
    fs::write(&document, "# Index").unwrap();
    fs::write(root.join("guide.md"), "# Guide").unwrap();

    let code_blocks = concat!(
        "~~~text\nguide.md\n~~~\n\n",
        "````text\nguide.md\n````\n\n",
        "    guide.md\n",
    );
    assert!(parse_local_links(code_blocks, &document).is_empty());
    assert_eq!(parse_local_links("`guide.md`", &document), ["guide.md"]);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn only_file_uris_are_classified_as_local_links() {
    let document_dir = Path::new("/tmp/docs");
    for destination in [
        "data:text/plain,x.md",
        "javascript:void(0)/x.md",
        "unknownscheme://host/a.md",
        "mailto:guide.md",
    ] {
        assert!(!is_local_markdown_link(destination), "{destination}");
        assert_eq!(
            resolve_local_link_path(destination, document_dir).unwrap(),
            None,
            "{destination}"
        );
    }

    assert!(is_local_markdown_link("guide.md"));
    assert!(is_local_markdown_link("docs%20with%20spaces/guide.md"));
    assert_eq!(
        resolve_local_link_path("100%done.md", document_dir).unwrap(),
        Some(document_dir.join("100%done.md"))
    );

    let file_path = std::env::temp_dir().join("md-viewer-link-scheme-guide.md");
    let file_uri = url::Url::from_file_path(&file_path).unwrap().to_string();
    assert!(is_local_markdown_link(&file_uri));
    assert_eq!(
        resolve_local_link_path(&file_uri, document_dir).unwrap(),
        Some(file_path)
    );
}

#[cfg(target_os = "windows")]
#[test]
fn windows_drive_prefixes_are_not_treated_as_uri_schemes() {
    assert_eq!(explicit_uri_scheme(r"C:\docs\guide.md"), None);
    assert_eq!(explicit_uri_scheme("D:/docs/guide.md"), None);
    assert!(is_local_markdown_link(r"C:\docs\guide.md"));
    assert!(is_local_markdown_link("D:/docs/guide.md"));
}

#[test]
fn explicit_non_file_schemes_are_not_registered_as_link_hooks() {
    let document = Path::new("/tmp/index.md");
    let content = concat!(
        "[data](data:text/plain,x.md) ",
        "[script](javascript:void(0)/x.md) ",
        "[unknown](unknownscheme://host/a.md)",
    );

    assert!(parse_local_links(content, document).is_empty());
}

#[test]
fn absolute_path_and_file_uri_are_registered_as_local_links() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-auto-link-uri-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(root.join("docs with spaces")).unwrap();
    let document = root.join("index.md");
    let absolute_target = root.join("guide.md");
    let uri_target = root.join("docs with spaces/guide.md");
    fs::write(&document, "# Index").unwrap();
    fs::write(&absolute_target, "# Absolute guide").unwrap();
    fs::write(&uri_target, "# URI guide").unwrap();

    let absolute = absolute_target.to_string_lossy();
    let file_uri = url::Url::from_file_path(&uri_target).unwrap().to_string();
    let content = format!("`{absolute}` and `{file_uri}`");
    let links = parse_local_links(&content, &document);

    assert_eq!(links, vec![absolute.to_string(), file_uri]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn commonmark_reference_with_encoded_path_is_registered() {
    let root = std::env::temp_dir().join(format!(
        "md-viewer-reference-link-{}-{}",
        std::process::id(),
        now_epoch_secs()
    ));
    fs::create_dir_all(root.join("docs with spaces")).unwrap();
    let document = root.join("index.md");
    fs::write(&document, "# Index").unwrap();
    fs::write(root.join("docs with spaces/guide(1).md"), "# Guide").unwrap();

    let content = "Read [the guide][guide].\n\n[guide]: docs%20with%20spaces/guide(1).md \"Title\"";
    assert_eq!(
        parse_local_links(content, &document),
        vec!["docs%20with%20spaces/guide(1).md"]
    );
    assert!(
        resolve_local_link_path("docs%20with%20spaces/guide(1).md", &root)
            .unwrap()
            .is_some_and(|path| path.is_file())
    );

    fs::remove_dir_all(root).unwrap();
}
