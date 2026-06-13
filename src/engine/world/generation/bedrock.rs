//! Bedrock generation stage.
//!
//! Purpose:
//! Guarantee an unbroken bottom layer independent of terrain, ore, cave, or
//! biome configuration.

use crate::engine::world::generation::{GenerationContext, GenerationStage};
use crate::engine::world::{BlockId, BlockPosition, CHUNK_SIZE, Chunk};

#[derive(Debug, Copy, Clone)]
pub struct BedrockStage {
    bedrock: BlockId,
}

impl BedrockStage {
    pub fn new(bedrock: BlockId) -> Self {
        Self { bedrock }
    }
}

impl GenerationStage for BedrockStage {
    fn name(&self) -> &str {
        "engine:bedrock"
    }

    fn generate(&self, chunk: &mut Chunk, _context: &GenerationContext) {
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                chunk
                    .set_block(BlockPosition { x, y: 0, z }, self.bedrock)
                    .expect("bedrock generated positions stay inside chunk bounds");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::world::{ChunkPosition, generation::GenerationContext};

    #[test]
    fn bedrock_stage_replaces_entire_bottom_layer() {
        let air = BlockId::from(0);
        let bedrock = BlockId::from(1);
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, air);

        BedrockStage::new(bedrock).generate(&mut chunk, &GenerationContext { seed: 1, air });

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                assert_eq!(chunk.block(BlockPosition { x, y: 0, z }), Some(bedrock));
            }
        }
    }
}
