use std::collections::{HashMap, HashSet};

use glam::Vec3;

use crate::content::BlockIds;
use crate::engine::mesh::chunk_mesher::{ChunkMesh, ChunkMesher, FaceDirection};
use crate::engine::world::generation::{GenerationContext, GenerationPipeline};
use crate::engine::world::save::{
    BlockEntityKindSave, BlockEntityPositionSave, BlockEntitySave, InventorySave, ItemStackSave,
    WorldSaveStore,
};
use crate::engine::world::{
    Axis, BlockId, BlockProperties, BlockRegistry, BlockShape, BlockState, Chunk, ChunkPosition,
    CraftingRecipeRegistry, HorizontalDirection, Inventory, ItemId, ItemRegistry, ItemStack,
    ItemStackMetadata, LootEntity, PlacementRuleKind, SlabOrientation, SmeltingRecipeRegistry,
    StairHalf, ToolDefinition, smelting_result,
};

use super::constants::{
    CORRECT_TOOL_SECONDS_PER_HARDNESS, INEFFICIENT_BREAK_SECONDS_PER_HARDNESS,
    MIN_BLOCK_BREAK_SECONDS,
};
use super::render_types::Vertex;
use super::spatial::{
    WorldBlockPosition, chunk_position_for_render_position,
    dirty_horizontal_chunk_positions_for_block, horizontal_neighbor_chunk_positions,
    neighbor_world_block_position, render_y_to_block_world, split_world_block_position,
    world_block_from_render, world_block_position_from_chunk_position,
};
use super::texture::TextureAtlas;
use super::world_render::{RenderChunkBounds, build_render_mesh};

