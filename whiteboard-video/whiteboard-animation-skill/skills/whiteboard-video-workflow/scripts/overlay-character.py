#!/usr/bin/env python3
"""Add a consistent small watercolor character silhouette to generated book images."""

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


DEFAULT_POSITIONS = [
    (0.70, 0.72, 0.205),
    (0.30, 0.72, 0.195),
    (0.74, 0.72, 0.195),
    (0.22, 0.72, 0.185),
    (0.70, 0.73, 0.190),
    (0.24, 0.74, 0.215),
    (0.70, 0.72, 0.190),
    (0.28, 0.72, 0.205),
]


def draw_character(base: Image.Image, cx: float, foot_y: float, scale: float) -> None:
    """Draw one consistent back-view woman silhouette on an RGBA image."""
    width, height = base.size
    layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    size = height * scale
    x = cx * width
    y = foot_y * height

    draw.ellipse([x - size * 0.42, y - size * 0.045, x + size * 0.42, y + size * 0.075], fill=(36, 39, 38, 40))
    leg_color = (42, 46, 53, 132)
    draw.rounded_rectangle([x - size * 0.13, y - size * 0.50, x - size * 0.035, y - size * 0.05], radius=max(1, int(size * 0.025)), fill=leg_color)
    draw.rounded_rectangle([x + size * 0.035, y - size * 0.50, x + size * 0.13, y - size * 0.05], radius=max(1, int(size * 0.025)), fill=leg_color)
    # Long dark skirt and linen apron read as a modern adult letter-shop owner,
    # not a school uniform or period costume.
    draw.polygon(
        [(x - size * 0.18, y - size * 0.58), (x + size * 0.18, y - size * 0.58), (x + size * 0.24, y - size * 0.08), (x - size * 0.24, y - size * 0.08)],
        fill=(34, 41, 48, 162),
    )
    draw.polygon(
        [(x - size * 0.12, y - size * 0.78), (x + size * 0.12, y - size * 0.78), (x + size * 0.17, y - size * 0.20), (x - size * 0.17, y - size * 0.20)],
        fill=(196, 184, 160, 128),
    )
    draw.rounded_rectangle([x - size * 0.20, y - size * 0.92, x + size * 0.20, y - size * 0.55], radius=max(1, int(size * 0.08)), fill=(226, 218, 201, 178))
    draw.line([x, y - size * 0.90, x, y - size * 0.54], fill=(132, 126, 118, 92), width=max(1, int(size * 0.018)))
    draw.ellipse([x - size * 0.145, y - size * 1.08, x + size * 0.145, y - size * 0.81], fill=(30, 38, 43, 210))
    draw.pieslice([x - size * 0.16, y - size * 1.02, x + size * 0.16, y - size * 0.72], 0, 180, fill=(30, 38, 43, 220))
    draw.line([x - size * 0.18, y - size * 0.82, x - size * 0.30, y - size * 0.60], fill=(217, 209, 192, 142), width=max(2, int(size * 0.035)))
    draw.line([x + size * 0.18, y - size * 0.82, x + size * 0.30, y - size * 0.62], fill=(217, 209, 192, 142), width=max(2, int(size * 0.035)))

    layer = layer.filter(ImageFilter.GaussianBlur(radius=0.45))
    base.alpha_composite(layer)


def overlay_images(input_dir: Path, output_dir: Path) -> list[Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    files = sorted(input_dir.glob("*.png"))
    written = []
    for index, image_path in enumerate(files):
        image = Image.open(image_path).convert("RGBA")
        cx, foot_y, scale = DEFAULT_POSITIONS[index % len(DEFAULT_POSITIONS)]
        draw_character(image, cx, foot_y, scale)
        output_path = output_dir / f"overlay_{index + 1:02d}.png"
        image.convert("RGB").save(output_path)
        written.append(output_path)
    return written


def main() -> None:
    parser = argparse.ArgumentParser(description="Overlay a consistent small character silhouette on book-video images.")
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("output_dir", type=Path)
    args = parser.parse_args()

    outputs = overlay_images(args.input_dir, args.output_dir)
    for output in outputs:
        print(output)


if __name__ == "__main__":
    main()
