use super::*;

impl RenderState {
    pub(super) fn handle_menu_click(&mut self) {
        let point = cursor_to_ui_point(self.cursor_position, self.size);

        match self.mode {
            AppMode::MainMenu => {
                if UI_MAIN_PLAY.contains(point) {
                    self.mode = AppMode::ManageWorlds;
                    self.refresh_worlds();
                    self.update_window_title();
                }
            }
            AppMode::ManageWorlds => {
                if UI_WORLDS_PLAY.contains(point) {
                    self.load_selected_world();
                } else if UI_WORLDS_NEW.contains(point) {
                    self.start_world_creation();
                } else if UI_WORLDS_RENAME.contains(point) {
                    self.start_world_rename();
                } else if UI_WORLDS_DELETE.contains(point) {
                    self.delete_selected_world();
                } else if UI_WORLDS_BACK.contains(point) {
                    self.mode = AppMode::MainMenu;
                    self.update_window_title();
                } else if let Some(index) = world_list_hit_index(point, self.worlds.len()) {
                    self.selected_world = index;
                    self.update_window_title();
                }
            }
            AppMode::ConfigNewWorld => {
                if UI_CONFIG_NAME_FIELD.contains(point) {
                    self.new_world_config.focused = ConfigField::Name;
                    self.update_window_title();
                } else if UI_CONFIG_SEED_FIELD.contains(point) {
                    self.new_world_config.focused = ConfigField::Seed;
                    self.update_window_title();
                } else if UI_CONFIG_CREATE.contains(point) {
                    self.create_configured_world();
                } else if UI_CONFIG_BACK.contains(point) {
                    self.mode = AppMode::ManageWorlds;
                    self.update_window_title();
                }
            }
            AppMode::RenamingWorld => {
                if UI_RENAME_SAVE.contains(point) {
                    self.finish_text_entry();
                } else if UI_RENAME_BACK.contains(point) {
                    self.mode = AppMode::ManageWorlds;
                    self.text_entry = TextEntry::default();
                    self.update_window_title();
                }
            }
            AppMode::InGame if self.paused => {
                if UI_PAUSE_KEEP_PLAYING.contains(point) {
                    self.resume_game();
                } else if UI_PAUSE_SAVE_QUIT.contains(point) {
                    self.save_and_quit_to_main_menu();
                }
            }
            _ => {}
        }
    }

    pub(super) fn refresh_worlds(&mut self) {
        self.worlds = self.save_store.list_worlds().unwrap_or_else(|error| {
            eprintln!("{error}");
            Vec::new()
        });
        if self.worlds.is_empty() {
            self.selected_world = 0;
        } else {
            self.selected_world = self.selected_world.min(self.worlds.len() - 1);
        }
    }

    pub(super) fn select_previous_world(&mut self) {
        if self.worlds.is_empty() {
            return;
        }
        self.selected_world = self.selected_world.saturating_sub(1);
        self.update_window_title();
    }

    pub(super) fn select_next_world(&mut self) {
        if self.worlds.is_empty() {
            return;
        }
        self.selected_world = (self.selected_world + 1).min(self.worlds.len() - 1);
        self.update_window_title();
    }

    pub(super) fn start_world_creation(&mut self) {
        self.new_world_config
            .start(default_world_name(self.worlds.len()));
        self.mode = AppMode::ConfigNewWorld;
        self.update_window_title();
    }

    pub(super) fn start_world_rename(&mut self) {
        let Some(world) = self.worlds.get(self.selected_world) else {
            return;
        };
        self.text_entry.start(world.name.clone());
        self.mode = AppMode::RenamingWorld;
        self.update_window_title();
    }

    pub(super) fn finish_text_entry(&mut self) {
        match self.mode {
            AppMode::RenamingWorld => {
                let Some(world) = self.worlds.get(self.selected_world) else {
                    self.mode = AppMode::ManageWorlds;
                    self.update_window_title();
                    return;
                };
                let name = self.text_entry.finish();
                match self.save_store.rename_world(&world.id, &name) {
                    Ok(_) => {
                        self.mode = AppMode::ManageWorlds;
                        self.refresh_worlds();
                        self.update_window_title();
                    }
                    Err(error) => self.report_save_error(error),
                }
            }
            _ => {}
        }
    }

