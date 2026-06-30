#!/usr/bin/env python3
"""Render controlled documentary-style illustrations for No Future Without Forgiveness."""

from __future__ import annotations

import json
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont


BOOK_FOLDER = "001\u6ca1\u6709\u5bbd\u6055\u5c31\u6ca1\u6709\u672a\u6765"
W, H = 1920, 1080
SUBTITLE_SAFE_Y = 770

PAPER = (244, 235, 215)
CREAM = (250, 245, 231)
INK = (38, 42, 43)
MUTED = (100, 112, 108)
OCHRE = (205, 154, 82)
RUST = (174, 82, 58)
TEAL = (75, 124, 128)
BLUE = (111, 153, 166)
SKIN_DARK = (112, 74, 48)
SKIN_MED = (177, 122, 83)
SKIN_LIGHT = (219, 174, 132)
WHITE_CLOTH = (239, 235, 221)


def find_book_dir() -> Path:
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
        shade = int(10 * (y / H))
        draw.line((0, y, W, y), fill=(PAPER[0] - shade, PAPER[1] - shade, PAPER[2] - shade))
    return image, draw


def grain(image: Image.Image) -> Image.Image:
    noise = Image.effect_noise((W, H), 9).convert("L")
    tint = Image.new("RGB", (W, H), (128, 128, 128))
    tint.putalpha(noise.point(lambda value: int(abs(value - 128) * 0.18)))
    out = image.convert("RGBA")
    out.alpha_composite(tint)
    return out.convert("RGB")


def soft_rect(draw: ImageDraw.ImageDraw, xy, fill, outline=None, width=3, radius=18):
    draw.rounded_rectangle(xy, radius=radius, fill=fill, outline=outline, width=width)


def line(draw: ImageDraw.ImageDraw, points, fill=INK, width=5):
    draw.line(points, fill=fill, width=width, joint="curve")


