#!/usr/bin/env python3
import argparse
import json
import os
import re
import shutil
import subprocess
import time
from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance, ImageFont


WIDTH = 1920
HEIGHT = 1080
TARGET_MIN_SECONDS = 30 * 60
MAX_SUBTITLE_LINE_CHARS = 18
COVER_SECONDS = 5


def safe_stem(path: Path) -> str:
    stem = path.stem.strip() or "book"
    return "".join(ch if ch.isalnum() or ch in ("-", "_") else "_" for ch in stem)[:80]


def safe_output_name(value: str, fallback: str = "book") -> str:
    value = str(value or "").strip() or fallback
    value = re.sub(r'[\\/:*?"<>|\r\n\t]+', "_", value)
    value = re.sub(r"\s+", "_", value)
    return value.strip("._ ")[:80] or fallback


def run(cmd: list[str], cwd: Path | None = None) -> None:
    completed = subprocess.run(
        cmd,
        cwd=str(cwd) if cwd else None,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="ignore",
    )
    if completed.returncode != 0:
        raise RuntimeError(
            "Command failed:\n{}\n\nstdout:\n{}\n\nstderr:\n{}".format(
                " ".join(str(part) for part in cmd),
                completed.stdout[-4000:],
                completed.stderr[-4000:],
            )
        )


def ffprobe_duration_ms(path: Path) -> int:
    output = subprocess.run(
        [
            "ffprobe",
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            str(path),
        ],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="ignore",
    )
    return int(round(float(output.stdout.strip()) * 1000))


def newest_audio(material_root: Path) -> Path | None:
    candidates = []
    for pattern in ("audio/**/*.mp3", "audio/**/*.wav", "*.mp3", "*.wav"):
        candidates.extend(path for path in material_root.glob(pattern) if path.is_file())
    if not candidates:
        return None
    return max(candidates, key=lambda path: path.stat().st_mtime)


def find_material_root(epub: Path, output_dir: Path | None) -> Path | None:
    if output_dir:
        current = output_dir
        for candidate in [current, *current.parents]:
            if (candidate / "narration.txt").exists() or (candidate / "materials.json").exists():
                return candidate
    output_root = epub.parent / "output"
    if not output_root.exists():
        return None
    matches = []
    for child in output_root.iterdir():
        if child.is_dir() and ((child / "narration.txt").exists() or (child / "materials.json").exists()):
            matches.append(child)
    return max(matches, key=lambda path: path.stat().st_mtime) if matches else None


def read_text(path: Path, fallback: str = "") -> str:
    if not path.exists():
        return fallback
    return path.read_text(encoding="utf-8", errors="ignore").strip()


def read_material_json(material_root: Path) -> dict:
    path = material_root / "materials.json"
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8", errors="ignore"))
    except Exception:
        return {}


def split_subtitle_text(text: str, max_chars: int = MAX_SUBTITLE_LINE_CHARS) -> list[str]:
    text = " ".join(text.replace("\r", "\n").split())
    if not text:
        return []
    hard_breaks = "。！？；!?;"
    chunks: list[str] = []
    current = ""
    for ch in text:
        current += ch
        if ch in hard_breaks or len(current) >= max_chars:
            chunks.append(current.strip())
            current = ""
    if current.strip():
        chunks.append(current.strip())
    lines: list[str] = []
    for chunk in chunks:
        if len(chunk) <= max_chars:
            lines.append(chunk)
        else:
            for index in range(0, len(chunk), max_chars):
                lines.append(chunk[index : index + max_chars])
    return [line for line in lines if line]


def load_chinese_subtitle_lines(material_root: Path) -> list[str]:
    text = read_text(material_root / "subtitles.txt")
    if text.strip():
        return [line.strip() for line in text.splitlines() if line.strip()]
    material = read_material_json(material_root)
    subtitles = material.get("subtitles")
    if isinstance(subtitles, list):
        lines = [str(line).strip() for line in subtitles if str(line).strip()]
        if lines:
            return lines
    narration = material.get("narration") or read_text(material_root / "narration.txt")
    return split_subtitle_text(str(narration))


