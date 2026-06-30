#!/usr/bin/env python3
"""Render controlled documentary-style illustrations for No Future Without Forgiveness."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont


BOOK_FOLDER = "001\u6ca1\u6709\u5bbd\u6055\u5c31\u6ca1\u6709\u672a\u6765"
W, H = 1920, 1080
SUBTITLE_SAFE_Y = 770
ANTIALIAS_SCALE = 2

PAPER = (244, 235, 215)
CREAM = (250, 245, 231)
INK = (38, 42, 43)
MUTED = (100, 112, 108)
OCHRE = (205, 154, 82)
RUST = (174, 82, 58)
TEAL = (75, 124, 128)
BLUE = (111, 153, 166)
OLIVE = (96, 128, 91)
CLAY = (187, 130, 83)
GOLD = (229, 181, 86)
SKIN_DARK = (112, 74, 48)
SKIN_MED = (177, 122, 83)
SKIN_LIGHT = (219, 174, 132)
WHITE_CLOTH = (239, 235, 221)


def find_book_dir(book_dir: Path | None = None) -> Path:
    if book_dir is not None:
        if not book_dir.exists():
            raise FileNotFoundError(book_dir)
        return book_dir
    matches = [path for path in Path("D:/books").rglob(BOOK_FOLDER) if path.is_dir()]
    if not matches:
        raise FileNotFoundError(BOOK_FOLDER)
    return matches[0]


def font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/arial.ttf",
    ]
    for candidate in candidates:
        path = Path(candidate)
        if path.exists():
            return ImageFont.truetype(str(path), size)
    return ImageFont.load_default()


def canvas() -> tuple[Image.Image, ImageDraw.ImageDraw]:
    image = Image.new("RGB", (W, H), PAPER)
    draw = ImageDraw.Draw(image)
    for y in range(H):
        shade = int(7 * (y / H))
        draw.line((0, y, W, y), fill=(PAPER[0] - shade, PAPER[1] - shade, PAPER[2] - shade))
    for x in range(-60, W, 90):
        draw.line((x, 0, x + 360, H), fill=(238, 229, 207), width=1)
    return image, draw


def subtitle_band(draw: ImageDraw.ImageDraw) -> None:
    for y in range(SUBTITLE_SAFE_Y, H):
        t = (y - SUBTITLE_SAFE_Y) / max(1, H - SUBTITLE_SAFE_Y)
        color = (
            int(247 * (1 - t) + 241 * t),
            int(240 * (1 - t) + 230 * t),
            int(224 * (1 - t) + 208 * t),
        )
        draw.line((0, y, W, y), fill=color)
    draw.line((0, SUBTITLE_SAFE_Y, W, SUBTITLE_SAFE_Y), fill=(221, 206, 176), width=3)
    for x in range(60, W, 180):
        draw.arc((x, SUBTITLE_SAFE_Y + 24, x + 95, SUBTITLE_SAFE_Y + 66), 195, 345, fill=(229, 215, 188), width=2)


def sun_or_moon(draw: ImageDraw.ImageDraw, x: int, y: int, r: int, color=GOLD) -> None:
    for i in range(4, 0, -1):
        shade = tuple(min(255, c + 20 * (4 - i)) for c in color)
        draw.ellipse((x - r * i, y - r * i, x + r * i, y + r * i), fill=shade)


def grain(image: Image.Image) -> Image.Image:
    noise = Image.effect_noise((W, H), 9).convert("L")
    tint = Image.new("RGB", (W, H), (128, 128, 128))
    tint.putalpha(noise.point(lambda value: int(abs(value - 128) * 0.18)))
    out = image.convert("RGBA")
    out.alpha_composite(tint)
    return out.convert("RGB")


def antialias_finish(image: Image.Image) -> Image.Image:
    """Smooth hard vector edges while keeping the hand-drawn look readable."""
    large = image.resize((W * ANTIALIAS_SCALE, H * ANTIALIAS_SCALE), Image.Resampling.LANCZOS)
    large = large.filter(ImageFilter.GaussianBlur(radius=0.25))
    smoothed = large.resize((W, H), Image.Resampling.LANCZOS)
    return smoothed.filter(ImageFilter.UnsharpMask(radius=0.8, percent=70, threshold=4))


def soft_rect(draw: ImageDraw.ImageDraw, xy, fill, outline=None, width=3, radius=18):
    draw.rounded_rectangle(xy, radius=radius, fill=fill, outline=outline, width=width)


def line(draw: ImageDraw.ImageDraw, points, fill=INK, width=5):
    draw.line(points, fill=fill, width=width, joint="curve")


def shadow(draw: ImageDraw.ImageDraw, xy, alpha_fill=(68, 54, 42)):
    x1, y1, x2, y2 = xy
    draw.ellipse((x1, y1, x2, y2), fill=tuple(max(0, c - 10) for c in alpha_fill))


def limb(draw: ImageDraw.ImageDraw, start, end, width: int, fill, outline=INK):
    x1, y1 = start
    x2, y2 = end
    draw.line((x1, y1, x2, y2), fill=outline, width=width + 4)
    draw.line((x1, y1, x2, y2), fill=fill, width=width)
    r = max(2, width // 2)
    draw.ellipse((x1 - r, y1 - r, x1 + r, y1 + r), fill=fill, outline=None)
    draw.ellipse((x2 - r, y2 - r, x2 + r, y2 + r), fill=fill, outline=None)


def shoe(draw: ImageDraw.ImageDraw, x: int, y: int, scale: float, direction: int = 1):
    w = int(34 * scale)
    h = int(14 * scale)
    if direction < 0:
        box = (x - w, y - h, x + int(6 * scale), y + h)
    else:
        box = (x - int(6 * scale), y - h, x + w, y + h)
    draw.rounded_rectangle(box, radius=max(2, int(6 * scale)), fill=(48, 45, 40), outline=INK, width=max(1, int(2 * scale)))


def person(draw: ImageDraw.ImageDraw, x: int, y: int, scale: float = 1.0, skin=SKIN_MED, clothes=TEAL, pose="stand", hair=INK):
    head_r = int(36 * scale)
    body_w = int(92 * scale)
    body_h = int(138 * scale)
    line_w = max(2, int(4 * scale))
    darker = tuple(max(0, c - 34) for c in clothes)
    lighter = tuple(min(255, c + 25) for c in clothes)
    shadow(draw, (x - int(52 * scale), y + int(72 * scale), x + int(54 * scale), y + int(100 * scale)), (202, 190, 170))
    neck_w = int(24 * scale)
    draw.rounded_rectangle((x - neck_w // 2, y - body_h - int(8 * scale), x + neck_w // 2, y - body_h + int(24 * scale)), radius=int(8 * scale), fill=skin, outline=INK, width=max(1, int(2 * scale)))
    draw.ellipse((x - head_r, y - body_h - head_r * 2, x + head_r, y - body_h), fill=skin, outline=INK, width=max(2, int(3 * scale)))
    draw.pieslice((x - head_r - int(4 * scale), y - body_h - head_r * 2 - int(12 * scale), x + head_r + int(4 * scale), y - body_h + int(20 * scale)), 178, 358, fill=hair, outline=INK)
    draw.arc((x - head_r, y - body_h - head_r * 2 - 8, x + head_r, y - body_h + 12), 190, 350, fill=INK, width=max(2, int(3 * scale)))
    eye_y = y - body_h - int(head_r * 1.15)
    draw.ellipse((x - int(13 * scale), eye_y, x - int(8 * scale), eye_y + int(5 * scale)), fill=INK)
    draw.ellipse((x + int(8 * scale), eye_y, x + int(13 * scale), eye_y + int(5 * scale)), fill=INK)
    draw.arc((x - int(13 * scale), eye_y + int(10 * scale), x + int(13 * scale), eye_y + int(24 * scale)), 20, 160, fill=INK, width=max(1, int(2 * scale)))
    soft_rect(draw, (x - body_w // 2, y - body_h, x + body_w // 2, y), clothes, INK, max(2, int(3 * scale)), int(24 * scale))
    draw.polygon([(x - body_w // 2 + int(6 * scale), y - body_h + int(10 * scale)), (x + body_w // 2 - int(6 * scale), y - body_h + int(10 * scale)), (x + body_w // 2 - int(18 * scale), y - body_h + int(58 * scale)), (x - body_w // 2 + int(18 * scale), y - body_h + int(58 * scale))], fill=lighter, outline=None)
    line(draw, [(x - body_w // 3, y - body_h + int(30 * scale)), (x + body_w // 3, y - body_h + int(30 * scale))], darker, max(2, int(3 * scale)))
    draw.line((x - body_w // 2 + 14, y - body_h + 58 * scale, x + body_w // 2 - 14, y - body_h + 58 * scale), fill=darker, width=max(2, int(2 * scale)))
    draw.line((x, y - body_h + int(34 * scale), x, y - int(10 * scale)), fill=darker, width=max(1, int(2 * scale)))
    if pose == "sit":
        draw.rounded_rectangle((x - int(68 * scale), y - int(6 * scale), x - int(6 * scale), y + int(34 * scale)), radius=int(17 * scale), fill=darker, outline=INK, width=line_w)
        draw.rounded_rectangle((x + int(6 * scale), y - int(6 * scale), x + int(68 * scale), y + int(34 * scale)), radius=int(17 * scale), fill=darker, outline=INK, width=line_w)
        limb(draw, (x - int(45 * scale), y + int(28 * scale)), (x - int(84 * scale), y + int(62 * scale)), max(5, int(13 * scale)), darker)
        limb(draw, (x + int(45 * scale), y + int(28 * scale)), (x + int(84 * scale), y + int(62 * scale)), max(5, int(13 * scale)), darker)
        shoe(draw, x - int(85 * scale), y + int(64 * scale), scale, -1)
        shoe(draw, x + int(85 * scale), y + int(64 * scale), scale, 1)
    else:
        draw.rounded_rectangle((x - int(39 * scale), y - int(3 * scale), x - int(6 * scale), y + int(82 * scale)), radius=int(15 * scale), fill=darker, outline=INK, width=line_w)
        draw.rounded_rectangle((x + int(6 * scale), y - int(3 * scale), x + int(39 * scale), y + int(82 * scale)), radius=int(15 * scale), fill=darker, outline=INK, width=line_w)
        limb(draw, (x - int(22 * scale), y + int(76 * scale)), (x - int(34 * scale), y + int(115 * scale)), max(5, int(12 * scale)), darker)
        limb(draw, (x + int(22 * scale), y + int(76 * scale)), (x + int(34 * scale), y + int(115 * scale)), max(5, int(12 * scale)), darker)
        shoe(draw, x - int(35 * scale), y + int(116 * scale), scale, -1)
        shoe(draw, x + int(35 * scale), y + int(116 * scale), scale, 1)
    arm_y = y - body_h + int(36 * scale)
    for side in (-1, 1):
        shoulder = (x + side * body_w // 2, arm_y)
        elbow = (x + side * int(66 * scale), y - int(68 * scale))
        hand = (x + side * int(82 * scale), y - int(42 * scale))
        limb(draw, shoulder, elbow, max(4, int(12 * scale)), lighter)
        limb(draw, elbow, hand, max(4, int(11 * scale)), lighter)
        draw.ellipse((hand[0] - int(9 * scale), hand[1] - int(9 * scale), hand[0] + int(9 * scale), hand[1] + int(9 * scale)), fill=skin, outline=INK, width=max(1, int(2 * scale)))


def table(draw, x, y, w, h, fill=(168, 122, 76)):
    shadow(draw, (x + 10, y + h + 28, x + w - 10, y + h + 92), (198, 182, 158))
    soft_rect(draw, (x, y, x + w, y + h), fill, INK, 4, 14)
    draw.line((x + 18, y + 24, x + w - 18, y + 24), fill=tuple(max(0, c - 28) for c in fill), width=3)
    for lx in (x + 40, x + w - 40):
        line(draw, [(lx, y + h), (lx - 20, y + h + 120)], INK, 6)


def window(draw, x, y, w, h, sky=BLUE):
    shadow(draw, (x + 18, y + 18, x + w + 36, y + h + 36), (210, 197, 172))
    soft_rect(draw, (x, y, x + w, y + h), (225, 232, 222), INK, 5, 10)
    draw.rectangle((x + 12, y + 12, x + w - 12, y + h - 12), fill=sky)
    draw.polygon((x + 18, y + h - 20, x + w - 20, y + 30, x + w - 20, y + 100, x + 130, y + h - 20), fill=(198, 219, 218))
    line(draw, [(x + w // 2, y + 12), (x + w // 2, y + h - 12)], INK, 3)
    line(draw, [(x + 12, y + h // 2), (x + w - 12, y + h // 2)], INK, 3)


def hills(draw, y=410):
    draw.polygon([(0, y + 70), (260, y - 40), (520, y + 70)], fill=(176, 133, 91))
    draw.polygon([(360, y + 80), (760, y - 90), (1120, y + 70)], fill=(151, 111, 84))
    draw.polygon([(900, y + 60), (1280, y - 20), (1660, y + 60)], fill=(137, 128, 100))
    for x in range(0, W, 180):
        draw.ellipse((x, y + 60, x + 90, y + 130), fill=(77, 119, 89), outline=None)
    for x in range(35, W, 115):
        line(draw, [(x, y + 142), (x + 45, y + 126)], (118, 137, 103), 3)


def distant_town(draw, y: int = 510, start: int = 1180) -> None:
    for i, x in enumerate(range(start, start + 430, 92)):
        soft_rect(draw, (x, y - (i % 2) * 18, x + 72, y + 58), (203, 181, 139), INK, 3, 8)
        draw.polygon((x - 8, y - (i % 2) * 18, x + 36, y - 46 - (i % 2) * 18, x + 80, y - (i % 2) * 18), fill=RUST, outline=INK)
        draw.rectangle((x + 24, y + 18, x + 48, y + 58), fill=(119, 91, 71), outline=INK, width=2)


def road(draw, start=(960, 760), end=(960, 1080), color=(204, 179, 132)):
    sx, sy = start
    ex, ey = end
    draw.polygon([(sx - 90, sy), (sx + 90, sy), (ex + 360, ey), (ex - 360, ey)], fill=color)
    line(draw, [(sx, sy + 20), (ex, ey - 20)], (232, 218, 178), 7)
    for off in (-95, 95):
        line(draw, [(sx + off // 3, sy + 18), (ex + off * 3, ey)], (170, 142, 104), 3)


def paper_stack(draw, x, y, w=150, h=95, sheets=3):
    for i in range(sheets):
        soft_rect(draw, (x + i * 10, y - i * 8, x + w + i * 10, y + h - i * 8), WHITE_CLOTH, INK, 3, 5)
        draw.line((x + 18 + i * 10, y + 25 - i * 8, x + w - 18 + i * 10, y + 25 - i * 8), fill=(183, 176, 158), width=2)
        draw.line((x + 18 + i * 10, y + 50 - i * 8, x + w - 30 + i * 10, y + 50 - i * 8), fill=(183, 176, 158), width=2)


def framed_photo(draw, x, y, w=110, h=85, tint=(198, 176, 132)):
    soft_rect(draw, (x, y, x + w, y + h), (96, 74, 54), INK, 3, 6)
    draw.rectangle((x + 10, y + 10, x + w - 10, y + h - 10), fill=tint, outline=(126, 107, 82), width=2)
    draw.ellipse((x + int(w * 0.42), y + int(h * 0.24), x + int(w * 0.58), y + int(h * 0.43)), fill=(92, 80, 68))
    draw.polygon([(x + 20, y + h - 12), (x + w // 2, y + h // 2), (x + w - 18, y + h - 12)], fill=(145, 128, 95))


def archive_box(draw, x, y, w=150, h=90, fill=(194, 169, 119)):
    soft_rect(draw, (x, y, x + w, y + h), fill, INK, 4, 8)
    draw.rectangle((x + 20, y + 18, x + w - 20, y + 46), fill=(231, 222, 195), outline=INK, width=2)
    draw.line((x + 28, y + 32, x + w - 28, y + 32), fill=(165, 151, 126), width=2)


def chair(draw, x, y, scale=1.0, fill=(191, 174, 138)):
    w = int(58 * scale)
    h = int(72 * scale)
    soft_rect(draw, (x, y, x + w, y + h), fill, INK, max(2, int(3 * scale)), int(8 * scale))
    draw.rectangle((x + int(10 * scale), y + h, x + w - int(10 * scale), y + h + int(18 * scale)), fill=tuple(max(0, c - 18) for c in fill), outline=INK, width=max(1, int(2 * scale)))


def audience_row(draw, x, y, count=6, scale=0.42):
    for i in range(count):
        px = x + i * int(76 * scale / 0.42)
        chair(draw, px - int(24 * scale), y - int(78 * scale), scale * 0.9, fill=(196, 181, 148))
        person(draw, px, y, scale, skin=[SKIN_DARK, SKIN_MED, SKIN_LIGHT][i % 3], clothes=[TEAL, OCHRE, RUST][i % 3], pose="sit")


def tea_cup(draw, x, y, scale=1.0):
    w = int(82 * scale)
    h = int(42 * scale)
    draw.ellipse((x, y + h - int(10 * scale), x + w, y + h + int(18 * scale)), fill=(196, 178, 145), outline=INK, width=max(2, int(3 * scale)))
    soft_rect(draw, (x + int(8 * scale), y, x + w - int(8 * scale), y + h), (233, 218, 185), INK, max(2, int(3 * scale)), int(14 * scale))
    draw.arc((x + w - int(10 * scale), y + int(8 * scale), x + w + int(28 * scale), y + h), -80, 90, fill=INK, width=max(2, int(3 * scale)))


def plant(draw, x, y, scale=1.0):
    soft_rect(draw, (x - int(26 * scale), y, x + int(26 * scale), y + int(46 * scale)), (158, 109, 72), INK, 3, 8)
    for angle in (-65, -35, 0, 35, 65):
        length = int(85 * scale)
        rad = math.radians(angle - 90)
        x2 = x + int(math.cos(rad) * length)
        y2 = y + int(math.sin(rad) * length)
        line(draw, [(x, y), (x2, y2)], (64, 112, 72), max(2, int(4 * scale)))
        draw.ellipse((x2 - int(16 * scale), y2 - int(10 * scale), x2 + int(16 * scale), y2 + int(10 * scale)), fill=(82, 138, 88), outline=INK, width=2)


def patterned_rug(draw, x, y, w, h):
    draw.ellipse((x, y, x + w, y + h), fill=(194, 173, 139), outline=(167, 145, 112), width=3)
    for i in range(5):
        draw.arc((x + 40 + i * 36, y + 12, x + w - 40 - i * 36, y + h - 12), 0, 180, fill=(174, 151, 115), width=2)


def floor_boards(draw, y=640, color=(221, 207, 180)):
    draw.rectangle((0, y, W, SUBTITLE_SAFE_Y), fill=color)
    for offset in range(-260, W, 180):
        line(draw, [(offset, y + 8), (offset + 360, SUBTITLE_SAFE_Y - 18)], (201, 184, 154), 2)
    draw.line((0, y, W, y), fill=(194, 176, 148), width=3)


def wall_shelf(draw, x, y, w, books=6):
    draw.rounded_rectangle((x, y, x + w, y + 16), radius=5, fill=(131, 92, 59), outline=INK, width=3)
    colors = [TEAL, RUST, OCHRE, BLUE, OLIVE, CLAY]
    bx = x + 18
    for i in range(books):
        bw = 22 + (i % 3) * 8
        bh = 62 + (i % 4) * 9
        draw.rounded_rectangle((bx, y - bh, bx + bw, y), radius=3, fill=colors[i % len(colors)], outline=INK, width=2)
        draw.line((bx + 5, y - bh + 9, bx + bw - 5, y - bh + 9), fill=(238, 226, 196), width=2)
        bx += bw + 9
    tea_cup(draw, x + w - 92, y - 48, 0.5)


def curtain(draw, x, y, w, h, side="left", fill=(196, 145, 98)):
    if side == "left":
        pts = [(x, y), (x + w, y + 20), (x + int(w * 0.7), y + h), (x + 10, y + h - 12)]
    else:
        pts = [(x + w, y), (x, y + 20), (x + int(w * 0.3), y + h), (x + w - 10, y + h - 12)]
    draw.polygon(pts, fill=fill, outline=INK)
    for i in range(1, 4):
        px = x + i * w // 4
        draw.line((px, y + 18, px - (8 if side == "left" else -8), y + h - 18), fill=tuple(max(0, c - 28) for c in fill), width=3)


def loose_notes(draw, x, y, count=5):
    for i in range(count):
        px = x + (i % 3) * 70
        py = y + (i // 3) * 52
        soft_rect(draw, (px, py, px + 54, py + 38), WHITE_CLOTH, (174, 159, 132), 2, 4)
        draw.line((px + 9, py + 13, px + 45, py + 13), fill=(170, 160, 140), width=2)
        draw.line((px + 9, py + 24, px + 35, py + 24), fill=(170, 160, 140), width=2)


def candle_group(draw, x, y, count=4, scale=1.0):
    for i in range(count):
        px = x + int(i * 54 * scale)
        h = int((58 + (i % 3) * 18) * scale)
        draw.rounded_rectangle((px, y - h, px + int(24 * scale), y), radius=int(7 * scale), fill=(238, 225, 190), outline=INK, width=max(2, int(3 * scale)))
        draw.polygon([(px + int(12 * scale), y - h - int(28 * scale)), (px + int(3 * scale), y - h - int(6 * scale)), (px + int(21 * scale), y - h - int(6 * scale))], fill=(246, 194, 76), outline=INK)
        draw.ellipse((px - int(8 * scale), y - int(3 * scale), px + int(32 * scale), y + int(9 * scale)), fill=(201, 184, 150), outline=INK, width=2)


def shrubs(draw, y, count=12):
    for i in range(count):
        x = 30 + i * (W - 80) // max(1, count - 1)
        draw.ellipse((x - 32, y - 28 - (i % 2) * 9, x + 36, y + 18), fill=(69, 117, 83), outline=None)
        line(draw, [(x - 20, y + 12), (x - 54, y + 25)], (106, 124, 89), 2)


def picture_wall(draw, x, y, count=3):
    for i in range(count):
        framed_photo(draw, x + i * 118, y + (i % 2) * 18, 86, 66, tint=[(194, 174, 132), (174, 190, 185), (207, 184, 151)][i % 3])


def lamp_glow(draw, cx, cy, r):
    for i in range(5, 0, -1):
        color = (245, 218 - i * 7, 126 - i * 3)
        draw.ellipse((cx - r * i / 5, cy - r * i / 5, cx + r * i / 5, cy + r * i / 5), fill=tuple(int(c) for c in color))


def label(draw, text: str, x: int, y: int):
    # Labels are metadata for review contact sheets only; scene images avoid text.
    draw.text((x, y), text, fill=INK, font=font(28))


def scene_1(draw):
    floor_boards(draw, 642, (221, 207, 181))
    sun_or_moon(draw, 570, 385, 38, (237, 200, 91))
    window(draw, 1240, 110, 430, 300, sky=(45, 74, 96))
    curtain(draw, 1214, 105, 72, 326, "left", fill=(159, 117, 88))
    curtain(draw, 1628, 105, 72, 326, "right", fill=(159, 117, 88))
    wall_shelf(draw, 195, 315, 310, 7)
    picture_wall(draw, 690, 250, 3)
    table(draw, 320, 585, 920, 74)
    soft_rect(draw, (520, 438, 620, 610), OCHRE, INK, 4, 28)
    lamp_glow(draw, 570, 385, 130)
    draw.polygon([(540, 438), (600, 438), (570, 350)], fill=(248, 214, 121), outline=INK)
    soft_rect(draw, (790, 508, 1040, 578), (219, 208, 184), INK, 4, 12)
    draw.line((815, 540, 1015, 540), fill=MUTED, width=3)
    paper_stack(draw, 850, 430, 170, 92, 2)
    loose_notes(draw, 1025, 420, 5)
    tea_cup(draw, 1118, 518, 0.9)
    framed_photo(draw, 680, 470, 96, 72)
    patterned_rug(draw, 390, 680, 760, 70)
    person(draw, 480, 610, 0.98, skin=SKIN_MED, clothes=TEAL, pose="sit", hair=(50, 48, 42))
    plant(draw, 1180, 588, 0.65)
    draw.polygon([(1265, 210), (1330, 155), (1400, 220), (1375, 275), (1295, 275)], fill=RUST, outline=INK)
    draw.rectangle((1308, 225, 1362, 275), fill=(226, 218, 190), outline=INK, width=3)


def scene_2(draw):
    hills(draw, 330)
    distant_town(draw, 520, 1240)
    shrubs(draw, 548, 15)
    draw.rectangle((0, 575, W, SUBTITLE_SAFE_Y), fill=(214, 196, 158))
    for x in range(80, 1600, 165):
        draw.rectangle((x, 600, x + 105, 640), fill=(197, 178, 139), outline=(162, 141, 107), width=2)
    for x in (360, 560, 760, 960, 1160):
        line(draw, [(x, 360), (x, SUBTITLE_SAFE_Y)], INK, 8)
    line(draw, [(250, 470), (1350, 470)], INK, 8)
    line(draw, [(250, 535), (1350, 535)], (72, 79, 78), 4)
    soft_rect(draw, (1180, 455, 1425, 555), (78, 92, 101), INK, 4, 18)
    draw.ellipse((1210, 540, 1265, 595), fill=INK)
    draw.ellipse((1350, 540, 1405, 595), fill=INK)
    draw.rectangle((1215, 475, 1265, 515), fill=(145, 170, 190), outline=INK, width=2)
    draw.rectangle((1300, 475, 1350, 515), fill=(145, 170, 190), outline=INK, width=2)
    archive_box(draw, 1015, 610, 145, 78)
    draw.rectangle((140, 610, 250, 690), fill=(180, 157, 118), outline=INK, width=4)
    draw.polygon([(128, 610), (195, 560), (264, 610)], fill=RUST, outline=INK)
    for i, x in enumerate((340, 460, 590, 720, 850)):
        person(draw, x, 670, 0.67, skin=[SKIN_DARK, SKIN_MED, SKIN_LIGHT][i % 3], clothes=[TEAL, OCHRE, RUST][i % 3], hair=[INK, (82, 59, 42), (66, 54, 43)][i % 3])
        soft_rect(draw, (x + 35, 525, x + 95, 570), WHITE_CLOTH, INK, 3, 5)
    paper_stack(draw, 1165, 620, 120, 70, 2)
    loose_notes(draw, 1450, 585, 4)


def scene_3(draw):
    hills(draw, 300)
    sun_or_moon(draw, 1500, 185, 30, (240, 199, 92))
    road(draw, (960, 515), (960, 1080))
    shrubs(draw, 560, 14)
    soft_rect(draw, (740, 420, 1180, 535), (218, 205, 178), INK, 5, 12)
    soft_rect(draw, (900, 450, 1010, 520), WHITE_CLOTH, INK, 4, 7)
    soft_rect(draw, (1035, 455, 1115, 520), (190, 175, 135), INK, 3, 6)
    archive_box(draw, 620, 465, 118, 65)
    tea_cup(draw, 1135, 470, 0.62)
    for i, x in enumerate(range(280, 1530, 155)):
        person(draw, x, 660 + (i % 2) * 16, 0.62, skin=[SKIN_DARK, SKIN_MED, SKIN_LIGHT][i % 3], clothes=[TEAL, OCHRE, RUST][i % 3])
        if i % 2 == 0:
            soft_rect(draw, (x + 26, 535, x + 72, 568), WHITE_CLOTH, INK, 2, 4)
    draw.rectangle((1290, 185, 1380, 410), fill=(205, 190, 158), outline=INK, width=5)
    draw.polygon([(1260, 185), (1420, 185), (1340, 105)], fill=RUST, outline=INK)
    plant(draw, 1500, 610, 0.58)


def scene_4(draw):
    draw.rectangle((140, 120, 1780, SUBTITLE_SAFE_Y), fill=(226, 218, 202), outline=INK, width=6)
    draw.rectangle((170, 525, 1750, 665), fill=(214, 202, 178), outline=None)
    draw.rectangle((180, 150, 420, 315), fill=(215, 203, 178), outline=INK, width=4)
    draw.arc((238, 202, 362, 296), 200, 340, fill=(166, 143, 110), width=5)
    picture_wall(draw, 1450, 185, 2)
    for y in (250, 375, 500):
        draw.line((220, y, 1700, y), fill=(205, 193, 169), width=3)
    for x in (460, 790, 1120, 1450):
        line(draw, [(x, 145), (x, 330)], (190, 178, 154), 4)
    audience_row(draw, 440, 650, 4, 0.28)
    audience_row(draw, 1210, 650, 4, 0.28)
    table(draw, 530, 485, 860, 78, fill=(156, 110, 72))
    for x in (700, 920, 1140):
        draw.ellipse((x, 435, x + 28, 463), fill=INK)
        line(draw, [(x + 14, 463), (x + 14, 502)], INK, 5)
    soft_rect(draw, (1260, 400, 1380, 485), (190, 176, 136), INK, 4, 10)
    paper_stack(draw, 1280, 375, 100, 70, 3)
    paper_stack(draw, 585, 400, 120, 80, 2)
    archive_box(draw, 1395, 500, 130, 76)
    archive_box(draw, 1528, 520, 112, 64, fill=(176, 154, 116))
    framed_photo(draw, 410, 500, 90, 70)
    patterned_rug(draw, 545, 590, 840, 70)
    person(draw, 820, 480, 0.78, skin=SKIN_DARK, clothes=TEAL, pose="sit")
    person(draw, 1070, 480, 0.72, skin=SKIN_LIGHT, clothes=OCHRE, pose="sit")
    soft_rect(draw, (265, 540, 370, 668), (202, 186, 154), INK, 4, 10)
    loose_notes(draw, 430, 390, 4)


def scene_5(draw):
    floor_boards(draw, 642, (222, 208, 183))
    window(draw, 1240, 135, 400, 310, sky=(151, 173, 177))
    curtain(draw, 1212, 132, 70, 322, "left", fill=(142, 124, 112))
    curtain(draw, 1582, 132, 70, 322, "right", fill=(142, 124, 112))
    draw.rectangle((180, 190, 520, 475), fill=(225, 215, 194), outline=INK, width=4)
    for y in (250, 320, 390):
        draw.line((215, y, 485, y), fill=(195, 184, 161), width=3)
    for x in range(1280, 1600, 55):
        line(draw, [(x, 145), (x - 40, 430)], (123, 145, 150), 2)
    table(draw, 430, 590, 980, 75)
    patterned_rug(draw, 535, 690, 740, 64)
    person(draw, 760, 585, 1.02, skin=SKIN_DARK, clothes=(122, 130, 128), pose="sit")
    paper_stack(draw, 930, 500, 165, 100, 3)
    loose_notes(draw, 1085, 468, 4)
    tea_cup(draw, 1085, 620, 0.92)
    framed_photo(draw, 1180, 520, 130, 94)
    archive_box(draw, 1325, 530, 150, 82)
    draw.rectangle((980, 520, 1070, 585), fill=(185, 161, 120), outline=INK, width=4)
    plant(draw, 325, 590, 0.55)


def scene_6(draw):
    hills(draw, 290)
    distant_town(draw, 535, 1245)
    shrubs(draw, 560, 13)
    draw.rectangle((0, 555, W, SUBTITLE_SAFE_Y), fill=(208, 195, 160))
    draw.polygon([(900, 560), (1030, 560), (1560, 1080), (1200, 1080)], fill=(185, 154, 107))
    draw.polygon([(850, 560), (970, 560), (600, 1080), (240, 1080)], fill=(132, 116, 96))
    for x in range(250, 620, 75):
        line(draw, [(x, 675), (x + 52, 720)], (104, 102, 88), 3)
        draw.ellipse((x - 6, 664, x + 12, 682), fill=(124, 112, 92), outline=None)
    for x in (1260, 1375, 1490):
        soft_rect(draw, (x, 515, x + 120, 610), (205, 184, 139), INK, 4, 12)
        draw.polygon([(x - 10, 515), (x + 60, 465), (x + 130, 515)], fill=RUST, outline=INK)
        draw.rectangle((x + 42, 560, x + 75, 610), fill=(122, 94, 66), outline=INK, width=2)
    plant(draw, 1515, 640, 0.65)
    plant(draw, 1320, 680, 0.5)
    person(draw, 725, 705, 0.78, skin=SKIN_DARK, clothes=TEAL)
    person(draw, 1080, 700, 0.78, skin=SKIN_LIGHT, clothes=OCHRE)
    person(draw, 930, 665, 0.6, skin=SKIN_MED, clothes=RUST, pose="sit")
    chair(draw, 895, 626, 0.78, fill=(188, 171, 135))


def scene_7(draw):
    draw.rectangle((0, 0, W, H), fill=(224, 217, 199))
    hills(draw, 250)
    shrubs(draw, 560, 10)
    draw.rectangle((260, 350, 870, 620), fill=(190, 178, 157), outline=INK, width=6)
    draw.polygon([(230, 350), (565, 210), (900, 350)], fill=(137, 110, 92), outline=INK)
    draw.rectangle((500, 435, 630, 620), fill=(82, 83, 80), outline=INK, width=5)
    draw.arc((320, 380, 440, 500), 180, 360, fill=(125, 105, 86), width=5)
    draw.arc((700, 380, 820, 500), 180, 360, fill=(125, 105, 86), width=5)
    candle_group(draw, 1020, 705, 6, 0.9)
    draw.polygon([(970, 545), (1420, 540), (1450, 625), (940, 635)], fill=WHITE_CLOTH, outline=INK)
    framed_photo(draw, 980, 430, 110, 82, tint=(184, 171, 145))
    loose_notes(draw, 1140, 438, 3)
    for x in (1480, 1600):
        person(draw, x, 700, 0.68, skin=SKIN_DARK, clothes=(92, 92, 88))


def scene_8(draw):
    floor_boards(draw, 642, (222, 209, 184))
    sun_or_moon(draw, 1460, 270, 34, (245, 205, 112))
    window(draw, 1190, 110, 430, 310, sky=(154, 194, 205))
    curtain(draw, 1165, 108, 70, 320, "left", fill=(189, 142, 94))
    curtain(draw, 1586, 108, 70, 320, "right", fill=(189, 142, 94))
    road(draw, (1395, 395), (1500, 1080), color=(210, 187, 136))
    draw.rectangle((180, 150, 455, 420), fill=(224, 214, 190), outline=INK, width=4)
    for y in (220, 292, 364):
        draw.line((210, y, 425, y), fill=(194, 181, 154), width=3)
    table(draw, 370, 590, 900, 74)
    soft_rect(draw, (725, 500, 930, 570), (218, 207, 182), INK, 4, 10)
    tea_cup(draw, 1045, 512, 0.82)
    loose_notes(draw, 1075, 445, 4)
    draw.rectangle((520, 390, 625, 590), fill=(98, 142, 103), outline=INK, width=5)
    draw.ellipse((475, 300, 675, 430), fill=(88, 137, 95), outline=INK, width=4)
    plant(draw, 1210, 565, 0.8)
    paper_stack(draw, 785, 435, 135, 75, 2)
    framed_photo(draw, 965, 445, 92, 70)
    for x, skin, clothes in [(520, SKIN_MED, TEAL), (735, SKIN_DARK, OCHRE), (950, SKIN_LIGHT, RUST)]:
        person(draw, x, 585, 0.8, skin=skin, clothes=clothes, pose="sit")


SCENE_RENDERERS = [scene_1, scene_2, scene_3, scene_4, scene_5, scene_6, scene_7, scene_8]


def render_scene(index: int, scene: dict, output: Path) -> Path:
    image, draw = canvas()
    SCENE_RENDERERS[index - 1](draw)
    subtitle_band(draw)
    image = antialias_finish(grain(image))
    path = output / f"scene_{index:02d}_{scene['time'].replace(':', '').replace('-', '_')}.png"
    image.save(path)
    return path


def contact_sheet(paths: list[Path], output: Path) -> Path:
    thumbs = []
    for path in paths:
        image = Image.open(path).convert("RGB")
        image.thumbnail((480, 270), Image.Resampling.LANCZOS)
        thumb = Image.new("RGB", (480, 300), CREAM)
        thumb.paste(image, ((480 - image.width) // 2, 0))
        ImageDraw.Draw(thumb).text((14, 272), path.stem, fill=INK, font=font(22))
        thumbs.append(thumb)
    sheet = Image.new("RGB", (960, 1200), PAPER)
    for i, thumb in enumerate(thumbs):
        sheet.paste(thumb, ((i % 2) * 480, (i // 2) * 300))
    path = output / "contact_sheet_8.png"
    sheet.save(path)
    return path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--book-dir", type=Path)
    parser.add_argument("--design-dir", type=Path)
    parser.add_argument("--output-dir", type=Path)
    args = parser.parse_args()

    book_dir = find_book_dir(args.book_dir)
    design_dir = args.design_dir or (book_dir / "output_regen_design_001")
    style = json.loads((design_dir / "visual_style_bible.json").read_text(encoding="utf-8"))
    output = args.output_dir or (book_dir / "output_regen_programmatic_001")
    output.mkdir(exist_ok=True)
    paths = [render_scene(scene["index"], scene, output) for scene in style["scenes"]]
    sheet = contact_sheet(paths, output)
    manifest = {
        "source": str(design_dir / "visual_style_bible.json"),
        "style": "controlled_programmatic_documentary_illustration",
        "width": W,
        "height": H,
        "subtitleSafeY": SUBTITLE_SAFE_Y,
        "images": [str(path) for path in paths],
        "contactSheet": str(sheet),
    }
    manifest_path = output / "programmatic_visual_manifest.json"
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"manifest": str(manifest_path), "contactSheet": str(sheet), "count": len(paths)}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
