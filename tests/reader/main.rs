//! Tests of md-viewer as a reader meets it: the whole app, run without a
//! window, driven and observed only through `support`.
//!
//! Nothing in these modules may name egui, eframe, kittest or anything
//! inside the app. They speak to `support::Reader`; see `tests/README.md`.

#[cfg(test)]
mod support;

#[cfg(test)]
mod appearance;
#[cfg(test)]
mod code;
#[cfg(test)]
mod diagrams;
#[cfg(test)]
mod docs;
#[cfg(test)]
mod editing;
#[cfg(test)]
mod find;
#[cfg(test)]
mod history;
#[cfg(test)]
mod home;
#[cfg(test)]
mod lightbox;
#[cfg(test)]
mod links;
#[cfg(test)]
mod live_reload;
#[cfg(test)]
mod math;
#[cfg(test)]
mod opening;
#[cfg(test)]
mod pages;
#[cfg(test)]
mod pictures;
#[cfg(test)]
mod prose;
#[cfg(test)]
mod rendering;
#[cfg(test)]
mod session;
#[cfg(test)]
mod sidebars;
#[cfg(test)]
mod tables;
#[cfg(test)]
mod tabs;
#[cfg(test)]
mod themes;
#[cfg(test)]
mod width;
