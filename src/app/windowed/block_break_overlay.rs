use glam::Vec3;

use super::render_types::Vertex;
use super::spatial::WorldBlockPosition;

pub(super) struct BlockBreakOverlayMesh {
    pub(super) vertices: Vec<Vertex>,
    pub(super) indices: Vec<u32>,
}

pub(super) fn build_block_break_overlay_mesh(
    block: WorldBlockPosition,
    progress_ratio: f32,
) -> BlockBreakOverlayMesh {
    let stage = block_break_stage(progress_ratio);
    let mut mesh = BlockBreakOverlayMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    if stage == 0 {
        return mesh;
    }

    let min = Vec3::new(
        block.x as f32 - 8.0,
        block.y as f32 - 64.0,
        block.z as f32 - 8.0,
    );
    let max = min + Vec3::splat(1.0);

    add_break_face(
        &mut mesh,
        Vec3::new(min.x, min.y, max.z + 0.026),
        Vec3::X,
        Vec3::Y,
        stage,
    );
    add_break_face(
        &mut mesh,
        Vec3::new(max.x, min.y, min.z - 0.026),
        -Vec3::X,
        Vec3::Y,
        stage,
    );
    add_break_face(
        &mut mesh,
        Vec3::new(max.x + 0.026, min.y, max.z),
        -Vec3::Z,
        Vec3::Y,
        stage,
    );
    add_break_face(
        &mut mesh,
        Vec3::new(min.x - 0.026, min.y, min.z),
        Vec3::Z,
        Vec3::Y,
        stage,
    );
    add_break_face(
        &mut mesh,
        Vec3::new(min.x, max.y + 0.026, max.z),
        Vec3::X,
        -Vec3::Z,
        stage,
    );
    add_break_face(
        &mut mesh,
        Vec3::new(min.x, min.y - 0.026, min.z),
        Vec3::X,
        Vec3::Z,
        stage,
    );

    mesh
}

fn block_break_stage(progress_ratio: f32) -> u8 {
    let ratio = progress_ratio.clamp(0.0, 1.0);
    if ratio <= 0.0 {
        0
    } else {
        (ratio * 10.0).ceil().clamp(1.0, 10.0) as u8
    }
}

fn add_break_face(
    mesh: &mut BlockBreakOverlayMesh,
    origin: Vec3,
    axis_u: Vec3,
    axis_v: Vec3,
    stage: u8,
) {
    for (row, pattern_row) in BREAK_PATTERN.iter().enumerate() {
        for (column, pixel) in pattern_row.bytes().enumerate() {
            let Some(pixel_stage) = pixel_stage(pixel) else {
                continue;
            };
            if pixel_stage > stage {
                continue;
            }
            add_pixel(mesh, origin, axis_u, axis_v, column, row);
        }
    }
}

fn pixel_stage(pixel: u8) -> Option<u8> {
    match pixel {
        b'1'..=b'9' => Some(pixel - b'0'),
        b'A' => Some(10),
        _ => None,
    }
}

fn add_pixel(
    mesh: &mut BlockBreakOverlayMesh,
    origin: Vec3,
    axis_u: Vec3,
    axis_v: Vec3,
    column: usize,
    row: usize,
) {
    let pixel = 1.0 / 16.0;
    let u0 = column as f32 * pixel;
    let u1 = u0 + pixel;
    let v1 = 1.0 - row as f32 * pixel;
    let v0 = v1 - pixel;
    let color = [0.035, 0.032, 0.03];
    let base = mesh.vertices.len() as u32;
    let positions = [
        origin + axis_u * u0 + axis_v * v0,
        origin + axis_u * u1 + axis_v * v0,
        origin + axis_u * u1 + axis_v * v1,
        origin + axis_u * u0 + axis_v * v1,
    ];

    for position in positions {
        mesh.vertices.push(Vertex {
            position: position.to_array(),
            color,
            tex_coords: [0.0, 0.0],
        });
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

const BREAK_PATTERN: [&str; 16] = [
    "................",
    "..........AA....",
    ".........99A....",
    "........889.....",
    "...7...778......",
    "...67.667.......",
    "....6566........",
    "....455.........",
    "...3445.........",
    "..2334..........",
    "..1223.....8....",
    "..112.....789...",
    ".........6789...",
    "........56......",
    "........4.......",
    "................",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_overlay_uses_staged_pixel_quads() {
        let block = WorldBlockPosition { x: 8, y: 64, z: 8 };

        assert!(
            build_block_break_overlay_mesh(block, 0.0)
                .vertices
                .is_empty()
        );

        let early = build_block_break_overlay_mesh(block, 0.01);
        let late = build_block_break_overlay_mesh(block, 1.0);

        assert_eq!(early.vertices.len() % 4, 0);
        assert_eq!(early.indices.len() % 6, 0);
        assert!(late.vertices.len() > early.vertices.len());
        assert!(late.indices.len() > early.indices.len());
    }
}
