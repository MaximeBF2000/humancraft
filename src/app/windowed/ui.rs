use winit::dpi::{PhysicalPosition, PhysicalSize};

use super::render_types::Vertex;
use super::ui_builder::UiMeshBuilder;
use super::{AppMode, ConfigField, RenderState};

#[derive(Debug, Copy, Clone)]
pub(super) struct UiPoint {
    pub(super) x: f32,
    pub(super) y: f32,
}

pub(super) fn cursor_to_ui_point(
    position: PhysicalPosition<f64>,
    size: PhysicalSize<u32>,
) -> UiPoint {
    UiPoint {
        x: (position.x / size.width.max(1) as f64 * 2.0 - 1.0) as f32,
        y: (1.0 - position.y / size.height.max(1) as f64 * 2.0) as f32,
    }
}

#[derive(Debug, Copy, Clone)]
pub(super) struct UiRect {
    pub(super) left: f32,
    pub(super) right: f32,
    pub(super) bottom: f32,
    pub(super) top: f32,
}

impl UiRect {
    pub(super) const fn new(left: f32, bottom: f32, right: f32, top: f32) -> Self {
        Self {
            left,
            right,
            bottom,
            top,
        }
    }

    pub(super) fn contains(self, point: UiPoint) -> bool {
        point.x >= self.left
            && point.x <= self.right
            && point.y >= self.bottom
            && point.y <= self.top
    }

    pub(super) fn center_x(self) -> f32 {
        (self.left + self.right) * 0.5
    }

    pub(super) fn center_y(self) -> f32 {
        (self.bottom + self.top) * 0.5
    }
}

pub(super) const UI_MAIN_PLAY: UiRect = UiRect::new(-0.28, -0.05, 0.28, 0.08);
pub(super) const UI_WORLDS_PLAY: UiRect = UiRect::new(0.38, 0.42, 0.78, 0.54);
pub(super) const UI_WORLDS_NEW: UiRect = UiRect::new(0.38, 0.25, 0.78, 0.37);
pub(super) const UI_WORLDS_RENAME: UiRect = UiRect::new(0.38, 0.08, 0.78, 0.20);
pub(super) const UI_WORLDS_DELETE: UiRect = UiRect::new(0.38, -0.09, 0.78, 0.03);
pub(super) const UI_WORLDS_BACK: UiRect = UiRect::new(0.38, -0.46, 0.78, -0.34);
pub(super) const UI_CONFIG_NAME_FIELD: UiRect = UiRect::new(-0.30, 0.22, 0.54, 0.34);
pub(super) const UI_CONFIG_SEED_FIELD: UiRect = UiRect::new(-0.30, -0.02, 0.54, 0.10);
pub(super) const UI_CONFIG_CREATE: UiRect = UiRect::new(-0.30, -0.32, 0.04, -0.20);
pub(super) const UI_CONFIG_BACK: UiRect = UiRect::new(0.20, -0.32, 0.54, -0.20);
pub(super) const UI_RENAME_SAVE: UiRect = UiRect::new(-0.30, -0.20, 0.04, -0.08);
pub(super) const UI_RENAME_BACK: UiRect = UiRect::new(0.20, -0.20, 0.54, -0.08);
pub(super) const UI_PAUSE_KEEP_PLAYING: UiRect = UiRect::new(-0.46, -0.08, -0.02, 0.05);
pub(super) const UI_PAUSE_SAVE_QUIT: UiRect = UiRect::new(0.02, -0.08, 0.46, 0.05);

pub(super) fn world_list_hit_index(point: UiPoint, world_count: usize) -> Option<usize> {
    let count = world_count.min(7);
    for index in 0..count {
        let top = 0.45 - index as f32 * 0.13;
        let rect = UiRect::new(-0.78, top - 0.10, 0.26, top);
        if rect.contains(point) {
            return Some(index);
        }
    }
    None
}

