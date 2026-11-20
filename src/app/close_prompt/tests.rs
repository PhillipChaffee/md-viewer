use eframe::egui;

use super::*;
use crate::app::lightbox::{LightboxState, LightboxTexture};
use crate::app::tests::Harness;
use crate::app::ui_harness::frame_with;

/// `a.md`, `b.md` and `c.md` in three tabs, `c.md` in front.
fn three_tabs() -> Harness {
    let mut harness = Harness::with_three_docs();
    for name in ["b.md", "c.md"] {
        harness.app.open_in_new_tab(harness.path(name));
    }
    harness
}

/// Leave the tab at `index` with an edit that is not saved.
fn edit(harness: &mut Harness, index: usize) {
    harness.app.tabs[index].content.push_str(" edited");
}

/// The place of the tab the prompt asks about, if it asks.
fn asked(harness: &Harness) -> Option<usize> {
    match &harness.app.closing {
        CloseState::Asking(pending) => Some(pending.asked),
        CloseState::Idle | CloseState::Quitting => None,
    }
}

/// Run one frame in which the window asks to close.
fn frame_asked_to_close(harness: &mut Harness) -> egui::FullOutput {
    let mut input = egui::RawInput::default();
    let window = input.viewports.entry(egui::ViewportId::ROOT).or_default();
    window.events.push(egui::ViewportEvent::Close);
    frame_with(harness, input)
}

fn sent(output: &egui::FullOutput, wanted: &egui::ViewportCommand) -> bool {
    let mut commands = output
        .viewport_output
        .values()
        .flat_map(|viewport| &viewport.commands);
    commands.any(|command| command == wanted)
}

fn answer(harness: &mut Harness, answer: CloseAnswer) {
    harness.apply(AppAction::AnswerClosePrompt(answer));
}

#[test]
fn a_goal_closes_the_tabs_it_names_and_no_other() {
    let closed = |goal: Goal| [0, 1, 2].map(|index| goal.closes(index));

    assert_eq!(closed(Goal::Tab(1)), [false, true, false]);
    assert_eq!(closed(Goal::OtherTabs(1)), [true, false, true]);
    assert_eq!(closed(Goal::Quit), [true, true, true]);
}

#[test]
fn a_tab_with_nothing_unsaved_closes_in_the_call_that_asked() {
    let mut harness = three_tabs();
    edit(&mut harness, 0);

    harness.app.request_close_tab(1);

    assert_eq!(harness.app.tabs.len(), 2);
    assert_eq!(asked(&harness), None);
    harness.app.request_close_other_tabs(0);
    assert_eq!(
        harness.app.tabs.len(),
        1,
        "the kept tab's edits are not at risk"
    );
    assert_eq!(asked(&harness), None);
}

#[test]
fn the_question_names_the_document_and_is_written_once() {
    let mut harness = three_tabs();
    edit(&mut harness, 1);

    harness.app.request_close_tab(1);

    let CloseState::Asking(pending) = &harness.app.closing else {
        panic!("the prompt is not asking");
    };
    assert_eq!(pending.question, "Save changes to b.md?");
    assert!(pending.focus_due);
    assert_eq!(harness.app.tabs.len(), 3);

    harness.idle();
    let CloseState::Asking(pending) = &harness.app.closing else {
        panic!("the prompt stopped asking");
    };
    assert!(!pending.focus_due, "Save takes the keyboard once");
}

#[test]
fn close_others_asks_in_strip_order_and_closes_after_the_last_answer() {
    let mut harness = three_tabs();
    for index in 0..3 {
        edit(&mut harness, index);
    }

    harness.app.request_close_other_tabs(1);
    assert_eq!(asked(&harness), Some(0));
    answer(&mut harness, CloseAnswer::Discard);
    assert_eq!(asked(&harness), Some(2), "the kept tab is passed over");
    assert_eq!(harness.app.tabs.len(), 3);
    answer(&mut harness, CloseAnswer::Discard);

    assert_eq!(asked(&harness), None);
    assert_eq!(harness.app.tabs.len(), 1);
    assert_eq!(harness.active_path(), harness.path("b.md"));
    assert!(
        harness.app.tabs[0].is_dirty(),
        "the kept tab keeps its edits"
    );
}

#[test]
fn an_answer_with_no_question_changes_nothing() {
    let mut harness = three_tabs();
    edit(&mut harness, 2);

    for given in [CloseAnswer::Save, CloseAnswer::Discard, CloseAnswer::Cancel] {
        answer(&mut harness, given);
    }

    assert_eq!(harness.app.tabs.len(), 3);
    assert!(harness.app.tabs[2].is_dirty());
    assert!(matches!(harness.app.closing, CloseState::Idle));
}

#[test]
fn the_window_is_kept_open_only_while_edits_are_unsaved() {
    let refuse = egui::ViewportCommand::CancelClose;
    let mut harness = three_tabs();

    let output = frame_asked_to_close(&mut harness);
    assert!(!sent(&output, &refuse), "nothing is unsaved");
    assert_eq!(asked(&harness), None);

    edit(&mut harness, 1);
    let output = frame_asked_to_close(&mut harness);
    assert!(sent(&output, &refuse));
    assert_eq!(asked(&harness), Some(1));
    assert!(
        !sent(&harness.idle(), &refuse),
        "a frame that was not asked"
    );
}

#[test]
fn the_last_answer_closes_the_window_and_the_close_goes_through() {
    let mut harness = three_tabs();
    edit(&mut harness, 1);
    edit(&mut harness, 2);
    frame_asked_to_close(&mut harness);

    answer(&mut harness, CloseAnswer::Discard);
    assert_eq!(
        asked(&harness),
        Some(2),
        "the next unsaved tab is asked about"
    );
    answer(&mut harness, CloseAnswer::Discard);
    let output = harness.idle();

    assert!(sent(&output, &egui::ViewportCommand::Close));
    assert!(matches!(harness.app.closing, CloseState::Quitting));
    assert_eq!(harness.app.tabs.len(), 3, "the session keeps its tabs");
    let output = frame_asked_to_close(&mut harness);
    assert!(!sent(&output, &egui::ViewportCommand::CancelClose));
    assert_eq!(asked(&harness), None);
}

#[test]
fn the_prompt_closes_an_open_lightbox_so_it_can_be_seen() {
    let mut harness = three_tabs();
    edit(&mut harness, 1);
    harness.app.lightbox.open = Some(LightboxState {
        texture: LightboxTexture::Loaded(egui::TextureId::default()),
        base_size: egui::vec2(100.0, 100.0),
        zoom: 1.0,
        open_id: 1,
        scroll_offset: egui::Vec2::ZERO,
    });

    frame_asked_to_close(&mut harness);

    assert_eq!(asked(&harness), Some(1));
    assert!(harness.app.lightbox.open.is_none());
}
