use super::Modal;
use crate::app::close_prompt::CloseAnswer;
use crate::app::tests::Harness;
use crate::app::ui_harness::enlarged;

/// One document open with an edit that is not saved.
fn unsaved() -> Harness {
    let mut harness = Harness::with_three_docs();
    harness.app.tabs[0].content.push_str(" edited");
    harness
}

#[test]
fn a_page_with_nothing_over_it_takes_its_own_input() {
    let harness = unsaved();

    assert_eq!(harness.app.modal(), None);
}

#[test]
fn the_prompt_is_modal_until_it_is_answered() {
    let mut harness = unsaved();

    harness.app.request_close_tab(0);
    assert_eq!(harness.app.modal(), Some(Modal::Prompt));

    harness.app.answer_close_prompt(CloseAnswer::Cancel);
    assert_eq!(harness.app.modal(), None);
}

#[test]
fn an_enlarged_picture_is_modal_until_it_is_closed() {
    let mut harness = unsaved();

    harness.app.lightbox.open = Some(enlarged());
    assert_eq!(harness.app.modal(), Some(Modal::Lightbox));

    harness.app.lightbox.open = None;
    assert_eq!(harness.app.modal(), None);
}
