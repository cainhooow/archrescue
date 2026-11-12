#!/usr/bin/env python3
"""Index original mockup dimensions, palettes, and pixel runs at stable shell edges."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from PIL import Image


SAMPLES = {
    "rail_boundary": (282, 90),
    "header_boundary": (600, 127),
    "canvas": (400, 880),
    "footer_boundary": (600, 906),
    "rail": (41, 600),
    "footer": (600, 950),
}


def hex_rgb(color):
    return "#%02X%02X%02X" % color


def color_runs(image, fixed, start, end, vertical):
    sample = image.crop((fixed, start, fixed + 1, end) if vertical else (start, fixed, end, fixed + 1))
    colors = list(sample.get_flattened_data())
    spans = []
    first = 0
    for index, color in enumerate(colors, 1):
        if index == len(colors) or color != colors[index]:
            spans.append([first + start, index + start, hex_rgb(color)])
            first = index
    return spans


def inspect(file):
    with Image.open(file) as original:
        image = original.convert("RGB")
    if image.size != (1600, 1000):
        raise ValueError(f"{file} has an unexpected canvas")

    client = (35, 73, 1565, 970)
    pixels = image.crop(client)
    with file.open("rb") as stream:
        sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
    return {
        "image": str(file),
        "sha256": sha256,
        "canvas": list(image.size),
        "client": client,
        "palette": [[hex_rgb(color), count] for color, count in Counter(pixels.get_flattened_data()).most_common(20)],
        "samples": {name: {"position": position, "color": hex_rgb(image.getpixel(position))} for name, position in SAMPLES.items()},
        "horizontal_runs_at_y90": color_runs(image, 90, 35, 1565, False),
        "vertical_runs_at_x300": color_runs(image, 300, 73, 970, True),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("png_root", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    reports = [inspect(file) for file in sorted(args.png_root.glob("*/*.png"))]
    args.output.write_text(json.dumps(reports, indent=2) + "\n")


if __name__ == "__main__":
    main()