#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) struct RaycastHit {
    pub(super) block: WorldBlockPosition,
    pub(super) previous: WorldBlockPosition,
    pub(super) face: FaceDirection,
    pub(super) hit_position: Vec3,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) struct BlockBreakProgress {
    pub(super) target: WorldBlockPosition,
    pub(super) ratio: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct ActiveBlockBreak {
    target: WorldBlockPosition,
    elapsed_seconds: f32,
    required_seconds: f32,
}

const CHEST_SLOT_COUNT: usize = 27;
const FURNACE_SLOT_COUNT: usize = 3;
const FURNACE_INPUT_SLOT: usize = 0;
const FURNACE_FUEL_SLOT: usize = 1;
const FURNACE_OUTPUT_SLOT: usize = 2;

#[derive(Debug, Clone)]
pub(super) enum BlockEntity {
    Chest(Inventory),
    Furnace(FurnaceEntity),
}

#[derive(Debug, Clone)]
pub(super) struct FurnaceEntity {
    pub(super) inventory: Inventory,
    pub(super) burn_ticks: u32,
    pub(super) fuel_ticks: u32,
    pub(super) cook_ticks: u32,
    pub(super) cook_ticks_total: u32,
}

impl FurnaceEntity {
    fn new() -> Self {
        Self {
            inventory: Inventory::new(FURNACE_SLOT_COUNT, 0),
            burn_ticks: 0,
            fuel_ticks: 0,
            cook_ticks: 0,
            cook_ticks_total: 200,
        }
    }
}

pub(super) struct ClientWorld {
    pub(super) world_id: String,
    pub(super) chunks: HashMap<ChunkPosition, Chunk>,
    pub(super) blocks: BlockRegistry,
    pub(super) items: ItemRegistry,
    pub(super) recipes: CraftingRecipeRegistry,
    pub(super) smelting_recipes: SmeltingRecipeRegistry,
    pub(super) block_ids: BlockIds,
    pub(super) player_inventory: Inventory,
    pub(super) loot_entities: Vec<LootEntity>,
    pub(super) block_entities: HashMap<WorldBlockPosition, BlockEntity>,
    pub(super) chest_item_inventories: HashMap<u64, Inventory>,
    next_chest_item_inventory_id: u64,
    generation_pipeline: GenerationPipeline,
    generation_context: GenerationContext,
    render_distance_chunks: i32,
    mesher: ChunkMesher,
    active_block_break: Option<ActiveBlockBreak>,
    pub(super) random_tick_counter: u64,
    pub(super) pending_gravity_checks: HashSet<WorldBlockPosition>,
    pub(super) falling_block_motion: HashMap<WorldBlockPosition, FallingBlockMotion>,
}

#[derive(Debug, Copy, Clone, Default)]
pub(super) struct FallingBlockMotion {
    pub(super) velocity_blocks_per_tick: f32,
    pub(super) fall_distance_blocks: f32,
}

impl ClientWorld {
    pub(super) fn new(
        blocks: BlockRegistry,
        items: ItemRegistry,
        recipes: CraftingRecipeRegistry,
        smelting_recipes: SmeltingRecipeRegistry,
        block_ids: BlockIds,
        generation_pipeline: GenerationPipeline,
        generation_context: GenerationContext,
        render_distance_chunks: i32,
        world_id: String,
    ) -> Self {
        Self {
            world_id,
            chunks: HashMap::new(),
            blocks,
            items,
            recipes,
            smelting_recipes,
            block_ids,
            player_inventory: Inventory::player(),
            loot_entities: Vec::new(),
            block_entities: HashMap::new(),
            chest_item_inventories: HashMap::new(),
            next_chest_item_inventory_id: 1,
            generation_pipeline,
            generation_context,
            render_distance_chunks,
            mesher: ChunkMesher,
            active_block_break: None,
            random_tick_counter: 0,
            pending_gravity_checks: HashSet::new(),
            falling_block_motion: HashMap::new(),
        }
    }

    pub(super) fn insert_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert(chunk.position(), chunk);
    }

    #[cfg(test)]
    pub(super) fn ensure_chunks_around_render_position(
        &mut self,
        position: Vec3,
        max_new_chunks: usize,
    ) -> Vec<ChunkPosition> {
        self.ensure_chunks_around_render_position_with_saves(position, max_new_chunks, None)
    }

    pub(super) fn ensure_chunks_around_render_position_with_store(
        &mut self,
        position: Vec3,
        max_new_chunks: usize,
        save_store: &WorldSaveStore,
    ) -> Vec<ChunkPosition> {
        self.ensure_chunks_around_render_position_with_saves(
            position,
            max_new_chunks,
            Some(save_store),
        )
    }

    fn ensure_chunks_around_render_position_with_saves(
        &mut self,
        position: Vec3,
        max_new_chunks: usize,
        save_store: Option<&WorldSaveStore>,
    ) -> Vec<ChunkPosition> {
        let center = chunk_position_for_render_position(position);
        let mut dirty_chunks = HashSet::new();
        let mut missing_chunks = Vec::new();

        for z in center.z - self.render_distance_chunks..=center.z + self.render_distance_chunks {
            for x in center.x - self.render_distance_chunks..=center.x + self.render_distance_chunks
            {
                let chunk_position = ChunkPosition { x, z };
                if self.chunks.contains_key(&chunk_position) {
                    continue;
                }
                missing_chunks.push(chunk_position);
            }
        }

        missing_chunks.sort_by_key(|chunk| {
            let dx = (chunk.x - center.x).abs();
            let dz = (chunk.z - center.z).abs();
            (dx.max(dz), dx + dz, chunk.z, chunk.x)
        });

        for chunk_position in missing_chunks.into_iter().take(max_new_chunks) {
            let chunk = save_store
                .and_then(|store| {
                    store
                        .load_chunk(&self.world_id, chunk_position, &self.blocks)
                        .ok()
                        .flatten()
                })
                .unwrap_or_else(|| {
                    self.generation_pipeline
                        .generate_chunk(chunk_position, &self.generation_context)
                });
            self.insert_chunk(chunk);
            self.mark_chunk_and_horizontal_neighbors_dirty(chunk_position, &mut dirty_chunks);
        }

        dirty_chunks.into_iter().collect()
    }

    pub(super) fn build_chunk_render_mesh(
        &self,
        chunk_position: ChunkPosition,
        texture_atlas: &TextureAtlas,
    ) -> Option<(Vec<Vertex>, Vec<u32>)> {
        let chunk = self.chunks.get(&chunk_position)?;
        let mesh = self.mesh_chunk_for_render(chunk_position, chunk);
        Some(build_render_mesh(
            &[(chunk_position, mesh)],
            &self.blocks,
            texture_atlas,
            RenderChunkBounds::from_chunk_positions(self.chunks.keys().copied()),
        ))
    }

    pub(super) fn mesh_chunk_for_render(
        &self,
        chunk_position: ChunkPosition,
        chunk: &Chunk,
    ) -> ChunkMesh {
        self.mesher
            .mesh_chunk_with_neighbor_lookup(chunk, &self.blocks, |position, direction| {
                let world_position =
                    world_block_position_from_chunk_position(chunk_position, position);
                self.block(neighbor_world_block_position(world_position, direction))
            })
    }

    pub(super) fn place_block_for_player(
        &mut self,
        position: WorldBlockPosition,
        block: BlockId,
        player_eye_position: Vec3,
    ) -> Vec<ChunkPosition> {
        if self.block_intersects_player_state(position, BlockState::new(block), player_eye_position)
        {
            return Vec::new();
        }
        self.place_block(position, block)
    }

    pub(super) fn place_selected_hotbar_block_for_player(
        &mut self,
        hit: RaycastHit,
        selected_hotbar_slot: usize,
        player_eye_position: Vec3,
        player_forward: Vec3,
    ) -> Vec<ChunkPosition> {
        let Some(stack) = self.player_inventory.slot(selected_hotbar_slot) else {
            return Vec::new();
        };
        let Some(block_key) = self
            .items
            .get(stack.item)
            .and_then(|definition| definition.place_block.as_ref())
        else {
            return Vec::new();
        };
        let Some(block) = self.blocks.id_for_key(block_key) else {
            return Vec::new();
        };

        let dirty_chunks =
            self.place_block_with_rule_for_player(hit, block, player_eye_position, player_forward);
        if dirty_chunks.is_empty() {
            return dirty_chunks;
        }
        self.restore_placed_chest_inventory(hit, block, stack);

        let mut remaining = stack;
        remaining.count -= 1;
        self.player_inventory.set_slot(
            selected_hotbar_slot,
            if remaining.count == 0 {
                None
            } else {
                Some(remaining)
            },
        );
        dirty_chunks
    }

    pub(super) fn selected_hotbar_item(&self, selected_hotbar_slot: usize) -> Option<ItemId> {
        self.player_inventory
            .slot(selected_hotbar_slot)
            .map(|stack| stack.item)
    }

    pub(super) fn raycast(&self, origin: Vec3, direction: Vec3) -> Option<RaycastHit> {
        let mut previous = world_block_from_render(origin);
        let step = 0.08;
        let max_distance = 6.0;
        let steps = (max_distance / step) as usize;

        for index in 1..=steps {
            let sample = origin + direction * (index as f32 * step);
            let current = world_block_from_render(sample);
            if current == previous {
                continue;
            }
            if self.is_raycast_target(current) {
                return Some(RaycastHit {
                    block: current,
                    previous,
                    face: clicked_face(current, previous),
                    hit_position: sample,
                });
            }
            previous = current;
        }

        None
    }

    #[cfg(test)]
    pub(super) fn break_block(&mut self, position: WorldBlockPosition) -> Vec<ChunkPosition> {
        self.clear_block_break_progress();
        self.break_block_immediately(position, None)
    }

    #[cfg(test)]
    pub(super) fn break_block_with_item(
        &mut self,
        position: WorldBlockPosition,
        held_item: Option<ItemId>,
    ) -> Vec<ChunkPosition> {
        self.clear_block_break_progress();
        self.break_block_immediately(position, held_item)
    }

    fn break_block_immediately(
        &mut self,
        position: WorldBlockPosition,
        held_item: Option<ItemId>,
    ) -> Vec<ChunkPosition> {
        if self.is_unbreakable(position) {
            return Vec::new();
        }
        if let Some(block) = self.block(position) {
            if self.can_harvest_block(block, held_item) {
                if !self.spawn_container_loot_for_block(block, position) {
                    self.spawn_loot_for_block(block, position);
                }
            }
        }
        self.drop_or_preserve_block_entity_contents(position);
        self.set_block(position, self.block_ids.air)
    }

    pub(super) fn continue_breaking_block(
        &mut self,
        position: WorldBlockPosition,
        delta_seconds: f32,
        held_item: Option<ItemId>,
    ) -> Vec<ChunkPosition> {
        let Some(required_seconds) = self.block_break_seconds(position, held_item) else {
            self.clear_block_break_progress();
            return Vec::new();
        };

        let progress = self.active_block_break.get_or_insert(ActiveBlockBreak {
            target: position,
            elapsed_seconds: 0.0,
            required_seconds,
        });
        if progress.target != position {
            *progress = ActiveBlockBreak {
                target: position,
                elapsed_seconds: 0.0,
                required_seconds,
            };
        } else {
            progress.required_seconds = required_seconds;
        }

        progress.elapsed_seconds += delta_seconds.max(0.0);
        if progress.elapsed_seconds < progress.required_seconds {
            return Vec::new();
        }

        self.active_block_break = None;
        self.break_block_immediately(position, held_item)
    }

    pub(super) fn clear_block_break_progress(&mut self) {
        self.active_block_break = None;
    }

    pub(super) fn block_break_progress(&self) -> Option<BlockBreakProgress> {
        let progress = self.active_block_break?;
        if progress.required_seconds <= 0.0 {
            return Some(BlockBreakProgress {
                target: progress.target,
                ratio: 1.0,
            });
        }
        Some(BlockBreakProgress {
            target: progress.target,
            ratio: (progress.elapsed_seconds / progress.required_seconds).clamp(0.0, 1.0),
        })
    }

    pub(super) fn place_block(
        &mut self,
        position: WorldBlockPosition,
        block: BlockId,
    ) -> Vec<ChunkPosition> {
        if self.is_solid(position) {
            return Vec::new();
        }
        self.set_block(position, block)
    }

    fn place_block_with_rule_for_player(
        &mut self,
        hit: RaycastHit,
        block: BlockId,
        player_eye_position: Vec3,
        player_forward: Vec3,
    ) -> Vec<ChunkPosition> {
        let Some(definition) = self.blocks.get(block) else {
            return Vec::new();
        };
        let player_facing = horizontal_facing_from_direction(player_forward);
        let mut placement_position = hit.previous;
        let mut state = BlockState::new(block);

        match definition.placement {
            PlacementRuleKind::Simple => {}
            PlacementRuleKind::AxisFromClickedFace => {
                state.properties = BlockProperties::Axis {
                    axis: axis_from_face(hit.face),
                };
            }
            PlacementRuleKind::FacePlayerHorizontal => {
                let facing = player_facing.opposite();
                state.properties = if definition.has_tag("furnace") {
                    BlockProperties::Furnace { facing, lit: false }
                } else {
                    BlockProperties::HorizontalFacing { facing }
                };
            }
            PlacementRuleKind::Slab => {
                let desired = slab_orientation_from_face(hit.face);
                if self.block(hit.block) == Some(self.block_ids.wooden_slab)
                    && self.block_state(hit.block).is_some_and(|existing| {
                        existing.properties
                            == BlockProperties::Slab {
                                orientation: desired.opposite(),
                            }
                    })
                {
                    placement_position = hit.block;
                    state = BlockState::new(self.block_ids.oak_planks);
                } else {
                    state.properties = BlockProperties::Slab {
                        orientation: desired,
                    };
                }
            }
            PlacementRuleKind::Stairs => {
                state.properties = BlockProperties::Stairs {
                    facing: player_facing,
                    half: stair_half_from_hit(hit),
                };
            }
            PlacementRuleKind::PersistentLeaves => {
                state.properties = BlockProperties::Leaves { persistent: true };
            }
            PlacementRuleKind::Sapling => {
                state.properties = BlockProperties::Sapling { stage: 0 };
            }
        }

        if self.block_intersects_player_state(placement_position, state, player_eye_position) {
            return Vec::new();
        }
        if self.is_solid(placement_position) && placement_position != hit.block {
            return Vec::new();
        }
        let dirty = self.set_block_state(placement_position, state);
        if !dirty.is_empty() {
            self.create_block_entity_for_state(placement_position, state);
        }
        dirty
    }

    pub(super) fn is_solid(&self, position: WorldBlockPosition) -> bool {
        self.block(position)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| definition.solid)
            .unwrap_or(false)
    }

    fn is_unbreakable(&self, position: WorldBlockPosition) -> bool {
        self.block(position)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| definition.has_tag("unbreakable"))
            .unwrap_or(false)
    }

    fn block_break_seconds(
        &self,
        position: WorldBlockPosition,
        held_item: Option<ItemId>,
    ) -> Option<f32> {
        let block = self.block(position)?;
        let definition = self.blocks.get(block)?;
        if definition.has_tag("unbreakable") {
            return None;
        }
        if definition.hardness <= 0.0 {
            return Some(0.0);
        }
        let tool = self.tool_for_item(held_item);
        let seconds_per_hardness = match definition.effective_tool {
            Some(required_kind) if tool.is_some_and(|tool| tool.kind == required_kind) => {
                CORRECT_TOOL_SECONDS_PER_HARDNESS / tool.unwrap().speed_multiplier
            }
            Some(_) if definition.harvest_requirement.is_some() => {
                INEFFICIENT_BREAK_SECONDS_PER_HARDNESS
            }
            _ => CORRECT_TOOL_SECONDS_PER_HARDNESS,
        };
        Some((definition.hardness * seconds_per_hardness).max(MIN_BLOCK_BREAK_SECONDS))
    }

    fn is_raycast_target(&self, position: WorldBlockPosition) -> bool {
        self.block(position)
            .and_then(|block| self.blocks.get(block))
            .map(|definition| definition.solid || definition.shape != BlockShape::Empty)
            .unwrap_or(false)
    }

    fn can_harvest_block(&self, block: BlockId, held_item: Option<ItemId>) -> bool {
        let Some(definition) = self.blocks.get(block) else {
            return false;
        };
        let Some(requirement) = definition.harvest_requirement else {
            return true;
        };
        self.tool_for_item(held_item).is_some_and(|tool| {
            tool.kind == requirement.tool && tool.harvest_level >= requirement.min_level
        })
    }

    fn tool_for_item(&self, item: Option<ItemId>) -> Option<ToolDefinition> {
        item.and_then(|item| self.items.get(item))
            .and_then(|definition| definition.tool)
    }

    pub(super) fn block(&self, position: WorldBlockPosition) -> Option<BlockId> {
        self.block_state(position).map(|state| state.block)
    }

    pub(super) fn block_state(&self, position: WorldBlockPosition) -> Option<BlockState> {
        let (chunk_position, block_position) = split_world_block_position(position)?;
        self.chunks
            .get(&chunk_position)
            .and_then(|chunk| chunk.block_state(block_position))
    }

    pub(super) fn set_block(
        &mut self,
        position: WorldBlockPosition,
        block: BlockId,
    ) -> Vec<ChunkPosition> {
        self.set_block_state(position, BlockState::new(block))
    }

    pub(super) fn set_block_state(
        &mut self,
        position: WorldBlockPosition,
        block: BlockState,
    ) -> Vec<ChunkPosition> {
        let Some((chunk_position, block_position)) = split_world_block_position(position) else {
            return Vec::new();
        };
        let Some(chunk) = self.chunks.get_mut(&chunk_position) else {
            return Vec::new();
        };
        let Ok(()) = chunk.set_block_state(block_position, block) else {
            return Vec::new();
        };
        self.enqueue_gravity_checks_after_block_update(position);

        let mut dirty_chunks = HashSet::new();
        dirty_chunks.insert(chunk_position);
        for neighbor in dirty_horizontal_chunk_positions_for_block(chunk_position, block_position) {
            if self.chunks.contains_key(&neighbor) {
                dirty_chunks.insert(neighbor);
            }
        }
        dirty_chunks.into_iter().collect()
    }

    fn mark_chunk_and_horizontal_neighbors_dirty(
        &self,
        chunk_position: ChunkPosition,
        dirty_chunks: &mut HashSet<ChunkPosition>,
    ) {
        dirty_chunks.insert(chunk_position);
        for neighbor in horizontal_neighbor_chunk_positions(chunk_position) {
            if self.chunks.contains_key(&neighbor) {
                dirty_chunks.insert(neighbor);
            }
        }
    }

    fn create_block_entity_for_state(&mut self, position: WorldBlockPosition, state: BlockState) {
        let Some(definition) = self.blocks.get(state.block) else {
            return;
        };
        if definition.has_tag("chest") {
            self.block_entities
                .entry(position)
                .or_insert_with(|| BlockEntity::Chest(Inventory::new(CHEST_SLOT_COUNT, 0)));
        } else if definition.has_tag("furnace") {
            self.block_entities
                .entry(position)
                .or_insert_with(|| BlockEntity::Furnace(FurnaceEntity::new()));
        } else {
            self.block_entities.remove(&position);
        }
    }

    fn restore_placed_chest_inventory(
        &mut self,
        hit: RaycastHit,
        block: BlockId,
        placed_stack: ItemStack,
    ) {
        let Some(definition) = self.blocks.get(block) else {
            return;
        };
        if !definition.has_tag("chest") {
            return;
        }
        let Some(ItemStackMetadata::ChestInventory(inventory_id)) = placed_stack.metadata else {
            return;
        };
        let Some(inventory) = self.chest_item_inventories.remove(&inventory_id) else {
            return;
        };
        let position = hit.previous;
        if let Some(BlockEntity::Chest(chest_inventory)) = self.block_entities.get_mut(&position) {
            *chest_inventory = inventory;
        }
    }

    fn spawn_container_loot_for_block(
        &mut self,
        block: BlockId,
        position: WorldBlockPosition,
    ) -> bool {
        let Some(definition) = self.blocks.get(block) else {
            return false;
        };
        if !definition.has_tag("chest") {
            return false;
        }
        let Some(drop_key) = definition.drops.first() else {
            return false;
        };
        let Some(item) = self.items.id_for_key(drop_key) else {
            return false;
        };
        let inventory = match self.block_entities.get(&position) {
            Some(BlockEntity::Chest(inventory)) => inventory.clone(),
            _ => Inventory::new(CHEST_SLOT_COUNT, 0),
        };
        let inventory_id = self.next_chest_item_inventory_id;
        self.next_chest_item_inventory_id += 1;
        self.chest_item_inventories.insert(inventory_id, inventory);
        self.spawn_loot_stack(
            ItemStack::with_metadata(item, 1, ItemStackMetadata::ChestInventory(inventory_id)),
            position,
        );
        true
    }

    fn drop_or_preserve_block_entity_contents(&mut self, position: WorldBlockPosition) {
        let Some(entity) = self.block_entities.remove(&position) else {
            return;
        };
        match entity {
            BlockEntity::Chest(_inventory) => {}
            BlockEntity::Furnace(furnace) => {
                for stack in furnace.inventory.slots().iter().flatten().copied() {
                    self.spawn_loot_stack(stack, position);
                }
            }
        }
    }

    pub(super) fn tick_block_entities(&mut self) -> Vec<ChunkPosition> {
        let mut dirty_chunks = Vec::new();
        let positions: Vec<_> = self.block_entities.keys().copied().collect();
        for position in positions {
            let Some(BlockEntity::Furnace(furnace)) = self.block_entities.get_mut(&position) else {
                continue;
            };
            let was_lit = furnace.burn_ticks > 0;
            tick_furnace_entity(furnace, &self.items, &self.smelting_recipes);
            let is_lit = furnace.burn_ticks > 0;
            if was_lit != is_lit
                && let Some(mut state) = self.block_state(position)
            {
                let facing = match state.properties {
                    BlockProperties::Furnace { facing, .. } => facing,
                    BlockProperties::HorizontalFacing { facing } => facing,
                    _ => HorizontalDirection::North,
                };
                state.properties = BlockProperties::Furnace {
                    facing,
                    lit: is_lit,
                };
                dirty_chunks.extend(self.set_block_state(position, state));
            }
        }
        dirty_chunks
    }

    pub(super) fn ensure_block_entity_for_position(
        &mut self,
        position: WorldBlockPosition,
    ) -> bool {
        let Some(state) = self.block_state(position) else {
            return false;
        };
        let Some(definition) = self.blocks.get(state.block) else {
            return false;
        };
        if !definition.has_tag("chest") && !definition.has_tag("furnace") {
            return false;
        }
        self.create_block_entity_for_state(position, state);
        self.block_entities.contains_key(&position)
    }

    pub(super) fn block_entities_to_save(&self) -> Vec<BlockEntitySave> {
        let mut entities: Vec<_> = self
            .block_entities
            .iter()
            .filter_map(|(position, entity)| {
                let save_position =
                    BlockEntityPositionSave::new(position.x, position.y, position.z);
                match entity {
                    BlockEntity::Chest(inventory) => Some(BlockEntitySave::chest(
                        save_position,
                        inventory_to_save(inventory, &self.items),
                    )),
                    BlockEntity::Furnace(furnace) => Some(BlockEntitySave::furnace(
                        save_position,
                        inventory_to_save(&furnace.inventory, &self.items),
                        furnace.burn_ticks,
                        furnace.fuel_ticks,
                        furnace.cook_ticks,
                        furnace.cook_ticks_total,
                    )),
                }
            })
            .collect();
        entities.sort_by_key(|entity| (entity.position.x, entity.position.y, entity.position.z));
        entities
    }

    pub(super) fn load_saved_block_entities(&mut self, saved_entities: Vec<BlockEntitySave>) {
        self.block_entities.clear();
        for saved in saved_entities {
            let position = WorldBlockPosition {
                x: saved.position.x,
                y: saved.position.y,
                z: saved.position.z,
            };
            let Some(state) = self.block_state(position) else {
                continue;
            };
            let Some(definition) = self.blocks.get(state.block) else {
                continue;
            };
            match saved.kind {
                BlockEntityKindSave::Chest { inventory } if definition.has_tag("chest") => {
                    self.block_entities.insert(
                        position,
                        BlockEntity::Chest(inventory_from_save(&inventory, &self.items)),
                    );
                }
                BlockEntityKindSave::Furnace {
                    inventory,
                    burn_ticks,
                    fuel_ticks,
                    cook_ticks,
                    cook_ticks_total,
                } if definition.has_tag("furnace") => {
                    self.block_entities.insert(
                        position,
                        BlockEntity::Furnace(FurnaceEntity {
                            inventory: inventory_from_save(&inventory, &self.items),
                            burn_ticks,
                            fuel_ticks,
                            cook_ticks,
                            cook_ticks_total,
                        }),
                    );
                }
                _ => {}
            }
        }
    }
}

