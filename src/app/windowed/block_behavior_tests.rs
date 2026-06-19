use super::*;
use crate::app::windowed::client_world::FurnaceEntity;
use crate::content::{GameContent, bootstrap_content, default_generation_pipeline};
use crate::engine::mesh::chunk_mesher::ChunkMesher;
use crate::engine::world::{BlockPosition, BlockProperties, BlockState, Chunk, ChunkPosition};

fn test_client_world(content: &GameContent) -> ClientWorld {
    ClientWorld::new(
        content.blocks.clone(),
        content.items.clone(),
        content.recipes.clone(),
        content.smelting_recipes.clone(),
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

fn empty_chunk(content: &GameContent) -> Chunk {
    Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air)
}

#[test]
fn saplings_are_placeable_cross_blocks() {
    let content = bootstrap_content().unwrap();
    let mut chunk = empty_chunk(&content);
    chunk
        .set_block(
            BlockPosition { x: 2, y: 1, z: 2 },
            content.block_ids.oak_sapling,
        )
        .unwrap();

    let mesh = ChunkMesher.mesh_chunk(&chunk, &content.blocks);

    assert_eq!(mesh.quads.len(), 4);
    assert!(
        content
            .items
            .id_for_key("humancraft:oak_sapling")
            .and_then(|item| content.items.get(item))
            .and_then(|item| item.place_block.as_deref())
            .is_some_and(|block| block == "humancraft:oak_sapling")
    );
}

#[test]
fn saplings_are_raycast_targets_and_break_immediately() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    world.insert_chunk(empty_chunk(&content));
    world.set_block_state(
        WorldBlockPosition { x: 6, y: 8, z: 6 },
        BlockState::with_properties(
            content.block_ids.oak_sapling,
            BlockProperties::Sapling { stage: 0 },
        ),
    );

    let hit = world.raycast(Vec3::new(-4.0, -55.5, -1.5), Vec3::X);

    assert_eq!(
        hit.map(|hit| hit.block),
        Some(WorldBlockPosition { x: 6, y: 8, z: 6 })
    );
    let dirty = world.continue_breaking_block(WorldBlockPosition { x: 6, y: 8, z: 6 }, 0.0, None);
    assert!(!dirty.is_empty());
    assert_eq!(
        world.block(WorldBlockPosition { x: 6, y: 8, z: 6 }),
        Some(content.block_ids.air)
    );
}

#[test]
fn oak_leaves_use_fifteen_percent_sapling_chance() {
    let content = bootstrap_content().unwrap();
    let leaves = content.blocks.get(content.block_ids.oak_leaves).unwrap();
    let chance = leaves
        .behavior
        .leaf_decay
        .as_ref()
        .map(|behavior| behavior.sapling_drop.chance);

    assert_eq!(chance, Some(0.15));
}

#[test]
fn breaking_many_oak_leaves_can_drop_saplings() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    world.insert_chunk(empty_chunk(&content));
    let sapling = content.items.id_for_key("humancraft:oak_sapling").unwrap();

    for x in 0..16 {
        for z in 0..16 {
            let position = WorldBlockPosition { x, y: 8, z };
            world.set_block_state(
                position,
                BlockState::with_properties(
                    content.block_ids.oak_leaves,
                    BlockProperties::Leaves { persistent: true },
                ),
            );
            world.break_block(position);
        }
    }

    assert!(
        world
            .loot_entities
            .iter()
            .any(|loot| loot.stack.item == sapling)
    );
}

#[test]
fn burning_furnace_repairs_plain_block_state_to_lit_furnace_state() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = empty_chunk(&content);
    let position = WorldBlockPosition { x: 2, y: 2, z: 2 };
    chunk
        .set_block(
            BlockPosition { x: 2, y: 2, z: 2 },
            content.block_ids.furnace,
        )
        .unwrap();
    world.insert_chunk(chunk);

    let sand = content.items.id_for_key("humancraft:sand").unwrap();
    let coal = content.items.id_for_key("humancraft:coal").unwrap();
    let mut inventory = Inventory::new(3, 0);
    inventory.set_slot(0, Some(ItemStack::new(sand, 1)));
    inventory.set_slot(1, Some(ItemStack::new(coal, 1)));
    world.block_entities.insert(
        position,
        BlockEntity::Furnace(FurnaceEntity {
            inventory,
            burn_ticks: 0,
            fuel_ticks: 0,
            cook_ticks: 0,
            cook_ticks_total: 200,
        }),
    );

    world.tick_block_entities();

    assert!(matches!(
        world.block_state(position).map(|state| state.properties),
        Some(BlockProperties::Furnace { lit: true, .. })
    ));
}

#[test]
fn generated_leaves_decay_but_player_placed_leaves_persist() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = empty_chunk(&content);
    chunk
        .set_block_state(
            BlockPosition { x: 2, y: 4, z: 2 },
            BlockState::with_properties(
                content.block_ids.oak_leaves,
                BlockProperties::Leaves { persistent: false },
            ),
        )
        .unwrap();
    chunk
        .set_block_state(
            BlockPosition { x: 4, y: 4, z: 2 },
            BlockState::with_properties(
                content.block_ids.oak_leaves,
                BlockProperties::Leaves { persistent: true },
            ),
        )
        .unwrap();
    world.insert_chunk(chunk);

    world.tick_all_block_behaviors_for_tests();

    assert_eq!(
        world.block(WorldBlockPosition { x: 2, y: 4, z: 2 }),
        Some(content.block_ids.air)
    );
    assert_eq!(
        world.block(WorldBlockPosition { x: 4, y: 4, z: 2 }),
        Some(content.block_ids.oak_leaves)
    );
}

