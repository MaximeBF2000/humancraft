use super::*;

impl RenderState {
    pub(super) fn handle_key(&mut self, event: &KeyEvent) -> bool {
        if event.state == ElementState::Pressed {
            if self.mode == AppMode::InGame && is_inventory_key(event) {
                self.set_inventory_open(!self.inventory_open);
                return true;
            }

            if self.mode == AppMode::InGame && !self.paused && self.handle_hotbar_key(event) {
                return true;
            }

            if self.handle_menu_key(event) {
                return true;
            }
        }

        if event.state == ElementState::Pressed
            && matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape))
            && self.mode == AppMode::InGame
        {
            if self.inventory_open {
                self.set_inventory_open(false);
                return true;
            }
            self.set_paused(!self.paused);
            return true;
        }

        if self.mode == AppMode::InGame && !self.paused && !self.inventory_open {
            self.input.handle_key(event);
        }
        true
    }

    pub(super) fn handle_menu_key(&mut self, event: &KeyEvent) -> bool {
        match self.mode {
            AppMode::MainMenu => {
                if is_confirm_key(event) {
                    self.mode = AppMode::ManageWorlds;
                    self.refresh_worlds();
                    self.update_window_title();
                    return true;
                }
            }
            AppMode::ManageWorlds => {
                if is_confirm_key(event) {
                    self.load_selected_world();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::ArrowUp)) {
                    self.select_previous_world();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::ArrowDown)) {
                    self.select_next_world();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Delete)) {
                    self.delete_selected_world();
                    return true;
                }
                if character_key(event, "n") {
                    self.start_world_creation();
                    return true;
                }
                if character_key(event, "r") {
                    self.start_world_rename();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.mode = AppMode::MainMenu;
                    self.update_window_title();
                    return true;
                }
            }
            AppMode::ConfigNewWorld => {
                if is_confirm_key(event) {
                    self.create_configured_world();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Tab)) {
                    self.new_world_config.toggle_focus();
                    self.update_window_title();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Backspace)) {
                    self.new_world_config.pop();
                    self.update_window_title();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.mode = AppMode::ManageWorlds;
                    self.update_window_title();
                    return true;
                }
                if let Key::Character(character) = event.logical_key.as_ref() {
                    self.new_world_config.push(character);
                    self.update_window_title();
                    return true;
                }
            }
            AppMode::RenamingWorld => {
                if is_confirm_key(event) {
                    self.finish_text_entry();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Backspace)) {
                    self.text_entry.pop();
                    self.update_window_title();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.mode = AppMode::ManageWorlds;
                    self.text_entry = TextEntry::default();
                    self.update_window_title();
                    return true;
                }
                if let Key::Character(character) = event.logical_key.as_ref() {
                    self.text_entry.push(character);
                    self.update_window_title();
                    return true;
                }
            }
            AppMode::InGame => {
                if self.paused {
                    if is_confirm_key(event) {
                        self.resume_game();
                        return true;
                    }
                    if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                        self.resume_game();
                        return true;
                    }
                    if character_key(event, "q") {
                        self.save_and_quit_to_main_menu();
                        return true;
                    }
                }
            }
        }

        false
    }

    pub(super) fn handle_text_input(&mut self, text: &str) {
        if text.is_ascii() {
            return;
        }
        match self.mode {
            AppMode::ConfigNewWorld => {
                self.new_world_config.push(text);
                self.update_window_title();
            }
            AppMode::RenamingWorld => {
                self.text_entry.push(text);
                self.update_window_title();
            }
            _ => {}
        }
    }

    pub(super) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = position;
        if self.mode == AppMode::InGame && self.inventory_open {
            self.update_inventory_drag();
        }
    }

    pub(super) fn handle_focus_lost(&mut self) {
        if self.mode == AppMode::InGame {
            self.set_paused(true);
        }
    }

    pub(super) fn handle_hotbar_key(&mut self, event: &KeyEvent) -> bool {
        if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::ArrowLeft)) {
            self.selected_hotbar_slot =
                (self.selected_hotbar_slot + INVENTORY_HOTBAR_SLOTS - 1) % INVENTORY_HOTBAR_SLOTS;
            return true;
        }
        if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::ArrowRight)) {
            self.selected_hotbar_slot = (self.selected_hotbar_slot + 1) % INVENTORY_HOTBAR_SLOTS;
            return true;
        }
        false
    }

    pub(super) fn handle_mouse_motion(&mut self, delta_x: f32, delta_y: f32) {
        if self.mode == AppMode::InGame && !self.paused && !self.inventory_open {
            self.camera.apply_mouse_delta(delta_x, delta_y);
        }
    }

    pub(super) fn handle_mouse_button(&mut self, button: MouseButton, mouse_state: ElementState) {
        if self.mode == AppMode::InGame
            && self.inventory_open
            && (button == MouseButton::Left || button == MouseButton::Right)
        {
            match mouse_state {
                ElementState::Pressed => self.start_inventory_mouse(button),
                ElementState::Released => self.finish_inventory_mouse(button),
            }
            return;
        }

        if mouse_state == ElementState::Released {
            self.held_block_interaction.release(button);
            if button == MouseButton::Left {
                if let Some(world) = self.world.as_mut() {
                    world.clear_block_break_progress();
                }
            }
            return;
        }

        if self.mode != AppMode::InGame || self.paused {
            if button == MouseButton::Left {
                self.handle_menu_click();
            }
            return;
        }

        if self.paused {
            return;
        }

        if !matches!(button, MouseButton::Left | MouseButton::Right) {
            return;
        }
        self.held_block_interaction.press(button);
        if button == MouseButton::Left {
            return;
        }
        let dirty_chunks = self.apply_block_interaction(button);

        if !dirty_chunks.is_empty() {
            self.mark_dirty_chunks_for_save(&dirty_chunks);
            self.rebuild_chunk_meshes(&dirty_chunks);
        }
    }

    pub(super) fn set_paused(&mut self, paused: bool) {
        if self.mode != AppMode::InGame {
            return;
        }
        if paused {
            self.stow_inventory_cursor();
            self.inventory_drag = None;
            self.held_block_interaction.clear();
            if let Some(world) = self.world.as_mut() {
                world.clear_block_break_progress();
            }
        }
        self.paused = paused;
        if paused {
            self.inventory_open = false;
        }
        self.input.clear_movement();
        if paused {
            self.mark_player_state_dirty();
            release_cursor(&self.window);
            self.window
                .set_title("HumanCraft - Paused (Esc to resume, close window to quit)");
        } else {
            capture_cursor(&self.window);
            self.update_window_title();
        }
    }

    pub(super) fn set_inventory_open(&mut self, inventory_open: bool) {
        if self.mode != AppMode::InGame || self.paused {
            return;
        }
        self.held_block_interaction.clear();
        if let Some(world) = self.world.as_mut() {
            world.clear_block_break_progress();
        }
        if !inventory_open {
            self.stow_inventory_cursor();
            self.inventory_drag = None;
        }
        self.inventory_open = inventory_open;
        self.input.clear_movement();
        if inventory_open {
            release_cursor(&self.window);
            self.window
                .set_title("HumanCraft - Inventory (E or Esc to close)");
        } else {
            capture_cursor(&self.window);
            self.update_window_title();
        }
    }

    pub(super) fn start_inventory_mouse(&mut self, button: MouseButton) {
        let Some(button) = inventory_mouse_button(button) else {
            return;
        };
        let slot = self.inventory_slot_at_cursor();
        self.inventory_drag = Some(InventoryDrag::new(button, slot));
    }

    pub(super) fn finish_inventory_mouse(&mut self, button: MouseButton) {
        let Some(button) = inventory_mouse_button(button) else {
            return;
        };
        let Some(drag) = self.inventory_drag.take() else {
            return;
        };
        if drag.button != button {
            return;
        }

        let slot = self.inventory_slot_at_cursor().or(drag.start_slot);
        match drag.button {
            InventoryMouseButton::Left if drag.changed_slots && !drag.slots.is_empty() => {
                if let Some(world) = self.world.as_mut() {
                    distribute_carried_stack_evenly(
                        &mut world.player_inventory,
                        &mut self.inventory_cursor,
                        &drag.slots,
                        &world.items,
                    );
                }
            }
            InventoryMouseButton::Left => {
                if let (Some(world), Some(slot)) = (self.world.as_mut(), slot) {
                    left_click_inventory_slot(
                        &mut world.player_inventory,
                        &mut self.inventory_cursor,
                        slot,
                        &world.items,
                    );
                }
            }
            InventoryMouseButton::Right if drag.applied_drag => {}
            InventoryMouseButton::Right => {
                if let (Some(world), Some(slot)) = (self.world.as_mut(), slot) {
                    right_click_inventory_slot(
                        &mut world.player_inventory,
                        &mut self.inventory_cursor,
                        slot,
                        &world.items,
                    );
                }
            }
        }
    }

    pub(super) fn update_inventory_drag(&mut self) {
        let Some(slot) = self.inventory_slot_at_cursor() else {
            return;
        };
        let Some(drag) = self.inventory_drag.as_mut() else {
            return;
        };
        if !drag.push_slot(slot) {
            return;
        }
        if drag.button == InventoryMouseButton::Right {
            if let Some(world) = self.world.as_mut() {
                if place_one_carried_item(
                    &mut world.player_inventory,
                    &mut self.inventory_cursor,
                    slot,
                    &world.items,
                ) {
                    drag.applied_drag = true;
                }
            }
        }
    }

    pub(super) fn inventory_slot_at_cursor(&self) -> Option<usize> {
        if self.mode != AppMode::InGame || !self.inventory_open {
            return None;
        }
        let point = cursor_to_ui_point(self.cursor_position, self.size);
        let aspect = self.config.width.max(1) as f32 / self.config.height.max(1) as f32;
        inventory_slot_at_point(point, aspect)
    }

    pub(super) fn stow_inventory_cursor(&mut self) {
        let Some(stack) = self.inventory_cursor.take() else {
            return;
        };
        let Some(world) = self.world.as_mut() else {
            self.inventory_cursor = Some(stack);
            return;
        };
        self.inventory_cursor = world.player_inventory.add_stack(stack, &world.items);
    }
}
