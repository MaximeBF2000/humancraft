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
        let rough_detail = interpolated_value_noise_2d(
            seed ^ 0xD37A_1175_D37A_1175,
            x,
            z,
            (biome.detail_scale / 2).max(1),
        ) - 0.5;
        let ridge_sample = interpolated_value_noise_2d(
            seed ^ 0xBADC_0FFE_E0DD_F00D,
            x,
            z,
            biome.ridge_scale.max(1),
        );
        let ridges = 1.0 - (ridge_sample * 2.0 - 1.0).abs();
        let combined = low_frequency * 0.65 + detail * 0.35;
        biome.base_height as f32
            + combined * biome.height_variation as f32
            + rough_detail * biome.roughness
            + ridges * biome.ridge_strength
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
                let exposed_surface =
                    exposed_surface_block(self, context.seed, x, z, surface_y, biome);

                for y in 0..=surface_y {
                    let block = if y == surface_y {
                        exposed_surface
                            .unwrap_or_else(|| block_for_depth_from_surface(biome, surface_y - y))
                    } else {
                        block_for_depth_from_surface(biome, surface_y - y)
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

fn exposed_surface_block(
    terrain: &TerrainStage,
    seed: u64,
    x: i32,
    z: i32,
    surface_y: usize,
    biome: &BiomeDefinition,
) -> Option<crate::engine::world::BlockId> {
    let rule = biome.exposed_surface?;
    let east = terrain.height_at(seed, x + 1, z);
    let west = terrain.height_at(seed, x - 1, z);
    let south = terrain.height_at(seed, x, z + 1);
    let north = terrain.height_at(seed, x, z - 1);
    let max_neighbor = east.max(west).max(south).max(north);
    let min_neighbor = east.min(west).min(south).min(north);
    let slope = max_neighbor - min_neighbor;

    (rule
        .min_height
        .map(|min_height| surface_y >= min_height)
        .unwrap_or(false)
        || slope >= rule.min_slope)
        .then_some(rule.block)
}

fn block_for_depth_from_surface(
    biome: &BiomeDefinition,
    depth_from_surface: usize,
) -> crate::engine::world::BlockId {
    let mut covered_depth = 0;
    for layer in &biome.layers {
        covered_depth += layer.depth;
        if depth_from_surface < covered_depth {
            return layer.block;
        }
    }
    biome.stone
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
    fn terrain_uses_custom_biome_layers_from_surface_down() {
        let air = BlockId::from(0);
        let sand = BlockId::from(1);
        let sandstone = BlockId::from(2);
        let stone = BlockId::from(3);
        let biome_source = BiomeSource::new(vec![
            BiomeDefinition::new("test:desert", sand, sandstone, stone)
                .layers([
                    crate::engine::world::generation::biome::TerrainLayer::new(sand, 3),
                    crate::engine::world::generation::biome::TerrainLayer::new(sandstone, 4),
                ])
                .terrain(64, 1, 4, 96, 32),
        ]);
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
            Some(sand)
        );
        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: surface_y - 3,
                z: 0
            }),
            Some(sandstone)
        );
        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: surface_y - 7,
                z: 0
            }),
            Some(stone)
        );
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

    #[test]
    fn exposed_surface_rule_can_replace_high_or_steep_surface() {
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let biome_source = BiomeSource::new(vec![
            BiomeDefinition::new("test:mountain", grass, dirt, stone)
                .terrain(90, 20, 3, 96, 32)
                .exposed_surface(
                    crate::engine::world::generation::biome::ExposedSurfaceRule::new(
                        stone,
                        Some(80),
                        3,
                    ),
                ),
        ]);
        let stage = TerrainStage::new(biome_source);
        let context = GenerationContext {
            seed: 1234,
            air: BlockId::from(0),
        };
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, context.air);

        stage.generate(&mut chunk, &context);
        let surface_y = stage.height_at(context.seed, 0, 0);

        assert_eq!(
            chunk.block(BlockPosition {
                x: 0,
                y: surface_y,
                z: 0
            }),
            Some(stone)
        );
    }
}
