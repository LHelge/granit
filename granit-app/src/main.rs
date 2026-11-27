//! Granit's native desktop app, built on GPUI.
//!
//! `granit-app [<cave>]` opens the cave given on the command line, or else
//! the last-open cave, or else asks for one with the folder picker. A cave
//! given on the command line is never remembered as the last-open cave, so
//! testing against a scratch cave leaves the real one alone.
//!
//! Module layout:
//! - [`host`]: the app-wide [`host::Host`] global holding the core state.
//! - [`cave_ops`]: cave operations of the UI on top of the core.
//! - [`events`]: cave-change notifications ([`events::CaveEvents`]).
//! - [`runtime`]: the tokio runtime and its bridge to GPUI tasks.
//! - [`workspace`]: the root view (sidebar and main area).
//! - [`explorer`]: the sidebar's note tree.
//! - [`note_view`]: the main area, the open note in the live-preview editor.
//! - [`chat`]: the agent chat panel; [`agent_ops`]: its agent settings.
//! - [`theme`], [`assets`]: Catppuccin themes; icons.
//! - [`last_cave`]: the remembered last-open cave path.

mod agent_ops;
mod assets;
mod cave_ops;
mod chat;
mod events;
mod explorer;
mod host;
mod last_cave;
mod note_view;
mod runtime;
mod theme;
mod workspace;

use std::path::PathBuf;

use gpui::{App, KeyBinding};
use log::warn;

use host::{Host, HostOptions};
use workspace::StartupCave;

gpui_kit::actions!(granit, [Quit]);

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cave_arg = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .map(|path| std::path::absolute(&path).unwrap_or(path));
    // A cave from the command line must not replace the remembered one.
    let options = HostOptions::from_environment(cave_arg.is_none());

    gpui_kit::application()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            Host::init(options, cx);
            workspace::bind_keys(cx);
            cx.bind_keys([KeyBinding::new(workspace::QUIT_KEY, Quit, None)]);
            cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let startup = startup_cave(cave_arg, cx);
            workspace::open_main_window(startup, cx).expect("failed to open the main window");
            cx.activate(true);
        });
}

/// The cave to open at startup: the command-line one, else the remembered
/// one, else the folder picker. A remembered folder that no longer exists is
/// forgotten; one that fails to open is kept for the next launch.
fn startup_cave(cave_arg: Option<PathBuf>, cx: &App) -> StartupCave {
    if let Some(path) = cave_arg {
        return StartupCave::Open(path);
    }
    let Some(store) = Host::last_cave(cx) else {
        return StartupCave::Pick;
    };
    match store.load() {
        Ok(Some(path)) if path.is_dir() => StartupCave::Open(path),
        Ok(Some(path)) => {
            warn!("The last-open cave {} is gone", path.display());
            if let Err(err) = store.clear() {
                warn!("Failed to forget the last-open cave: {err}");
            }
            StartupCave::Pick
        }
        Ok(None) => StartupCave::Pick,
        Err(err) => {
            warn!("Failed to read the last-open cave, starting with none: {err}");
            StartupCave::None
        }
    }
}

#[cfg(test)]
mod tests;
