//! HumanCraft recipe registration.

use crate::engine::registry::RegistryError;
use crate::engine::world::{
    CraftingRecipeDefinition, CraftingRecipeRegistry, SmeltingRecipeDefinition,
    SmeltingRecipeRegistry,
};

pub fn register_recipes(recipes: &mut CraftingRecipeRegistry) -> Result<(), RegistryError> {
    recipes.register(CraftingRecipeDefinition::shapeless(
        "humancraft:oak_planks_from_oak_log",
        ["humancraft:oak_log"],
        "humancraft:oak_planks",
        4,
    ))?;
    recipes
        .register(CraftingRecipeDefinition::shaped(
            "humancraft:crafting_table_from_oak_planks",
            2,
            2,
            [
                Some("humancraft:oak_planks"),
                Some("humancraft:oak_planks"),
                Some("humancraft:oak_planks"),
                Some("humancraft:oak_planks"),
            ],
            "humancraft:crafting_table",
            1,
        ))
        .map(|_| ())?;
    recipes.register(CraftingRecipeDefinition::shaped(
        "humancraft:sticks_from_oak_planks",
        1,
        2,
        [Some("humancraft:oak_planks"), Some("humancraft:oak_planks")],
        "humancraft:stick",
        4,
    ))?;
    recipes.register(CraftingRecipeDefinition::shaped(
        "humancraft:chest_from_oak_planks",
        3,
        3,
        [
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            None,
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
        ],
        "humancraft:chest",
        1,
    ))?;
    recipes.register(CraftingRecipeDefinition::shaped(
        "humancraft:furnace_from_cobblestone",
        3,
        3,
        [
            Some("humancraft:cobblestone"),
            Some("humancraft:cobblestone"),
            Some("humancraft:cobblestone"),
            Some("humancraft:cobblestone"),
            None,
            Some("humancraft:cobblestone"),
            Some("humancraft:cobblestone"),
            Some("humancraft:cobblestone"),
            Some("humancraft:cobblestone"),
        ],
        "humancraft:furnace",
        1,
    ))?;
    recipes.register(CraftingRecipeDefinition::shaped(
        "humancraft:wooden_stairs_from_oak_planks",
        3,
        3,
        [
            Some("humancraft:oak_planks"),
            None,
            None,
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            None,
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
        ],
        "humancraft:wooden_stairs",
        4,
    ))?;
    recipes.register(CraftingRecipeDefinition::shaped(
        "humancraft:wooden_stairs_from_oak_planks_mirrored",
        3,
        3,
        [
            None,
            None,
            Some("humancraft:oak_planks"),
            None,
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
        ],
        "humancraft:wooden_stairs",
        4,
    ))?;
    recipes.register(CraftingRecipeDefinition::shaped(
        "humancraft:wooden_slab_from_oak_planks",
        3,
        1,
        [
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
            Some("humancraft:oak_planks"),
        ],
        "humancraft:wooden_slab",
        6,
    ))?;

    register_tool_recipes(recipes, "wood", "humancraft:oak_planks", "wooden")?;
    register_tool_recipes(recipes, "stone", "humancraft:cobblestone", "stone")?;
    register_tool_recipes(recipes, "iron", "humancraft:iron_ingot", "iron")?;
    register_tool_recipes(recipes, "diamond", "humancraft:diamond", "diamond")
}

pub fn register_smelting_recipes(
    recipes: &mut SmeltingRecipeRegistry,
) -> Result<(), RegistryError> {
    recipes.register(SmeltingRecipeDefinition::new(
        "humancraft:smelt_sand_to_glass",
        "humancraft:sand",
        "humancraft:glass",
        1,
        200,
    ))?;
    recipes
        .register(SmeltingRecipeDefinition::new(
            "humancraft:smelt_raw_iron_to_iron_ingot",
            "humancraft:raw_iron",
            "humancraft:iron_ingot",
            1,
            200,
        ))
        .map(|_| ())
}

fn register_tool_recipes(
    recipes: &mut CraftingRecipeRegistry,
    material_key: &str,
    ingredient: &str,
    recipe_name_material: &str,
) -> Result<(), RegistryError> {
    recipes.register(CraftingRecipeDefinition::shaped(
        format!("humancraft:{recipe_name_material}_pickaxe"),
        3,
        3,
        [
            Some(ingredient),
            Some(ingredient),
            Some(ingredient),
            None,
            Some("humancraft:stick"),
            None,
            None,
            Some("humancraft:stick"),
            None,
        ],
        format!("humancraft:{material_key}_pickaxe"),
        1,
    ))?;
    recipes.register(CraftingRecipeDefinition::shaped(
        format!("humancraft:{recipe_name_material}_shovel"),
        1,
        3,
        [
            Some(ingredient),
            Some("humancraft:stick"),
            Some("humancraft:stick"),
        ],
        format!("humancraft:{material_key}_shovel"),
        1,
    ))?;
    recipes.register(CraftingRecipeDefinition::shaped(
        format!("humancraft:{recipe_name_material}_axe"),
        2,
        3,
        [
            Some(ingredient),
            Some(ingredient),
            Some(ingredient),
            Some("humancraft:stick"),
            None,
            Some("humancraft:stick"),
        ],
        format!("humancraft:{material_key}_axe"),
        1,
    ))?;
    recipes
        .register(CraftingRecipeDefinition::shaped(
            format!("humancraft:{recipe_name_material}_axe_mirrored"),
            2,
            3,
            [
                Some(ingredient),
                Some(ingredient),
                Some("humancraft:stick"),
                Some(ingredient),
                Some("humancraft:stick"),
                None,
            ],
            format!("humancraft:{material_key}_axe"),
            1,
        ))
        .map(|_| ())
}
