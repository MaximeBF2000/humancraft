//! Ore generation stage.
//!
//! Purpose:
//! Place any number of ore definitions using reusable distribution data.
//! Content supplies ore definitions; the engine only understands generic
//! placement constraints.

use crate::engine::world::generation::{
    GenerationContext, GenerationStage, value_noise_2d, world_x, world_z,
};
use crate::engine::world::{BlockId, BlockPosition, CHUNK_HEIGHT, CHUNK_SIZE, Chunk};

#[derive(Debug, Clone)]
pub struct OreDefinition {
    pub key: String,
    pub block: BlockId,
    pub replaces: BlockId,
    pub min_y: usize,
    pub max_y: usize,
    pub threshold: f32,
}

#[derive(Debug, Clone, Default)]
pub struct OreStage {
    ores: Vec<OreDefinition>,
}

impl OreStage {
    pub fn new(ores: Vec<OreDefinition>) -> Self {
        Self { ores }
    }

    pub fn ores(&self) -> &[OreDefinition] {
        &self.ores
    }
}

impl GenerationStage for OreStage {
    fn name(&self) -> &str {
        "engine:ores"
    }

    fn generate(&self, chunk: &mut Chunk, context: &GenerationContext) {
        for ore in &self.ores {
            let max_y = ore.max_y.min(CHUNK_HEIGHT - 1);
            for local_x in 0..CHUNK_SIZE {
                for local_z in 0..CHUNK_SIZE {
                    let x = world_x(chunk.position(), local_x);
                    let z = world_z(chunk.position(), local_z);
                    for y in ore.min_y..=max_y {
                        let position = BlockPosition {
                            x: local_x,
                            y,
                            z: local_z,
                        };
                        if chunk.block(position) != Some(ore.replaces) {
                            continue;
                        }

                        let sample = value_noise_2d(
                            context.seed ^ hash_key(&ore.key) ^ y as u64,
                            x.div_euclid(3),
                            z.div_euclid(3),
                        );

                        if sample >= ore.threshold {
                            chunk
                                .set_block(position, ore.block)
                                .expect("ore generated positions stay inside chunk bounds");
                        }
                    }
                }
            }
        }
    }
}

fn hash_key(key: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for byte in key.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::world::generation::biome::{BiomeDefinition, BiomeSource};
    use crate::engine::world::{ChunkPosition, generation::terrain::TerrainStage};

    #[test]
    fn ore_only_replaces_target_block() {
        let air = BlockId::from(0);
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let coal = BlockId::from(4);
        let context = GenerationContext { seed: 99, air };
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, air);
        let terrain = TerrainStage::new(BiomeSource::new(vec![BiomeDefinition::new(
            "test:plain",
            grass,
            dirt,
            stone,
        )]));
        terrain.generate(&mut chunk, &context);
        let surface_y = terrain.height_at(context.seed, 0, 0);
        let topsoil_y = surface_y - 1;

        OreStage::new(vec![OreDefinition {
            key: "test:coal".to_string(),
            block: coal,
            replaces: stone,
            min_y: 0,
            max_y: 32,
            threshold: 0.0,
        }])
        .generate(&mut chunk, &context);

        assert_eq!(chunk.block(BlockPosition { x: 0, y: 1, z: 0 }), Some(coal));
        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: topsoil_y,
                z: 0
            }),
            Some(dirt)
        );
    }
}
