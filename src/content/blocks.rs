//! HumanCraft block registration.

use crate::engine::registry::RegistryError;
use crate::engine::world::{BlockDefinition, BlockId, BlockRegistry, BlockTextures};

#[derive(Debug, Copy, Clone)]
pub struct BlockIds {
    pub air: BlockId,
    pub grass: BlockId,
    pub dirt: BlockId,
    pub stone: BlockId,
    pub cobblestone: BlockId,
    pub coal_ore: BlockId,
    pub iron_ore: BlockId,
    pub gold_ore: BlockId,
    pub diamond_ore: BlockId,
    pub oak_log: BlockId,
    pub oak_leaves: BlockId,
    pub sand: BlockId,
    pub sandstone: BlockId,
    pub bedrock: BlockId,
}

pub fn register_blocks(blocks: &mut BlockRegistry) -> Result<BlockIds, RegistryError> {
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
    let cobblestone = blocks.register(
        BlockDefinition::new("humancraft:cobblestone", "Cobblestone")
            .hardness(2.0)
            .drops(["humancraft:cobblestone"])
            .tags(["terrain", "stone", "ore_host"])
            .textures(BlockTextures::all("humancraft:block/cobblestone/top")),
    )?;
    let coal_ore = blocks.register(
        BlockDefinition::new("humancraft:coal_ore", "Coal Ore")
            .hardness(3.0)
            .drops(["humancraft:coal"])
            .tags(["ore", "stone"])
            .textures(BlockTextures::all("humancraft:block/coal_ore/top")),
    )?;
    let iron_ore = blocks.register(
        BlockDefinition::new("humancraft:iron_ore", "Iron Ore")
            .hardness(3.0)
            .drops(["humancraft:raw_iron"])
            .tags(["ore", "stone"])
            .textures(BlockTextures::all("humancraft:block/iron_ore/top")),
    )?;
    let gold_ore = blocks.register(
        BlockDefinition::new("humancraft:gold_ore", "Gold Ore")
            .hardness(3.0)
            .drops(["humancraft:raw_gold"])
            .tags(["ore", "stone"])
            .textures(BlockTextures::all("humancraft:block/gold_ore/top")),
    )?;
    let diamond_ore = blocks.register(
        BlockDefinition::new("humancraft:diamond_ore", "Diamond Ore")
            .hardness(3.0)
            .drops(["humancraft:diamond"])
            .tags(["ore", "stone"])
            .textures(BlockTextures::all("humancraft:block/diamond_ore/top")),
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
    let sand = blocks.register(
        BlockDefinition::new("humancraft:sand", "Sand")
            .hardness(0.5)
            .drops(["humancraft:sand"])
            .tags(["terrain", "sand"])
            .textures(BlockTextures::all("humancraft:block/sand/top")),
    )?;
    let sandstone = blocks.register(
        BlockDefinition::new("humancraft:sandstone", "Sandstone")
            .hardness(0.8)
            .drops(["humancraft:sandstone"])
            .tags(["terrain", "stone", "ore_host"])
            .textures(BlockTextures::all("humancraft:block/sandstone/top")),
    )?;
    let bedrock = blocks.register(
        BlockDefinition::new("humancraft:bedrock", "Bedrock")
            .hardness(f32::INFINITY)
            .drops(std::iter::empty::<&str>())
            .tags(["terrain", "stone", "unbreakable"])
            .textures(BlockTextures::all("humancraft:block/bedrock/top")),
    )?;

    Ok(BlockIds {
        air,
        grass,
        dirt,
        stone,
        cobblestone,
        coal_ore,
        iron_ore,
        gold_ore,
        diamond_ore,
        oak_log,
        oak_leaves,
        sand,
        sandstone,
        bedrock,
    })
}
