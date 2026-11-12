#!/usr/bin/env python3
"""Compare an actual fullscreen capture to an owner PNG using explicit registration."""

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image


def digest(path):
    return hashlib.file_digest(path.open("rb"), "sha256").hexdigest()


def reference_canvas(source, size):
    # The mockup's actual client excludes its decorative desktop and title bar.
    # Fit that fixed-size client into the guest image for comparison only.
    # This transform has no bearing on application layout or acceptance.
    return source.crop((35, 73, 1565, 970)).resize(size, Image.Resampling.LANCZOS)


def compare(args):
    actual = Image.open(args.runtime).convert("RGB")
    source = Image.open(args.reference).convert("RGB")
    if source.size != (1600, 1000):
        raise ValueError("Expected an original 1600 × 1000 ArchRescue mockup")
    reference = reference_canvas(source, actual.size)

    expected_pixels = np.asarray(reference, dtype=np.int16)
    actual_pixels = np.asarray(actual, dtype=np.int16)
    difference = np.abs(expected_pixels - actual_pixels)
    side_by_side = Image.new("RGB", (actual.width * 2, actual.height))
    side_by_side.paste(reference)
    side_by_side.paste(actual, (actual.width, 0))
    products = {
        "reference": reference,
        "overlays": Image.blend(reference, actual, 0.5),
        "differences": Image.fromarray(difference.astype(np.uint8)),
        "heatmaps": Image.fromarray(np.minimum(difference * 4, 255).astype(np.uint8)),
        "comparisons": side_by_side,
    }
    for category, image in products.items():
        folder = args.output / category
        folder.mkdir(parents=True, exist_ok=True)
        image.save(folder / f"{args.name}.png", optimize=True)

    report = {
        "reference": str(args.reference),
        "reference_sha256": digest(args.reference),
        "runtime": str(args.runtime),
        "screenshot_sha256": digest(args.runtime),
        "source_commit": args.commit,
        "executable_sha256": digest(args.executable),
        "scale": args.scale,
        "dimensions": list(actual.size),
        "registration": "Client crop [35,73,1565,970] resized independently on each axis",
        "mean_absolute_rgb_error": float(difference.mean()),
        "root_mean_square_rgb_error": float(np.sqrt(np.mean(difference.astype(float) ** 2))),
        "p95_absolute_channel_error": float(np.percentile(difference, 95)),
        "identical_pixel_fraction": float(np.mean(np.all(difference == 0, axis=2))),
        "masked_pixels": 0,
    }
    geometry = args.output / "geometry"
    geometry.mkdir(parents=True, exist_ok=True)
    (geometry / f"{args.name}.json").write_text(json.dumps(report, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ["reference", "runtime", "executable", "output"]:
        parser.add_argument(argument, type=Path)
    parser.add_argument("--name", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--scale", type=float, default=1.0)
    compare(parser.parse_args())


if __name__ == "__main__":
    main()
