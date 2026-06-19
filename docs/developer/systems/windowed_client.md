# Windowed Client Structure

The windowed client is still the main development harness for gameplay, input,
rendering, and save integration. Keep its modules split by ownership so new
features do not accumulate in the event-loop shell.

## Modules

- `src/app/windowed.rs`: winit/wgpu app shell, GPU pipeline setup, top-level
  `RenderState` storage, and application-handler wiring.
- `src/app/windowed/app_input.rs`: keyboard, mouse, pause, inventory/crafting
  cursor, and held block interaction handling for `RenderState`.
- `src/app/windowed/block_behaviors.rs`: loaded-chunk block behavior ticking
  driven by block definition metadata. It currently handles gravity, generated
  leaf decay and sapling drops, grass spreading, and sapling growth.
- `src/app/windowed/block_break_overlay.rs`: textured block damage overlay
  mesh generation using the default `destroy_stage_0` through
  `destroy_stage_9` textures for the current break target.
- `src/app/windowed/camera.rs`: player camera orientation, fixed-tick movement,
  collision-driven movement, sprinting, sneaking, and save conversion.
- `src/app/windowed/client_world.rs`: loaded client chunks, save-backed chunk
  streaming, raycasts, placement context construction, hardness-based block
  break progress, block edits, block entities, furnace ticking, and chunk mesh
  preparation. It also converts runtime chest/furnace block entities to and
  from the generic save representation.
- `src/app/windowed/input.rs`: in-game movement input state driven by the
  active shortcut bindings.
- `src/app/windowed/frame.rs`: per-frame gameplay update, block interaction
  continuation, fixed-rate block entity ticking, chunk remesh queue
  processing, render pass construction, and target overlay updates.
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
  for movement, chunk budgets, world fog, inventory layout, block-interaction
  cadence, and loot behavior.
- `src/app/windowed/world_lifecycle.rs`: world menu clicks, save create/rename/
  delete/load, active-world flush including placed block entities,
  save-and-quit, and window-title updates.
- `src/app/windowed/world_render.rs`: per-chunk GPU buffers, loaded-chunk
  deduplication, renderer mesh conversion, and temporary preview boundary
  filtering.
- `src/app/windowed/tests.rs`: windowed-client regression tests kept outside
  the app shell.
- `src/app/windowed/block_behavior_tests.rs`: focused regression tests for the
  reusable block behavior system.

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
- Keep chunk-streaming values explicit in `constants.rs`. The current
  windowed client keeps an 11 x 11 chunk area resident, generates at most three
  new chunks per frame, rebuilds at most four chunk meshes per frame, and sorts
  pending chunk work by distance with a camera-forward bias.
- Keep coarse chunk draw culling renderer-local. The current renderer always
  draws nearby chunks, then skips farther chunk buffers outside a broad
  horizontal forward cone while leaving those chunks loaded and meshed.
- Keep the GPU chunk-buffer working set bounded to the active render radius.
  Chunks remain resident in `ClientWorld` for gameplay and save safety, but
  terrain buffers outside the active radius are released and rebuilt through
  the normal remesh budget when the player returns.
- Terrain fog is a renderer-side loading mask. The world shader blends distant
  terrain into the shared sky color from `WORLD_FOG_START_BLOCKS` to
  `WORLD_FOG_END_BLOCKS`, capped by `WORLD_FOG_MAX_AMOUNT`; keep those values
  aligned with render distance changes.
- Keep files under 250 lines by default. If a file is already over that limit,
  extract a cohesive responsibility before adding more behavior to it.
- Prefer moving cohesive private helpers into a module before adding more
  behavior to `windowed.rs`.

## Remaining Oversized Files

- `src/app/windowed.rs` is still larger than the long-term target because GPU
  pipeline setup remains in one constructor. Extract renderer pipeline setup
  before adding more rendering features.
- `src/app/windowed/tests.rs` should be split by domain as more regression
  tests are added. New block behavior coverage now lives in
  `block_behavior_tests.rs`.
- `src/app/windowed/inventory_ui.rs`, `client_world.rs`, `app_input.rs`,
  `frame.rs`, `world_lifecycle.rs`, `texture.rs`, and `ui_builder.rs` remain
  above the default 250-line target. `client_world.rs` no longer owns the new
  custom block behavior loop, but still needs further extraction around block
  entity persistence and placement/breaking helpers. `app_input.rs` and
  `inventory_ui.rs` remain priority refactor candidates before more input or UI
  features are added.
