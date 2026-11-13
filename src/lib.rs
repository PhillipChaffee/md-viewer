//! A fast markdown viewer with tabs, a file explorer, and live reload.
//!
//! The `md-viewer` binary is a thin caller of [`run`]. Every module is
//! private to this crate.

use std::io;

use clap::Parser;

use crate::cli::{
    app_creator, dead_wayland_variables, launch, launched_from_terminal, native_options,
    spawn_detached, Args, APP_ID,
};

mod app;
mod cli;
mod diagram;
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
mod theme;
mod watcher;

/// Runs `md-viewer` as the process it was started in: wires the process's
/// own arguments, streams, environment, executable, and the native event
/// loop into `cli`, which holds every decision made with them.
///
/// # Errors
///
/// Returns the error eframe reports when the window cannot be opened.
pub fn run() -> eframe::Result<()> {
    env_logger::init();

    let args = Args::parse();
    let from_terminal = launched_from_terminal([&io::stdin(), &io::stdout(), &io::stderr()]);
    let respawn = || spawn_detached(std::env::current_exe()?, std::env::args_os().collect());

    launch(&args, from_terminal, &respawn, &|args| {
        for name in dead_wayland_variables(&std::env::var_os) {
            std::env::remove_var(name);
        }
        eframe::run_native(APP_ID, native_options(), app_creator(args))
    })
}
