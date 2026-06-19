use std::collections::{HashSet, VecDeque};

use crate::engine::mesh::chunk_mesher::FaceDirection;
use crate::engine::world::{
    BlockId, BlockProperties, BlockState, CHUNK_HEIGHT, CHUNK_SIZE, ChanceDrop, ChunkPosition,
    SaplingGrowthBehavior,
};

use super::client_world::ClientWorld;
use super::spatial::{
    WorldBlockPosition, neighbor_world_block_position, world_block_position_from_chunk_position,
};

const RANDOM_TICKS_PER_CHUNK: usize = 3;
// const FALLING_BLOCK_GRAVITY_PER_TICK: f32 = 0.04;
const FALLING_BLOCK_GRAVITY_PER_TICK: f32 = 0.15;
const FALLING_BLOCK_DRAG: f32 = 0.98;

impl ClientWorld {
    pub(super) fn tick_block_behaviors(&mut self) -> Vec<ChunkPosition> {
        self.random_tick_counter = self.random_tick_counter.wrapping_add(1);
        let mut dirty = self.tick_gravity_blocks();
        dirty.extend(self.tick_random_blocks(false));
        dirty
    }

    #[cfg(test)]
    pub(super) fn tick_all_block_behaviors_for_tests(&mut self) -> Vec<ChunkPosition> {
        self.random_tick_counter = self.random_tick_counter.wrapping_add(1);
        let mut dirty = self.tick_gravity_blocks();
        dirty.extend(self.tick_random_blocks(true));
        dirty
    }

    fn tick_random_blocks(&mut self, exhaustive: bool) -> Vec<ChunkPosition> {
        let positions = if exhaustive {
            self.all_loaded_block_positions()
        } else {
            self.random_tick_positions()
        };
        let mut dirty = Vec::new();
        for position in positions {
            dirty.extend(self.random_tick_block(position));
        }
        dirty
    }

    fn random_tick_block(&mut self, position: WorldBlockPosition) -> Vec<ChunkPosition> {
        let Some(state) = self.block_state(position) else {
            return Vec::new();
        };
        let Some(definition) = self.blocks.get(state.block) else {
            return Vec::new();
        };
        let leaf_decay = definition.behavior.leaf_decay.clone();
        let grass_spread = definition.behavior.grass_spread.clone();
        let sapling_growth = definition.behavior.sapling_growth.clone();

        if let Some(behavior) = leaf_decay
            && matches!(
                state.properties,
                BlockProperties::Leaves { persistent: false }
            )
            && !self.leaf_connected_to_log(
                position,
                state.block,
                &behavior.log_tag,
                behavior.max_distance,
            )
        {
            let mut dirty = Vec::new();
            self.drop_chance(position, &behavior.sapling_drop);
            dirty.extend(self.set_block(position, self.block_ids.air));
            return dirty;
        }

        if let Some(behavior) = grass_spread {
            return self.spread_grass_from(position, &behavior);
        }

        if let Some(behavior) = sapling_growth
            && let BlockProperties::Sapling { stage } = state.properties
        {
            if stage == 0 {
                let mut next = state;
                next.properties = BlockProperties::Sapling { stage: 1 };
                return self.set_block_state(position, next);
            }
            return self.grow_sapling(position, &behavior);
        }

        Vec::new()
    }

    fn tick_gravity_blocks(&mut self) -> Vec<ChunkPosition> {
        let mut positions: Vec<_> = self.pending_gravity_checks.drain().collect();
        positions.sort_by_key(|position| position.y);

        let mut dirty = Vec::new();
        for position in positions {
            if self
                .block(position)
                .and_then(|block| self.blocks.get(block))
                .is_some_and(|definition| definition.behavior.gravity)
            {
                dirty.extend(self.fall_block_one_cell(position));
            }
        }
        dirty
    }

    pub(super) fn enqueue_gravity_checks_after_block_update(
        &mut self,
        position: WorldBlockPosition,
    ) {
        self.pending_gravity_checks.insert(position);
        if position.y + 1 < CHUNK_HEIGHT as i32 {
            self.pending_gravity_checks.insert(WorldBlockPosition {
                y: position.y + 1,
                ..position
            });
        }
    }

