use super::FigureSpace;
use egui::{pos2, vec2, Rect};

const SPACE: FigureSpace = FigureSpace {
    axis: 500.0,
    width: 800.0,
    min_left: 40.0,
};

#[test]
fn a_figure_that_fits_is_shown_at_its_natural_size_on_the_axis() {
    let shown = SPACE.place(vec2(300.0, 120.0), 70.0);

    assert_eq!(shown, Rect::from_min_size(pos2(350.0, 70.0), vec2(300.0, 120.0)));
}

#[test]
fn a_figure_wider_than_its_room_is_scaled_down_to_it() {
    let shown = SPACE.place(vec2(1600.0, 400.0), 0.0);

    assert_eq!(shown, Rect::from_min_size(pos2(100.0, 0.0), vec2(800.0, 200.0)));
}

#[test]
fn a_figure_is_never_placed_left_of_the_pane() {
    let off_center = FigureSpace { axis: 300.0, ..SPACE };

    let shown = off_center.place(vec2(700.0, 100.0), 0.0);

    assert_eq!(shown.left(), 40.0, "300 - 350 would be left of the pane");
    assert_eq!(shown.size(), vec2(700.0, 100.0), "moving it does not resize it");
}

#[test]
fn a_figure_without_a_size_takes_no_room() {
    let shown = SPACE.place(vec2(0.0, 0.0), 0.0);

    assert!(shown.is_finite());
    assert_eq!(shown.height(), 0.0);
}