fn inventory_to_save(inventory: &Inventory, items: &ItemRegistry) -> InventorySave {
    InventorySave {
        slots: inventory
            .slots()
            .iter()
            .map(|slot| {
                slot.and_then(|stack| {
                    items
                        .get(stack.item)
                        .map(|definition| ItemStackSave::new(definition.key.clone(), stack.count))
                })
            })
            .collect(),
    }
}

fn inventory_from_save(save: &InventorySave, items: &ItemRegistry) -> Inventory {
    let slots = save
        .slots
        .iter()
        .map(|slot| {
            slot.as_ref().and_then(|stack| {
                items
                    .id_for_key(&stack.item_key)
                    .map(|item| ItemStack::new(item, stack.count))
            })
        })
        .collect();
    Inventory::from_slots(slots, 0)
}

fn tick_furnace_entity(
    furnace: &mut FurnaceEntity,
    items: &ItemRegistry,
    recipes: &SmeltingRecipeRegistry,
) {
    if furnace.burn_ticks > 0 {
        furnace.burn_ticks -= 1;
    }

    if furnace.burn_ticks == 0 && furnace_can_smelt(furnace, items, recipes) {
        if let Some(mut fuel_stack) = furnace.inventory.slot(FURNACE_FUEL_SLOT) {
            if let Some(fuel_ticks) = items.get(fuel_stack.item).and_then(|item| item.fuel_ticks) {
                fuel_stack.count -= 1;
                furnace.inventory.set_slot(
                    FURNACE_FUEL_SLOT,
                    if fuel_stack.count == 0 {
                        None
                    } else {
                        Some(fuel_stack)
                    },
                );
                furnace.burn_ticks = fuel_ticks;
                furnace.fuel_ticks = fuel_ticks;
            }
        }
    }

    if furnace.burn_ticks > 0 && furnace_can_smelt(furnace, items, recipes) {
        if let Some((_, cook_ticks)) = furnace
            .inventory
            .slot(FURNACE_INPUT_SLOT)
            .and_then(|stack| smelting_result(recipes, items, stack.item))
        {
            furnace.cook_ticks_total = cook_ticks;
        }
        furnace.cook_ticks += 1;
        if furnace.cook_ticks >= furnace.cook_ticks_total {
            smelt_one_item(furnace, items, recipes);
            furnace.cook_ticks = 0;
        }
    } else {
        furnace.cook_ticks = furnace.cook_ticks.saturating_sub(2);
    }
}

