use winit::event::MouseButton;

use super::constants::BLOCK_INTERACTION_REPEAT_SECONDS;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum AppMode {
    MainMenu,
    ManageWorlds,
    ConfigNewWorld,
    RenamingWorld,
    InGame,
}

#[derive(Debug, Default, Copy, Clone)]
pub(super) struct HeldBlockInteraction {
    button: Option<MouseButton>,
    repeat_seconds: f32,
}

impl HeldBlockInteraction {
    pub(super) fn press(&mut self, button: MouseButton) {
        self.button = Some(button);
        self.repeat_seconds = 0.0;
    }

    pub(super) fn release(&mut self, button: MouseButton) {
        if self.button == Some(button) {
            self.clear();
        }
    }

    pub(super) fn clear(&mut self) {
        self.button = None;
        self.repeat_seconds = 0.0;
    }

    pub(super) fn repeat_button(&mut self, delta_seconds: f32) -> Option<MouseButton> {
        let button = self.button?;
        if button == MouseButton::Left {
            return None;
        }
        self.repeat_seconds += delta_seconds;
        if self.repeat_seconds < BLOCK_INTERACTION_REPEAT_SECONDS {
            return None;
        }
        self.repeat_seconds = 0.0;
        Some(button)
    }

    pub(super) fn is_held(&self, button: MouseButton) -> bool {
        self.button == Some(button)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum ConfigField {
    Name,
    Seed,
}

#[derive(Debug, Clone)]
pub(super) struct NewWorldConfig {
    pub(super) name: String,
    pub(super) seed: String,
    pub(super) focused: ConfigField,
}

impl Default for NewWorldConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            seed: String::new(),
            focused: ConfigField::Name,
        }
    }
}

impl NewWorldConfig {
    pub(super) fn start(&mut self, fallback_name: String) {
        self.name = fallback_name;
        self.seed.clear();
        self.focused = ConfigField::Name;
    }

    pub(super) fn push(&mut self, text: &str) {
        let target = match self.focused {
            ConfigField::Name => &mut self.name,
            ConfigField::Seed => &mut self.seed,
        };
        for character in text.chars() {
            if !character.is_control() && target.chars().count() < 64 {
                target.push(character);
            }
        }
    }

    pub(super) fn pop(&mut self) {
        match self.focused {
            ConfigField::Name => {
                self.name.pop();
            }
            ConfigField::Seed => {
                self.seed.pop();
            }
        }
    }

    pub(super) fn toggle_focus(&mut self) {
        self.focused = match self.focused {
            ConfigField::Name => ConfigField::Seed,
            ConfigField::Seed => ConfigField::Name,
        };
    }

    pub(super) fn final_name(&self) -> String {
        let trimmed = self.name.trim();
        if trimmed.is_empty() {
            "New World".to_string()
        } else {
            trimmed.to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct TextEntry {
    value: String,
    fallback: String,
}

impl Default for TextEntry {
    fn default() -> Self {
        Self {
            value: String::new(),
            fallback: String::new(),
        }
    }
}

impl TextEntry {
    pub(super) fn start(&mut self, fallback: impl Into<String>) {
        self.value.clear();
        self.fallback = fallback.into();
    }

    pub(super) fn push(&mut self, text: &str) {
        for character in text.chars() {
            if !character.is_control() && self.value.chars().count() < 64 {
                self.value.push(character);
            }
        }
    }

    pub(super) fn pop(&mut self) {
        self.value.pop();
    }

    pub(super) fn finish(&self) -> String {
        let trimmed = self.value.trim();
        if trimmed.is_empty() {
            self.fallback.clone()
        } else {
            trimmed.to_string()
        }
    }

    pub(super) fn display(&self) -> &str {
        if self.value.is_empty() {
            &self.fallback
        } else {
            &self.value
        }
    }
}
