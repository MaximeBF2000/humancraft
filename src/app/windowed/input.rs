use std::time::Instant;

use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

use super::constants::SPRINT_DOUBLE_TAP_SECONDS;

#[derive(Debug, Default)]
pub(super) struct InputState {
    pub(super) forward: bool,
    pub(super) backward: bool,
    pub(super) left: bool,
    pub(super) right: bool,
    pub(super) jump: bool,
    pub(super) sneak: bool,
    pub(super) sprint: bool,
    pub(super) last_forward_press: Option<Instant>,
}

impl InputState {
    pub(super) fn handle_key(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        self.handle_logical_key_at(event.logical_key.as_ref(), pressed, Instant::now());

        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::Space => self.jump = pressed,
                KeyCode::ShiftLeft | KeyCode::ShiftRight => self.sneak = pressed,
                _ => {}
            }
        }

        if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Shift)) {
            self.sneak = pressed;
        }
    }

    #[cfg(test)]
    pub(super) fn handle_logical_key(&mut self, key: Key<&str>, pressed: bool) {
        self.handle_logical_key_at(key, pressed, Instant::now());
    }

    pub(super) fn handle_logical_key_at(&mut self, key: Key<&str>, pressed: bool, now: Instant) {
        match key {
            Key::Character(character) => match character.to_lowercase().as_str() {
                "z" => {
                    if pressed && !self.forward {
                        if self.last_forward_press.is_some_and(|last| {
                            now.duration_since(last).as_secs_f32() <= SPRINT_DOUBLE_TAP_SECONDS
                        }) {
                            self.sprint = true;
                        }
                        self.last_forward_press = Some(now);
                    } else if !pressed {
                        self.sprint = false;
                    }
                    self.forward = pressed;
                }
                "s" => self.backward = pressed,
                "q" => self.left = pressed,
                "d" => self.right = pressed,
                _ => {}
            },
            _ => {}
        }
    }

    pub(super) fn clear_movement(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.jump = false;
        self.sneak = false;
        self.sprint = false;
    }
}
