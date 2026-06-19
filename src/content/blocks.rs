//! HumanCraft block registration.

use crate::engine::registry::RegistryError;
use crate::engine::world::{
    BlockBehavior, BlockDefinition, BlockId, BlockRegistry, BlockShape, BlockTextures, ChanceDrop,
    GrassSpreadBehavior, LeafDecayBehavior, PlacementRuleKind, SaplingGrowthBehavior, ToolKind,
};

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
    pub oak_sapling: BlockId,
    pub oak_planks: BlockId,
    pub crafting_table: BlockId,
    pub sand: BlockId,
    pub sandstone: BlockId,
    pub bedrock: BlockId,
    pub glass: BlockId,
    pub chest: BlockId,
    pub furnace: BlockId,
    pub wooden_stairs: BlockId,
    pub wooden_slab: BlockId,
}

pub fn register_blocks(blocks: &mut BlockRegistry) -> Result<BlockIds, RegistryError> {
    let air = blocks.register(
        BlockDefinition::new("humancraft:air", "Air")
            .hardness(0.0)
            .transparent(true)
            .solid(false)
            .shape(BlockShape::Empty)
            .tags(["replaceable"]),
    )?;
    let grass = blocks.register(
        BlockDefinition::new("humancraft:grass", "Grass Block")
            .hardness(0.6)
            .drops(["humancraft:dirt"])
            .effective_tool(ToolKind::Shovel)
            .tags(["terrain", "soil"])
            .behavior(BlockBehavior {
                grass_spread: Some(GrassSpreadBehavior {
                    target_block_key: "humancraft:dirt".to_string(),
                    attempts_per_random_tick: 4,
                    horizontal_range: 1,
                    down_range: 3,
                    up_range: 1,
                }),
                ..BlockBehavior::default()
            })
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
            .effective_tool(ToolKind::Shovel)
            .tags(["terrain", "soil"])
            .textures(BlockTextures::all("humancraft:block/dirt/top")),
    )?;
    let stone = blocks.register(
        BlockDefinition::new("humancraft:stone", "Stone")
            .hardness(1.5)
            .drops(["humancraft:cobblestone"])
            .effective_tool(ToolKind::Pickaxe)
            .harvest_requirement(ToolKind::Pickaxe, 1)
            .tags(["terrain", "stone", "ore_host"])
            .textures(BlockTextures::all("humancraft:block/stone/top")),
    )?;
    let cobblestone = blocks.register(
        BlockDefinition::new("humancraft:cobblestone", "Cobblestone")
            .hardness(2.0)
            .drops(["humancraft:cobblestone"])
            .effective_tool(ToolKind::Pickaxe)
            .harvest_requirement(ToolKind::Pickaxe, 1)
            .tags(["terrain", "stone", "ore_host"])
            .textures(BlockTextures::all("humancraft:block/cobblestone/top")),
    )?;
    let coal_ore = blocks.register(
        BlockDefinition::new("humancraft:coal_ore", "Coal Ore")
            .hardness(3.0)
            .drops(["humancraft:coal"])
            .effective_tool(ToolKind::Pickaxe)
            .harvest_requirement(ToolKind::Pickaxe, 1)
            .tags(["ore", "stone"])
            .textures(BlockTextures::all("humancraft:block/coal_ore/top")),
    )?;
    let iron_ore = blocks.register(
        BlockDefinition::new("humancraft:iron_ore", "Iron Ore")
            .hardness(3.0)
            .drops(["humancraft:raw_iron"])
            .effective_tool(ToolKind::Pickaxe)
            .harvest_requirement(ToolKind::Pickaxe, 2)
            .tags(["ore", "stone"])
            .textures(BlockTextures::all("humancraft:block/iron_ore/top")),
    )?;
    let gold_ore = blocks.register(
        BlockDefinition::new("humancraft:gold_ore", "Gold Ore")
            .hardness(3.0)
            .drops(["humancraft:raw_gold"])
            .effective_tool(ToolKind::Pickaxe)
            .harvest_requirement(ToolKind::Pickaxe, 3)
            .tags(["ore", "stone"])
            .textures(BlockTextures::all("humancraft:block/gold_ore/top")),
    )?;
    let diamond_ore = blocks.register(
        BlockDefinition::new("humancraft:diamond_ore", "Diamond Ore")
            .hardness(3.0)
            .drops(["humancraft:diamond"])
            .effective_tool(ToolKind::Pickaxe)
            .harvest_requirement(ToolKind::Pickaxe, 3)
            .tags(["ore", "stone"])
            .textures(BlockTextures::all("humancraft:block/diamond_ore/top")),
    )?;
    let oak_log = blocks.register(
        BlockDefinition::new("humancraft:oak_log", "Oak Log")
            .hardness(2.0)
            .drops(["humancraft:oak_log"])
            .effective_tool(ToolKind::Axe)
            .placement(PlacementRuleKind::AxisFromClickedFace)
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
            .drops(std::iter::empty::<&str>())
            .placement(PlacementRuleKind::PersistentLeaves)
            .tags(["leaves", "foliage", "tree_canopy"])
            .behavior(BlockBehavior {
                leaf_decay: Some(LeafDecayBehavior {
                    log_tag: "tree_trunk".to_string(),
                    max_distance: 6,
                    sapling_drop: ChanceDrop {
                        item_key: "humancraft:oak_sapling".to_string(),
                        chance: 0.15,
                    },
                }),
                ..BlockBehavior::default()
            })
            .textures(BlockTextures::all("humancraft:block/oak_leaves/top")),
    )?;
    let oak_sapling = blocks.register(
        BlockDefinition::new("humancraft:oak_sapling", "Oak Sapling")
            .hardness(0.0)
            .transparent(true)
            .solid(false)
            .drops(["humancraft:oak_sapling"])
            .placement(PlacementRuleKind::Sapling)
            .shape(BlockShape::Cross)
            .tags(["plant", "sapling", "replaceable"])
            .behavior(BlockBehavior {
                sapling_growth: Some(SaplingGrowthBehavior {
                    grow_on_tags: vec!["soil".to_string()],
                    trunk_block_key: "humancraft:oak_log".to_string(),
                    leaves_block_key: "humancraft:oak_leaves".to_string(),
                    min_trunk_height: 4,
                    max_trunk_height: 5,
                    canopy_radius: 2,
                    required_light: 9,
                    required_clearance: 5,
                }),
                ..BlockBehavior::default()
            })
            .textures(BlockTextures::all("humancraft:block/oak_sapling/top")),
    )?;
    let oak_planks = blocks.register(
        BlockDefinition::new("humancraft:oak_planks", "Oak Planks")
            .hardness(2.0)
            .drops(["humancraft:oak_planks"])
            .effective_tool(ToolKind::Axe)
            .tags(["wood", "planks"])
            .textures(BlockTextures::all("humancraft:block/oak_planks/top")),
    )?;
    let crafting_table = blocks.register(
        BlockDefinition::new("humancraft:crafting_table", "Crafting Table")
            .hardness(2.5)
            .drops(["humancraft:crafting_table"])
            .effective_tool(ToolKind::Axe)
            .tags(["wood", "utility", "crafting_table"])
            .textures(BlockTextures {
                top: "humancraft:block/crafting_table/top".to_string(),
                bottom: "humancraft:block/crafting_table/bottom".to_string(),
                north: "humancraft:block/crafting_table/front".to_string(),
                south: "humancraft:block/crafting_table/back".to_string(),
                east: "humancraft:block/crafting_table/right".to_string(),
                west: "humancraft:block/crafting_table/left".to_string(),
            }),
    )?;
    let sand = blocks.register(
        BlockDefinition::new("humancraft:sand", "Sand")
            .hardness(0.5)
            .drops(["humancraft:sand"])
            .effective_tool(ToolKind::Shovel)
            .tags(["terrain", "sand"])
            .behavior(BlockBehavior {
                gravity: true,
                ..BlockBehavior::default()
            })
            .textures(BlockTextures::all("humancraft:block/sand/top")),
    )?;
    let sandstone = blocks.register(
        BlockDefinition::new("humancraft:sandstone", "Sandstone")
            .hardness(0.8)
            .drops(["humancraft:sandstone"])
            .effective_tool(ToolKind::Pickaxe)
            .harvest_requirement(ToolKind::Pickaxe, 1)
            .tags(["terrain", "stone", "ore_host"])
            .textures(BlockTextures::top_bottom_sides(
                "humancraft:block/sandstone/top",
                "humancraft:block/sandstone/bottom",
                "humancraft:block/sandstone/front",
            )),
    )?;
    let bedrock = blocks.register(
        BlockDefinition::new("humancraft:bedrock", "Bedrock")
            .hardness(f32::INFINITY)
            .drops(std::iter::empty::<&str>())
            .tags(["terrain", "stone", "unbreakable"])
            .textures(BlockTextures::all("humancraft:block/bedrock/top")),
    )?;
    let glass = blocks.register(
        BlockDefinition::new("humancraft:glass", "Glass")
            .hardness(0.3)
            .transparent(true)
            .drops(["humancraft:glass"])
            .tags(["glass"])
            .textures(BlockTextures::all("humancraft:block/glass/top")),
    )?;
    let chest = blocks.register(
        BlockDefinition::new("humancraft:chest", "Chest")
            .hardness(2.5)
            .drops(["humancraft:chest"])
            .effective_tool(ToolKind::Axe)
            .placement(PlacementRuleKind::FacePlayerHorizontal)
            .tags(["wood", "utility", "container", "chest"])
            .textures(BlockTextures {
                top: "humancraft:block/chest/top".to_string(),
                bottom: "humancraft:block/chest/bottom".to_string(),
                north: "humancraft:block/chest/front".to_string(),
                south: "humancraft:block/chest/back".to_string(),
                east: "humancraft:block/chest/right".to_string(),
                west: "humancraft:block/chest/left".to_string(),
            }),
    )?;
    let furnace = blocks.register(
        BlockDefinition::new("humancraft:furnace", "Furnace")
            .hardness(3.5)
            .drops(["humancraft:furnace"])
            .effective_tool(ToolKind::Pickaxe)
            .harvest_requirement(ToolKind::Pickaxe, 1)
            .placement(PlacementRuleKind::FacePlayerHorizontal)
            .tags(["stone", "utility", "container", "furnace"])
            .textures(BlockTextures {
                top: "humancraft:block/furnace/top".to_string(),
                bottom: "humancraft:block/furnace/top".to_string(),
                north: "humancraft:block/furnace/front".to_string(),
                south: "humancraft:block/furnace/side".to_string(),
                east: "humancraft:block/furnace/side".to_string(),
                west: "humancraft:block/furnace/side".to_string(),
            }),
    )?;
    let wooden_stairs = blocks.register(
        BlockDefinition::new("humancraft:wooden_stairs", "Wooden Stairs")
            .hardness(2.0)
            .drops(["humancraft:wooden_stairs"])
            .effective_tool(ToolKind::Axe)
            .placement(PlacementRuleKind::Stairs)
            .shape(BlockShape::Stairs)
            .tags(["wood", "stairs"])
            .textures(BlockTextures::all("humancraft:block/oak_planks/top")),
    )?;
    let wooden_slab = blocks.register(
        BlockDefinition::new("humancraft:wooden_slab", "Wooden Slab")
            .hardness(2.0)
            .drops(["humancraft:wooden_slab"])
            .effective_tool(ToolKind::Axe)
            .placement(PlacementRuleKind::Slab)
            .shape(BlockShape::Slab)
            .tags(["wood", "slab"])
            .textures(BlockTextures::all("humancraft:block/oak_planks/top")),
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
        oak_sapling,
        oak_planks,
        crafting_table,
        sand,
        sandstone,
        bedrock,
        glass,
        chest,
        furnace,
        wooden_stairs,
        wooden_slab,
    })
}
