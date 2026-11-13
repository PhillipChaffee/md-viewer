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
mod contents_sidebar;
#[cfg(test)]
mod diagrams;
#[cfg(test)]
mod docs;
#[cfg(test)]
mod editing;
#[cfg(test)]
mod editing_depth;
#[cfg(test)]
mod files_sidebar;
#[cfg(test)]
mod find;
#[cfg(test)]
mod find_depth;
#[cfg(test)]
mod history;
#[cfg(test)]
mod history_depth;
#[cfg(test)]
mod home;
#[cfg(test)]
mod home_depth;
#[cfg(test)]
mod lightbox;
#[cfg(test)]
mod links;
#[cfg(test)]
mod live_reload;
#[cfg(test)]
mod live_reload_depth;
#[cfg(test)]
mod math;
#[cfg(test)]
mod opening;
#[cfg(test)]
mod opening_depth;
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
mod session_depth;
#[cfg(test)]
mod sidebars;
#[cfg(test)]
mod tables;
#[cfg(test)]
mod tabs;
#[cfg(test)]
mod tabs_depth;
#[cfg(test)]
mod themes;
#[cfg(test)]
mod width;
