//! World data and world-facing engine systems.

pub mod block;
pub mod chunk;
pub mod generation;
pub mod item;
pub mod save;

pub use block::{BlockDefinition, BlockId, BlockRegistry, BlockTextures};
pub use chunk::{BlockPosition, CHUNK_HEIGHT, CHUNK_SIZE, CHUNK_VOLUME, Chunk, ChunkPosition};
pub use item::{ItemDefinition, ItemId, ItemRegistry};
