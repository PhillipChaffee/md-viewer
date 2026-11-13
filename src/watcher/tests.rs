use super::*;

#[test]
fn watcher_retry_sequence_stops_at_configured_limit() {
    assert_eq!(next_watcher_retry(0), Some(1));
    assert_eq!(next_watcher_retry(1), Some(2));
    assert_eq!(next_watcher_retry(2), Some(3));
    assert_eq!(next_watcher_retry(3), None);
    assert_eq!(next_watcher_retry(u32::MAX), None);
}
