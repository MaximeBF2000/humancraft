# HumanCraft Architecture Overview

HumanCraft is split into engine systems and game content from the first
implementation slice.

## Current Shape

- `src/engine`: reusable systems that must not know HumanCraft-specific blocks,
  items, biomes, entities, or recipes.
- `src/content`: HumanCraft bootstrap content that registers concrete
  definitions through engine registries.
- `src/main.rs`: temporary CLI bootstrap used to prove the engine can register
  content and generate a sample chunk.

## Design Rules

- Blocks and items are IDs at runtime.
- Properties live in definitions stored in registries.
- Generation is a pipeline of independent stages.
- Rendering is not allowed to own gameplay logic.
- Content is allowed to name `humancraft:*` keys; engine code is not.

## Near-Term Direction

The next major boundary is rendering. The renderer should consume chunk state
and block definitions, then produce meshes and GPU buffers without mutating
world logic.
