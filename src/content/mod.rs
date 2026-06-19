//! HumanCraft content bootstrap.
//!
//! This module may name concrete blocks and items. Engine modules should not.

mod blocks;
mod generation;
mod items;
mod recipes;

pub use blocks::BlockIds;
pub use generation::default_generation_pipeline;

use crate::engine::registry::RegistryError;
use crate::engine::world::{
    BlockRegistry, CraftingRecipeRegistry, ItemRegistry, SmeltingRecipeRegistry,
};

#[derive(Debug, Clone)]
pub struct GameContent {
    pub blocks: BlockRegistry,
    pub items: ItemRegistry,
    pub recipes: CraftingRecipeRegistry,
    pub smelting_recipes: SmeltingRecipeRegistry,
    pub block_ids: BlockIds,
}

pub fn bootstrap_content() -> Result<GameContent, RegistryError> {
    let mut blocks = BlockRegistry::new();
    let block_ids = blocks::register_blocks(&mut blocks)?;

    let mut items = ItemRegistry::new();
    items::register_items(&mut items, block_ids)?;

    let mut recipes = CraftingRecipeRegistry::new();
    recipes::register_recipes(&mut recipes)?;
    let mut smelting_recipes = SmeltingRecipeRegistry::new();
    recipes::register_smelting_recipes(&mut smelting_recipes)?;

    Ok(GameContent {
        blocks,
        items,
        recipes,
        smelting_recipes,
        block_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::world::{
        BlockPosition, ChunkPosition, Inventory, ItemStack, crafting_result,
        generation::biome::{BiomeDefinition, BiomeSource, ExposedSurfaceRule, TerrainLayer},
        generation::{GenerationContext, GenerationPipeline, terrain::TerrainStage},
    };

    use crate::content::generation::overworld_biome_source;

    #[test]
    fn bootstrap_registers_initial_content() {
        let content = bootstrap_content().unwrap();

        assert!(content.blocks.get_by_key("humancraft:stone").is_some());
        assert!(
            content
                .blocks
                .get_by_key("humancraft:cobblestone")
                .is_some()
        );
        assert!(content.blocks.get_by_key("humancraft:oak_log").is_some());
        assert!(content.blocks.get_by_key("humancraft:oak_leaves").is_some());
        assert!(content.blocks.get_by_key("humancraft:oak_planks").is_some());
        assert!(
            content
                .blocks
                .get_by_key("humancraft:crafting_table")
                .is_some()
        );
        assert!(content.blocks.get_by_key("humancraft:sand").is_some());
        assert!(content.blocks.get_by_key("humancraft:sandstone").is_some());
        assert!(content.blocks.get_by_key("humancraft:bedrock").is_some());
        assert!(content.blocks.get_by_key("humancraft:glass").is_some());
        assert!(content.blocks.get_by_key("humancraft:chest").is_some());
        assert!(content.blocks.get_by_key("humancraft:furnace").is_some());
        assert!(
            content
                .blocks
                .get_by_key("humancraft:wooden_stairs")
                .is_some()
        );
        assert!(
            content
                .blocks
                .get_by_key("humancraft:wooden_slab")
                .is_some()
        );
        assert!(content.items.get_by_key("humancraft:diamond").is_some());
        assert!(content.items.get_by_key("humancraft:stick").is_some());
        assert!(content.items.get_by_key("humancraft:iron_ingot").is_some());
        assert!(
            content
                .items
                .get_by_key("humancraft:wood_pickaxe")
                .is_some()
        );
        assert!(content.items.get_by_key("humancraft:diamond_axe").is_some());
        assert!(content.items.get_by_key("humancraft:oak_planks").is_some());
        assert!(
            content
                .items
                .get_by_key("humancraft:crafting_table")
                .is_some()
        );
        assert_eq!(
            content
                .items
                .get_by_key("humancraft:chest")
                .map(|(_, item)| item.max_stack_size),
            Some(1)
        );
        assert!(
            content
                .recipes
                .get_by_key("humancraft:oak_planks_from_oak_log")
                .is_some()
        );
        assert!(
            content
                .recipes
                .get_by_key("humancraft:crafting_table_from_oak_planks")
                .is_some()
        );
        assert!(
            content
                .recipes
                .get_by_key("humancraft:sticks_from_oak_planks")
                .is_some()
        );
        assert!(
            content
                .recipes
                .get_by_key("humancraft:chest_from_oak_planks")
                .is_some()
        );
        assert!(
            content
                .recipes
                .get_by_key("humancraft:furnace_from_cobblestone")
                .is_some()
        );
        assert!(
            content
                .recipes
                .get_by_key("humancraft:wooden_stairs_from_oak_planks")
                .is_some()
        );
        assert!(
            content
                .recipes
                .get_by_key("humancraft:wooden_slab_from_oak_planks")
                .is_some()
        );
        assert!(
            content
                .recipes
                .get_by_key("humancraft:iron_pickaxe")
                .is_some()
        );
        assert_eq!(
            content
                .items
                .get_by_key("humancraft:cobblestone")
                .and_then(|(_, item)| item.place_block.as_deref()),
            Some("humancraft:cobblestone")
        );
        assert_eq!(content.block_ids.air.raw(), 0);
    }

    #[test]
    fn furnace_content_defines_fuels_and_smelting_recipes() {
        let content = bootstrap_content().unwrap();
        let coal = content.items.get_by_key("humancraft:coal").unwrap().1;
        let planks = content.items.get_by_key("humancraft:oak_planks").unwrap().1;
        let stick = content.items.get_by_key("humancraft:stick").unwrap().1;

        assert_eq!(coal.fuel_ticks, Some(1600));
        assert_eq!(planks.fuel_ticks, Some(300));
        assert_eq!(stick.fuel_ticks, Some(100));
        assert!(
            content
                .smelting_recipes
                .get_by_key("humancraft:smelt_sand_to_glass")
                .is_some()
        );
        assert!(
            content
                .smelting_recipes
                .get_by_key("humancraft:smelt_raw_iron_to_iron_ingot")
                .is_some()
        );
    }

    #[test]
    fn crafting_table_recipe_uses_two_by_two_oak_planks() {
        let content = bootstrap_content().unwrap();
        let oak_planks = content.items.id_for_key("humancraft:oak_planks").unwrap();
        let crafting_table = content
            .items
            .id_for_key("humancraft:crafting_table")
            .unwrap();
        let mut grid = Inventory::new(4, 0);

        for slot in 0..4 {
            grid.set_slot(slot, Some(ItemStack::new(oak_planks, 1)));
        }

        assert_eq!(
            crafting_result(&content.recipes, &content.items, &grid, 2),
            Some(ItemStack::new(crafting_table, 1))
        );
    }

    #[test]
    fn stick_recipe_uses_two_vertical_oak_planks() {
        let content = bootstrap_content().unwrap();
        let oak_planks = content.items.id_for_key("humancraft:oak_planks").unwrap();
        let stick = content.items.id_for_key("humancraft:stick").unwrap();
        let mut grid = Inventory::new(4, 0);
        grid.set_slot(0, Some(ItemStack::new(oak_planks, 1)));
        grid.set_slot(2, Some(ItemStack::new(oak_planks, 1)));

        assert_eq!(
            crafting_result(&content.recipes, &content.items, &grid, 2),
            Some(ItemStack::new(stick, 4))
        );
    }

    #[test]
    fn iron_pickaxe_recipe_uses_original_three_by_three_shape() {
        let content = bootstrap_content().unwrap();
        let iron_ingot = content.items.id_for_key("humancraft:iron_ingot").unwrap();
        let stick = content.items.id_for_key("humancraft:stick").unwrap();
        let iron_pickaxe = content.items.id_for_key("humancraft:iron_pickaxe").unwrap();
        let mut grid = Inventory::new(9, 0);
        for slot in 0..3 {
            grid.set_slot(slot, Some(ItemStack::new(iron_ingot, 1)));
        }
        grid.set_slot(4, Some(ItemStack::new(stick, 1)));
        grid.set_slot(7, Some(ItemStack::new(stick, 1)));

        assert_eq!(
            crafting_result(&content.recipes, &content.items, &grid, 3),
            Some(ItemStack::new(iron_pickaxe, 1))
        );
    }

    #[test]
    fn chest_recipe_uses_ring_of_oak_planks() {
        let content = bootstrap_content().unwrap();
        let oak_planks = content.items.id_for_key("humancraft:oak_planks").unwrap();
        let chest = content.items.id_for_key("humancraft:chest").unwrap();
        let mut grid = Inventory::new(9, 0);
        for slot in [0, 1, 2, 3, 5, 6, 7, 8] {
            grid.set_slot(slot, Some(ItemStack::new(oak_planks, 1)));
        }

        assert_eq!(
            crafting_result(&content.recipes, &content.items, &grid, 3),
            Some(ItemStack::new(chest, 1))
        );
    }

    #[test]
    fn furnace_recipe_uses_ring_of_cobblestone() {
        let content = bootstrap_content().unwrap();
        let cobblestone = content.items.id_for_key("humancraft:cobblestone").unwrap();
        let furnace = content.items.id_for_key("humancraft:furnace").unwrap();
        let mut grid = Inventory::new(9, 0);
        for slot in [0, 1, 2, 3, 5, 6, 7, 8] {
            grid.set_slot(slot, Some(ItemStack::new(cobblestone, 1)));
        }

        assert_eq!(
            crafting_result(&content.recipes, &content.items, &grid, 3),
            Some(ItemStack::new(furnace, 1))
        );
    }

    #[test]
    fn wooden_stair_recipe_uses_stair_shape() {
        let content = bootstrap_content().unwrap();
        let oak_planks = content.items.id_for_key("humancraft:oak_planks").unwrap();
        let stairs = content
            .items
            .id_for_key("humancraft:wooden_stairs")
            .unwrap();
        let mut grid = Inventory::new(9, 0);
        for slot in [0, 3, 4, 6, 7, 8] {
            grid.set_slot(slot, Some(ItemStack::new(oak_planks, 1)));
        }

        assert_eq!(
            crafting_result(&content.recipes, &content.items, &grid, 3),
            Some(ItemStack::new(stairs, 4))
        );
    }

    #[test]
    fn wooden_slab_recipe_uses_three_oak_planks() {
        let content = bootstrap_content().unwrap();
        let oak_planks = content.items.id_for_key("humancraft:oak_planks").unwrap();
        let slab = content.items.id_for_key("humancraft:wooden_slab").unwrap();
        let mut grid = Inventory::new(9, 0);
        for slot in [3, 4, 5] {
            grid.set_slot(slot, Some(ItemStack::new(oak_planks, 1)));
        }

        assert_eq!(
            crafting_result(&content.recipes, &content.items, &grid, 3),
            Some(ItemStack::new(slab, 6))
        );
    }

    #[test]
    fn block_drops_resolve_to_registered_items() {
        let content = bootstrap_content().unwrap();

        for (_, block) in content.blocks.iter() {
            for drop in &block.drops {
                assert!(
                    content.items.get_by_key(drop).is_some(),
                    "{} drops unknown item {drop}",
                    block.key
                );
            }
        }
    }

    #[test]
    fn default_pipeline_generates_non_empty_chunk() {
        let content = bootstrap_content().unwrap();
        let pipeline = default_generation_pipeline(content.block_ids);
        let context = GenerationContext {
            seed: 1234,
            air: content.block_ids.air,
        };
        let chunk = pipeline.generate_chunk(ChunkPosition { x: 0, z: 0 }, &context);

        assert_ne!(
            chunk.block(BlockPosition { x: 0, y: 0, z: 0 }),
            Some(content.block_ids.air)
        );
        assert_eq!(
            pipeline.stage_names(),
            vec![
                "engine:terrain",
                "engine:ores",
                "engine:bedrock",
                "engine:trees"
            ]
        );
    }

    #[test]
    fn overworld_defines_required_biomes() {
        let content = bootstrap_content().unwrap();
        let source = overworld_biome_source(content.block_ids);
        let keys: Vec<&str> = source
            .biomes()
            .iter()
            .map(|biome| biome.key.as_str())
            .collect();

        assert_eq!(
            keys,
            vec![
                "humancraft:plains",
                "humancraft:forest",
                "humancraft:mountains",
                "humancraft:desert"
            ]
        );
    }

    #[test]
    fn overworld_terrain_starts_at_least_at_sea_level() {
        let content = bootstrap_content().unwrap();
        let terrain = TerrainStage::new(overworld_biome_source(content.block_ids));

        for x in -160..160 {
            for z in -160..160 {
                assert!(terrain.height_at(1234, x, z) >= 64);
            }
        }
    }

    #[test]
    fn default_pipeline_generates_bedrock_bottom_layer() {
        let content = bootstrap_content().unwrap();
        let pipeline = default_generation_pipeline(content.block_ids);
        let context = GenerationContext {
            seed: 1234,
            air: content.block_ids.air,
        };
        let chunk = pipeline.generate_chunk(ChunkPosition { x: 0, z: 0 }, &context);

        for x in 0..crate::engine::world::CHUNK_SIZE {
            for z in 0..crate::engine::world::CHUNK_SIZE {
                assert_eq!(
                    chunk.block(BlockPosition { x, y: 0, z }),
                    Some(content.block_ids.bedrock)
                );
            }
        }
    }

    #[test]
    fn desert_biome_uses_sand_then_sandstone_then_stone() {
        let content = bootstrap_content().unwrap();
        let desert = BiomeDefinition::new(
            "test:desert",
            content.block_ids.sand,
            content.block_ids.sandstone,
            content.block_ids.stone,
        )
        .terrain(64, 1, 4, 96, 32)
        .layers([
            TerrainLayer::new(content.block_ids.sand, 5),
            TerrainLayer::new(content.block_ids.sandstone, 6),
        ]);
        let terrain = TerrainStage::new(BiomeSource::new(vec![desert]));
        let context = GenerationContext {
            seed: 1234,
            air: content.block_ids.air,
        };
        let chunk = GenerationPipeline::new()
            .add_stage(terrain.clone())
            .generate_chunk(ChunkPosition { x: 0, z: 0 }, &context);
        let surface_y = terrain.height_at(context.seed, 0, 0);

        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: surface_y,
                z: 0
            }),
            Some(content.block_ids.sand)
        );
        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: surface_y - 5,
                z: 0
            }),
            Some(content.block_ids.sandstone)
        );
        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: surface_y - 11,
                z: 0
            }),
            Some(content.block_ids.stone)
        );
    }

    #[test]
    fn biome_profiles_have_meaningful_height_variation() {
        let content = bootstrap_content().unwrap();
        let source = overworld_biome_source(content.block_ids);
        let terrain = TerrainStage::new(source.clone());

        for biome in source.biomes() {
            let mut min_height = usize::MAX;
            let mut max_height = 0;
            for x in (0..256).step_by(4) {
                for z in (0..256).step_by(4) {
                    let height = terrain.height_for_biome(1234, x, z, biome);
                    min_height = min_height.min(height);
                    max_height = max_height.max(height);
                }
            }

            let expected_variation = if biome.key == "humancraft:mountains" {
                24
            } else {
                8
            };
            assert!(
                max_height - min_height >= expected_variation,
                "{} variation was only {}",
                biome.key,
                max_height - min_height
            );
        }
    }

    #[test]
    fn mountain_profile_exposes_stone_surfaces() {
        let content = bootstrap_content().unwrap();
        let mountain = BiomeDefinition::new(
            "test:mountains",
            content.block_ids.grass,
            content.block_ids.dirt,
            content.block_ids.stone,
        )
        .terrain(84, 44, 3, 104, 20)
        .relief(12.0, 12.0, 52)
        .exposed_surface(ExposedSurfaceRule::new(content.block_ids.stone, None, 3));
        let terrain = TerrainStage::new(BiomeSource::new(vec![mountain]));
        let context = GenerationContext {
            seed: 1234,
            air: content.block_ids.air,
        };
        let mut exposed_stone = 0;

        for chunk_x in 0..3 {
            for chunk_z in 0..3 {
                let chunk_position = ChunkPosition {
                    x: chunk_x,
                    z: chunk_z,
                };
                let chunk = GenerationPipeline::new()
                    .add_stage(terrain.clone())
                    .generate_chunk(chunk_position, &context);
                for x in 0..crate::engine::world::CHUNK_SIZE {
                    for z in 0..crate::engine::world::CHUNK_SIZE {
                        let world_x = chunk_x * crate::engine::world::CHUNK_SIZE as i32 + x as i32;
                        let world_z = chunk_z * crate::engine::world::CHUNK_SIZE as i32 + z as i32;
                        let surface_y = terrain.height_at(context.seed, world_x, world_z);
                        if chunk.block(BlockPosition { x, y: surface_y, z })
                            == Some(content.block_ids.stone)
                        {
                            exposed_stone += 1;
                        }
                    }
                }
            }
        }

        assert!(exposed_stone > 0);
    }

    #[test]
    fn mountain_profile_keeps_some_high_grassy_tops() {
        let content = bootstrap_content().unwrap();
        let mountain = BiomeDefinition::new(
            "test:mountains",
            content.block_ids.grass,
            content.block_ids.dirt,
            content.block_ids.stone,
        )
        .terrain(84, 44, 3, 104, 20)
        .relief(12.0, 12.0, 52)
        .exposed_surface(ExposedSurfaceRule::new(content.block_ids.stone, None, 3));
        let terrain = TerrainStage::new(BiomeSource::new(vec![mountain]));
        let context = GenerationContext {
            seed: 1234,
            air: content.block_ids.air,
        };
        let chunk = GenerationPipeline::new()
            .add_stage(terrain.clone())
            .generate_chunk(ChunkPosition { x: 0, z: 0 }, &context);
        let mut high_grass = 0;

        for x in 0..crate::engine::world::CHUNK_SIZE {
            for z in 0..crate::engine::world::CHUNK_SIZE {
                let surface_y = terrain.height_at(context.seed, x as i32, z as i32);
                if surface_y >= 96
                    && chunk.block(BlockPosition { x, y: surface_y, z })
                        == Some(content.block_ids.grass)
                {
                    high_grass += 1;
                }
            }
        }

        assert!(high_grass > 0);
    }

    #[test]
    fn overworld_terrain_stays_continuous_across_sampled_area() {
        let content = bootstrap_content().unwrap();
        let terrain = TerrainStage::new(overworld_biome_source(content.block_ids));

        for x in -160..160 {
            for z in -160..160 {
                let height = terrain.height_at(1234, x, z);
                let east = terrain.height_at(1234, x + 1, z);
                let south = terrain.height_at(1234, x, z + 1);

                assert!(
                    height.abs_diff(east) <= 6,
                    "east height jumped from {height} to {east} at {x},{z}"
                );
                assert!(
                    height.abs_diff(south) <= 6,
                    "south height jumped from {height} to {south} at {x},{z}"
                );
            }
        }
    }

    #[test]
    fn default_pipeline_generates_trees() {
        let content = bootstrap_content().unwrap();
        let pipeline = default_generation_pipeline(content.block_ids);
        let context = GenerationContext {
            seed: 1234,
            air: content.block_ids.air,
        };

        let mut has_log = false;
        let mut has_leaves = false;
        for x in -2..=2 {
            for z in -2..=2 {
                let chunk = pipeline.generate_chunk(ChunkPosition { x, z }, &context);
                has_log |= chunk.blocks().contains(&content.block_ids.oak_log);
                has_leaves |= chunk.blocks().contains(&content.block_ids.oak_leaves);
            }
        }

        assert!(has_log);
        assert!(has_leaves);
    }
}
