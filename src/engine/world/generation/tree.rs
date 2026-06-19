//! Tree decoration generation stage.
//!
//! Purpose:
//! Place reusable tree definitions after terrain and resource stages. Content
//! owns concrete tree data; this stage owns deterministic placement mechanics.

use crate::engine::world::generation::biome::BiomeSource;
use crate::engine::world::generation::{
    GenerationContext, GenerationStage, value_noise_2d, world_x, world_z,
};
use crate::engine::world::{
    BlockId, BlockPosition, BlockProperties, BlockState, CHUNK_HEIGHT, CHUNK_SIZE, Chunk,
};

#[derive(Debug, Clone)]
pub struct TreeDefinition {
    pub key: String,
    pub trunk: BlockId,
    pub leaves: BlockId,
    pub grow_on: Vec<BlockId>,
    pub replaceable: Vec<BlockId>,
    pub biomes: Vec<String>,
    pub min_trunk_height: usize,
    pub max_trunk_height: usize,
    pub canopy_radius: usize,
    pub density: f32,
}

impl TreeDefinition {
    pub fn new(key: impl Into<String>, trunk: BlockId, leaves: BlockId) -> Self {
        Self {
            key: key.into(),
            trunk,
            leaves,
            grow_on: Vec::new(),
            replaceable: Vec::new(),
            biomes: Vec::new(),
            min_trunk_height: 4,
            max_trunk_height: 5,
            canopy_radius: 2,
            density: 0.02,
        }
    }

    pub fn grow_on(mut self, blocks: impl IntoIterator<Item = BlockId>) -> Self {
        self.grow_on = blocks.into_iter().collect();
        self
    }

    pub fn replaceable(mut self, blocks: impl IntoIterator<Item = BlockId>) -> Self {
        self.replaceable = blocks.into_iter().collect();
        self
    }

    pub fn biomes(mut self, biomes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.biomes = biomes.into_iter().map(Into::into).collect();
        self
    }

    pub fn shape(
        mut self,
        min_trunk_height: usize,
        max_trunk_height: usize,
        canopy_radius: usize,
    ) -> Self {
        self.min_trunk_height = min_trunk_height.max(1);
        self.max_trunk_height = max_trunk_height.max(self.min_trunk_height);
        self.canopy_radius = canopy_radius;
        self
    }

    pub fn density(mut self, density: f32) -> Self {
        self.density = density.clamp(0.0, 1.0);
        self
    }
}

#[derive(Debug, Clone)]
pub struct TreeStage {
    biome_source: BiomeSource,
    trees: Vec<TreeDefinition>,
}

impl TreeStage {
    pub fn new(biome_source: BiomeSource, trees: Vec<TreeDefinition>) -> Self {
        Self {
            biome_source,
            trees,
        }
    }
}

impl GenerationStage for TreeStage {
    fn name(&self) -> &str {
        "engine:trees"
    }

    fn generate(&self, chunk: &mut Chunk, context: &GenerationContext) {
        for local_x in 0..CHUNK_SIZE {
            for local_z in 0..CHUNK_SIZE {
                let x = world_x(chunk.position(), local_x);
                let z = world_z(chunk.position(), local_z);
                let biome = self.biome_source.biome_at(context.seed, x, z);

                for tree in &self.trees {
                    if !tree.biomes.iter().any(|key| key == &biome.key) {
                        continue;
                    }
                    if !tree_origin_allowed_in_chunk(local_x, local_z, tree.canopy_radius) {
                        continue;
                    }
                    if !should_place_tree(context.seed, x, z, tree) {
                        continue;
                    }

                    let Some(surface_y) = find_surface_y(chunk, local_x, local_z, context.air)
                    else {
                        continue;
                    };
                    let Some(surface_block) = chunk.block(BlockPosition {
                        x: local_x,
                        y: surface_y,
                        z: local_z,
                    }) else {
                        continue;
                    };
                    if !tree.grow_on.contains(&surface_block) {
                        continue;
                    }

                    let trunk_height = trunk_height(context.seed, x, z, tree);
                    if surface_y + trunk_height + tree.canopy_radius + 1 >= CHUNK_HEIGHT {
                        continue;
                    }
                    if !tree_space_is_clear(chunk, local_x, surface_y, local_z, tree, trunk_height)
                    {
                        continue;
                    }

                    place_tree(chunk, local_x, surface_y, local_z, tree, trunk_height);
                    break;
                }
            }
        }
    }
}

fn tree_origin_allowed_in_chunk(local_x: usize, local_z: usize, canopy_radius: usize) -> bool {
    let margin = canopy_radius + 1;
    local_x >= margin
        && local_z >= margin
        && local_x + margin < CHUNK_SIZE
        && local_z + margin < CHUNK_SIZE
}