    pub(super) fn create_configured_world(&mut self) {
        let name = self.new_world_config.final_name();
        let seed = self
            .new_world_config
            .seed
            .trim()
            .parse::<u64>()
            .unwrap_or_else(|_| new_world_seed(self.worlds.len()));
        let placeholder_player = PlayerSave::new(
            0.0,
            0.0,
            20.0,
            -90.0_f32.to_radians(),
            -18.0_f32.to_radians(),
        );
        match self
            .save_store
            .create_world(&name, seed, placeholder_player)
        {
            Ok(metadata) => {
                self.refresh_worlds();
                if let Some(index) = self.worlds.iter().position(|world| world.id == metadata.id) {
                    self.selected_world = index;
                }
                self.load_world(metadata);
            }
            Err(error) => self.report_save_error(error),
        }
    }

    pub(super) fn delete_selected_world(&mut self) {
        let Some(world) = self.worlds.get(self.selected_world) else {
            return;
        };
        let id = world.id.clone();
        if let Err(error) = self.save_store.delete_world(&id) {
            self.report_save_error(error);
            return;
        }
        self.refresh_worlds();
        self.update_window_title();
    }

    pub(super) fn load_selected_world(&mut self) {
        let Some(metadata) = self.worlds.get(self.selected_world).cloned() else {
            self.start_world_creation();
            return;
        };
        self.load_world(metadata);
    }

    pub(super) fn load_world(&mut self, mut metadata: WorldMetadata) {
        let content = bootstrap_content().expect("content should bootstrap");
        let pipeline = default_generation_pipeline(content.block_ids);
        let generation_context = GenerationContext {
            seed: metadata.seed,
            air: content.block_ids.air,
        };
        let mut world = ClientWorld::new(
            content.blocks,
            content.items,
            content.recipes,
            content.block_ids,
            pipeline,
            generation_context,
            CLIENT_RENDER_DISTANCE_CHUNKS,
            metadata.id.clone(),
        );
        world.player_inventory = inventory_from_save(&metadata.inventory, &world.items);

        let saved_eye = Vec3::new(
            metadata.player.eye_x,
            metadata.player.eye_y,
            metadata.player.eye_z,
        );
        let generated_chunks = world.ensure_chunks_around_render_position_with_store(
            saved_eye,
            usize::MAX,
            &self.save_store,
        );
        let spawn_eye = if metadata.player.eye_y == 0.0 {
            world.safe_spawn_eye_position(Vec3::new(0.0, 0.0, 20.0))
        } else {
            saved_eye
        };
        self.camera = Camera::from_save(PlayerSave::new(
            spawn_eye.x,
            spawn_eye.y,
            spawn_eye.z,
            metadata.player.yaw,
            metadata.player.pitch,
        ));
        metadata.player = self.camera.to_save();

        self.world = Some(world);
        self.active_world = Some(metadata);
        self.chunk_buffers.clear();
        self.pending_chunk_remeshes.clear();
        self.dirty_save_chunks.clear();
        for chunk_position in &generated_chunks {
            self.dirty_save_chunks.insert(*chunk_position);
        }
        self.player_state_dirty = false;
        self.inventory_cursor = None;
        self.inventory_drag = None;
        self.crafting_table_open = false;
        self.inventory_crafting_grid = Inventory::new(4, 0);
        self.crafting_table_grid = Inventory::new(9, 0);
        self.crafting_result = None;
        self.selected_hotbar_slot = 0;
        self.chunk_buffers = if let Some(world) = &self.world {
            build_chunk_render_buffers(&self.device, world, &self.texture_atlas, &generated_chunks)
        } else {
            HashMap::new()
        };
        self.mode = AppMode::InGame;
        self.paused = false;
        self.inventory_open = false;
        self.input.clear_movement();
        capture_cursor(&self.window);
        self.update_window_title();
    }

