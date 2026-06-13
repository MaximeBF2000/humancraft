# HumanCraft

HumanCraft is an early Rust voxel survival game foundation. The current build
opens a native window with generated voxel terrain and a controllable camera.

## Run

From the project root:

```bash
cargo run
```

Controls:

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
- `Esc`: pause/resume and release/capture the mouse cursor
- Window close button: quit

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

Not implemented yet:

- Saving/loading
