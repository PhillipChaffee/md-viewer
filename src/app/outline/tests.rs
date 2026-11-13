use super::*;

#[test]
fn a_measured_header_scrolls_to_just_above_its_position() {
    assert_eq!(
        header_jump(Some(400.0), 10, 200, 4000.0),
        Some(HeaderJump {
            scroll_offset: 350.0,
            is_estimate: false,
        })
    );
}

#[test]
fn a_measured_header_near_the_top_scrolls_to_the_top() {
    assert_eq!(
        header_jump(Some(20.0), 1, 200, 4000.0),
        Some(HeaderJump {
            scroll_offset: 0.0,
            is_estimate: false,
        })
    );
}

#[test]
fn an_unmeasured_header_is_estimated_from_its_line_number() {
    // Line 50 of 200 is a quarter of the way down a 4000px document.
    assert_eq!(
        header_jump(None, 50, 200, 4000.0),
        Some(HeaderJump {
            scroll_offset: 950.0,
            is_estimate: true,
        })
    );
}

#[test]
fn an_unmeasured_header_cannot_be_estimated_before_layout() {
    assert_eq!(header_jump(None, 50, 200, 0.0), None);
    assert_eq!(header_jump(None, 50, 0, 4000.0), None);
}

#[test]
fn a_measured_position_does_not_depend_on_layout_totals() {
    assert_eq!(
        header_jump(Some(400.0), 50, 0, 0.0),
        Some(HeaderJump {
            scroll_offset: 350.0,
            is_estimate: false,
        })
    );
}
