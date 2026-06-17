//! Item definitions and IDs.
//!
//! Purpose:
//! Represent inventory content as data. Blocks can have matching item
//! definitions, but the item system remains generic for tools, food, and future
//! content categories.

use crate::engine::registry::{Definition, Registry};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ItemId(usize);

impl ItemId {
    pub const fn raw(self) -> usize {
        self.0
    }
}

impl From<usize> for ItemId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<ItemId> for usize {
    fn from(value: ItemId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemDefinition {
    pub key: String,
    pub display_name: String,
    pub max_stack_size: u16,
    pub place_block: Option<String>,
    pub texture: String,
    pub tags: Vec<String>,
    pub tool: Option<ToolDefinition>,
}

impl ItemDefinition {
    pub fn new(key: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            display_name: display_name.into(),
            max_stack_size: 64,
            place_block: None,
            texture: "humancraft:missing".to_string(),
            tags: Vec::new(),
            tool: None,
        }
    }

    pub fn max_stack_size(mut self, max_stack_size: u16) -> Self {
        self.max_stack_size = max_stack_size;
        self
    }

    pub fn place_block(mut self, block_key: impl Into<String>) -> Self {
        self.place_block = Some(block_key.into());
        self
    }

    pub fn texture(mut self, texture: impl Into<String>) -> Self {
        self.texture = texture.into();
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn tool(mut self, tool: ToolDefinition) -> Self {
        self.tool = Some(tool);
        self
    }
}

impl Definition for ItemDefinition {
    fn key(&self) -> &str {
        &self.key
    }
}

pub type ItemRegistry = Registry<ItemId, ItemDefinition>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ToolKind {
    Pickaxe,
    Shovel,
    Axe,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ToolMaterial {
    Wood,
    Stone,
    Iron,
    Diamond,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ToolDefinition {
    pub kind: ToolKind,
    pub material: ToolMaterial,
    pub harvest_level: u8,
    pub speed_multiplier: f32,
}

impl ToolDefinition {
    pub const fn new(kind: ToolKind, material: ToolMaterial) -> Self {
        Self {
            kind,
            material,
            harvest_level: material.harvest_level(),
            speed_multiplier: material.speed_multiplier(),
        }
    }
}

impl ToolMaterial {
    pub const fn harvest_level(self) -> u8 {
        match self {
            Self::Wood => 1,
            Self::Stone => 2,
            Self::Iron => 3,
            Self::Diamond => 4,
        }
    }

    pub const fn speed_multiplier(self) -> f32 {
        match self {
            Self::Wood => 2.0,
            Self::Stone => 4.0,
            Self::Iron => 6.0,
            Self::Diamond => 8.0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u16,
}

impl ItemStack {
    pub fn new(item: ItemId, count: u16) -> Self {
        Self { item, count }
    }

    pub fn is_empty(self) -> bool {
        self.count == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    slots: Vec<Option<ItemStack>>,
    hotbar_slots: usize,
}

impl Inventory {
    pub fn new(slot_count: usize, hotbar_slots: usize) -> Self {
        assert!(hotbar_slots <= slot_count);
        Self {
            slots: vec![None; slot_count],
            hotbar_slots,
        }
    }

    pub fn player() -> Self {
        Self::new(36, 9)
    }

    pub fn from_slots(slots: Vec<Option<ItemStack>>, hotbar_slots: usize) -> Self {
        assert!(hotbar_slots <= slots.len());
        Self {
            slots,
            hotbar_slots,
        }
    }

    pub fn slots(&self) -> &[Option<ItemStack>] {
        &self.slots
    }

    pub fn slot(&self, index: usize) -> Option<ItemStack> {
        self.slots.get(index).copied().flatten()
    }

    pub fn set_slot(&mut self, index: usize, stack: Option<ItemStack>) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = stack;
        }
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn hotbar_slots(&self) -> &[Option<ItemStack>] {
        &self.slots[..self.hotbar_slots]
    }

    pub fn add_stack(&mut self, mut stack: ItemStack, items: &ItemRegistry) -> Option<ItemStack> {
        if stack.is_empty() {
            return None;
        }

        let max_stack_size = items
            .get(stack.item)
            .map(|definition| definition.max_stack_size)
            .unwrap_or(64);

        for slot in &mut self.slots {
            let Some(existing) = slot else {
                continue;
            };
            if existing.item != stack.item || existing.count >= max_stack_size {
                continue;
            }

            let space = max_stack_size - existing.count;
            let moved = stack.count.min(space);
            existing.count += moved;
            stack.count -= moved;
            if stack.count == 0 {
                return None;
            }
        }

        for slot in &mut self.slots {
            if slot.is_some() {
                continue;
            }
            let moved = stack.count.min(max_stack_size);
            *slot = Some(ItemStack::new(stack.item, moved));
            stack.count -= moved;
            if stack.count == 0 {
                return None;
            }
        }

        Some(stack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_items() -> ItemRegistry {
        let mut items = ItemRegistry::new();
        items
            .register(ItemDefinition::new("test:stone", "Stone"))
            .unwrap();
        items
            .register(ItemDefinition::new("test:tool", "Tool").max_stack_size(1))
            .unwrap();
        items
    }

    #[test]
    fn inventory_merges_stacks_before_using_empty_slots() {
        let items = test_items();
        let stone = items.id_for_key("test:stone").unwrap();
        let mut inventory = Inventory::new(2, 1);

        assert_eq!(inventory.add_stack(ItemStack::new(stone, 63), &items), None);
        assert_eq!(inventory.add_stack(ItemStack::new(stone, 2), &items), None);

        assert_eq!(inventory.slots()[0], Some(ItemStack::new(stone, 64)));
        assert_eq!(inventory.slots()[1], Some(ItemStack::new(stone, 1)));
    }

    #[test]
    fn inventory_returns_overflow_when_full() {
        let items = test_items();
        let tool = items.id_for_key("test:tool").unwrap();
        let mut inventory = Inventory::new(1, 1);

        assert_eq!(
            inventory.add_stack(ItemStack::new(tool, 2), &items),
            Some(ItemStack::new(tool, 1))
        );
        assert_eq!(inventory.slots()[0], Some(ItemStack::new(tool, 1)));
    }
}
