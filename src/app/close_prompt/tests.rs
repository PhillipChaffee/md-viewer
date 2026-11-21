use eframe::egui;

use super::*;
use crate::app::tests::Harness;
use crate::app::ui_harness::{enlarged, frame_asked_to_close, sent};

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

fn answer(harness: &mut Harness, answer: CloseAnswer) {
    harness.apply(AppAction::AnswerClosePrompt(answer));
}

#[test]
fn a_goal_replaces_the_documents_of_the_tabs_it_names_and_no_other() {
    let harness = three_tabs();
    let replaced =
        |goal: Goal| [0, 1, 2].map(|index| goal.replaces(index, &harness.app.tabs[index]));
    let leave = |route: Route| Goal::Leave(1, route);

    assert_eq!(replaced(Goal::Tab(1)), [false, true, false]);
    assert_eq!(replaced(Goal::OtherTabs(1)), [true, false, true]);
    assert_eq!(replaced(Goal::Quit), [true, true, true]);
    assert_eq!(
        replaced(leave(Route::File(harness.path("a.md")))),
        [false, true, false]
    );
    assert_eq!(
        replaced(leave(Route::File(harness.path("b.md")))),
        [false; 3],
        "the tab shows that file already"
    );
    assert_eq!(replaced(leave(Route::Back)), [false; 3], "no way back");
}

#[test]
fn leaving_a_document_asks_only_when_edits_would_be_lost() {
    let mut harness = three_tabs();
    let to_a = Route::File(harness.path("a.md"));

    edit(&mut harness, 0);
    harness.app.request_route(to_a);
    assert_eq!(asked(&harness), None, "the edits are in another tab");
    assert_eq!(harness.active_path(), harness.path("a.md"));

    edit(&mut harness, 2);
    harness.app.request_route(Route::Forward);
    harness.app.request_route(Route::Link("#c".to_owned()));
    harness
        .app
        .request_route(Route::Link("missing.md".to_owned()));
    assert_eq!(asked(&harness), None, "none of these leads away");
    assert!(harness.app.tabs[2].is_dirty());

    harness.app.request_route(Route::Back);
    assert_eq!(asked(&harness), Some(2));
    assert_eq!(harness.active_path(), harness.path("a.md"));
    let CloseState::Asking(pending) = &harness.app.closing else {
        panic!("the prompt is not asking");
    };
    assert_eq!(pending.goal, Goal::Leave(2, Route::Back));
}

#[test]
fn each_answer_about_leaving_a_document_is_carried_out() {
    let asked_to_leave = || {
        let mut harness = three_tabs();
        edit(&mut harness, 2);
        harness.app.request_route(Route::File(harness.path("a.md")));
        assert_eq!(asked(&harness), Some(2));
        harness
    };
    let on_disk = |harness: &Harness| std::fs::read_to_string(harness.path("c.md")).unwrap();

    let mut harness = asked_to_leave();
    answer(&mut harness, CloseAnswer::Save);
    assert_eq!(on_disk(&harness), "# C edited");
    assert_eq!(harness.active_path(), harness.path("a.md"));
    assert_eq!(harness.app.tabs[2].history_back, [harness.path("c.md")]);

    let mut harness = asked_to_leave();
    answer(&mut harness, CloseAnswer::Discard);
    assert_eq!(on_disk(&harness), "# C");
    assert_eq!(harness.active_path(), harness.path("a.md"));
    assert!(!harness.app.tabs[2].is_dirty());

    let mut harness = asked_to_leave();
    answer(&mut harness, CloseAnswer::Cancel);
    assert_eq!(on_disk(&harness), "# C");
    assert_eq!(harness.active_path(), harness.path("c.md"));
    assert!(harness.app.tabs[2].is_dirty());
    assert_eq!(asked(&harness), None);
    assert_eq!(harness.app.tabs.len(), 3);
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
    assert_eq!(pending.question, "Save changes to “B” (b.md)?");
    assert!(pending.focus_due);
    assert_eq!(harness.app.tabs.len(), 3);

    harness.idle();
    let CloseState::Asking(pending) = &harness.app.closing else {
        panic!("the prompt stopped asking");
    };
    assert!(!pending.focus_due, "Save takes the keyboard once");
}

#[test]
fn a_long_name_is_cut_after_a_word_and_a_short_one_is_whole() {
    let exactly = "x".repeat(NAME_CHARS);
    assert_eq!(shortened("Alpha"), "Alpha");
    assert_eq!(shortened(&exactly), exactly.as_str());

    let words = "word ".repeat(13);
    assert_eq!(shortened(&words), format!("{}…", "word ".repeat(12).trim()));
    let unbroken = "é".repeat(NAME_CHARS + 1);
    assert_eq!(shortened(&unbroken), format!("{}…", "é".repeat(NAME_CHARS)));
    let late_break = format!(" {unbroken}");
    assert_eq!(
        shortened(&late_break).chars().count(),
        NAME_CHARS + 1,
        "a space that leads the name is no place to cut"
    );
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
    harness.app.lightbox.open = Some(enlarged());

    frame_asked_to_close(&mut harness);

    assert_eq!(asked(&harness), Some(1));
    assert!(harness.app.lightbox.open.is_none());
}
