//! HumanCraft item registration.

use crate::content::BlockIds;
use crate::engine::registry::RegistryError;
use crate::engine::world::{ItemDefinition, ItemRegistry, ToolDefinition, ToolKind, ToolMaterial};

pub fn register_items(items: &mut ItemRegistry, _blocks: BlockIds) -> Result<(), RegistryError> {
    register_block_item(items, "humancraft:dirt", "Dirt", "humancraft:dirt")?;
    register_block_item(items, "humancraft:grass", "Grass Block", "humancraft:grass")?;
    register_block_item(items, "humancraft:stone", "Stone", "humancraft:stone")?;
    register_block_item(
        items,
        "humancraft:cobblestone",
        "Cobblestone",
        "humancraft:cobblestone",
    )?;
    register_block_item(
        items,
        "humancraft:coal_ore",
        "Coal Ore",
        "humancraft:coal_ore",
    )?;
    register_block_item(
        items,
        "humancraft:iron_ore",
        "Iron Ore",
        "humancraft:iron_ore",
    )?;
    register_block_item(
        items,
        "humancraft:gold_ore",
        "Gold Ore",
        "humancraft:gold_ore",
    )?;
    register_block_item(
        items,
        "humancraft:diamond_ore",
        "Diamond Ore",
        "humancraft:diamond_ore",
    )?;
    items.register(
        ItemDefinition::new("humancraft:coal", "Coal")
            .texture(item_texture_key("humancraft:coal"))
            .fuel_ticks(1600)
            .tags(["fuel"]),
    )?;
    items.register(
        ItemDefinition::new("humancraft:raw_iron", "Raw Iron")
            .texture(item_texture_key("humancraft:raw_iron"))
            .tags(["ore_drop"]),
    )?;
    items.register(
        ItemDefinition::new("humancraft:iron_ingot", "Iron Ingot")
            .texture(item_texture_key("humancraft:iron_ingot"))
            .tags(["ingot"]),
    )?;
    items.register(
        ItemDefinition::new("humancraft:raw_gold", "Raw Gold")
            .texture(item_texture_key("humancraft:raw_gold"))
            .tags(["ore_drop"]),
    )?;
    items.register(
        ItemDefinition::new("humancraft:diamond", "Diamond")
            .texture(item_texture_key("humancraft:diamond"))
            .tags(["gem"]),
    )?;
    items.register(
        ItemDefinition::new("humancraft:stick", "Stick")
            .texture(item_texture_key("humancraft:stick"))
            .fuel_ticks(100)
            .tags(["crafting_material"]),
    )?;
    register_tool_family(items, ToolMaterial::Wood, "wood", "Wooden")?;
    register_tool_family(items, ToolMaterial::Stone, "stone", "Stone")?;
    register_tool_family(items, ToolMaterial::Iron, "iron", "Iron")?;
    register_tool_family(items, ToolMaterial::Diamond, "diamond", "Diamond")?;
    register_block_item(items, "humancraft:oak_log", "Oak Log", "humancraft:oak_log")?;
    register_block_item(
        items,
        "humancraft:oak_leaves",
        "Oak Leaves",
        "humancraft:oak_leaves",
    )?;
    register_block_item(
        items,
        "humancraft:oak_planks",
        "Oak Planks",
        "humancraft:oak_planks",
    )?;
    register_block_item(
        items,
        "humancraft:crafting_table",
        "Crafting Table",
        "humancraft:crafting_table",
    )?;
    items.register(
        ItemDefinition::new("humancraft:oak_sapling", "Oak Sapling")
            .place_block("humancraft:oak_sapling")
            .texture(item_texture_key("humancraft:oak_sapling"))
            .tags(["sapling"]),
    )?;
    register_block_item(items, "humancraft:sand", "Sand", "humancraft:sand")?;
    register_block_item(
        items,
        "humancraft:sandstone",
        "Sandstone",
        "humancraft:sandstone",
    )?;
    register_block_item(items, "humancraft:bedrock", "Bedrock", "humancraft:bedrock")?;
    register_block_item(items, "humancraft:glass", "Glass", "humancraft:glass")?;
    items
        .register(
            ItemDefinition::new("humancraft:chest", "Chest")
                .max_stack_size(1)
                .place_block("humancraft:chest")
                .texture(item_texture_key("humancraft:chest")),
        )
        .map(|_| ())?;
    register_block_item(items, "humancraft:furnace", "Furnace", "humancraft:furnace")?;
    register_block_item(
        items,
        "humancraft:wooden_stairs",
        "Wooden Stairs",
        "humancraft:wooden_stairs",
    )?;
    register_block_item(
        items,
        "humancraft:wooden_slab",
        "Wooden Slab",
        "humancraft:wooden_slab",
    )?;

    Ok(())
}

fn register_block_item(
    items: &mut ItemRegistry,
    key: &str,
    display_name: &str,
    block_key: &str,
) -> Result<(), RegistryError> {
    let mut definition = ItemDefinition::new(key, display_name)
        .place_block(block_key)
        .texture(item_texture_key(key));
    if let Some(ticks) = block_item_fuel_ticks(key) {
        definition = definition.fuel_ticks(ticks);
    }
    items.register(definition).map(|_| ())
}

fn block_item_fuel_ticks(key: &str) -> Option<u32> {
    match key {
        "humancraft:oak_log" => Some(300),
        "humancraft:oak_planks" => Some(300),
        "humancraft:wooden_stairs" => Some(300),
        "humancraft:wooden_slab" => Some(150),
        _ => None,
    }
}

fn register_tool_family(
    items: &mut ItemRegistry,
    material: ToolMaterial,
    material_key: &str,
    material_name: &str,
) -> Result<(), RegistryError> {
    register_tool(
        items,
        material,
        material_key,
        material_name,
        ToolKind::Pickaxe,
        "pickaxe",
        "Pickaxe",
    )?;
    register_tool(
        items,
        material,
        material_key,
        material_name,
        ToolKind::Shovel,
        "shovel",
        "Shovel",
    )?;
    register_tool(
        items,
        material,
        material_key,
        material_name,
        ToolKind::Axe,
        "axe",
        "Axe",
    )
}

fn register_tool(
    items: &mut ItemRegistry,
    material: ToolMaterial,
    material_key: &str,
    material_name: &str,
    kind: ToolKind,
    kind_key: &str,
    kind_name: &str,
) -> Result<(), RegistryError> {
    let key = format!("humancraft:{material_key}_{kind_key}");
    items
        .register(
            ItemDefinition::new(&key, format!("{material_name} {kind_name}"))
                .max_stack_size(1)
                .texture(item_texture_key(&key))
                .tool(ToolDefinition::new(kind, material))
                .tags(["tool"]),
        )
        .map(|_| ())
}

fn item_texture_key(item_key: &str) -> String {
    let name = item_key
        .strip_prefix("humancraft:")
        .unwrap_or(item_key)
        .replace(':', "_");
    format!("humancraft:item/{name}")
}
