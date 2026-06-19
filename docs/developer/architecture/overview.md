# HumanCraft Architecture Overview

HumanCraft is split into engine systems and game content from the first
implementation slice.

## Current Shape

- `src/engine`: reusable systems that must not know HumanCraft-specific blocks,
  items, biomes, entities, or recipes.
- `src/content`: HumanCraft bootstrap content that registers concrete
  definitions through engine registries.
- `src/app/windowed.rs`: native client shell for window events, GPU resources,
  render orchestration, and menu flow.
- `src/app/windowed/`: private windowed-client modules for loaded client-world
  state, inventory interaction helpers, and explicit tuning constants. See
  `docs/developer/systems/windowed_client.md`.
- `src/main.rs`: temporary CLI bootstrap used to prove the engine can register
  content and generate a sample chunk.

## Design Rules

- Blocks and items are IDs at runtime.
- Properties live in definitions stored in registries.
- Inventory, crafting, and loot store generic item stacks, not content-specific
  values.
- Chunks store `BlockState` values so placement-specific properties such as
  facing, slab orientation, stair half, furnace lit state, leaf persistence, and
  sapling growth stage stay separate from block definitions.
- Reusable block behavior data lives on block definitions. The current
  windowed-client tick system consumes that data for gravity, leaf decay, grass
  spread, and sapling growth without hard-coding individual block keys.
- Generation is a pipeline of independent stages.
- Rendering is not allowed to own gameplay logic.
- Content is allowed to name `humancraft:*` keys; engine code is not.

## Near-Term Direction

The next major boundary is UI/rendering inside the windowed client. Texture
atlas loading, menu state, and UI mesh construction should move out of the app
shell as cohesive modules while preserving the current gameplay behavior.
