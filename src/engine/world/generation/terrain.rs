//! Terrain generation stage.
//!
//! Purpose:
//! Fill a chunk column from bedrock-ish stone through dirt and grass using a
//! generic height function. This stage owns terrain shape, not specific biome
//! rules.

use crate::engine::world::generation::biome::{BiomeDefinition, BiomeSource};
use crate::engine::world::generation::{
    GenerationContext, GenerationStage, interpolated_value_noise_2d, world_x, world_z,
};
use crate::engine::world::{BlockPosition, CHUNK_HEIGHT, CHUNK_SIZE, Chunk};

#[derive(Debug, Clone)]
pub struct TerrainStage {
    pub biome_source: BiomeSource,
}

impl TerrainStage {
    pub fn new(biome_source: BiomeSource) -> Self {
        Self { biome_source }
    }

    pub fn height_at(&self, seed: u64, x: i32, z: i32) -> usize {
        self.blended_height_at(seed, x, z)
    }

    pub fn height_for_biome(&self, seed: u64, x: i32, z: i32, biome: &BiomeDefinition) -> usize {
        self.height_for_biome_f32(seed, x, z, biome)
            .round()
            .clamp(1.0, (CHUNK_HEIGHT - 1) as f32) as usize
    }

    fn blended_height_at(&self, seed: u64, x: i32, z: i32) -> usize {
        let height = self
            .biome_source
            .influences_at(seed, x, z)
            .into_iter()
            .map(|influence| {
                self.height_for_biome_f32(seed, x, z, influence.biome) * influence.weight
            })
            .sum::<f32>();

        height.round().clamp(1.0, (CHUNK_HEIGHT - 1) as f32) as usize
    }

    fn height_for_biome_f32(&self, seed: u64, x: i32, z: i32, biome: &BiomeDefinition) -> f32 {
        let low_frequency = interpolated_value_noise_2d(seed, x, z, biome.terrain_scale.max(1));
        let detail = interpolated_value_noise_2d(
            seed ^ 0xA5A5_A5A5_A5A5_A5A5,
            x,
            z,
            biome.detail_scale.max(1),
        );
        let combined = low_frequency * 0.75 + detail * 0.25;
        biome.base_height as f32 + combined * biome.height_variation as f32
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
                let biome = self.biome_source.biome_at(context.seed, x, z);
                let surface_y = self.height_at(context.seed, x, z);

                for y in 0..=surface_y {
                    let block = if y == surface_y {
                        biome.surface
                    } else if y + biome.dirt_depth >= surface_y {
                        biome.subsurface
                    } else {
                        biome.stone
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
    use crate::engine::world::{BlockId, BlockRegistry, ChunkPosition};

    #[test]
    fn terrain_places_grass_above_dirt_and_stone() {
        let air = BlockId::from(0);
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let biome_source =
            BiomeSource::new(vec![BiomeDefinition::new("test:plain", grass, dirt, stone)]);
        let stage = TerrainStage::new(biome_source);
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

    #[test]
    fn terrain_height_is_continuous_across_chunk_border() {
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let biome_source = BiomeSource::new(vec![
            BiomeDefinition::new("test:plain", grass, dirt, stone).terrain(62, 8, 4, 96, 32),
            BiomeDefinition::new("test:forest", grass, dirt, stone).terrain(64, 10, 4, 96, 32),
            BiomeDefinition::new("test:mountain", grass, dirt, stone).terrain(72, 22, 3, 96, 32),
        ])
        .with_min_region_chunks(8);
        let stage = TerrainStage::new(biome_source);

        for z in -64..64 {
            let west_height = stage.height_at(1234, 15, z);
            let east_height = stage.height_at(1234, 16, z);

            assert!(
                west_height.abs_diff(east_height) <= 2,
                "height jumped from {west_height} to {east_height} at z {z}"
            );
        }
    }

    #[test]
    fn terrain_height_is_continuous_across_biome_region_border() {
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let biome_source = BiomeSource::new(vec![
            BiomeDefinition::new("test:plain", grass, dirt, stone).terrain(62, 8, 4, 96, 32),
            BiomeDefinition::new("test:mountain", grass, dirt, stone).terrain(72, 22, 3, 128, 40),
        ])
        .with_min_region_chunks(8)
        .with_transition_chunks(2);
        let stage = TerrainStage::new(biome_source);
        let region_border_x = 8 * CHUNK_SIZE as i32;

        for z in -64..64 {
            let west_height = stage.height_at(1234, region_border_x - 1, z);
            let east_height = stage.height_at(1234, region_border_x, z);

            assert!(
                west_height.abs_diff(east_height) <= 2,
                "height jumped from {west_height} to {east_height} at z {z}"
            );
        }
    }
}