fn furnace_can_smelt(
    furnace: &FurnaceEntity,
    items: &ItemRegistry,
    recipes: &SmeltingRecipeRegistry,
) -> bool {
    let Some(input) = furnace.inventory.slot(FURNACE_INPUT_SLOT) else {
        return false;
    };
    let Some((output, _)) = smelting_result(recipes, items, input.item) else {
        return false;
    };
    match furnace.inventory.slot(FURNACE_OUTPUT_SLOT) {
        None => true,
        Some(existing) if existing.item == output.item => {
            let max = items
                .get(output.item)
                .map(|definition| definition.max_stack_size)
                .unwrap_or(64);
            existing.count.saturating_add(output.count) <= max
        }
        Some(_) => false,
    }
}

fn smelt_one_item(
    furnace: &mut FurnaceEntity,
    items: &ItemRegistry,
    recipes: &SmeltingRecipeRegistry,
) {
    let Some(mut input) = furnace.inventory.slot(FURNACE_INPUT_SLOT) else {
        return;
    };
    let Some((output, _)) = smelting_result(recipes, items, input.item) else {
        return;
    };
    input.count -= 1;
    furnace.inventory.set_slot(
        FURNACE_INPUT_SLOT,
        if input.count == 0 { None } else { Some(input) },
    );
    match furnace.inventory.slot(FURNACE_OUTPUT_SLOT) {
        None => furnace
            .inventory
            .set_slot(FURNACE_OUTPUT_SLOT, Some(output)),
        Some(mut existing) => {
            existing.count += output.count;
            furnace
                .inventory
                .set_slot(FURNACE_OUTPUT_SLOT, Some(existing));
        }
    }
}

