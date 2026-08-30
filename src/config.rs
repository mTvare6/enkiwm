use directories::ProjectDirs;
use serde::Deserialize;
use smithay::reexports::calloop::{
    channel::{self, Event},
    LoopHandle,
};
use std::{fs::File, io::Read, path::PathBuf, time::Duration};

use notify_debouncer_mini::{
    new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer,
};

#[derive(Deserialize, Default)]
pub struct Config {
    pub terminal: Option<String>,
}

impl Config {
    pub fn reload() -> Self {
        get_config_text().and_then(|buf| toml::from_str(&buf).ok()).unwrap_or_default()
    }

    pub fn terminal(&self) -> String {
        self.terminal.clone().unwrap_or_else(|| String::from("kitty"))
    }
}

pub fn watch<D, F: FnMut(&mut D) + 'static>(loop_handle: LoopHandle<D>, mut on_change: F) -> Option<Debouncer<RecommendedWatcher>> {
    let config_file = get_config_file()?;
    let config_dir = config_file.parent()?;

    let (tx, rx) = channel::channel();
    loop_handle
        .insert_source(rx, move |event, _, state| {
            if let Event::Msg(()) = event {
                on_change(state);
            }
        })
        .ok()?;

    let watch_file = config_file.clone();
    let mut debouncer = new_debouncer(Duration::from_millis(500), move |result: DebounceEventResult| match result {
        Ok(events) if events.iter().any(|event| event.path == watch_file) => {
            let _ = tx.send(());
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(?error, "while watching config file"),
    })
    .ok()?;

    debouncer.watcher().watch(config_dir, RecursiveMode::NonRecursive).ok()?;

    Some(debouncer)
}

fn get_config_file() -> Option<PathBuf> {
    let proj_dir = ProjectDirs::from("org", "enkiwm", "enki")?;
    let config_dir = proj_dir.config_dir();
    std::fs::create_dir_all(config_dir).ok()?;
    Some(config_dir.join("config.toml"))
}

fn get_config_text() -> Option<String> {
    let config_file = get_config_file()?;
    let mut file = if config_file.exists() { File::open(config_file) } else { File::create(config_file) }.ok()?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    Some(buf)
}
