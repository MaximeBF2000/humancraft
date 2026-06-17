use glam::Vec3;

use crate::engine::world::{Inventory, ItemDefinition, ItemStack, LootEntity};

use super::camera::Camera;
use super::client_world::ClientWorld;
use super::constants::*;
use super::render_types::Vertex;
use super::texture::{AtlasTile, TextureAtlas, player_hand_texture_key};
use super::ui::{UiPoint, UiRect};
use super::ui_builder::UiMeshBuilder;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum CraftingUiKind {
    Inventory,
    Table,
}

const GUI_SOURCE_WIDTH: f32 = 176.0;
const GUI_SOURCE_HEIGHT: f32 = 166.0;
const GUI_PANEL_HEIGHT: f32 = 1.24;
const GUI_SLOT_SIZE: f32 = 18.0;
const CLOSED_HOTBAR_SLOT_HEIGHT: f32 = 0.112;
const INVENTORY_GAP_Y: f32 = 0.012;

pub(super) fn build_gameplay_ui_mesh(
    world: &ClientWorld,
    inventory_open: bool,
    crafting_kind: CraftingUiKind,
    crafting_grid: &Inventory,
    crafting_result: Option<ItemStack>,
    aspect: f32,
    selected_hotbar_slot: usize,
    cursor_stack: Option<ItemStack>,
    cursor_point: UiPoint,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut ui = UiMeshBuilder::default();
    if inventory_open {
        draw_inventory_panel(&mut ui, crafting_kind, aspect);
        for index in 0..crafting_grid.slot_count() {
            draw_inventory_slot(
                &mut ui,
                crafting_input_slot_rect(index, crafting_kind, aspect),
                false,
            );
        }
        draw_inventory_slot(
            &mut ui,
            crafting_result_slot_rect(crafting_kind, aspect),
            crafting_result.is_some(),
        );
        draw_crafting_arrow(&mut ui, crafting_kind, aspect);
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
            draw_stack_count(&mut ui, rect, stack.count);
        }
    }

    if inventory_open {
        for (index, stack) in crafting_grid.slots().iter().enumerate() {
            let Some(stack) = stack else {
                continue;
            };
            let rect = crafting_input_slot_rect(index, crafting_kind, aspect);
            if stack.count > 1 {
                draw_stack_count(&mut ui, rect, stack.count);
            }
        }
        if let Some(stack) = crafting_result {
            if stack.count > 1 {
                draw_stack_count(
                    &mut ui,
                    crafting_result_slot_rect(crafting_kind, aspect),
                    stack.count,
                );
            }
        }
        if let Some(stack) = cursor_stack {
            if stack.count > 1 {
                draw_stack_count(
                    &mut ui,
                    carried_item_rect(cursor_point, aspect),
                    stack.count,
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
    crafting_kind: CraftingUiKind,
    crafting_grid: &Inventory,
    crafting_result: Option<ItemStack>,
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
        push_item_icon_mesh(
            world,
            texture_atlas,
            &mut vertices,
            &mut indices,
            definition,
            inventory_icon_rect(inventory_slot_rect(index, inventory_open, aspect)),
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
        } else {
            push_player_hand_mesh(texture_atlas, &mut vertices, &mut indices, aspect);
        }
    }
    if inventory_open {
        for (index, stack) in crafting_grid.slots().iter().enumerate() {
            let Some(stack) = stack else {
                continue;
            };
            let Some(definition) = world.items.get(stack.item) else {
                continue;
            };
            push_item_icon_mesh(
                world,
                texture_atlas,
                &mut vertices,
                &mut indices,
                definition,
                inventory_icon_rect(crafting_input_slot_rect(index, crafting_kind, aspect)),
            );
        }
        if let Some(stack) = crafting_result {
            if let Some(definition) = world.items.get(stack.item) {
                push_item_icon_mesh(
                    world,
                    texture_atlas,
                    &mut vertices,
                    &mut indices,
                    definition,
                    inventory_icon_rect(crafting_result_slot_rect(crafting_kind, aspect)),
                );
            }
        }
        if let Some(stack) = cursor_stack {
            if let Some(definition) = world.items.get(stack.item) {
                push_item_icon_mesh(
                    world,
                    texture_atlas,
                    &mut vertices,
                    &mut indices,
                    definition,
                    carried_item_rect(cursor_point, aspect),
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
        if let Some(block) = definition
            .place_block
            .as_ref()
            .and_then(|key| world.blocks.get_by_key(key))
            .map(|(_, block)| block)
        {
            push_loot_block_mesh(&mut vertices, &mut indices, texture_atlas, block, loot);
            continue;
        }
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

fn push_loot_block_mesh(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    texture_atlas: &TextureAtlas,
    block: &crate::engine::world::BlockDefinition,
    loot: &LootEntity,
) {
    let half = LOOT_RENDER_HALF_SIZE * 0.72;
    let axis_x = Vec3::new(
        loot.rotation_radians.cos(),
        0.0,
        loot.rotation_radians.sin(),
    ) * half;
    let axis_z = Vec3::new(
        -loot.rotation_radians.sin(),
        0.0,
        loot.rotation_radians.cos(),
    ) * half;
    let axis_y = Vec3::Y * half;
    let center = loot.position + Vec3::Y * half;

    push_world_textured_quad(
        vertices,
        indices,
        [
            center - axis_x - axis_y + axis_z,
            center + axis_x - axis_y + axis_z,
            center + axis_x + axis_y + axis_z,
            center - axis_x + axis_y + axis_z,
        ],
        texture_atlas.tile(&block.textures.south),
        [0.95, 0.95, 0.95],
    );
    push_world_textured_quad(
        vertices,
        indices,
        [
            center + axis_x - axis_y - axis_z,
            center - axis_x - axis_y - axis_z,
            center - axis_x + axis_y - axis_z,
            center + axis_x + axis_y - axis_z,
        ],
        texture_atlas.tile(&block.textures.north),
        [0.85, 0.85, 0.85],
    );
    push_world_textured_quad(
        vertices,
        indices,
        [
            center + axis_x - axis_y + axis_z,
            center + axis_x - axis_y - axis_z,
            center + axis_x + axis_y - axis_z,
            center + axis_x + axis_y + axis_z,
        ],
        texture_atlas.tile(&block.textures.east),
        [0.72, 0.72, 0.72],
    );
    push_world_textured_quad(
        vertices,
        indices,
        [
            center - axis_x - axis_y - axis_z,
            center - axis_x - axis_y + axis_z,
            center - axis_x + axis_y + axis_z,
            center - axis_x + axis_y - axis_z,
        ],
        texture_atlas.tile(&block.textures.west),
        [0.78, 0.78, 0.78],
    );
    push_world_textured_quad(
        vertices,
        indices,
        [
            center - axis_x + axis_y + axis_z,
            center + axis_x + axis_y + axis_z,
            center + axis_x + axis_y - axis_z,
            center - axis_x + axis_y - axis_z,
        ],
        texture_atlas.tile(&block.textures.top),
        [1.0, 1.0, 1.0],
    );
    push_world_textured_quad(
        vertices,
        indices,
        [
            center - axis_x - axis_y - axis_z,
            center + axis_x - axis_y - axis_z,
            center + axis_x - axis_y + axis_z,
            center - axis_x - axis_y + axis_z,
        ],
        texture_atlas.tile(&block.textures.bottom),
        [0.55, 0.55, 0.55],
    );
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
    let pixel = slot_height(rect) / GUI_SLOT_SIZE;
    if selected {
        ui.rect(rect, [0.96, 0.96, 0.64]);
    }
    let outer = if selected {
        inset_rect(rect, pixel)
    } else {
        rect
    };
    ui.rect(outer, [0.23, 0.23, 0.23]);
    let frame = inset_rect(outer, pixel);
    ui.rect(frame, [0.62, 0.62, 0.62]);
    ui.rect(
        UiRect::new(frame.left, frame.bottom, frame.right, frame.bottom + pixel),
        [0.36, 0.36, 0.36],
    );
    ui.rect(
        UiRect::new(
            frame.right - pixel / aspect_for_rect(rect),
            frame.bottom,
            frame.right,
            frame.top,
        ),
        [0.36, 0.36, 0.36],
    );
    ui.rect(
        UiRect::new(frame.left, frame.top - pixel, frame.right, frame.top),
        [0.90, 0.90, 0.90],
    );
    ui.rect(
        UiRect::new(
            frame.left,
            frame.bottom,
            frame.left + pixel / aspect_for_rect(rect),
            frame.top,
        ),
        [0.90, 0.90, 0.90],
    );
    ui.rect(inset_rect(frame, pixel * 2.0), [0.58, 0.58, 0.58]);
}

fn aspect_for_rect(rect: UiRect) -> f32 {
    (slot_height(rect) / slot_width(rect)).max(0.1)
}

fn draw_inventory_panel(ui: &mut UiMeshBuilder, kind: CraftingUiKind, aspect: f32) {
    let panel = inventory_panel_rect(aspect);
    ui.rect(panel, [0.03, 0.03, 0.03]);
    ui.rect(
        inset_rect(panel, panel_pixel(1.0, aspect)),
        [0.78, 0.78, 0.78],
    );
    ui.rect(
        inset_rect(panel, panel_pixel(3.0, aspect)),
        [0.48, 0.48, 0.48],
    );
    ui.rect(
        inset_rect(panel, panel_pixel(4.0, aspect)),
        [0.76, 0.76, 0.76],
    );

    if kind == CraftingUiKind::Inventory {
        ui.rect(gui_rect(27.0, 7.0, 51.0, 70.0, aspect), [0.01, 0.01, 0.01]);
        for row in 0..4 {
            draw_inventory_slot(
                ui,
                gui_rect(
                    7.0,
                    7.0 + row as f32 * GUI_SLOT_SIZE,
                    GUI_SLOT_SIZE,
                    GUI_SLOT_SIZE,
                    aspect,
                ),
                false,
            );
        }
    }
}

fn draw_crafting_arrow(ui: &mut UiMeshBuilder, kind: CraftingUiKind, aspect: f32) {
    let (x, y) = match kind {
        CraftingUiKind::Inventory => (126.0, 40.0),
        CraftingUiKind::Table => (90.0, 35.0),
    };
    let body = gui_rect(x, y, 14.0, 4.0, aspect);
    let top = gui_rect(x + 10.0, y - 4.0, 4.0, 12.0, aspect);
    let tip = [
        [top.right, top.top, 0.0],
        [
            gui_rect(x + 18.0, y + 2.0, 1.0, 1.0, aspect).left,
            body.bottom + slot_height(body) * 0.5,
            0.0,
        ],
        [top.right, top.bottom, 0.0],
        [top.right, top.bottom, 0.0],
    ];
    ui.rect(body, [0.55, 0.55, 0.55]);
    ui.rect(top, [0.55, 0.55, 0.55]);
    ui.quad(tip, [0.55, 0.55, 0.55]);
}

fn draw_stack_count(ui: &mut UiMeshBuilder, rect: UiRect, count: u16) {
    let text = count.to_string();
    let x = rect.right - slot_width(rect) * 0.38;
    let y = rect.bottom + slot_height(rect) * 0.30;
    ui.text(
        x + slot_width(rect) * 0.04,
        y - slot_height(rect) * 0.05,
        0.0038,
        [0.08, 0.08, 0.08],
        &text,
    );
    ui.text(x, y, 0.0038, [1.0, 1.0, 1.0], &text);
}

pub(super) fn inventory_slot_rect(index: usize, inventory_open: bool, aspect: f32) -> UiRect {
    let slot_height = if inventory_open {
        gui_slot_height()
    } else {
        CLOSED_HOTBAR_SLOT_HEIGHT
    };
    let slot_width = slot_height / aspect.max(0.1);
    let gap_y = INVENTORY_GAP_Y;
    let gap_x = gap_y / aspect.max(0.1);
    if inventory_open {
        let (source_x, source_y) = if index < INVENTORY_HOTBAR_SLOTS {
            (8.0 + index as f32 * GUI_SLOT_SIZE, 142.0)
        } else {
            let inventory_index = index - INVENTORY_HOTBAR_SLOTS;
            let row = inventory_index / 9;
            let column = inventory_index % 9;
            (
                8.0 + column as f32 * GUI_SLOT_SIZE,
                84.0 + row as f32 * GUI_SLOT_SIZE,
            )
        };
        gui_rect(source_x, source_y, GUI_SLOT_SIZE, GUI_SLOT_SIZE, aspect)
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

pub(super) fn crafting_input_slot_at_point(
    point: UiPoint,
    kind: CraftingUiKind,
    aspect: f32,
) -> Option<usize> {
    let slot_count = match kind {
        CraftingUiKind::Inventory => 4,
        CraftingUiKind::Table => 9,
    };
    for index in 0..slot_count {
        if crafting_input_slot_rect(index, kind, aspect).contains(point) {
            return Some(index);
        }
    }
    None
}

pub(super) fn crafting_result_slot_at_point(
    point: UiPoint,
    kind: CraftingUiKind,
    aspect: f32,
) -> bool {
    crafting_result_slot_rect(kind, aspect).contains(point)
}

pub(super) fn crafting_input_slot_rect(index: usize, kind: CraftingUiKind, aspect: f32) -> UiRect {
    let columns = match kind {
        CraftingUiKind::Inventory => 2,
        CraftingUiKind::Table => 3,
    };
    let row = index / columns;
    let column = index % columns;
    let (left, top) = match kind {
        CraftingUiKind::Inventory => (88.0, 26.0),
        CraftingUiKind::Table => (30.0, 17.0),
    };
    gui_rect(
        left + column as f32 * GUI_SLOT_SIZE,
        top + row as f32 * GUI_SLOT_SIZE,
        GUI_SLOT_SIZE,
        GUI_SLOT_SIZE,
        aspect,
    )
}

pub(super) fn crafting_result_slot_rect(kind: CraftingUiKind, aspect: f32) -> UiRect {
    let (left, top) = match kind {
        CraftingUiKind::Inventory => (144.0, 36.0),
        CraftingUiKind::Table => (124.0, 35.0),
    };
    gui_rect(left, top, GUI_SLOT_SIZE, GUI_SLOT_SIZE, aspect)
}

fn inventory_panel_rect(aspect: f32) -> UiRect {
    let height = GUI_PANEL_HEIGHT;
    let width = height * (GUI_SOURCE_WIDTH / GUI_SOURCE_HEIGHT) / aspect.max(0.1);
    UiRect::new(-width * 0.5, -height * 0.5, width * 0.5, height * 0.5)
}

fn gui_slot_height() -> f32 {
    GUI_PANEL_HEIGHT * GUI_SLOT_SIZE / GUI_SOURCE_HEIGHT
}

fn panel_pixel(pixels: f32, aspect: f32) -> f32 {
    slot_height(gui_rect(0.0, 0.0, pixels, pixels, aspect))
}

fn gui_rect(x: f32, y: f32, width: f32, height: f32, aspect: f32) -> UiRect {
    let panel = inventory_panel_rect(aspect);
    let panel_width = slot_width(panel);
    let panel_height = slot_height(panel);
    let left = panel.left + panel_width * (x / GUI_SOURCE_WIDTH);
    let right = panel.left + panel_width * ((x + width) / GUI_SOURCE_WIDTH);
    let top = panel.top - panel_height * (y / GUI_SOURCE_HEIGHT);
    let bottom = panel.top - panel_height * ((y + height) / GUI_SOURCE_HEIGHT);
    UiRect::new(left, bottom, right, top)
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

fn push_player_hand_mesh(
    texture_atlas: &TextureAtlas,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    aspect: f32,
) {
    let tile = texture_atlas.tile(&player_hand_texture_key());
    for face in player_arm_overlay_faces(aspect) {
        push_textured_ui_quad(vertices, indices, face.positions, tile, face.color);
    }
}

fn push_item_icon_mesh(
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    item: &ItemDefinition,
    rect: UiRect,
) {
    if let Some(block) = item
        .place_block
        .as_ref()
        .and_then(|key| world.blocks.get_by_key(key))
        .map(|(_, block)| block)
    {
        push_slot_block_icon_mesh(vertices, indices, texture_atlas, block, rect);
    } else {
        push_textured_ui_rect(vertices, indices, rect, texture_atlas.tile(&item.texture));
    }
}

fn push_slot_block_icon_mesh(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    texture_atlas: &TextureAtlas,
    block: &crate::engine::world::BlockDefinition,
    rect: UiRect,
) {
    let faces = slot_block_icon_faces(rect);
    push_textured_ui_quad(
        vertices,
        indices,
        faces.front,
        texture_atlas.tile(&block.textures.south),
        [0.90, 0.90, 0.90],
    );
    push_textured_ui_quad(
        vertices,
        indices,
        faces.right,
        texture_atlas.tile(&block.textures.east),
        [0.66, 0.66, 0.66],
    );
    push_textured_ui_quad(
        vertices,
        indices,
        faces.top,
        texture_atlas.tile(&block.textures.top),
        [1.0, 1.0, 1.0],
    );
}

pub(super) struct SlotBlockIconFaces {
    pub(super) front: [[f32; 3]; 4],
    pub(super) right: [[f32; 3]; 4],
    pub(super) top: [[f32; 3]; 4],
}

pub(super) fn slot_block_icon_faces(rect: UiRect) -> SlotBlockIconFaces {
    let x = |factor: f32| rect.left + slot_width(rect) * factor;
    let y = |factor: f32| rect.bottom + slot_height(rect) * factor;
    SlotBlockIconFaces {
        front: [
            [x(0.22), y(0.58), 0.0],
            [x(0.50), y(0.42), 0.0],
            [x(0.50), y(0.13), 0.0],
            [x(0.22), y(0.29), 0.0],
        ],
        right: [
            [x(0.50), y(0.42), 0.0],
            [x(0.78), y(0.58), 0.0],
            [x(0.78), y(0.29), 0.0],
            [x(0.50), y(0.13), 0.0],
        ],
        top: [
            [x(0.22), y(0.58), 0.0],
            [x(0.50), y(0.74), 0.0],
            [x(0.78), y(0.58), 0.0],
            [x(0.50), y(0.42), 0.0],
        ],
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
    let front_bottom_left = adjust(0.42, -1.08);
    let front_bottom_right = adjust(1.04, -1.00);
    let front_top_right = adjust(1.04, -0.58);
    let front_top_left = adjust(0.42, -0.68);
    let depth = |point: [f32; 3]| [point[0] + 0.30 * scale_x, point[1] + 0.14, 0.0];
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
    let adjust = |x: f32, y: f32| [0.80 + (x - 0.80) * scale_x, y, 0.0];
    let wrist_left = adjust(0.74, -1.04);
    let wrist_right = adjust(1.12, -1.02);
    let elbow_left = adjust(0.58, -0.60);
    let elbow_right = adjust(0.84, -0.47);
    let depth = |point: [f32; 3]| [point[0] + 0.24 * scale_x, point[1] + 0.02, 0.0];
    let wrist_right_back = depth(wrist_right);
    let elbow_right_back = depth(elbow_right);
    let elbow_left_back = depth(elbow_left);

    [
        UiFace {
            positions: [wrist_left, wrist_right, elbow_right, elbow_left],
            color: [0.98, 0.92, 0.86],
        },
        UiFace {
            positions: [wrist_right, wrist_right_back, elbow_right_back, elbow_right],
            color: [0.72, 0.58, 0.48],
        },
        UiFace {
            positions: [elbow_left, elbow_right, elbow_right_back, elbow_left_back],
            color: [1.0, 0.96, 0.90],
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

fn push_world_textured_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    positions: [Vec3; 4],
    tile: AtlasTile,
    color: [f32; 3],
) {
    let tex_coords = tile.uv_quad();
    let base = vertices.len() as u32;
    for index in 0..4 {
        vertices.push(Vertex {
            position: positions[index].to_array(),
            color,
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
