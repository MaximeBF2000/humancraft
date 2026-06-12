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
    BlockId, BlockPosition, BlockRegistry, CHUNK_HEIGHT, CHUNK_SIZE, Chunk,
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
        let mut mesh = ChunkMesh::default();

        for y in 0..CHUNK_HEIGHT {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let position = BlockPosition { x, y, z };
                    let Some(block) = chunk.block(position) else {
                        continue;
                    };
                    if !is_visible_block(block, blocks) {
                        continue;
                    }

                    for direction in [
                        FaceDirection::North,
                        FaceDirection::South,
                        FaceDirection::East,
                        FaceDirection::West,
                        FaceDirection::Up,
                        FaceDirection::Down,
                    ] {
                        if self.face_is_exposed(chunk, blocks, position, direction) {
                            mesh.quads.push(MeshQuad {
                                block,
                                direction,
                                vertices: face_vertices(position, direction),
                            });
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
    ) -> bool {
        let neighbor = neighbor_position(position, direction);
        let Some(neighbor_position) = neighbor else {
            return true;
        };

        match chunk.block(neighbor_position) {
            Some(block) => !is_occluding_block(block, blocks),
            None => true,
        }
    }
}

fn is_visible_block(block: BlockId, blocks: &BlockRegistry) -> bool {
    blocks
        .get(block)
        .map(|definition| definition.solid || !definition.transparent)
        .unwrap_or(false)
}

fn is_occluding_block(block: BlockId, blocks: &BlockRegistry) -> bool {
    blocks
        .get(block)
        .map(|definition| definition.solid && !definition.transparent)
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
    let x = position.x as f32;
    let y = position.y as f32;
    let z = position.z as f32;
    let x1 = x + 1.0;
    let y1 = y + 1.0;
    let z1 = z + 1.0;

    match direction {
        FaceDirection::North => [[x, y, z], [x1, y, z], [x1, y1, z], [x, y1, z]],
        FaceDirection::South => [[x1, y, z1], [x, y, z1], [x, y1, z1], [x1, y1, z1]],
        FaceDirection::East => [[x1, y, z], [x1, y, z1], [x1, y1, z1], [x1, y1, z]],
        FaceDirection::West => [[x, y, z1], [x, y, z], [x, y1, z], [x, y1, z1]],
        FaceDirection::Up => [[x, y1, z], [x1, y1, z], [x1, y1, z1], [x, y1, z1]],
        FaceDirection::Down => [[x, y, z1], [x1, y, z1], [x1, y, z], [x, y, z]],
    }
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
}
