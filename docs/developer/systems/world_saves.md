# World Save System

## Purpose

World saves store player-facing world identity and durable state while keeping
world generation deterministic and reusable.

## Save Root

The windowed client stores worlds under:

```text
saves/worlds/<world-id>/
```

Each world contains:

- `world.txt`: versioned metadata with name, seed, player eye position, yaw,
  pitch, inventory slots, and timestamps.
- `chunks/<x>_<z>.hcc`: binary keyed-palette block data for edited or saved
  chunks.

## Runtime Policy

- The game opens to a main menu, then a world-management screen.
- Creating a world records a seed. A blank seed entry uses an automatic seed;
  a typed numeric seed can reproduce the same generated terrain in another
  world.
- Chunk streaming asks the save store for a chunk first. If no saved chunk file
  exists, the deterministic generation pipeline creates the chunk from the
  world's seed.
- Initially streamed chunks are queued for the next explicit save so newly
  generated chunks and legacy regenerated chunks are written in the current
  keyed format on `Save & Quit`.
- Block edits mark affected chunks dirty in memory. Gameplay does not write
  chunk files during the render/update loop.
- Player position, camera orientation, and inventory stay in memory while
  playing.
- `Save & Quit` and window close flush metadata plus dirty chunks to disk in one
  explicit save pass.

## Format Notes

- Metadata is a simple `key=value` text file with `version=1`.
- Inventory slots are saved in metadata as optional item key/count lines, so
  item stacks survive content registration order changes.
- Current chunk files start with an `HCCNK002` magic header, store chunk
  coordinates, write a block-key palette, then write `CHUNK_VOLUME`
  little-endian `u16` palette indices.
- Block keys are resolved through the current block registry when loading, so
  saved chunks survive block registration order changes.
- Legacy `HCCNK001` raw-ID chunks are ignored and regenerated from the world
  seed. Those files predate keyed palettes and can otherwise reinterpret old
  oak logs, leaves, sand, or other blocks as unrelated newer block IDs.
- Flushes build one contiguous byte buffer per chunk before writing to avoid
  many tiny writes.

## Known Limitations

- There is no chunk unloading yet, so loaded chunks remain resident for the
  session.
- Delete is immediate in the early UI.
- World configuration only supports name and seed. Game mode, health, hunger,
  and other player data are planned.
