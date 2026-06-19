use glam::Vec3;

use crate::engine::world::{BlockId, ItemId, ItemStack, LootEntity};

use super::client_world::ClientWorld;
use super::constants::{
    LOOT_AIR_DRAG, LOOT_GRAVITY_PER_TICK, LOOT_GROUND_DRAG, LOOT_PICKUP_RADIUS,
    LOOT_RENDER_HALF_SIZE, LOOT_ROTATION_RADIANS_PER_SECOND, PHYSICS_TICK_SECONDS,
    PLAYER_STANDING_EYE_HEIGHT,
};
use super::spatial::{WorldBlockPosition, render_position_for_world_block_center};

impl ClientWorld {
    pub(super) fn spawn_dropped_stack(&mut self, stack: ItemStack, position: Vec3, forward: Vec3) {
        if stack.is_empty() {
            return;
        }
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z)
            .try_normalize()
            .unwrap_or(Vec3::Z);
        let spawn_position = position + flat_forward * 0.45 - Vec3::Y * 0.35;
        let velocity = flat_forward * 0.12 + Vec3::Y * 0.16;
        self.loot_entities
            .push(LootEntity::new(stack, spawn_position).with_velocity(velocity));
    }

    pub(super) fn spawn_loot_for_block(&mut self, block: BlockId, position: WorldBlockPosition) {
        let Some(definition) = self.blocks.get(block) else {
            return;
        };
        let chance_drop = definition
            .behavior
            .leaf_decay
            .as_ref()
            .map(|behavior| behavior.sapling_drop.clone());
        let drops: Vec<_> = definition.drops.clone();
        for drop_key in drops {
            let Some(item) = self.items.id_for_key(&drop_key) else {
                eprintln!(
                    "block {} references unknown drop {drop_key}",
                    definition.key
                );
                continue;
            };
            let offset = loot_spawn_offset(position, item);
            let render_position = render_position_for_world_block_center(position) + offset;
            let velocity = Vec3::new(offset.x * 0.06, 0.12, offset.z * 0.06);
            self.loot_entities.push(
                LootEntity::new(ItemStack::new(item, 1), render_position).with_velocity(velocity),
            );
        }
        if let Some(drop) = chance_drop
            && chance_sample_for_drop(position, block) <= drop.chance
            && let Some(item) = self.items.id_for_key(&drop.item_key)
        {
            self.spawn_loot_stack(ItemStack::new(item, 1), position);
        }
    }

    pub(super) fn spawn_loot_stack(&mut self, stack: ItemStack, position: WorldBlockPosition) {
        if stack.is_empty() {
            return;
        }
        let offset = loot_spawn_offset(position, stack.item);
        let render_position = render_position_for_world_block_center(position) + offset;
        let velocity = Vec3::new(offset.x * 0.06, 0.12, offset.z * 0.06);
        self.loot_entities
            .push(LootEntity::new(stack, render_position).with_velocity(velocity));
    }

    pub(super) fn update_loot(&mut self, player_eye_position: Vec3, delta_seconds: f32) {
        let ticks = (delta_seconds / PHYSICS_TICK_SECONDS).clamp(0.0, 4.0);
        for index in 0..self.loot_entities.len() {
            let mut loot = self.loot_entities[index];
            loot.rotation_radians = (loot.rotation_radians
                + LOOT_ROTATION_RADIANS_PER_SECOND * delta_seconds)
                % std::f32::consts::TAU;
            loot.velocity.y -= LOOT_GRAVITY_PER_TICK * ticks;
            loot.velocity *= LOOT_AIR_DRAG.powf(ticks);

            let movement = loot.velocity * ticks;
            let next_x = loot.position + Vec3::new(movement.x, 0.0, 0.0);
            if self.collides_loot_at(next_x) {
                loot.velocity.x = 0.0;
            } else {
                loot.position = next_x;
            }

            let next_z = loot.position + Vec3::new(0.0, 0.0, movement.z);
            if self.collides_loot_at(next_z) {
                loot.velocity.z = 0.0;
            } else {
                loot.position = next_z;
            }

            let next_y = loot.position + Vec3::new(0.0, movement.y, 0.0);
            if self.collides_loot_at(next_y) {
                if loot.velocity.y < 0.0 {
                    loot.velocity.x *= LOOT_GROUND_DRAG;
                    loot.velocity.z *= LOOT_GROUND_DRAG;
                }
                loot.velocity.y = 0.0;
            } else {
                loot.position = next_y;
            }
            self.loot_entities[index] = loot;
        }

        let player_feet = Vec3::new(
            player_eye_position.x,
            player_eye_position.y - PLAYER_STANDING_EYE_HEIGHT,
            player_eye_position.z,
        );
        let mut index = 0;
        while index < self.loot_entities.len() {
            if self.loot_entities[index].position.distance(player_feet) > LOOT_PICKUP_RADIUS {
                index += 1;
                continue;
            }

            let remainder = self
                .player_inventory
                .add_stack(self.loot_entities[index].stack, &self.items);
            if let Some(stack) = remainder {
                self.loot_entities[index].stack = stack;
                index += 1;
            } else {
                self.loot_entities.swap_remove(index);
            }
        }
    }

    fn collides_loot_at(&self, position: Vec3) -> bool {
        let half = Vec3::new(0.12, 0.0, 0.12);
        let min = position - half;
        let max = position + Vec3::new(0.12, LOOT_RENDER_HALF_SIZE * 2.0, 0.12);
        self.collides_aabb(min, max)
    }
}

fn loot_spawn_offset(position: WorldBlockPosition, item: ItemId) -> Vec3 {
    let seed = position.x as i64 * 73_856_093
        ^ position.y as i64 * 19_349_663
        ^ position.z as i64 * 83_492_791
        ^ item.raw() as i64 * 2_654_435_761;
    let angle = (seed as f32).sin() * std::f32::consts::TAU;
    Vec3::new(angle.cos() * 0.12, -0.24, angle.sin() * 0.12)
}

fn chance_sample_for_drop(position: WorldBlockPosition, block: BlockId) -> f32 {
    let seed = position.x as i64 * 73_856_093
        ^ position.y as i64 * 19_349_663
        ^ position.z as i64 * 83_492_791
        ^ block.raw() as i64 * 2_654_435_761
        ^ 0x5eed_5eed;
    ((seed as f32).sin() * 0.5 + 0.5).clamp(0.0, 1.0)
}
