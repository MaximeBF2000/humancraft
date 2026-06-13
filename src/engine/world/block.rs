//! Block definitions and IDs.
//!
//! Purpose:
//! Describe block properties without embedding behavior in block values.
//!
//! Extension points:
//! New blocks are registered as definitions. Systems such as mining, meshing,
//! lighting, and drops consume properties from this registry.

use crate::engine::registry::{Definition, Registry};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

impl BlockId {
    pub const fn raw(self) -> usize {
        self.0
    }
}

impl From<usize> for BlockId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<BlockId> for usize {
    fn from(value: BlockId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockDefinition {
    pub key: String,
    pub display_name: String,
    pub hardness: f32,
    pub transparent: bool,
    pub solid: bool,
    pub drops: Vec<String>,
    pub tags: Vec<String>,
    pub textures: BlockTextures,
}

impl BlockDefinition {
    pub fn new(key: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            display_name: display_name.into(),
            hardness: 1.0,
            transparent: false,
            solid: true,
            drops: Vec::new(),
            tags: Vec::new(),
            textures: BlockTextures::missing(),
        }
    }

    pub fn hardness(mut self, hardness: f32) -> Self {
        self.hardness = hardness;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub fn solid(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }

    pub fn drops(mut self, drops: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.drops = drops.into_iter().map(Into::into).collect();
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn textures(mut self, textures: BlockTextures) -> Self {
        self.textures = textures;
        self
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|candidate| candidate == tag)
    }
}

impl Definition for BlockDefinition {
    fn key(&self) -> &str {
        &self.key
    }
}

pub type BlockRegistry = Registry<BlockId, BlockDefinition>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockTextures {
    pub top: String,
    pub bottom: String,
    pub north: String,
    pub south: String,
    pub east: String,
    pub west: String,
}

impl BlockTextures {
    pub fn missing() -> Self {
        Self::all("humancraft:missing")
    }

    pub fn all(texture: impl Into<String>) -> Self {
        let texture = texture.into();
        Self {
            top: texture.clone(),
            bottom: texture.clone(),
            north: texture.clone(),
            south: texture.clone(),
            east: texture.clone(),
            west: texture,
        }
    }

    pub fn top_bottom_sides(
        top: impl Into<String>,
        bottom: impl Into<String>,
        sides: impl Into<String>,
    ) -> Self {
        let sides = sides.into();
        Self {
            top: top.into(),
            bottom: bottom.into(),
            north: sides.clone(),
            south: sides.clone(),
            east: sides.clone(),
            west: sides,
        }
    }
}
