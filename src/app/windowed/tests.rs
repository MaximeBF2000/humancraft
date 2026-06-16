use super::spatial::{split_world_block_position, world_block_from_render};
use super::ui::UiRect;
use super::*;
use crate::content::{GameContent, bootstrap_content, default_generation_pipeline};
use crate::engine::mesh::chunk_mesher::{FaceDirection, MeshQuad};
use crate::engine::world::{BlockPosition, CHUNK_SIZE, Chunk, ChunkPosition, ItemId, LootEntity};

fn test_client_world(content: &GameContent) -> ClientWorld {
    ClientWorld::new(
        content.blocks.clone(),
        content.items.clone(),
        content.recipes.clone(),
        content.block_ids,
        default_generation_pipeline(content.block_ids),
        GenerationContext {
            seed: 1,
            air: content.block_ids.air,
        },
        CLIENT_RENDER_DISTANCE_CHUNKS,
        "test-world".to_string(),
    )
}

#[test]
fn logical_zqsd_controls_drive_movement() {
    let mut input = InputState::default();

    input.handle_logical_key(Key::Character("z"), true);
    input.handle_logical_key(Key::Character("q"), true);

    assert!(input.forward);
    assert!(input.left);
    assert!(!input.backward);
    assert!(!input.right);

    input.handle_logical_key(Key::Character("z"), false);

    assert!(!input.forward);
    assert!(input.left);
}

#[test]
fn wasd_keys_are_not_treated_as_forward_left_on_azerty_bindings() {
    let mut input = InputState::default();

    input.handle_logical_key(Key::Character("w"), true);
    input.handle_logical_key(Key::Character("a"), true);

    assert!(!input.forward);
    assert!(!input.left);
}

#[test]
fn double_tapping_forward_enables_sprint() {
    let mut input = InputState::default();
    let start = Instant::now();

    input.handle_logical_key_at(Key::Character("z"), true, start);
    input.handle_logical_key_at(
        Key::Character("z"),
        false,
        start + std::time::Duration::from_millis(40),
    );
    input.handle_logical_key_at(
        Key::Character("z"),
        true,
        start + std::time::Duration::from_millis(160),
    );

    assert!(input.forward);
    assert!(input.sprint);
}

#[test]
fn slow_forward_tap_does_not_enable_sprint() {
    let mut input = InputState::default();
    let start = Instant::now();

    input.handle_logical_key_at(Key::Character("z"), true, start);
    input.handle_logical_key_at(
        Key::Character("z"),
        false,
        start + std::time::Duration::from_millis(40),
    );
    input.handle_logical_key_at(
        Key::Character("z"),
        true,
        start + std::time::Duration::from_millis(600),
    );

    assert!(input.forward);
    assert!(!input.sprint);
}

#[test]
fn preview_filter_keeps_real_underground_faces_visible() {
    let content = bootstrap_content().unwrap();
    let wall = MeshQuad {
        block: content.block_ids.stone,
        direction: FaceDirection::North,
        vertices: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 40.0, 0.0],
            [0.0, 40.0, 0.0],
        ],
    };
    let surface_side = MeshQuad {
        vertices: [
            [0.0, 60.0, 0.0],
            [1.0, 60.0, 0.0],
            [1.0, 70.0, 0.0],
            [0.0, 70.0, 0.0],
        ],
        ..wall.clone()
    };
    let top = MeshQuad {
        direction: FaceDirection::Up,
        ..wall.clone()
    };
    let bottom = MeshQuad {
        direction: FaceDirection::Down,
        ..wall.clone()
    };
    let world_bottom = MeshQuad {
        vertices: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        ..bottom.clone()
    };

    assert!(should_render_preview_quad(
        &wall,
        ChunkPosition { x: 0, z: 0 },
        None
    ));
    assert!(should_render_preview_quad(
        &surface_side,
        ChunkPosition { x: 0, z: 0 },
        None
    ));
    assert!(should_render_preview_quad(
        &top,
        ChunkPosition { x: 0, z: 0 },
        None
    ));
    assert!(should_render_preview_quad(
        &bottom,
        ChunkPosition { x: 0, z: 0 },
        None
    ));
    assert!(should_render_preview_quad(
        &world_bottom,
        ChunkPosition { x: 0, z: 0 },
        None
    ));
}

#[test]
fn preview_filter_hides_outer_render_boundary_faces() {
    let content = bootstrap_content().unwrap();
    let outer_west_wall = MeshQuad {
        block: content.block_ids.stone,
        direction: FaceDirection::West,
        vertices: [
            [0.0, 60.0, 0.0],
            [0.0, 60.0, 1.0],
            [0.0, 70.0, 1.0],
            [0.0, 70.0, 0.0],
        ],
    };
    let inner_west_wall = MeshQuad {
        vertices: [
            [1.0, 60.0, 0.0],
            [1.0, 60.0, 1.0],
            [1.0, 70.0, 1.0],
            [1.0, 70.0, 0.0],
        ],
        ..outer_west_wall.clone()
    };
    let bounds = Some(RenderChunkBounds {
        min_x: -4,
        max_x: 0,
        min_z: -2,
        max_z: 2,
    });

    assert!(!should_render_preview_quad(
        &outer_west_wall,
        ChunkPosition { x: -4, z: 0 },
        bounds
    ));
    assert!(should_render_preview_quad(
        &inner_west_wall,
        ChunkPosition { x: -4, z: 0 },
        bounds
    ));
}

