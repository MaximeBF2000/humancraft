use glam::Vec3;

use super::render_types::Vertex;
use super::spatial::WorldBlockPosition;

pub(super) fn build_crosshair_mesh(width: u32, height: u32) -> (Vec<Vertex>, Vec<u32>) {
    let color = [0.96, 0.96, 0.96];
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let thickness = 0.0035;
    let vertical_length = 0.032;
    let horizontal_length = vertical_length / aspect;
    let vertices = vec![
        Vertex {
            position: [-horizontal_length, -thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [horizontal_length, -thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [horizontal_length, thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-horizontal_length, thickness, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-thickness, -vertical_length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [thickness, -vertical_length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [thickness, vertical_length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-thickness, vertical_length, 0.0],
            color,
            tex_coords: [0.0, 0.0],
        },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    (vertices, indices)
}

pub(super) fn build_outline_vertices(block: WorldBlockPosition) -> Vec<Vertex> {
    let color = [1.0, 0.92, 0.18];
    let min = Vec3::new(
        block.x as f32 - 8.0 - 0.025,
        block.y as f32 - 64.0 - 0.025,
        block.z as f32 - 8.0 - 0.025,
    );
    let max = min + Vec3::splat(1.05);
    let corners = [
        [min.x, min.y, min.z],
        [max.x, min.y, min.z],
        [max.x, max.y, min.z],
        [min.x, max.y, min.z],
        [min.x, min.y, max.z],
        [max.x, min.y, max.z],
        [max.x, max.y, max.z],
        [min.x, max.y, max.z],
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    edges
        .into_iter()
        .flat_map(|(a, b)| {
            [
                Vertex {
                    position: corners[a],
                    color,
                    tex_coords: [0.0, 0.0],
                },
                Vertex {
                    position: corners[b],
                    color,
                    tex_coords: [0.0, 0.0],
                },
            ]
        })
        .collect()
}
