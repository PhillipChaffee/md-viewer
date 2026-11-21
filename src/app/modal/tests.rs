use eframe::egui;

use super::Modal;
use crate::app::close_prompt::CloseAnswer;
use crate::app::lightbox::{LightboxState, LightboxTexture};
use crate::app::tests::Harness;

/// A picture as the lightbox holds one that is enlarged.
fn enlarged() -> LightboxState {
    LightboxState {
        texture: LightboxTexture::Loaded(egui::TextureId::default()),
        base_size: egui::vec2(100.0, 100.0),
        zoom: 1.0,
        open_id: 1,
        scroll_offset: egui::Vec2::ZERO,
    }
}

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
