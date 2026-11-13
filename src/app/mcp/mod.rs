//! The one place the app meets the MCP bridge, which drives it in E2E
//! tests. With the `mcp` feature each function here hands its arguments to
//! the bridge; without it each does nothing, and a call costs nothing.
//!
//! No gate builds the `mcp` feature (the bridge crate is not published), so
//! the bodies that use the bridge stay a plain list of its calls.

use std::fmt;
#[cfg(feature = "mcp")]
use std::time::Duration;

use eframe::egui;
#[cfg(feature = "mcp")]
use egui_mcp_bridge::{McpBridge, McpUiExt};

#[cfg(all(test, not(feature = "mcp")))]
mod tests;

/// The app's end of the MCP bridge.
pub(super) struct Mcp {
    #[cfg(feature = "mcp")]
    bridge: McpBridge,
}

impl Mcp {
    /// Start the bridge listening.
    #[cfg(feature = "mcp")]
    pub(super) fn new() -> Self {
        let bridge = McpBridge::builder().port(9877).build();
        log::info!("MCP bridge listening on port {}", bridge.port());
        Self { bridge }
    }

    #[cfg(not(feature = "mcp"))]
    pub(super) const fn new() -> Self {
        Self {}
    }
}

/// Tell the bridge about a widget by its response. `name` is what an E2E
/// test finds the widget by, and `value` is the state it reads from it.
#[cfg(feature = "mcp")]
pub(super) fn widget(
    mcp: &Mcp,
    name: fmt::Arguments<'_>,
    kind: &str,
    response: &egui::Response,
    value: Option<&str>,
) {
    mcp.bridge
        .register_widget(&name.to_string(), kind, response, value);
}

#[cfg(not(feature = "mcp"))]
pub(super) const fn widget(
    _mcp: &Mcp,
    _name: fmt::Arguments<'_>,
    _kind: &str,
    _response: &egui::Response,
    _value: Option<&str>,
) {
}

/// Run the commands the bridge has received and add the input they ask
/// for to the frame's.
#[cfg(feature = "mcp")]
pub(super) fn raw_input(mcp: &mut Mcp, raw_input: &mut egui::RawInput) {
    mcp.bridge.process_commands();
    mcp.bridge.inject_raw_input(raw_input);
}

#[cfg(not(feature = "mcp"))]
pub(super) const fn raw_input(_mcp: &mut Mcp, _raw_input: &mut egui::RawInput) {}

/// Get the bridge ready for a frame's widgets.
#[cfg(feature = "mcp")]
pub(super) fn begin_frame(mcp: &mut Mcp, ctx: &egui::Context) {
    ctx.enable_accesskit();
    mcp.bridge.begin_frame();
    mcp.bridge.store_in_context(ctx); // Enable McpUiExt methods
                                      // Poll for MCP commands at reasonable rate (prevents CPU spin on virtual displays)
    ctx.request_repaint_after(Duration::from_millis(50));
}

#[cfg(not(feature = "mcp"))]
pub(super) const fn begin_frame(_mcp: &mut Mcp, _ctx: &egui::Context) {}

/// Hand the bridge the frame's `AccessKit` output.
#[cfg(feature = "mcp")]
pub(super) fn capture(mcp: &mut Mcp, ctx: &egui::Context) {
    mcp.bridge.capture_output(ctx);
}

#[cfg(not(feature = "mcp"))]
pub(super) const fn capture(_mcp: &mut Mcp, _ctx: &egui::Context) {}