#[test]
fn preview_filter_hides_loaded_world_bottom_boundary_faces() {
    let content = bootstrap_content().unwrap();
    let bottom = MeshQuad {
        block: content.block_ids.bedrock,
        direction: FaceDirection::Down,
        vertices: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
    };
    let bounds = Some(RenderChunkBounds {
        min_x: 0,
        max_x: 0,
        min_z: 0,
        max_z: 0,
    });

    assert!(!should_render_preview_quad(
        &bottom,
        ChunkPosition { x: 0, z: 0 },
        bounds
    ));
}

#[test]
fn splits_negative_world_positions_into_chunk_and_local_positions() {
    let (chunk, block) = split_world_block_position(WorldBlockPosition {
        x: -1,
        y: 64,
        z: -17,
    })
    .unwrap();

    assert_eq!(chunk, ChunkPosition { x: -1, z: -2 });
    assert_eq!(
        block,
        BlockPosition {
            x: 15,
            y: 64,
            z: 15
        }
    );
}

#[test]
fn render_positions_map_to_chunk_positions() {
    assert_eq!(
        chunk_position_for_render_position(Vec3::new(0.0, 0.0, 0.0)),
        ChunkPosition { x: 0, z: 0 }
    );
    assert_eq!(
        chunk_position_for_render_position(Vec3::new(-9.0, 0.0, -9.0)),
        ChunkPosition { x: -1, z: -1 }
    );
}

#[test]
fn client_world_generates_chunks_around_player_position() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);

    let initial_dirty =
        world.ensure_chunks_around_render_position(Vec3::new(0.0, 0.0, 0.0), usize::MAX);
    assert_eq!(world.chunks.len(), 25);
    assert_eq!(initial_dirty.len(), 25);
    assert!(initial_dirty.contains(&ChunkPosition { x: 0, z: 0 }));
    assert!(
        world
            .ensure_chunks_around_render_position(Vec3::new(0.0, 0.0, 0.0), usize::MAX)
            .is_empty()
    );
    assert_eq!(world.chunks.len(), 25);

    let distant_dirty =
        world.ensure_chunks_around_render_position(Vec3::new(80.0, 0.0, -72.0), usize::MAX);
    assert!(!distant_dirty.is_empty());
    assert!(world.chunks.contains_key(&ChunkPosition { x: 5, z: -4 }));
    assert!(world.chunks.len() > 25);
}

#[test]
fn client_world_respects_chunk_load_budget() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);

    let dirty = world.ensure_chunks_around_render_position(Vec3::new(0.0, 0.0, 0.0), 3);

    assert_eq!(world.chunks.len(), 3);
    assert!(!dirty.is_empty());
    assert!(world.chunks.contains_key(&ChunkPosition { x: 0, z: 0 }));
    assert!(!world.chunks.contains_key(&ChunkPosition { x: -2, z: -2 }));
}

#[test]
fn generated_distant_chunks_are_deterministic() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let position = Vec3::new(112.0, 0.0, -96.0);

    world.ensure_chunks_around_render_position(position, usize::MAX);
    let first_block = world.block(WorldBlockPosition {
        x: 120,
        y: 64,
        z: -88,
    });

    let mut other_world = test_client_world(&content);
    other_world.ensure_chunks_around_render_position(position, usize::MAX);
    let second_block = other_world.block(WorldBlockPosition {
        x: 120,
        y: 64,
        z: -88,
    });

    assert_eq!(first_block, second_block);
}

