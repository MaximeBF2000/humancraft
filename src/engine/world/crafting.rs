//! Data-driven crafting recipes.

use crate::engine::registry::{Definition, Registry};

use super::{Inventory, ItemId, ItemRegistry, ItemStack};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RecipeId(usize);

impl From<usize> for RecipeId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<RecipeId> for usize {
    fn from(value: RecipeId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftingRecipeDefinition {
    pub key: String,
    pub kind: CraftingRecipeKind,
    pub result: CraftingRecipeResult,
}

impl CraftingRecipeDefinition {
    pub fn shapeless(
        key: impl Into<String>,
        ingredients: impl IntoIterator<Item = impl Into<String>>,
        result_item: impl Into<String>,
        result_count: u16,
    ) -> Self {
        Self {
            key: key.into(),
            kind: CraftingRecipeKind::Shapeless {
                ingredients: ingredients.into_iter().map(Into::into).collect(),
            },
            result: CraftingRecipeResult::new(result_item, result_count),
        }
    }

    pub fn shaped(
        key: impl Into<String>,
        width: usize,
        height: usize,
        pattern: impl IntoIterator<Item = Option<impl Into<String>>>,
        result_item: impl Into<String>,
        result_count: u16,
    ) -> Self {
        Self {
            key: key.into(),
            kind: CraftingRecipeKind::Shaped {
                width,
                height,
                pattern: pattern
                    .into_iter()
                    .map(|item| item.map(Into::into))
                    .collect(),
            },
            result: CraftingRecipeResult::new(result_item, result_count),
        }
    }
}

impl Definition for CraftingRecipeDefinition {
    fn key(&self) -> &str {
        &self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CraftingRecipeKind {
    Shapeless {
        ingredients: Vec<String>,
    },
    Shaped {
        width: usize,
        height: usize,
        pattern: Vec<Option<String>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftingRecipeResult {
    pub item: String,
    pub count: u16,
}

impl CraftingRecipeResult {
    pub fn new(item: impl Into<String>, count: u16) -> Self {
        Self {
            item: item.into(),
            count,
        }
    }
}

pub type CraftingRecipeRegistry = Registry<RecipeId, CraftingRecipeDefinition>;

pub fn crafting_result(
    recipes: &CraftingRecipeRegistry,
    items: &ItemRegistry,
    grid: &Inventory,
    grid_width: usize,
) -> Option<ItemStack> {
    recipes
        .iter()
        .find_map(|(_, recipe)| match_recipe(recipe, items, grid, grid_width))
}

fn match_recipe(
    recipe: &CraftingRecipeDefinition,
    items: &ItemRegistry,
    grid: &Inventory,
    grid_width: usize,
) -> Option<ItemStack> {
    let result_item = items.id_for_key(&recipe.result.item)?;
    match &recipe.kind {
        CraftingRecipeKind::Shapeless { ingredients } => {
            match_shapeless(ingredients, items, grid)?;
        }
        CraftingRecipeKind::Shaped {
            width,
            height,
            pattern,
        } => {
            match_shaped(*width, *height, pattern, items, grid, grid_width)?;
        }
    }
    Some(ItemStack::new(result_item, recipe.result.count))
}

fn match_shapeless(ingredients: &[String], items: &ItemRegistry, grid: &Inventory) -> Option<()> {
    let mut expected: Vec<ItemId> = ingredients
        .iter()
        .map(|key| items.id_for_key(key))
        .collect::<Option<_>>()?;
    let mut provided: Vec<ItemId> = grid
        .slots()
        .iter()
        .filter_map(|slot| slot.map(|stack| stack.item))
        .collect();
    if expected.len() != provided.len() {
        return None;
    }
    expected.sort_by_key(|item| item.raw());
    provided.sort_by_key(|item| item.raw());
    (expected == provided).then_some(())
}

fn match_shaped(
    width: usize,
    height: usize,
    pattern: &[Option<String>],
    items: &ItemRegistry,
    grid: &Inventory,
    grid_width: usize,
) -> Option<()> {
    if width == 0 || height == 0 || pattern.len() != width * height {
        return None;
    }
    let grid_height = grid.slot_count() / grid_width;
    if width > grid_width || height > grid_height {
        return None;
    }
    let resolved_pattern: Vec<Option<ItemId>> = pattern
        .iter()
        .map(|slot| match slot {
            Some(key) => items.id_for_key(key).map(Some),
            None => Some(None),
        })
        .collect::<Option<_>>()?;

    for offset_y in 0..=grid_height - height {
        for offset_x in 0..=grid_width - width {
            if shaped_pattern_matches(
                &resolved_pattern,
                width,
                height,
                grid,
                grid_width,
                offset_x,
                offset_y,
            ) {
                return Some(());
            }
        }
    }
    None
}

fn shaped_pattern_matches(
    pattern: &[Option<ItemId>],
    pattern_width: usize,
    pattern_height: usize,
    grid: &Inventory,
    grid_width: usize,
    offset_x: usize,
    offset_y: usize,
) -> bool {
    for slot in 0..grid.slot_count() {
        let x = slot % grid_width;
        let y = slot / grid_width;
        let expected = if x >= offset_x
            && x < offset_x + pattern_width
            && y >= offset_y
            && y < offset_y + pattern_height
        {
            pattern[(y - offset_y) * pattern_width + (x - offset_x)]
        } else {
            None
        };
        if grid.slot(slot).map(|stack| stack.item) != expected {
            return false;
        }
    }
    true
}

pub fn consume_crafting_ingredients(grid: &mut Inventory) {
    for index in 0..grid.slot_count() {
        let Some(mut stack) = grid.slot(index) else {
            continue;
        };
        stack.count -= 1;
        grid.set_slot(index, if stack.count == 0 { None } else { Some(stack) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::world::ItemDefinition;

    fn test_items() -> ItemRegistry {
        let mut items = ItemRegistry::new();
        items
            .register(ItemDefinition::new("test:log", "Log"))
            .unwrap();
        items
            .register(ItemDefinition::new("test:planks", "Planks"))
            .unwrap();
        items
            .register(ItemDefinition::new("test:stick", "Stick"))
            .unwrap();
        items
    }

    #[test]
    fn shapeless_recipe_matches_ingredient_anywhere_in_grid() {
        let items = test_items();
        let log = items.id_for_key("test:log").unwrap();
        let planks = items.id_for_key("test:planks").unwrap();
        let mut recipes = CraftingRecipeRegistry::new();
        recipes
            .register(CraftingRecipeDefinition::shapeless(
                "test:planks",
                ["test:log"],
                "test:planks",
                4,
            ))
            .unwrap();
        let mut grid = Inventory::new(4, 0);
        grid.set_slot(3, Some(ItemStack::new(log, 1)));

        assert_eq!(
            crafting_result(&recipes, &items, &grid, 2),
            Some(ItemStack::new(planks, 4))
        );
    }

    #[test]
    fn shaped_recipe_matches_exact_pattern_inside_larger_grid() {
        let items = test_items();
        let planks = items.id_for_key("test:planks").unwrap();
        let stick = items.id_for_key("test:stick").unwrap();
        let mut recipes = CraftingRecipeRegistry::new();
        recipes
            .register(CraftingRecipeDefinition::shaped(
                "test:sticks",
                1,
                2,
                [Some("test:planks"), Some("test:planks")],
                "test:stick",
                4,
            ))
            .unwrap();
        let mut grid = Inventory::new(9, 0);
        grid.set_slot(1, Some(ItemStack::new(planks, 1)));
        grid.set_slot(4, Some(ItemStack::new(planks, 1)));

        assert_eq!(
            crafting_result(&recipes, &items, &grid, 3),
            Some(ItemStack::new(stick, 4))
        );

        grid.set_slot(8, Some(ItemStack::new(planks, 1)));
        assert_eq!(crafting_result(&recipes, &items, &grid, 3), None);
    }
}
