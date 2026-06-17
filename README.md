# HumanCraft

HumanCraft is an early Rust voxel survival game foundation. The current build
opens a native window with a main menu, saved worlds, generated voxel terrain,
and a controllable player camera.

## Run

From the project root:

```bash
cargo run
```

Controls:

- Main menu: click the center button or press `Enter` to open world management
- Main menu Settings: change gameplay shortcuts, saved in `saves/settings.txt`
- Manage worlds: `Enter` loads the selected world, `N` creates a world, `R`
  renames the selected world, `Delete` deletes it, and arrow keys change
  selection
- New world config: type a name and optional numeric seed; `Tab` switches
  fields and `Enter` creates the world
- `Z`: move forward
- `S`: move backward
- `Q`: move left
- `D`: move right
- Double-tap `Z`: sprint
- Hold `Shift`: sneak
- `Space`: jump
- Mouse: look around
- Left click or hold left click: break the targeted block
- Right click or hold right click: place the selected hotbar block, or open a
  crafting table when targeting one
- Left/Right arrows: change the selected hotbar slot
- `E`: open or close the inventory and crafting overlay
- `Esc`: open the pause overlay
- Pause overlay: `Keep Playing` resumes; `Save & Quit` writes player/chunk
  changes and returns to the main menu; Settings changes gameplay shortcuts
- Window close button: save and quit

## Debug Commands

```bash
cargo run -- preview
cargo run -- stats
cargo run -- play
```

`preview` writes:

- `out/preview/heightmap.txt`
- `out/preview/heightmap.ppm`
- `out/preview/chunk.obj`

`play` is a terminal-only debug mode. The normal way to run the game is
`cargo run`.

## Tests

```bash
cargo test
```

## Current Scope

Implemented:

- Registries
- Block and item definitions
- Chunk storage
- Terrain generation
- Ore generation
- Renderer-neutral chunk meshing
- Native `winit` + `wgpu` window
- ZQSD camera movement
- Captured mouse-look
- Grounded movement with Minecraft-style acceleration, gravity, jumping,
  sprinting, and sneaking
- Crosshair raycast block breaking and placing
- Selected-block outline
- Basic player collision AABB
- Directional face shading
- Main menu and saved world management
- Per-world generation seeds
- Saved player position and saved edited chunks
- Player inventory, hotbar selection, crafting grids, and dropped loot pickup
- Persistent shortcut rebinding for movement, inventory, pause, dropping, and
  hotbar selection

Not implemented yet:

- Health, hunger, armor/offhand slots, container inventories, and game mode
  persistence
