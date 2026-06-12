#!/usr/bin/env python3

import argparse
import json
from pathlib import Path
from PIL import Image


FACE_SIZE = 16
RGBA_OPAQUE = 255
FACE_NAMES = ["top", "bottom", "front", "back", "left", "right"]


def normalize_pixel(pixel):
    if not isinstance(pixel, list):
        raise ValueError(f"Pixel must be a list. Got: {pixel}")

    if len(pixel) == 3:
        r, g, b = pixel
        a = RGBA_OPAQUE
    elif len(pixel) == 4:
        r, g, b, a = pixel
    else:
        raise ValueError(f"Pixel must be RGB or RGBA. Got: {pixel}")

    values = [r, g, b, a]

    if not all(isinstance(v, int) and 0 <= v <= 255 for v in values):
        raise ValueError(f"Pixel values must be integers from 0 to 255. Got: {pixel}")

    return values


def normalize_face(face):
    if not isinstance(face, list) or len(face) != FACE_SIZE:
        raise ValueError("A face must be a 16x16 array")

    return [[normalize_pixel(pixel) for pixel in row] for row in face]


def looks_like_face(value):
    return (
        isinstance(value, list)
        and len(value) == FACE_SIZE
        and all(isinstance(row, list) and len(row) == FACE_SIZE for row in value)
    )


def normalize_block(data):
    if looks_like_face(data):
        face = normalize_face(data)
        return [face, face, face, face, face, face]

    if not isinstance(data, list):
        raise ValueError("Input must be a face array or a block array")

    if len(data) == 3:
        top = normalize_face(data[0])
        bottom = normalize_face(data[1])
        sides = normalize_face(data[2])
        return [top, bottom, sides, sides, sides, sides]

    if len(data) == 6:
        return [normalize_face(face) for face in data]

    raise ValueError(
        "Input must be either 16x16, 3x16x16, or 6x16x16. " "Pixels can be RGB or RGBA."
    )


def face_to_image(face):
    image = Image.new("RGBA", (FACE_SIZE, FACE_SIZE))

    for y in range(FACE_SIZE):
        for x in range(FACE_SIZE):
            image.putpixel((x, y), tuple(face[y][x]))

    return image


def generate_face_images(block, output_dir):
    normalized_block = normalize_block(block)

    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    for face_name, face in zip(FACE_NAMES, normalized_block):
        image = face_to_image(face)
        image.save(output_dir / f"{face_name}.png", "PNG")


def load_input(input_value):
    stripped = input_value.strip()

    if stripped.startswith("[") or stripped.startswith("{"):
        return json.loads(stripped)

    path = Path(input_value)

    with path.open("r", encoding="utf-8") as file:
        return json.load(file)


def main():
    parser = argparse.ArgumentParser(
        description="Generate Minecraft block face PNGs from compact texture arrays."
    )

    parser.add_argument(
        "array",
        help="JSON array string or path to a JSON file.",
    )

    parser.add_argument(
        "-o",
        "--output",
        default="minecraft_block_texture",
        help="Output folder path.",
    )

    args = parser.parse_args()

    data = load_input(args.array)
    generate_face_images(data, args.output)

    print(f"Saved face textures to {args.output}")


if __name__ == "__main__":
    main()
