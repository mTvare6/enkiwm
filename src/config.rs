use directories::ProjectDirs;
use serde::{de::Error as _, Deserialize, Deserializer};
use smithay::input::keyboard::{xkb, Keysym, ModifiersState};
use smithay::reexports::calloop::{
    channel::{self, Event},
    LoopHandle,
};
use std::{collections::HashMap, fs::File, io::Read, path::PathBuf, time::Duration};

use notify_debouncer_mini::{
    new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer,
};

#[derive(Deserialize, Default)]
pub struct Config {
    pub terminal: Option<String>,
    #[serde(default, deserialize_with = "deserialize_programs")]
    pub programs: HashMap<Keystroke, String>,
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Keystroke {
    pub modifiers: Modifiers,
    pub keysym: Keysym,
}

impl<'de> Deserialize<'de> for Keystroke {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(D::Error::custom)
    }
}

impl std::str::FromStr for Keystroke {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split('+').map(str::trim).filter(|part| !part.is_empty()).peekable();
        let mut modifiers = Modifiers::default();
        let mut key = None;

        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                key = Some(part);
                break;
            }

            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => modifiers.ctrl = true,
                "alt" => modifiers.alt = true,
                "shift" => modifiers.shift = true,
                "logo" | "super" | "meta" => modifiers.logo = true,
                _ => return Err(format!("unknown modifier `{part}` in keystroke `{value}`")),
            }
        }

        let key = key.ok_or_else(|| format!("keystroke `{value}` has no key"))?;
        let mut keysym = xkb::keysym_from_name(key, xkb::KEYSYM_NO_FLAGS);
        if keysym == Keysym::NoSymbol {
            keysym = xkb::keysym_from_name(key, xkb::KEYSYM_CASE_INSENSITIVE);
        }
        if keysym == Keysym::NoSymbol {
            return Err(format!("unknown key `{key}` in keystroke `{value}`"));
        }

        Ok(Self {
            modifiers,
            keysym,
        })
    }
}

fn deserialize_programs<'de, D: Deserializer<'de>>(deserializer: D) -> Result<HashMap<Keystroke, String>, D::Error> {
    let configured = HashMap::<String, Keystroke>::deserialize(deserializer)?;
    let mut programs = HashMap::with_capacity(configured.len());

    for (program, keystroke) in configured {
        if let Some(previous) = programs.insert(keystroke, program.clone()) {
            // Retain serde error types
            // TODO: Rewrite all hastily used Option into Result
            return Err(D::Error::custom(format!("programs `{previous}` and `{program}` use the same keystroke")));
        }
    }

    Ok(programs)
}

impl Keystroke {
    fn from_input(modifiers: &ModifiersState, keysym: Keysym) -> Self {
        Self {
            modifiers: Modifiers {
                ctrl: modifiers.ctrl,
                alt: modifiers.alt,
                shift: modifiers.shift,
                logo: modifiers.logo,
            },
            keysym,
        }
    }
}

impl Config {
    pub fn reload() -> Self {
        get_config_text().and_then(|buf| toml::from_str(&buf).ok()).unwrap_or_default()
    }

    pub fn terminal(&self) -> String {
        self.terminal.clone().unwrap_or_else(|| String::from("kitty"))
    }

    pub fn program_for_keystroke(&self, modifiers: &ModifiersState, keysym: Keysym) -> Option<&str> {
        self.programs.get(&Keystroke::from_input(modifiers, keysym)).map(String::as_str)
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
