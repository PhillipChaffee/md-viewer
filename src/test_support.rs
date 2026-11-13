//! Assertions shared by the unit tests.

/// Asserts two pixel or zoom values are equal to within float rounding.
///
/// The values compared in this crate are small exact constants and clamped
/// results, so the tolerance only absorbs representation error.
#[track_caller]
pub(crate) fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= f32::EPSILON * expected.abs().max(1.0),
        "expected {expected}, got {actual}"
    );
}
