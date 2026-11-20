//! Pictures far down a page: one is read only once the reader comes near
//! it.

use std::time::Duration;

use super::pages::{each, illustrated, GREEN_GIF, NEAR, PULSE_GIF};
use super::pictures::pulse_color;
use super::support::{site, Launch, Reader};

/// As many lines as fill several windows.
fn lines(name: &str, count: usize) -> String {
    each(0..count, |line| format!("{name} {line}\n\n"))
}

/// A page with `picture` far down it. The heading "Close by" leads to a
/// place from where the picture is less than a window further down, and
/// still out of view: the lines after the heading fill the window.
fn far_down(picture: &str) -> Launch {
    let text = format!(
        "# Gallery\n\n{}## Close by\n\n{}{picture}\n\nthe end\n\n{}",
        lines("first", 150),
        lines("second", 30),
        lines("last", 60),
    );
    illustrated(&text)
}

/// Long enough for a picture to be asked for, if the app were going to.
const A_WHILE: Duration = Duration::from_millis(300);

#[test]
fn a_picture_on_the_web_is_not_fetched_until_the_reader_is_within_a_window_of_it() {
    let web = site(&[("pulse.gif", PULSE_GIF)]);
    let mut reader = far_down(&format!("![a picture]({}/pulse.gif)", web.address)).open();
    reader.idle_for(A_WHILE);
    let at_the_top = (reader.is_at_rest(), web.times_asked_for("pulse.gif"));

    reader.click("Close by");
    reader.wait_for("the picture to be asked for", |_| {
        web.times_asked_for("pulse.gif") > 0
    });
    let close_by = (reader.in_view("second 0"), reader.in_view("the end"));
    reader.click("Gallery");
    reader.idle_for(A_WHILE);

    assert_eq!(at_the_top, (true, 0), "at rest, and nothing was fetched");
    assert_eq!(close_by, (true, false), "the picture is not in view yet");
    assert_eq!(web.times_asked_for("pulse.gif"), 1, "it is fetched once");
}

#[test]
fn a_moving_picture_is_read_from_its_file_when_the_reader_comes_near_and_not_before() {
    // What the file holds when the page is opened is not what is shown:
    // the file is not read while the picture is far away.
    let mut reader = far_down("![a picture](late.gif)")
        .bytes("late.gif", GREEN_GIF)
        .open();
    reader.idle_for(A_WHILE);
    assert!(reader.is_at_rest());
    reader.write_bytes("late.gif", PULSE_GIF);

    reader.click("Close by");
    reader.wait_for("the last line before the picture", |reader| {
        reader.in_view("second 0")
    });
    while !reader.in_view("the end") {
        reader.scroll_down(100.0);
    }
    reader.wait_for("the picture to be drawn", |reader| {
        !reader.pictures().is_empty()
    });

    let picture = reader.pictures()[0];
    let before = pulse_color(&reader, picture);
    reader.wait_for("the picture to move", |reader| {
        pulse_color(reader, picture) != before
    });
}

/// A page with the six `pictures`, 80 pixels high each, between its start
/// and the heading "Target", all of them more than a window from either.
fn pictures_above_a_heading(pictures: &[String; 6]) -> Reader {
    let text = format!(
        "# Gallery\n\n{}{}{}## Target\n\n{}",
        lines("first", 100),
        each(pictures, |picture| format!("{picture}\n\n")),
        lines("between", 60),
        lines("after", 80),
    );
    illustrated(&text).open()
}

/// Jump to the heading "Target" and return how far down the window the
/// first line after it is.
#[track_caller]
fn jump_to_the_target(reader: &mut Reader) -> f32 {
    reader.click("Target");
    reader.wait_for("the target to scroll into view", |reader| {
        reader.in_view("after 0")
    });
    reader.text_area("after 0").top
}

#[test]
fn a_heading_below_pictures_that_were_never_read_is_jumped_to_and_stays_put() {
    let pictures = [(); 6].map(|()| "![a picture](blue.png)".to_owned());
    let mut reader = pictures_above_a_heading(&pictures);
    reader.idle_for(A_WHILE);

    let landed = jump_to_the_target(&mut reader);
    let heading = reader.text_area("Target");
    reader.idle_for(A_WHILE);
    let later = reader.text_area("after 0").top;
    reader.click("Gallery");
    reader.wait_for("the top", |reader| reader.in_view("first 0"));
    let again = jump_to_the_target(&mut reader);

    // The heading is as far below the top of the pane as on a page that
    // has no picture to wait for.
    let no_pictures = [(); 6].map(|()| "no picture".to_owned());
    let plain = jump_to_the_target(&mut pictures_above_a_heading(&no_pictures));
    assert!((landed - plain).abs() < NEAR, "{landed} against {plain}");
    assert!(heading.top < landed && heading.top > 0.0, "{heading:?}");
    assert!((later - landed).abs() < NEAR, "it moved to {later}");
    assert!((again - landed).abs() < NEAR, "the second jump: {again}");
}
