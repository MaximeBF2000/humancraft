use glam::{Mat4, Vec3};

use crate::engine::world::save::PlayerSave;

use super::client_world::ClientWorld;
use super::constants::*;
use super::input::InputState;

#[derive(Debug, Copy, Clone)]
pub(super) struct Camera {
    pub(super) position: Vec3,
    pub(super) yaw: f32,
    pub(super) pitch: f32,
    pub(super) horizontal_velocity: Vec3,
    pub(super) vertical_velocity: f32,
    pub(super) grounded: bool,
    sneaking: bool,
    sprinting: bool,
    physics_accumulator: f32,
}

impl Camera {
    pub(super) fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: -90.0_f32.to_radians(),
            pitch: -18.0_f32.to_radians(),
            horizontal_velocity: Vec3::ZERO,
            vertical_velocity: 0.0,
            grounded: false,
            sneaking: false,
            sprinting: false,
            physics_accumulator: 0.0,
        }
    }

    pub(super) fn from_save(save: PlayerSave) -> Self {
        let mut camera = Self::new(Vec3::new(save.eye_x, save.eye_y, save.eye_z));
        camera.yaw = save.yaw;
        camera.pitch = save.pitch;
        camera
    }

    pub(super) fn to_save(self) -> PlayerSave {
        PlayerSave::new(
            self.position.x,
            self.position.y,
            self.position.z,
            self.yaw,
            self.pitch,
        )
    }

    pub(super) fn update(&mut self, input: &InputState, world: &ClientWorld, delta_seconds: f32) {
        self.physics_accumulator += delta_seconds;
        while self.physics_accumulator >= PHYSICS_TICK_SECONDS {
            self.tick(input, world);
            self.physics_accumulator -= PHYSICS_TICK_SECONDS;
        }

        if self.position.y < -80.0 {
            self.position = world.safe_spawn_eye_position(Vec3::new(0.0, 0.0, 20.0));
            self.horizontal_velocity = Vec3::ZERO;
            self.vertical_velocity = 0.0;
            self.grounded = false;
            self.physics_accumulator = 0.0;
        }
    }

    fn tick(&mut self, input: &InputState, world: &ClientWorld) {
        self.update_sneaking(input.sneak, world);
        self.sprinting = input.sprint && input.forward && !input.sneak;

        let forward = self.forward();
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = flat_forward.cross(Vec3::Y).normalize_or_zero();
        let mut movement = Vec3::ZERO;

        if input.forward {
            movement += flat_forward;
        }
        if input.backward {
            movement -= flat_forward;
        }
        if input.right {
            movement += right;
        }
        if input.left {
            movement -= right;
        }

        if movement.length_squared() > 0.0 {
            let mut acceleration = if self.grounded {
                WALK_ACCELERATION
            } else {
                AIR_ACCELERATION
            };
            if self.sprinting {
                acceleration *= SPRINT_MULTIPLIER;
            }
            if input.sneak {
                acceleration *= SNEAK_MULTIPLIER;
            }
            self.horizontal_velocity += movement.normalize() * acceleration;
        }

        let jumped = input.jump && self.grounded;
        if jumped {
            self.vertical_velocity = JUMP_VELOCITY;
            if self.sprinting {
                self.horizontal_velocity += flat_forward * SPRINT_JUMP_BOOST;
            }
            self.grounded = false;
        }

        if !jumped {
            self.vertical_velocity -= GRAVITY_PER_TICK;
            self.vertical_velocity *= AIR_DRAG;
        }
        self.grounded = false;
        let vertical_delta = Vec3::new(0.0, self.vertical_velocity, 0.0);
        if !self.try_move(vertical_delta, world) {
            if vertical_delta.y < 0.0 {
                self.grounded = true;
            }
            self.vertical_velocity = 0.0;
        }
        if jumped && self.vertical_velocity != 0.0 {
            self.vertical_velocity -= GRAVITY_PER_TICK;
            self.vertical_velocity *= AIR_DRAG;
        }

        self.move_horizontally(world, input.sneak);
        self.horizontal_velocity *= if self.grounded {
            GROUND_FRICTION
        } else {
            AIR_HORIZONTAL_DRAG
        };
    }

    fn update_sneaking(&mut self, sneak: bool, world: &ClientWorld) {
        if self.sneaking == sneak {
            return;
        }

        let old_eye_height = self.eye_height();
        self.sneaking = sneak;
        let new_eye_height = self.eye_height();
        let adjusted = self.position + Vec3::Y * (new_eye_height - old_eye_height);
        if world.collides_player_at_eye_height(adjusted, new_eye_height) {
            self.sneaking = !sneak;
        } else {
            self.position = adjusted;
        }
    }

    fn move_horizontally(&mut self, world: &ClientWorld, sneak: bool) {
        let delta_x = Vec3::new(self.horizontal_velocity.x, 0.0, 0.0);
        if !self.try_horizontal_move(delta_x, world, sneak) && self.grounded {
            self.horizontal_velocity.x = 0.0;
        }

        let delta_z = Vec3::new(0.0, 0.0, self.horizontal_velocity.z);
        if !self.try_horizontal_move(delta_z, world, sneak) && self.grounded {
            self.horizontal_velocity.z = 0.0;
        }
    }

    fn try_horizontal_move(&mut self, delta: Vec3, world: &ClientWorld, sneak: bool) -> bool {
        if delta.length_squared() == 0.0 {
            return true;
        }

        let original = self.position;
        if self.try_move(delta, world)
            && (!sneak
                || !self.grounded
                || world.has_player_ground_support(self.position, self.eye_height()))
        {
            return true;
        }
        self.position = original;

        if !self.grounded || sneak {
            return false;
        }

        if self.try_move(Vec3::Y * STEP_HEIGHT, world) && self.try_move(delta, world) {
            self.try_move(Vec3::Y * -STEP_HEIGHT, world);
            return true;
        }

        self.position = original;
        false
    }

    fn try_move(&mut self, delta: Vec3, world: &ClientWorld) -> bool {
        if delta.length_squared() == 0.0 {
            return true;
        }

        let next = self.position + delta;
        if world.collides_player_at_eye_height(next, self.eye_height()) {
            return false;
        }

        self.position = next;
        true
    }

    pub(super) fn apply_mouse_delta(&mut self, delta_x: f32, delta_y: f32) {
        let sensitivity = 0.0025;
        self.yaw += delta_x * sensitivity;
        self.pitch = (self.pitch - delta_y * sensitivity).clamp(-1.553, 1.553);
    }

    pub(super) fn view_projection(&self, width: u32, height: u32) -> Mat4 {
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let view = Mat4::look_to_rh(self.position, self.forward(), Vec3::Y);
        let fov = if self.sprinting {
            SPRINT_FOV_DEGREES
        } else {
            NORMAL_FOV_DEGREES
        };
        let projection = Mat4::perspective_rh(fov.to_radians(), aspect, 0.1, 500.0);
        projection * view
    }

    pub(super) fn eye_height(&self) -> f32 {
        if self.sneaking {
            PLAYER_SNEAKING_EYE_HEIGHT
        } else {
            PLAYER_STANDING_EYE_HEIGHT
        }
    }

    pub(super) fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }
}
