use glam::Vec3;

use super::render_types::Vertex;
use super::spatial::WorldBlockPosition;
use super::texture::{TextureAtlas, destroy_stage_texture_key};

pub(super) struct BlockBreakOverlayMesh {
    pub(super) vertices: Vec<Vertex>,
    pub(super) indices: Vec<u32>,
}

pub(super) fn build_block_break_overlay_mesh(
    block: WorldBlockPosition,
    progress_ratio: f32,
    texture_atlas: &TextureAtlas,
) -> BlockBreakOverlayMesh {
    let Some(stage) = block_break_stage(progress_ratio) else {
        return BlockBreakOverlayMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
    };
    let tile = texture_atlas.tile(&destroy_stage_texture_key(stage));
    let mut mesh = BlockBreakOverlayMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };

    let min = Vec3::new(
        block.x as f32 - 8.0,
        block.y as f32 - 64.0,
        block.z as f32 - 8.0,
    );
    let max = min + Vec3::splat(1.0);
    let offset = 0.026;

    add_break_face(
        &mut mesh,
        [
            Vec3::new(min.x, min.y, max.z + offset),
            Vec3::new(max.x, min.y, max.z + offset),
            Vec3::new(max.x, max.y, max.z + offset),
            Vec3::new(min.x, max.y, max.z + offset),
        ],
        tile.uv_quad(),
    );
    add_break_face(
        &mut mesh,
        [
            Vec3::new(max.x, min.y, min.z - offset),
            Vec3::new(min.x, min.y, min.z - offset),
            Vec3::new(min.x, max.y, min.z - offset),
            Vec3::new(max.x, max.y, min.z - offset),
        ],
        tile.uv_quad(),
    );
    add_break_face(
        &mut mesh,
        [
            Vec3::new(max.x + offset, min.y, max.z),
            Vec3::new(max.x + offset, min.y, min.z),
            Vec3::new(max.x + offset, max.y, min.z),
            Vec3::new(max.x + offset, max.y, max.z),
        ],
        tile.uv_quad(),
    );
    add_break_face(
        &mut mesh,
        [
            Vec3::new(min.x - offset, min.y, min.z),
            Vec3::new(min.x - offset, min.y, max.z),
            Vec3::new(min.x - offset, max.y, max.z),
            Vec3::new(min.x - offset, max.y, min.z),
        ],
        tile.uv_quad(),
    );
    add_break_face(
        &mut mesh,
        [
            Vec3::new(min.x, max.y + offset, max.z),
            Vec3::new(max.x, max.y + offset, max.z),
            Vec3::new(max.x, max.y + offset, min.z),
            Vec3::new(min.x, max.y + offset, min.z),
        ],
        tile.uv_quad(),
    );
    add_break_face(
        &mut mesh,
        [
            Vec3::new(min.x, min.y - offset, min.z),
            Vec3::new(max.x, min.y - offset, min.z),
            Vec3::new(max.x, min.y - offset, max.z),
            Vec3::new(min.x, min.y - offset, max.z),
        ],
        tile.uv_quad(),
    );

    mesh
}

fn block_break_stage(progress_ratio: f32) -> Option<u8> {
    let ratio = progress_ratio.clamp(0.0, 1.0);
    if ratio <= 0.0 {
        None
    } else {
        Some(((ratio * 10.0).ceil() as u8).saturating_sub(1).min(9))
    }
}

fn add_break_face(
    mesh: &mut BlockBreakOverlayMesh,
    positions: [Vec3; 4],
    tex_coords: [[f32; 2]; 4],
) {
    let base = mesh.vertices.len() as u32;
    for index in 0..4 {
        mesh.vertices.push(Vertex {
            position: positions[index].to_array(),
            color: [1.0, 1.0, 1.0],
            tex_coords: tex_coords[index],
        });
    }
    mesh.indices.extend_from_slice(&[
        base,
        base + 1,
        base + 2,
        base,
        base + 2,
        base + 3,
        base + 2,
        base + 1,
        base,
        base + 3,
        base + 2,
        base,
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_overlay_maps_progress_to_destroy_stage_indices() {
        assert_eq!(block_break_stage(0.0), None);
        assert_eq!(block_break_stage(0.01), Some(0));
        assert_eq!(block_break_stage(0.10), Some(0));
        assert_eq!(block_break_stage(0.11), Some(1));
        assert_eq!(block_break_stage(1.0), Some(9));
    }
}