#[test]
fn saved_chunks_override_generation_when_streaming() {
    let content = bootstrap_content().unwrap();
    let root = std::env::temp_dir().join(format!(
        "humancraft-window-save-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let store = WorldSaveStore::new(&root);
    let metadata = store
        .create_world("Saved Chunk", 1, PlayerSave::new(0.0, 0.0, 0.0, 0.0, 0.0))
        .unwrap();
    let chunk_position = ChunkPosition { x: 0, z: 0 };
    let mut saved_chunk = Chunk::filled(chunk_position, content.block_ids.air);
    saved_chunk
        .set_block(
            BlockPosition { x: 8, y: 64, z: 8 },
            content.block_ids.diamond_ore,
        )
        .unwrap();
    store.save_chunk(&metadata.id, &saved_chunk).unwrap();

    let mut world = ClientWorld::new(
        content.blocks.clone(),
        content.items.clone(),
        content.recipes.clone(),
        content.block_ids,
        default_generation_pipeline(content.block_ids),
        GenerationContext {
            seed: metadata.seed,
            air: content.block_ids.air,
        },
        CLIENT_RENDER_DISTANCE_CHUNKS,
        metadata.id.clone(),
    );
    world.ensure_chunks_around_render_position_with_store(Vec3::ZERO, usize::MAX, &store);

    assert_eq!(
        world.block(WorldBlockPosition { x: 8, y: 64, z: 8 }),
        Some(content.block_ids.diamond_ore)
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn client_world_can_break_and_place_blocks() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(chunk);
    let position = WorldBlockPosition { x: 1, y: 1, z: 1 };

    assert!(world.is_solid(position));
    assert_eq!(
        world.break_block(position),
        vec![ChunkPosition { x: 0, z: 0 }]
    );
    assert!(!world.is_solid(position));
    assert_eq!(
        world.place_block(position, content.block_ids.dirt),
        vec![ChunkPosition { x: 0, z: 0 }]
    );
    assert!(world.is_solid(position));
}

#[test]
fn breaking_block_spawns_configured_loot() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(chunk);

    let dirty = world.break_block(WorldBlockPosition { x: 1, y: 1, z: 1 });
    let cobblestone = world.items.id_for_key("humancraft:cobblestone").unwrap();

    assert_eq!(dirty, vec![ChunkPosition { x: 0, z: 0 }]);
    assert_eq!(world.loot_entities.len(), 1);
    assert_eq!(world.loot_entities[0].stack, ItemStack::new(cobblestone, 1));
}

#[test]
fn block_breaking_uses_hardness_progress_before_editing_world() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.dirt)
        .unwrap();
    chunk
        .set_block(BlockPosition { x: 2, y: 1, z: 1 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(chunk);
    let dirt = WorldBlockPosition { x: 1, y: 1, z: 1 };
    let stone = WorldBlockPosition { x: 2, y: 1, z: 1 };

    assert!(world.continue_breaking_block(dirt, 0.74).is_empty());
    assert_eq!(world.block(dirt), Some(content.block_ids.dirt));
    assert!(!world.continue_breaking_block(dirt, 0.01).is_empty());
    assert_eq!(world.block(dirt), Some(content.block_ids.air));

    assert!(world.continue_breaking_block(stone, 2.24).is_empty());
    assert_eq!(world.block(stone), Some(content.block_ids.stone));
    assert!(!world.continue_breaking_block(stone, 0.01).is_empty());
    assert_eq!(world.block(stone), Some(content.block_ids.air));
}

#[test]
fn block_break_progress_resets_when_target_changes() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.dirt)
        .unwrap();
    chunk
        .set_block(BlockPosition { x: 2, y: 1, z: 1 }, content.block_ids.dirt)
        .unwrap();
    world.insert_chunk(chunk);
    let first = WorldBlockPosition { x: 1, y: 1, z: 1 };
    let second = WorldBlockPosition { x: 2, y: 1, z: 1 };

    world.continue_breaking_block(first, 0.5);
    world.continue_breaking_block(second, 0.3);

    let progress = world.block_break_progress().unwrap();
    assert_eq!(progress.target, second);
    assert!((progress.ratio - 0.4).abs() < 0.001);
    assert_eq!(world.block(first), Some(content.block_ids.dirt));
    assert_eq!(world.block(second), Some(content.block_ids.dirt));
}

#[test]
fn player_pickup_adds_loot_to_inventory() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    world.insert_chunk(Chunk::filled(
        ChunkPosition { x: 0, z: 0 },
        content.block_ids.air,
    ));
    let dirt = world.items.id_for_key("humancraft:dirt").unwrap();
    world.loot_entities.push(LootEntity::new(
        ItemStack::new(dirt, 3),
        Vec3::new(0.0, 0.05, 0.0),
    ));

    world.update_loot(Vec3::new(0.0, PLAYER_STANDING_EYE_HEIGHT + 0.05, 0.0), 0.05);

    assert!(world.loot_entities.is_empty());
    assert_eq!(
        world.player_inventory.hotbar_slots()[0],
        Some(ItemStack::new(dirt, 3))
    );
}

#[test]
fn inventory_save_round_trips_registered_item_stacks() {
    let content = bootstrap_content().unwrap();
    let dirt = content.items.id_for_key("humancraft:dirt").unwrap();
    let diamond = content.items.id_for_key("humancraft:diamond").unwrap();
    let mut inventory = Inventory::player();
    inventory.add_stack(ItemStack::new(dirt, 64), &content.items);
    inventory.add_stack(ItemStack::new(diamond, 3), &content.items);

    let saved = inventory_to_save(&inventory, &content.items);
    let restored = inventory_from_save(&saved, &content.items);

    assert_eq!(restored.slots()[0], Some(ItemStack::new(dirt, 64)));
    assert_eq!(restored.slots()[1], Some(ItemStack::new(diamond, 3)));
}

