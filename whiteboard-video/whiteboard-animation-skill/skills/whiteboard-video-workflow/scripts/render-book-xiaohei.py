#!/usr/bin/env python3
"""Render deterministic xiaohei-style book illustrations for Camellia's Letter."""

from __future__ import annotations

import argparse
import json
import math
import re
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


W, H = 1920, 1080
RENDER_SCALE = 1
OUTPUT_SCALE = 1
INK = (34, 36, 38)
MUTED = (116, 125, 124)
PAPER = (248, 246, 239)
BEIGE = (226, 216, 196)
TEAL = (90, 128, 130)
RED = (188, 70, 62)
YELLOW = (226, 176, 78)
BLUE = (70, 130, 190)
ORANGE = (232, 133, 54)


@dataclass(frozen=True)
class Scene:
    title: str
    subtitle: str
    layout: str


SCENES = [
    Scene("reopen letter shop", "a brass key, camellia pot, and blank envelopes", "shop"),
    Scene("write for another heart", "desk, pen, tea, envelope, and quiet concentration", "desk"),
    Scene("family table distance", "two rice bowls, school bag, and an unsent letter", "kitchen"),
    Scene("half-open doorway", "two cushions, shoes, and a conversation not yet started", "doorway"),
    Scene("old letters return", "wooden box, tied envelopes, camellia cloth, dust light", "box"),
    Scene("ferry to memory", "rail, wrapped letter box, wave icon, distant island", "ferry"),
    Scene("blocked night writing", "lamp, rain window, tea cup, and blank paper", "night"),
    Scene("quiet repair", "sea rail, sealed envelope, camellias, and sunrise", "sea"),
]


FULL_SCENE_BLUEPRINTS = [
    ("把灯调暗一点", ["山茶的情书", "慢慢打开", "纸张柔软"], "book"),
    ("代写不是排句子", ["替心找姿势", "道歉", "祝福", "诀别"], "translate"),
    ("五口之家像太空旅行", ["怀孕生产育儿", "忙乱失重", "重新营业"], "orbit"),
    ("她想回到代写桌前", ["家务", "升学", "青春期", "距离"], "desk"),
    ("青春期像半掩的门", ["妈妈", "孩子", "没回应"], "door"),
    ("门外的母亲", ["想敲门", "又怕太响", "关系的缝"], "door"),
    ("热红酒给自己一点暖", ["肉桂", "蜂蜜", "橘子", "老房子"], "kettle"),
    ("可以停一停", ["不用立刻回应", "暖一暖", "独处片刻"], "rest"),
    ("祖母留下的旧信", ["严厉", "深刻", "难和解"], "box"),
    ("渡轮带着旧信出发", ["伊豆大岛", "木盒", "海风"], "ferry"),
    ("把旧日心意送到终点", ["不是评判", "是安放", "让它们相遇"], "ritual"),
    ("代写最难的是触碰孤独", ["委托", "愤怒", "希望", "真心"], "machine"),
    ("任务结束后空虚来了", ["升学完成", "大任务结束", "不知道自己是谁"], "empty"),
    ("写不动的时候", ["不是懒", "是慢慢没力气", "还在呼吸"], "lamp"),
    ("家人听懂了沉默", ["蜜朗", "平凡的懂", "一杯饮品"], "table"),
    ("阳菜长大了", ["校服", "青春期", "未来先披上身"], "uniform"),
    ("山茶文具店仍在发光", ["纸张", "信封", "印章", "旧物有生命"], "shop"),
    ("没寄出的信", ["道歉", "感谢", "告别", "放在心里"], "mailbox"),
    ("四季替人说话", ["线香花火", "金木犀", "山茶", "明日叶"], "seasons"),
    ("温柔的人也会不够好", ["照顾别人", "怀疑自己", "暗暗发光"], "mirror"),
    ("旧信不再孤单", ["爱", "思念", "秘密", "岁月"], "box"),
    ("看见长辈也是普通人", ["年轻过", "笨拙过", "也曾不安"], "bridge"),
    ("珍贵的话要等一等", ["沉默足够久", "才出现一句真话"], "lamp"),
    ("老屋托住夜晚", ["风声", "纸张", "慢下来"], "room"),
    ("也要给自己代笔", ["被倾听", "被认真安放", "值得"], "self_letter"),
    ("在新生活里呼吸", ["不是回到过去", "找到新的节奏"], "path"),
    ("未说出口也可以先放好", ["安全的地方", "明天再写", "没关系"], "heart"),
    ("听见生活深处的回声", ["睡前", "通勤", "独处"], "echo"),
    ("从文具店走向自我修复", ["家人", "旧信", "离别", "修复"], "map"),
    ("愿你也写下一句", ["不必完美", "继续往前", "慢慢来"], "ending"),
]


PROP_KEYWORDS = [
    ("灯光", "lamp"),
    ("灯", "lamp"),
    ("手机", "phone"),
    ("窗帘", "curtain"),
    ("窗", "window"),
    ("车声", "car"),
    ("纸张", "paper"),
    ("纸", "paper"),
    ("墨迹", "ink"),
    ("墨水", "ink"),
    ("信纸", "paper"),
    ("信封", "envelope"),
    ("信", "envelope"),
    ("笔墨", "pen"),
    ("笔", "pen"),
    ("印章", "stamp"),
    ("风", "wind"),
    ("旧屋", "old_house"),
    ("老屋", "old_house"),
    ("孩子", "child"),
    ("女儿", "child"),
    ("阳菜", "child"),
    ("小梅", "child"),
    ("莲太郎", "child"),
    ("文具店", "shop"),
    ("祖母", "elder"),
    ("丈夫", "family"),
    ("家务", "chores"),
    ("升学", "school"),
    ("门", "door"),
    ("电视", "tv"),
    ("水桶", "bucket"),
    ("热红酒", "wine"),
    ("红酒", "wine"),
    ("肉桂", "spice"),
    ("丁香", "spice"),
    ("八角", "spice"),
    ("蜂蜜", "honey"),
    ("橘子", "orange"),
    ("苹果", "apple"),
    ("老房子", "old_house"),
    ("旧信", "old_letters"),
    ("木盒", "box"),
    ("汽船", "ferry"),
    ("渡轮", "ferry"),
    ("海", "sea"),
    ("港", "port"),
    ("雨", "rain"),
    ("杯子", "cup"),
    ("饮品", "cup"),
    ("校服", "uniform"),
    ("道歉", "letter_words"),
    ("感谢", "letter_words"),
    ("花火", "fireworks"),
    ("金木犀", "osmanthus"),
    ("山茶", "camellia"),
    ("明日叶", "leaf"),
    ("莲", "lotus"),
    ("花", "flower"),
    ("泥", "mud"),
    ("父母", "family"),
    ("长辈", "elder"),
    ("夜晚", "night"),
    ("家庭", "family"),
    ("书", "book"),
    ("桌", "desk"),
    ("椅子", "chair"),
    ("厨房", "kitchen"),
    ("饭桌", "table"),
    ("便当", "bento"),
    ("书包", "bag"),
    ("钥匙", "key"),
    ("茶杯", "cup"),
    ("植物", "plant"),
    ("岛", "island"),
    ("波浪", "waves"),
]


PROP_LABELS = {
    "lamp": "灯",
    "phone": "手机",
    "curtain": "窗帘",
    "window": "窗",
    "car": "车声",
    "paper": "纸张",
    "ink": "墨水",
    "envelope": "信封",
    "pen": "笔",
    "stamp": "印章",
    "wind": "风",
    "old_house": "老屋",
    "child": "孩子",
    "shop": "文具店",
    "elder": "长辈",
    "family": "家人",
    "chores": "家务",
    "school": "升学",
    "door": "门",
    "tv": "电视",
    "bucket": "水桶",
    "wine": "红酒",
    "spice": "香料",
    "honey": "蜂蜜",
    "orange": "橘子",
    "apple": "苹果",
    "old_letters": "旧信",
    "box": "木盒",
    "ferry": "汽船",
    "sea": "海",
    "port": "港口",
    "rain": "雨",
    "cup": "杯子",
    "uniform": "校服",
    "letter_words": "心意",
    "fireworks": "花火",
    "osmanthus": "金木犀",
    "camellia": "山茶",
    "leaf": "明日叶",
    "lotus": "莲",
    "flower": "花",
    "mud": "泥",
    "night": "夜晚",
    "book": "书",
    "desk": "桌",
    "chair": "椅子",
    "kitchen": "厨房",
    "table": "饭桌",
    "bento": "便当",
    "bag": "书包",
    "key": "钥匙",
    "plant": "植物",
    "island": "岛",
    "waves": "波浪",
}