def load_subtitle_lines(material_root: Path) -> list[str]:
    english_cache = material_root / "subtitles_en.json"
    chinese_text = read_text(material_root / "subtitles.txt")
    if english_cache.exists() and chinese_text.strip():
        chinese_lines = [line.strip() for line in chinese_text.splitlines() if line.strip()]
        try:
            english_lines = json.loads(english_cache.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            english_lines = []
        if isinstance(english_lines, list) and len(english_lines) == len(chinese_lines):
            return [
                f"{zh}\n{str(en).strip()}"
                for zh, en in zip(chinese_lines, english_lines)
                if zh and str(en).strip()
            ]

    bilingual_text = read_text(material_root / "subtitles_bilingual.txt")
    if bilingual_text.strip():
        raw_lines = [line.strip() for line in bilingual_text.splitlines() if line.strip()]
        lines = []
        for index in range(0, len(raw_lines), 2):
            lines.append("\n".join(raw_lines[index : index + 2]))
        if lines:
            return lines

    material = read_material_json(material_root)
    subtitles = material.get("subtitles")
    if isinstance(subtitles, list):
        lines = [str(line).strip() for line in subtitles if str(line).strip()]
        if lines:
            return lines
    subtitles_text = read_text(material_root / "subtitles.txt")
    lines = [line.strip() for line in subtitles_text.splitlines() if line.strip()]
    if lines:
        return lines
    narration = material.get("narration") or read_text(material_root / "narration.txt")
    return split_subtitle_text(str(narration))


def format_srt_time(ms: int) -> str:
    ms = max(0, ms)
    hours = ms // 3_600_000
    ms %= 3_600_000
    minutes = ms // 60_000
    ms %= 60_000
    seconds = ms // 1000
    millis = ms % 1000
    return f"{hours:02}:{minutes:02}:{seconds:02},{millis:03}"


def format_ass_time(ms: int) -> str:
    ms = max(0, ms)
    hours = ms // 3_600_000
    ms %= 3_600_000
    minutes = ms // 60_000
    ms %= 60_000
    seconds = ms // 1000
    centis = (ms % 1000) // 10
    return f"{hours}:{minutes:02}:{seconds:02}.{centis:02}"


def parse_srt_time(value: str) -> int:
    match = re.match(r"(\d+):(\d{2}):(\d{2})[,.](\d{1,3})", value.strip())
    if not match:
        raise ValueError(f"Invalid SRT timestamp: {value}")
    hours, minutes, seconds, millis = match.groups()
    return (
        int(hours) * 3_600_000
        + int(minutes) * 60_000
        + int(seconds) * 1000
        + int(millis.ljust(3, "0")[:3])
    )


def read_srt_events(path: Path) -> list[tuple[int, int, str]]:
    text = read_text(path)
    blocks = re.split(r"\n\s*\n", text.replace("\r\n", "\n").replace("\r", "\n"))
    events: list[tuple[int, int, str]] = []
    for block in blocks:
        lines = [line.strip() for line in block.splitlines() if line.strip()]
        if not lines:
            continue
        time_line = next((line for line in lines if "-->" in line), "")
        if not time_line:
            continue
        time_index = lines.index(time_line)
        start_raw, end_raw = [part.strip().split()[0] for part in time_line.split("-->", 1)]
        body = "\n".join(lines[time_index + 1 :]).strip()
        if body:
            events.append((parse_srt_time(start_raw), parse_srt_time(end_raw), body))
    return events


def offset_events(events: list[tuple[int, int, str]], offset_ms: int) -> list[tuple[int, int, str]]:
    return [(start + offset_ms, end + offset_ms, text) for start, end, text in events]


def build_subtitle_events(lines: list[str], duration_ms: int, offset_ms: int = 0) -> list[tuple[int, int, str]]:
    if not lines:
        return []
    weights = [max(3, len(line)) for line in lines]
    total = max(1, sum(weights))
    cursor = 0
    events = []
    for index, line in enumerate(lines):
        if index == len(lines) - 1:
            end = duration_ms
        else:
            end = int(round((sum(weights[: index + 1]) / total) * duration_ms))
        end = max(cursor + 700, min(duration_ms, end))
        events.append((cursor + offset_ms, end + offset_ms, line))
        cursor = min(duration_ms, end + 80)
    return events


def write_srt(path: Path, events: list[tuple[int, int, str]]) -> None:
    parts = []
    for index, (start, end, text) in enumerate(events, 1):
        parts.append(f"{index}\n{format_srt_time(start)} --> {format_srt_time(end)}\n{text}\n")
    path.write_text("\n".join(parts), encoding="utf-8")


def ass_escape(text: str) -> str:
    return text.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}").replace("\n", "\\N")


def split_bilingual_subtitle_text(text: str) -> tuple[str, str]:
    parts = [part.strip() for part in str(text or "").replace("\\N", "\n").splitlines() if part.strip()]
    if len(parts) >= 2:
        return parts[0], " ".join(parts[1:])
    return str(text or "").strip(), ""


