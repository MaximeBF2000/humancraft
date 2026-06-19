//! Furnace smelting recipe definitions.

use crate::engine::registry::{Definition, Registry};
use crate::engine::world::{ItemId, ItemRegistry, ItemStack};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SmeltingRecipeId(usize);

impl From<usize> for SmeltingRecipeId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<SmeltingRecipeId> for usize {
    fn from(value: SmeltingRecipeId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmeltingRecipeDefinition {
    pub key: String,
    pub input: String,
    pub output: String,
    pub output_count: u16,
    pub cook_ticks: u32,
}

impl SmeltingRecipeDefinition {
    pub fn new(
        key: impl Into<String>,
        input: impl Into<String>,
        output: impl Into<String>,
        output_count: u16,
        cook_ticks: u32,
    ) -> Self {
        Self {
            key: key.into(),
            input: input.into(),
            output: output.into(),
            output_count,
            cook_ticks,
        }
    }
}

impl Definition for SmeltingRecipeDefinition {
    fn key(&self) -> &str {
        &self.key
    }
}

pub type SmeltingRecipeRegistry = Registry<SmeltingRecipeId, SmeltingRecipeDefinition>;

pub fn smelting_result(
    recipes: &SmeltingRecipeRegistry,
    items: &ItemRegistry,
    input: ItemId,
) -> Option<(ItemStack, u32)> {
    let input_key = &items.get(input)?.key;
    for (_, recipe) in recipes.iter() {
        if &recipe.input != input_key {
            continue;
        }
        let output = items.id_for_key(&recipe.output)?;
        return Some((
            ItemStack::new(output, recipe.output_count),
            recipe.cook_ticks,
        ));
    }
    None
}
