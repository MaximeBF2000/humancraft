---
name: create-item-texture
description: Generate Minecraft-style 16x16 item icon PNG textures from compact JSON RGB/RGBA arrays or from an existing block face texture. Use when creating or updating textures under textures/items for inventory icons, dropped loot, tools, resources, or block-item icons.
---

# Create Item Texture

Generate a single `16 x 16` RGBA item icon PNG for inventory and loot rendering.

Use the bundled script:

```bash
python3 .agents/skills/create-item-texture/scripts/generate_item_texture.py '<json-array-or-source-png>' -o textures/items/<item_name>.png
```

Save generated textures under:

```text
textures/items/{item_name}.png
```

## Workflow

1. Choose a source:
   - JSON `16 x 16 x 3` or `16 x 16 x 4` pixel array for custom item art.
   - Existing PNG block face for block-item icons.
2. Run `scripts/generate_item_texture.py`.
3. Verify the output PNG exists and is exactly `16 x 16` RGBA.

## Texture Format

Pixels can be RGB:

```json
[105, 72, 40]
```

or RGBA:

```json
[105, 72, 40, 255]
```

If alpha is omitted, the script assumes full opacity.

## Block Item Icons

For block items, prefer reusing the most representative block face:

```bash
python3 .agents/skills/create-item-texture/scripts/generate_item_texture.py textures/blocks/stone/top.png -o textures/items/stone.png
```

Use `--tilt` only when a flat face is too hard to distinguish from a UI slot.

## Visual Style Guidelines

- Use coherent pixel art with small color variations instead of flat colors.
- Keep important silhouettes inside the central `14 x 14` area.
- Use transparency for non-block resources such as coal, diamonds, saplings, and raw ore.
- Avoid anti-aliased edges; nearest-neighbor 16x16 pixel art should stay crisp.
