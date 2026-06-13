//! HumanCraft content bootstrap.
//!
//! This module may name concrete blocks and items. Engine modules should not.

use crate::engine::registry::RegistryError;
use crate::engine::world::generation::GenerationPipeline;
use crate::engine::world::generation::biome::{BiomeDefinition, BiomeSource};
use crate::engine::world::generation::ore::{OreDefinition, OreStage};
use crate::engine::world::generation::terrain::TerrainStage;
use crate::engine::world::generation::tree::{TreeDefinition, TreeStage};
use crate::engine::world::{
    BlockDefinition, BlockId, BlockRegistry, BlockTextures, ItemDefinition, ItemRegistry,
};

#[derive(Debug, Clone)]
pub struct GameContent {
    pub blocks: BlockRegistry,
    pub items: ItemRegistry,
    pub block_ids: BlockIds,
}

#[derive(Debug, Copy, Clone)]
pub struct BlockIds {
    pub air: BlockId,
    pub grass: BlockId,
    pub dirt: BlockId,
    pub stone: BlockId,
    pub coal_ore: BlockId,
    pub iron_ore: BlockId,
    pub gold_ore: BlockId,
    pub diamond_ore: BlockId,
    pub oak_log: BlockId,
    pub oak_leaves: BlockId,
}

pub fn bootstrap_content() -> Result<GameContent, RegistryError> {
    let mut blocks = BlockRegistry::new();
    let air = blocks.register(
        BlockDefinition::new("humancraft:air", "Air")
            .hardness(0.0)
            .transparent(true)
            .solid(false)
            .tags(["replaceable"]),
    )?;
    let grass = blocks.register(
        BlockDefinition::new("humancraft:grass", "Grass Block")
            .hardness(0.6)
            .drops(["humancraft:dirt"])
            .tags(["terrain", "soil"])
            .textures(BlockTextures::top_bottom_sides(
                "humancraft:block/grass/top",
                "humancraft:block/dirt/bottom",
                "humancraft:block/grass/front",
            )),
    )?;
    let dirt = blocks.register(
        BlockDefinition::new("humancraft:dirt", "Dirt")
            .hardness(0.5)
            .drops(["humancraft:dirt"])
            .tags(["terrain", "soil"])
            .textures(BlockTextures::all("humancraft:block/dirt/top")),
    )?;
    let stone = blocks.register(
        BlockDefinition::new("humancraft:stone", "Stone")
            .hardness(1.5)
            .drops(["humancraft:cobblestone"])
            .tags(["terrain", "stone", "ore_host"])
            .textures(BlockTextures::all("humancraft:block/stone/top")),
    )?;
    let coal_ore = blocks.register(
        BlockDefinition::new("humancraft:coal_ore", "Coal Ore")
            .hardness(3.0)
            .drops(["humancraft:coal"])
            .tags(["ore", "stone"]),
    )?;
    let iron_ore = blocks.register(
        BlockDefinition::new("humancraft:iron_ore", "Iron Ore")
            .hardness(3.0)
            .drops(["humancraft:raw_iron"])
            .tags(["ore", "stone"]),
    )?;
    let gold_ore = blocks.register(
        BlockDefinition::new("humancraft:gold_ore", "Gold Ore")
            .hardness(3.0)
            .drops(["humancraft:raw_gold"])
            .tags(["ore", "stone"]),
    )?;
    let diamond_ore = blocks.register(
        BlockDefinition::new("humancraft:diamond_ore", "Diamond Ore")
            .hardness(3.0)
            .drops(["humancraft:diamond"])
            .tags(["ore", "stone"]),
    )?;
    let oak_log = blocks.register(
        BlockDefinition::new("humancraft:oak_log", "Oak Log")
            .hardness(2.0)
            .drops(["humancraft:oak_log"])
            .tags(["wood", "tree_trunk"])
            .textures(BlockTextures::top_bottom_sides(
                "humancraft:block/oak_log/top",
                "humancraft:block/oak_log/bottom",
                "humancraft:block/oak_log/front",
            )),
    )?;
    let oak_leaves = blocks.register(
        BlockDefinition::new("humancraft:oak_leaves", "Oak Leaves")
            .hardness(0.2)
            .transparent(true)
            .drops(["humancraft:oak_sapling"])
            .tags(["leaves", "foliage", "tree_canopy"])
            .textures(BlockTextures::all("humancraft:block/oak_leaves/top")),
    )?;

    let mut items = ItemRegistry::new();
    register_block_item(&mut items, "humancraft:dirt", "Dirt", "humancraft:dirt")?;
    register_block_item(
        &mut items,
        "humancraft:grass",
        "Grass Block",
        "humancraft:grass",
    )?;
    register_block_item(&mut items, "humancraft:stone", "Stone", "humancraft:stone")?;
    register_block_item(
        &mut items,
        "humancraft:coal_ore",
        "Coal Ore",
        "humancraft:coal_ore",
    )?;
    items.register(ItemDefinition::new("humancraft:coal", "Coal").tags(["fuel"]))?;
    items.register(ItemDefinition::new("humancraft:raw_iron", "Raw Iron").tags(["ore_drop"]))?;
    items.register(ItemDefinition::new("humancraft:raw_gold", "Raw Gold").tags(["ore_drop"]))?;
    items.register(ItemDefinition::new("humancraft:diamond", "Diamond").tags(["gem"]))?;
    register_block_item(
        &mut items,
        "humancraft:oak_log",
        "Oak Log",
        "humancraft:oak_log",
    )?;
    register_block_item(
        &mut items,
        "humancraft:oak_leaves",
        "Oak Leaves",
        "humancraft:oak_leaves",
    )?;
    items
        .register(ItemDefinition::new("humancraft:oak_sapling", "Oak Sapling").tags(["sapling"]))?;

    Ok(GameContent {
        blocks,
        items,
        block_ids: BlockIds {
            air,
            grass,
            dirt,
            stone,
            coal_ore,
            iron_ore,
            gold_ore,
            diamond_ore,
            oak_log,
            oak_leaves,
        },
    })
}

