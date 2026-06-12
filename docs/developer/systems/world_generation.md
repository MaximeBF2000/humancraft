# World Generation System

## Purpose

World generation composes independent stages into deterministic chunk
generation.

## Current Stages

- `TerrainStage`: fills stone, dirt, and grass using deterministic value noise.
- `OreStage`: places arbitrary ore definitions inside matching host blocks.

## Responsibilities

- Start each chunk filled with air.
- Apply stages in configured order.
- Keep stage behavior generic.
- Let content supply block IDs and ore distribution definitions.
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
- Add biome, cave, water, tree, decoration, and structure stages.
- Move generation settings to serialized data files.
- Add chunk streaming policy for unloading, persistence, and save/load.

## Known Limitations

- Noise is intentionally simple and dependency-free.
- Terrain does not interpolate between sample points.
- Ore distribution is value-noise based, not vein based.
- There are no biomes, caves, water, trees, or decorations yet.
- The windowed client keeps generated chunks resident; there is no unload or
  persistence policy yet.