    fn fall_block_one_cell(&mut self, position: WorldBlockPosition) -> Vec<ChunkPosition> {
        let Some(state) = self.block_state(position) else {
            self.falling_block_motion.remove(&position);
            return Vec::new();
        };
        let below = WorldBlockPosition {
            y: position.y - 1,
            ..position
        };
        if position.y <= 0 || !self.is_replaceable_for_falling_block(below) {
            self.falling_block_motion.remove(&position);
            return Vec::new();
        }
        let mut motion = self
            .falling_block_motion
            .remove(&position)
            .unwrap_or_default();
        motion.velocity_blocks_per_tick =
            (motion.velocity_blocks_per_tick + FALLING_BLOCK_GRAVITY_PER_TICK) * FALLING_BLOCK_DRAG;
        motion.fall_distance_blocks += motion.velocity_blocks_per_tick;
        if motion.fall_distance_blocks < 1.0 {
            self.falling_block_motion.insert(position, motion);
            self.pending_gravity_checks.insert(position);
            return Vec::new();
        }

        motion.fall_distance_blocks -= 1.0;
        let mut dirty = self.set_block(position, self.block_ids.air);
        dirty.extend(self.set_block_state(below, state));
        self.falling_block_motion.insert(below, motion);
        self.pending_gravity_checks.insert(below);
        dirty
    }

