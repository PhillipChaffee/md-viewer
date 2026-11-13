use super::*;

#[test]
fn push_recent_dedupes_and_moves_to_front() {
    let mut v = Vec::new();
    push_recent(&mut v, Path::new("/a.md"), 100);
    push_recent(&mut v, Path::new("/b.md"), 200);
    push_recent(&mut v, Path::new("/a.md"), 300); // re-open a → moves to front, no dup
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].path, PathBuf::from("/a.md"));
    assert_eq!(v[0].last_opened, 300);
    assert_eq!(v[1].path, PathBuf::from("/b.md"));
}

#[test]
fn push_recent_caps_length_keeping_newest() {
    let mut v = Vec::new();
    for i in 0..(RECENT_FILES_CAP + 5) {
        push_recent(&mut v, Path::new(&format!("/f{i}.md")), i as u64);
    }
    assert_eq!(v.len(), RECENT_FILES_CAP);
    // The most recently pushed entry is at the front.
    assert_eq!(
        v[0].path,
        PathBuf::from(format!("/f{}.md", RECENT_FILES_CAP + 4))
    );
}

#[test]
fn relative_time_buckets() {
    let now = 1_000_000u64;
    assert_eq!(format_relative_time(now, now), "just now");
    assert_eq!(format_relative_time(now - 120, now), "2m ago");
    assert_eq!(format_relative_time(now - 7200, now), "2h ago");
    assert_eq!(format_relative_time(now - 90_000, now), "yesterday");
    assert_eq!(format_relative_time(now - 3 * 86_400, now), "3d ago");
    // Clock skew (future timestamp) must not underflow.
    assert_eq!(format_relative_time(now + 500, now), "just now");
}

#[test]
fn relative_time_switches_unit_exactly_at_each_boundary() {
    let now = 10_000_000u64;
    let ago = |seconds: u64| format_relative_time(now - seconds, now);
    assert_eq!(ago(59), "just now");
    assert_eq!(ago(60), "1m ago");
    assert_eq!(ago(3_599), "59m ago");
    assert_eq!(ago(3_600), "1h ago");
    assert_eq!(ago(86_399), "23h ago");
    assert_eq!(ago(86_400), "yesterday");
    assert_eq!(ago(172_799), "yesterday");
    assert_eq!(ago(172_800), "2d ago");
    assert_eq!(ago(604_799), "6d ago");
    assert_eq!(ago(604_800), "1w ago");
    assert_eq!(ago(2_591_999), "4w ago");
    assert_eq!(ago(2_592_000), "1mo ago");
    assert_eq!(ago(9_000_000), "3mo ago");
}
