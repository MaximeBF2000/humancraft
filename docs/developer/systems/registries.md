# Registry System

## Purpose

Registries assign compact stable IDs to definitions with unique string keys.

## Responsibilities

- Preserve registration order as stable numeric IDs.
- Reject duplicate keys.
- Provide lookup by ID or key.
- Keep content registration separate from engine systems.

## Inputs

- Definitions implementing `Definition`.

## Outputs

- Stable IDs such as `BlockId` and `ItemId`.
- Immutable definition lookup for systems.

## Dependencies

- Rust standard library collections.

## Extension Points

- Deserialize definitions from data files.
- Add registry namespaces or dependency ordering.
- Persist ID mappings in save metadata.

## Known Limitations

- Registries are immutable after bootstrap by convention only.
- There is no data-file loader yet.
- ID remapping for saves is not implemented yet.