    fn is_replaceable_for_falling_block(&self, position: WorldBlockPosition) -> bool {
        self.block(position)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| !definition.solid || definition.has_tag("replaceable"))
            .unwrap_or(false)
    }

    fn spread_grass_from(
        &mut self,
        source: WorldBlockPosition,
        behavior: &crate::engine::world::GrassSpreadBehavior,
    ) -> Vec<ChunkPosition> {
        if !self.block_above_is_clear(source) {
            return Vec::new();
        }
        let Some(target_block) = self.blocks.id_for_key(&behavior.target_block_key) else {
            return Vec::new();
        };
        let mut dirty = Vec::new();
        for attempt in 0..behavior.attempts_per_random_tick {
            let dx = random_range(
                self.random_tick_counter,
                source,
                attempt as u64,
                -behavior.horizontal_range,
                behavior.horizontal_range,
            );
            let dy = random_range(
                self.random_tick_counter,
                source,
                64 + attempt as u64,
                -behavior.down_range,
                behavior.up_range,
            );
            let dz = random_range(
                self.random_tick_counter,
                source,
                128 + attempt as u64,
                -behavior.horizontal_range,
                behavior.horizontal_range,
            );
            let target = WorldBlockPosition {
                x: source.x + dx,
                y: source.y + dy,
                z: source.z + dz,
            };
            if self.block(target) == Some(target_block) && self.block_above_is_clear(target) {
                dirty.extend(self.set_block(target, self.block_ids.grass));
            }
        }
        dirty
    }

    fn block_above_is_clear(&self, position: WorldBlockPosition) -> bool {
        let above = WorldBlockPosition {
            y: position.y + 1,
            ..position
        };
        self.block(above)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| !definition.solid)
            .unwrap_or(true)
    }

    fn grow_sapling(
        &mut self,
        position: WorldBlockPosition,
        behavior: &SaplingGrowthBehavior,
    ) -> Vec<ChunkPosition> {
        if !self.sapling_has_valid_soil(position, behavior)
            || !self.sapling_space_is_clear(position, behavior)
        {
            return Vec::new();
        }
        let Some(trunk) = self.blocks.id_for_key(&behavior.trunk_block_key) else {
            return Vec::new();
        };
        let Some(leaves) = self.blocks.id_for_key(&behavior.leaves_block_key) else {
            return Vec::new();
        };
        let height = random_usize(
            self.random_tick_counter,
            position,
            behavior.min_trunk_height,
            behavior.max_trunk_height,
        );

        let mut dirty = self.set_block(position, trunk);
        for y in position.y + 1..position.y + height as i32 {
            dirty.extend(self.set_block(WorldBlockPosition { y, ..position }, trunk));
        }
        let leaf_center_y = position.y + height as i32 - 1;
        for y in leaf_center_y - 1..=leaf_center_y + behavior.canopy_radius as i32 {
            let vertical_distance = y.abs_diff(leaf_center_y);
            let layer_radius = behavior
                .canopy_radius
                .saturating_sub((vertical_distance / 2) as usize)
                as i32;
            for x in position.x - layer_radius..=position.x + layer_radius {
                for z in position.z - layer_radius..=position.z + layer_radius {
                    if (position.x - x).abs() + (position.z - z).abs() > layer_radius * 2 {
                        continue;
                    }
                    let leaf_position = WorldBlockPosition { x, y, z };
                    if self.block(leaf_position) == Some(trunk) {
                        continue;
                    }
                    if self.is_replaceable_for_tree(leaf_position) {
                        dirty.extend(self.set_block_state(
                            leaf_position,
                            BlockState::with_properties(
                                leaves,
                                BlockProperties::Leaves { persistent: false },
                            ),
                        ));
                    }
                }
            }
        }
        let soil = WorldBlockPosition {
            y: position.y - 1,
            ..position
        };
        if self.block(soil) == Some(self.block_ids.grass) {
            dirty.extend(self.set_block(soil, self.block_ids.dirt));
        }
        dirty
    }

    fn sapling_has_valid_soil(
        &self,
        position: WorldBlockPosition,
        behavior: &SaplingGrowthBehavior,
    ) -> bool {
        let soil = WorldBlockPosition {
            y: position.y - 1,
            ..position
        };
        self.block(soil)
            .and_then(|block| self.blocks.get(block))
            .is_some_and(|definition| {
                behavior
                    .grow_on_tags
                    .iter()
                    .any(|tag| definition.has_tag(tag))
            })
    }

    fn sapling_space_is_clear(
        &self,
        position: WorldBlockPosition,
        behavior: &SaplingGrowthBehavior,
    ) -> bool {
        if position.y + behavior.required_clearance as i32 >= CHUNK_HEIGHT as i32 {
            return false;
        }
        let radius = 1;
        for y in position.y + 1..=position.y + behavior.required_clearance as i32 {
            for x in position.x - radius..=position.x + radius {
                for z in position.z - radius..=position.z + radius {
                    if !self.is_replaceable_for_tree(WorldBlockPosition { x, y, z }) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn is_replaceable_for_tree(&self, position: WorldBlockPosition) -> bool {
        self.block(position)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| {
                !definition.solid
                    || definition.has_tag("replaceable")
                    || definition.has_tag("leaves")
            })
            .unwrap_or(false)
    }

    fn leaf_connected_to_log(
        &self,
        start: WorldBlockPosition,
        leaf_block: BlockId,
        log_tag: &str,
        max_distance: u8,
    ) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([(start, 0_u8)]);
        while let Some((position, distance)) = queue.pop_front() {
            if !visited.insert(position) {
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
                let neighbor = neighbor_world_block_position(position, direction);
                let Some(block) = self.block(neighbor) else {
                    continue;
                };
                if self
                    .blocks
                    .get(block)
                    .is_some_and(|definition| definition.has_tag(log_tag))
                {
                    return true;
                }
                if block == leaf_block && distance < max_distance {
                    queue.push_back((neighbor, distance + 1));
                }
            }
        }
        false
    }

    fn drop_chance(&mut self, position: WorldBlockPosition, drop: &ChanceDrop) {
        if chance_sample(self.random_tick_counter, position, 0x5A9D) > drop.chance {
            return;
        }
        if let Some(item) = self.items.id_for_key(&drop.item_key) {
            self.spawn_loot_stack(crate::engine::world::ItemStack::new(item, 1), position);
        }
    }

    fn random_tick_positions(&self) -> Vec<WorldBlockPosition> {
        let mut positions = Vec::new();
        for chunk_position in self.chunks.keys().copied() {
            for attempt in 0..RANDOM_TICKS_PER_CHUNK {
                let sample = hash4(
                    self.random_tick_counter,
                    chunk_position.x as u64,
                    chunk_position.z as u64,
                    attempt as u64,
                );
                positions.push(world_block_position_from_chunk_position(
                    chunk_position,
                    crate::engine::world::BlockPosition {
                        x: (sample & 15) as usize,
                        y: ((sample >> 4) % CHUNK_HEIGHT as u64) as usize,
                        z: ((sample >> 12) & 15) as usize,
                    },
                ));
            }
        }
        positions
    }

    fn all_loaded_block_positions(&self) -> Vec<WorldBlockPosition> {
        let mut positions = Vec::new();
        for chunk_position in self.chunks.keys().copied() {
            for y in 0..CHUNK_HEIGHT {
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        positions.push(world_block_position_from_chunk_position(
                            chunk_position,
                            crate::engine::world::BlockPosition { x, y, z },
                        ));
                    }
                }
            }
        }
        positions
    }
}

fn random_range(tick: u64, position: WorldBlockPosition, salt: u64, min: i32, max: i32) -> i32 {
    let span = (max - min + 1).max(1) as u64;
    min + (hash_position(tick, position, salt) % span) as i32
}

fn random_usize(tick: u64, position: WorldBlockPosition, min: usize, max: usize) -> usize {
    let span = max.saturating_sub(min) + 1;
    min + (hash_position(tick, position, 0x77EE) as usize % span)
}

fn chance_sample(tick: u64, position: WorldBlockPosition, salt: u64) -> f32 {
    let value = hash_position(tick, position, salt) >> 40;
    value as f32 / 0xFF_FFFF as f32
}

fn hash_position(tick: u64, position: WorldBlockPosition, salt: u64) -> u64 {
    hash4(
        tick ^ salt,
        position.x as u64,
        position.y as u64,
        position.z as u64,
    )
}

fn hash4(a: u64, b: u64, c: u64, d: u64) -> u64 {
    let mut x = a ^ b.rotate_left(17) ^ c.rotate_left(31) ^ d.rotate_left(47);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}
