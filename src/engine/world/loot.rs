//! Dropped item entities.
//!
//! Purpose:
//! Represent collectable item stacks in the world without tying loot behavior
//! to blocks, rendering, or a future entity framework.

use glam::Vec3;

use crate::engine::world::item::ItemStack;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LootEntity {
    pub stack: ItemStack,
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation_radians: f32,
}

impl LootEntity {
    pub fn new(stack: ItemStack, position: Vec3) -> Self {
        Self {
            stack,
            position,
            velocity: Vec3::ZERO,
            rotation_radians: 0.0,
        }
    }

    pub fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }
}