#[test]
fn inventory_left_click_picks_up_merges_and_swaps_stacks() {
    let content = bootstrap_content().unwrap();
    let dirt = content.items.id_for_key("humancraft:dirt").unwrap();
    let stone = content.items.id_for_key("humancraft:stone").unwrap();
    let mut inventory = Inventory::new(3, 1);
    let mut cursor = None;
    inventory.set_slot(0, Some(ItemStack::new(dirt, 10)));
    inventory.set_slot(1, Some(ItemStack::new(dirt, 60)));
    inventory.set_slot(2, Some(ItemStack::new(stone, 4)));

    left_click_inventory_slot(&mut inventory, &mut cursor, 0, &content.items);
    assert_eq!(cursor, Some(ItemStack::new(dirt, 10)));
    assert_eq!(inventory.slot(0), None);

    left_click_inventory_slot(&mut inventory, &mut cursor, 1, &content.items);
    assert_eq!(inventory.slot(1), Some(ItemStack::new(dirt, 64)));
    assert_eq!(cursor, Some(ItemStack::new(dirt, 6)));

    left_click_inventory_slot(&mut inventory, &mut cursor, 2, &content.items);
    assert_eq!(inventory.slot(2), Some(ItemStack::new(dirt, 6)));
    assert_eq!(cursor, Some(ItemStack::new(stone, 4)));
}

#[test]
fn inventory_right_click_splits_and_places_one_item() {
    let content = bootstrap_content().unwrap();
    let dirt = content.items.id_for_key("humancraft:dirt").unwrap();
    let mut inventory = Inventory::new(2, 1);
    let mut cursor = None;
    inventory.set_slot(0, Some(ItemStack::new(dirt, 7)));

    right_click_inventory_slot(&mut inventory, &mut cursor, 0, &content.items);
    assert_eq!(inventory.slot(0), Some(ItemStack::new(dirt, 3)));
    assert_eq!(cursor, Some(ItemStack::new(dirt, 4)));

    right_click_inventory_slot(&mut inventory, &mut cursor, 1, &content.items);
    assert_eq!(inventory.slot(1), Some(ItemStack::new(dirt, 1)));
    assert_eq!(cursor, Some(ItemStack::new(dirt, 3)));
}

#[test]
fn inventory_drag_distributes_and_right_drag_places_one_per_slot() {
    let content = bootstrap_content().unwrap();
    let dirt = content.items.id_for_key("humancraft:dirt").unwrap();
    let mut inventory = Inventory::new(4, 1);
    let mut cursor = Some(ItemStack::new(dirt, 8));

    distribute_carried_stack_evenly(&mut inventory, &mut cursor, &[0, 1, 2], &content.items);
    assert_eq!(cursor, None);
    assert_eq!(inventory.slot(0), Some(ItemStack::new(dirt, 3)));
    assert_eq!(inventory.slot(1), Some(ItemStack::new(dirt, 3)));
    assert_eq!(inventory.slot(2), Some(ItemStack::new(dirt, 2)));

    cursor = Some(ItemStack::new(dirt, 3));
    assert!(place_one_carried_item(
        &mut inventory,
        &mut cursor,
        0,
        &content.items
    ));
    assert!(place_one_carried_item(
        &mut inventory,
        &mut cursor,
        3,
        &content.items
    ));
    assert_eq!(inventory.slot(0), Some(ItemStack::new(dirt, 4)));
    assert_eq!(inventory.slot(3), Some(ItemStack::new(dirt, 1)));
    assert_eq!(cursor, Some(ItemStack::new(dirt, 1)));
}

#[test]
fn selected_hotbar_item_controls_block_placement() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    world.insert_chunk(Chunk::filled(
        ChunkPosition { x: 0, z: 0 },
        content.block_ids.air,
    ));
    let dirt = world.items.id_for_key("humancraft:dirt").unwrap();
    let coal = world.items.id_for_key("humancraft:coal").unwrap();
    let player_eye = Vec3::new(0.0, 2.0, 0.0);
    let position = WorldBlockPosition { x: 1, y: 1, z: 1 };

    world
        .player_inventory
        .set_slot(0, Some(ItemStack::new(coal, 1)));
    assert!(
        world
            .place_selected_hotbar_block_for_player(position, 0, player_eye)
            .is_empty()
    );
    assert_eq!(world.block(position), Some(content.block_ids.air));

    world
        .player_inventory
        .set_slot(0, Some(ItemStack::new(dirt, 2)));
    assert_eq!(
        world.place_selected_hotbar_block_for_player(position, 0, player_eye),
        vec![ChunkPosition { x: 0, z: 0 }]
    );
    assert_eq!(world.block(position), Some(content.block_ids.dirt));
    assert_eq!(
        world.player_inventory.slot(0),
        Some(ItemStack::new(dirt, 1))
    );

    let cobblestone = world.items.id_for_key("humancraft:cobblestone").unwrap();
    let cobblestone_position = WorldBlockPosition { x: 2, y: 1, z: 1 };
    world
        .player_inventory
        .set_slot(0, Some(ItemStack::new(cobblestone, 1)));
    assert_eq!(
        world.place_selected_hotbar_block_for_player(cobblestone_position, 0, player_eye),
        vec![ChunkPosition { x: 0, z: 0 }]
    );
    assert_eq!(
        world.block(cobblestone_position),
        Some(content.block_ids.cobblestone)
    );
    assert_eq!(world.player_inventory.slot(0), None);
}