#[test]
fn connected_generated_leaves_do_not_decay() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = empty_chunk(&content);
    chunk
        .set_block(
            BlockPosition { x: 2, y: 4, z: 2 },
            content.block_ids.oak_log,
        )
        .unwrap();
    chunk
        .set_block_state(
            BlockPosition { x: 3, y: 4, z: 2 },
            BlockState::with_properties(
                content.block_ids.oak_leaves,
                BlockProperties::Leaves { persistent: false },
            ),
        )
        .unwrap();
    world.insert_chunk(chunk);

    world.tick_all_block_behaviors_for_tests();

    assert_eq!(
        world.block(WorldBlockPosition { x: 3, y: 4, z: 2 }),
        Some(content.block_ids.oak_leaves)
    );
}

#[test]
fn grass_spreads_to_clear_adjacent_dirt() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = empty_chunk(&content);
    chunk
        .set_block(BlockPosition { x: 8, y: 4, z: 8 }, content.block_ids.grass)
        .unwrap();
    for x in 7..=9 {
        for z in 7..=9 {
            chunk
                .set_block(BlockPosition { x, y: 4, z }, content.block_ids.dirt)
                .unwrap();
        }
    }
    chunk
        .set_block(BlockPosition { x: 8, y: 4, z: 8 }, content.block_ids.grass)
        .unwrap();
    world.insert_chunk(chunk);

    for _ in 0..32 {
        world.tick_all_block_behaviors_for_tests();
    }

    let grass_count = (7..=9)
        .flat_map(|x| (7..=9).map(move |z| WorldBlockPosition { x, y: 4, z }))
        .filter(|position| world.block(*position) == Some(content.block_ids.grass))
        .count();
    assert!(grass_count > 1);
}

#[test]
fn oak_saplings_grow_into_tree_blocks_when_space_is_clear() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    let mut chunk = empty_chunk(&content);
    chunk
        .set_block(BlockPosition { x: 8, y: 3, z: 8 }, content.block_ids.grass)
        .unwrap();
    chunk
        .set_block_state(
            BlockPosition { x: 8, y: 4, z: 8 },
            BlockState::with_properties(
                content.block_ids.oak_sapling,
                BlockProperties::Sapling { stage: 1 },
            ),
        )
        .unwrap();
    world.insert_chunk(chunk);

    world.tick_all_block_behaviors_for_tests();

    assert_eq!(
        world.block(WorldBlockPosition { x: 8, y: 4, z: 8 }),
        Some(content.block_ids.oak_log)
    );
    assert!(
        world
            .chunks
            .get(&ChunkPosition { x: 0, z: 0 })
            .unwrap()
            .blocks()
            .contains(&content.block_ids.oak_leaves)
    );
    assert_eq!(
        world.block(WorldBlockPosition { x: 8, y: 3, z: 8 }),
        Some(content.block_ids.dirt)
    );
}

#[test]
fn sand_falls_into_clear_space_below() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    world.insert_chunk(empty_chunk(&content));
    world.set_block(
        WorldBlockPosition { x: 6, y: 8, z: 6 },
        content.block_ids.sand,
    );

    world.tick_all_block_behaviors_for_tests();

    assert_eq!(
        world.block(WorldBlockPosition { x: 6, y: 8, z: 6 }),
        Some(content.block_ids.sand)
    );
    for _ in 0..6 {
        world.tick_all_block_behaviors_for_tests();
    }

    assert_eq!(
        world.block(WorldBlockPosition { x: 6, y: 8, z: 6 }),
        Some(content.block_ids.air)
    );
    assert_eq!(
        world.block(WorldBlockPosition { x: 6, y: 7, z: 6 }),
        Some(content.block_ids.sand)
    );
}

#[test]
fn removing_support_schedules_sand_to_fall() {
    let content = bootstrap_content().unwrap();
    let mut world = test_client_world(&content);
    world.insert_chunk(empty_chunk(&content));
    world.set_block(
        WorldBlockPosition { x: 6, y: 7, z: 6 },
        content.block_ids.dirt,
    );
    world.set_block(
        WorldBlockPosition { x: 6, y: 8, z: 6 },
        content.block_ids.sand,
    );
    world.tick_all_block_behaviors_for_tests();

    world.set_block(
        WorldBlockPosition { x: 6, y: 7, z: 6 },
        content.block_ids.air,
    );
    world.tick_all_block_behaviors_for_tests();

    assert_eq!(
        world.block(WorldBlockPosition { x: 6, y: 8, z: 6 }),
        Some(content.block_ids.sand)
    );
    for _ in 0..6 {
        world.tick_all_block_behaviors_for_tests();
    }

    assert_eq!(
        world.block(WorldBlockPosition { x: 6, y: 8, z: 6 }),
        Some(content.block_ids.air)
    );
    assert_eq!(
        world.block(WorldBlockPosition { x: 6, y: 7, z: 6 }),
        Some(content.block_ids.sand)
    );
}
