use crate::engine::world::save::{InventorySave, ItemStackSave};
use crate::engine::world::{Inventory, ItemId, ItemRegistry, ItemStack};

use super::constants::INVENTORY_HOTBAR_SLOTS;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum InventoryMouseButton {
    Left,
    Right,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) enum InventorySlotId {
    Player(usize),
    CraftingInput(usize),
    CraftingResult,
}

#[derive(Debug, Clone)]
pub(super) struct InventoryDrag {
    pub(super) button: InventoryMouseButton,
    pub(super) start_slot: Option<InventorySlotId>,
    pub(super) slots: Vec<InventorySlotId>,
    pub(super) changed_slots: bool,
    pub(super) applied_drag: bool,
    pub(super) start_cursor: Option<ItemStack>,
    pub(super) start_player_slots: Vec<Option<ItemStack>>,
    pub(super) start_crafting_slots: Vec<Option<ItemStack>>,
}

impl InventoryDrag {
    pub(super) fn new(
        button: InventoryMouseButton,
        start_slot: Option<InventorySlotId>,
        cursor: Option<ItemStack>,
        player_inventory: &Inventory,
        crafting_grid: &Inventory,
    ) -> Self {
        Self {
            button,
            start_slot,
            slots: Vec::new(),
            changed_slots: false,
            applied_drag: false,
            start_cursor: cursor,
            start_player_slots: player_inventory.slots().to_vec(),
            start_crafting_slots: crafting_grid.slots().to_vec(),
        }
    }

    pub(super) fn push_slot(&mut self, slot: InventorySlotId) -> bool {
        if self.slots.contains(&slot) {
            return false;
        }
        if self.start_slot.is_some_and(|start| start != slot) {
            self.changed_slots = true;
        }
        self.slots.push(slot);
        true
    }
}

pub(super) fn left_click_inventory_slot(
    inventory: &mut Inventory,
    cursor: &mut Option<ItemStack>,
    slot_index: usize,
    items: &ItemRegistry,
) {
    let slot = inventory.slot(slot_index);
    match (*cursor, slot) {
        (None, None) => {}
        (None, Some(stack)) => {
            inventory.set_slot(slot_index, None);
            *cursor = Some(stack);
        }
        (Some(held), None) => {
            inventory.set_slot(slot_index, Some(held));
            *cursor = None;
        }
        (Some(mut held), Some(mut slot_stack)) if held.item == slot_stack.item => {
            let max_stack_size = max_stack_size(held.item, items);
            let moved = held
                .count
                .min(max_stack_size.saturating_sub(slot_stack.count));
            slot_stack.count += moved;
            held.count -= moved;
            inventory.set_slot(slot_index, Some(slot_stack));
            *cursor = if held.count == 0 { None } else { Some(held) };
        }
        (Some(held), Some(slot_stack)) => {
            inventory.set_slot(slot_index, Some(held));
            *cursor = Some(slot_stack);
        }
    }
}

pub(super) fn right_click_inventory_slot(
    inventory: &mut Inventory,
    cursor: &mut Option<ItemStack>,
    slot_index: usize,
    items: &ItemRegistry,
) {
    if cursor.is_some() {
        place_one_carried_item(inventory, cursor, slot_index, items);
        return;
    }

    let Some(mut stack) = inventory.slot(slot_index) else {
        return;
    };
    let picked = stack.count.div_ceil(2);
    stack.count -= picked;
    inventory.set_slot(
        slot_index,
        if stack.count == 0 { None } else { Some(stack) },
    );
    *cursor = Some(ItemStack::new(stack.item, picked));
}

pub(super) fn place_one_carried_item(
    inventory: &mut Inventory,
    cursor: &mut Option<ItemStack>,
    slot_index: usize,
    items: &ItemRegistry,
) -> bool {
    let Some(mut held) = *cursor else {
        return false;
    };
    let slot = inventory.slot(slot_index);
    match slot {
        None => {
            inventory.set_slot(slot_index, Some(ItemStack::new(held.item, 1)));
            held.count -= 1;
        }
        Some(mut slot_stack)
            if slot_stack.item == held.item
                && slot_stack.count < max_stack_size(held.item, items) =>
        {
            slot_stack.count += 1;
            inventory.set_slot(slot_index, Some(slot_stack));
            held.count -= 1;
        }
        _ => return false,
    }

    *cursor = if held.count == 0 { None } else { Some(held) };
    true
}

pub(super) fn distribute_carried_stack_evenly(
    inventory: &mut Inventory,
    cursor: &mut Option<ItemStack>,
    slots: &[usize],
    items: &ItemRegistry,
) {
    let Some(held) = *cursor else {
        return;
    };
    let mut targets: Vec<_> = slots
        .iter()
        .copied()
        .filter(|slot| can_accept_item(inventory.slot(*slot), held.item, items))
        .collect();
    targets.sort_unstable();
    targets.dedup();

    while cursor.is_some() {
        let mut moved_any = false;
        for slot in &targets {
            if place_one_carried_item(inventory, cursor, *slot, items) {
                moved_any = true;
            }
            if cursor.is_none() {
                return;
            }
        }
        if !moved_any {
            return;
        }
    }
}

