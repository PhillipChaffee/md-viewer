//! A request to quit from the system: let through with nothing unsaved,
//! and turned into the window's own request to close otherwise.

use eframe::egui;
use mac_window::QuitAnswer;

use crate::app::close_prompt::CloseAnswer;
use crate::app::tests::Harness;
use crate::app::ui_harness::frame_with;

const CLOSE: egui::ViewportCommand = egui::ViewportCommand::Close;
const REFUSE: egui::ViewportCommand = egui::ViewportCommand::CancelClose;

/// Leave the document in front with an edit that is not saved, and run
/// the frame the edit is made in.
fn edit(harness: &mut Harness) {
    harness.app.tabs[0].content.push_str(" edited");
    harness.idle();
}

fn sent(output: &egui::FullOutput, wanted: &egui::ViewportCommand) -> bool {
    let mut commands = output
        .viewport_output
        .values()
        .flat_map(|viewport| &viewport.commands);
    commands.any(|command| command == wanted)
}

/// Run the next frame the way eframe does after `output`: a window told
/// to close in one frame asks to close in the next.
fn next_frame(harness: &mut Harness, output: &egui::FullOutput) -> egui::FullOutput {
    let mut input = egui::RawInput::default();
    if sent(output, &CLOSE) {
        let window = input.viewports.entry(egui::ViewportId::ROOT).or_default();
        window.events.push(egui::ViewportEvent::Close);
    }
    frame_with(harness, input)
}

fn prompt_shows(harness: &Harness) -> bool {
    harness.app.closing.is_asking()
}

#[test]
fn with_nothing_unsaved_the_system_may_quit_at_once() {
    let mut harness = Harness::with_three_docs();
    let ask = harness.app.quit_asker();
    harness.idle();

    assert_eq!(ask(), QuitAnswer::Now);

    let output = harness.idle();
    assert!(!sent(&output, &CLOSE), "the system does the quitting");
    assert!(!prompt_shows(&harness));
}

#[test]
fn with_edits_unsaved_the_system_is_refused_and_the_prompt_asks() {
    let mut harness = Harness::with_three_docs();
    let ask = harness.app.quit_asker();
    edit(&mut harness);

    assert_eq!(ask(), QuitAnswer::NotYet);

    let output = harness.idle();
    assert!(sent(&output, &CLOSE), "the window is asked to close");
    assert!(sent(&output, &egui::ViewportCommand::Focus));
    assert!(sent(&output, &egui::ViewportCommand::Minimized(false)));
    let output = next_frame(&mut harness, &output);
    assert!(sent(&output, &REFUSE), "the close is refused");
    assert!(prompt_shows(&harness));
}

#[test]
fn cancel_keeps_the_app_and_a_later_request_asks_again() {
    let mut harness = Harness::with_three_docs();
    let ask = harness.app.quit_asker();
    edit(&mut harness);
    ask();
    let output = harness.idle();
    next_frame(&mut harness, &output);

    harness.app.answer_close_prompt(CloseAnswer::Cancel);
    let output = harness.idle();

    assert!(!sent(&output, &CLOSE));
    assert!(!prompt_shows(&harness));
    assert!(harness.app.tabs[0].is_dirty());
    assert_eq!(ask(), QuitAnswer::NotYet);
}

#[test]
fn discard_goes_on_to_close_the_window() {
    let mut harness = Harness::with_three_docs();
    let ask = harness.app.quit_asker();
    edit(&mut harness);
    ask();
    let output = harness.idle();
    next_frame(&mut harness, &output);

    harness.app.answer_close_prompt(CloseAnswer::Discard);
    let output = harness.idle();

    assert!(sent(&output, &CLOSE));
    let output = next_frame(&mut harness, &output);
    assert!(!sent(&output, &REFUSE), "the close goes through");
}

#[test]
fn save_writes_the_edits_and_then_the_system_may_quit() {
    let mut harness = Harness::with_three_docs();
    let ask = harness.app.quit_asker();
    edit(&mut harness);
    ask();
    let output = harness.idle();
    next_frame(&mut harness, &output);

    harness.app.answer_close_prompt(CloseAnswer::Save);
    let output = harness.idle();

    assert!(sent(&output, &CLOSE));
    assert!(!harness.app.tabs[0].is_dirty());
    assert_eq!(ask(), QuitAnswer::Now, "nothing is left to ask about");
}

#[test]
fn a_request_while_the_prompt_shows_leaves_it_asking() {
    let mut harness = Harness::with_three_docs();
    let ask = harness.app.quit_asker();
    edit(&mut harness);
    ask();
    let output = harness.idle();
    next_frame(&mut harness, &output);
    assert!(prompt_shows(&harness));

    assert_eq!(ask(), QuitAnswer::NotYet);
    let output = harness.idle();
    let output = next_frame(&mut harness, &output);

    assert!(sent(&output, &REFUSE));
    assert!(prompt_shows(&harness));
    assert!(harness.app.tabs[0].is_dirty());
}
