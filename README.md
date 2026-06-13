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
- Left click: break the targeted block
- Right click: place dirt against the targeted block
- `Esc`: open the pause overlay
- Pause overlay: `Keep Playing` resumes; `Save & Quit` writes player/chunk
  changes and returns to the main menu
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

Not implemented yet:

- Inventory, health, hunger, and game mode persistence
