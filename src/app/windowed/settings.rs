use std::fs;
use std::io;
use std::path::PathBuf;

use super::session::{KeyBindings, ShortcutAction};

const SETTINGS_FILE: &str = "settings.txt";

#[derive(Debug, Clone)]
pub(super) struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub(super) fn default() -> Self {
        Self {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("saves")
                .join(SETTINGS_FILE),
        }
    }

    #[cfg(test)]
    pub(super) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(super) fn load_key_bindings(&self) -> KeyBindings {
        match fs::read_to_string(&self.path) {
            Ok(contents) => parse_key_bindings(&contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => KeyBindings::default(),
            Err(error) => {
                eprintln!("settings load error: {error}");
                KeyBindings::default()
            }
        }
    }

    pub(super) fn save_key_bindings(&self, bindings: &KeyBindings) {
        if let Some(parent) = self.path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            eprintln!("settings directory error: {error}");
            return;
        }
        if let Err(error) = fs::write(&self.path, serialize_key_bindings(bindings)) {
            eprintln!("settings save error: {error}");
        }
    }
}

fn parse_key_bindings(contents: &str) -> KeyBindings {
    let mut bindings = KeyBindings::default();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "version=1" {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let Some(action_key) = key.strip_prefix("shortcut.") else {
            continue;
        };
        let Some(action) = ShortcutAction::from_key(action_key) else {
            continue;
        };
        let label = value.trim();
        if !label.is_empty() && label.chars().all(|character| !character.is_control()) {
            bindings.set_label(action, label.to_ascii_uppercase());
        }
    }
    bindings
}

fn serialize_key_bindings(bindings: &KeyBindings) -> String {
    let mut output = String::from("version=1\n");
    for (action, label) in bindings.iter() {
        output.push_str("shortcut.");
        output.push_str(action.key());
        output.push('=');
        output.push_str(label);
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_shortcut_bindings() {
        let path = std::env::temp_dir().join(format!(
            "humancraft-settings-test-{}.txt",
            std::process::id()
        ));
        let store = SettingsStore::new(&path);
        let mut bindings = KeyBindings::default();
        bindings.set_label(ShortcutAction::MoveForward, "W".to_string());
        bindings.set_label(ShortcutAction::Drop, "R".to_string());

        store.save_key_bindings(&bindings);
        let loaded = store.load_key_bindings();
        let _ = fs::remove_file(path);

        assert_eq!(loaded.label(ShortcutAction::MoveForward), "W");
        assert_eq!(loaded.label(ShortcutAction::Drop), "R");
    }

    #[test]
    fn settings_ignore_unknown_or_missing_shortcut_keys() {
        let loaded = parse_key_bindings(
            "version=1\nshortcut.move_forward=W\nshortcut.unknown=NOPE\nbroken\n",
        );

        assert_eq!(loaded.label(ShortcutAction::MoveForward), "W");
        assert_eq!(loaded.label(ShortcutAction::MoveBackward), "S");
    }
}
