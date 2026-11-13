//! A fast markdown viewer with tabs, a file explorer, and live reload.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;

use crate::cli::{
    fall_back_to_x11_if_wayland_socket_is_dead, launched_from_terminal, run_native_once,
    should_detach, spawn_detached_child, Args,
};

mod app;
mod cli;
mod dialogs;
mod explorer;
mod headers;
mod icons;
mod layout;
mod links;
mod persist;
mod recent;
mod search;
mod system_fonts;
mod tab;
#[cfg(test)]
mod test_support;
mod watcher;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let args = Args::parse();

    if should_detach(&args, launched_from_terminal()) {
        match spawn_detached_child() {
            Ok(()) => return Ok(()),
            // `error` is the one level env_logger prints when `RUST_LOG` is
            // unset, so whoever launched from the terminal still sees why
            // the window stayed attached to it.
            Err(err) => {
                log::error!("Failed to detach md-viewer process: {err}. Running in foreground.");
            }
        }
    }

    fall_back_to_x11_if_wayland_socket_is_dead();
    run_native_once(&args)
}
