# World Generation System

## Purpose

World generation composes independent stages into deterministic chunk
generation.

## Current Stages

- `TerrainStage`: fills terrain columns from biome terrain profiles using
  deterministic value noise.
- `OreStage`: places arbitrary ore definitions inside matching host blocks.
- `BedrockStage`: guarantees the bottom chunk layer is an unbroken bedrock
  layer.
- `TreeStage`: places arbitrary tree definitions in allowed biomes after base
  terrain and ores are generated.

## Biomes

- `BiomeSource`: deterministic biome lookup from seed and world coordinates.
- `BiomeDefinition`: data for one biome's terrain layer stack, fallback stone
  block, height range, terrain noise scale, relief controls, ridge controls,
  and optional exposed-surface rules.
- Terrain layer stacks are read from the surface downward. Plains and forest
  use grass then dirt then stone. Desert uses sand, then sandstone, then stone.
- Current HumanCraft terrain is configured so sampled overworld surfaces start
  at Y 64 or higher; mountains are substantially higher.
- Biomes can add secondary roughness and ridge noise on top of base height
  variation. Mountains use stronger relief and ridge settings, deserts use
  dune-like relief, and plains/forests use gentler rolling relief.
- Mountains expose stone on steep surfaces. High flatter mountaintops remain
  grassy, so stone reads as cliff/rough-slope material rather than replacing
  every mountain top.
- Biome regions are macro-cells measured in chunks. The current overworld uses
  10-chunk regions with a 2-chunk transition band.
- Inside the core of a biome region, one biome owns terrain and decorations.
  Inside a transition band, nearby region biome profiles blend their height
  contributions.
- Current HumanCraft content defines:
  - `humancraft:plains`
  - `humancraft:forest`
  - `humancraft:mountains`
  - `humancraft:desert`

## Terrain Continuity

- Terrain noise is interpolated between lattice samples instead of reading raw
  random values per cell.
- Terrain height is computed from blended biome influences, so neighboring
  chunks use the same continuous world-coordinate height function.
- Tests enforce that heights stay continuous across chunk borders for the
  current terrain generator.
- Tests also enforce minimum biome height variation and exposed mountain stone
  surfaces for the current HumanCraft content.

## Responsibilities

- Start each chunk filled with air.
- Apply stages in configured order.
- Keep stage behavior generic.
- Let content supply block IDs, biome profiles, ore distribution definitions,
  bedrock block, and tree definitions.
- Guarantee the bottom Y layer is bedrock after terrain and ores have run.
- Generate chunks deterministically from world seed and chunk position so client
  systems can request new terrain as the player moves.
- Treat generation as the fallback source of chunk data. When a saved chunk
  exists for a world, the windowed client loads that chunk instead of
  regenerating it.

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
- Add chunk unloading and background save scheduling.

## Known Limitations

- Noise is intentionally simple and dependency-free.
- Terrain noise interpolation is intentionally simple and should eventually be
  replaced with a proper noise library.
- Ore distribution is value-noise based, not vein based.
- Tree placement clips origins away from chunk edges to avoid partial trees
  until cross-chunk decoration exists.
- There are no caves, water, or non-tree decorations yet.
- The windowed client keeps generated chunks resident; there is no unload
  policy yet.
