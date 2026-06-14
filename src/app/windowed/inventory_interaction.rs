use crate::engine::world::save::{InventorySave, ItemStackSave};
use crate::engine::world::{Inventory, ItemId, ItemRegistry, ItemStack};

use super::constants::INVENTORY_HOTBAR_SLOTS;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum InventoryMouseButton {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub(super) struct InventoryDrag {
    pub(super) button: InventoryMouseButton,
    pub(super) start_slot: Option<usize>,
    pub(super) slots: Vec<usize>,
    pub(super) changed_slots: bool,
    pub(super) applied_drag: bool,
}

impl InventoryDrag {
    pub(super) fn new(button: InventoryMouseButton, start_slot: Option<usize>) -> Self {
        Self {
            button,
            start_slot,
            slots: Vec::new(),
            changed_slots: false,
            applied_drag: false,
        }
    }

    pub(super) fn push_slot(&mut self, slot: usize) -> bool {
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
