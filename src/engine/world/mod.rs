//! World data and world-facing engine systems.

pub mod block;
pub mod chunk;
pub mod crafting;
pub mod generation;
pub mod item;
pub mod loot;
pub mod save;

pub use block::{BlockDefinition, BlockHarvestRequirement, BlockId, BlockRegistry, BlockTextures};
pub use chunk::{BlockPosition, CHUNK_HEIGHT, CHUNK_SIZE, CHUNK_VOLUME, Chunk, ChunkPosition};
pub use crafting::{
    CraftingRecipeDefinition, CraftingRecipeRegistry, consume_crafting_ingredients, crafting_result,
};
pub use item::{
    Inventory, ItemDefinition, ItemId, ItemRegistry, ItemStack, ToolDefinition, ToolKind,
    ToolMaterial,
};
pub use loot::LootEntity;
