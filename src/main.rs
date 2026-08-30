// SPDX-License-Identifier: MPL-2.0

#![allow(irrefutable_let_patterns)]

mod command;
mod cursor;
mod grabs;
mod handlers;
mod layout;
mod math;
mod state;

use std::io::IsTerminal;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::State;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let mut event_loop: EventLoop<State> = EventLoop::try_new()?;

    let display: Display<State> = Display::new()?;

    let mut state = State::new(&mut event_loop, display);

    std::env::remove_var("DISPLAY");
    std::env::set_var("WAYLAND_DISPLAY", &state.enki.socket_name);
    std::env::set_var("OZONE_PLATFORM", "wayland");
    std::env::set_var("QT_QPA_PLATFORM", "wayland");

    spawn_client();

    event_loop.run(None, &mut state, move |state| {
        let _ = state.enki.display_handle.flush_clients();
        state.enki.space.refresh();
        state.enki.popups.cleanup();
        state.enki.grid.cleanup();
        state.backend.event_loop_tick(&state.enki);
    })?;

    Ok(())
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        let is_tty = std::io::stdout().is_terminal();
        tracing_subscriber::fmt().with_ansi(is_tty).init();
    }
}

fn spawn_client() {
    let mut args = std::env::args().skip(1);
    let flag = args.next();
    let arg = args.next();

    match (flag.as_deref(), arg) {
        (Some("-c") | Some("--command"), Some(command)) => {
            std::process::Command::new(command).spawn().ok();
        }
        _ => {
            std::process::Command::new("kitty").spawn().ok();
        }
    }
}