pub(super) fn collect_matching_stacks(
    inventory: &mut Inventory,
    cursor: &mut Option<ItemStack>,
    hovered_slot: Option<usize>,
    items: &ItemRegistry,
) {
    let Some(mut held) = *cursor else {
        return;
    };
    let max_stack_size = max_stack_size(held.item, items);
    if held.count >= max_stack_size {
        return;
    }

    if let Some(slot) = hovered_slot {
        collect_from_slot(inventory, &mut held, slot, max_stack_size);
    }
    for slot in 0..inventory.slot_count() {
        if held.count >= max_stack_size {
            break;
        }
        if Some(slot) == hovered_slot {
            continue;
        }
        collect_from_slot(inventory, &mut held, slot, max_stack_size);
    }
    *cursor = Some(held);
}

fn collect_from_slot(
    inventory: &mut Inventory,
    held: &mut ItemStack,
    slot_index: usize,
    max_stack_size: u16,
) {
    let Some(mut stack) = inventory.slot(slot_index) else {
        return;
    };
    if stack.item != held.item {
        return;
    }
    let moved = stack.count.min(max_stack_size.saturating_sub(held.count));
    if moved == 0 {
        return;
    }
    held.count += moved;
    stack.count -= moved;
    inventory.set_slot(
        slot_index,
        if stack.count == 0 { None } else { Some(stack) },
    );
}

pub(super) fn quick_transfer_player_slot(
    inventory: &mut Inventory,
    slot_index: usize,
    items: &ItemRegistry,
) -> bool {
    let Some(stack) = inventory.slot(slot_index) else {
        return false;
    };
    inventory.set_slot(slot_index, None);
    let target_slots = if slot_index < INVENTORY_HOTBAR_SLOTS {
        INVENTORY_HOTBAR_SLOTS..inventory.slot_count()
    } else {
        0..INVENTORY_HOTBAR_SLOTS
    };
    let remainder = move_stack_into_slots(inventory, stack, target_slots, items);
    inventory.set_slot(slot_index, remainder);
    remainder != Some(stack)
}

pub(super) fn move_stack_into_player_inventory(
    inventory: &mut Inventory,
    stack: ItemStack,
    items: &ItemRegistry,
) -> Option<ItemStack> {
    move_stack_into_slots(inventory, stack, 0..inventory.slot_count(), items)
}

fn move_stack_into_slots(
    inventory: &mut Inventory,
    mut stack: ItemStack,
    slots: std::ops::Range<usize>,
    items: &ItemRegistry,
) -> Option<ItemStack> {
    if stack.is_empty() {
        return None;
    }
    let max_stack_size = max_stack_size(stack.item, items);

    for slot_index in slots.clone() {
        let Some(mut existing) = inventory.slot(slot_index) else {
            continue;
        };
        if existing.item != stack.item || existing.count >= max_stack_size {
            continue;
        }
        let moved = stack.count.min(max_stack_size - existing.count);
        existing.count += moved;
        stack.count -= moved;
        inventory.set_slot(slot_index, Some(existing));
        if stack.count == 0 {
            return None;
        }
    }

    for slot_index in slots {
        if inventory.slot(slot_index).is_some() {
            continue;
        }
        let moved = stack.count.min(max_stack_size);
        inventory.set_slot(slot_index, Some(ItemStack::new(stack.item, moved)));
        stack.count -= moved;
        if stack.count == 0 {
            return None;
        }
    }

    Some(stack)
}

pub(super) fn swap_player_slots(inventory: &mut Inventory, left: usize, right: usize) -> bool {
    if left == right {
        return false;
    }
    let left_stack = inventory.slot(left);
    let right_stack = inventory.slot(right);
    inventory.set_slot(left, right_stack);
    inventory.set_slot(right, left_stack);
    left_stack != right_stack
}

pub(super) fn take_from_slot(
    inventory: &mut Inventory,
    slot_index: usize,
    full_stack: bool,
) -> Option<ItemStack> {
    let mut stack = inventory.slot(slot_index)?;
    if full_stack || stack.count == 1 {
        inventory.set_slot(slot_index, None);
        return Some(stack);
    }
    stack.count -= 1;
    inventory.set_slot(slot_index, Some(stack));
    Some(ItemStack::new(stack.item, 1))
}

pub(super) fn take_from_cursor(
    cursor: &mut Option<ItemStack>,
    full_stack: bool,
) -> Option<ItemStack> {
    let mut stack = (*cursor)?;
    if full_stack || stack.count == 1 {
        *cursor = None;
        return Some(stack);
    }
    stack.count -= 1;
    *cursor = Some(stack);
    Some(ItemStack::new(stack.item, 1))
}

fn can_accept_item(slot: Option<ItemStack>, item: ItemId, items: &ItemRegistry) -> bool {
    match slot {
        None => true,
        Some(stack) => stack.item == item && stack.count < max_stack_size(item, items),
    }
}

fn max_stack_size(item: ItemId, items: &ItemRegistry) -> u16 {
    items
        .get(item)
        .map(|definition| definition.max_stack_size)
        .unwrap_or(64)
}

pub(super) fn inventory_to_save(inventory: &Inventory, items: &ItemRegistry) -> InventorySave {
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

pub(super) fn inventory_from_save(save: &InventorySave, items: &ItemRegistry) -> Inventory {
    let mut slots: Vec<_> = save
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
    if slots.len() < Inventory::player().slots().len() {
        slots.resize(Inventory::player().slots().len(), None);
    }
    Inventory::from_slots(slots, INVENTORY_HOTBAR_SLOTS)
}
