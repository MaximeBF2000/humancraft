//! World generation pipeline.
//!
//! Purpose:
//! Compose independent generation stages into deterministic chunk generation.
//!
//! Inputs:
//! World seed, chunk position, block registry IDs supplied by content.
//!
//! Outputs:
//! Populated chunks.
//!
//! Extension points:
//! Add new stages for caves, water, trees, decorations, and structures without
//! changing existing stage implementations.

pub mod biome;
pub mod ore;
pub mod terrain;
pub mod tree;

use crate::engine::world::{BlockId, Chunk, ChunkPosition};

#[derive(Debug, Clone)]
pub struct GenerationContext {
    pub seed: u64,
    pub air: BlockId,
}

pub trait GenerationStage {
    fn name(&self) -> &str;
    fn generate(&self, chunk: &mut Chunk, context: &GenerationContext);
}

#[derive(Default)]
pub struct GenerationPipeline {
    stages: Vec<Box<dyn GenerationStage>>,
}

impl GenerationPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_stage(mut self, stage: impl GenerationStage + 'static) -> Self {
        self.stages.push(Box::new(stage));
        self
    }

    pub fn generate_chunk(&self, position: ChunkPosition, context: &GenerationContext) -> Chunk {
        let mut chunk = Chunk::filled(position, context.air);
        for stage in &self.stages {
            stage.generate(&mut chunk, context);
        }
        chunk
    }

    pub fn stage_names(&self) -> Vec<&str> {
        self.stages.iter().map(|stage| stage.name()).collect()
    }
}

/// Small deterministic value-noise helper used until the project pulls in the
/// planned `noise` crate.
pub fn value_noise_2d(seed: u64, x: i32, z: i32) -> f32 {
    let mut n = seed
        .wrapping_add((x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add((z as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    n ^= n >> 30;
    n = n.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    n ^= n >> 27;
    n = n.wrapping_mul(0x94D0_49BB_1331_11EB);
    n ^= n >> 31;
    (n as f64 / u64::MAX as f64) as f32
}

pub fn interpolated_value_noise_2d(seed: u64, x: i32, z: i32, scale: i32) -> f32 {
    let scale = scale.max(1);
    let cell_x = x.div_euclid(scale);
    let cell_z = z.div_euclid(scale);
    let local_x = x.rem_euclid(scale) as f32 / scale as f32;
    let local_z = z.rem_euclid(scale) as f32 / scale as f32;
    let blend_x = smoothstep(local_x);
    let blend_z = smoothstep(local_z);

    let north_west = value_noise_2d(seed, cell_x, cell_z);
    let north_east = value_noise_2d(seed, cell_x + 1, cell_z);
    let south_west = value_noise_2d(seed, cell_x, cell_z + 1);
    let south_east = value_noise_2d(seed, cell_x + 1, cell_z + 1);

    let north = lerp(north_west, north_east, blend_x);
    let south = lerp(south_west, south_east, blend_x);
    lerp(north, south, blend_z)
}

pub fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

pub fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

pub fn world_x(chunk_position: ChunkPosition, local_x: usize) -> i32 {
    chunk_position.x * super::CHUNK_SIZE as i32 + local_x as i32
}

pub fn world_z(chunk_position: ChunkPosition, local_z: usize) -> i32 {
    chunk_position.z * super::CHUNK_SIZE as i32 + local_z as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FillStage {
        block: BlockId,
    }

    impl GenerationStage for FillStage {
        fn name(&self) -> &str {
            "test:fill"
        }

        fn generate(&self, chunk: &mut Chunk, _context: &GenerationContext) {
            chunk
                .set_block(
                    crate::engine::world::BlockPosition { x: 0, y: 0, z: 0 },
                    self.block,
                )
                .unwrap();
        }
    }

    #[test]
    fn pipeline_applies_stages_in_order() {
        let air = BlockId::from(0);
        let stone = BlockId::from(1);
        let dirt = BlockId::from(2);
        let pipeline = GenerationPipeline::new()
            .add_stage(FillStage { block: stone })
            .add_stage(FillStage { block: dirt });

        let chunk = pipeline.generate_chunk(
            ChunkPosition { x: 0, z: 0 },
            &GenerationContext { seed: 7, air },
        );

        assert_eq!(
            chunk.block(crate::engine::world::BlockPosition { x: 0, y: 0, z: 0 }),
            Some(dirt)
        );
    }

    #[test]
    fn value_noise_is_deterministic() {
        assert_eq!(value_noise_2d(42, -3, 9), value_noise_2d(42, -3, 9));
        assert_ne!(value_noise_2d(42, -3, 9), value_noise_2d(43, -3, 9));
    }

    #[test]
    fn interpolated_noise_changes_smoothly_inside_cell() {
        let first = interpolated_value_noise_2d(42, 10, -20, 64);
        let second = interpolated_value_noise_2d(42, 11, -20, 64);

        assert!((first - second).abs() < 0.05);
    }
}