#[test]
fn inventory_slots_are_square_in_screen_pixels() {
    let aspect = 16.0 / 9.0;
    let hotbar = inventory_slot_rect(0, false, aspect);
    let inventory = inventory_slot_rect(0, true, aspect);

    assert!((slot_width(hotbar) * aspect - slot_height(hotbar)).abs() < 0.0001);
    assert!((slot_width(inventory) * aspect - slot_height(inventory)).abs() < 0.0001);
    assert!(slot_height(hotbar) > 0.10);
}

#[test]
fn full_inventory_layout_keeps_crafting_and_player_slots_separate() {
    for aspect in [4.0 / 3.0, 16.0 / 9.0] {
        let player_slots: Vec<_> = (0..Inventory::player().slot_count())
            .map(|index| inventory_slot_rect(index, true, aspect))
            .collect();
        let inventory_crafting_slots: Vec<_> = (0..4)
            .map(|index| crafting_input_slot_rect(index, CraftingUiKind::Inventory, aspect))
            .collect();
        let table_crafting_slots: Vec<_> = (0..9)
            .map(|index| crafting_input_slot_rect(index, CraftingUiKind::Table, aspect))
            .collect();
        let inventory_result = crafting_result_slot_rect(CraftingUiKind::Inventory, aspect);
        let table_result = crafting_result_slot_rect(CraftingUiKind::Table, aspect);

        assert_rects_do_not_overlap(&player_slots, &inventory_crafting_slots);
        assert_rects_do_not_overlap(&player_slots, &table_crafting_slots);
        assert!(
            player_slots
                .iter()
                .all(|slot| !rects_overlap(*slot, inventory_result))
        );
        assert!(
            player_slots
                .iter()
                .all(|slot| !rects_overlap(*slot, table_result))
        );
        assert!(
            inventory_crafting_slots
                .iter()
                .all(|slot| !rects_overlap(*slot, inventory_result))
        );
        assert!(
            table_crafting_slots
                .iter()
                .all(|slot| !rects_overlap(*slot, table_result))
        );
    }
}

fn assert_rects_do_not_overlap(left: &[UiRect], right: &[UiRect]) {
    for left_rect in left {
        for right_rect in right {
            assert!(
                !rects_overlap(*left_rect, *right_rect),
                "rectangles should not overlap: {left_rect:?} and {right_rect:?}"
            );
        }
    }
}

fn rects_overlap(left: UiRect, right: UiRect) -> bool {
    left.left < right.right
        && left.right > right.left
        && left.bottom < right.top
        && left.top > right.bottom
}

#[test]
fn held_block_overlay_uses_three_visible_faces() {
    let faces = held_block_overlay_faces(16.0 / 9.0);
    let all_faces = [faces.front, faces.right, faces.top];

    for face in all_faces {
        assert!(quad_area(face) > 0.002);
        for [x, y, _] in face {
            assert!((-1.0..=1.0).contains(&x));
            assert!((-1.0..=1.0).contains(&y));
        }
    }
    assert_eq!(faces.front[1], faces.right[0]);
    assert_eq!(faces.front[2], faces.right[3]);
    assert_eq!(faces.front[2], faces.top[1]);
    assert_eq!(faces.front[3], faces.top[0]);
}

#[test]
fn held_block_overlay_is_framed_like_a_first_person_held_block() {
    let faces = held_block_overlay_faces(16.0 / 9.0);
    let all_points = faces
        .front
        .into_iter()
        .chain(faces.right)
        .chain(faces.top)
        .collect::<Vec<_>>();
    let min_x = all_points
        .iter()
        .map(|point| point[0])
        .fold(f32::MAX, f32::min);
    let max_x = all_points
        .iter()
        .map(|point| point[0])
        .fold(f32::MIN, f32::max);
    let min_y = all_points
        .iter()
        .map(|point| point[1])
        .fold(f32::MAX, f32::min);
    let max_y = all_points
        .iter()
        .map(|point| point[1])
        .fold(f32::MIN, f32::max);

    assert!(min_x > 0.55);
    assert!(max_x < 0.90);
    assert!(min_y < -0.90);
    assert!(max_y > -0.45);
}

#[test]
fn player_arm_overlay_uses_three_visible_faces() {
    let faces = player_arm_overlay_faces(16.0 / 9.0);

    for face in faces {
        assert!(quad_area(face.positions) > 0.002);
        for [x, y, _] in face.positions {
            assert!((-1.0..=1.0).contains(&x));
            assert!((-1.0..=1.0).contains(&y));
        }
    }
}

#[test]
fn loot_mesh_stays_above_entity_contact_point_and_rotates_around_y() {
    let loot = LootEntity {
        stack: ItemStack::new(ItemId::from(1), 1),
        position: Vec3::new(0.0, 2.0, 0.0),
        velocity: Vec3::ZERO,
        rotation_radians: std::f32::consts::FRAC_PI_2,
    };
    let corners = loot_billboard_corners(&loot);

    assert!(corners.iter().all(|corner| corner.y >= 2.0));
    assert!(corners.iter().any(|corner| corner.z.abs() > 0.20));
    assert!(corners.iter().all(|corner| corner.x.abs() < 0.001));
}

