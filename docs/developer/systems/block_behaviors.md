# Block Behaviors

## Purpose

Block behaviors are reusable mechanics declared on `BlockDefinition` data and
ticked by the windowed client while chunks are loaded.

The current implementation keeps behavior code in
`src/app/windowed/block_behaviors.rs` because loaded-chunk mutation and loot
spawning still live in `ClientWorld`. The data shape lives in
`src/engine/world/block.rs` so behavior definitions can move deeper into engine
systems later without changing content registration.

## Current Behaviors

- Gravity: blocks with `behavior.gravity` move downward through air or
  replaceable/non-solid blocks. Sand uses this behavior. The current cell-based
  implementation keeps per-block falling motion and applies the vanilla-style
  falling-block constants of `0.04` blocks/tick acceleration and `0.98` drag,
  moving the stored block down only after accumulated sub-block movement reaches
  a full block.
- Leaf decay: generated leaves with `BlockProperties::Leaves {
  persistent: false }` decay on random block ticks when not connected to a log
  tag through leaf blocks within the configured distance. Oak leaves use the
  Java-style maximum distance of 6 and a HumanCraft-tuned 15% oak sapling drop
  chance. Player placed leaves use `persistent: true` and do not decay.
- Grass spread: grass random ticks try four nearby positions in a 3 x 5 x 3
  range and convert clear dirt blocks to grass.
- Sapling growth: saplings have stage 0 and stage 1 states. A random tick first
  advances stage 0 to stage 1; a later tick grows a tree if the configured soil
  and clearance rules pass. Oak saplings require soil below and at least five
  clear blocks in a 3 x 3 column above.

## Content Data

HumanCraft declares the first behavior users in `src/content/blocks.rs`:

- `humancraft:sand`: gravity.
- `humancraft:grass`: grass spread to `humancraft:dirt`.
- `humancraft:oak_leaves`: leaf decay with an oak sapling chance drop.
- `humancraft:oak_sapling`: sapling growth into oak logs and oak leaves.

Use behavior metadata before adding block-key branches. If a future block needs
to fall, decay, spread, or grow, attach the existing behavior data to its
definition first.

## Persistence

Chunks persist leaf and sapling state through the `HCCNK003` block-state palette:

- `Leaves { persistent }`
- `Sapling { stage }`

Tree generation writes non-persistent leaf states. Player placement writes
persistent leaf states and stage-0 sapling states.

## Reference Behavior

The current values are aligned with Minecraft Java behavior documented on the
Minecraft Wiki/Fandom pages for leaves, grass blocks, saplings, and sand:

- Oak and most non-jungle leaves drop saplings 5% of the time in Minecraft.
  HumanCraft currently uses 15% for playability.
- Java leaf decay uses connection to logs through leaves up to distance 6, and
  player-placed leaves do not decay.
- Grass spreads from random-ticked grass to dirt in a 3 x 5 x 3 range when the
  destination is clear above.
- Oak saplings use two growth stages and need at least five spaces above in a
  3 x 3 column.
- Sand falls when unsupported. HumanCraft approximates vanilla falling block
  motion with `0.04` blocks/tick acceleration and `0.98` drag.
