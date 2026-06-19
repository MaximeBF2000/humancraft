use std::collections::{HashMap, HashSet};

use wgpu::util::DeviceExt;

use crate::engine::mesh::chunk_mesher::{ChunkMesh, MeshQuad};
use crate::engine::world::{BlockRegistry, ChunkPosition};

use super::client_world::ClientWorld;
use super::constants::CHUNK_WORLD_SIZE;
use super::render_types::Vertex;
use super::texture::{TextureAtlas, render_material};

pub(super) struct ChunkRenderBuffer {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) index_buffer: wgpu::Buffer,
    pub(super) index_count: u32,
}

impl ChunkRenderBuffer {
    pub(super) fn new(
        device: &wgpu::Device,
        chunk_position: ChunkPosition,
        vertices: &[Vertex],
        indices: &[u32],
    ) -> Self {
        let vertex_label = format!("Chunk Vertex Buffer {chunk_position:?}");
        let index_label = format!("Chunk Index Buffer {chunk_position:?}");
        let vertex_buffer = if vertices.is_empty() {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&vertex_label),
                size: 4,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&vertex_label),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        };
        let index_buffer = if indices.is_empty() {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&index_label),
                size: 4,
                usage: wgpu::BufferUsages::INDEX,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&index_label),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            })
        };

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

pub(super) fn build_chunk_render_buffers(
    device: &wgpu::Device,
    world: &ClientWorld,
    texture_atlas: &TextureAtlas,
    chunk_positions: &[ChunkPosition],
) -> HashMap<ChunkPosition, ChunkRenderBuffer> {
    let mut buffers = HashMap::new();
    for chunk_position in unique_loaded_chunk_positions(chunk_positions, world) {
        if let Some((vertices, indices)) =
            world.build_chunk_render_mesh(chunk_position, texture_atlas)
        {
            buffers.insert(
                chunk_position,
                ChunkRenderBuffer::new(device, chunk_position, &vertices, &indices),
            );
        }
    }
    buffers
}

pub(super) fn unique_loaded_chunk_positions(
    chunk_positions: &[ChunkPosition],
    world: &ClientWorld,
) -> Vec<ChunkPosition> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for chunk_position in chunk_positions {
        if world.chunks.contains_key(chunk_position) && seen.insert(*chunk_position) {
            unique.push(*chunk_position);
        }
    }
    unique
}

pub(super) fn build_render_mesh(
    chunk_meshes: &[(ChunkPosition, ChunkMesh)],
    blocks: &BlockRegistry,
    texture_atlas: &TextureAtlas,
    render_bounds: Option<RenderChunkBounds>,
) -> (Vec<Vertex>, Vec<u32>) {
    let quad_count = chunk_meshes
        .iter()
        .map(|(_, mesh)| mesh.quads.len())
        .sum::<usize>();
    let mut vertices = Vec::with_capacity(quad_count * 4);
    let mut indices = Vec::with_capacity(quad_count * 6);

    for (chunk_position, mesh) in chunk_meshes {
        let chunk_offset_x = chunk_position.x as f32 * 16.0;
        let chunk_offset_z = chunk_position.z as f32 * 16.0;
        for quad in &mesh.quads {
            if !should_render_preview_quad(quad, *chunk_position, render_bounds) {
                continue;
            }

            let base = vertices.len() as u32;
            let (color, texture_key) =
                render_material(quad.block, quad.state, quad.direction, blocks);
            let tex_coords = texture_atlas.tile(&texture_key).uv_quad();
            for (index, vertex) in quad.vertices.into_iter().enumerate() {
                vertices.push(Vertex {
                    position: [
                        vertex[0] + chunk_offset_x - 8.0,
                        vertex[1] - 64.0,
                        vertex[2] + chunk_offset_z - 8.0,
                    ],
                    color,
                    tex_coords: tex_coords[index],
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    (vertices, indices)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) struct RenderChunkBounds {
    pub(super) min_x: i32,
    pub(super) max_x: i32,
    pub(super) min_z: i32,
    pub(super) max_z: i32,
}

impl RenderChunkBounds {
    pub(super) fn from_chunk_positions(
        positions: impl IntoIterator<Item = ChunkPosition>,
    ) -> Option<Self> {
        let mut positions = positions.into_iter();
        let first = positions.next()?;
        let mut bounds = Self {
            min_x: first.x,
            max_x: first.x,
            min_z: first.z,
            max_z: first.z,
        };

        for position in positions {
            bounds.min_x = bounds.min_x.min(position.x);
            bounds.max_x = bounds.max_x.max(position.x);
            bounds.min_z = bounds.min_z.min(position.z);
            bounds.max_z = bounds.max_z.max(position.z);
        }

        Some(bounds)
    }
}

pub(super) fn should_render_preview_quad(
    quad: &MeshQuad,
    chunk_position: ChunkPosition,
    render_bounds: Option<RenderChunkBounds>,
) -> bool {
    if is_outer_render_boundary(quad, chunk_position, render_bounds) {
        return false;
    }

    true
}

fn is_outer_render_boundary(
    quad: &MeshQuad,
    chunk_position: ChunkPosition,
    render_bounds: Option<RenderChunkBounds>,
) -> bool {
    use crate::engine::mesh::chunk_mesher::FaceDirection;

    let Some(render_bounds) = render_bounds else {
        return false;
    };

    let offset_x = chunk_position.x as f32 * CHUNK_WORLD_SIZE;
    let offset_z = chunk_position.z as f32 * CHUNK_WORLD_SIZE;
    let render_min_world_x = render_bounds.min_x as f32 * CHUNK_WORLD_SIZE;
    let render_max_world_x = (render_bounds.max_x + 1) as f32 * CHUNK_WORLD_SIZE;
    let render_min_world_z = render_bounds.min_z as f32 * CHUNK_WORLD_SIZE;
    let render_max_world_z = (render_bounds.max_z + 1) as f32 * CHUNK_WORLD_SIZE;
    match quad.direction {
        FaceDirection::West => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[0] + offset_x, render_min_world_x)),
        FaceDirection::East => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[0] + offset_x, render_max_world_x)),
        FaceDirection::North => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[2] + offset_z, render_min_world_z)),
        FaceDirection::South => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[2] + offset_z, render_max_world_z)),
        FaceDirection::Up => false,
        FaceDirection::Down => quad
            .vertices
            .iter()
            .all(|vertex| nearly_equal(vertex[1], 0.0)),
    }
}

fn nearly_equal(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.001
}
