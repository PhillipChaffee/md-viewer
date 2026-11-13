//! End-to-end tests of the `md-viewer` binary itself: its command line,
//! and its real window on a private X display.
//!
//! These modules speak only to `support`; see `tests/README.md`.

#[cfg(test)]
mod support;

#[cfg(test)]
mod cli;
#[cfg(test)]
mod cli_depth;
#[cfg(all(test, target_os = "linux"))]
mod terminal;
#[cfg(all(test, target_os = "linux"))]
mod window;
#[cfg(all(test, target_os = "linux"))]
mod window_depth;
