//! The default build, where there is no bridge: the app must run exactly
//! as if the seam were not called.

use eframe::egui;

use super::Mcp;

#[test]
fn without_the_bridge_the_frame_hooks_leave_the_frame_alone() {
    let mut mcp = Mcp::new();
    let mut input = egui::RawInput {
        events: vec![egui::Event::PointerGone],
        ..Default::default()
    };

    super::raw_input(&mut mcp, &mut input);
    assert_eq!(input.events, [egui::Event::PointerGone]);

    let ctx = egui::Context::default();
    let mut frame = || {
        ctx.run(egui::RawInput::default(), |ctx| {
            super::begin_frame(&mut mcp, ctx);
            super::capture(&mut mcp, ctx);
        })
    };
    // egui asks for a second frame after its first, whatever is drawn.
    let _ = frame();
    let settled = frame();
    assert!(
        settled.platform_output.accesskit_update.is_none(),
        "only the bridge turns the accessibility tree on"
    );
    let repaint = settled.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    assert_eq!(
        repaint,
        std::time::Duration::MAX,
        "an idle app must not be kept awake"
    );
}
