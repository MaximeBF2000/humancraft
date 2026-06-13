//! Biome selection data used by generation stages.
//!
//! Purpose:
//! Define reusable biome profiles and deterministic biome lookup without
//! baking HumanCraft-specific content into engine code.

use crate::engine::world::BlockId;
use crate::engine::world::CHUNK_SIZE;
use crate::engine::world::generation::{smoothstep, value_noise_2d};

#[derive(Debug, Clone)]
pub struct BiomeDefinition {
    pub key: String,
    pub surface: BlockId,
    pub subsurface: BlockId,
    pub stone: BlockId,
    pub layers: Vec<TerrainLayer>,
    pub base_height: usize,
    pub height_variation: usize,
    pub dirt_depth: usize,
    pub terrain_scale: i32,
    pub detail_scale: i32,
    pub roughness: f32,
    pub ridge_strength: f32,
    pub ridge_scale: i32,
    pub exposed_surface: Option<ExposedSurfaceRule>,
}

impl BiomeDefinition {
    pub fn new(
        key: impl Into<String>,
        surface: BlockId,
        subsurface: BlockId,
        stone: BlockId,
    ) -> Self {
        let surface = surface;
        let subsurface = subsurface;
        Self {
            key: key.into(),
            surface,
            subsurface,
            stone,
            layers: vec![
                TerrainLayer::new(surface, 1),
                TerrainLayer::new(subsurface, 4),
            ],
            base_height: 64,
            height_variation: 16,
            dirt_depth: 4,
            terrain_scale: 8,
            detail_scale: 3,
            roughness: 0.0,
            ridge_strength: 0.0,
            ridge_scale: 24,
            exposed_surface: None,
        }
    }

    pub fn terrain(
        mut self,
        base_height: usize,
        height_variation: usize,
        dirt_depth: usize,
        terrain_scale: i32,
        detail_scale: i32,
    ) -> Self {
        self.base_height = base_height;
        self.height_variation = height_variation;
        self.dirt_depth = dirt_depth;
        self.terrain_scale = terrain_scale.max(1);
        self.detail_scale = detail_scale.max(1);
        self.layers = vec![
            TerrainLayer::new(self.surface, 1),
            TerrainLayer::new(self.subsurface, dirt_depth),
        ];
        self
    }

    pub fn relief(mut self, roughness: f32, ridge_strength: f32, ridge_scale: i32) -> Self {
        self.roughness = roughness.max(0.0);
        self.ridge_strength = ridge_strength.max(0.0);
        self.ridge_scale = ridge_scale.max(1);
        self
    }

    pub fn exposed_surface(mut self, rule: ExposedSurfaceRule) -> Self {
        self.exposed_surface = Some(rule);
        self
    }

