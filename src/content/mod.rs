//! HumanCraft content bootstrap.
//!
//! This module may name concrete blocks and items. Engine modules should not.

use crate::engine::registry::RegistryError;
use crate::engine::world::generation::GenerationPipeline;
use crate::engine::world::generation::ore::{OreDefinition, OreStage};
use crate::engine::world::generation::terrain::TerrainStage;
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
        },
    })
}

pub fn default_generation_pipeline(blocks: BlockIds) -> GenerationPipeline {
    GenerationPipeline::new()
        .add_stage(TerrainStage::new(blocks.grass, blocks.dirt, blocks.stone))
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
    use crate::engine::world::{BlockPosition, ChunkPosition, generation::GenerationContext};

    #[test]
    fn bootstrap_registers_initial_content() {
        let content = bootstrap_content().unwrap();

        assert!(content.blocks.get_by_key("humancraft:stone").is_some());
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
            vec!["engine:terrain", "engine:ores"]
        );
    }
}
