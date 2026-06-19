//! Chunk meshing.
//!
//! Purpose:
//! Build visible faces for a chunk using block definition properties.
//!
//! Inputs:
//! - Chunk block IDs.
//! - Block definitions for solidity and transparency.
//!
//! Outputs:
//! - Renderer-neutral quads with positions and block IDs.
//!
//! Extension points:
//! - Greedy meshing can replace the face-per-block output while keeping the
//!   public `ChunkMesh` shape stable.
//! - Lighting, texture atlas coordinates, and ambient occlusion can be attached
//!   to quads later.

use crate::engine::world::{
    BlockAabb, BlockId, BlockPosition, BlockRegistry, BlockShape, BlockState, CHUNK_HEIGHT,
    CHUNK_SIZE, Chunk, block_state_aabbs,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FaceDirection {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshQuad {
    pub block: BlockId,
    pub state: BlockState,
    pub direction: FaceDirection,
    pub vertices: [[f32; 3]; 4],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChunkMesh {
    pub quads: Vec<MeshQuad>,
}

impl ChunkMesh {
    pub fn vertex_count(&self) -> usize {
        self.quads.len() * 4
    }

    pub fn triangle_count(&self) -> usize {
        self.quads.len() * 2
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChunkMesher;

impl ChunkMesher {
    pub fn mesh_chunk(&self, chunk: &Chunk, blocks: &BlockRegistry) -> ChunkMesh {
        self.mesh_chunk_with_neighbor_lookup(chunk, blocks, |_, _| None)
    }

    pub fn mesh_chunk_with_neighbor_lookup(
        &self,
        chunk: &Chunk,
        blocks: &BlockRegistry,
        mut outside_neighbor: impl FnMut(BlockPosition, FaceDirection) -> Option<BlockId>,
    ) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();

        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let position = BlockPosition { x, y, z };
                    let Some(state) = chunk.block_state(position) else {
                        continue;
                    };
                    if !is_visible_block(state.block, blocks) {
                        continue;
                    }
                    let Some(definition) = blocks.get(state.block) else {
                        continue;
                    };

                    if definition.shape == BlockShape::Cross {
                        for vertices in cross_vertices(position) {
                            mesh.quads.push(MeshQuad {
                                block: state.block,
                                state,
                                direction: FaceDirection::North,
                                vertices,
                            });
                        }
                        continue;
                    }

                    for aabb in block_state_aabbs(definition, state).iter() {
                        for direction in [
                            FaceDirection::North,
                            FaceDirection::South,
                            FaceDirection::East,
                            FaceDirection::West,
                            FaceDirection::Up,
                            FaceDirection::Down,
                        ] {
                            if self.face_is_exposed(
                                chunk,
                                blocks,
                                position,
                                direction,
                                &mut outside_neighbor,
                            ) {
                                mesh.quads.push(MeshQuad {
                                    block: state.block,
                                    state,
                                    direction,
                                    vertices: aabb_face_vertices(position, aabb, direction),
                                });
                            }
                        }
                    }
                }
            }
        }

        mesh
    }

    fn face_is_exposed(
        &self,
        chunk: &Chunk,
        blocks: &BlockRegistry,
        position: BlockPosition,
        direction: FaceDirection,
        outside_neighbor: &mut impl FnMut(BlockPosition, FaceDirection) -> Option<BlockId>,
    ) -> bool {
        if let Some(neighbor_position) = neighbor_position(position, direction) {
            return chunk
                .block(neighbor_position)
                .map(|block| !is_occluding_block(block, blocks))
                .unwrap_or(true);
        }

        match outside_neighbor(position, direction) {
            Some(block) => !is_occluding_block(block, blocks),
            None => true,
        }
    }
}

fn is_visible_block(block: BlockId, blocks: &BlockRegistry) -> bool {
    blocks
        .get(block)
        .map(|definition| {
            definition.shape == BlockShape::Cross || definition.solid || !definition.transparent
        })
        .unwrap_or(false)
}

fn is_occluding_block(block: BlockId, blocks: &BlockRegistry) -> bool {
    blocks
        .get(block)
        .map(|definition| {
            definition.solid && !definition.transparent && definition.shape == BlockShape::FullCube
        })
        .unwrap_or(false)
}

fn neighbor_position(position: BlockPosition, direction: FaceDirection) -> Option<BlockPosition> {
    match direction {
        FaceDirection::North => position
            .z
            .checked_sub(1)
            .map(|z| BlockPosition { z, ..position }),
        FaceDirection::South => {
            let z = position.z + 1;
            (z < CHUNK_SIZE).then_some(BlockPosition { z, ..position })
        }
        FaceDirection::East => {
            let x = position.x + 1;
            (x < CHUNK_SIZE).then_some(BlockPosition { x, ..position })
        }
        FaceDirection::West => position
            .x
            .checked_sub(1)
            .map(|x| BlockPosition { x, ..position }),
        FaceDirection::Up => {
            let y = position.y + 1;
            (y < CHUNK_HEIGHT).then_some(BlockPosition { y, ..position })
        }
        FaceDirection::Down => position
            .y
            .checked_sub(1)
            .map(|y| BlockPosition { y, ..position }),
    }
}

fn face_vertices(position: BlockPosition, direction: FaceDirection) -> [[f32; 3]; 4] {
    aabb_face_vertices(
        position,
        BlockAabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        direction,
    )
}

fn aabb_face_vertices(
    position: BlockPosition,
    aabb: BlockAabb,
    direction: FaceDirection,
) -> [[f32; 3]; 4] {
    let x = position.x as f32;
    let y = position.y as f32;
    let z = position.z as f32;
    let x0 = x + aabb.min[0];
    let y0 = y + aabb.min[1];
    let z0 = z + aabb.min[2];
    let x1 = x + aabb.max[0];
    let y1 = y + aabb.max[1];
    let z1 = z + aabb.max[2];

    match direction {
        FaceDirection::North => [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
        FaceDirection::South => [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
        FaceDirection::East => [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]],
        FaceDirection::West => [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
        FaceDirection::Up => [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
        FaceDirection::Down => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
    }
}

fn cross_vertices(position: BlockPosition) -> [[[f32; 3]; 4]; 4] {
    let x = position.x as f32;
    let y = position.y as f32;
    let z = position.z as f32;
    [
        [
            [x, y, z],
            [x + 1.0, y, z + 1.0],
            [x + 1.0, y + 1.0, z + 1.0],
            [x, y + 1.0, z],
        ],
        [
            [x + 1.0, y, z + 1.0],
            [x, y, z],
            [x, y + 1.0, z],
            [x + 1.0, y + 1.0, z + 1.0],
        ],
        [
            [x + 1.0, y, z],
            [x, y, z + 1.0],
            [x, y + 1.0, z + 1.0],
            [x + 1.0, y + 1.0, z],
        ],
        [
            [x, y, z + 1.0],
            [x + 1.0, y, z],
            [x + 1.0, y + 1.0, z],
            [x, y + 1.0, z + 1.0],
        ],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::bootstrap_content;
    use crate::engine::world::{Chunk, ChunkPosition};

    #[test]
    fn single_block_mesh_has_six_faces() {
        let content = bootstrap_content().unwrap();
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();

        let mesh = ChunkMesher.mesh_chunk(&chunk, &content.blocks);

        assert_eq!(mesh.quads.len(), 6);
        assert_eq!(mesh.vertex_count(), 24);
        assert_eq!(mesh.triangle_count(), 12);
    }

    #[test]
    fn adjacent_solid_blocks_hide_shared_faces() {
        let content = bootstrap_content().unwrap();
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(BlockPosition { x: 1, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();
        chunk
            .set_block(BlockPosition { x: 2, y: 1, z: 1 }, content.block_ids.stone)
            .unwrap();

        let mesh = ChunkMesher.mesh_chunk(&chunk, &content.blocks);

        assert_eq!(mesh.quads.len(), 10);
    }

    #[test]
    fn face_vertices_are_counter_clockwise_from_outside() {
        let position = BlockPosition { x: 1, y: 2, z: 3 };
        let cases = [
            (FaceDirection::North, [0.0, 0.0, -1.0]),
            (FaceDirection::South, [0.0, 0.0, 1.0]),
            (FaceDirection::East, [1.0, 0.0, 0.0]),
            (FaceDirection::West, [-1.0, 0.0, 0.0]),
            (FaceDirection::Up, [0.0, 1.0, 0.0]),
            (FaceDirection::Down, [0.0, -1.0, 0.0]),
        ];

        for (direction, expected_normal) in cases {
            let vertices = face_vertices(position, direction);
            let normal = triangle_normal(vertices[0], vertices[1], vertices[2]);

            assert_eq!(
                normal, expected_normal,
                "{direction:?} face winding changed"
            );
        }
    }

    #[test]
    fn outside_opaque_neighbor_hides_chunk_border_face() {
        let content = bootstrap_content().unwrap();
        let mut chunk = Chunk::filled(ChunkPosition { x: 0, z: 0 }, content.block_ids.air);
        chunk
            .set_block(
                BlockPosition {
                    x: CHUNK_SIZE - 1,
                    y: 1,
                    z: 1,
                },
                content.block_ids.stone,
            )
            .unwrap();

        let mesh =
            ChunkMesher.mesh_chunk_with_neighbor_lookup(&chunk, &content.blocks, |_, direction| {
                (direction == FaceDirection::East).then_some(content.block_ids.stone)
            });

        assert_eq!(mesh.quads.len(), 5);
        assert!(
            !mesh
                .quads
                .iter()
                .any(|quad| quad.direction == FaceDirection::East)
        );
    }

    fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];

        [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ]
    }
}
