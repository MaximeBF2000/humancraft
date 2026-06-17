# Windowed Client Structure

The windowed client is still the main development harness for gameplay, input,
rendering, and save integration. Keep its modules split by ownership so new
features do not accumulate in the event-loop shell.

## Modules

- `src/app/windowed.rs`: winit/wgpu app shell, GPU pipeline setup, top-level
  `RenderState` storage, and application-handler wiring.
- `src/app/windowed/app_input.rs`: keyboard, mouse, pause, inventory/crafting
  cursor, and held block interaction handling for `RenderState`.
- `src/app/windowed/block_break_overlay.rs`: textured block damage overlay
  mesh generation using the default `destroy_stage_0` through
  `destroy_stage_9` textures for the current break target.
- `src/app/windowed/camera.rs`: player camera orientation, fixed-tick movement,
  collision-driven movement, sprinting, sneaking, and save conversion.
- `src/app/windowed/client_world.rs`: loaded client chunks, save-backed chunk
  streaming, raycasts, hardness-based block break progress, block edits, and
  chunk mesh preparation.
- `src/app/windowed/input.rs`: in-game movement input state driven by the
  active shortcut bindings.
- `src/app/windowed/frame.rs`: per-frame gameplay update, block interaction
  continuation, chunk remesh queue processing, render pass construction, and
  target overlay updates.
- `src/app/windowed/hud.rs`: crosshair and selected-block outline geometry.
- `src/app/windowed/inventory_ui.rs`: hotbar, inventory/crafting overlays,
  item icon, held item/block, player arm, and dropped-loot billboard mesh
  construction.
- `src/app/windowed/player_collision.rs`: safe spawn search, player AABB
  collision checks, ground support checks, and occupied-block placement
  rejection.
- `src/app/windowed/render_types.rs`: shared GPU vertex and camera-uniform
  structs.
- `src/app/windowed/session.rs`: app mode, world-name text entry, new-world
  config state, shortcut binding data, settings/shortcut menu state, and held
  block interaction cadence.
- `src/app/windowed/settings.rs`: durable client settings storage for shortcut
  bindings under `saves/settings.txt`.
- `src/app/windowed/spatial.rs`: render/world coordinate conversion, world block
  positions, player AABB construction, and chunk-neighbor dirtying helpers.
- `src/app/windowed/loot.rs`: dropped item spawning, gravity/drag updates, block
  collision, rotation, and pickup into the player inventory.
- `src/app/windowed/shaders.rs`: WGSL source strings used by the current
  renderer pipelines.
- `src/app/windowed/texture.rs`: renderer-side texture atlas construction,
  texture-key resolution for blocks, items, destroy overlays, and the
  first-person hand overlay, fallback pixels, and depth texture creation.
- `src/app/windowed/ui.rs`: menu UI primitives, menu hit targets, and menu mesh
  construction.
- `src/app/windowed/ui_builder.rs`: reusable solid-color UI mesh builder and
  bitmap glyph drawing.
- `src/app/windowed/inventory_interaction.rs`: typed inventory slot IDs, click,
  drag, cursor-stack, quick-transfer, double-click collect, hotbar swap, drop,
  and save conversion helpers.
- `src/app/windowed/constants.rs`: explicit windowed-client tuning constants
  for movement, chunk budgets, inventory layout, block-interaction cadence, and
  loot behavior.
- `src/app/windowed/world_lifecycle.rs`: world menu clicks, save create/rename/
  delete/load, active-world flush, save-and-quit, and window-title updates.
- `src/app/windowed/world_render.rs`: per-chunk GPU buffers, loaded-chunk
  deduplication, renderer mesh conversion, and temporary preview boundary
  filtering.
- `src/app/windowed/tests.rs`: windowed-client regression tests kept outside
  the app shell.

## Rules

- Keep renderer-owned code responsible for GPU resources and draw ordering.
- Keep world mutation in `ClientWorld` or an engine system, not in mesh/UI
  builders.
- Keep inventory stack semantics in `inventory_interaction.rs`; render code
  should only ask for slot rectangles and icon meshes.
- Keep constants explicit. If a value affects player-facing behavior, update
  player and developer docs when changing it.
- Keep keyboard shortcut behavior routed through `KeyBindings` instead of
  adding new hardcoded logical-key checks. Persist shortcut changes through
  `SettingsStore`, not world metadata.
- Keep files under 250 lines by default. If a file is already over that limit,
  extract a cohesive responsibility before adding more behavior to it.
- Prefer moving cohesive private helpers into a module before adding more
  behavior to `windowed.rs`.

## Remaining Oversized Files

- `src/app/windowed.rs` is still larger than the long-term target because GPU
  pipeline setup remains in one constructor. Extract renderer pipeline setup
  before adding more rendering features.
- `src/app/windowed/tests.rs` should be split by domain as more regression
  tests are added.
- `src/app/windowed/inventory_ui.rs`, `client_world.rs`, `app_input.rs`,
  `frame.rs`, `world_lifecycle.rs`, `texture.rs`, and `ui_builder.rs` remain
  above the default 250-line target. Treat them as extraction candidates before
  adding new behavior in those areas.
