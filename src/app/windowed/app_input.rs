use super::*;

impl RenderState {
    pub(super) fn handle_key(&mut self, event: &KeyEvent) -> bool {
        self.update_modifier_keys(event);
        if event.state == ElementState::Pressed {
            if self.mode == AppMode::Shortcuts
                && let Some(action) = self.rebinding_shortcut
                && let Some(label) = shortcut_label_for_event(event)
            {
                self.key_bindings.set_label(action, label);
                self.rebinding_shortcut = None;
                self.update_window_title();
                return true;
            }

            if self.mode == AppMode::InGame
                && self.inventory_open
                && self.handle_inventory_keyboard(event)
            {
                return true;
            }

            if self.mode == AppMode::InGame
                && self.shortcut_pressed(event, ShortcutAction::Inventory)
            {
                self.set_inventory_open(!self.inventory_open);
                return true;
            }

            if self.mode == AppMode::InGame
                && !self.paused
                && !self.inventory_open
                && self.shortcut_pressed(event, ShortcutAction::Drop)
            {
                self.drop_selected_hotbar_stack(self.modifier_control);
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

        if event.state == ElementState::Pressed
            && self.mode == AppMode::InGame
            && self.shortcut_pressed(event, ShortcutAction::Pause)
        {
            if self.inventory_open {
                self.set_inventory_open(false);
                return true;
            }
            self.set_paused(!self.paused);
            return true;
        }

        if self.mode == AppMode::InGame && !self.paused && !self.inventory_open {
            self.input.handle_key(event, &self.key_bindings);
        }
        true
    }

    pub(super) fn handle_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifier_shift = modifiers.shift_key();
        self.modifier_control = modifiers.control_key();
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
                if character_key(event, "s") {
                    self.open_settings(false);
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
            AppMode::Settings => {
                if is_confirm_key(event) {
                    self.mode = AppMode::Shortcuts;
                    self.rebinding_shortcut = None;
                    self.update_window_title();
                    return true;
                }
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.close_settings();
                    return true;
                }
            }
            AppMode::Shortcuts => {
                if matches!(event.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    if self.rebinding_shortcut.take().is_some() {
                        self.update_window_title();
                    } else {
                        self.mode = AppMode::Settings;
                        self.update_window_title();
                    }
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
                    if character_key(event, "s") {
                        self.open_settings(true);
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
        if self.shortcut_pressed(event, ShortcutAction::HotbarPrevious) {
            self.selected_hotbar_slot =
                (self.selected_hotbar_slot + INVENTORY_HOTBAR_SLOTS - 1) % INVENTORY_HOTBAR_SLOTS;
            return true;
        }
        if self.shortcut_pressed(event, ShortcutAction::HotbarNext) {
            self.selected_hotbar_slot = (self.selected_hotbar_slot + 1) % INVENTORY_HOTBAR_SLOTS;
            return true;
        }
        if let Some(index) = bound_hotbar_key(event, &self.key_bindings) {
            self.selected_hotbar_slot = index;
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

    fn update_modifier_keys(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::ShiftLeft | KeyCode::ShiftRight => self.modifier_shift = pressed,
                KeyCode::ControlLeft | KeyCode::ControlRight => self.modifier_control = pressed,
                _ => {}
            }
        }
        match event.logical_key.as_ref() {
            Key::Named(NamedKey::Shift) => self.modifier_shift = pressed,
            Key::Named(NamedKey::Control) => self.modifier_control = pressed,
            _ => {}
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
            self.last_inventory_click = None;
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
        self.last_inventory_click = None;
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
        let slot = self.inventory_slot_at_cursor();
        if let Some(world) = self.world.as_ref() {
            self.inventory_drag = Some(InventoryDrag::new(
                button,
                slot,
                self.inventory_cursor,
                &world.player_inventory,
                self.active_crafting_grid(),
            ));
        }
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
            InventoryMouseButton::Left
                if drag.changed_slots && !drag.slots.is_empty() && !drag.applied_drag =>
            {
                self.distribute_dragged_slots(&drag.slots);
            }
            InventoryMouseButton::Left if drag.applied_drag => {}
            InventoryMouseButton::Left => {
                if let Some(slot) = slot {
                    let clicked_item = self
                        .stack_at_inventory_slot(slot)
                        .map(|stack| stack.item)
                        .or_else(|| self.inventory_cursor.map(|stack| stack.item));
                    if self.is_double_click(slot, drag.button) {
                        self.collect_matching_visible_stacks(slot);
                    } else {
                        self.click_inventory_slot(slot, drag.button);
                    }
                    if let Some(item) = clicked_item {
                        self.last_inventory_click = Some((Instant::now(), drag.button, item));
                    }
                } else if let Some(stack) = take_from_cursor(&mut self.inventory_cursor, true) {
                    self.drop_stack_near_player(stack);
                }
            }
            InventoryMouseButton::Right if drag.applied_drag => {}
            InventoryMouseButton::Right => {
                if let Some(slot) = slot {
                    let clicked_item = self
                        .stack_at_inventory_slot(slot)
                        .map(|stack| stack.item)
                        .or_else(|| self.inventory_cursor.map(|stack| stack.item));
                    self.click_inventory_slot(slot, drag.button);
                    if let Some(item) = clicked_item {
                        self.last_inventory_click = Some((Instant::now(), drag.button, item));
                    }
                }
            }
        }
    }

    pub(super) fn update_inventory_drag(&mut self) {
        let Some(slot) = self.inventory_slot_at_cursor() else {
            return;
        };
        let button = {
            let Some(drag) = self.inventory_drag.as_mut() else {
                return;
            };
            if !drag.push_slot(slot) {
                return;
            }
            drag.button
        };
        if button == InventoryMouseButton::Right {
            if self.place_one_in_slot(slot) {
                if let Some(drag) = self.inventory_drag.as_mut() {
                    drag.applied_drag = true;
                }
            }
        } else if button == InventoryMouseButton::Left {
            self.preview_left_drag_distribution();
        }
    }

    pub(super) fn inventory_slot_at_cursor(&self) -> Option<InventorySlotId> {
        self.crafting_result_slot_at_cursor()
            .then_some(InventorySlotId::CraftingResult)
            .or_else(|| {
                self.crafting_input_slot_at_cursor()
                    .map(InventorySlotId::CraftingInput)
            })
            .or_else(|| {
                self.player_inventory_slot_at_cursor()
                    .map(InventorySlotId::Player)
            })
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

    fn click_inventory_slot(&mut self, slot: InventorySlotId, button: InventoryMouseButton) {
        if self.modifier_shift && button == InventoryMouseButton::Left {
            self.quick_transfer_slot(slot);
            return;
        }
        match slot {
            InventorySlotId::Player(slot) => {
                if let Some(world) = self.world.as_mut() {
                    match button {
                        InventoryMouseButton::Left => left_click_inventory_slot(
                            &mut world.player_inventory,
                            &mut self.inventory_cursor,
                            slot,
                            &world.items,
                        ),
                        InventoryMouseButton::Right => right_click_inventory_slot(
                            &mut world.player_inventory,
                            &mut self.inventory_cursor,
                            slot,
                            &world.items,
                        ),
                    }
                }
            }
            InventorySlotId::CraftingInput(slot) => self.click_crafting_input_slot(slot, button),
            InventorySlotId::CraftingResult => {
                self.click_crafting_result_slot(button, self.modifier_shift)
            }
        }
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

    fn click_crafting_result_slot(&mut self, button: InventoryMouseButton, quick_move: bool) {
        if button != InventoryMouseButton::Left {
            return;
        }
        if quick_move {
            self.quick_craft_to_inventory();
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

    fn quick_transfer_slot(&mut self, slot: InventorySlotId) {
        match slot {
            InventorySlotId::Player(slot) => {
                if let Some(world) = self.world.as_mut() {
                    quick_transfer_player_slot(&mut world.player_inventory, slot, &world.items);
                }
            }
            InventorySlotId::CraftingInput(slot) => {
                let Some(stack) = self.active_crafting_grid().slot(slot) else {
                    return;
                };
                let Some(world) = self.world.as_mut() else {
                    return;
                };
                let remainder = move_stack_into_player_inventory(
                    &mut world.player_inventory,
                    stack,
                    &world.items,
                );
                self.active_crafting_grid_mut().set_slot(slot, remainder);
                self.update_crafting_result();
            }
            InventorySlotId::CraftingResult => self.quick_craft_to_inventory(),
        }
    }

    fn quick_craft_to_inventory(&mut self) {
        loop {
            let Some(result) = self.crafting_result else {
                break;
            };
            let Some(world) = self.world.as_mut() else {
                break;
            };
            let remainder =
                move_stack_into_player_inventory(&mut world.player_inventory, result, &world.items);
            if remainder.is_some() {
                break;
            }
            consume_crafting_ingredients(self.active_crafting_grid_mut());
            self.update_crafting_result();
        }
    }

    fn distribute_dragged_slots(&mut self, slots: &[InventorySlotId]) {
        let player_slots: Vec<_> = slots
            .iter()
            .filter_map(|slot| match slot {
                InventorySlotId::Player(index) => Some(*index),
                _ => None,
            })
            .collect();
        let crafting_slots: Vec<_> = slots
            .iter()
            .filter_map(|slot| match slot {
                InventorySlotId::CraftingInput(index) => Some(*index),
                _ => None,
            })
            .collect();
        if let Some(world) = self.world.as_mut() {
            distribute_carried_stack_evenly(
                &mut world.player_inventory,
                &mut self.inventory_cursor,
                &player_slots,
                &world.items,
            );
        }
        if !crafting_slots.is_empty() {
            let Some(world) = self.world.as_ref() else {
                return;
            };
            let items = world.items.clone();
            let mut cursor = self.inventory_cursor;
            distribute_carried_stack_evenly(
                self.active_crafting_grid_mut(),
                &mut cursor,
                &crafting_slots,
                &items,
            );
            self.inventory_cursor = cursor;
            self.update_crafting_result();
        }
    }

    fn preview_left_drag_distribution(&mut self) {
        let Some(drag) = self.inventory_drag.as_ref() else {
            return;
        };
        if drag.button != InventoryMouseButton::Left
            || !drag.changed_slots
            || drag.start_cursor.is_none()
        {
            return;
        }
        let slots = drag.slots.clone();
        self.restore_inventory_drag_snapshot();
        self.distribute_dragged_slots(&slots);
        if let Some(drag) = self.inventory_drag.as_mut() {
            drag.applied_drag = true;
        }
    }

    fn restore_inventory_drag_snapshot(&mut self) {
        let Some(drag) = self.inventory_drag.as_ref() else {
            return;
        };
        self.inventory_cursor = drag.start_cursor;
        if let Some(world) = self.world.as_mut() {
            world.player_inventory =
                Inventory::from_slots(drag.start_player_slots.clone(), INVENTORY_HOTBAR_SLOTS);
        }
        let restored_grid = Inventory::from_slots(drag.start_crafting_slots.clone(), 0);
        if self.crafting_table_open {
            self.crafting_table_grid = restored_grid;
        } else {
            self.inventory_crafting_grid = restored_grid;
        }
        self.update_crafting_result();
    }

    fn place_one_in_slot(&mut self, slot: InventorySlotId) -> bool {
        match slot {
            InventorySlotId::Player(slot) => self.world.as_mut().is_some_and(|world| {
                place_one_carried_item(
                    &mut world.player_inventory,
                    &mut self.inventory_cursor,
                    slot,
                    &world.items,
                )
            }),
            InventorySlotId::CraftingInput(slot) => {
                let Some(world) = self.world.as_ref() else {
                    return false;
                };
                let items = world.items.clone();
                let mut cursor = self.inventory_cursor;
                let placed = place_one_carried_item(
                    self.active_crafting_grid_mut(),
                    &mut cursor,
                    slot,
                    &items,
                );
                self.inventory_cursor = cursor;
                if placed {
                    self.update_crafting_result();
                }
                placed
            }
            InventorySlotId::CraftingResult => false,
        }
    }

    fn collect_matching_visible_stacks(&mut self, hovered_slot: InventorySlotId) {
        if self.inventory_cursor.is_none() {
            return;
        }
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let items = world.items.clone();
        match hovered_slot {
            InventorySlotId::Player(slot) => {
                if let Some(world) = self.world.as_mut() {
                    collect_matching_stacks(
                        &mut world.player_inventory,
                        &mut self.inventory_cursor,
                        Some(slot),
                        &items,
                    );
                }
                let mut cursor = self.inventory_cursor;
                collect_matching_stacks(self.active_crafting_grid_mut(), &mut cursor, None, &items);
                self.inventory_cursor = cursor;
            }
            InventorySlotId::CraftingInput(slot) => {
                let mut cursor = self.inventory_cursor;
                collect_matching_stacks(
                    self.active_crafting_grid_mut(),
                    &mut cursor,
                    Some(slot),
                    &items,
                );
                self.inventory_cursor = cursor;
                if let Some(world) = self.world.as_mut() {
                    collect_matching_stacks(
                        &mut world.player_inventory,
                        &mut self.inventory_cursor,
                        None,
                        &items,
                    );
                }
                self.update_crafting_result();
            }
            InventorySlotId::CraftingResult => {}
        }
    }

    fn is_double_click(&self, slot: InventorySlotId, button: InventoryMouseButton) -> bool {
        let Some(cursor) = self.inventory_cursor else {
            return false;
        };
        let _ = slot;
        if button != InventoryMouseButton::Left {
            return false;
        }
        self.last_inventory_click
            .is_some_and(|(time, last_button, item)| {
                last_button == button && item == cursor.item && time.elapsed().as_secs_f32() <= 0.35
            })
    }

    fn handle_inventory_keyboard(&mut self, event: &KeyEvent) -> bool {
        if let Some(index) = bound_hotbar_key(event, &self.key_bindings) {
            if let Some(slot) = self.inventory_slot_at_cursor() {
                self.swap_hovered_slot_with_hotbar(slot, index);
                return true;
            }
        }
        if self.shortcut_pressed(event, ShortcutAction::Drop) {
            self.drop_hovered_or_cursor_stack(self.modifier_control);
            return true;
        }
        if character_key(event, "f") {
            return true;
        }
        false
    }

    fn swap_hovered_slot_with_hotbar(&mut self, slot: InventorySlotId, hotbar_index: usize) {
        match slot {
            InventorySlotId::Player(slot) => {
                if let Some(world) = self.world.as_mut() {
                    swap_player_slots(&mut world.player_inventory, slot, hotbar_index);
                }
            }
            InventorySlotId::CraftingInput(slot) => {
                let Some(world) = self.world.as_ref() else {
                    return;
                };
                let hotbar_stack = world.player_inventory.slot(hotbar_index);
                let crafting_stack = self.active_crafting_grid().slot(slot);
                if let Some(world) = self.world.as_mut() {
                    world
                        .player_inventory
                        .set_slot(hotbar_index, crafting_stack);
                }
                self.active_crafting_grid_mut().set_slot(slot, hotbar_stack);
                self.update_crafting_result();
            }
            InventorySlotId::CraftingResult => {}
        }
    }

    fn drop_hovered_or_cursor_stack(&mut self, full_stack: bool) {
        let dropped = if self.inventory_cursor.is_some() {
            take_from_cursor(&mut self.inventory_cursor, full_stack)
        } else {
            match self.inventory_slot_at_cursor() {
                Some(InventorySlotId::Player(slot)) => self.world.as_mut().and_then(|world| {
                    take_from_slot(&mut world.player_inventory, slot, full_stack)
                }),
                Some(InventorySlotId::CraftingInput(slot)) => {
                    let dropped = take_from_slot(self.active_crafting_grid_mut(), slot, full_stack);
                    self.update_crafting_result();
                    dropped
                }
                _ => None,
            }
        };
        if let Some(stack) = dropped {
            self.drop_stack_near_player(stack);
        }
    }

    fn drop_stack_near_player(&mut self, stack: ItemStack) {
        if let Some(world) = self.world.as_mut() {
            world.spawn_dropped_stack(stack, self.camera.position, self.camera.forward());
        }
    }

    fn drop_selected_hotbar_stack(&mut self, full_stack: bool) {
        let Some(world) = self.world.as_mut() else {
            return;
        };
        if let Some(stack) = take_from_slot(
            &mut world.player_inventory,
            self.selected_hotbar_slot,
            full_stack,
        ) {
            world.spawn_dropped_stack(stack, self.camera.position, self.camera.forward());
        }
    }

    fn shortcut_pressed(&self, event: &KeyEvent, action: ShortcutAction) -> bool {
        if event.state != ElementState::Pressed {
            return false;
        }
        shortcut_label_for_event(event)
            .is_some_and(|label| self.key_bindings.matches(action, &label))
    }

    pub(super) fn hovered_inventory_stack(&self) -> Option<ItemStack> {
        self.stack_at_inventory_slot(self.inventory_slot_at_cursor()?)
    }

    fn stack_at_inventory_slot(&self, slot: InventorySlotId) -> Option<ItemStack> {
        match slot {
            InventorySlotId::Player(slot) => self.world.as_ref()?.player_inventory.slot(slot),
            InventorySlotId::CraftingInput(slot) => self.active_crafting_grid().slot(slot),
            InventorySlotId::CraftingResult => self.crafting_result,
        }
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

fn bound_hotbar_key(event: &KeyEvent, bindings: &KeyBindings) -> Option<usize> {
    let label = shortcut_label_for_event(event)?;
    let actions = [
        ShortcutAction::Hotbar1,
        ShortcutAction::Hotbar2,
        ShortcutAction::Hotbar3,
        ShortcutAction::Hotbar4,
        ShortcutAction::Hotbar5,
        ShortcutAction::Hotbar6,
        ShortcutAction::Hotbar7,
        ShortcutAction::Hotbar8,
        ShortcutAction::Hotbar9,
    ];
    actions
        .iter()
        .position(|action| bindings.matches(*action, &label))
}