fn should_place_tree(seed: u64, x: i32, z: i32, tree: &TreeDefinition) -> bool {
    let sample = value_noise_2d(seed ^ hash_key(&tree.key), x, z);
    sample >= 1.0 - tree.density
}

fn trunk_height(seed: u64, x: i32, z: i32, tree: &TreeDefinition) -> usize {
    let range = tree.max_trunk_height - tree.min_trunk_height + 1;
    let sample = value_noise_2d(seed ^ hash_key(&tree.key) ^ 0x7AEE_7AEE, x, z);
    tree.min_trunk_height + (sample * range as f32).floor() as usize % range
}

fn find_surface_y(chunk: &Chunk, local_x: usize, local_z: usize, air: BlockId) -> Option<usize> {
    (0..CHUNK_HEIGHT).rev().find(|y| {
        chunk.block(BlockPosition {
            x: local_x,
            y: *y,
            z: local_z,
        }) != Some(air)
    })
}

fn tree_space_is_clear(
    chunk: &Chunk,
    local_x: usize,
    surface_y: usize,
    local_z: usize,
    tree: &TreeDefinition,
    trunk_height: usize,
) -> bool {
    for y in surface_y + 1..=surface_y + trunk_height {
        let position = BlockPosition {
            x: local_x,
            y,
            z: local_z,
        };
        if !is_replaceable(chunk, position, tree) {
            return false;
        }
    }

    let leaf_base_y = surface_y + trunk_height - 1;
    for y in leaf_base_y..=surface_y + trunk_height + tree.canopy_radius {
        for x in local_x - tree.canopy_radius..=local_x + tree.canopy_radius {
            for z in local_z - tree.canopy_radius..=local_z + tree.canopy_radius {
                let position = BlockPosition { x, y, z };
                if !is_replaceable(chunk, position, tree) {
                    return false;
                }
            }
        }
    }

    true
}

fn place_tree(
    chunk: &mut Chunk,
    local_x: usize,
    surface_y: usize,
    local_z: usize,
    tree: &TreeDefinition,
    trunk_height: usize,
) {
    for y in surface_y + 1..=surface_y + trunk_height {
        chunk
            .set_block(
                BlockPosition {
                    x: local_x,
                    y,
                    z: local_z,
                },
                tree.trunk,
            )
            .expect("tree trunk positions stay inside chunk bounds");
    }

    let leaf_base_y = surface_y + trunk_height - 1;
    for y in leaf_base_y..=surface_y + trunk_height + tree.canopy_radius {
        let vertical_distance = y.abs_diff(surface_y + trunk_height);
        let layer_radius = tree.canopy_radius.saturating_sub(vertical_distance / 2);
        for x in local_x - layer_radius..=local_x + layer_radius {
            for z in local_z - layer_radius..=local_z + layer_radius {
                let horizontal_distance = local_x.abs_diff(x) + local_z.abs_diff(z);
                if horizontal_distance > layer_radius * 2 {
                    continue;
                }
                let position = BlockPosition { x, y, z };
                if chunk.block(position) == Some(tree.trunk) {
                    continue;
                }
                chunk
                    .set_block_state(
                        position,
                        BlockState::with_properties(
                            tree.leaves,
                            BlockProperties::Leaves { persistent: false },
                        ),
                    )
                    .expect("tree leaf positions stay inside chunk bounds");
            }
        }
    }
}

fn is_replaceable(chunk: &Chunk, position: BlockPosition, tree: &TreeDefinition) -> bool {
    chunk
        .block(position)
        .map(|block| tree.replaceable.contains(&block))
        .unwrap_or(false)
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
    use crate::engine::world::generation::biome::BiomeDefinition;
    use crate::engine::world::generation::terrain::TerrainStage;
    use crate::engine::world::{ChunkPosition, generation::GenerationPipeline};

    #[test]
    fn tree_stage_places_configured_tree_blocks() {
        let air = BlockId::from(0);
        let grass = BlockId::from(1);
        let dirt = BlockId::from(2);
        let stone = BlockId::from(3);
        let log = BlockId::from(4);
        let leaves = BlockId::from(5);
        let biome_source = BiomeSource::new(vec![BiomeDefinition::new(
            "test:forest",
            grass,
            dirt,
            stone,
        )]);
        let pipeline = GenerationPipeline::new()
            .add_stage(TerrainStage::new(biome_source.clone()))
            .add_stage(TreeStage::new(
                biome_source,
                vec![
                    TreeDefinition::new("test:oak", log, leaves)
                        .grow_on([grass])
                        .replaceable([air, leaves])
                        .biomes(["test:forest"])
                        .density(1.0),
                ],
            ));

        let chunk = pipeline.generate_chunk(
            ChunkPosition { x: 0, z: 0 },
            &GenerationContext { seed: 1, air },
        );

        assert!(chunk.blocks().contains(&log));
        assert!(chunk.blocks().contains(&leaves));
    }
}
