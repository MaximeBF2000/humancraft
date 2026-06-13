# World Generation System

## Purpose

World generation composes independent stages into deterministic chunk
generation.

## Current Stages

- `TerrainStage`: fills terrain columns from biome terrain profiles using
  deterministic value noise.
- `OreStage`: places arbitrary ore definitions inside matching host blocks.
- `TreeStage`: places arbitrary tree definitions in allowed biomes after base
  terrain and ores are generated.

## Biomes

- `BiomeSource`: deterministic biome lookup from seed and world coordinates.
- `BiomeDefinition`: data for one biome's terrain surface, subsurface, stone
  host block, height range, soil depth, and terrain noise scale.
- Biome regions are macro-cells measured in chunks. The current overworld uses
  10-chunk regions with a 2-chunk transition band.
- Inside the core of a biome region, one biome owns terrain and decorations.
  Inside a transition band, nearby region biome profiles blend their height
  contributions.
- Current HumanCraft content defines:
  - `humancraft:plains`
  - `humancraft:forest`
  - `humancraft:mountains`

## Terrain Continuity

- Terrain noise is interpolated between lattice samples instead of reading raw
  random values per cell.
- Terrain height is computed from blended biome influences, so neighboring
  chunks use the same continuous world-coordinate height function.
- Tests enforce that heights stay continuous across chunk borders for the
  current terrain generator.

## Responsibilities

- Start each chunk filled with air.
- Apply stages in configured order.
- Keep stage behavior generic.
- Let content supply block IDs, biome profiles, ore distribution definitions,
  and tree definitions.
- Generate chunks deterministically from world seed and chunk position so client
  systems can request new terrain as the player moves.

## Inputs

- `GenerationContext`
- `ChunkPosition`
- Registered block IDs supplied by content bootstrap.

## Outputs

- Populated `Chunk`

## Dependencies

- Chunk system.
- Block IDs from block registry bootstrap.

## Extension Points

- Replace temporary value noise with the planned `noise` crate.
- Add cave, water, decoration, and structure stages.
- Allow tree and structure generators to write across chunk boundaries through a
  feature-placement or deferred-decoration pass.
- Move generation settings to serialized data files.
- Add chunk streaming policy for unloading, persistence, and save/load.

## Known Limitations

- Noise is intentionally simple and dependency-free.
- Terrain noise interpolation is intentionally simple and should eventually be
  replaced with a proper noise library.
- Ore distribution is value-noise based, not vein based.
- Tree placement clips origins away from chunk edges to avoid partial trees
  until cross-chunk decoration exists.
- There are no caves, water, or non-tree decorations yet.
- The windowed client keeps generated chunks resident; there is no unload or
  persistence policy yet.
