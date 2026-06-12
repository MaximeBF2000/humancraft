//! Terrain generation stage.
//!
//! Purpose:
//! Fill a chunk column from bedrock-ish stone through dirt and grass using a
//! generic height function. This stage owns terrain shape, not specific biome
//! rules.

use crate::engine::world::generation::{
    GenerationContext, GenerationStage, value_noise_2d, world_x, world_z,
};
use crate::engine::world::{BlockId, BlockPosition, CHUNK_HEIGHT, CHUNK_SIZE, Chunk};

#[derive(Debug, Clone)]
pub struct TerrainStage {
    pub grass: BlockId,
    pub dirt: BlockId,
    pub stone: BlockId,
    pub base_height: usize,
    pub height_variation: usize,
    pub dirt_depth: usize,
}

impl TerrainStage {
    pub fn new(grass: BlockId, dirt: BlockId, stone: BlockId) -> Self {
        Self {
            grass,
            dirt,
            stone,
            base_height: 64,
            height_variation: 18,
            dirt_depth: 4,
        }
    }

    pub fn height_at(&self, seed: u64, x: i32, z: i32) -> usize {
        let low_frequency = value_noise_2d(seed, x.div_euclid(8), z.div_euclid(8));
        let detail = value_noise_2d(
            seed ^ 0xA5A5_A5A5_A5A5_A5A5,
            x.div_euclid(3),
            z.div_euclid(3),
        );
        let combined = low_frequency * 0.75 + detail * 0.25;
        let height = self.base_height as f32 + combined * self.height_variation as f32;
        height.round().clamp(1.0, (CHUNK_HEIGHT - 1) as f32) as usize
    }
}

impl GenerationStage for TerrainStage {
    fn name(&self) -> &str {
        "engine:terrain"
    }

    fn generate(&self, chunk: &mut Chunk, context: &GenerationContext) {
        for local_x in 0..CHUNK_SIZE {
            for local_z in 0..CHUNK_SIZE {
                let x = world_x(chunk.position(), local_x);
                let z = world_z(chunk.position(), local_z);
                let surface_y = self.height_at(context.seed, x, z);

                for y in 0..=surface_y {
                    let block = if y == surface_y {
                        self.grass
                    } else if y + self.dirt_depth >= surface_y {
                        self.dirt
                    } else {
                        self.stone
                    };

                    chunk
                        .set_block(
                            BlockPosition {
                                x: local_x,
                                y,
                                z: local_z,
                            },
                            block,
                        )
                        .expect("terrain generated positions stay inside chunk bounds");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::world::{BlockRegistry, ChunkPosition};

    #[test]
    fn terrain_places_grass_above_dirt_and_stone() {
        let air = BlockId::from(0);
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let stage = TerrainStage::new(grass, dirt, stone);
        let context = GenerationContext { seed: 12, air };
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, air);

        stage.generate(&mut chunk, &context);
        let surface_y = stage.height_at(context.seed, 0, 0);

        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: surface_y,
                z: 0
            }),
            Some(grass)
        );
        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: surface_y - 1,
                z: 0
            }),
            Some(dirt)
        );
        assert_eq!(chunk.block(BlockPosition { x: 0, y: 1, z: 0 }), Some(stone));
        assert!(BlockRegistry::new().is_empty());
    }
}
