use super::*;

#[test]
fn a_measured_header_scrolls_to_just_above_its_position() {
    assert_eq!(
        header_jump(Some(400.0), 10, 200, 4000.0),
        HeaderJump {
            scroll_offset: 350.0,
            is_estimate: false,
        }
    );
}

#[test]
fn a_measured_header_near_the_top_scrolls_to_the_top() {
    assert_eq!(
        header_jump(Some(20.0), 1, 200, 4000.0),
        HeaderJump {
            scroll_offset: 0.0,
            is_estimate: false,
        }
    );
}

#[test]
fn an_unmeasured_header_is_estimated_from_its_line_number() {
    // Line 50 of 200 is a quarter of the way down a 4000px document.
    assert_eq!(
        header_jump(None, 50, 200, 4000.0),
        HeaderJump {
            scroll_offset: 950.0,
            is_estimate: true,
        }
    );
}

#[test]
fn before_layout_an_unmeasured_header_is_estimated_at_the_top() {
    let at_the_top = HeaderJump {
        scroll_offset: 0.0,
        is_estimate: true,
    };

    assert_eq!(header_jump(None, 50, 200, 0.0), at_the_top);
    assert_eq!(header_jump(None, 50, 0, 4000.0), at_the_top);
}

#[test]
fn a_measured_position_does_not_depend_on_layout_totals() {
    assert_eq!(
        header_jump(Some(400.0), 50, 0, 0.0),
        HeaderJump {
            scroll_offset: 350.0,
            is_estimate: false,
        }
    );
}
