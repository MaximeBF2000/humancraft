use glam::Vec3;

use super::client_world::ClientWorld;
use super::constants::{PLAYER_STANDING_EYE_HEIGHT, SNEAK_EDGE_PROBE_DEPTH};
use super::spatial::{
    WorldBlockPosition, aabb_intersects, player_aabb, player_aabb_at_eye_height,
    render_x_to_block_world, render_y_to_block_world, render_z_to_block_world,
};

impl ClientWorld {
    pub(super) fn safe_spawn_eye_position(&self, preferred_position: Vec3) -> Vec3 {
        for radius in 0_i32..=8 {
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dz.abs() != radius {
                        continue;
                    }

                    let candidate_x = preferred_position.x + dx as f32;
                    let candidate_z = preferred_position.z + dz as f32;
                    let Some(eye_y) = self.ground_eye_y(candidate_x, candidate_z) else {
                        continue;
                    };
                    let candidate = Vec3::new(candidate_x, eye_y, candidate_z);
                    if !self.collides_player_at(candidate) {
                        return candidate;
                    }
                }
            }
        }

        let fallback = Vec3::new(
            preferred_position.x,
            preferred_position.y + 96.0,
            preferred_position.z,
        );
        self.first_clear_eye_position_above(fallback)
            .unwrap_or(fallback)
    }

    fn ground_eye_y(&self, render_x: f32, render_z: f32) -> Option<f32> {
        self.surface_block_y_at(render_x, render_z)
            .and_then(|block_y| {
                self.first_clear_eye_position_above_world_y(render_x, render_z, block_y + 1)
            })
    }

    fn first_clear_eye_position_above(&self, start: Vec3) -> Option<Vec3> {
        let world_start_y = render_y_to_block_world(start.y).floor() as i32;
        self.first_clear_eye_position_above_world_y(start.x, start.z, world_start_y)
            .map(|eye_y| Vec3::new(start.x, eye_y, start.z))
    }

    fn first_clear_eye_position_above_world_y(
        &self,
        render_x: f32,
        render_z: f32,
        world_start_y: i32,
    ) -> Option<f32> {
        for feet_y in world_start_y.max(0)..crate::engine::world::CHUNK_HEIGHT as i32 {
            let eye_y = feet_y as f32 - 64.0 + PLAYER_STANDING_EYE_HEIGHT + 0.05;
            if !self.collides_player_at(Vec3::new(render_x, eye_y, render_z)) {
                return Some(eye_y);
            }
        }

        None
    }

    pub(super) fn collides_player_at(&self, eye_position: Vec3) -> bool {
        self.collides_player_at_eye_height(eye_position, PLAYER_STANDING_EYE_HEIGHT)
    }

    pub(super) fn collides_player_at_eye_height(
        &self,
        eye_position: Vec3,
        eye_height: f32,
    ) -> bool {
        let (min, max) = player_aabb_at_eye_height(eye_position, eye_height);
        self.collides_aabb(min, max)
    }

    pub(super) fn has_player_ground_support(&self, eye_position: Vec3, eye_height: f32) -> bool {
        let (min, max) = player_aabb_at_eye_height(eye_position, eye_height);
        let probe_min = Vec3::new(min.x, min.y - SNEAK_EDGE_PROBE_DEPTH, min.z);
        let probe_max = Vec3::new(max.x, min.y, max.z);
        self.collides_aabb(probe_min, probe_max)
    }

    pub(super) fn block_intersects_player(
        &self,
        position: WorldBlockPosition,
        player_eye_position: Vec3,
    ) -> bool {
        let block_min = Vec3::new(
            position.x as f32 - 8.0,
            position.y as f32 - 64.0,
            position.z as f32 - 8.0,
        );
        let block_max = block_min + Vec3::splat(1.0);
        let (player_min, player_max) = player_aabb(player_eye_position);

        aabb_intersects(block_min, block_max, player_min, player_max)
    }

    pub(super) fn collides_aabb(&self, min: Vec3, max: Vec3) -> bool {
        let epsilon = 0.001;
        let min_x = render_x_to_block_world(min.x).floor() as i32;
        let max_x = render_x_to_block_world(max.x - epsilon).floor() as i32;
        let min_y = render_y_to_block_world(min.y).floor() as i32;
        let max_y = render_y_to_block_world(max.y - epsilon).floor() as i32;
        let min_z = render_z_to_block_world(min.z).floor() as i32;
        let max_z = render_z_to_block_world(max.z - epsilon).floor() as i32;

        for y in min_y..=max_y {
            for z in min_z..=max_z {
                for x in min_x..=max_x {
                    if self.is_solid(WorldBlockPosition { x, y, z }) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn surface_block_y_at(&self, render_x: f32, render_z: f32) -> Option<i32> {
        let world_x = render_x_to_block_world(render_x).floor() as i32;
        let world_z = render_z_to_block_world(render_z).floor() as i32;
        (0..crate::engine::world::CHUNK_HEIGHT as i32)
            .rev()
            .find(|y| {
                self.is_solid(WorldBlockPosition {
                    x: world_x,
                    y: *y,
                    z: world_z,
                })
            })
    }
}
