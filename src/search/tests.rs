use super::*;

#[test]
fn search_correction_request_is_consumed_even_without_a_rendered_match() {
    let mut pending = true;

    assert!(take_search_correction_request(&mut pending));
    assert!(!pending);
    assert!(!take_search_correction_request(&mut pending));
}

#[test]
fn find_matches_empty_query_returns_none() {
    assert_eq!(find_matches("hello world", ""), vec![]);
}

#[test]
fn find_matches_empty_content_returns_none() {
    assert_eq!(find_matches("", "hello"), vec![]);
}

#[test]
fn find_matches_single_ascii() {
    let m = find_matches("hello world", "world");
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].byte_start, 6);
    assert_eq!(m[0].byte_end, 11);
    assert_eq!(m[0].line_number, 1);
}

#[test]
fn find_matches_case_insensitive_ascii() {
    let m = find_matches("Hello WORLD", "world");
    assert_eq!(m.len(), 1);
    assert_eq!(&"Hello WORLD"[m[0].byte_start..m[0].byte_end], "WORLD");
}

#[test]
fn find_matches_multiple_per_line() {
    // "foo bar foo baz foo" → 3 matches, all on line 1
    let m = find_matches("foo bar foo baz foo", "foo");
    assert_eq!(m.len(), 3);
    assert!(m.iter().all(|sm| sm.line_number == 1));
    let starts: Vec<_> = m.iter().map(|sm| sm.byte_start).collect();
    assert_eq!(starts, vec![0, 8, 16]);
}

#[test]
fn find_matches_line_numbers_one_based() {
    let content = "first line\nsecond match here\nthird\nmatch on four";
    let m = find_matches(content, "match");
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].line_number, 2);
    assert_eq!(m[1].line_number, 4);
}

#[test]
fn find_matches_preserves_byte_offsets_with_utf8() {
    // "café résumé naïve" — each accented char is 2 bytes in UTF-8
    let content = "café résumé naïve";
    let m = find_matches(content, "café");
    assert_eq!(m.len(), 1);
    // Confirm byte range slices back to the original substring
    assert_eq!(&content[m[0].byte_start..m[0].byte_end], "café");
}

#[test]
fn find_matches_skips_cross_newline_matches() {
    // Query "oo\nb" theoretically spans two lines — should not be reported
    let content = "foo\nbar";
    let m = find_matches(content, "oo\nb");
    assert_eq!(m, vec![]);
}

#[test]
fn find_matches_overlapping_uses_match_indices_semantics() {
    // str::match_indices does NOT overlap matches: "aaaa" / "aa" → [0, 2]
    let m = find_matches("aaaa", "aa");
    let starts: Vec<_> = m.iter().map(|sm| sm.byte_start).collect();
    assert_eq!(starts, vec![0, 2]);
}

#[test]
fn find_matches_skips_image_alt_text() {
    // Image alt text isn't visibly rendered (used only as hover/screen-reader text).
    // A match inside the alt portion of `![alt](url)` would have no visible target
    // for highlighting/scrolling — exclude it.
    let content = "See ![Syntax docs](pic.png) and the syntax guide.";
    let m = find_matches(content, "syntax");
    let starts: Vec<_> = m.iter().map(|sm| sm.byte_start).collect();
    assert_eq!(m.len(), 1, "got {:?}", m);
    // The surviving match is "syntax" in "the syntax guide", not "Syntax" in alt
    assert_eq!(&content[starts[0]..starts[0] + 6], "syntax");
}

#[test]
fn find_matches_skips_image_url() {
    // Image URL isn't visibly rendered — exclude matches inside.
    let content = "![ok](path/syntax.png) more text";
    let m = find_matches(content, "syntax");
    assert_eq!(m.len(), 0, "URL match should be filtered: {:?}", m);
}

#[test]
fn find_matches_skips_link_url_keeps_link_text() {
    // Link text IS rendered (visible clickable text). Link URL is not.
    let content = "Click [syntax docs](https://example.com/syntax.html) here";
    let m = find_matches(content, "syntax");
    // Two raw matches: "syntax" in link text (visible) + "syntax" in URL (invisible)
    assert_eq!(m.len(), 1, "should keep link text, drop URL: {:?}", m);
    let s = m[0].byte_start;
    // The kept match is "syntax" in "[syntax docs]" (visible link text)
    assert_eq!(&content[s..s + 6], "syntax");
    // And it's the FIRST occurrence (inside the brackets), not the URL one
    assert!(s < content.find("https").unwrap());
}

#[test]
fn find_matches_multiple_alt_text_images() {
    let content = "![a one](u.png) one ![two two](w.png) two";
    let m = find_matches(content, "one");
    let starts: Vec<_> = m.iter().map(|sm| sm.byte_start).collect();
    // Two raw "one" matches in alt + 1 in visible text → only 1 survives
    assert_eq!(m.len(), 1);
    assert_eq!(&content[starts[0]..starts[0] + 3], "one");

    let m2 = find_matches(content, "two");
    // Two raw "two" matches in alt + 1 in visible text → only 1 survives
    assert_eq!(m2.len(), 1);
}

#[test]
fn shortcode_search_range_uses_raw_source_bytes() {
    let content = "before :pushpin: after";
    let matches = find_matches(content, ":pushpin:");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].byte_start, 7);
    assert_eq!(matches[0].byte_end, 16);
    assert_eq!(
        &content[matches[0].byte_start..matches[0].byte_end],
        ":pushpin:"
    );
}

#[test]
fn rendered_emoji_does_not_match_shortcode_only_source() {
    assert!(find_matches("before :pushpin: after", "📌").is_empty());
}

#[test]
fn literal_unicode_emoji_still_matches_raw_source() {
    let matches = find_matches("before 📌 after", "📌");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].byte_start, 7);
    assert_eq!(matches[0].byte_end, 11);
}
