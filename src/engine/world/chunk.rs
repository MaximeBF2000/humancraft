//! Chunk storage.
//!
//! Purpose:
//! Own block IDs for one fixed-size chunk. Meshing, generation, persistence, and
//! gameplay systems operate on chunks through this API.
//!
//! Known limitations:
//! Light values and metadata are planned but not stored yet.

use crate::engine::world::{BlockId, BlockState};

pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_HEIGHT: usize = 256;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_HEIGHT;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ChunkPosition {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockPosition {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkError {
    OutOfBounds(BlockPosition),
    InvalidBlockCount { expected: usize, actual: usize },
}

#[derive(Debug, Clone)]
pub struct Chunk {
    position: ChunkPosition,
    blocks: Vec<BlockState>,
}

impl Chunk {
    pub fn filled(position: ChunkPosition, block: BlockId) -> Self {
        Self {
            position,
            blocks: vec![BlockState::new(block); CHUNK_VOLUME],
        }
    }

    pub fn from_blocks(position: ChunkPosition, blocks: Vec<BlockId>) -> Result<Self, ChunkError> {
        Self::from_states(position, blocks.into_iter().map(BlockState::new).collect())
    }

    pub fn from_states(
        position: ChunkPosition,
        blocks: Vec<BlockState>,
    ) -> Result<Self, ChunkError> {
        if blocks.len() != CHUNK_VOLUME {
            return Err(ChunkError::InvalidBlockCount {
                expected: CHUNK_VOLUME,
                actual: blocks.len(),
            });
        }

        Ok(Self { position, blocks })
    }

    pub fn position(&self) -> ChunkPosition {
        self.position
    }

    pub fn block(&self, position: BlockPosition) -> Option<BlockId> {
        self.block_state(position).map(|state| state.block)
    }

    pub fn block_state(&self, position: BlockPosition) -> Option<BlockState> {
        Self::index(position).map(|index| self.blocks[index])
    }

    pub fn set_block(&mut self, position: BlockPosition, block: BlockId) -> Result<(), ChunkError> {
        self.set_block_state(position, BlockState::new(block))
    }

    pub fn set_block_state(
        &mut self,
        position: BlockPosition,
        block: BlockState,
    ) -> Result<(), ChunkError> {
        let index = Self::index(position).ok_or(ChunkError::OutOfBounds(position))?;
        self.blocks[index] = block;
        Ok(())
    }

    pub fn blocks(&self) -> Vec<BlockId> {
        self.blocks.iter().map(|state| state.block).collect()
    }

    pub fn block_states(&self) -> &[BlockState] {
        &self.blocks
    }

    fn index(position: BlockPosition) -> Option<usize> {
        if position.x >= CHUNK_SIZE || position.y >= CHUNK_HEIGHT || position.z >= CHUNK_SIZE {
            return None;
        }

        Some(position.y * CHUNK_SIZE * CHUNK_SIZE + position.z * CHUNK_SIZE + position.x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_blocks_by_position() {
        let air = BlockId::from(0);
        let stone = BlockId::from(1);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, air);

        let position = BlockPosition { x: 3, y: 7, z: 11 };
        chunk.set_block(position, stone).unwrap();

        assert_eq!(chunk.block(position), Some(stone));
    }

    #[test]
    fn rejects_out_of_bounds_positions() {
        let air = BlockId::from(0);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, air);

        let result = chunk.set_block(
            BlockPosition {
                x: CHUNK_SIZE,
                y: 0,
                z: 0,
            },
            air,
        );

        assert!(matches!(result, Err(ChunkError::OutOfBounds(_))));
    }
}
