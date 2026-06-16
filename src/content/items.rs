//! HumanCraft item registration.

use crate::content::BlockIds;
use crate::engine::registry::RegistryError;
use crate::engine::world::{ItemDefinition, ItemRegistry};

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
            .tags(["fuel"]),
    )?;
    items.register(
        ItemDefinition::new("humancraft:raw_iron", "Raw Iron")
            .texture(item_texture_key("humancraft:raw_iron"))
            .tags(["ore_drop"]),
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

    Ok(())
}

fn register_block_item(
    items: &mut ItemRegistry,
    key: &str,
    display_name: &str,
    block_key: &str,
) -> Result<(), RegistryError> {
    items
        .register(
            ItemDefinition::new(key, display_name)
                .place_block(block_key)
                .texture(item_texture_key(key)),
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
