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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDefinition {
    pub key: String,
    pub display_name: String,
    pub max_stack_size: u16,
    pub place_block: Option<String>,
    pub tags: Vec<String>,
}

impl ItemDefinition {
    pub fn new(key: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            display_name: display_name.into(),
            max_stack_size: 64,
            place_block: None,
            tags: Vec::new(),
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

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }
}

impl Definition for ItemDefinition {
    fn key(&self) -> &str {
        &self.key
    }
}

pub type ItemRegistry = Registry<ItemId, ItemDefinition>;
