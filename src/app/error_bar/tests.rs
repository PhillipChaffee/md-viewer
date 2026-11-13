use eframe::egui;

use crate::app::tests::Harness;
use crate::app::ui_harness::UiHarness;
use crate::test_support::assert_close;
use crate::theme::ThemeId;

#[test]
fn the_error_bar_shows_the_message_until_it_is_dismissed() {
    let mut harness = UiHarness::with_three_docs();
    harness.app_mut().error_message = Some("Unable to open notes.md".to_owned());
    harness.settle();
    assert!(harness.shows("Unable to open notes.md"));

    harness.click_exact("Dismiss");

    assert_eq!(harness.app().error_message, None);
    assert!(!harness.shows("Unable to open notes.md"));
}

#[test]
fn the_error_bar_is_written_in_the_themes_warning_color() {
    let mut harness = Harness::with_three_docs();
    harness.app.theme = ThemeId::Dusk;
    harness.app.error_message = Some("Unable to open notes.md".to_owned());
    harness.idle();

    let output = harness.idle();

    let message = crate::test_support::painted_text_starting(&output, "Unable to open");
    assert_eq!(
        message.galley.job.sections[0].format.color,
        ThemeId::Dusk.palette().warning
    );
}

#[test]
fn the_error_bar_opens_with_a_warning_sign_and_keeps_its_dismiss_button_in_the_window() {
    let mut harness = UiHarness::with_three_docs();
    let long = "Unable to open a file with a very long name. ".repeat(12);
    harness.app_mut().error_message = Some(long);
    harness.settle();
    let warning = ThemeId::Sepia.palette().warning;
    let top = crate::app::chrome::BAR_HEIGHT;
    let strip = top..=top + crate::app::chrome::STRIP_HEIGHT;

    let dismiss = harness.labelled("Dismiss")[0].0;
    assert_close(dismiss.x, harness.labelled("More")[0].0.x);
    assert!(strip.contains(&dismiss.y));

    let shapes = crate::test_support::painted_shapes(harness.ui.output());
    let sign = shapes.iter().find_map(|shape| match shape {
        egui::Shape::Circle(ring) if ring.stroke.color == warning => Some(ring.center),
        _ => None,
    });
    let sign = sign.expect("no ringed sign in the warning color");
    assert!(strip.contains(&sign.y));
    let message = shapes
        .iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(text) if text.galley.text().starts_with("Unable to open") => {
                Some(text)
            }
            _ => None,
        })
        .unwrap();
    assert!(
        message.galley.elided,
        "cut short, not wrapped or pushed out"
    );
    assert!(sign.x < message.pos.x);
    let right = message.pos.x + message.galley.size().x;
    assert!(right < dismiss.x, "{right} runs under the button");
}
