# Changelog

All notable project changes are tracked here. `PROGRESS.md` remains the working
status file for verification notes, next steps, and implementation details.

## Unreleased

### Latest Session Summary

- Read the project PRD, development philosophy, and progress notes before
  continuing implementation.
- Generated and added stone block face textures under `textures/blocks/stone`
  with the `create-block-texture` skill.
- Added block texture metadata through `BlockTextures` and registered texture
  references for grass, dirt, and stone.
- Added PNG texture loading with the `image` crate.
- Built the first renderer-side block texture atlas for the windowed `wgpu`
  client.
- Extended world render vertices with UV coordinates while keeping chunk
  meshing renderer-neutral.
- Updated the terrain shader to sample block textures with directional face
  shading.
- Fixed texture path resolution so assets load from the project root regardless
  of the process working directory.
- Fixed atlas sampling and lifetime details by insetting UVs by half a texel
  and keeping the underlying GPU atlas texture alive.
- Fixed the shader pipeline swap that made terrain render as flat white/dark
  gray faces even though the atlas had loaded correctly.
- Replaced unsafe spawn placement with a safe spawn search that validates the
  full player AABB before accepting a spawn position.
- Extended visible-face culling across loaded chunk boundaries in the windowed
  renderer, including re-exposing faces after neighboring border blocks are
  removed.
- Updated `PROGRESS.md` throughout the work with implementation notes,
  verification results, and next steps.
- Added and ran tests covering texture metadata mapping, real stone PNG
  loading, and safe spawn placement.

### Project Setup

- Initialized the Rust Cargo project as `humancraft`.
- Added the library/application split.
- Added `.gitignore` for Cargo build output and generated preview output.
- Added `README.md` with current run, debug, and test commands.
- Added `PROGRESS.md` for ongoing implementation tracking.

### Core Engine

- Added generic registry primitives with duplicate-key protection.
- Added block and item definitions backed by registries.
- Added fixed-size chunk storage for `16 x 16 x 256` block IDs.
- Added a composable world generation pipeline.
- Added deterministic terrain generation.
- Added generic ore generation driven by ore definitions.
- Added initial content registration for air, grass, dirt, stone, coal ore,
  iron ore, gold ore, diamond ore, and starter item definitions.

### World Rendering

- Added renderer-neutral chunk meshing with visible-face culling.
- Added preview artifact export for generated chunks:
  - ASCII heightmap
  - PPM heightmap
  - OBJ chunk mesh
- Added native `winit` + `wgpu` windowed dev mode.
- Added colored terrain rendering.
- Changed windowed rendering from one exposed chunk to a centered `5 x 5`
  terrain patch.
- Hid deep underground side faces and outer render-patch boundary faces so the
  temporary finite terrain area does not render as a sliced column.
- Switched the camera projection to the depth range expected by `wgpu`.
- Added directional face shading for clearer untextured block faces.
- Added PNG loading through the `image` crate.
- Added reusable block texture metadata via `BlockTextures`.
- Registered texture metadata for grass, dirt, and stone.
- Generated `textures/blocks/stone` with the `create-block-texture` skill.
- Added a simple renderer-side block texture atlas.
- Extended world render vertices with UV coordinates while keeping chunk
  meshing renderer-neutral.
- Updated the terrain shader to sample block textures while preserving
  directional face shading.
- Added fallback texture handling for blocks without PNG assets.
- Fixed texture path resolution to use `CARGO_MANIFEST_DIR`.
- Inset atlas UVs by half a texel to reduce texture bleeding.
- Fixed a shader pipeline swap that caused textured terrain to still render as
  flat shaded faces.
- Kept the GPU atlas texture alive inside `TextureAtlas`.
- Added startup atlas accounting for loaded texture keys and fallback use.
- Extended the renderer-neutral chunk mesher with an optional outside-neighbor
  lookup so systems can cull hidden faces across chunk boundaries without
  moving visibility rules into GPU code.

### Player And Interaction

- Added a terminal playtest mode for early movement, mining, and placing.
- Added French-layout `ZQSD` movement using logical keyboard characters.
- Replaced arrow-key camera control with captured mouse-look.
- Added `Esc` pause/menu behavior that releases and recaptures the mouse cursor.
- Added a persistent windowed client world that stores generated chunks.
- Added grounded player-style movement with gravity and `Space` jump.
- Replaced height-only ground collision with an axis-separated player AABB.
- Added center crosshair rendering.
- Added camera raycasting against blocks.
- Added selected-block outline rendering for the current raycast target.
- Added left-click block breaking and right-click dirt placement.
- Rebuilt the chunk render mesh after block edits.
- Replaced initial spawn placement with a safe spawn search that validates the
  full player AABB.

### Skills And Tooling

- Added and repaired the `create-block-texture` skill under
  `.agents/skills/create-block-texture`.
- Moved the block texture generator into the standard skill resource path:
  `.agents/skills/create-block-texture/scripts/generate_block_texture.py`.
- Added `.agents/skills/create-block-texture/agents/openai.yaml`.
- Validated the `create-block-texture` skill with `quick_validate.py`.
- Smoke-tested the moved texture generator and confirmed it emits six `16 x 16`
  RGBA PNG block faces.

### Documentation

- Added developer documentation for:
  - architecture overview
  - registry system
  - chunk system
  - world generation
  - chunk meshing
  - terminal playtest
- Updated `README.md` as the run loop evolved.

### Tests And Verification

- Added unit tests for registries, chunks, generation pipeline, terrain, ores,
  content bootstrap, meshing, preview export, terminal playtest, AZERTY input,
  render filtering, coordinate splitting, mutable client-world block edits,
  raycasting, player AABB collision, selected-block outline geometry, texture
  metadata mapping, texture loading, and safe spawn placement.
- Verified the project repeatedly with:
  - `cargo fmt`
  - `cargo check`
  - `cargo test`
  - `cargo run -- preview`
  - `cargo run`
