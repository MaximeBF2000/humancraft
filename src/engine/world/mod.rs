//! World data and world-facing engine systems.

pub mod block;
pub mod chunk;
pub mod crafting;
pub mod generation;
pub mod item;
pub mod loot;
pub mod save;
pub mod smelting;

pub use block::{
    Axis, BlockAabb, BlockBehavior, BlockDefinition, BlockHarvestRequirement, BlockId,
    BlockProperties, BlockRegistry, BlockShape, BlockState, BlockTextures, ChanceDrop,
    GrassSpreadBehavior, HorizontalDirection, LeafDecayBehavior, PlacementRuleKind,
    SaplingGrowthBehavior, SlabOrientation, StairHalf, block_state_aabbs,
};
pub use chunk::{BlockPosition, CHUNK_HEIGHT, CHUNK_SIZE, CHUNK_VOLUME, Chunk, ChunkPosition};
pub use crafting::{
    CraftingRecipeDefinition, CraftingRecipeRegistry, consume_crafting_ingredients, crafting_result,
};
pub use item::{
    Inventory, ItemDefinition, ItemId, ItemRegistry, ItemStack, ItemStackMetadata, ToolDefinition,
    ToolKind, ToolMaterial,
};
pub use loot::LootEntity;
pub use smelting::{SmeltingRecipeDefinition, SmeltingRecipeRegistry, smelting_result};