fn clicked_face(block: WorldBlockPosition, previous: WorldBlockPosition) -> FaceDirection {
    if previous.x > block.x {
        FaceDirection::East
    } else if previous.x < block.x {
        FaceDirection::West
    } else if previous.y > block.y {
        FaceDirection::Up
    } else if previous.y < block.y {
        FaceDirection::Down
    } else if previous.z > block.z {
        FaceDirection::South
    } else {
        FaceDirection::North
    }
}

fn horizontal_facing_from_direction(direction: Vec3) -> HorizontalDirection {
    if direction.x.abs() > direction.z.abs() {
        if direction.x >= 0.0 {
            HorizontalDirection::East
        } else {
            HorizontalDirection::West
        }
    } else if direction.z >= 0.0 {
        HorizontalDirection::South
    } else {
        HorizontalDirection::North
    }
}

fn slab_orientation_from_face(face: FaceDirection) -> SlabOrientation {
    match face {
        FaceDirection::Up => SlabOrientation::Bottom,
        FaceDirection::Down => SlabOrientation::Top,
        FaceDirection::East => SlabOrientation::West,
        FaceDirection::West => SlabOrientation::East,
        FaceDirection::South => SlabOrientation::North,
        FaceDirection::North => SlabOrientation::South,
    }
}

fn axis_from_face(face: FaceDirection) -> Axis {
    match face {
        FaceDirection::East | FaceDirection::West => Axis::X,
        FaceDirection::North | FaceDirection::South => Axis::Z,
        FaceDirection::Up | FaceDirection::Down => Axis::Y,
    }
}

fn stair_half_from_hit(hit: RaycastHit) -> StairHalf {
    match hit.face {
        FaceDirection::Down => StairHalf::Top,
        FaceDirection::Up => StairHalf::Bottom,
        _ => {
            let local_y = render_y_to_block_world(hit.hit_position.y) - hit.block.y as f32;
            if local_y > 0.5 {
                StairHalf::Top
            } else {
                StairHalf::Bottom
            }
        }
    }
}