#[test]
fn held_block_interaction_repeats_after_cadence_until_released() {
    let mut interaction = HeldBlockInteraction::default();

    interaction.press(MouseButton::Right);
    assert_eq!(interaction.repeat_button(0.05), None);
    assert_eq!(interaction.repeat_button(0.09), None);
    assert_eq!(interaction.repeat_button(0.01), Some(MouseButton::Right));
    assert_eq!(interaction.repeat_button(0.01), None);
    interaction.release(MouseButton::Right);
    assert_eq!(
        interaction.repeat_button(BLOCK_INTERACTION_REPEAT_SECONDS),
        None
    );

    interaction.press(MouseButton::Left);
    assert_eq!(
        interaction.repeat_button(BLOCK_INTERACTION_REPEAT_SECONDS * 2.0),
        None
    );
    assert!(interaction.is_held(MouseButton::Left));
}

#[test]
fn loot_from_block_below_another_block_spawns_in_open_space_and_falls() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
        .unwrap();
    chunk
        .set_block(BlockPosition { x: 1, y: 2, z: 1 }, content.block_ids.dirt)
        .unwrap();
    world.insert_chunk(chunk);

    world.break_block(WorldBlockPosition { x: 1, y: 1, z: 1 });
    assert_eq!(world.loot_entities.len(), 1);
    assert!(world.loot_entities[0].position.y + LOOT_RENDER_HALF_SIZE * 2.0 < 2.0);

    let player_far_away = Vec3::new(8.0, PLAYER_STANDING_EYE_HEIGHT + 1.0, 8.0);
    for _ in 0..20 {
        world.update_loot(player_far_away, PHYSICS_TICK_SECONDS);
    }

    assert!(world.loot_entities[0].position.y < 1.5);
}

fn quad_area(points: [[f32; 3]; 4]) -> f32 {
    let triangles = [[0, 1, 2], [0, 2, 3]];
    triangles
        .iter()
        .map(|[a, b, c]| {
            let a = glam::Vec2::new(points[*a][0], points[*a][1]);
            let b = glam::Vec2::new(points[*b][0], points[*b][1]);
            let c = glam::Vec2::new(points[*c][0], points[*c][1]);
            ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() * 0.5
        })
        .sum()
}

#[test]
fn client_world_does_not_break_unbreakable_blocks() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(
            BlockPosition { x: 1, y: 0, z: 1 },
            content.block_ids.bedrock,
        )
        .unwrap();
    world.insert_chunk(chunk);
    let position = WorldBlockPosition { x: 1, y: 0, z: 1 };

    assert!(world.is_solid(position));
    assert!(world.continue_breaking_block(position, 60.0).is_empty());
    assert_eq!(world.block(position), Some(content.block_ids.bedrock));
}

#[test]
fn player_facing_placement_rejects_player_occupied_blocks() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    world.insert_chunk(Chunk::filled(
        ChunkPosition { x: 0, z: 0 },
        content.block_ids.air,
    ));
    let player_eye = Vec3::new(0.0, 1.62, 0.0);
    let feet_block = WorldBlockPosition { x: 8, y: 64, z: 8 };
    let head_block = WorldBlockPosition { x: 8, y: 65, z: 8 };
    let nearby_block = WorldBlockPosition { x: 9, y: 64, z: 8 };

    assert!(
        world
            .place_block_for_player(feet_block, content.block_ids.dirt, player_eye)
            .is_empty()
    );
    assert!(
        world
            .place_block_for_player(head_block, content.block_ids.dirt, player_eye)
            .is_empty()
    );
    assert_eq!(world.block(feet_block), Some(content.block_ids.air));
    assert_eq!(world.block(head_block), Some(content.block_ids.air));

    assert_eq!(
        world.place_block_for_player(nearby_block, content.block_ids.dirt, player_eye),
        vec![ChunkPosition { x: 0, z: 0 }]
    );
    assert_eq!(world.block(nearby_block), Some(content.block_ids.dirt));
}

#[test]
fn block_edits_mark_only_affected_chunks_dirty() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut center_chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    center_chunk
        .set_block(BlockPosition { x: 8, y: 1, z: 8 }, content.block_ids.stone)
        .unwrap();
    center_chunk
        .set_block(
            BlockPosition {
                x: CHUNK_SIZE - 1,
                y: 1,
                z: 1,
            },
            content.block_ids.stone,
        )
        .unwrap();
    world.insert_chunk(center_chunk);
    world.insert_chunk(Chunk::filled(
        ChunkPosition { x: 1, z: 0 },
        content.block_ids.air,
    ));

    assert_eq!(
        world.break_block(WorldBlockPosition { x: 8, y: 1, z: 8 }),
        vec![ChunkPosition { x: 0, z: 0 }]
    );

    let dirty = world.break_block(WorldBlockPosition { x: 15, y: 1, z: 1 });
    assert_eq!(dirty.len(), 2);
    assert!(dirty.contains(&ChunkPosition { x: 0, z: 0 }));
    assert!(dirty.contains(&ChunkPosition { x: 1, z: 0 }));
}

