use std::collections::{HashMap, HashSet};

use glam::Vec3;

use crate::content::BlockIds;
use crate::engine::mesh::chunk_mesher::{ChunkMesh, ChunkMesher};
use crate::engine::world::generation::{GenerationContext, GenerationPipeline};
use crate::engine::world::save::WorldSaveStore;
use crate::engine::world::{
    BlockId, BlockRegistry, Chunk, ChunkPosition, CraftingRecipeRegistry, Inventory, ItemId,
    ItemRegistry, LootEntity, ToolDefinition,
};

use super::constants::{
    CORRECT_TOOL_SECONDS_PER_HARDNESS, INEFFICIENT_BREAK_SECONDS_PER_HARDNESS,
    MIN_BLOCK_BREAK_SECONDS,
};
use super::render_types::Vertex;
use super::spatial::{
    WorldBlockPosition, chunk_position_for_render_position,
    dirty_horizontal_chunk_positions_for_block, horizontal_neighbor_chunk_positions,
    neighbor_world_block_position, split_world_block_position, world_block_from_render,
    world_block_position_from_chunk_position,
};
use super::texture::TextureAtlas;
use super::world_render::{RenderChunkBounds, build_render_mesh};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) struct RaycastHit {
    pub(super) block: WorldBlockPosition,
    pub(super) previous: WorldBlockPosition,
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

pub(super) struct ClientWorld {
    pub(super) world_id: String,
    pub(super) chunks: HashMap<ChunkPosition, Chunk>,
    pub(super) blocks: BlockRegistry,
    pub(super) items: ItemRegistry,
    pub(super) recipes: CraftingRecipeRegistry,
    pub(super) block_ids: BlockIds,
    pub(super) player_inventory: Inventory,
    pub(super) loot_entities: Vec<LootEntity>,
    generation_pipeline: GenerationPipeline,
    generation_context: GenerationContext,
    render_distance_chunks: i32,
    mesher: ChunkMesher,
    active_block_break: Option<ActiveBlockBreak>,
}

impl ClientWorld {
    pub(super) fn new(
        blocks: BlockRegistry,
        items: ItemRegistry,
        recipes: CraftingRecipeRegistry,
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
            block_ids,
            player_inventory: Inventory::player(),
            loot_entities: Vec::new(),
            generation_pipeline,
            generation_context,
            render_distance_chunks,
            mesher: ChunkMesher,
            active_block_break: None,
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
        if self.block_intersects_player(position, player_eye_position) {
            return Vec::new();
        }
        self.place_block(position, block)
    }

    pub(super) fn place_selected_hotbar_block_for_player(
        &mut self,
        position: WorldBlockPosition,
        selected_hotbar_slot: usize,
        player_eye_position: Vec3,
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

        let dirty_chunks = self.place_block_for_player(position, block, player_eye_position);
        if dirty_chunks.is_empty() {
            return dirty_chunks;
        }

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
            if self.is_solid(current) {
                return Some(RaycastHit {
                    block: current,
                    previous,
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
                self.spawn_loot_for_block(block, position);
            }
        }
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
        if definition.has_tag("unbreakable") || definition.hardness <= 0.0 {
            return None;
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
        let (chunk_position, block_position) = split_world_block_position(position)?;
        self.chunks
            .get(&chunk_position)
            .and_then(|chunk| chunk.block(block_position))
    }

    fn set_block(&mut self, position: WorldBlockPosition, block: BlockId) -> Vec<ChunkPosition> {
        let Some((chunk_position, block_position)) = split_world_block_position(position) else {
            return Vec::new();
        };
        let Some(chunk) = self.chunks.get_mut(&chunk_position) else {
            return Vec::new();
        };
        let Ok(()) = chunk.set_block(block_position, block) else {
            return Vec::new();
        };

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
}
