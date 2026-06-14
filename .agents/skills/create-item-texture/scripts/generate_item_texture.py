#!/usr/bin/env python3

import argparse
import json
from pathlib import Path
from PIL import Image


TEXTURE_SIZE = 16
RGBA_OPAQUE = 255


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
    if not isinstance(face, list) or len(face) != TEXTURE_SIZE:
        raise ValueError("An item texture must be a 16x16 array")

    normalized = []
    for row in face:
        if not isinstance(row, list) or len(row) != TEXTURE_SIZE:
            raise ValueError("An item texture must be a 16x16 array")
        normalized.append([normalize_pixel(pixel) for pixel in row])
    return normalized


def face_to_image(face):
    image = Image.new("RGBA", (TEXTURE_SIZE, TEXTURE_SIZE))
    for y in range(TEXTURE_SIZE):
        for x in range(TEXTURE_SIZE):
            image.putpixel((x, y), tuple(face[y][x]))
    return image


def load_source(source):
    stripped = source.strip()
    if stripped.startswith("["):
        return face_to_image(normalize_face(json.loads(stripped)))

    path = Path(source)
    if path.suffix.lower() == ".json":
        with path.open("r", encoding="utf-8") as file:
            return face_to_image(normalize_face(json.load(file)))

    image = Image.open(path).convert("RGBA")
    if image.size != (TEXTURE_SIZE, TEXTURE_SIZE):
        image = image.resize((TEXTURE_SIZE, TEXTURE_SIZE), Image.Resampling.NEAREST)
    return image


def tilted_icon(image):
    canvas = Image.new("RGBA", (TEXTURE_SIZE, TEXTURE_SIZE), (0, 0, 0, 0))
    source = image.resize((12, 12), Image.Resampling.NEAREST)
    canvas.alpha_composite(source, (2, 2))
    return canvas


def main():
    parser = argparse.ArgumentParser(
        description="Generate a Minecraft-style 16x16 item icon PNG."
    )
    parser.add_argument("source", help="JSON array, JSON file path, or PNG file path.")
    parser.add_argument("-o", "--output", required=True, help="Output PNG path.")
    parser.add_argument(
        "--tilt",
        action="store_true",
        help="Inset block-face sources into a transparent icon silhouette.",
    )
    args = parser.parse_args()

    image = load_source(args.source)
    if args.tilt:
        image = tilted_icon(image)

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    image.save(output, "PNG")
    print(f"Saved item texture to {output}")


if __name__ == "__main__":
    main()
