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
            self.stow_active_crafting_grid();
            self.inventory_drag = None;
            self.crafting_table_open = false;
            self.crafting_result = None;
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
            self.stow_active_crafting_grid();
            self.inventory_drag = None;
            self.crafting_table_open = false;
            self.crafting_result = None;
        }
        self.inventory_open = inventory_open;
        if inventory_open {
            self.crafting_table_open = false;
            self.update_crafting_result();
        }
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

    pub(super) fn open_crafting_table(&mut self) {
        if self.mode != AppMode::InGame || self.paused {
            return;
        }
        self.held_block_interaction.clear();
        self.stow_inventory_cursor();
        if self.inventory_open {
            self.stow_active_crafting_grid();
        }
        self.inventory_drag = None;
        self.inventory_open = true;
        self.crafting_table_open = true;
        self.update_crafting_result();
        self.input.clear_movement();
        release_cursor(&self.window);
        self.window
            .set_title("HumanCraft - Crafting Table (E or Esc to close)");
    }

    pub(super) fn start_inventory_mouse(&mut self, button: MouseButton) {
        let Some(button) = inventory_mouse_button(button) else {
            return;
        };
        let slot = self.player_inventory_slot_at_cursor();
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

        if let Some(slot) = self.crafting_input_slot_at_cursor() {
            self.click_crafting_input_slot(slot, drag.button);
            return;
        }
        if self.crafting_result_slot_at_cursor() {
            self.click_crafting_result_slot(drag.button);
            return;
        }

        let slot = self.player_inventory_slot_at_cursor().or(drag.start_slot);
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
        let Some(slot) = self.player_inventory_slot_at_cursor() else {
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

    pub(super) fn player_inventory_slot_at_cursor(&self) -> Option<usize> {
        if self.mode != AppMode::InGame || !self.inventory_open {
            return None;
        }
        let point = cursor_to_ui_point(self.cursor_position, self.size);
        let aspect = self.config.width.max(1) as f32 / self.config.height.max(1) as f32;
        inventory_slot_at_point(point, aspect)
    }

    pub(super) fn crafting_input_slot_at_cursor(&self) -> Option<usize> {
        if self.mode != AppMode::InGame || !self.inventory_open {
            return None;
        }
        let point = cursor_to_ui_point(self.cursor_position, self.size);
        let aspect = self.config.width.max(1) as f32 / self.config.height.max(1) as f32;
        crafting_input_slot_at_point(point, self.crafting_ui_kind(), aspect)
    }

    pub(super) fn crafting_result_slot_at_cursor(&self) -> bool {
        if self.mode != AppMode::InGame || !self.inventory_open {
            return false;
        }
        let point = cursor_to_ui_point(self.cursor_position, self.size);
        let aspect = self.config.width.max(1) as f32 / self.config.height.max(1) as f32;
        crafting_result_slot_at_point(point, self.crafting_ui_kind(), aspect)
    }

    pub(super) fn crafting_ui_kind(&self) -> CraftingUiKind {
        if self.crafting_table_open {
            CraftingUiKind::Table
        } else {
            CraftingUiKind::Inventory
        }
    }

    pub(super) fn active_crafting_grid(&self) -> &Inventory {
        if self.crafting_table_open {
            &self.crafting_table_grid
        } else {
            &self.inventory_crafting_grid
        }
    }

    fn active_crafting_grid_mut(&mut self) -> &mut Inventory {
        if self.crafting_table_open {
            &mut self.crafting_table_grid
        } else {
            &mut self.inventory_crafting_grid
        }
    }

    fn active_crafting_width(&self) -> usize {
        if self.crafting_table_open { 3 } else { 2 }
    }

    pub(super) fn update_crafting_result(&mut self) {
        self.crafting_result = self.world.as_ref().and_then(|world| {
            crafting_result(
                &world.recipes,
                &world.items,
                self.active_crafting_grid(),
                self.active_crafting_width(),
            )
        });
    }

    fn click_crafting_input_slot(&mut self, slot: usize, button: InventoryMouseButton) {
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let items = world.items.clone();
        match button {
            InventoryMouseButton::Left => {
                let mut cursor = self.inventory_cursor;
                left_click_inventory_slot(
                    self.active_crafting_grid_mut(),
                    &mut cursor,
                    slot,
                    &items,
                );
                self.inventory_cursor = cursor;
            }
            InventoryMouseButton::Right => {
                let mut cursor = self.inventory_cursor;
                right_click_inventory_slot(
                    self.active_crafting_grid_mut(),
                    &mut cursor,
                    slot,
                    &items,
                );
                self.inventory_cursor = cursor;
            }
        }
        self.update_crafting_result();
    }

    fn click_crafting_result_slot(&mut self, button: InventoryMouseButton) {
        if button != InventoryMouseButton::Left {
            return;
        }
        let Some(result) = self.crafting_result else {
            return;
        };
        let Some(world) = self.world.as_ref() else {
            return;
        };
        if !cursor_can_accept_result(self.inventory_cursor, result, &world.items) {
            return;
        }
        self.inventory_cursor = merge_result_into_cursor(self.inventory_cursor, result);
        consume_crafting_ingredients(self.active_crafting_grid_mut());
        self.update_crafting_result();
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

    pub(super) fn stow_active_crafting_grid(&mut self) {
        if self.world.is_none() {
            return;
        }
        let mut stacks = Vec::new();
        {
            let grid = self.active_crafting_grid_mut();
            for index in 0..grid.slot_count() {
                if let Some(stack) = grid.slot(index) {
                    grid.set_slot(index, None);
                    stacks.push((index, stack));
                }
            }
        }

        let mut overflows = Vec::new();
        if let Some(world) = self.world.as_mut() {
            let items = world.items.clone();
            for (index, stack) in stacks {
                if let Some(overflow) = world.player_inventory.add_stack(stack, &items) {
                    overflows.push((index, overflow));
                }
            }
        }

        if !overflows.is_empty() {
            let grid = self.active_crafting_grid_mut();
            for (index, overflow) in overflows {
                grid.set_slot(index, Some(overflow));
            }
        }
        self.update_crafting_result();
    }
}

fn cursor_can_accept_result(
    cursor: Option<ItemStack>,
    result: ItemStack,
    items: &crate::engine::world::ItemRegistry,
) -> bool {
    match cursor {
        None => true,
        Some(stack) if stack.item == result.item => {
            let max_stack_size = items
                .get(result.item)
                .map(|definition| definition.max_stack_size)
                .unwrap_or(64);
            stack.count.saturating_add(result.count) <= max_stack_size
        }
        Some(_) => false,
    }
}

fn merge_result_into_cursor(cursor: Option<ItemStack>, result: ItemStack) -> Option<ItemStack> {
    match cursor {
        None => Some(result),
        Some(mut stack) => {
            stack.count += result.count;
            Some(stack)
        }
    }
}
