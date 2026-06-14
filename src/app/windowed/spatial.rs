use glam::Vec3;

use crate::engine::mesh::chunk_mesher::FaceDirection;
use crate::engine::world::{BlockPosition, CHUNK_HEIGHT, CHUNK_SIZE, ChunkPosition};

use super::constants::{PLAYER_HEIGHT, PLAYER_RADIUS, PLAYER_STANDING_EYE_HEIGHT};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct WorldBlockPosition {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) z: i32,
}

pub(super) fn world_block_from_render(position: Vec3) -> WorldBlockPosition {
    WorldBlockPosition {
        x: render_x_to_block_world(position.x).floor() as i32,
        y: render_y_to_block_world(position.y).floor() as i32,
        z: render_z_to_block_world(position.z).floor() as i32,
    }
}

pub(super) fn render_position_for_world_block_center(position: WorldBlockPosition) -> Vec3 {
    Vec3::new(
        position.x as f32 - 7.5,
        position.y as f32 - 63.5,
        position.z as f32 - 7.5,
    )
}

pub(super) fn chunk_position_for_render_position(position: Vec3) -> ChunkPosition {
    chunk_position_for_world_xz(
        render_x_to_block_world(position.x).floor() as i32,
        render_z_to_block_world(position.z).floor() as i32,
    )
}

fn chunk_position_for_world_xz(x: i32, z: i32) -> ChunkPosition {
    ChunkPosition {
        x: x.div_euclid(CHUNK_SIZE as i32),
        z: z.div_euclid(CHUNK_SIZE as i32),
    }
}

pub(super) fn world_block_position_from_chunk_position(
    chunk_position: ChunkPosition,
    block_position: BlockPosition,
) -> WorldBlockPosition {
    WorldBlockPosition {
        x: chunk_position.x * CHUNK_SIZE as i32 + block_position.x as i32,
        y: block_position.y as i32,
        z: chunk_position.z * CHUNK_SIZE as i32 + block_position.z as i32,
    }
}

pub(super) fn neighbor_world_block_position(
    position: WorldBlockPosition,
    direction: FaceDirection,
) -> WorldBlockPosition {
    match direction {
        FaceDirection::North => WorldBlockPosition {
            z: position.z - 1,
            ..position
        },
        FaceDirection::South => WorldBlockPosition {
            z: position.z + 1,
            ..position
        },
        FaceDirection::East => WorldBlockPosition {
            x: position.x + 1,
            ..position
        },
        FaceDirection::West => WorldBlockPosition {
            x: position.x - 1,
            ..position
        },
        FaceDirection::Up => WorldBlockPosition {
            y: position.y + 1,
            ..position
        },
        FaceDirection::Down => WorldBlockPosition {
            y: position.y - 1,
            ..position
        },
    }
}

pub(super) fn render_x_to_block_world(render_x: f32) -> f32 {
    render_x + 8.0
}

pub(super) fn render_y_to_block_world(render_y: f32) -> f32 {
    render_y + 64.0
}

pub(super) fn render_z_to_block_world(render_z: f32) -> f32 {
    render_z + 8.0
}

pub(super) fn player_aabb(eye_position: Vec3) -> (Vec3, Vec3) {
    player_aabb_at_eye_height(eye_position, PLAYER_STANDING_EYE_HEIGHT)
}

pub(super) fn player_aabb_at_eye_height(eye_position: Vec3, eye_height: f32) -> (Vec3, Vec3) {
    (
        Vec3::new(
            eye_position.x - PLAYER_RADIUS,
            eye_position.y - eye_height,
            eye_position.z - PLAYER_RADIUS,
        ),
        Vec3::new(
            eye_position.x + PLAYER_RADIUS,
            eye_position.y - eye_height + PLAYER_HEIGHT,
            eye_position.z + PLAYER_RADIUS,
        ),
    )
}

pub(super) fn aabb_intersects(
    left_min: Vec3,
    left_max: Vec3,
    right_min: Vec3,
    right_max: Vec3,
) -> bool {
    left_min.x < right_max.x
        && left_max.x > right_min.x
        && left_min.y < right_max.y
        && left_max.y > right_min.y
        && left_min.z < right_max.z
        && left_max.z > right_min.z
}

pub(super) fn split_world_block_position(
    position: WorldBlockPosition,
) -> Option<(ChunkPosition, BlockPosition)> {
    if position.y < 0 || position.y >= CHUNK_HEIGHT as i32 {
        return None;
    }

    let chunk_position = chunk_position_for_world_xz(position.x, position.z);
    let block_position = BlockPosition {
        x: position.x.rem_euclid(CHUNK_SIZE as i32) as usize,
        y: position.y as usize,
        z: position.z.rem_euclid(CHUNK_SIZE as i32) as usize,
    };

    Some((chunk_position, block_position))
}

pub(super) fn horizontal_neighbor_chunk_positions(
    chunk_position: ChunkPosition,
) -> [ChunkPosition; 4] {
    [
        ChunkPosition {
            x: chunk_position.x - 1,
            z: chunk_position.z,
        },
        ChunkPosition {
            x: chunk_position.x + 1,
            z: chunk_position.z,
        },
        ChunkPosition {
            x: chunk_position.x,
            z: chunk_position.z - 1,
        },
        ChunkPosition {
            x: chunk_position.x,
            z: chunk_position.z + 1,
        },
    ]
}

pub(super) fn dirty_horizontal_chunk_positions_for_block(
    chunk_position: ChunkPosition,
    block_position: BlockPosition,
) -> Vec<ChunkPosition> {
    let mut dirty = Vec::with_capacity(4);
    if block_position.x == 0 {
        dirty.push(ChunkPosition {
            x: chunk_position.x - 1,
            z: chunk_position.z,
        });
    }
    if block_position.x + 1 == CHUNK_SIZE {
        dirty.push(ChunkPosition {
            x: chunk_position.x + 1,
            z: chunk_position.z,
        });
    }
    if block_position.z == 0 {
        dirty.push(ChunkPosition {
            x: chunk_position.x,
            z: chunk_position.z - 1,
        });
    }
    if block_position.z + 1 == CHUNK_SIZE {
        dirty.push(ChunkPosition {
            x: chunk_position.x,
            z: chunk_position.z + 1,
        });
    }
    dirty
}