pub fn default_generation_pipeline(blocks: BlockIds) -> GenerationPipeline {
    let biome_source = overworld_biome_source(blocks);
    GenerationPipeline::new()
        .add_stage(TerrainStage::new(biome_source.clone()))
        .add_stage(OreStage::new(vec![
            OreDefinition {
                key: "humancraft:coal_ore".to_string(),
                block: blocks.coal_ore,
                replaces: blocks.stone,
                min_y: 4,
                max_y: 96,
                threshold: 0.965,
            },
            OreDefinition {
                key: "humancraft:iron_ore".to_string(),
                block: blocks.iron_ore,
                replaces: blocks.stone,
                min_y: 4,
                max_y: 64,
                threshold: 0.982,
            },
            OreDefinition {
                key: "humancraft:gold_ore".to_string(),
                block: blocks.gold_ore,
                replaces: blocks.stone,
                min_y: 4,
                max_y: 32,
                threshold: 0.992,
            },
            OreDefinition {
                key: "humancraft:diamond_ore".to_string(),
                block: blocks.diamond_ore,
                replaces: blocks.stone,
                min_y: 4,
                max_y: 16,
                threshold: 0.996,
            },
        ]))
        .add_stage(TreeStage::new(
            biome_source,
            vec![
                TreeDefinition::new("humancraft:oak_tree", blocks.oak_log, blocks.oak_leaves)
                    .grow_on([blocks.grass])
                    .replaceable([blocks.air, blocks.oak_leaves])
                    .biomes(["humancraft:forest"])
                    .shape(4, 6, 2)
                    .density(0.055),
                TreeDefinition::new(
                    "humancraft:plains_oak_tree",
                    blocks.oak_log,
                    blocks.oak_leaves,
                )
                .grow_on([blocks.grass])
                .replaceable([blocks.air, blocks.oak_leaves])
                .biomes(["humancraft:plains"])
                .shape(4, 5, 2)
                .density(0.008),
            ],
        ))
}

fn overworld_biome_source(blocks: BlockIds) -> BiomeSource {
    BiomeSource::new(vec![
        BiomeDefinition::new("humancraft:plains", blocks.grass, blocks.dirt, blocks.stone)
            .terrain(62, 7, 4, 96, 32),
        BiomeDefinition::new("humancraft:forest", blocks.grass, blocks.dirt, blocks.stone)
            .terrain(64, 10, 4, 96, 32),
        BiomeDefinition::new(
            "humancraft:mountains",
            blocks.grass,
            blocks.dirt,
            blocks.stone,
        )
        .terrain(72, 26, 3, 128, 40),
    ])
    .with_min_region_chunks(10)
    .with_transition_chunks(2)
}

fn register_block_item(
    items: &mut ItemRegistry,
    key: &str,
    display_name: &str,
    block_key: &str,
) -> Result<(), RegistryError> {
    items
        .register(ItemDefinition::new(key, display_name).place_block(block_key))
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::world::{
        BlockPosition, ChunkPosition,
        generation::{GenerationContext, terrain::TerrainStage},
    };

    #[test]
    fn bootstrap_registers_initial_content() {
        let content = bootstrap_content().unwrap();

        assert!(content.blocks.get_by_key("humancraft:stone").is_some());
        assert!(content.blocks.get_by_key("humancraft:oak_log").is_some());
        assert!(content.blocks.get_by_key("humancraft:oak_leaves").is_some());
        assert!(content.items.get_by_key("humancraft:diamond").is_some());
        assert_eq!(content.block_ids.air.raw(), 0);
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
            vec!["engine:terrain", "engine:ores", "engine:trees"]
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
                "humancraft:mountains"
            ]
        );
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
                    height.abs_diff(east) <= 3,
                    "east height jumped from {height} to {east} at {x},{z}"
                );
                assert!(
                    height.abs_diff(south) <= 3,
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
