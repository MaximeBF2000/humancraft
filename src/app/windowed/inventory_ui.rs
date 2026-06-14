use glam::Vec3;

use crate::engine::world::{Inventory, ItemStack, LootEntity};

use super::camera::Camera;
use super::client_world::ClientWorld;
use super::constants::*;
use super::render_types::Vertex;
use super::texture::{AtlasTile, TextureAtlas};
use super::ui::{UiPoint, UiRect};
use super::ui_builder::UiMeshBuilder;

pub(super) fn build_gameplay_ui_mesh(
    world: &ClientWorld,
    inventory_open: bool,
    aspect: f32,
    selected_hotbar_slot: usize,
    cursor_stack: Option<ItemStack>,
    cursor_point: UiPoint,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut ui = UiMeshBuilder::default();
    if inventory_open {
        ui.rect(UiRect::new(-0.64, -0.52, 0.64, 0.56), [0.03, 0.03, 0.03]);
        ui.rect(UiRect::new(-0.62, -0.50, 0.62, 0.54), [0.58, 0.58, 0.56]);
        ui.rect(UiRect::new(-0.59, -0.43, 0.59, 0.45), [0.43, 0.43, 0.41]);
        ui.rect(UiRect::new(-0.56, -0.40, 0.56, 0.36), [0.50, 0.50, 0.48]);
        ui.center_text(0.0, 0.47, 0.007, [0.18, 0.18, 0.17], "INVENTORY");
        ui.center_text(0.0, -0.455, 0.0048, [0.20, 0.20, 0.19], "E OR ESC TO CLOSE");
        for index in 0..world.player_inventory.slots().len() {
            let rect = inventory_slot_rect(index, true, aspect);
            draw_inventory_slot(&mut ui, rect, index == selected_hotbar_slot);
        }
    } else {
        for index in 0..INVENTORY_HOTBAR_SLOTS {
            draw_inventory_slot(
                &mut ui,
                inventory_slot_rect(index, false, aspect),
                index == selected_hotbar_slot,
            );
        }
        if world.player_inventory.slot(selected_hotbar_slot).is_none() {
            draw_player_arm(&mut ui, aspect);
        }
    }

    for (index, stack) in world.player_inventory.slots().iter().enumerate() {
        if !inventory_open && index >= INVENTORY_HOTBAR_SLOTS {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        let rect = inventory_slot_rect(index, inventory_open, aspect);
        if stack.count > 1 {
            ui.text(
                rect.right - slot_width(rect) * 0.38,
                rect.bottom + slot_height(rect) * 0.30,
                0.0038,
                [0.96, 0.96, 0.90],
                &stack.count.to_string(),
            );
        }
    }

    if inventory_open {
        if let Some(stack) = cursor_stack {
            if stack.count > 1 {
                let rect = carried_item_rect(cursor_point, aspect);
                ui.text(
                    rect.right - slot_width(rect) * 0.38,
                    rect.bottom + slot_height(rect) * 0.30,
                    0.0038,
                    [0.96, 0.96, 0.90],
                    &stack.count.to_string(),
                );
            }
        }
    }

    ui.finish()
}

pub(super) fn build_inventory_icon_mesh(
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    inventory_open: bool,
    aspect: f32,
    selected_hotbar_slot: usize,
    cursor_stack: Option<ItemStack>,
    cursor_point: UiPoint,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (index, stack) in world.player_inventory.slots().iter().enumerate() {
        if !inventory_open && index >= INVENTORY_HOTBAR_SLOTS {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        let Some(definition) = world.items.get(stack.item) else {
            continue;
        };
        let rect = inventory_icon_rect(inventory_slot_rect(index, inventory_open, aspect));
        push_textured_ui_rect(
            &mut vertices,
            &mut indices,
            rect,
            texture_atlas.tile(&definition.texture),
        );
    }
    if !inventory_open {
        if let Some(stack) = world.player_inventory.slot(selected_hotbar_slot) {
            if let Some(definition) = world.items.get(stack.item) {
                push_held_item_mesh(
                    world,
                    texture_atlas,
                    &mut vertices,
                    &mut indices,
                    definition,
                    aspect,
                );
            }
        }
    }
    if inventory_open {
        if let Some(stack) = cursor_stack {
            if let Some(definition) = world.items.get(stack.item) {
                push_textured_ui_rect(
                    &mut vertices,
                    &mut indices,
                    carried_item_rect(cursor_point, aspect),
                    texture_atlas.tile(&definition.texture),
                );
            }
        }
    }
    (vertices, indices)
}

pub(super) fn build_loot_mesh(
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    _camera: &Camera,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for loot in &world.loot_entities {
        let Some(definition) = world.items.get(loot.stack.item) else {
            continue;
        };
        let tile = texture_atlas.tile(&definition.texture);
        let corners = loot_billboard_corners(loot);
        let tex_coords = tile.uv_quad();
        let base = vertices.len() as u32;
        for (index, corner) in corners.into_iter().enumerate() {
            vertices.push(Vertex {
                position: corner.to_array(),
                color: [1.0, 1.0, 1.0],
                tex_coords: tex_coords[index],
            });
        }
        indices.extend_from_slice(&[
            base,
            base + 1,
            base + 2,
            base,
            base + 2,
            base + 3,
            base + 2,
            base + 1,
            base,
            base + 3,
            base + 2,
            base,
        ]);
    }
    (vertices, indices)
}

pub(super) fn loot_billboard_corners(loot: &LootEntity) -> [Vec3; 4] {
    let axis_x = Vec3::new(
        loot.rotation_radians.cos(),
        0.0,
        loot.rotation_radians.sin(),
    ) * LOOT_RENDER_HALF_SIZE;
    let axis_y = Vec3::Y * LOOT_RENDER_HALF_SIZE;
    let center = loot.position + Vec3::Y * LOOT_RENDER_HALF_SIZE;
    [
        center - axis_x - axis_y,
        center + axis_x - axis_y,
        center + axis_x + axis_y,
        center - axis_x + axis_y,
    ]
}

fn draw_inventory_slot(ui: &mut UiMeshBuilder, rect: UiRect, selected: bool) {
    ui.rect(rect, [0.04, 0.04, 0.04]);
    ui.rect(
        inset_rect(rect, 0.004),
        if selected {
            [0.92, 0.94, 0.82]
        } else {
            [0.62, 0.62, 0.60]
        },
    );
    ui.rect(inset_rect(rect, 0.010), [0.28, 0.29, 0.28]);
    ui.rect(
        UiRect::new(
            rect.left + slot_width(rect) * 0.14,
            rect.top - slot_height(rect) * 0.16,
            rect.right - slot_width(rect) * 0.10,
            rect.top - slot_height(rect) * 0.08,
        ),
        [0.40, 0.41, 0.39],
    );
}

fn draw_player_arm(ui: &mut UiMeshBuilder, aspect: f32) {
    for face in player_arm_overlay_faces(aspect) {
        ui.quad(face.positions, face.color);
    }
}

pub(super) fn inventory_slot_rect(index: usize, inventory_open: bool, aspect: f32) -> UiRect {
    let slot_height = 0.112;
    let slot_width = slot_height / aspect.max(0.1);
    let gap_y = 0.012;
    let gap_x = gap_y / aspect.max(0.1);
    if inventory_open {
        let columns = 9;
        let (row, column, top_start) = if index < INVENTORY_HOTBAR_SLOTS {
            (0, index, -0.20)
        } else {
            let inventory_index = index - INVENTORY_HOTBAR_SLOTS;
            (inventory_index / columns, inventory_index % columns, 0.24)
        };
        let total_width = columns as f32 * slot_width + (columns - 1) as f32 * gap_x;
        let left = -total_width * 0.5 + column as f32 * (slot_width + gap_x);
        let top = top_start - row as f32 * (slot_height + gap_y);
        UiRect::new(left, top - slot_height, left + slot_width, top)
    } else {
        let total_width = INVENTORY_HOTBAR_SLOTS as f32 * slot_width
            + (INVENTORY_HOTBAR_SLOTS - 1) as f32 * gap_x;
        let left = -total_width * 0.5 + index as f32 * (slot_width + gap_x);
        UiRect::new(left, -0.95, left + slot_width, -0.95 + slot_height)
    }
}

pub(super) fn inventory_slot_at_point(point: UiPoint, aspect: f32) -> Option<usize> {
    for index in 0..Inventory::player().slot_count() {
        if inventory_slot_rect(index, true, aspect).contains(point) {
            return Some(index);
        }
    }
    None
}

fn inset_rect(rect: UiRect, inset: f32) -> UiRect {
    UiRect::new(
        rect.left + inset,
        rect.bottom + inset,
        rect.right - inset,
        rect.top - inset,
    )
}

fn inventory_icon_rect(rect: UiRect) -> UiRect {
    let width = slot_width(rect);
    let height = slot_height(rect);
    UiRect::new(
        rect.left + width * 0.18,
        rect.bottom + height * 0.34,
        rect.right - width * 0.22,
        rect.top - height * 0.12,
    )
}

fn carried_item_rect(point: UiPoint, aspect: f32) -> UiRect {
    let height = 0.10;
    let width = height / aspect.max(0.1);
    UiRect::new(
        point.x - width * 0.5,
        point.y - height * 0.5,
        point.x + width * 0.5,
        point.y + height * 0.5,
    )
}

fn push_held_item_mesh(
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    item: &crate::engine::world::ItemDefinition,
    aspect: f32,
) {
    if let Some(block) = item
        .place_block
        .as_ref()
        .and_then(|key| world.blocks.get_by_key(key))
        .map(|(_, block)| block)
    {
        push_held_block_mesh(vertices, indices, texture_atlas, block, aspect);
    } else {
        push_held_sprite_mesh(vertices, indices, texture_atlas.tile(&item.texture), aspect);
    }
}

fn push_held_block_mesh(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    texture_atlas: &TextureAtlas,
    block: &crate::engine::world::BlockDefinition,
    aspect: f32,
) {
    let faces = held_block_overlay_faces(aspect);

    push_textured_ui_quad(
        vertices,
        indices,
        faces.front,
        texture_atlas.tile(&block.textures.south),
        [0.95, 0.95, 0.95],
    );
    push_textured_ui_quad(
        vertices,
        indices,
        faces.right,
        texture_atlas.tile(&block.textures.east),
        [0.72, 0.72, 0.72],
    );
    push_textured_ui_quad(
        vertices,
        indices,
        faces.top,
        texture_atlas.tile(&block.textures.top),
        [1.0, 1.0, 1.0],
    );
}

pub(super) struct HeldBlockOverlayFaces {
    pub(super) front: [[f32; 3]; 4],
    pub(super) right: [[f32; 3]; 4],
    pub(super) top: [[f32; 3]; 4],
}

pub(super) fn held_block_overlay_faces(aspect: f32) -> HeldBlockOverlayFaces {
    let scale_x = 1.0 / aspect.max(0.1);
    let adjust = |x: f32, y: f32| [0.72 + (x - 0.72) * scale_x, y, 0.0];
    let front_bottom_left = adjust(0.45, -0.96);
    let front_bottom_right = adjust(0.82, -0.88);
    let front_top_right = adjust(0.82, -0.50);
    let front_top_left = adjust(0.45, -0.58);
    let depth = |point: [f32; 3]| [point[0] + 0.15 * scale_x, point[1] + 0.14, 0.0];
    let back_top_left = depth(front_top_left);
    let back_top_right = depth(front_top_right);
    let back_bottom_right = depth(front_bottom_right);

    HeldBlockOverlayFaces {
        front: [
            front_bottom_left,
            front_bottom_right,
            front_top_right,
            front_top_left,
        ],
        right: [
            front_bottom_right,
            back_bottom_right,
            back_top_right,
            front_top_right,
        ],
        top: [
            front_top_left,
            front_top_right,
            back_top_right,
            back_top_left,
        ],
    }
}

#[derive(Copy, Clone)]
pub(super) struct UiFace {
    pub(super) positions: [[f32; 3]; 4],
    color: [f32; 3],
}

pub(super) fn player_arm_overlay_faces(aspect: f32) -> [UiFace; 3] {
    let scale_x = 1.0 / aspect.max(0.1);
    let adjust = |x: f32, y: f32| [0.78 + (x - 0.78) * scale_x, y, 0.0];
    let wrist_left = adjust(0.64, -0.99);
    let wrist_right = adjust(0.91, -0.99);
    let elbow_left = adjust(0.56, -0.61);
    let elbow_right = adjust(0.78, -0.49);
    let depth = |point: [f32; 3]| [point[0] + 0.18 * scale_x, point[1] + 0.07, 0.0];
    let wrist_right_back = depth(wrist_right);
    let elbow_right_back = depth(elbow_right);
    let elbow_left_back = depth(elbow_left);

    [
        UiFace {
            positions: [wrist_left, wrist_right, elbow_right, elbow_left],
            color: [0.70, 0.46, 0.30],
        },
        UiFace {
            positions: [wrist_right, wrist_right_back, elbow_right_back, elbow_right],
            color: [0.50, 0.31, 0.20],
        },
        UiFace {
            positions: [elbow_left, elbow_right, elbow_right_back, elbow_left_back],
            color: [0.82, 0.57, 0.38],
        },
    ]
}

fn push_held_sprite_mesh(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    tile: AtlasTile,
    aspect: f32,
) {
    let scale_x = 1.0 / aspect.max(0.1);
    let adjust = |x: f32, y: f32| [0.76 + (x - 0.76) * scale_x, y, 0.0];
    push_textured_ui_quad(
        vertices,
        indices,
        [
            adjust(0.58, -0.94),
            adjust(0.96, -0.80),
            adjust(0.84, -0.39),
            adjust(0.46, -0.53),
        ],
        tile,
        [1.0, 1.0, 1.0],
    );
}

pub(super) fn slot_width(rect: UiRect) -> f32 {
    rect.right - rect.left
}

pub(super) fn slot_height(rect: UiRect) -> f32 {
    rect.top - rect.bottom
}

fn push_textured_ui_rect(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    rect: UiRect,
    tile: AtlasTile,
) {
    let tex_coords = tile.uv_quad();
    let base = vertices.len() as u32;
    let positions = [
        [rect.left, rect.bottom, 0.0],
        [rect.right, rect.bottom, 0.0],
        [rect.right, rect.top, 0.0],
        [rect.left, rect.top, 0.0],
    ];
    for index in 0..4 {
        vertices.push(Vertex {
            position: positions[index],
            color: [1.0, 1.0, 1.0],
            tex_coords: tex_coords[index],
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_textured_ui_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    positions: [[f32; 3]; 4],
    tile: AtlasTile,
    color: [f32; 3],
) {
    let tex_coords = tile.uv_quad();
    let base = vertices.len() as u32;
    for index in 0..4 {
        vertices.push(Vertex {
            position: positions[index],
            color,
            tex_coords: tex_coords[index],
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
