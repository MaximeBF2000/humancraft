//! HumanCraft recipe registration.

use crate::engine::registry::RegistryError;
use crate::engine::world::{CraftingRecipeDefinition, CraftingRecipeRegistry};

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
        .map(|_| ())
}