def font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    size = max(1, int(size * RENDER_SCALE))
    for name in ("arial.ttf", "C:/Windows/Fonts/arial.ttf", "C:/Windows/Fonts/segoeui.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            continue
    return ImageFont.load_default()


def zh_font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    size = max(1, int(size * RENDER_SCALE))
    for name in (
        "C:/Windows/Fonts/simkai.ttf",
        "C:/Windows/Fonts/STKAITI.TTF",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/simhei.ttf",
    ):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            continue
    return font(size)


def canvas() -> Image.Image:
    return Image.new("RGB", (W * RENDER_SCALE, H * RENDER_SCALE), "white")


def _scale_value(value):
    if isinstance(value, (int, float)):
        return value * RENDER_SCALE
    if isinstance(value, tuple):
        return tuple(_scale_value(v) for v in value)
    if isinstance(value, list):
        return [_scale_value(v) for v in value]
    return value


class ScaledDraw:
    def __init__(self, draw: ImageDraw.ImageDraw):
        self._draw = draw

    def line(self, xy, fill=None, width=1, joint=None):
        self._draw.line(_scale_value(xy), fill=fill, width=max(1, int(width * RENDER_SCALE)), joint=joint)

    def rounded_rectangle(self, xy, radius=0, fill=None, outline=None, width=1):
        self._draw.rounded_rectangle(
            _scale_value(xy),
            radius=max(0, int(radius * RENDER_SCALE)),
            fill=fill,
            outline=outline,
            width=max(1, int(width * RENDER_SCALE)),
        )

    def rectangle(self, xy, fill=None, outline=None, width=1):
        self._draw.rectangle(_scale_value(xy), fill=fill, outline=outline, width=max(1, int(width * RENDER_SCALE)))

    def ellipse(self, xy, fill=None, outline=None, width=1):
        self._draw.ellipse(_scale_value(xy), fill=fill, outline=outline, width=max(1, int(width * RENDER_SCALE)))

    def polygon(self, xy, fill=None, outline=None):
        self._draw.polygon(_scale_value(xy), fill=fill, outline=outline)

    def pieslice(self, xy, start, end, fill=None, outline=None, width=1):
        self._draw.pieslice(_scale_value(xy), start, end, fill=fill, outline=outline, width=max(1, int(width * RENDER_SCALE)))

    def arc(self, xy, start, end, fill=None, width=1):
        self._draw.arc(_scale_value(xy), start, end, fill=fill, width=max(1, int(width * RENDER_SCALE)))

    def text(self, xy, text, font=None, fill=None, **kwargs):
        self._draw.text(_scale_value(xy), text, font=font, fill=fill, **kwargs)

    def textbbox(self, xy, text, font=None, **kwargs):
        box = self._draw.textbbox(_scale_value(xy), text, font=font, **kwargs)
        return tuple(v / RENDER_SCALE for v in box)


def draw_for(im: Image.Image) -> ScaledDraw:
    return ScaledDraw(ImageDraw.Draw(im))


def finish_image(im: Image.Image) -> Image.Image:
    target = (W * OUTPUT_SCALE, H * OUTPUT_SCALE)
    if im.size == target:
        return im
    return im.resize(target, Image.Resampling.LANCZOS)


def line(draw: ImageDraw.ImageDraw, points, fill=INK, width=5):
    draw.line(points, fill=fill, width=width, joint="curve")


def rounded(draw: ImageDraw.ImageDraw, box, radius=24, outline=INK, width=5, fill=None):
    draw.rounded_rectangle(box, radius=radius, outline=outline, width=width, fill=fill)


def draw_text(draw: ImageDraw.ImageDraw, xy, text: str, size=34, fill=MUTED):
    draw.text(xy, text, font=font(size), fill=fill)


def zh_text(draw: ImageDraw.ImageDraw, xy, text: str, size=34, fill=INK):
    draw.text(xy, text, font=zh_font(size), fill=fill)


def text_size(draw: ImageDraw.ImageDraw, text: str, size=34):
    if not text:
        return 0, 0
    box = draw.textbbox((0, 0), text, font=zh_font(size))
    return box[2] - box[0], box[3] - box[1]


def wrap_zh(text: str, max_chars: int, max_lines: int | None = None) -> list[str]:
    if not text:
        return []
    chars = list(text)
    lines = ["".join(chars[i : i + max_chars]) for i in range(0, len(chars), max_chars)]
    if max_lines and len(lines) > max_lines:
        lines = lines[:max_lines]
        if len(lines[-1]) >= 2:
            lines[-1] = lines[-1][:-1] + "…"
    return lines


def wrap_zh_pixels(draw: ImageDraw.ImageDraw, text: str, max_width: int, size: int, max_lines: int | None = None) -> list[str]:
    if not text:
        return []
    lines: list[str] = []
    current = ""
    for ch in text:
        trial = current + ch
        tw, _ = text_size(draw, trial, size)
        if current and tw > max_width:
            lines.append(current)
            current = ch
        else:
            current = trial
    if current:
        lines.append(current)
    if max_lines and len(lines) > max_lines:
        lines = lines[:max_lines]
        if len(lines[-1]) >= 2:
            lines[-1] = lines[-1][:-1] + "…"
    return lines


def zh_multiline(draw: ImageDraw.ImageDraw, xy, text: str, size=28, fill=INK, max_chars=8, max_lines=3, line_gap=6):
    x, y = xy
    for i, part in enumerate(wrap_zh(text, max_chars, max_lines)):
        zh_text(draw, (x, y + i * (size + line_gap)), part, size=size, fill=fill)


def arrow(draw: ImageDraw.ImageDraw, start, end, fill=ORANGE, width=4, dashed=False):
    x1, y1 = start
    x2, y2 = end
    if dashed:
        steps = 18
        for i in range(0, steps, 2):
            a = i / steps
            b = min((i + 1) / steps, 1)
            line(draw, [(x1 + (x2 - x1) * a, y1 + (y2 - y1) * a), (x1 + (x2 - x1) * b, y1 + (y2 - y1) * b)], fill=fill, width=width)
    else:
        line(draw, [start, end], fill=fill, width=width)
    ang = math.atan2(y2 - y1, x2 - x1)
    size = 18
    p1 = (x2 - math.cos(ang - 0.55) * size, y2 - math.sin(ang - 0.55) * size)
    p2 = (x2 - math.cos(ang + 0.55) * size, y2 - math.sin(ang + 0.55) * size)
    draw.polygon([end, p1, p2], fill=fill)


def smooth_wave(draw: ImageDraw.ImageDraw, x1, y, x2, amplitude=10, wavelength=96, fill=TEAL, width=4, cycles=None):
    """Draw a horizontal sine wave; avoids jagged diagonal guide lines."""
    if x2 < x1:
        x1, x2 = x2, x1
    length = max(1, x2 - x1)
    if cycles is None:
        cycles = max(1.0, length / wavelength)
    samples = max(24, int(length / 8))
    pts = []
    for i in range(samples + 1):
        t = i / samples
        x = x1 + length * t
        yy = y + math.sin(t * cycles * math.tau) * amplitude
        pts.append((x, yy))
    line(draw, pts, fill=fill, width=width)


def smooth_wave_arrow(draw: ImageDraw.ImageDraw, x1, y, x2, fill=BLUE, width=4, amplitude=8):
    join_x = x2 - 32
    smooth_wave(draw, x1, y, join_x, amplitude=amplitude, wavelength=112, fill=fill, width=width)
    arrow(draw, (join_x, y), (x2, y), fill=fill, width=width, dashed=False)


def curved_arrow(draw: ImageDraw.ImageDraw, pts, fill=ORANGE, width=4, dashed=False):
    if dashed:
        for i in range(len(pts) - 1):
            arrow(draw, pts[i], pts[i + 1], fill=fill, width=width, dashed=True)
        return
    line(draw, pts, fill=fill, width=width)
    arrow(draw, pts[-2], pts[-1], fill=fill, width=width)


def label(draw: ImageDraw.ImageDraw, text: str, x: int, y: int, fill=RED, size=34, underline=False):
    zh_text(draw, (x, y), text, size=size, fill=fill)
    if underline:
        tw, th = text_size(draw, text, size)
        underline_y = y + th + 12
        smooth_wave(draw, x - 6, underline_y, x + tw + 6, amplitude=3, wavelength=70, fill=fill, width=3)


def note_box(draw: ImageDraw.ImageDraw, x, y, w, h, text: str, fill=(255, 255, 255), outline=INK):
    rounded(draw, [x, y, x + w, y + h], radius=8, fill=fill, outline=outline, width=3)
    max_chars = max(2, int(w / 34))
    lines = wrap_zh(text, max_chars, 2)
    line_height = 32
    start_y = y + max(10, (h - len(lines) * line_height) / 2 - 2)
    for i, part in enumerate(lines):
        tw, _ = text_size(draw, part, 28)
        zh_text(draw, (x + max(10, (w - tw) / 2), start_y + i * line_height), part, size=28, fill=INK)


def excerpt_box(draw: ImageDraw.ImageDraw, x, y, w, h, text: str):
    rounded(draw, [x, y, x + w, y + h], radius=8, fill="white", outline=INK, width=3)
    size = 26
    line_height = 32
    max_lines = max(2, int((h - 30) / line_height))
    lines = wrap_zh_pixels(draw, text, int(w - 58), size, max_lines)
    yy = y + 24
    for part in lines:
        zh_text(draw, (x + 30, yy), part, size=size, fill=MUTED)
        yy += line_height


def xiaohei(draw: ImageDraw.ImageDraw, x, y, s=1.0, pose="stand", eyes="calm"):
    """Draw the current infographic actor: a cute little girl IP."""
    skin = (255, 226, 198)
    hair = (30, 31, 32)
    dress = (95, 145, 150)
    apron = (255, 250, 238)
    cheek = (226, 112, 104)

    # shadow and legs
    draw.ellipse([x - 58 * s, y + 100 * s, x + 58 * s, y + 125 * s], fill=(230, 230, 224))
    leg_y = y + 82 * s
    line(draw, [(x - 24 * s, leg_y), (x - 34 * s, y + 138 * s)], width=max(2, int(4 * s)))
    line(draw, [(x + 24 * s, leg_y), (x + 34 * s, y + 138 * s)], width=max(2, int(4 * s)))

    # dress and apron
    draw.polygon(
        [(x - 58 * s, y - 5 * s), (x + 58 * s, y - 5 * s), (x + 76 * s, y + 96 * s), (x - 76 * s, y + 96 * s)],
        fill=dress,
        outline=INK,
    )
    draw.polygon(
        [(x - 32 * s, y + 4 * s), (x + 32 * s, y + 4 * s), (x + 42 * s, y + 82 * s), (x - 42 * s, y + 82 * s)],
        fill=apron,
        outline=INK,
    )
    rounded(draw, [x - 18 * s, y + 45 * s, x + 18 * s, y + 70 * s], radius=int(7 * s), fill="white", width=max(1, int(2 * s)))

    # arms by action
    if pose == "pull":
        line(draw, [(x - 48 * s, y + 8 * s), (x - 132 * s, y - 55 * s)], width=max(3, int(5 * s)))
        line(draw, [(x + 48 * s, y + 8 * s), (x + 126 * s, y - 45 * s)], width=max(3, int(5 * s)))
    elif pose == "push":
        line(draw, [(x + 48 * s, y + 0 * s), (x + 138 * s, y - 34 * s)], width=max(3, int(5 * s)))
        line(draw, [(x + 44 * s, y + 28 * s), (x + 128 * s, y + 36 * s)], width=max(3, int(5 * s)))
        line(draw, [(x - 48 * s, y + 12 * s), (x - 92 * s, y + 58 * s)], width=max(3, int(5 * s)))
    elif pose == "carry":
        line(draw, [(x - 44 * s, y + 0 * s), (x - 92 * s, y - 76 * s)], width=max(3, int(5 * s)))
        line(draw, [(x + 44 * s, y + 0 * s), (x + 92 * s, y - 76 * s)], width=max(3, int(5 * s)))
    elif pose == "write":
        line(draw, [(x - 44 * s, y + 18 * s), (x - 88 * s, y + 72 * s)], width=max(3, int(5 * s)))
        line(draw, [(x + 44 * s, y + 18 * s), (x + 106 * s, y + 62 * s)], width=max(3, int(5 * s)))
    else:
        line(draw, [(x - 48 * s, y + 12 * s), (x - 86 * s, y + 68 * s)], width=max(3, int(5 * s)))
        line(draw, [(x + 48 * s, y + 12 * s), (x + 86 * s, y + 68 * s)], width=max(3, int(5 * s)))

    # head, bob hair, face
    draw.ellipse([x - 54 * s, y - 122 * s, x + 54 * s, y - 14 * s], fill=skin, outline=INK, width=max(2, int(4 * s)))
    draw.pieslice([x - 66 * s, y - 130 * s, x + 66 * s, y + 10 * s], 180, 360, fill=hair, outline=INK, width=max(2, int(3 * s)))
    draw.rectangle([x - 56 * s, y - 78 * s, x + 56 * s, y - 40 * s], fill=hair)
    draw.ellipse([x - 62 * s, y - 78 * s, x - 34 * s, y - 18 * s], fill=hair)
    draw.ellipse([x + 34 * s, y - 78 * s, x + 62 * s, y - 18 * s], fill=hair)
    line(draw, [(x - 42 * s, y - 78 * s), (x + 42 * s, y - 78 * s)], fill=INK, width=max(2, int(4 * s)))

    eye_y = y - 58 * s
    if eyes == "wide":
        eye_r = 7 * s
        for ex in (-20, 20):
            draw.ellipse([x + ex * s - eye_r, eye_y - eye_r, x + ex * s + eye_r, eye_y + eye_r], fill=INK)
        draw.arc([x - 18 * s, y - 42 * s, x + 18 * s, y - 18 * s], 10, 170, fill=INK, width=max(2, int(3 * s)))
    elif eyes == "tired":
        for ex in (-20, 20):
            line(draw, [(x + (ex - 8) * s, eye_y), (x + (ex + 8) * s, eye_y + 4 * s)], fill=INK, width=max(2, int(3 * s)))
        line(draw, [(x - 13 * s, y - 30 * s), (x + 13 * s, y - 28 * s)], fill=INK, width=max(2, int(3 * s)))
    else:
        for ex in (-20, 20):
            draw.ellipse([x + ex * s - 5 * s, eye_y - 5 * s, x + ex * s + 5 * s, eye_y + 5 * s], fill=INK)
        draw.arc([x - 18 * s, y - 42 * s, x + 18 * s, y - 20 * s], 20, 160, fill=INK, width=max(2, int(3 * s)))
    draw.ellipse([x - 39 * s, y - 42 * s, x - 26 * s, y - 29 * s], fill=cheek)
    draw.ellipse([x + 26 * s, y - 42 * s, x + 39 * s, y - 29 * s], fill=cheek)
    camellia(draw, x + 42 * s, y - 96 * s, 13 * s)


def hand_note(draw, x, y, w=120, h=86, text=""):
    rounded(draw, [x, y, x + w, y + h], radius=6, fill="white", outline=INK, width=3)
    if text:
        max_chars = max(2, int((w - 28) / 26))
        lines = wrap_zh(text, max_chars, max(1, int((h - 22) / 28)))
        start_y = y + max(10, (h - len(lines) * 28) / 2)
        for i, part in enumerate(lines):
            zh_text(draw, (x + 14, start_y + i * 28), part, size=23, fill=MUTED)
    else:
        for i in range(3):
            line(draw, [(x + 18, y + 25 + i * 18), (x + w - 18 - i * 8, y + 25 + i * 18)], fill=(160, 160, 160), width=2)


def simple_book(draw, x, y, w=180, h=120, title="书"):
    rounded(draw, [x, y, x + w, y + h], radius=8, fill="white", outline=INK, width=4)
    line(draw, [(x + w * 0.5, y + 12), (x + w * 0.5, y + h - 12)], width=3)
    note_lines(draw, x + 24, y + 38, rows=3, w=int(w * 0.32))
    note_lines(draw, x + int(w * 0.58), y + 38, rows=3, w=int(w * 0.28))
    zh_text(draw, (x + 36, y - 40), title, size=28, fill=INK)


def camellia_branch(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    line(draw, [(x, y), (x + 150 * s, y - 65 * s), (x + 270 * s, y - 28 * s)], fill=(82, 102, 88), width=max(2, int(4 * s)))
    for ox, oy, r in [(55, -28, 20), (135, -60, 17), (230, -38, 22)]:
        camellia(draw, x + ox * s, y + oy * s, r * s)
    for ox, oy in [(90, -38), (190, -48), (255, -30)]:
        draw.ellipse([x + ox * s - 18 * s, y + oy * s - 8 * s, x + ox * s + 18 * s, y + oy * s + 8 * s], fill=(122, 153, 137), outline=INK, width=max(1, int(2 * s)))


def split_srt(path: Path, count: int):
    text = path.read_text(encoding="utf-8-sig", errors="replace")
    pattern = re.compile(r"(\d+)\s+([\d:,]+)\s+-->\s+([\d:,]+)\s+(.+?)(?=\n\s*\n|\Z)", re.S)

    def to_ms(value: str) -> int:
        h, m, sms = value.split(":")
        s, ms = sms.split(",")
        return (int(h) * 3600 + int(m) * 60 + int(s)) * 1000 + int(ms)

    entries = []
    for match in pattern.finditer(text):
        body = "".join(line.strip() for line in match.group(4).splitlines() if line.strip())
        entries.append((to_ms(match.group(2)), to_ms(match.group(3)), body))
    if not entries:
        raise ValueError(f"No SRT entries found: {path}")
    total = entries[-1][1]
    chunks = []
    for i in range(count):
        start = i * total // count
        end = (i + 1) * total // count
        chunk_text = "".join(body for a, b, body in entries if b > start and a < end)
        chunks.append({"index": i + 1, "start_ms": start, "end_ms": end, "text": chunk_text})
    return chunks


def pick_blueprint(index: int):
    if index <= len(FULL_SCENE_BLUEPRINTS):
        return FULL_SCENE_BLUEPRINTS[index - 1]
    return FULL_SCENE_BLUEPRINTS[(index - 1) % len(FULL_SCENE_BLUEPRINTS)]


def extract_props(text: str, limit: int = 6) -> list[str]:
    scored = []
    seen = set()
    for keyword, prop in PROP_KEYWORDS:
        pos = text.find(keyword)
        if pos >= 0 and prop not in seen:
            seen.add(prop)
            count = text.count(keyword)
            scored.append((pos, -count, prop))
    scored.sort()
    return [prop for _, __, prop in scored[:limit]]


def merge_props(kind: str, text: str) -> list[str]:
    props = extract_props(text, 6)
    defaults = {
        "book": ["book", "paper", "lamp"],
        "translate": ["paper", "pen", "envelope"],
        "orbit": ["child", "bag", "cup"],
        "desk": ["desk", "pen", "paper", "cup"],
        "door": ["door", "phone", "bag"],
        "kettle": ["wine", "spice", "orange", "cup"],
        "rest": ["window", "cup", "plant"],
        "box": ["box", "old_letters", "camellia"],
        "ferry": ["ferry", "sea", "island", "box"],
        "ritual": ["old_letters", "box", "flower"],
        "machine": ["paper", "pen", "stamp"],
        "empty": ["window", "cup", "paper"],
        "lamp": ["lamp", "rain", "paper", "pen"],
        "table": ["table", "cup", "family"],
        "uniform": ["uniform", "bag", "school"],
        "shop": ["shop", "paper", "envelope", "stamp"],
        "mailbox": ["envelope", "letter_words", "flower"],
        "seasons": ["fireworks", "osmanthus", "camellia", "leaf", "lotus"],
        "mirror": ["family", "cup", "window"],
        "bridge": ["elder", "old_letters", "bridge"],
        "room": ["window", "rain", "paper", "cup"],
        "self_letter": ["paper", "pen", "envelope"],
        "path": ["plant", "waves", "island"],
        "heart": ["envelope", "camellia", "cup"],
        "echo": ["window", "wind", "cup"],
        "map": ["shop", "family", "old_letters", "camellia"],
        "ending": ["paper", "pen", "camellia"],
    }
    for prop in defaults.get(kind, []):
        if prop not in props:
            props.append(prop)
        if len(props) >= 6:
            break
    return props[:6]


def scene_caption(kind: str, title: str, props: list[str]) -> str:
    captions = {
        "book": "这一页像一封打开的信。山茶、书和灯，先把故事的语气放轻。",
        "translate": "说不出口的话，被放进代写窗口。纸张和信封替人开口。",
        "orbit": "家务、育儿和升学围着一家人转。生活像一场忙乱的太空旅行。",
        "desk": "桌上摊着纸和笔。她试着把真话写得温柔一点。",
        "door": "门只开了一半。想靠近，也害怕听见答案。",
        "kettle": "热饮、香料和橘子让画面慢下来。这里写的是照顾和等待。",
        "rest": "窗边有茶和植物。故事在安静处重新呼吸。",
        "box": "旧信从盒子里翻出来。过去不再只是回忆，也成了证据。",
        "ferry": "渡轮、海面和远岛把人带回原点。她抱着信盒重新看见过去。",
        "ritual": "旧信被一封封整理。告别不是丢掉，而是重新安放。",
        "machine": "代写店像一台翻译机。它把沉默翻成可以寄出的纸。",
        "empty": "空房间里只剩杯子和纸。没有说出口的话，也在场。",
        "lamp": "雨夜里，灯照着纸面。卡住的不是笔，而是真话。",
        "table": "饭桌把一家人放在一起。沉默绕着碗边转圈。",
        "uniform": "校服和书包把时间往前推。孩子长大，家也跟着变化。",
        "shop": "文具店重新开门。信纸、印章和花，把日常重新接上。",
        "mailbox": "信封排成一排。道歉、感谢和告别，都等着被寄出。",
        "seasons": "四季在花和叶子里轮换。生活慢慢把伤口磨平。",
        "mirror": "镜子照见的不是完美。她开始和不够好的自己和解。",
        "bridge": "两端的人隔着一座桥。理解要慢慢走过去。",
        "room": "雨窗和茶杯把房间留住。她在安静里把心事理顺。",
        "self_letter": "这封信写给自己。不是解释过去，而是给明天留路。",
        "path": "小路通向新生活。她带着过去，但不再被过去拖住。",
        "heart": "有些话没有说出口。信封和山茶替心意留了位置。",
        "echo": "窗边的回声还在。旧话变轻，新的回答才进来。",
        "map": "文具店、家人和旧信连成地图。她终于知道该往哪里走。",
        "ending": "最后一页没有用力收束。纸、笔和山茶，把故事轻轻合上。",
    }
    if kind in captions:
        return captions[kind]
    labels = "、".join(PROP_LABELS.get(prop, prop) for prop in props[:3])
    return f"{title}。画面里有{labels}，用来提示这一段的情绪。"


def concept_object(draw, kind: str, x: int, y: int, s: float = 1.0):
    if kind in {"book", "self_letter"}:
        simple_book(draw, x, y, int(210 * s), int(145 * s), "山茶")
        camellia(draw, x + 190 * s, y - 12 * s, 25 * s)
    elif kind in {"translate", "machine"}:
        rounded(draw, [x, y, x + 310 * s, y + 220 * s], radius=int(22 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        zh_text(draw, (x + 78 * s, y - 42 * s), "代写窗口", size=int(28 * s), fill=INK)
        note_lines(draw, x + 52 * s, y + 68 * s, rows=4, w=int(190 * s))
        line(draw, [(x + 40 * s, y + 160 * s), (x + 270 * s, y + 160 * s)], fill=MUTED, width=max(2, int(3 * s)))
    elif kind == "orbit":
        draw.ellipse([x, y, x + 330 * s, y + 210 * s], outline=INK, width=max(3, int(4 * s)))
        for ox, oy, text in [(40, 65, "家务"), (140, 18, "育儿"), (230, 120, "升学")]:
            note_box(draw, x + ox * s, y + oy * s, 86 * s, 48 * s, text)
    elif kind in {"desk", "lamp"}:
        rounded(draw, [x, y + 145 * s, x + 390 * s, y + 220 * s], radius=int(20 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        hand_note(draw, x + 100 * s, y + 40 * s, 155 * s, 92 * s, "真话")
        line(draw, [(x + 280 * s, y + 55 * s), (x + 230 * s, y + 145 * s), (x + 330 * s, y + 145 * s), (x + 280 * s, y + 55 * s)], fill=YELLOW, width=max(3, int(5 * s)))
    elif kind == "door":
        rounded(draw, [x, y, x + 310 * s, y + 300 * s], radius=int(12 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        line(draw, [(x + 155 * s, y), (x + 155 * s, y + 300 * s)], width=max(3, int(4 * s)))
        note_box(draw, x + 190 * s, y + 100 * s, 95 * s, 50 * s, "想听见")
    elif kind == "kettle":
        rounded(draw, [x + 80 * s, y + 80 * s, x + 270 * s, y + 250 * s], radius=int(34 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        draw.arc([x + 235 * s, y + 120 * s, x + 330 * s, y + 215 * s], -70, 80, fill=INK, width=max(3, int(4 * s)))
        for ox, text in [(10, "肉桂"), (120, "蜂蜜"), (245, "橘子")]:
            hand_note(draw, x + ox * s, y - 20 * s, 92 * s, 62 * s, text)
    elif kind in {"box", "ritual"}:
        rounded(draw, [x, y + 85 * s, x + 330 * s, y + 265 * s], radius=int(18 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        line(draw, [(x, y + 145 * s), (x + 330 * s, y + 145 * s)], width=max(3, int(4 * s)))
        for i in range(4):
            hand_note(draw, x + 360 * s, y + (30 + i * 62) * s, 90 * s, 54 * s, "")
    elif kind == "ferry":
        line(draw, [(x, y + 180 * s), (x + 560 * s, y + 180 * s)], width=max(3, int(5 * s)))
        line(draw, [(x, y + 245 * s), (x + 560 * s, y + 245 * s)], fill=TEAL, width=max(3, int(4 * s)))
        for i in range(5):
            line(draw, [(x + (60 + i * 110) * s, y + 125 * s), (x + (60 + i * 110) * s, y + 275 * s)], width=max(2, int(4 * s)))
            smooth_wave(draw, x + i * 110 * s, y + 320 * s, x + (110 + i * 110) * s, amplitude=7 * s, wavelength=55 * s, fill=TEAL, width=max(2, int(3 * s)), cycles=1)
    elif kind == "empty":
        rounded(draw, [x, y, x + 330 * s, y + 230 * s], radius=int(20 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        zh_text(draw, (x + 90 * s, y + 92 * s), "空", size=int(62 * s), fill=MUTED)
    elif kind == "table":
        rounded(draw, [x, y + 120 * s, x + 460 * s, y + 205 * s], radius=int(42 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        for ox, text in [(80, "妈妈"), (220, "孩子"), (360, "没说完")]:
            draw.ellipse([x + (ox - 38) * s, y + 135 * s, x + (ox + 38) * s, y + 185 * s], outline=INK, width=max(2, int(3 * s)))
            zh_text(draw, (x + (ox - 32) * s, y + 215 * s), text, size=int(24 * s), fill=MUTED)
    elif kind == "uniform":
        rounded(draw, [x + 80 * s, y + 20 * s, x + 250 * s, y + 250 * s], radius=int(22 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        line(draw, [(x + 120 * s, y + 75 * s), (x + 210 * s, y + 75 * s)], fill=TEAL, width=max(3, int(5 * s)))
        hand_note(draw, x + 290 * s, y + 70 * s, 120 * s, 75 * s, "未来")
    elif kind == "shop":
        rounded(draw, [x, y, x + 370 * s, y + 255 * s], radius=int(18 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        for i, text in enumerate(["纸张", "信封", "印章"]):
            note_box(draw, x + 35 * s, y + (35 + i * 70) * s, 115 * s, 48 * s, text)
        camellia(draw, x + 300 * s, y + 60 * s, 30 * s)
    elif kind == "mailbox":
        rounded(draw, [x, y + 80 * s, x + 360 * s, y + 250 * s], radius=int(40 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        for i, text in enumerate(["道歉", "感谢", "告别"]):
            hand_note(draw, x + (20 + i * 115) * s, y - 20 * s, 95 * s, 58 * s, text)
    elif kind == "seasons":
        for i, (text, color) in enumerate([("夏", RED), ("秋", YELLOW), ("冬", TEAL), ("明日", BLUE)]):
            camellia(draw, x + (40 + i * 105) * s, y + 100 * s, 25 * s)
            zh_text(draw, (x + (24 + i * 105) * s, y + 165 * s), text, size=int(28 * s), fill=color)
    elif kind == "mirror":
        rounded(draw, [x, y, x + 250 * s, y + 270 * s], radius=int(100 * s), fill="white", outline=INK, width=max(3, int(4 * s)))
        zh_text(draw, (x + 75 * s, y + 115 * s), "不够好", size=int(30 * s), fill=RED)
    elif kind == "bridge":
        smooth_wave_arrow(draw, x + 8 * s, y + 168 * s, x + 370 * s, BLUE, width=max(3, int(4 * s)), amplitude=10 * s)
        note_box(draw, x, y + 220 * s, 125 * s, 60 * s, "长辈")
        note_box(draw, x + 270 * s, y + 220 * s, 125 * s, 60 * s, "普通人")
    elif kind == "room":
        window(draw, x, y, 310 * s, 210 * s, rain=True)
        hand_note(draw, x + 370 * s, y + 60 * s, 130 * s, 82 * s, "慢下来")
    elif kind == "path":
        smooth_wave_arrow(draw, x + 6 * s, y + 178 * s, x + 520 * s, BLUE, width=max(3, int(4 * s)), amplitude=10 * s)
        note_box(draw, x + 10 * s, y + 245 * s, 115 * s, 54 * s, "过去")
        note_box(draw, x + 410 * s, y + 80 * s, 120 * s, 54 * s, "新生活")
    elif kind == "heart":
        draw.ellipse([x, y, x + 310 * s, y + 250 * s], outline=INK, width=max(3, int(4 * s)))
        hand_note(draw, x + 100 * s, y + 70 * s, 130 * s, 86 * s, "未说出口")
    elif kind == "echo":
        for r in (70, 130, 190):
            draw.arc([x - r * s, y - r * s, x + r * s, y + r * s], 215, 325, fill=BLUE, width=max(2, int(3 * s)))
        hand_note(draw, x + 230 * s, y - 40 * s, 140 * s, 84 * s, "回声")
    elif kind == "map":
        for i, text in enumerate(["文具店", "家人", "旧信", "修复"]):
            note_box(draw, x + i * 135 * s, y + (40 if i % 2 else 120) * s, 110 * s, 54 * s, text)
            if i:
                arrow(draw, (x + (i * 135 - 24) * s, y + (146 if i % 2 else 66) * s), (x + i * 135 * s, y + (148 if i % 2 else 68) * s), ORANGE)
    else:
        hand_note(draw, x, y, 180 * s, 110 * s, "继续")


def draw_prop(draw, prop: str, x: int, y: int, s: float = 1.0):
    label_text = PROP_LABELS.get(prop, prop)
    if prop in {"paper", "old_letters", "envelope"}:
        hand_note(draw, x, y, 115 * s, 72 * s, label_text)
    elif prop in {"book"}:
        simple_book(draw, x, y + 18 * s, 118 * s, 82 * s, label_text)
    elif prop in {"pen", "ink", "stamp"}:
        stationery_cups(draw, x, y + 28 * s, 0.42 * s)
        zh_text(draw, (x + 12 * s, y + 110 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"cup", "wine"}:
        tea(draw, x + 52 * s, y + 28 * s, 0.58 * s)
        zh_text(draw, (x + 12 * s, y + 104 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"window", "rain", "curtain"}:
        window(draw, x, y, 132 * s, 90 * s, rain=prop == "rain")
        zh_text(draw, (x + 32 * s, y + 104 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"plant", "camellia", "flower", "osmanthus", "leaf", "lotus"}:
        if prop == "camellia":
            camellia(draw, x + 54 * s, y + 42 * s, 26 * s)
        else:
            plant(draw, x + 58 * s, y - 8 * s, 0.32 * s)
        zh_text(draw, (x + 22 * s, y + 105 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"bag", "school", "uniform"}:
        school_bag(draw, x, y, 0.42 * s)
        zh_text(draw, (x + 30 * s, y + 120 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"box"}:
        rounded(draw, [x, y + 28 * s, x + 130 * s, y + 100 * s], radius=int(12 * s), fill="white", outline=INK, width=max(2, int(3 * s)))
        line(draw, [(x, y + 58 * s), (x + 130 * s, y + 58 * s)], width=max(2, int(3 * s)))
        zh_text(draw, (x + 34 * s, y + 108 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"ferry", "sea", "waves", "island", "port"}:
        for i in range(2):
            smooth_wave(draw, x + i * 52 * s, y + 68 * s, x + (44 + i * 52) * s, amplitude=4 * s, wavelength=44 * s, fill=TEAL, width=max(2, int(3 * s)), cycles=1)
        if prop in {"ferry", "port"}:
            rounded(draw, [x + 20 * s, y + 22 * s, x + 120 * s, y + 54 * s], radius=int(8 * s), fill="white", outline=INK, width=max(2, int(3 * s)))
        zh_text(draw, (x + 30 * s, y + 92 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"phone", "tv"}:
        rounded(draw, [x + 24 * s, y + 20 * s, x + 105 * s, y + 100 * s], radius=int(12 * s), fill="white", outline=INK, width=max(2, int(3 * s)))
        zh_text(draw, (x + 32 * s, y + 108 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"door", "old_house", "shop", "desk", "table", "kitchen", "chair"}:
        rounded(draw, [x, y + 10 * s, x + 140 * s, y + 94 * s], radius=int(10 * s), fill="white", outline=INK, width=max(2, int(3 * s)))
        line(draw, [(x + 70 * s, y + 10 * s), (x + 70 * s, y + 94 * s)], width=max(2, int(3 * s)))
        zh_text(draw, (x + 28 * s, y + 108 * s), label_text, size=int(22 * s), fill=MUTED)
    elif prop in {"spice", "honey", "orange", "apple", "bento", "bucket", "key"}:
        draw.ellipse([x + 28 * s, y + 22 * s, x + 92 * s, y + 86 * s], outline=INK, fill="white", width=max(2, int(3 * s)))
        camellia(draw, x + 92 * s, y + 30 * s, 14 * s)
        zh_text(draw, (x + 22 * s, y + 105 * s), label_text, size=int(22 * s), fill=MUTED)
    else:
        hand_note(draw, x, y, 115 * s, 72 * s, label_text)


def draw_prop_cluster(draw, props: list[str], x: int, y: int):
    positions = [(0, 0), (165, -12), (330, 4), (0, 138), (165, 126), (330, 142)]
    for prop, (dx, dy) in zip(props[:6], positions):
        draw_prop(draw, prop, x + dx, y + dy, 0.92)


def scene_decor(draw, kind: str, index: int):
    # A restrained illustration layer: weather, nature, and lived-in objects.
    palette = [TEAL, BLUE, ORANGE, RED, YELLOW]
    if kind in {"book", "shop", "self_letter"}:
        shelf(draw, 118, 260, 210, 360, rows=3)
        stationery_cups(draw, 1350, 620, 0.62)
        camellia_branch(draw, 1400, 360, 0.45)
    elif kind in {"translate", "machine"}:
        stationery_cups(draw, 145, 670, 0.55)
        hand_note(draw, 1370, 285, 125, 75, "委托")
        hand_note(draw, 1510, 380, 125, 75, "真心")
        line(draw, [(1260, 730), (1580, 730)], fill=(226, 220, 206), width=3)
    elif kind in {"orbit", "table"}:
        tableware(draw, 1355, 640, 0.72)
        school_bag(draw, 1460, 500, 0.5)
        plant(draw, 225, 670, 0.38)
    elif kind == "door":
        plant(draw, 1480, 610, 0.45)
        book_stack(draw, 1325, 730, 0.5)
        hand_note(draw, 1340, 300, 130, 78, "门缝")
    elif kind == "kettle":
        window(draw, 105, 240, 300, 210, rain=False)
        plant(draw, 1440, 600, 0.44)
        camellia(draw, 1285, 300, 32)
    elif kind in {"rest", "empty", "mirror"}:
        window(draw, 1080, 265, 310, 220, rain=False)
        tea(draw, 1450, 640, 0.75)
        floor_objects(draw, 180, 750, 0.55)
    elif kind in {"box", "ritual"}:
        hatching(draw, [1080, 260, 1550, 760], step=36)
        framed_photo(draw, 1345, 300, 0.62)
        camellia_branch(draw, 1260, 790, 0.42)
    elif kind == "ferry":
        distant_island(draw, 1180, 360, 0.85)
        for i in range(4):
            smooth_wave(draw, 1160 + i * 115, 824, 1280 + i * 115, amplitude=7, wavelength=60, fill=TEAL, width=3, cycles=1)
    elif kind == "lamp":
        window(draw, 118, 250, 320, 240, rain=True)
        tea(draw, 1390, 610, 0.72)
        camellia(draw, 1500, 705, 28)
    elif kind == "uniform":
        school_bag(draw, 1320, 560, 0.55)
        hand_note(draw, 1455, 350, 120, 78, "校服")
        plant(draw, 180, 690, 0.4)
    elif kind == "mailbox":
        plant(draw, 1380, 610, 0.45)
        camellia_branch(draw, 1250, 360, 0.46)
    elif kind == "seasons":
        for i, color in enumerate(palette):
            x = 1170 + i * 80
            y = 345 + (i % 2) * 65
            camellia(draw, x, y, 20)
            line(draw, [(x - 24, y + 40), (x + 24, y + 80)], fill=color, width=3)
    elif kind == "bridge":
        distant_island(draw, 1180, 360, 0.62)
        plant(draw, 250, 675, 0.4)
    elif kind == "room":
        tea(draw, 1350, 650, 0.7)
        book_stack(draw, 1450, 700, 0.52)
        plant(draw, 220, 675, 0.42)
    elif kind == "path":
        plant(draw, 1180, 620, 0.42)
        for i in range(5):
            smooth_wave(draw, 1300 + i * 70, 796, 1370 + i * 70, amplitude=5, wavelength=35, fill=TEAL, width=3, cycles=1)
    elif kind == "heart":
        camellia_branch(draw, 1240, 740, 0.48)
        tea(draw, 1380, 610, 0.72)
    elif kind == "echo":
        window(draw, 1170, 280, 300, 200, rain=False)
        plant(draw, 1450, 630, 0.42)
    elif kind == "map":
        plant(draw, 1280, 635, 0.42)
        camellia(draw, 1510, 420, 30)
    else:
        camellia(draw, 1450, 330, 26)


def render_full_scene(index: int, chunk: dict) -> Image.Image:
    title, notes, kind = pick_blueprint(index)
    props = merge_props(kind, chunk.get("text", ""))
    im = clean_page()
    d = draw_for(im)
    label(d, title, 720, 95, RED, 40, underline=True)
    scene_decor(d, kind, index)
    draw_prop_cluster(d, props, 1090, 230)
    xiaohei(d, 560, 555, 1.35, pose=("write" if kind in {"desk", "lamp", "self_letter"} else "pull" if kind in {"book", "box"} else "stand"), eyes=("tired" if kind in {"empty", "lamp"} else "wide"))
    concept_object(d, kind, 920, 360, 1.0)
    arrow(d, (680, 545), (900, 500), ORANGE, width=4)

    y = 275
    for note in notes[:4]:
        hand_note(d, 250, y, 165, 88, note)
        arrow(d, (405, y + 40), (490, 500), ORANGE, width=3)
        y += 118

    caption = scene_caption(kind, title, props)
    excerpt_box(d, 1160, 650, 540, 168, caption)
    smooth_wave_arrow(d, 1048, 612, 1192, BLUE, width=3, amplitude=6)
    camellia(d, 1550, 270, 34)
    return im


def hatching(draw: ImageDraw.ImageDraw, box, step=28, color=(222, 218, 206), width=2):
    x1, y1, x2, y2 = box
    for x in range(int(x1), int(x2 + y2 - y1), step):
        line(draw, [(x, y2), (x - (y2 - y1), y1)], fill=color, width=width)


def floor_grid(draw: ImageDraw.ImageDraw, y=760, color=(222, 218, 206)):
    line(draw, [(130, y), (W - 130, y)], fill=color, width=4)
    for x in range(230, W - 130, 190):
        line(draw, [(x, y), (x - 120, H - 130)], fill=color, width=2)


def window(draw: ImageDraw.ImageDraw, x, y, w, h, rain=False):
    rounded(draw, [x, y, x + w, y + h], radius=12, fill=(252, 251, 245), width=4)
    line(draw, [(x + w / 2, y), (x + w / 2, y + h)], fill=MUTED, width=3)
    line(draw, [(x, y + h / 2), (x + w, y + h / 2)], fill=MUTED, width=3)
    line(draw, [(x - 36, y - 10), (x - 36, y + h + 12)], fill=(219, 210, 190), width=5)
    line(draw, [(x + w + 36, y - 10), (x + w + 36, y + h + 12)], fill=(219, 210, 190), width=5)
    for yy in range(int(y + 36), int(y + h), 46):
        line(draw, [(x - 34, yy), (x - 14, yy + 16)], fill=(219, 210, 190), width=3)
        line(draw, [(x + w + 34, yy), (x + w + 14, yy + 16)], fill=(219, 210, 190), width=3)
    if rain:
        for i in range(9):
            xx = x + 30 + i * (w - 60) / 8
            line(draw, [(xx, y + 30), (xx + 18, y + h - 38)], fill=TEAL, width=2)


def shelf(draw: ImageDraw.ImageDraw, x, y, w, h, rows=3):
    rounded(draw, [x, y, x + w, y + h], radius=18, fill=(245, 239, 225), width=4)
    for r in range(1, rows):
        yy = y + h * r / rows
        line(draw, [(x + 18, yy), (x + w - 18, yy)], fill=MUTED, width=3)
    for r in range(rows):
        yy = y + 24 + r * h / rows
        for i in range(4):
            bx = x + 32 + i * (w - 80) / 4
            rounded(draw, [bx, yy, bx + 38, yy + 70], radius=4, fill=(255, 254, 249), width=2)
        if r % 2 == 0:
            postcard_stack(draw, x + w - 125, yy + 28, 0.45)
        else:
            camellia(draw, x + w - 82, yy + 50, 18)


def plant(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    rounded(draw, [x - 30 * s, y + 70 * s, x + 30 * s, y + 130 * s], radius=int(12 * s), fill=(220, 205, 178), width=max(2, int(3 * s)))
    for a in (-70, -38, -12, 18, 48, 75):
        dx = math.cos(math.radians(a)) * 85 * s
        dy = math.sin(math.radians(a)) * 70 * s
        line(draw, [(x, y + 80 * s), (x + dx, y + dy)], fill=TEAL, width=max(2, int(4 * s)))
        draw.ellipse([x + dx - 20 * s, y + dy - 12 * s, x + dx + 20 * s, y + dy + 12 * s], fill=(122, 153, 137), outline=INK, width=max(1, int(2 * s)))


def small_sparkles(draw: ImageDraw.ImageDraw, points):
    for x, y in points:
        line(draw, [(x - 10, y), (x + 10, y)], fill=YELLOW, width=3)
        line(draw, [(x, y - 10), (x, y + 10)], fill=YELLOW, width=3)


def wall_frames(draw: ImageDraw.ImageDraw, items):
    for x, y, w, h in items:
        rounded(draw, [x, y, x + w, y + h], radius=12, fill=(252, 250, 243), outline=(218, 212, 198), width=3)
        line(draw, [(x + 18, y + h - 26), (x + w * 0.42, y + h * 0.55), (x + w * 0.66, y + h - 34), (x + w - 18, y + h * 0.48)], fill=TEAL, width=3)
        draw.ellipse([x + w - 38, y + 18, x + w - 18, y + 38], fill=YELLOW, outline=None)


def floor_objects(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    rounded(draw, [x, y, x + 140 * s, y + 78 * s], radius=int(18 * s), fill=(236, 229, 214), width=max(2, int(4 * s)))
    line(draw, [(x + 28 * s, y + 30 * s), (x + 112 * s, y + 30 * s)], fill=MUTED, width=max(2, int(3 * s)))
    plant(draw, x + 210 * s, y - 95 * s, 0.36 * s)


def book_stack(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    colors = [(238, 230, 210), (225, 216, 196), (245, 241, 230)]
    for i, c in enumerate(colors):
        rounded(draw, [x + i * 18 * s, y - i * 22 * s, x + (180 + i * 18) * s, y + (34 - i * 22) * s], radius=int(8 * s), fill=c, width=max(2, int(3 * s)))


def postcard_stack(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    for i, c in enumerate([(247, 244, 235), (237, 231, 216), (255, 253, 247)]):
        ox = i * 16 * s
        oy = -i * 12 * s
        rounded(draw, [x + ox, y + oy, x + ox + 150 * s, y + oy + 88 * s], radius=int(8 * s), fill=c, outline=(150, 150, 143), width=max(1, int(2 * s)))
        line(draw, [(x + ox + 18 * s, y + oy + 26 * s), (x + ox + 96 * s, y + oy + 26 * s)], fill=(180, 178, 168), width=max(1, int(2 * s)))
        line(draw, [(x + ox + 18 * s, y + oy + 50 * s), (x + ox + 122 * s, y + oy + 50 * s)], fill=(180, 178, 168), width=max(1, int(2 * s)))


def paper_sheet(draw: ImageDraw.ImageDraw, x, y, w=210, h=135, angle=0, quiet=True):
    outline = (150, 150, 143) if quiet else MUTED
    rounded(draw, [x, y, x + w, y + h], radius=10, fill=(255, 254, 249), outline=outline, width=2)
    for i in range(3):
        yy = y + 34 + i * 28
        line(draw, [(x + 26, yy), (x + w - 34, yy)], fill=(196, 192, 180), width=2)


def rice_bowl(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    draw.ellipse([x - 64 * s, y - 20 * s, x + 64 * s, y + 28 * s], fill=(255, 254, 249), outline=INK, width=max(3, int(5 * s)))
    rounded(draw, [x - 56 * s, y - 2 * s, x + 56 * s, y + 58 * s], radius=int(24 * s), fill=(246, 241, 228), width=max(3, int(5 * s)))
    for dx in (-22, 0, 22):
        draw.ellipse([x + dx * s - 8 * s, y - 24 * s, x + dx * s + 8 * s, y - 8 * s], fill=(255, 255, 250), outline=None)


def school_bag(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    rounded(draw, [x, y, x + 180 * s, y + 210 * s], radius=int(26 * s), fill=(224, 218, 204), width=max(3, int(5 * s)))
    rounded(draw, [x + 30 * s, y - 42 * s, x + 150 * s, y + 44 * s], radius=int(40 * s), fill=None, width=max(3, int(5 * s)))
    line(draw, [(x + 32 * s, y + 72 * s), (x + 148 * s, y + 72 * s)], fill=MUTED, width=max(2, int(4 * s)))
    rounded(draw, [x + 48 * s, y + 112 * s, x + 132 * s, y + 168 * s], radius=int(12 * s), fill=(244, 240, 226), width=max(2, int(3 * s)))


def stationery_cups(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    rounded(draw, [x, y, x + 82 * s, y + 92 * s], radius=int(18 * s), fill=(236, 229, 214), width=max(2, int(4 * s)))
    for i, color in enumerate([INK, TEAL, RED, MUTED]):
        xx = x + (18 + i * 16) * s
        line(draw, [(xx, y + 8 * s), (xx + 22 * s, y - 78 * s)], fill=color, width=max(2, int(4 * s)))
    rounded(draw, [x + 110 * s, y + 18 * s, x + 220 * s, y + 72 * s], radius=int(10 * s), fill=(255, 254, 249), outline=(150, 150, 143), width=max(1, int(2 * s)))
    for i in range(3):
        line(draw, [(x + 126 * s, y + (34 + i * 14) * s), (x + 198 * s, y + (34 + i * 14) * s)], fill=(190, 186, 174), width=max(1, int(2 * s)))


def shop_counter(draw: ImageDraw.ImageDraw, x, y, w, h):
    rounded(draw, [x, y, x + w, y + h], radius=24, fill=(232, 222, 202), width=5)
    line(draw, [(x + 28, y + 58), (x + w - 28, y + 58)], fill=(180, 165, 138), width=4)
    for i in range(4):
        rounded(draw, [x + 34 + i * 125, y + 92, x + 112 + i * 125, y + 154], radius=10, fill=(247, 243, 232), outline=(174, 163, 145), width=2)


def tableware(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    draw.ellipse([x - 68 * s, y - 28 * s, x + 68 * s, y + 36 * s], fill=(255, 254, 249), outline=INK, width=max(3, int(5 * s)))
    line(draw, [(x - 95 * s, y - 30 * s), (x - 22 * s, y + 24 * s)], fill=MUTED, width=max(2, int(4 * s)))
    line(draw, [(x + 24 * s, y + 24 * s), (x + 104 * s, y - 26 * s)], fill=MUTED, width=max(2, int(4 * s)))


def distant_island(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    line(draw, [(x, y), (x + 140 * s, y - 40 * s), (x + 290 * s, y), (x, y)], fill=(206, 222, 218), width=max(3, int(5 * s)))
    draw.arc([x + 72 * s, y - 145 * s, x + 190 * s, y - 28 * s], 205, 338, fill=YELLOW, width=max(4, int(8 * s)))


def framed_photo(draw: ImageDraw.ImageDraw, x, y, s=1.0):
    rounded(draw, [x, y, x + 170 * s, y + 130 * s], radius=int(14 * s), fill=(248, 246, 239), width=max(3, int(4 * s)))
    line(draw, [(x + 28 * s, y + 90 * s), (x + 70 * s, y + 48 * s), (x + 112 * s, y + 88 * s), (x + 142 * s, y + 56 * s)], fill=TEAL, width=max(2, int(4 * s)))
    camellia(draw, x + 126 * s, y + 34 * s, 14 * s)


def table(draw: ImageDraw.ImageDraw, x, y, w, h):
    rounded(draw, [x, y, x + w, y + h], radius=28, fill=(238, 230, 210), width=5)
    line(draw, [(x + 70, y + h), (x + 44, y + h + 120)], fill=INK, width=5)
    line(draw, [(x + w - 70, y + h), (x + w - 44, y + h + 120)], fill=INK, width=5)


def character(
    draw: ImageDraw.ImageDraw,
    x: int,
    y: int,
    s: float = 1.0,
    pose: str = "stand",
    direction: str = "front",
):
    """Draw the fixed adult letter-writer IP with 8-direction views."""
    direction = direction.replace("-", "_")
    is_back = direction.startswith("back")
    is_left = direction.endswith("left") or direction == "left"
    is_right = direction.endswith("right") or direction == "right"
    is_side = direction in {"left", "right"}
    is_front = direction == "front"
    is_diagonal_front = direction.startswith("front_")
    is_diagonal_back = direction.startswith("back_")
    turn = -1 if is_left else (1 if is_right else 0)
    profile = 1.0 if is_side else (0.58 if (is_diagonal_front or is_diagonal_back) else 0.0)
    body_shift = turn * (18 if is_side else 10 if (is_diagonal_front or is_diagonal_back) else 0) * s
    face_shift = turn * (22 if is_side else 12 if (is_diagonal_front or is_diagonal_back) else 0) * s

    draw.ellipse([x - 72 * s, y + 184 * s, x + 72 * s, y + 214 * s], fill=(224, 220, 208))

    skirt_left = -70 + turn * 10 * profile
    skirt_right = 70 + turn * 20 * profile
    draw.polygon(
        [
            (x + body_shift + skirt_left * s, y + 66 * s),
            (x + body_shift + skirt_right * s, y + 66 * s),
            (x + (skirt_right + 18) * s, y + 188 * s),
            (x + (skirt_left - 18) * s, y + 188 * s),
        ],
        fill=TEAL,
        outline=INK,
    )
    line(draw, [(x + body_shift + turn * 10 * s, y + 80 * s), (x + turn * 18 * s, y + 184 * s)], width=max(3, int(4 * s)))

    torso_left = -58 + turn * 12 * profile
    torso_right = 58 + turn * 18 * profile
    rounded(draw, [x + body_shift + torso_left * s, y - 18 * s, x + body_shift + torso_right * s, y + 82 * s], radius=int(26 * s), fill=(241, 236, 222), width=max(3, int(5 * s)))

    if is_back:
        strap_offset = turn * 10 * profile * s
        line(draw, [(x - 36 * s + strap_offset, y + 8 * s), (x - 50 * s + strap_offset, y + 150 * s)], fill=BEIGE, width=max(3, int(6 * s)))
        line(draw, [(x + 36 * s + strap_offset, y + 8 * s), (x + 50 * s + strap_offset, y + 150 * s)], fill=BEIGE, width=max(3, int(6 * s)))
        rounded(draw, [x - 28 * s + body_shift, y + 62 * s, x + 30 * s + body_shift, y + 110 * s], radius=int(10 * s), fill=BEIGE, width=max(2, int(3 * s)))
        line(draw, [(x - 22 * s + body_shift, y + 84 * s), (x + 22 * s + body_shift, y + 84 * s)], fill=(160, 150, 130), width=max(1, int(2 * s)))
    else:
        apron_shift = body_shift + turn * 4 * s
        draw.polygon(
            [
                (x - 34 * s + apron_shift, y + 20 * s),
                (x + 34 * s + apron_shift, y + 20 * s),
                (x + 44 * s + apron_shift, y + 150 * s),
                (x - 44 * s + apron_shift, y + 150 * s),
            ],
            fill=BEIGE,
            outline=INK,
        )
        rounded(draw, [x - 20 * s + apron_shift, y + 76 * s, x + 20 * s + apron_shift, y + 116 * s], radius=int(8 * s), fill=(255, 254, 249), width=max(1, int(2 * s)))
        if is_front or is_diagonal_front:
            line(draw, [(x - 15 * s + apron_shift, y + 96 * s), (x + 15 * s + apron_shift, y + 96 * s)], fill=(170, 162, 146), width=max(1, int(2 * s)))

    if pose == "write":
        line(draw, [(x + body_shift - 48 * s, y + 28 * s), (x - 92 * s + turn * 26 * s, y + 92 * s)], width=max(4, int(7 * s)))
        line(draw, [(x + body_shift + 48 * s, y + 28 * s), (x + 96 * s + turn * 26 * s, y + 92 * s)], width=max(4, int(7 * s)))
    elif pose == "hold":
        line(draw, [(x + body_shift - 48 * s, y + 28 * s), (x - 26 * s + turn * 20 * s, y + 104 * s)], width=max(4, int(7 * s)))
        line(draw, [(x + body_shift + 48 * s, y + 28 * s), (x + 26 * s + turn * 20 * s, y + 104 * s)], width=max(4, int(7 * s)))
        envelope(draw, x - 38 * s + turn * 18 * s, y + 86 * s, 76 * s, 42 * s)
    else:
        line(draw, [(x + body_shift - 50 * s, y + 26 * s), (x - 84 * s + turn * 28 * s, y + 110 * s)], width=max(4, int(7 * s)))
        line(draw, [(x + body_shift + 50 * s, y + 26 * s), (x + 84 * s + turn * 28 * s, y + 110 * s)], width=max(4, int(7 * s)))

    head_x = x + face_shift
    if is_side:
        draw.ellipse([head_x - 44 * s, y - 116 * s, head_x + 46 * s, y - 18 * s], fill=(26, 28, 29))
        draw.pieslice([head_x - 56 * s, y - 118 * s, head_x + 42 * s, y + 8 * s], 85 if is_right else 95, 280 if is_right else 275, fill=INK)
        nose = head_x + (48 if is_right else -48) * s
        draw.polygon(
            [(nose, y - 66 * s), (head_x + turn * 30 * s, y - 56 * s), (head_x + turn * 30 * s, y - 76 * s)],
            fill=INK,
        )
        line(draw, [(head_x + turn * 12 * s, y - 42 * s), (head_x + turn * 32 * s, y - 38 * s)], fill=(72, 74, 74), width=max(2, int(3 * s)))
    else:
        head_w = 54 * s if (is_front or direction == "back") else 50 * s
        draw.ellipse([head_x - head_w, y - 122 * s, head_x + head_w, y - 14 * s], fill=INK)
        draw.pieslice([head_x - 62 * s, y - 118 * s, head_x + 62 * s, y + 20 * s], 0, 180, fill=INK)
        if is_back:
            draw.ellipse([head_x - 48 * s, y - 110 * s, head_x + 48 * s, y - 18 * s], fill=(24, 26, 27))
            line(draw, [(head_x, y - 104 * s), (head_x, y - 22 * s)], fill=(48, 50, 50), width=max(2, int(3 * s)))
        else:
            draw.ellipse([head_x - 44 * s, y - 104 * s, head_x + 44 * s, y - 18 * s], fill=(32, 35, 36))
        if not is_back:
            draw.arc([head_x - 28 * s + turn * 6 * s, y - 64 * s, head_x + 28 * s + turn * 6 * s, y - 28 * s], 20, 160, fill=(72, 74, 74), width=max(2, int(3 * s)))
            line(draw, [(head_x - 36 * s, y - 86 * s), (head_x + 34 * s, y - 86 * s)], fill=(20, 22, 23), width=max(2, int(4 * s)))

    if not is_back:
        pin_x = x + (44 if turn >= 0 else -44) * s
        camellia(draw, pin_x, y + 0 * s, 14 * s)


def envelope(draw, x, y, w, h, accent=False, quiet=True):
    fill = (255, 254, 249)
    outline = (86, 88, 86) if not quiet else (128, 132, 128)
    width = max(1, int(w / (95 if quiet else 80)))
    rounded(draw, [x, y, x + w, y + h], radius=int(min(w, h) * 0.08), outline=outline, fill=fill, width=width)
    line(draw, [(x, y), (x + w / 2, y + h * 0.58), (x + w, y)], fill=outline, width=width)
    line(draw, [(x, y + h), (x + w * 0.40, y + h * 0.45)], fill=outline, width=width)
    line(draw, [(x + w, y + h), (x + w * 0.60, y + h * 0.45)], fill=outline, width=width)
    if accent:
        r = min(w, h) * (0.055 if quiet else 0.075)
        cx, cy = x + w * 0.78, y + h * 0.58
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=RED)


def camellia(draw, x, y, r):
    for a in range(0, 360, 60):
        dx = math.cos(math.radians(a)) * r * 0.55
        dy = math.sin(math.radians(a)) * r * 0.55
        draw.ellipse([x + dx - r * 0.45, y + dy - r * 0.32, x + dx + r * 0.45, y + dy + r * 0.32], fill=RED, outline=INK, width=max(1, int(r / 8)))
    draw.ellipse([x - r * 0.22, y - r * 0.22, x + r * 0.22, y + r * 0.22], fill=YELLOW, outline=INK)


def pen(draw, x, y, length=170):
    line(draw, [(x, y), (x + length, y - length * 0.24)], width=7)
    draw.polygon([(x + length, y - length * 0.24), (x + length + 24, y - length * 0.30), (x + length + 6, y - length * 0.12)], fill=INK)


def tea(draw, x, y, s=1.0):
    rounded(draw, [x - 50 * s, y, x + 50 * s, y + 58 * s], radius=int(20 * s), fill=(255, 255, 250), width=max(3, int(5 * s)))
    draw.arc([x + 36 * s, y + 12 * s, x + 88 * s, y + 54 * s], -75, 75, fill=INK, width=max(3, int(5 * s)))
    for dx in (-22, 0, 22):
        line(draw, [(x + dx * s, y - 32 * s), (x + (dx + 10) * s, y - 62 * s)], fill=MUTED, width=max(2, int(4 * s)))


def scene_frame(draw, title: str, index: int, show_text: bool = False):
    rounded(draw, [86, 76, W - 86, H - 76], radius=42, outline=(225, 220, 208), width=4, fill=(255, 254, 249))
    if show_text:
        draw_text(draw, (130, 104), f"{index:02d}", size=42, fill=RED)
        draw_text(draw, (205, 111), title, size=34, fill=INK)


def note_lines(draw: ImageDraw.ImageDraw, x, y, rows=4, w=190):
    for i in range(rows):
        yy = y + i * 24
        line(draw, [(x, yy), (x + w - i * 18, yy)], fill=(150, 150, 150), width=2)


def clean_page() -> Image.Image:
    return Image.new("RGB", (W * RENDER_SCALE, H * RENDER_SCALE), "white")


def render_scene(scene: Scene, index: int) -> Image.Image:
    im = canvas()
    d = ImageDraw.Draw(im)
    scene_frame(d, scene.title, index)

    if scene.layout == "shop":
        floor_grid(d, 790)
        wall_frames(d, [(650, 150, 170, 125), (1120, 170, 190, 140), (1385, 160, 150, 110)])
        shelf(d, 180, 250, 420, 500)
        rounded(d, [705, 245, 1035, 745], radius=22, fill=(250, 246, 235))
        line(d, [(870, 245), (870, 745)], width=4)
        rounded(d, [820, 510, 920, 610], radius=50, fill=(255, 254, 249), width=4)
        shop_counter(d, 690, 650, 630, 150)
        stationery_cups(d, 744, 590, 0.78)
        plant(d, 1290, 530, 0.8)
        postcard_stack(d, 1110, 435, 0.7)
        book_stack(d, 1280, 492, 0.62)
        framed_photo(d, 1385, 320, 0.72)
        envelope(d, 1140, 338, 76, 46, accent=True)
        camellia(d, 650, 720, 46)
        floor_objects(d, 150, 785, 0.72)
        character(d, 1430, 690, 1.02, pose="hold", direction="front_left")
        small_sparkles(d, [(1100, 270), (1240, 320), (1500, 410)])
    elif scene.layout == "desk":
        floor_grid(d, 820)
        wall_frames(d, [(760, 225, 150, 110), (960, 250, 120, 90)])
        window(d, 1180, 230, 360, 260)
        shelf(d, 200, 290, 260, 360, rows=3)
        table(d, 520, 640, 850, 130)
        paper_sheet(d, 820, 536, 260, 150, quiet=True)
        envelope(d, 1088, 610, 92, 54, accent=True)
        pen(d, 1110, 622)
        tea(d, 1260, 525, 1.15)
        stationery_cups(d, 700, 555, 0.55)
        camellia(d, 595, 555, 44)
        book_stack(d, 620, 620, 0.8)
        plant(d, 1500, 625, 0.44)
        character(d, 430, 650, 0.92, pose="write", direction="right")
    elif scene.layout == "kitchen":
        floor_grid(d, 800)
        window(d, 220, 250, 330, 230)
        wall_frames(d, [(610, 260, 160, 110), (812, 240, 130, 95)])
        rounded(d, [1120, 250, 1520, 520], radius=22, fill=(245, 239, 225), width=4)
        for x in (1180, 1300, 1420):
            line(d, [(x, 270), (x, 500)], fill=MUTED, width=3)
        for y in (330, 430):
            line(d, [(1145, y), (1490, y)], fill=(220, 212, 196), width=3)
        table(d, 530, 585, 740, 150)
        rice_bowl(d, 690, 545, 1.05)
        rice_bowl(d, 960, 545, 1.05)
        tableware(d, 1115, 570, 0.64)
        paper_sheet(d, 820, 630, 160, 86, quiet=True)
        school_bag(d, 1300, 560, 0.92)
        framed_photo(d, 1160, 322, 0.72)
        floor_objects(d, 174, 798, 0.62)
        character(d, 355, 680, 0.9, pose="stand", direction="front_right")
    elif scene.layout == "doorway":
        floor_grid(d, 810)
        wall_frames(d, [(260, 290, 155, 120), (1450, 270, 150, 110)])
        rounded(d, [560, 230, 1360, 760], radius=18, fill=(250, 248, 240))
        line(d, [(960, 230), (960, 760)], width=5)
        for x in range(620, 1320, 120):
            line(d, [(x, 230), (x, 760)], fill=(229, 225, 214), width=2)
        rounded(d, [700, 665, 875, 760], radius=22, fill=(237, 232, 222))
        rounded(d, [1055, 665, 1230, 760], radius=22, fill=(237, 232, 222))
        rounded(d, [330, 740, 500, 800], radius=18, fill=(232, 226, 215))
        line(d, [(335, 790), (492, 745)], fill=MUTED, width=3)
        paper_sheet(d, 394, 682, 84, 52, quiet=True)
        tea(d, 1255, 620, 0.72)
        plant(d, 1450, 610, 0.65)
        book_stack(d, 1180, 642, 0.48)
        character(d, 500, 680, 0.88, pose="stand", direction="front_right")
    elif scene.layout == "box":
        floor_grid(d, 820)
        wall_frames(d, [(250, 270, 160, 120), (1450, 250, 150, 115)])
        hatching(d, [500, 360, 1390, 780], step=36)
        rounded(d, [640, 500, 1080, 780], radius=28, fill=(204, 168, 114))
        line(d, [(640, 588), (1080, 588)], width=5)
        line(d, [(710, 500), (710, 780)], fill=(160, 127, 83), width=4)
        rounded(d, [745, 410, 1015, 520], radius=22, fill=(226, 208, 170), width=4)
        line(d, [(770, 460), (990, 460)], fill=(160, 127, 83), width=3)
        for i in range(5):
            paper_sheet(d, 1135 + (i % 2) * 150, 430 + i * 56, 128, 74, quiet=True)
        envelope(d, 1282, 586, 96, 56, accent=True)
        camellia(d, 500, 710, 52)
        framed_photo(d, 530, 390, 0.82)
        book_stack(d, 1110, 742, 0.62)
        small_sparkles(d, [(610, 420), (1120, 435), (1360, 540)])
        character(d, 400, 690, 0.9, pose="hold", direction="front_right")
    elif scene.layout == "ferry":
        distant_island(d, 1220, 410, 0.95)
        for i in range(6):
            smooth_wave(d, 260 + i * 220, 840, 440 + i * 220, amplitude=10, wavelength=90, fill=TEAL, width=4, cycles=1)
            smooth_wave(d, 270 + i * 220, 895, 434 + i * 220, amplitude=8, wavelength=82, fill=(160, 184, 184), width=3, cycles=1)
        line(d, [(240, 650), (1610, 650)], width=8)
        for x in range(330, 1590, 170):
            line(d, [(x, 500), (x, 790)], width=5)
        line(d, [(240, 780), (1610, 780)], fill=TEAL, width=6)
        rounded(d, [1040, 510, 1270, 655], radius=22, fill=(211, 188, 150))
        line(d, [(1060, 556), (1248, 556)], fill=(160, 127, 83), width=3)
        rounded(d, [1320, 420, 1540, 560], radius=30, fill=(248, 246, 239), width=5)
        line(d, [(1320, 560), (1540, 420)], fill=TEAL, width=4)
        plant(d, 450, 590, 0.42)
        envelope(d, 1088, 552, 80, 48, accent=True)
        book_stack(d, 520, 675, 0.5)
        character(d, 720, 650, 0.92, pose="stand", direction="back_right")
    elif scene.layout == "night":
        floor_grid(d, 820)
        wall_frames(d, [(1120, 280, 145, 105), (1320, 260, 160, 120)])
        table(d, 450, 615, 890, 135)
        paper_sheet(d, 830, 540, 230, 138, quiet=True)
        envelope(d, 1074, 612, 86, 52)
        tea(d, 1180, 535, 1.1)
        window(d, 460, 280, 350, 260, rain=True)
        line(d, [(930, 350), (845, 535), (1015, 535), (930, 350)], fill=YELLOW, width=6)
        rounded(d, [870, 535, 990, 565], radius=12, fill=YELLOW, width=4)
        stationery_cups(d, 610, 560, 0.56)
        character(d, 360, 675, 0.88, pose="write", direction="right")
        camellia(d, 1390, 690, 38)
        book_stack(d, 1320, 630, 0.7)
        floor_objects(d, 1465, 790, 0.54)
    elif scene.layout == "sea":
        distant_island(d, 340, 480, 0.92)
        for i in range(6):
            smooth_wave(d, 330 + i * 190, 812, 510 + i * 190, amplitude=10, wavelength=90, fill=TEAL, width=4, cycles=1)
            smooth_wave(d, 350 + i * 190, 872, 490 + i * 190, amplitude=8, wavelength=70, fill=(160, 184, 184), width=3, cycles=1)
        line(d, [(250, 680), (1630, 680)], width=8)
        line(d, [(250, 790), (1630, 790)], fill=TEAL, width=5)
        for x in range(330, 1620, 190):
            line(d, [(x, 600), (x, 830)], width=5)
        d.arc([1180, 330, 1500, 650], 200, 340, fill=YELLOW, width=10)
        paper_sheet(d, 460, 620, 128, 74, quiet=True)
        book_stack(d, 585, 702, 0.48)
        camellia(d, 720, 620, 48)
        plant(d, 1500, 610, 0.55)
        plant(d, 250, 645, 0.42)
        character(d, 1050, 675, 0.96, pose="stand", direction="back_left")

    return im


def render_infographic_scene(index: int) -> Image.Image:
    im = clean_page()
    d = draw_for(im)

    if index == 1:
        label(d, "把书拆成一封信", 760, 105, RED, 42, underline=True)
        simple_book(d, 190, 425, 220, 150, "山茶")
        note_box(d, 510, 430, 160, 70, "人物")
        note_box(d, 510, 535, 160, 70, "心事")
        note_box(d, 510, 640, 160, 70, "关系")
        xiaohei(d, 875, 560, 1.38, pose="pull", eyes="wide")
        rounded(d, [1060, 420, 1370, 700], radius=18, fill="white", outline=INK, width=4)
        zh_text(d, (1120, 490), "8000字", 42, INK)
        zh_text(d, (1128, 560), "讲书稿", 42, INK)
        arrow(d, (420, 500), (500, 468), ORANGE)
        arrow(d, (680, 565), (780, 560), ORANGE)
        arrow(d, (970, 560), (1060, 560), ORANGE)
        label(d, "不是复述", 365, 325, BLUE, 30)
        label(d, "先抓情绪线", 1000, 330, RED, 32)
        hand_note(d, 1450, 465, 140, 96, "旁白")
        smooth_wave_arrow(d, 1375, 548, 1450, BLUE, width=4, amplitude=4)
    elif index == 2:
        label(d, "代写店像一台翻译机", 700, 110, RED, 42, underline=True)
        for y, t in [(330, "说不出口"), (480, "不好意思"), (630, "怕伤人")]:
            hand_note(d, 180, y, 150, 86, t)
            arrow(d, (345, y + 43), (610, 510), ORANGE)
        rounded(d, [610, 300, 1100, 725], radius=28, fill="white", outline=INK, width=5)
        zh_text(d, (750, 270), "代写窗口", 34, INK)
        xiaohei(d, 855, 545, 1.48, pose="write", eyes="calm")
        note_lines(d, 946, 485, rows=5, w=110)
        for y, t in [(360, "能寄出"), (505, "能被懂"), (650, "不刺痛")]:
            hand_note(d, 1300, y, 160, 86, t)
            arrow(d, (1105, 510), (1300, y + 43), ORANGE)
        label(d, "把沉默翻成纸", 690, 785, BLUE, 34)
    elif index == 3:
        label(d, "饭桌上的距离", 800, 115, RED, 42, underline=True)
        rounded(d, [410, 455, 1510, 610], radius=60, fill="white", outline=INK, width=5)
        for x, t in [(590, "妈妈"), (970, "孩子"), (1320, "没说完")]:
            d.ellipse([x - 58, 480, x + 58, 560], fill="white", outline=INK, width=4)
            zh_text(d, (x - 42, 625), t, 30, MUTED)
        xiaohei(d, 775, 420, 1.15, pose="carry", eyes="tired")
        hand_note(d, 755, 260, 150, 90, "便当")
        smooth_wave_arrow(d, 700, 722, 1250, BLUE, width=4, amplitude=10)
        label(d, "话在碗边绕圈", 815, 790, BLUE, 32)
        label(d, "不是不爱", 530, 330, RED, 32)
        label(d, "是不知道怎么开口", 1180, 330, RED, 32)
    elif index == 4:
        label(d, "门只开了一半", 780, 115, RED, 42, underline=True)
        rounded(d, [610, 220, 1280, 780], radius=16, fill="white", outline=INK, width=5)
        line(d, [(945, 220), (945, 780)], width=5)
        line(d, [(610, 780), (1280, 780)], width=5)
        xiaohei(d, 520, 620, 1.32, pose="push", eyes="wide")
        hand_note(d, 410, 385, 130, 86, "想进去")
        note_box(d, 1015, 445, 160, 72, "想听见")
        note_box(d, 1015, 560, 160, 72, "又害怕")
        arrow(d, (610, 610), (830, 610), ORANGE)
        smooth_wave_arrow(d, 840, 842, 1100, BLUE, width=4, amplitude=8)
        label(d, "关系的缝", 915, 160, BLUE, 32)
        label(d, "先停一下", 1220, 820, RED, 30)
    elif index == 5:
        label(d, "旧信不是回忆，是证据", 685, 110, RED, 42, underline=True)
        rounded(d, [640, 425, 1110, 705], radius=26, fill="white", outline=INK, width=5)
        line(d, [(640, 520), (1110, 520)], width=5)
        xiaohei(d, 520, 610, 1.22, pose="pull", eyes="wide")
        for i in range(5):
            hand_note(d, 1145 + (i % 2) * 130, 330 + i * 90, 118, 76, "")
        arrow(d, (600, 560), (700, 535), ORANGE)
        for pt in [(1110, 500), (1145, 430), (1145, 610)]:
            arrow(d, pt, (pt[0] + 80, pt[1] - 20), ORANGE)
        label(d, "翻出来", 445, 405, BLUE, 30)
        label(d, "原来早就写过", 1190, 760, RED, 32)
        camellia(d, 840, 385, 36)
        simple_book(d, 245, 420, 150, 110, "过去")
    elif index == 6:
        label(d, "渡轮把人带回原点", 695, 105, RED, 42, underline=True)
        for i in range(5):
            smooth_wave(d, 260 + i * 270, 766, 500 + i * 270, amplitude=12, wavelength=120, fill=INK, width=4, cycles=1)
        line(d, [(260, 620), (1600, 620)], width=5)
        line(d, [(260, 710), (1600, 710)], width=4)
        for x in range(340, 1540, 170):
            line(d, [(x, 545), (x, 760)], width=4)
        xiaohei(d, 760, 545, 1.2, pose="stand", eyes="calm")
        hand_note(d, 1040, 490, 150, 90, "信盒")
        arrow(d, (850, 535), (1040, 525), ORANGE)
        smooth_wave_arrow(d, 400, 392, 1390, BLUE, width=4, amplitude=12)
        label(d, "海风", 430, 360, BLUE, 28)
        label(d, "不是旅行", 1190, 290, RED, 30)
        label(d, "是回看", 1330, 485, RED, 30)
    elif index == 7:
        label(d, "雨夜卡住的不是笔", 715, 110, RED, 42, underline=True)
        window(d, 285, 245, 360, 270, rain=True)
        rounded(d, [720, 570, 1280, 690], radius=28, fill="white", outline=INK, width=5)
        xiaohei(d, 610, 585, 1.26, pose="write", eyes="tired")
        hand_note(d, 850, 430, 180, 110, "空白")
        hand_note(d, 1090, 430, 180, 110, "真话")
        arrow(d, (705, 555), (850, 500), ORANGE)
        arrow(d, (1030, 500), (1090, 500), ORANGE, dashed=True)
        line(d, [(930, 335), (850, 560), (1010, 560), (930, 335)], fill=YELLOW, width=5)
        label(d, "灯很亮", 980, 320, BLUE, 30)
        label(d, "心里没路", 1120, 740, RED, 32)
    elif index == 8:
        label(d, "修复不是结局，是继续写", 670, 110, RED, 42, underline=True)
        simple_book(d, 270, 445, 180, 130, "旧信")
        xiaohei(d, 640, 525, 1.25, pose="carry", eyes="calm")
        rounded(d, [875, 375, 1115, 660], radius=22, fill="white", outline=INK, width=5)
        zh_text(d, (930, 470), "新的", 36, INK)
        zh_text(d, (930, 530), "回信", 36, INK)
        xiaohei(d, 1340, 530, 1.08, pose="stand", eyes="wide")
        arrow(d, (455, 505), (560, 505), ORANGE)
        arrow(d, (730, 505), (875, 505), ORANGE)
        arrow(d, (1115, 505), (1270, 505), ORANGE)
        smooth_wave_arrow(d, 380, 780, 1430, BLUE, width=4, amplitude=12)
        label(d, "把过去放轻", 455, 770, BLUE, 30)
        label(d, "还能往前", 1260, 735, RED, 32)
        camellia(d, 1145, 370, 38)

    return im


def render_character_sheet(output: Path) -> None:
    im = clean_page()
    d = draw_for(im)
    label(d, "小女孩动作表", 760, 110, RED, 46, underline=True)
    zh_text(d, (600, 185), "同一个角色，负责拉、推、写、搬、观察", 34, INK)
    poses = [
        ("stand", "观察", 300, 545, "calm"),
        ("pull", "拉出重点", 560, 545, "wide"),
        ("push", "推开关系", 820, 545, "wide"),
        ("write", "写下真话", 1080, 545, "calm"),
        ("carry", "抱着旧信", 1340, 545, "tired"),
        ("stand", "重新出发", 1600, 545, "wide"),
    ]
    for pose, text, x, y, eyes in poses:
        xiaohei(d, x, y, 1.12, pose=pose, eyes=eyes)
        zh_text(d, (x - 58, 760), text, 28, MUTED)
    camellia(d, 225, 800, 30)
    hand_note(d, 170, 845, 130, 80, "山茶")
    hand_note(d, 1500, 835, 150, 84, "回信")
    output.parent.mkdir(parents=True, exist_ok=True)
    finish_image(im).save(output)


def main() -> None:
    global RENDER_SCALE, OUTPUT_SCALE
    parser = argparse.ArgumentParser(description="Render Camellia's Letter xiaohei-style PNG illustrations.")
    parser.add_argument("output_dir", type=Path)
    parser.add_argument("--legacy-scenes", action="store_true", help="Render the older room-card scene set.")
    parser.add_argument("--scale", type=int, default=None, help="Deprecated alias for --aa-scale.")
    parser.add_argument("--aa-scale", type=int, default=3, help="Internal supersampling scale for antialiasing.")
    parser.add_argument("--output-scale", type=int, default=1, help="Output resolution scale. Use 2 for 3840x2160.")
    parser.add_argument("--srt", type=Path, help="Render a full timeline-matched image set from an SRT file.")
    parser.add_argument("--count", type=int, default=30, help="Number of timeline images to render in --srt mode.")
    args = parser.parse_args()
    aa_scale = args.scale if args.scale is not None else args.aa_scale
    OUTPUT_SCALE = max(1, args.output_scale)
    RENDER_SCALE = max(OUTPUT_SCALE, aa_scale)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    render_character_sheet(args.output_dir / "character_sheet.png")
    if args.srt:
        chunks = split_srt(args.srt, args.count)
        timeline = []
        for chunk in chunks:
            idx = chunk["index"]
            title, notes, kind = pick_blueprint(idx)
            props = merge_props(kind, chunk["text"])
            caption = scene_caption(kind, title, props)
            start = chunk["start_ms"]
            end = chunk["end_ms"]
            filename = f"scene_{idx:02d}_{start:07d}_{end:07d}.png"
            finish_image(render_full_scene(idx, chunk)).save(args.output_dir / filename)
            timeline.append(
                {
                    "index": idx,
                    "image": filename,
                    "start_ms": start,
                    "end_ms": end,
                    "duration_ms": end - start,
                    "title": title,
                    "notes": notes,
                    "kind": kind,
                    "props": props,
                    "prop_labels": [PROP_LABELS.get(prop, prop) for prop in props],
                    "caption": caption,
                    "subtitle_excerpt": chunk["text"][:120],
                }
            )
        (args.output_dir / "image_timeline.json").write_text(json.dumps(timeline, ensure_ascii=False, indent=2), encoding="utf-8")
        (args.output_dir / "scene_plan.json").write_text(
            json.dumps(
                {
                    "srt": str(args.srt),
                    "count": len(timeline),
                    "total_duration_ms": chunks[-1]["end_ms"],
                    "image_size": [W * OUTPUT_SCALE, H * OUTPUT_SCALE],
                    "aa_scale": RENDER_SCALE,
                    "output_scale": OUTPUT_SCALE,
                    "timeline": timeline,
                },
                ensure_ascii=False,
                indent=2,
            ),
            encoding="utf-8",
        )
    elif args.legacy_scenes:
        for i, scene in enumerate(SCENES, start=1):
            finish_image(render_scene(scene, i)).save(args.output_dir / f"scene_{i:02d}.png")
    else:
        for i in range(1, 9):
            finish_image(render_infographic_scene(i)).save(args.output_dir / f"scene_{i:02d}.png")
    print(args.output_dir.resolve())


if __name__ == "__main__":
    main()
