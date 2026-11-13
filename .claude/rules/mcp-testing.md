# MCP Testing Rules

This project supports E2E testing via egui-mcp-bridge. Follow these rules when adding UI widgets to ensure testability.

Every call to the bridge goes through `src/app/mcp/mod.rs`. That file is the only one that names the `mcp` feature. With the feature its functions pass their arguments to the bridge, and without it they do nothing.

## Quick Reference

```rust
use crate::app::mcp;

// Register widgets after creating them
let btn = ui.small_button("⊞").on_hover_text("Expand all");
mcp::widget(&self.mcp, format_args!("Explorer: Expand All"), "button", &btn, None);
if btn.clicked() { ... }
```

The name is a `format_args!`, so a name built from data (`format_args!("File: {}", row.name)`) allocates nothing in the default build.

## When to Register Widgets

### Always Register (Interactive)

These widgets MUST be registered for MCP testing:

| Widget Type | Registration | Example |
| ------------- | -------------- | --------- |
| Buttons | `mcp::widget(mcp, name, "button", &response, None)` | `mcp::widget(&self.mcp, format_args!("Save"), "button", &btn, None)` |
| Small buttons | `mcp::widget(mcp, name, "button", &response, None)` | `mcp::widget(&self.mcp, format_args!("Close Tab"), "button", &btn, None)` |
| Selectable labels | `mcp::widget(mcp, name, "tab", &response, value)` | `mcp::widget(&self.mcp, format_args!("Tab: README"), "tab", &resp, Some("active"))` |
| Checkboxes | `mcp::widget(mcp, name, "checkbox", &response, value)` | `mcp::widget(&self.mcp, format_args!("Dark Mode"), "checkbox", &resp, Some("checked"))` |

A widget that is only a rectangle on screen, with no `Response` to hand over, is registered with `mcp::rect(mcp, name, kind, rect, value)`.

### Register When Stateful (Informational)

Register labels/text that display testable state:

```rust
// Directory with expand/collapse state
mcp::widget(
    &self.mcp,
    format_args!("Toggle: {name}"),
    "button",
    &toggle_response,
    Some(if expanded { "expanded" } else { "collapsed" }),
);
```

### Skip Registration (Decorative)

These typically don't need registration:

- Pure decorative icons/separators
- Static labels without testable state
- Layout helpers (spacers, etc.)

## Icon-Only Buttons

Icon-only buttons like `"⊞"`, `"×"`, `"↻"` don't generate meaningful AccessKit labels. They **always** need explicit registration:

```rust
// WRONG - not testable via MCP
ui.small_button("×").clicked()

// CORRECT - testable
let btn = ui.small_button("×");
mcp::widget(&self.mcp, format_args!("Close Tab"), "button", &btn, None);
if btn.clicked() { ... }
```

## Naming Conventions

Use descriptive, unique names that identify the widget's purpose:

| Pattern | Example |
| --------- | --------- |
| Action buttons | `"Save"`, `"Close"`, `"Refresh"` |
| Toggle buttons | `"Toggle: Dark Mode"`, `"Toggle: Outline"` |
| Tab buttons | `"Tab: filename.md"` |
| File entries | `"File: filename.md"` |
| Directory entries | `"Directory: dirname"` |
| Panel buttons | `"Explorer: Expand All"`, `"Outline: Collapse All"` |

## Feature Flag Handling

Never write `#[cfg(feature = "mcp")]` at a call site. The seam in `src/app/mcp/mod.rs` owns the feature, so a registration is one unconditional call:

```rust
let btn = ui.button("Save");
mcp::widget(&self.mcp, format_args!("Save"), "button", &btn, None);
if btn.clicked() {
    save_file();
}
```

A function that has no `self` takes the bridge as a `&Mcp` argument (`&self.mcp` at its caller) and registers the same way.

## Testing Your Widgets

After adding widgets, verify they're testable:

1. Run with MCP: `cargo run --features mcp -- file.md`
2. Connect: `egui_connect({ host: "127.0.0.1", port: 9877 })`
3. Snapshot: `egui_snapshot()` - verify your widget appears with a ref
4. Click: `egui_click({ ref: "nXX" })` - verify it works

## Checklist for New UI Code

- [ ] Interactive widgets have `mcp::widget()` or `mcp::rect()` calls, with no `#[cfg]` at the call site
- [ ] Icon-only buttons have descriptive names
- [ ] Stateful widgets include value parameter (e.g., "expanded", "active")
- [ ] Names are unique and descriptive
- [ ] Tested widget appears in `egui_snapshot()` output