def write_ass(path: Path, events: list[tuple[int, int, str]]) -> None:
    header = f"""[Script Info]
ScriptType: v4.00+
ScaledBorderAndShadow: yes
PlayResX: {WIDTH}
PlayResY: {HEIGHT}

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Chinese,Microsoft YaHei UI,82,&H00C8F6FF,&H000000FF,&H00000000,&H00000000,-1,0,0,0,100,100,0,0,1,5,2,2,120,120,158,1
Style: English,Microsoft YaHei UI,54,&H001AA5F2,&H000000FF,&H00000000,&H00000000,-1,0,0,0,100,100,0,0,1,5,2,2,120,120,90,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""
    lines = [header]
    for start, end, text in events:
        chinese, english = split_bilingual_subtitle_text(text)
        if chinese:
            lines.append(
                f"Dialogue: 0,{format_ass_time(start)},{format_ass_time(end)},Chinese,,0,0,0,,{ass_escape(chinese)}"
            )
        if english:
            lines.append(
                f"Dialogue: 0,{format_ass_time(start)},{format_ass_time(end)},English,,0,0,0,,{ass_escape(english)}"
            )
    path.write_text("\n".join(lines), encoding="utf-8")


def find_existing_aeneas_ass(material_root: Path) -> Path | None:
    candidates = [
        path
        for path in material_root.rglob("*.aeneas*.ass")
        if path.is_file() and "zh-en" in path.name.lower()
    ]
    if not candidates:
        return None
    return max(candidates, key=lambda path: path.stat().st_mtime)


def load_english_lines(material_root: Path, expected_count: int) -> list[str]:
    english_cache = material_root / "subtitles_en.json"
    if english_cache.exists():
        try:
            data = json.loads(english_cache.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            raise RuntimeError(f"English subtitle cache is invalid JSON: {english_cache}") from exc
        if isinstance(data, list):
            lines = [str(item).strip() for item in data]
            if len(lines) == expected_count and all(lines):
                return lines
            raise RuntimeError(
                f"English subtitle cache must contain {expected_count} non-empty lines, "
                f"but got {len(lines)}: {english_cache}"
            )

    translation_cache = material_root / "translation_cache.json"
    if translation_cache.exists():
        try:
            data = json.loads(translation_cache.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            raise RuntimeError(f"Translation cache is invalid JSON: {translation_cache}") from exc
        lines: list[str] = []
        if isinstance(data, list):
            for item in data:
                if isinstance(item, dict):
                    lines.append(str(item.get("en") or item.get("english") or item.get("translation") or "").strip())
                else:
                    lines.append(str(item).strip())
        elif isinstance(data, dict):
            items = data.get("items") if isinstance(data.get("items"), list) else None
            translations = data.get("translations") if isinstance(data.get("translations"), list) else None
            source = items or translations or []
            for item in source:
                if isinstance(item, dict):
                    lines.append(str(item.get("en") or item.get("english") or item.get("translation") or "").strip())
                else:
                    lines.append(str(item).strip())
        if len(lines) == expected_count and all(lines):
            return lines
        if lines:
            raise RuntimeError(
                f"Translation cache must contain {expected_count} non-empty English lines, "
                f"but got {len(lines)}: {translation_cache}"
            )

    raise RuntimeError(
        "English subtitles are required after aeneas alignment. "
        "Generate them with Codex/AI first and save subtitles_en.json or translation_cache.json."
    )


def run_aeneas_alignment(audio: Path, subtitle_lines: list[str], output_dir: Path, audio_language: str) -> tuple[Path, dict]:
    try:
        from aeneas.executetask import ExecuteTask
        from aeneas.task import Task
    except Exception as exc:
        raise RuntimeError(
            "aeneas.tools is required for final subtitle timing. "
            "Install/configure aeneas instead of falling back to estimated subtitle timing."
        ) from exc

    output_dir.mkdir(parents=True, exist_ok=True)
    text_file = output_dir / "aeneas_input.txt"
    srt_file = output_dir / "hard_subtitle.aeneas.chn.srt"
    text_file.write_text("\n".join(subtitle_lines), encoding="utf-8")
    language = (audio_language or "cmn").strip() or "cmn"
    config = (
        f"task_language={language}|"
        "is_text_type=plain|"
        "os_task_file_format=srt|"
        "task_adjust_boundary_algorithm=percent|"
        "task_adjust_boundary_percent_value=50"
    )
    task = Task(config_string=config)
    task.audio_file_path_absolute = str(audio)
    task.text_file_path_absolute = str(text_file)
    task.sync_map_file_path_absolute = str(srt_file)
    ExecuteTask(task).execute()
    task.output_sync_map_file()
    events = read_srt_events(srt_file)
    if len(events) != len(subtitle_lines):
        raise RuntimeError(
            f"aeneas cue count mismatch: expected {len(subtitle_lines)}, got {len(events)} from {srt_file}"
        )
    manifest = {
        "audioLanguage": language,
        "inputAudio": str(audio),
        "inputText": str(text_file),
        "outputSrt": str(srt_file),
        "cueCount": len(events),
        "firstCueMs": events[0][0] if events else None,
        "lastCueMs": events[-1][1] if events else None,
    }
    (output_dir / "hard_subtitle.aeneas.subtitle_manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return srt_file, manifest


def build_aeneas_subtitles(material_root: Path, audio: Path, video_dir: Path, audio_language: str) -> tuple[Path, Path, list[tuple[int, int, str]], dict]:
    existing_ass = find_existing_aeneas_ass(material_root)
    if existing_ass:
        ass_file = video_dir / "hard_subtitle.aeneas.zh-en.ass"
        events = offset_events(read_ass_dialogue_events(existing_ass), COVER_SECONDS * 1000)
        write_ass(ass_file, events)
        srt_file = video_dir / "hard_subtitle.aeneas.zh-en.srt"
        write_srt(srt_file, events)
        return ass_file, srt_file, events, {
            "subtitleTiming": "existing_aeneas_ass",
            "sourceAss": str(existing_ass),
            "cueCount": len(events),
            "delayMs": COVER_SECONDS * 1000,
        }

    chinese_lines = load_chinese_subtitle_lines(material_root)
    if not chinese_lines:
        raise RuntimeError("No Chinese subtitle lines found for aeneas alignment.")
    aeneas_dir = video_dir
    zh_srt, subtitle_manifest = run_aeneas_alignment(audio, chinese_lines, aeneas_dir, audio_language)
    zh_events = read_srt_events(zh_srt)
    english_lines = load_english_lines(material_root, len(zh_events))
    bilingual_events = [
        (start + COVER_SECONDS * 1000, end + COVER_SECONDS * 1000, f"{zh}\n{en}")
        for (start, end, zh), en in zip(zh_events, english_lines)
    ]
    srt_file = video_dir / "hard_subtitle.aeneas.zh-en.srt"
    ass_file = video_dir / "hard_subtitle.aeneas.zh-en.ass"
    write_srt(srt_file, bilingual_events)
    write_ass(ass_file, bilingual_events)
    subtitle_manifest = {
        **subtitle_manifest,
        "subtitleTiming": "aeneas",
        "zhEnSrt": str(srt_file),
        "zhEnAss": str(ass_file),
        "dialogueCount": len(bilingual_events) * 2,
    }
    return ass_file, srt_file, bilingual_events, subtitle_manifest


def read_ass_dialogue_events(path: Path) -> list[tuple[int, int, str]]:
    events: dict[tuple[int, int], list[str]] = {}
    for line in read_text(path).splitlines():
        if not line.startswith("Dialogue:"):
            continue
        parts = line.split(",", 9)
        if len(parts) < 10:
            continue
        start = parse_ass_time(parts[1])
        end = parse_ass_time(parts[2])
        text = parts[9].replace("\\N", "\n")
        events.setdefault((start, end), []).append(text)
    return [(start, end, "\n".join(lines)) for (start, end), lines in sorted(events.items())]


def parse_ass_time(value: str) -> int:
    match = re.match(r"(\d+):(\d{2}):(\d{2})\.(\d{1,2})", value.strip())
    if not match:
        raise ValueError(f"Invalid ASS timestamp: {value}")
    hours, minutes, seconds, centis = match.groups()
    return (
        int(hours) * 3_600_000
        + int(minutes) * 60_000
        + int(seconds) * 1000
        + int(centis.ljust(2, "0")[:2]) * 10
    )


def atempo_chain(factor: float) -> str:
    values = []
    remaining = factor
    while remaining < 0.5:
        values.append(0.5)
        remaining /= 0.5
    while remaining > 2.0:
        values.append(2.0)
        remaining /= 2.0
    values.append(remaining)
    return ",".join(f"atempo={value:.6f}" for value in values)


def prepare_narration_audio(source: Path, output: Path, target_min_seconds: int) -> tuple[Path, int, float]:
    source_ms = ffprobe_duration_ms(source)
    source_seconds = source_ms / 1000.0
    if source_seconds >= target_min_seconds:
        return source, source_ms, 1.0
    target_seconds = target_min_seconds
    tempo = source_seconds / target_seconds
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg was not found, so audio cannot be prepared.")
    run(
        [
            ffmpeg,
            "-y",
            "-i",
            str(source),
            "-filter:a",
            atempo_chain(tempo),
            "-vn",
            "-c:a",
            "libmp3lame",
            "-b:a",
            "160k",
            str(output),
        ]
    )
    return output, ffprobe_duration_ms(output), target_seconds / source_seconds


def ffmpeg_text_escape(value: str) -> str:
    return value.replace("\\", "\\\\").replace(":", "\\:").replace("'", "\\'").replace("%", "\\%")


def wrap_text(value: str, max_chars: int, max_lines: int) -> str:
    chars = list(value.strip())
    if not chars:
        return ""
    lines = []
    for index in range(0, len(chars), max_chars):
        if len(lines) >= max_lines:
            break
        lines.append("".join(chars[index : index + max_chars]))
    return "\n".join(lines)


def compact_text(value: str, max_chars: int) -> str:
    value = " ".join(value.strip().split())
    if len(value) <= max_chars:
        return value
    return value[: max(1, max_chars - 1)].rstrip() + "…"


def looks_mojibake(value: str) -> bool:
    return any(token in value for token in ("锛", "銆", "€", "涓", "绋", "瀛"))


def clean_label(value: str, fallback: str = "") -> str:
    value = " ".join(str(value or "").replace("\r", "\n").split())
    if looks_mojibake(value):
        return fallback
    return value


def extract_book_title(raw_title: str, epub_stem: str = "") -> str:
    raw_title = clean_label(raw_title) or clean_label(epub_stem) or "本期书籍"
    match = re.search(r"《([^》]+)》", raw_title)
    if match:
        return match.group(1).strip()
    for delimiter in ("：", ":", "｜", "|", "-", "_"):
        if delimiter in raw_title:
            head = raw_title.split(delimiter, 1)[0].strip()
            if head:
                return head.strip("《》 ")
    return raw_title.strip("《》 ")


def split_cover_title(title: str) -> tuple[str, int]:
    title = compact_text(title, 18)
    length = len(title)
    if length <= 4:
        return "\n".join(title), 116
    if length <= 8:
        mid = (length + 1) // 2
        return title[:mid] + "\n" + title[mid:], 104
    if length <= 14:
        punctuation_breaks = [index + 1 for index, ch in enumerate(title) if ch in "，、：:；;"]
        if punctuation_breaks:
            mid_target = length / 2
            mid = min(punctuation_breaks, key=lambda index: abs(index - mid_target))
            if 3 <= mid <= length - 3:
                return title[:mid].rstrip("，、：:；;") + "\n" + title[mid:].lstrip("，、：:；;"), 84
        mid = (length + 1) // 2
        return title[:mid] + "\n" + title[mid:], 84
    return wrap_text(title, 9, 2), 70


def font_path(*names: str) -> str | None:
    fonts_dir = Path(os.environ.get("WINDIR", r"C:\Windows")) / "Fonts"
    for name in names:
        candidate = fonts_dir / name
        if candidate.exists():
            return str(candidate)
    return None


def load_font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    preferred = (
        ("msyhbd.ttc", "simhei.ttf", "simsunb.ttf")
        if bold
        else ("msyh.ttc", "Deng.ttf", "simsun.ttc")
    )
    path = font_path(*preferred)
    if path:
        return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def text_size(draw: ImageDraw.ImageDraw, text: str, font: ImageFont.ImageFont) -> tuple[int, int]:
    bbox = draw.textbbox((0, 0), text, font=font)
    return bbox[2] - bbox[0], bbox[3] - bbox[1]


def wrap_text_by_width(draw: ImageDraw.ImageDraw, text: str, font: ImageFont.ImageFont, max_width: int) -> list[str]:
    text = " ".join(str(text or "").replace("\r", "\n").split())
    if not text:
        return []
    lines: list[str] = []
    current = ""
    for char in text:
        candidate = current + char
        if current and text_size(draw, candidate, font)[0] > max_width:
            lines.append(current)
            current = char
        else:
            current = candidate
    if current:
        lines.append(current)
    return lines


def first_complete_sentence(text: str, fallback: str) -> str:
    text = clean_label(text, fallback)
    if not text:
        return fallback
    for mark in ("。", "！", "？", ".", "!", "?"):
        index = text.find(mark)
        if 12 <= index <= 120:
            return text[: index + 1]
    return text


def cover_bottom_lines(title: str, subtitle: str, kicker: str) -> list[str]:
    book_title = extract_book_title(title)
    clean_subtitle = clean_label(subtitle)
    if "：" in clean_subtitle:
        clean_subtitle = clean_subtitle.split("：", 1)[1].strip()
    elif ":" in clean_subtitle:
        clean_subtitle = clean_subtitle.split(":", 1)[1].strip()
    clean_subtitle = re.sub(r"本期视频为《[^》]+》的中文听书解读与原创转述。?", "", clean_subtitle)
    clean_subtitle = re.sub(r"我们用[^。！？]+。?", "", clean_subtitle).strip()
    sentence = first_complete_sentence(clean_subtitle, "")
    if 18 <= len(sentence) <= 34:
        line1 = sentence
    elif kicker and " / " in kicker:
        left, right = kicker.split(" / ", 1)
        line1 = f"从{left}，到{right}。"
    else:
        line1 = f"从故事深处，到人生转折。"
    line2 = f"三十五分钟，听完《{book_title}》。"
    return [line1, line2]


def cover_kicker_from_material(material: dict, overview: dict) -> str:
    raw_tags = material.get("tags")
    tags = [str(tag).strip() for tag in raw_tags if str(tag).strip()] if isinstance(raw_tags, list) else []
    ignored = {
        "半小时听完一本书",
        "中文听书",
        "睡前听书",
        "audiobook",
        "book summary",
        "youtube中文",
    }
    book_title = str(overview.get("title") or "").strip()
    creator = str(overview.get("creator") or material.get("author") or material.get("creator") or "").strip()
    candidates = [
        tag
        for tag in tags
        if tag.lower() not in ignored
        and tag != book_title
        and tag != creator
        and len(tag) <= 8
        and not re.search(r"[A-Za-z]{3,}", tag)
    ]
    if len(candidates) >= 2:
        return f"{candidates[0]} / {candidates[1]}"
    if candidates:
        return f"{candidates[0]} / 中文听书解读"
    return "中文听书解读 / 睡前听书"


def ass_filter_path(path: Path) -> str:
    return path.name.replace("'", "\\'")


def image_candidates(directory: Path) -> list[Path]:
    if not directory.exists():
        return []
    candidates: list[Path] = []
    for pattern in ("*.png", "*.jpg", "*.jpeg", "*.webp"):
        candidates.extend(path for path in directory.glob(pattern) if path.is_file())
    return sorted(candidates, key=lambda path: path.name.lower())


def find_local_visual_dirs(material_root: Path) -> list[Path]:
    roots = [
        material_root / "visual_assets" / "originals",
        material_root / "assets" / "visual_assets" / "originals",
    ]
    dirs: list[Path] = []
    for root in roots:
        if not root.exists():
            continue
        for child in root.iterdir():
            if child.is_dir() and image_candidates(child):
                dirs.append(child)
    return sorted(dirs, key=lambda path: path.stat().st_mtime, reverse=True)


def appdata_exports_root() -> Path | None:
    appdata = os.environ.get("APPDATA")
    if appdata:
        root = Path(appdata) / "com.abookin30minutes.desktop" / "exports"
        if root.exists():
            return root
    fallback = Path.home() / "AppData" / "Roaming" / "com.abookin30minutes.desktop" / "exports"
    return fallback if fallback.exists() else None


def find_reference_visual_dirs() -> list[Path]:
    exports = appdata_exports_root()
    if not exports:
        return []
    dirs: list[Path] = []
    for timeline in exports.rglob("visual_timeline.json"):
        parent = timeline.parent
        if image_candidates(parent):
            dirs.append(parent)

    def score(path: Path) -> tuple[int, float]:
        name = path.name.lower()
        quality = 2 if "formal_content_images" in name else 1 if "generic_content_images" in name else 0
        return quality, path.stat().st_mtime

    return sorted(dirs, key=score, reverse=True)


def migrate_visual_assets(material_root: Path, video_dir: Path) -> tuple[list[Path], Path | None, str]:
    local_dirs = find_local_visual_dirs(material_root)
    source_dir: Path | None = local_dirs[0] if local_dirs else None
    source_kind = "task_visual_assets"
    if source_dir is None:
        references = find_reference_visual_dirs()
        source_dir = references[0] if references else None
        source_kind = "historical_reference_assets"
    if source_dir is None:
        return [], None, "none"

    dest_dir = video_dir
    dest_dir.mkdir(parents=True, exist_ok=True)
    copied: list[Path] = []
    for index, source in enumerate(image_candidates(source_dir)[:8], 1):
        suffix = source.suffix.lower() or ".png"
        dest = dest_dir / f"visual_{index:02d}_cinematic_background{suffix}"
        if source.resolve() != dest.resolve():
            shutil.copy2(source, dest)
        copied.append(dest)

    source_timeline = source_dir / "visual_timeline.json"
    if source_timeline.exists():
        shutil.copy2(source_timeline, video_dir / "source_visual_timeline.json")
    manifest = {
        "sourceKind": source_kind,
        "sourceDir": str(source_dir),
        "copiedAt": time.strftime("%Y-%m-%d %H:%M:%S"),
        "assets": [str(path) for path in copied],
    }
    (video_dir / "visual_assets_manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    return copied, dest_dir, source_kind


def render_cover_image(
    video_dir: Path,
    base_image: Path | None,
    title: str,
    subtitle: str,
    author: str = "",
    epub_stem: str = "",
    kicker: str = "",
) -> Path:
    cover = video_dir / "cover.jpg"
    book_title = extract_book_title(title or epub_stem, epub_stem)
    title_text, title_font_size = split_cover_title(book_title)
    if base_image and base_image.is_file():
        image = Image.open(base_image).convert("RGB")
        image.thumbnail((WIDTH, HEIGHT), Image.Resampling.LANCZOS)
        left = max(0, (image.width - WIDTH) // 2)
        top = max(0, (image.height - HEIGHT) // 2)
        if image.width < WIDTH or image.height < HEIGHT:
            scale = max(WIDTH / image.width, HEIGHT / image.height)
            image = image.resize((round(image.width * scale), round(image.height * scale)), Image.Resampling.LANCZOS)
            left = (image.width - WIDTH) // 2
            top = (image.height - HEIGHT) // 2
        image = image.crop((left, top, left + WIDTH, top + HEIGHT))
    else:
        image = Image.new("RGB", (WIDTH, HEIGHT), (23, 23, 23))

    image = ImageEnhance.Brightness(image).enhance(0.76)
    image = ImageEnhance.Contrast(image).enhance(1.08)
    image = ImageEnhance.Color(image).enhance(0.88)
    overlay = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 58))
    bottom_overlay = Image.new("RGBA", (WIDTH, 320), (0, 0, 0, 88))
    overlay.alpha_composite(bottom_overlay, (0, 760))
    image = Image.alpha_composite(image.convert("RGBA"), overlay)
    draw = ImageDraw.Draw(image)

    gold = (229, 193, 109, 245)
    white = (255, 255, 255, 255)
    muted = (199, 192, 173, 130)
    dark = (17, 20, 23, 220)

    label_font = load_font(48, bold=True)
    vol_font = load_font(32, bold=False)
    kicker_font = load_font(36)
    title_font = load_font(title_font_size, bold=True)
    author_font = load_font(42)
    bottom_font = load_font(34)
    footer_font = load_font(26)

    draw.rounded_rectangle((110, 74, 606, 162), radius=8, fill=dark, outline=gold, width=3)
    draw.text((148, 96), "半小时听完一本书", font=label_font, fill=white)
    draw.rounded_rectangle((1688, 74, 1850, 134), radius=8, fill=gold)
    draw.text((1712, 88), "VOL.001", font=vol_font, fill=(20, 20, 20, 255))

    draw.rectangle((1040, 250, 1046, 795), fill=gold)
    draw.text((1080, 262), clean_label(kicker, "中文听书解读 / 睡前听书"), font=kicker_font, fill=white)
    y = 362
    for line in title_text.splitlines():
        draw.text((1080, y), line, font=title_font, fill=white, stroke_width=3, stroke_fill=(0, 0, 0, 200))
        y += title_font_size + 10
    clean_author = compact_text(clean_label(author), 18)
    if clean_author:
        draw.text((1080, 710), clean_author, font=author_font, fill=(245, 240, 230, 255))

    bottom_y = 830
    for line in cover_bottom_lines(title, subtitle, clean_label(kicker, "")):
        draw.text((110, bottom_y), line, font=bottom_font, fill=white)
        bottom_y += 54
    draw.line((110, 992, 1850, 992), fill=gold, width=2)
    draw.text((110, 1024), "A BOOK IN 30 MINUTES", font=footer_font, fill=muted)
    footer_text = "睡前听书系列"
    footer_width, _ = text_size(draw, footer_text, footer_font)
    draw.text((1850 - footer_width, 1024), footer_text, font=footer_font, fill=muted)
    image.convert("RGB").save(cover, quality=94)
    return cover


def render_placeholder_background(video_dir: Path, title: str) -> Path:
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg was not found, so fallback background cannot be generated.")
    background = video_dir / "background.jpg"
    safe_title = ffmpeg_text_escape(wrap_text(title or "A Book in 30 Minutes", 24, 2))
    base = (
        "drawbox=x=0:y=0:w=iw:h=ih:color=0x2c2a26@0.28:t=fill,"
        "drawbox=x=0:y=0:w=iw:h=ih:color=black@0.30:t=fill,"
        "vignette=PI/4,"
        "drawbox=x=0:y=h*0.70:w=iw:h=ih*0.30:color=black@0.24:t=fill,"
        "drawtext=font='Microsoft YaHei':"
        f"text='{safe_title}':fontcolor=white@0.16:fontsize=68:x=(w-text_w)/2:y=h*0.16"
    )
    run([ffmpeg, "-y", "-f", "lavfi", "-i", f"color=c=0x26211d:s={WIDTH}x{HEIGHT}:d=1", "-frames:v", "1", "-vf", base, str(background)])
    return background


def common_video_encode_args(output: Path) -> list[str]:
    return [
        "-r",
        "30",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-preset",
        "veryfast",
        "-crf",
        "23",
        "-c:a",
        "aac",
        "-b:a",
        "160k",
        "-movflags",
        "+faststart",
        str(output),
    ]


def render_no_subtitle_video(
    output: Path,
    cover: Path,
    content_images: list[Path],
    audio: Path,
    background_music: Path | None,
) -> None:
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg was not found, so the video cannot be generated.")
    audio_seconds = ffprobe_duration_ms(audio) / 1000.0
    total_seconds = COVER_SECONDS + audio_seconds
    images = [cover, *content_images]
    if not content_images:
        raise RuntimeError("No visual images were prepared for video rendering.")
    body_segment = audio_seconds / max(1, len(content_images))
    cmd = [
        ffmpeg,
        "-y",
    ]
    durations = [float(COVER_SECONDS), *([body_segment] * len(content_images))]
    for image, duration in zip(images, durations):
        cmd.extend(
            [
                "-framerate",
                "30",
                "-loop",
                "1",
                "-t",
                f"{duration:.3f}",
                "-i",
                str(image),
            ]
        )
    audio_index = len(images)
    cmd.extend(
        [
            "-i",
            str(audio),
        ]
    )
    if background_music and background_music.is_file():
        cmd.extend(["-stream_loop", "-1", "-i", str(background_music)])

    video_filters = []
    video_labels = []
    for index, duration in enumerate(durations):
        frames = max(1, int(round(duration * 30)))
        zoom_direction = "+" if index % 2 == 0 else "-"
        zoom_expr = "min(zoom+0.00012,1.08)" if zoom_direction == "+" else "max(zoom-0.00008,1.0)"
        video_filters.append(
            f"[{index}:v]scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=increase,"
            f"crop={WIDTH}:{HEIGHT},"
            "eq=brightness=-0.02:saturation=0.92:contrast=1.04,"
            f"zoompan=z='{zoom_expr}':d={frames}:x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':s={WIDTH}x{HEIGHT}:fps=30,"
            f"trim=duration={duration:.3f},setpts=PTS-STARTPTS[v{index}]"
        )
        video_labels.append(f"[v{index}]")
    video_concat = "".join(video_labels) + f"concat=n={len(images)}:v=1:a=0[v]"
    if background_music and background_music.is_file():
        bgm_index = audio_index + 1
        audio_mix = (
            f"aevalsrc=0:d={COVER_SECONDS:.3f}:s=48000[silence];"
            f"[silence][{audio_index}:a]concat=n=2:v=0:a=1[narr];"
            f"[{bgm_index}:a]volume=0.10[bgm];"
            "[narr][bgm]amix=inputs=2:duration=first:dropout_transition=2[a]"
        )
    else:
        audio_mix = f"aevalsrc=0:d={COVER_SECONDS:.3f}:s=48000[silence];[silence][{audio_index}:a]concat=n=2:v=0:a=1[a]"
    cmd.extend(
        [
            "-filter_complex",
            ";".join([*video_filters, video_concat, audio_mix]),
            "-map",
            "[v]",
            "-map",
            "[a]",
            "-t",
            f"{total_seconds:.3f}",
            *common_video_encode_args(output),
        ]
    )
    run(cmd, cwd=output.parent)


def build_visual_timeline(path: Path, cover: Path, content_images: list[Path], duration_ms: int, source_dir: Path | None, source_kind: str) -> None:
    body_start = COVER_SECONDS * 1000
    body_duration = max(0, duration_ms)
    segment_duration = body_duration / max(1, len(content_images))
    segments = [
        {
            "startMs": 0,
            "endMs": body_start,
            "image": str(cover),
            "description": "Generated cinematic cover intro",
            "kind": "cover",
        }
    ]
    cursor = float(body_start)
    for index, image in enumerate(content_images):
        end = body_start + body_duration if index == len(content_images) - 1 else cursor + segment_duration
        segments.append(
            {
                "startMs": int(round(cursor)),
                "endMs": int(round(end)),
                "image": str(image),
                "description": f"Cinematic content image {index + 1}",
                "kind": "content",
            }
        )
        cursor = end
    path.write_text(
        json.dumps(
            {
                "sourceKind": source_kind,
                "sourceDir": str(source_dir) if source_dir else None,
                "segments": segments,
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )


def render_hard_subtitle_video(
    output: Path,
    no_subtitle_video: Path,
    ass_file: Path,
) -> None:
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg was not found, so the hard subtitle video cannot be generated.")
    ass_path = ass_filter_path(ass_file)
    cmd = [
        ffmpeg,
        "-y",
        "-i",
        str(no_subtitle_video),
        "-vf",
        f"ass='{ass_path}'",
        "-map",
        "0:v",
        "-map",
        "0:a?",
        *common_video_encode_args(output),
    ]
    run(cmd, cwd=ass_file.parent)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--epub", required=True)
    parser.add_argument("--skip-notify", action="store_true")
    parser.add_argument("--audio-language", default="cmn")
    parser.add_argument("--allow-placeholder-visuals", action="store_true")
    parser.add_argument("--output-dir")
    parser.add_argument("--background-music")
    args = parser.parse_args()

    epub = Path(args.epub)
    if not epub.exists():
        raise FileNotFoundError(f"EPUB not found: {epub}")

    output_dir = Path(args.output_dir) if args.output_dir else None
    material_root = find_material_root(epub, output_dir)
    if material_root is None:
        material_root = epub.parent / "output"
    video_dir = output_dir or material_root
    video_dir.mkdir(parents=True, exist_ok=True)

    source_audio = newest_audio(material_root)
    if source_audio is None:
        raise RuntimeError(f"No audio file was found for video generation: {material_root / 'audio'}")

    material = read_material_json(material_root)
    title = str(material.get("videoTitle") or read_text(material_root / "title.txt", epub.stem)).strip()
    description = str(material.get("description") or read_text(material_root / "description.txt", "")).strip()
    overview = material.get("overview") if isinstance(material.get("overview"), dict) else {}
    author = str(
        material.get("author")
        or material.get("creator")
        or overview.get("creator")
        or ""
    ).strip()
    cover_kicker = cover_kicker_from_material(material, overview)
    subtitle_label = description.splitlines()[0].strip() if description else "Tonight's book"
    output_stem = safe_output_name(extract_book_title(title, epub.stem) or epub.stem, safe_stem(epub))

    prepared_audio, duration_ms, stretch_ratio = prepare_narration_audio(
        source_audio,
        video_dir / "narration_for_video.mp3",
        TARGET_MIN_SECONDS,
    )
    ass_file, srt_file, events, subtitle_manifest = build_aeneas_subtitles(
        material_root,
        prepared_audio,
        video_dir,
        args.audio_language,
    )

    content_images, visual_source_dir, visual_source_kind = migrate_visual_assets(material_root, video_dir)
    if not content_images:
        if not args.allow_placeholder_visuals:
            raise RuntimeError("No cinematic visual assets were found. Generate or import visual assets before creating the final video.")
        content_images = [render_placeholder_background(video_dir, title)]
        visual_source_dir = video_dir
        visual_source_kind = "explicit_placeholder_visuals"
    cover = render_cover_image(
        video_dir,
        content_images[0] if content_images else None,
        title,
        subtitle_label,
        author,
        epub.stem,
        cover_kicker,
    )
    background_music = Path(args.background_music) if args.background_music else None
    no_subtitle_video = video_dir / f"{output_stem}_无字幕母版.mp4"
    hard_video = video_dir / f"{output_stem}_中英双语字幕_精修版.mp4"
    render_no_subtitle_video(no_subtitle_video, cover, content_images, prepared_audio, background_music)
    render_hard_subtitle_video(hard_video, no_subtitle_video, ass_file)

    no_subtitle_duration_ms = ffprobe_duration_ms(no_subtitle_video)
    video_duration_ms = ffprobe_duration_ms(hard_video)
    manifest = video_dir / "pipeline_manifest.json"
    result = {
        "materialDir": str(material_root),
        "pipelineManifest": str(manifest),
        "cover": str(cover),
        "background": str(content_images[0]) if content_images else None,
        "visualAssetsDir": str(visual_source_dir) if visual_source_dir else None,
        "visualSourceKind": visual_source_kind,
        "visualAssetCount": len(content_images),
        "visualTimeline": str(video_dir / "visual_timeline.json"),
        "noSubtitleVideo": str(no_subtitle_video),
        "hardSubtitleVideo": str(hard_video),
        "hardSubtitleManifest": str(ass_file),
        "hardSubtitleSrt": str(srt_file),
        "subtitleTiming": subtitle_manifest.get("subtitleTiming"),
        "subtitleManifest": subtitle_manifest,
        "narrationAudioForVideo": str(prepared_audio),
        "sourceAudio": str(source_audio),
        "backgroundMusic": str(background_music) if background_music and background_music.is_file() else None,
        "sourceAudioDurationMs": ffprobe_duration_ms(source_audio),
        "narrationAudioForVideoDurationMs": duration_ms,
        "noSubtitleVideoDurationMs": no_subtitle_duration_ms,
        "videoDurationMs": video_duration_ms,
        "coverSeconds": COVER_SECONDS,
        "subtitleCount": len(events),
        "stretchRatio": stretch_ratio,
        "elapsedSeconds": 0.0,
    }
    build_visual_timeline(video_dir / "visual_timeline.json", cover, content_images, duration_ms, visual_source_dir, visual_source_kind)
    manifest.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