    pub(super) fn mark_dirty_chunks_for_save(&mut self, dirty_chunks: &[ChunkPosition]) {
        let Some(world) = &self.world else {
            return;
        };
        for chunk_position in unique_loaded_chunk_positions(dirty_chunks, world) {
            self.dirty_save_chunks.insert(chunk_position);
        }
    }

    pub(super) fn mark_player_state_dirty(&mut self) {
        if self.active_world.is_some() {
            self.player_state_dirty = true;
        }
    }

    pub(super) fn flush_active_world_to_disk(&mut self) {
        self.stow_inventory_cursor();
        self.stow_active_crafting_grid();
        let Some(metadata) = self.active_world.as_mut() else {
            return;
        };
        metadata.player = self.camera.to_save();
        if let Some(world) = &self.world {
            metadata.inventory = inventory_to_save(&world.player_inventory, &world.items);
        }
        metadata.updated_at_unix_seconds = current_save_time();
        let world_id = metadata.id.clone();
        if let Err(error) = self.save_store.save_metadata(metadata) {
            self.report_save_error(error);
        }

        if let Some(world) = &self.world {
            let dirty_chunks: Vec<_> = self.dirty_save_chunks.iter().copied().collect();
            for chunk_position in dirty_chunks {
                if let Some(chunk) = world.chunks.get(&chunk_position) {
                    if let Err(error) = self.save_store.save_chunk(&world_id, chunk, &world.blocks)
                    {
                        self.report_save_error(error);
                    }
                }
            }
        }

        self.dirty_save_chunks.clear();
        self.player_state_dirty = false;
    }

    pub(super) fn resume_game(&mut self) {
        self.set_paused(false);
    }

    pub(super) fn save_and_quit_to_main_menu(&mut self) {
        self.flush_active_world_to_disk();
        self.world = None;
        self.active_world = None;
        self.chunk_buffers.clear();
        self.pending_chunk_remeshes.clear();
        self.dirty_save_chunks.clear();
        self.player_state_dirty = false;
        self.inventory_cursor = None;
        self.inventory_drag = None;
        self.crafting_table_open = false;
        self.inventory_crafting_grid = Inventory::new(4, 0);
        self.crafting_table_grid = Inventory::new(9, 0);
        self.crafting_result = None;
        self.input.clear_movement();
        self.paused = true;
        self.inventory_open = false;
        self.mode = AppMode::MainMenu;
        self.refresh_worlds();
        release_cursor(&self.window);
        self.update_window_title();
    }

    pub(super) fn report_save_error(&self, error: WorldSaveError) {
        eprintln!("{error}");
        self.window
            .set_title(&format!("HumanCraft - Save error: {error}"));
    }

    pub(super) fn with_updated_title(self) -> Self {
        self.update_window_title();
        self
    }

    pub(super) fn update_window_title(&self) {
        let title = match self.mode {
            AppMode::MainMenu => "HumanCraft - Main Menu: click Play or press Enter".to_string(),
            AppMode::ManageWorlds => {
                if let Some(world) = self.worlds.get(self.selected_world) {
                    format!(
                        "HumanCraft - Worlds: {} seed {} ({}/{}) | Enter load, N new, R rename, Delete delete",
                        world.name,
                        world.seed,
                        self.selected_world + 1,
                        self.worlds.len()
                    )
                } else {
                    "HumanCraft - Worlds: no saves | N create or Enter".to_string()
                }
            }
            AppMode::ConfigNewWorld => format!(
                "HumanCraft - Configure New World: name '{}', seed {} | Tab field, Enter create",
                self.new_world_config.final_name(),
                if self.new_world_config.seed.is_empty() {
                    "auto"
                } else {
                    self.new_world_config.seed.as_str()
                }
            ),
            AppMode::RenamingWorld => format!(
                "HumanCraft - Rename world: {} | type, Enter save, Esc cancel",
                self.text_entry.display()
            ),
            AppMode::InGame => self
                .active_world
                .as_ref()
                .map(|world| format!("HumanCraft - {} (seed {})", world.name, world.seed))
                .unwrap_or_else(|| "HumanCraft".to_string()),
        };
        self.window.set_title(&title);
    }
}