pub(super) fn build_menu_mesh(state: &RenderState) -> (Vec<Vertex>, Vec<u32>) {
    let mut ui = UiMeshBuilder::default();
    match state.mode {
        AppMode::MainMenu => {
            ui.rect(UiRect::new(-1.0, -1.0, 1.0, 1.0), [0.11, 0.13, 0.14]);
            ui.center_text(0.0, 0.52, 0.018, [0.92, 0.92, 0.88], "HUMANCRAFT");
            ui.button(UI_MAIN_PLAY, "PLAY", false);
        }
        AppMode::ManageWorlds => {
            ui.rect(UiRect::new(-1.0, -1.0, 1.0, 1.0), [0.10, 0.12, 0.13]);
            ui.center_text(0.0, 0.72, 0.012, [0.92, 0.92, 0.88], "MANAGE WORLDS");
            if state.worlds.is_empty() {
                ui.text(-0.76, 0.40, 0.007, [0.82, 0.82, 0.78], "NO WORLDS YET");
                ui.text(
                    -0.76,
                    0.28,
                    0.006,
                    [0.64, 0.66, 0.66],
                    "CREATE A WORLD TO START",
                );
            } else {
                for (index, world) in state.worlds.iter().take(7).enumerate() {
                    let top = 0.45 - index as f32 * 0.13;
                    let rect = UiRect::new(-0.78, top - 0.10, 0.26, top);
                    ui.rect(
                        rect,
                        if index == state.selected_world {
                            [0.32, 0.36, 0.34]
                        } else {
                            [0.18, 0.20, 0.20]
                        },
                    );
                    ui.text(
                        rect.left + 0.03,
                        rect.top - 0.028,
                        0.0048,
                        [0.95, 0.95, 0.90],
                        &world.name,
                    );
                    ui.text(
                        rect.left + 0.03,
                        rect.bottom + 0.030,
                        0.0036,
                        [0.66, 0.68, 0.68],
                        &format!("SEED {}", world.seed),
                    );
                }
            }
            ui.button(UI_WORLDS_PLAY, "PLAY", false);
            ui.button(UI_WORLDS_NEW, "NEW WORLD", false);
            ui.button(UI_WORLDS_RENAME, "RENAME", state.worlds.is_empty());
            ui.button(UI_WORLDS_DELETE, "DELETE", state.worlds.is_empty());
            ui.button(UI_WORLDS_BACK, "BACK", false);
        }
        AppMode::ConfigNewWorld => {
            ui.rect(UiRect::new(-1.0, -1.0, 1.0, 1.0), [0.10, 0.12, 0.13]);
            ui.center_text(0.0, 0.68, 0.012, [0.92, 0.92, 0.88], "CONFIG NEW WORLD");
            ui.text(-0.54, 0.38, 0.006, [0.85, 0.85, 0.80], "WORLD NAME");
            ui.field(
                UI_CONFIG_NAME_FIELD,
                &state.new_world_config.name,
                state.new_world_config.focused == ConfigField::Name,
            );
            ui.text(-0.54, 0.14, 0.006, [0.85, 0.85, 0.80], "SEED");
            ui.field(
                UI_CONFIG_SEED_FIELD,
                if state.new_world_config.seed.is_empty() {
                    "AUTO"
                } else {
                    &state.new_world_config.seed
                },
                state.new_world_config.focused == ConfigField::Seed,
            );
            ui.text(
                -0.54,
                -0.08,
                0.005,
                [0.64, 0.66, 0.66],
                "SAME NUMERIC SEED RECREATES TERRAIN",
            );
            ui.button(UI_CONFIG_CREATE, "CREATE", false);
            ui.button(UI_CONFIG_BACK, "BACK", false);
        }
        AppMode::RenamingWorld => {
            ui.rect(UiRect::new(-1.0, -1.0, 1.0, 1.0), [0.10, 0.12, 0.13]);
            ui.center_text(0.0, 0.58, 0.012, [0.92, 0.92, 0.88], "RENAME WORLD");
            ui.field(
                UI_CONFIG_NAME_FIELD,
                self_clamped_text(state.text_entry.display()),
                true,
            );
            ui.button(UI_RENAME_SAVE, "SAVE", false);
            ui.button(UI_RENAME_BACK, "BACK", false);
        }
        AppMode::InGame => {
            ui.rect(UiRect::new(-0.52, -0.22, 0.52, 0.30), [0.08, 0.09, 0.10]);
            ui.center_text(0.0, 0.16, 0.012, [0.92, 0.92, 0.88], "PAUSED");
            ui.button(UI_PAUSE_KEEP_PLAYING, "KEEP PLAYING", false);
            ui.button(UI_PAUSE_SAVE_QUIT, "SAVE & QUIT", false);
        }
    }
    ui.finish()
}

fn self_clamped_text(text: &str) -> &str {
    text
}