def person(draw: ImageDraw.ImageDraw, x: int, y: int, scale: float = 1.0, skin=SKIN_MED, clothes=TEAL, pose="stand"):
    head_r = int(30 * scale)
    body_w = int(74 * scale)
    body_h = int(118 * scale)
    draw.ellipse((x - head_r, y - body_h - head_r * 2, x + head_r, y - body_h), fill=skin, outline=INK, width=max(2, int(3 * scale)))
    draw.arc((x - head_r, y - body_h - head_r * 2 - 8, x + head_r, y - body_h + 12), 190, 350, fill=INK, width=max(2, int(4 * scale)))
    soft_rect(draw, (x - body_w // 2, y - body_h, x + body_w // 2, y), clothes, INK, max(2, int(3 * scale)), int(22 * scale))
    if pose == "sit":
        line(draw, [(x - body_w // 3, y - 8), (x - int(70 * scale), y + int(52 * scale))], INK, max(3, int(5 * scale)))
        line(draw, [(x + body_w // 3, y - 8), (x + int(70 * scale), y + int(52 * scale))], INK, max(3, int(5 * scale)))
    else:
        line(draw, [(x - body_w // 4, y), (x - int(38 * scale), y + int(90 * scale))], INK, max(3, int(5 * scale)))
        line(draw, [(x + body_w // 4, y), (x + int(38 * scale), y + int(90 * scale))], INK, max(3, int(5 * scale)))
    line(draw, [(x - body_w // 2, y - body_h + int(30 * scale)), (x - int(80 * scale), y - int(48 * scale))], INK, max(3, int(5 * scale)))
    line(draw, [(x + body_w // 2, y - body_h + int(30 * scale)), (x + int(80 * scale), y - int(48 * scale))], INK, max(3, int(5 * scale)))


def table(draw, x, y, w, h, fill=(168, 122, 76)):
    soft_rect(draw, (x, y, x + w, y + h), fill, INK, 4, 14)
    for lx in (x + 40, x + w - 40):
        line(draw, [(lx, y + h), (lx - 20, y + h + 120)], INK, 6)


def window(draw, x, y, w, h, sky=BLUE):
    soft_rect(draw, (x, y, x + w, y + h), (225, 232, 222), INK, 5, 10)
    draw.rectangle((x + 12, y + 12, x + w - 12, y + h - 12), fill=sky)
    line(draw, [(x + w // 2, y + 12), (x + w // 2, y + h - 12)], INK, 3)
    line(draw, [(x + 12, y + h // 2), (x + w - 12, y + h // 2)], INK, 3)


def hills(draw, y=410):
    draw.polygon([(0, y + 70), (260, y - 40), (520, y + 70)], fill=(176, 133, 91))
    draw.polygon([(360, y + 80), (760, y - 90), (1120, y + 70)], fill=(151, 111, 84))
    draw.polygon([(900, y + 60), (1280, y - 20), (1660, y + 60)], fill=(137, 128, 100))
    for x in range(0, W, 180):
        draw.ellipse((x, y + 60, x + 90, y + 130), fill=(77, 119, 89), outline=None)


def road(draw, start=(960, 760), end=(960, 1080), color=(204, 179, 132)):
    sx, sy = start
    ex, ey = end
    draw.polygon([(sx - 90, sy), (sx + 90, sy), (ex + 360, ey), (ex - 360, ey)], fill=color)
    line(draw, [(sx, sy + 20), (ex, ey - 20)], (232, 218, 178), 7)


def label(draw, text: str, x: int, y: int):
    # Labels are metadata for review contact sheets only; scene images avoid text.
    draw.text((x, y), text, fill=INK, font=font(28))


def scene_1(draw):
    window(draw, 1240, 110, 430, 300, sky=(45, 74, 96))
    table(draw, 320, 585, 920, 74)
    soft_rect(draw, (520, 438, 620, 610), OCHRE, INK, 4, 28)
    draw.polygon([(540, 438), (600, 438), (570, 350)], fill=(248, 214, 121), outline=INK)
    soft_rect(draw, (790, 508, 1040, 578), (219, 208, 184), INK, 4, 12)
    draw.line((815, 540, 1015, 540), fill=MUTED, width=3)
    draw.ellipse((1120, 510, 1235, 575), fill=(128, 77, 55), outline=INK, width=4)
    draw.arc((1135, 520, 1210, 610), 0, 180, fill=INK, width=4)
    person(draw, 480, 610, 0.92, skin=SKIN_MED, clothes=TEAL, pose="sit")
    draw.polygon([(1265, 210), (1330, 155), (1400, 220), (1375, 275), (1295, 275)], fill=RUST, outline=INK)
    draw.rectangle((1308, 225, 1362, 275), fill=(226, 218, 190), outline=INK, width=3)


def scene_2(draw):
    hills(draw, 330)
    draw.rectangle((0, 575, W, SUBTITLE_SAFE_Y), fill=(214, 196, 158))
    for x in (360, 560, 760, 960, 1160):
        line(draw, [(x, 360), (x, SUBTITLE_SAFE_Y)], INK, 8)
    line(draw, [(250, 470), (1350, 470)], INK, 8)
    soft_rect(draw, (1180, 455, 1425, 555), (78, 92, 101), INK, 4, 18)
    draw.ellipse((1210, 540, 1265, 595), fill=INK)
    draw.ellipse((1350, 540, 1405, 595), fill=INK)
    for i, x in enumerate((340, 460, 590, 720, 850)):
        person(draw, x, 670, 0.62, skin=[SKIN_DARK, SKIN_MED, SKIN_LIGHT][i % 3], clothes=[TEAL, OCHRE, RUST][i % 3])
        soft_rect(draw, (x + 35, 525, x + 95, 570), WHITE_CLOTH, INK, 3, 5)


def scene_3(draw):
    hills(draw, 300)
    road(draw, (960, 515), (960, 1080))
    soft_rect(draw, (740, 420, 1180, 535), (218, 205, 178), INK, 5, 12)
    soft_rect(draw, (900, 450, 1010, 520), WHITE_CLOTH, INK, 4, 7)
    for i, x in enumerate(range(280, 1530, 155)):
        person(draw, x, 660 + (i % 2) * 16, 0.58, skin=[SKIN_DARK, SKIN_MED, SKIN_LIGHT][i % 3], clothes=[TEAL, OCHRE, RUST][i % 3])
    draw.rectangle((1290, 185, 1380, 410), fill=(205, 190, 158), outline=INK, width=5)
    draw.polygon([(1260, 185), (1420, 185), (1340, 105)], fill=RUST, outline=INK)


def scene_4(draw):
    draw.rectangle((140, 120, 1780, SUBTITLE_SAFE_Y), fill=(226, 218, 202), outline=INK, width=6)
    for x in (460, 790, 1120, 1450):
        line(draw, [(x, 145), (x, 330)], (190, 178, 154), 4)
    table(draw, 530, 505, 860, 78, fill=(156, 110, 72))
    for x in (700, 920, 1140):
        draw.ellipse((x, 455, x + 28, 483), fill=INK)
        line(draw, [(x + 14, 483), (x + 14, 522)], INK, 5)
    soft_rect(draw, (1260, 420, 1380, 505), (190, 176, 136), INK, 4, 10)
    soft_rect(draw, (1280, 398, 1360, 430), WHITE_CLOTH, INK, 3, 4)
    person(draw, 820, 500, 0.72, skin=SKIN_DARK, clothes=TEAL, pose="sit")
    person(draw, 1070, 500, 0.65, skin=SKIN_LIGHT, clothes=OCHRE, pose="sit")
    for x, skin, clothes in [(360, SKIN_MED, RUST), (1510, SKIN_DARK, TEAL), (1625, SKIN_LIGHT, OCHRE)]:
        person(draw, x, 700, 0.6, skin=skin, clothes=clothes, pose="sit")
    soft_rect(draw, (265, 560, 370, 700), (202, 186, 154), INK, 4, 10)


def scene_5(draw):
    window(draw, 1240, 135, 400, 310, sky=(151, 173, 177))
    for x in range(1280, 1600, 55):
        line(draw, [(x, 145), (x - 40, 430)], (123, 145, 150), 2)
    table(draw, 430, 590, 980, 75)
    person(draw, 760, 585, 0.92, skin=SKIN_DARK, clothes=(122, 130, 128), pose="sit")
    soft_rect(draw, (930, 500, 1130, 620), WHITE_CLOTH, INK, 4, 8)
    draw.ellipse((1080, 620, 1235, 690), fill=(172, 128, 88), outline=INK, width=4)
    soft_rect(draw, (1180, 520, 1360, 615), (194, 169, 119), INK, 4, 8)
    draw.rectangle((980, 520, 1070, 585), fill=(185, 161, 120), outline=INK, width=4)


def scene_6(draw):
    hills(draw, 290)
    draw.rectangle((0, 555, W, SUBTITLE_SAFE_Y), fill=(208, 195, 160))
    draw.polygon([(900, 560), (1030, 560), (1560, 1080), (1200, 1080)], fill=(185, 154, 107))
    draw.polygon([(850, 560), (970, 560), (600, 1080), (240, 1080)], fill=(132, 116, 96))
    for x in range(250, 620, 75):
        line(draw, [(x, 700), (x + 65, 760)], (73, 78, 75), 5)
    for x in (1260, 1375, 1490):
        soft_rect(draw, (x, 515, x + 120, 610), (205, 184, 139), INK, 4, 12)
        draw.polygon([(x - 10, 515), (x + 60, 465), (x + 130, 515)], fill=RUST, outline=INK)
    person(draw, 725, 705, 0.72, skin=SKIN_DARK, clothes=TEAL)
    person(draw, 1080, 700, 0.72, skin=SKIN_LIGHT, clothes=OCHRE)
    person(draw, 930, 665, 0.55, skin=SKIN_MED, clothes=RUST, pose="sit")


def scene_7(draw):
    draw.rectangle((0, 0, W, H), fill=(224, 217, 199))
    hills(draw, 250)
    draw.rectangle((260, 350, 870, 620), fill=(190, 178, 157), outline=INK, width=6)
    draw.polygon([(230, 350), (565, 210), (900, 350)], fill=(137, 110, 92), outline=INK)
    draw.rectangle((500, 435, 630, 620), fill=(82, 83, 80), outline=INK, width=5)
    for x in (1020, 1120, 1220, 1320):
        draw.ellipse((x, 610, x + 35, 665), fill=(232, 190, 90), outline=INK, width=3)
        line(draw, [(x + 17, 665), (x + 17, 720)], INK, 4)
    draw.polygon([(970, 545), (1420, 540), (1450, 625), (940, 635)], fill=WHITE_CLOTH, outline=INK)
    for x in (1480, 1600):
        person(draw, x, 700, 0.62, skin=SKIN_DARK, clothes=(92, 92, 88))


def scene_8(draw):
    window(draw, 1190, 110, 430, 310, sky=(154, 194, 205))
    road(draw, (1395, 395), (1500, 1080), color=(210, 187, 136))
    table(draw, 370, 590, 900, 74)
    soft_rect(draw, (725, 500, 930, 570), (218, 207, 182), INK, 4, 10)
    draw.ellipse((1050, 510, 1170, 570), fill=(135, 86, 58), outline=INK, width=4)
    draw.rectangle((520, 390, 625, 590), fill=(98, 142, 103), outline=INK, width=5)
    draw.ellipse((475, 300, 675, 430), fill=(88, 137, 95), outline=INK, width=4)
    for x, skin, clothes in [(520, SKIN_MED, TEAL), (735, SKIN_DARK, OCHRE), (950, SKIN_LIGHT, RUST)]:
        person(draw, x, 585, 0.74, skin=skin, clothes=clothes, pose="sit")


SCENE_RENDERERS = [scene_1, scene_2, scene_3, scene_4, scene_5, scene_6, scene_7, scene_8]


def render_scene(index: int, scene: dict, output: Path) -> Path:
    image, draw = canvas()
    SCENE_RENDERERS[index - 1](draw)
    draw.rectangle((0, SUBTITLE_SAFE_Y, W, H), fill=(246, 240, 224))
    draw.line((0, SUBTITLE_SAFE_Y, W, SUBTITLE_SAFE_Y), fill=(218, 204, 176), width=4)
    image = grain(image).filter(ImageFilter.UnsharpMask(radius=1.2, percent=115, threshold=3))
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
    book_dir = find_book_dir()
    design_dir = book_dir / "output_regen_design_001"
    style = json.loads((design_dir / "visual_style_bible.json").read_text(encoding="utf-8"))
    output = book_dir / "output_regen_programmatic_001"
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