    pub fn layers(mut self, layers: impl IntoIterator<Item = TerrainLayer>) -> Self {
        self.layers = layers.into_iter().filter(|layer| layer.depth > 0).collect();
        assert!(
            !self.layers.is_empty(),
            "biome requires at least one terrain layer"
        );
        self
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ExposedSurfaceRule {
    pub block: BlockId,
    pub min_height: Option<usize>,
    pub min_slope: usize,
}

impl ExposedSurfaceRule {
    pub fn new(block: BlockId, min_height: Option<usize>, min_slope: usize) -> Self {
        Self {
            block,
            min_height,
            min_slope,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TerrainLayer {
    pub block: BlockId,
    pub depth: usize,
}

impl TerrainLayer {
    pub fn new(block: BlockId, depth: usize) -> Self {
        Self {
            block,
            depth: depth.max(1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BiomeSource {
    biomes: Vec<BiomeDefinition>,
    region_size_blocks: i32,
    transition_width_blocks: i32,
}

impl BiomeSource {
    pub fn new(biomes: Vec<BiomeDefinition>) -> Self {
        assert!(
            !biomes.is_empty(),
            "biome source requires at least one biome"
        );
        Self {
            biomes,
            region_size_blocks: CHUNK_SIZE as i32 * 6,
            transition_width_blocks: CHUNK_SIZE as i32,
        }
    }

    pub fn with_min_region_chunks(mut self, chunks: i32) -> Self {
        self.region_size_blocks = chunks.max(1) * CHUNK_SIZE as i32;
        self.transition_width_blocks = self
            .transition_width_blocks
            .min((self.region_size_blocks / 2).max(1));
        self
    }

    pub fn with_transition_chunks(mut self, chunks: i32) -> Self {
        self.transition_width_blocks =
            (chunks.max(0) * CHUNK_SIZE as i32).min((self.region_size_blocks / 2).max(1));
        self
    }

    pub fn biomes(&self) -> &[BiomeDefinition] {
        &self.biomes
    }

    pub fn biome_at(&self, seed: u64, x: i32, z: i32) -> &BiomeDefinition {
        let index = self.primary_biome_index_at(seed, x, z);
        &self.biomes[index]
    }

    pub fn influences_at(&self, seed: u64, x: i32, z: i32) -> Vec<BiomeInfluence<'_>> {
        let cell_x = x.div_euclid(self.region_size_blocks);
        let cell_z = z.div_euclid(self.region_size_blocks);
        let local_x = x.rem_euclid(self.region_size_blocks);
        let local_z = z.rem_euclid(self.region_size_blocks);
        let x_weights = axis_region_weights(cell_x, local_x, self);
        let z_weights = axis_region_weights(cell_z, local_z, self);

        let mut weights = vec![0.0; self.biomes.len()];
        for (region_x, x_weight) in x_weights {
            for (region_z, z_weight) in z_weights {
                let weight = x_weight * z_weight;
                if weight <= 0.0 {
                    continue;
                }
                let biome_index = self.region_biome_index(seed, region_x, region_z);
                weights[biome_index] += weight;
            }
        }

        weights
            .into_iter()
            .enumerate()
            .filter(|(_, weight)| *weight > 0.0)
            .map(|(index, weight)| BiomeInfluence {
                biome: &self.biomes[index],
                weight,
            })
            .collect()
    }

    pub fn primary_biome_index_at(&self, seed: u64, x: i32, z: i32) -> usize {
        self.influences_at(seed, x, z)
            .into_iter()
            .max_by(|a, b| a.weight.total_cmp(&b.weight))
            .map(|influence| {
                self.biomes
                    .iter()
                    .position(|biome| biome.key == influence.biome.key)
                    .expect("influence biome belongs to source")
            })
            .unwrap_or(0)
    }

    pub fn region_size_blocks(&self) -> i32 {
        self.region_size_blocks
    }

    pub fn transition_width_blocks(&self) -> i32 {
        self.transition_width_blocks
    }

    fn region_biome_index(&self, seed: u64, region_x: i32, region_z: i32) -> usize {
        let sample = value_noise_2d(seed ^ 0x3A55_B10E_5EED, region_x, region_z);
        let index = (sample.clamp(0.0, 0.999_999) * self.biomes.len() as f32).floor() as usize;
        index.min(self.biomes.len() - 1)
    }
}

fn axis_region_weights(cell: i32, local: i32, source: &BiomeSource) -> [(i32, f32); 2] {
    let transition = source.transition_width_blocks;
    if transition <= 0 {
        return [(cell, 1.0), (cell, 0.0)];
    }

    if local >= source.region_size_blocks - transition {
        let next = smoothstep(
            (local - (source.region_size_blocks - transition)) as f32 / transition as f32,
        );
        [(cell, 1.0 - next), (cell + 1, next)]
    } else {
        [(cell, 1.0), (cell, 0.0)]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BiomeInfluence<'a> {
    pub biome: &'a BiomeDefinition,
    pub weight: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biome_source_is_deterministic() {
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let source = BiomeSource::new(vec![
            BiomeDefinition::new("test:plain", grass, dirt, stone),
            BiomeDefinition::new("test:forest", grass, dirt, stone),
            BiomeDefinition::new("test:mountain", grass, dirt, stone),
        ]);

        let first = source.biome_at(42, -120, 90).key.as_str();
        let second = source.biome_at(42, -120, 90).key.as_str();

        assert_eq!(first, second);
    }

    #[test]
    fn biome_regions_have_configurable_minimum_chunk_size() {
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let source = BiomeSource::new(vec![BiomeDefinition::new("test:plain", grass, dirt, stone)])
            .with_min_region_chunks(8);

        assert_eq!(source.region_size_blocks(), 8 * CHUNK_SIZE as i32);
    }

    #[test]
    fn biome_regions_have_configurable_transition_width() {
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let source = BiomeSource::new(vec![BiomeDefinition::new("test:plain", grass, dirt, stone)])
            .with_min_region_chunks(8)
            .with_transition_chunks(2);

        assert_eq!(source.transition_width_blocks(), 2 * CHUNK_SIZE as i32);
    }

    #[test]
    fn biome_influences_are_normalized() {
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let source = BiomeSource::new(vec![
            BiomeDefinition::new("test:plain", grass, dirt, stone),
            BiomeDefinition::new("test:forest", grass, dirt, stone),
            BiomeDefinition::new("test:mountain", grass, dirt, stone),
        ]);

        let total: f32 = source
            .influences_at(42, 37, -91)
            .iter()
            .map(|influence| influence.weight)
            .sum();

        assert!((total - 1.0).abs() < 0.000_1);
    }

    #[test]
    fn biome_region_core_uses_one_biome() {
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let source = BiomeSource::new(vec![
            BiomeDefinition::new("test:plain", grass, dirt, stone),
            BiomeDefinition::new("test:forest", grass, dirt, stone),
        ])
        .with_min_region_chunks(8)
        .with_transition_chunks(1);

        assert_eq!(source.influences_at(42, 64, 64).len(), 1);
    }

    #[test]
    fn biome_accepts_custom_terrain_layers() {
        let sand = BlockId::from(1);
        let sandstone = BlockId::from(2);
        let stone = BlockId::from(3);
        let biome = BiomeDefinition::new("test:desert", sand, sandstone, stone)
            .layers([TerrainLayer::new(sand, 4), TerrainLayer::new(sandstone, 6)]);

        assert_eq!(biome.layers[0], TerrainLayer::new(sand, 4));
        assert_eq!(biome.layers[1], TerrainLayer::new(sandstone, 6));
    }
}
