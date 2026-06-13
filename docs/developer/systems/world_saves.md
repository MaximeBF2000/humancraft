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
  pitch, and timestamps.
- `chunks/<x>_<z>.hcc`: binary block ID data for edited or saved chunks.

## Runtime Policy

- The game opens to a main menu, then a world-management screen.
- Creating a world records a seed. A blank seed entry uses an automatic seed;
  a typed numeric seed can reproduce the same generated terrain in another
  world.
- Chunk streaming asks the save store for a chunk first. If no saved chunk file
  exists, the deterministic generation pipeline creates the chunk from the
  world's seed.
- Block edits mark affected chunks dirty in memory. Gameplay does not write
  chunk files during the render/update loop.
- Player position and camera orientation stay in memory while playing.
- `Save & Quit` and window close flush metadata plus dirty chunks to disk in one
  explicit save pass.

## Format Notes

- Metadata is a simple `key=value` text file with `version=1`.
- Chunk files start with an `HCCNK001` magic header, store chunk coordinates,
  then write `CHUNK_VOLUME` little-endian `u32` block IDs. Flushes build one
  contiguous byte buffer per chunk before writing to avoid many tiny writes.
- The current format stores block IDs directly. That is acceptable for this
  early content-fixed build, but a future modding-capable format should migrate
  to block keys or a per-save palette.

## Known Limitations

- There is no chunk unloading yet, so loaded chunks remain resident for the
  session.
- Delete is immediate in the early UI.
- World configuration only supports name and seed. Game mode, inventory,
  health, hunger, and other player data are planned.
