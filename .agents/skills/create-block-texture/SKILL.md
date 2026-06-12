---
name: create-block-texture
description: Generate Minecraft-style 16x16 block face PNG textures from compact JSON RGB/RGBA arrays using generate_block_texture.py. Use when creating or updating textures under textures/blocks/{block_name}.
---

# Create Block Texture

Generate Minecraft-style block face PNGs from compact JSON pixel arrays.

Use the bundled script:

```bash
python3 .agents/skills/create-block-texture/scripts/generate_block_texture.py '<json-array>' -o textures/blocks/<block_name>
```

Save generated textures under:

```text
textures/blocks/{block_name}
```

The script creates exactly these files:

```text
top.png
bottom.png
front.png
back.png
left.png
right.png
```

## Workflow

1. Decide whether the block needs one, three, or six faces.
2. Create a pure JSON array with 16x16 RGB or RGBA pixels.
3. Run `scripts/generate_block_texture.py`.
4. Verify the six PNG files exist and are 16x16 RGBA images.

Example for a block where all faces are identical:

```bash
python3 .agents/skills/create-block-texture/scripts/generate_block_texture.py '<16x16-json-face>' -o textures/blocks/stone
```

## Texture Format

Each face is a 16×16 pixel array.

Pixels can be RGB:

```json
[105, 72, 40]
```

or RGBA:

```json
[105, 72, 40, 255]
```

If alpha is omitted, the script assumes full opacity.

## Accepted Input Shapes

- One face: `16 x 16 x 3` or `16 x 16 x 4`; expands to all six faces.
- Three faces: `[top, bottom, sides]`; use for grass-like blocks.
- Six faces: `[top, bottom, front, back, left, right]`.

## Important Rules

- Always generate 16×16 faces.
- Use JSON arrays only.
- Do not include Python variable assignments or comments.
- Prefer RGB unless transparency is needed.
- Use RGBA only for transparent blocks.
- Dirt, stone, sand, planks, ores, etc. generally need only one face.
- Grass and similar blocks generally need three faces.
- Directional or complex blocks may need six faces.

## Visual Style Guidelines

Use coherent 16×16 pixel art with small color variations instead of flat colors.
