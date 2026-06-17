use winit::event::MouseButton;

use super::constants::BLOCK_INTERACTION_REPEAT_SECONDS;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum AppMode {
    MainMenu,
    ManageWorlds,
    ConfigNewWorld,
    RenamingWorld,
    Settings,
    Shortcuts,
    InGame,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum ShortcutAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Sneak,
    Inventory,
    Pause,
    Drop,
    HotbarPrevious,
    HotbarNext,
    Hotbar1,
    Hotbar2,
    Hotbar3,
    Hotbar4,
    Hotbar5,
    Hotbar6,
    Hotbar7,
    Hotbar8,
    Hotbar9,
}

pub(super) const SHORTCUT_ACTIONS: [ShortcutAction; 20] = [
    ShortcutAction::MoveForward,
    ShortcutAction::MoveBackward,
    ShortcutAction::MoveLeft,
    ShortcutAction::MoveRight,
    ShortcutAction::Jump,
    ShortcutAction::Sneak,
    ShortcutAction::Inventory,
    ShortcutAction::Pause,
    ShortcutAction::Drop,
    ShortcutAction::HotbarPrevious,
    ShortcutAction::HotbarNext,
    ShortcutAction::Hotbar1,
    ShortcutAction::Hotbar2,
    ShortcutAction::Hotbar3,
    ShortcutAction::Hotbar4,
    ShortcutAction::Hotbar5,
    ShortcutAction::Hotbar6,
    ShortcutAction::Hotbar7,
    ShortcutAction::Hotbar8,
    ShortcutAction::Hotbar9,
];

impl ShortcutAction {
    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::MoveForward => "move_forward",
            Self::MoveBackward => "move_backward",
            Self::MoveLeft => "move_left",
            Self::MoveRight => "move_right",
            Self::Jump => "jump",
            Self::Sneak => "crouch",
            Self::Inventory => "inventory",
            Self::Pause => "pause",
            Self::Drop => "drop",
            Self::HotbarPrevious => "hotbar_previous",
            Self::HotbarNext => "hotbar_next",
            Self::Hotbar1 => "hotbar_1",
            Self::Hotbar2 => "hotbar_2",
            Self::Hotbar3 => "hotbar_3",
            Self::Hotbar4 => "hotbar_4",
            Self::Hotbar5 => "hotbar_5",
            Self::Hotbar6 => "hotbar_6",
            Self::Hotbar7 => "hotbar_7",
            Self::Hotbar8 => "hotbar_8",
            Self::Hotbar9 => "hotbar_9",
        }
    }

    pub(super) fn from_key(key: &str) -> Option<Self> {
        SHORTCUT_ACTIONS
            .iter()
            .copied()
            .find(|action| action.key() == key)
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::MoveForward => "MOVE FORWARD",
            Self::MoveBackward => "MOVE BACKWARD",
            Self::MoveLeft => "MOVE LEFT",
            Self::MoveRight => "MOVE RIGHT",
            Self::Jump => "JUMP",
            Self::Sneak => "CROUCH",
            Self::Inventory => "INVENTORY",
            Self::Pause => "PAUSE",
            Self::Drop => "DROP ITEM",
            Self::HotbarPrevious => "HOTBAR PREV",
            Self::HotbarNext => "HOTBAR NEXT",
            Self::Hotbar1 => "HOTBAR 1",
            Self::Hotbar2 => "HOTBAR 2",
            Self::Hotbar3 => "HOTBAR 3",
            Self::Hotbar4 => "HOTBAR 4",
            Self::Hotbar5 => "HOTBAR 5",
            Self::Hotbar6 => "HOTBAR 6",
            Self::Hotbar7 => "HOTBAR 7",
            Self::Hotbar8 => "HOTBAR 8",
            Self::Hotbar9 => "HOTBAR 9",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::MoveForward => 0,
            Self::MoveBackward => 1,
            Self::MoveLeft => 2,
            Self::MoveRight => 3,
            Self::Jump => 4,
            Self::Sneak => 5,
            Self::Inventory => 6,
            Self::Pause => 7,
            Self::Drop => 8,
            Self::HotbarPrevious => 9,
            Self::HotbarNext => 10,
            Self::Hotbar1 => 11,
            Self::Hotbar2 => 12,
            Self::Hotbar3 => 13,
            Self::Hotbar4 => 14,
            Self::Hotbar5 => 15,
            Self::Hotbar6 => 16,
            Self::Hotbar7 => 17,
            Self::Hotbar8 => 18,
            Self::Hotbar9 => 19,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KeyBindings {
    labels: [String; SHORTCUT_ACTIONS.len()],
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            labels: [
                "Z".to_string(),
                "S".to_string(),
                "Q".to_string(),
                "D".to_string(),
                "SPACE".to_string(),
                "SHIFT".to_string(),
                "E".to_string(),
                "ESC".to_string(),
                "Q".to_string(),
                "LEFT".to_string(),
                "RIGHT".to_string(),
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
                "6".to_string(),
                "7".to_string(),
                "8".to_string(),
                "9".to_string(),
            ],
        }
    }
}

impl KeyBindings {
    pub(super) fn label(&self, action: ShortcutAction) -> &str {
        &self.labels[action.index()]
    }

    pub(super) fn set_label(&mut self, action: ShortcutAction, label: String) {
        self.labels[action.index()] = label;
    }

    pub(super) fn matches(&self, action: ShortcutAction, label: &str) -> bool {
        self.label(action).eq_ignore_ascii_case(label)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (ShortcutAction, &str)> {
        SHORTCUT_ACTIONS
            .iter()
            .copied()
            .map(|action| (action, self.label(action)))
    }
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