#[test]
fn client_world_mesh_culls_faces_across_chunk_boundaries() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut west_chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    let mut east_chunk = Chunk::filled(ChunkPosition { x: 1, z: 0 }, content.block_ids.air);
    west_chunk
        .set_block(
            BlockPosition {
                x: CHUNK_SIZE - 1,
                y: 1,
                z: 1,
            },
            content.block_ids.stone,
        )
        .unwrap();
    east_chunk
        .set_block(BlockPosition { x: 0, y: 1, z: 1 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(west_chunk);
    world.insert_chunk(east_chunk);

    let west_mesh = world.mesh_chunk_for_render(
        ChunkPosition { x: 0, z: 0 },
        world.chunks.get(&ChunkPosition { x: 0, z: 0 }).unwrap(),
    );
    let east_mesh = world.mesh_chunk_for_render(
        ChunkPosition { x: 1, z: 0 },
        world.chunks.get(&ChunkPosition { x: 1, z: 0 }).unwrap(),
    );

    assert_eq!(west_mesh.quads.len(), 5);
    assert_eq!(east_mesh.quads.len(), 5);
    assert!(
        !west_mesh
            .quads
            .iter()
            .any(|quad| quad.direction == FaceDirection::East)
    );
    assert!(
        !east_mesh
            .quads
            .iter()
            .any(|quad| quad.direction == FaceDirection::West)
    );
}

#[test]
fn client_world_mesh_exposes_border_face_after_neighbor_breaks() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut west_chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    let mut east_chunk = Chunk::filled(ChunkPosition { x: 1, z: 0 }, content.block_ids.air);
    west_chunk
        .set_block(
            BlockPosition {
                x: CHUNK_SIZE - 1,
                y: 1,
                z: 1,
            },
            content.block_ids.stone,
        )
        .unwrap();
    east_chunk
        .set_block(BlockPosition { x: 0, y: 1, z: 1 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(west_chunk);
    world.insert_chunk(east_chunk);

    assert!(
        !world
            .break_block(WorldBlockPosition { x: 16, y: 1, z: 1 })
            .is_empty()
    );
    let west_mesh = world.mesh_chunk_for_render(
        ChunkPosition { x: 0, z: 0 },
        world.chunks.get(&ChunkPosition { x: 0, z: 0 }).unwrap(),
    );

    assert_eq!(west_mesh.quads.len(), 6);
    assert!(
        west_mesh
            .quads
            .iter()
            .any(|quad| quad.direction == FaceDirection::East)
    );
}

#[test]
fn player_aabb_detects_wall_collision_without_surface_snap() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(BlockPosition { x: 8, y: 64, z: 8 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(chunk);

    let colliding_eye = Vec3::new(0.0, 2.0, 0.0);
    let nearby_eye = Vec3::new(1.4, 2.0, 0.0);

    assert!(world.collides_player_at(colliding_eye));
    assert!(!world.collides_player_at(nearby_eye));
}

#[test]
fn safe_spawn_position_does_not_collide_with_blocks() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    for y in 0..=64 {
        chunk
            .set_block(BlockPosition { x: 8, y, z: 8 }, content.block_ids.stone)
            .unwrap();
    }
    chunk
        .set_block(BlockPosition { x: 8, y: 65, z: 8 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(chunk);

    let spawn = world.safe_spawn_eye_position(Vec3::new(0.0, 0.0, 0.0));

    assert!(!world.collides_player_at(spawn));
    assert_ne!(
        world_block_from_render(spawn),
        WorldBlockPosition { x: 8, y: 65, z: 8 }
    );
}

#[test]
fn sneaking_prevents_player_from_walking_off_block_edge() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(BlockPosition { x: 8, y: 63, z: 8 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(chunk);

    let mut camera = Camera::new(Vec3::new(0.5, PLAYER_STANDING_EYE_HEIGHT + 0.05, 0.5));
    camera.grounded = true;
    let input = InputState {
        forward: true,
        sneak: true,
        ..InputState::default()
    };

    for _ in 0..80 {
        camera.update(&input, &world, PHYSICS_TICK_SECONDS);
    }

    assert!(world.has_player_ground_support(camera.position, camera.eye_height()));
}

#[test]
fn player_can_jump_onto_one_block_ledge() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    for z in 8..=12 {
        chunk
            .set_block(BlockPosition { x: 8, y: 63, z }, content.block_ids.stone)
            .unwrap();
    }
    chunk
        .set_block(BlockPosition { x: 8, y: 64, z: 8 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(chunk);

    let mut camera = Camera::new(Vec3::new(0.5, PLAYER_STANDING_EYE_HEIGHT + 0.05, 4.0));
    camera.grounded = true;
    let mut landed_on_ledge = false;
    for _ in 0..100 {
        let input = InputState {
            forward: true,
            jump: camera.position.z <= 2.2,
            ..InputState::default()
        };
        camera.update(&input, &world, PHYSICS_TICK_SECONDS);
        let feet_y = camera.position.y - camera.eye_height();
        if camera.grounded && feet_y >= 1.0 && camera.position.z < 1.0 {
            landed_on_ledge = true;
            break;
        }
    }

    assert!(
        landed_on_ledge,
        "expected jump and forward movement from a short distance to land on one-block ledge"
    );
}

#[test]
fn outline_has_twelve_edges() {
    let vertices = build_outline_vertices(WorldBlockPosition { x: 8, y: 64, z: 8 });

    assert_eq!(vertices.len(), 24);
}

#[test]
fn camera_can_look_nearly_straight_up_and_down() {
    let mut camera = Camera::new(Vec3::ZERO);

    camera.apply_mouse_delta(0.0, -10_000.0);
    assert!(camera.forward().y > 0.99);

    camera.apply_mouse_delta(0.0, 20_000.0);
    assert!(camera.forward().y < -0.99);
}

#[test]
fn crosshair_compensates_for_widescreen_aspect() {
    let (vertices, _) = build_crosshair_mesh(1280, 720);
    let horizontal_length = vertices[1].position[0] - vertices[0].position[0];
    let vertical_length = vertices[6].position[1] - vertices[5].position[1];
    let aspect = 1280.0 / 720.0;

    assert!((horizontal_length * aspect - vertical_length).abs() < 0.001);
}

#[test]
fn raycast_returns_hit_and_previous_empty_block() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
    chunk
        .set_block(BlockPosition { x: 4, y: 65, z: 4 }, content.block_ids.stone)
        .unwrap();
    world.insert_chunk(chunk);

    let hit = world
        .raycast(Vec3::new(-4.0, 1.5, -5.0), Vec3::new(0.0, 0.0, 1.0))
        .unwrap();

    assert_eq!(hit.block, WorldBlockPosition { x: 4, y: 65, z: 4 });
    assert_eq!(hit.previous, WorldBlockPosition { x: 4, y: 65, z: 3 });
}

#[test]
fn block_texture_keys_map_to_asset_paths() {
    let content = bootstrap_content().unwrap();
    let (_, grass) = content
        .blocks
        .get_by_key("humancraft:grass")
        .expect("grass should be registered");
    let (_, stone) = content
        .blocks
        .get_by_key("humancraft:stone")
        .expect("stone should be registered");
    let (_, sand) = content
        .blocks
        .get_by_key("humancraft:sand")
        .expect("sand should be registered");
    let (_, sandstone) = content
        .blocks
        .get_by_key("humancraft:sandstone")
        .expect("sandstone should be registered");
    let (_, bedrock) = content
        .blocks
        .get_by_key("humancraft:bedrock")
        .expect("bedrock should be registered");

    assert_eq!(
        texture_key_for_direction(grass, FaceDirection::Up),
        "humancraft:block/grass/top"
    );
    assert_eq!(
        texture_key_for_direction(grass, FaceDirection::Down),
        "humancraft:block/dirt/bottom"
    );
    assert_eq!(
        texture_path(texture_key_for_direction(stone, FaceDirection::North)).unwrap(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("textures")
            .join("blocks")
            .join("stone")
            .join("top.png")
    );
    assert!(load_texture_pixels(texture_key_for_direction(stone, FaceDirection::North)).is_some());
    assert!(load_texture_pixels(texture_key_for_direction(sand, FaceDirection::Up)).is_some());
    assert!(load_texture_pixels(texture_key_for_direction(sandstone, FaceDirection::Up)).is_some());
    assert!(load_texture_pixels(texture_key_for_direction(bedrock, FaceDirection::Up)).is_some());
}

#[test]
fn every_registered_non_air_block_uses_loadable_textures() {
    let content = bootstrap_content().unwrap();

    for (_, definition) in content.blocks.iter() {
        if definition.key == "humancraft:air" {
            continue;
        }

        for key in block_texture_keys(definition) {
            assert_ne!(
                key, "humancraft:missing",
                "{} should not use the missing texture fallback",
                definition.key
            );
            assert!(
                load_texture_pixels(key).is_some(),
                "{} references an invalid texture key {key}",
                definition.key
            );
        }
    }
}

#[test]
fn every_registered_item_uses_loadable_texture() {
    let content = bootstrap_content().unwrap();

    for (_, definition) in content.items.iter() {
        assert_ne!(
            definition.texture, "humancraft:missing",
            "{} should not use the missing texture fallback",
            definition.key
        );
        assert!(
            load_texture_pixels(&definition.texture).is_some(),
            "{} references an invalid texture key {}",
            definition.key,
            definition.texture
        );
    }
}

#[test]
fn oak_textures_load_and_leaves_have_cutout_alpha() {
    let content = bootstrap_content().unwrap();
    let (_, log) = content
        .blocks
        .get_by_key("humancraft:oak_log")
        .expect("oak log should be registered");
    let (_, leaves) = content
        .blocks
        .get_by_key("humancraft:oak_leaves")
        .expect("oak leaves should be registered");

    assert!(load_texture_pixels(texture_key_for_direction(log, FaceDirection::Up)).is_some());

    let pixels = load_texture_pixels(texture_key_for_direction(leaves, FaceDirection::North))
        .expect("oak leaves texture should load");
    let transparent_pixels = pixels.chunks_exact(4).filter(|pixel| pixel[3] < 32).count();

    assert!(transparent_pixels > 72);
}
