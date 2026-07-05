#!/usr/bin/env python3
import argparse
import difflib
import json
import os
import re
import shutil
import subprocess
import sys
import time
import uuid
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance, ImageFont


WIDTH = 1920
HEIGHT = 1080
TARGET_MIN_SECONDS = 30 * 60
MAX_SUBTITLE_LINE_CHARS = 18
HEADER_SECONDS = 3
HEADER_AUDIO_ENCODE_SECONDS = 2.976
HEADER_AUDIO_DURATION_MS = 3000
CINEMATIC_FPS = 30
VISUAL_SCENE_MIN_COUNT = 32
VISUAL_SCENE_MAX_COUNT = 64
VISUAL_SUBTITLE_LINES_PER_IMAGE = 28
WHITEBOARD_SCENE_COUNT = VISUAL_SCENE_MIN_COUNT
WHITEBOARD_IMAGE_GENERATOR = (
    Path.home()
    / ".codex"
    / "skills"
    / "whiteboard-video-workflow"
    / "scripts"
    / "generate-image.py"
)
WHITEBOARD_PROMPT_PREFIX = (
    "Minimal hand-drawn illustration, pure illustration without any text, "
    "off-white paper background(#F6F1E3), dark gray sketch lines, orange as the only accent color(#CD6441), "
    "lots of negative space, Notion-like doodle aesthetic, faceless round-headed human figure, "
    "clean editorial composition, conceptual rather than literal, simple background. "
    "Absolutely no text, no words, no letters, no typography, no realism, no 3D, no painterly texture, "
    "no high saturation, no complex scene, no photographic detail. "
    "The overall mood is restrained, lucid, and emotionally calm. Keep the whole series visually consistent."
)
CINEMATIC_MOTION_PROFILES = (
    ("slow_push_center", "min(1.015+on*0.00016,1.13)", "(iw-iw/zoom)*0.50", "(ih-ih/zoom)*0.50"),
    ("drift_right", "min(1.035+on*0.00012,1.14)", "(iw-iw/zoom)*(0.18+0.34*on/{den})", "(ih-ih/zoom)*0.46"),
    ("drift_left", "min(1.035+on*0.00012,1.14)", "(iw-iw/zoom)*(0.78-0.34*on/{den})", "(ih-ih/zoom)*0.50"),
    ("rise_slow", "min(1.025+on*0.00014,1.13)", "(iw-iw/zoom)*0.52", "(ih-ih/zoom)*(0.68-0.28*on/{den})"),
    ("descend_slow", "min(1.025+on*0.00014,1.13)", "(iw-iw/zoom)*0.48", "(ih-ih/zoom)*(0.25+0.30*on/{den})"),
    ("slow_pull_back", "max(1.13-on*0.00012,1.025)", "(iw-iw/zoom)*0.50", "(ih-ih/zoom)*0.50"),
)
CINEMATIC_ENABLE_MOTION = os.environ.get("ABOOK_CINEMATIC_MOTION", "").strip().lower() in {
    "1",
    "true",
    "yes",
    "on",
}


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


def newest_audio(material_root: Path, exclude_names: set[str] | None = None) -> Path | None:
    exclude_names = exclude_names or set()
    excluded_names = {name.lower() for name in exclude_names}

    def is_generated_audio(path: Path) -> bool:
        lower_name = path.name.lower()
        lower_stem = path.stem.lower()
        if lower_name in excluded_names or lower_name.startswith("_"):
            return True
        if re.match(r"part_\d+\.(mp3|wav)$", lower_name):
            return True
        if lower_name in {"concat.txt", "concat_video_source.txt", "header.mp3"}:
            return True
        if lower_stem.startswith(("hard_subtitle", "narration_for_video")):
            return True
        if lower_stem.endswith(("_无字幕母版", "_中英双语字幕_精修版", "_video_mix")):
            return True
        if lower_stem in {"video_mix", "narration_for_video"}:
            return True
        return False

    root_candidates = []
    for pattern in ("*.mp3", "*.wav"):
        root_candidates.extend(
            path for path in material_root.glob(pattern)
            if path.is_file() and not is_generated_audio(path)
        )
    if root_candidates:
        return max(root_candidates, key=lambda path: path.stat().st_mtime)

    nested_candidates = []
    for pattern in ("audio/**/*.mp3", "audio/**/*.wav"):
        nested_candidates.extend(
            path for path in material_root.glob(pattern)
            if path.is_file() and not is_generated_audio(path)
        )
    if not nested_candidates:
        return None
    return max(nested_candidates, key=lambda path: path.stat().st_mtime)


def audio_manifest_expected_duration(material_root: Path) -> int | None:
    manifest_path = material_root / "audio_manifest.json"
    if not manifest_path.exists():
        return None
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8", errors="ignore"))
    except Exception:
        return None
    duration = manifest.get("durationMs")
    return duration if isinstance(duration, int) and duration > 0 else None


def audio_manifest_final_audio(material_root: Path) -> Path | None:
    manifest_path = material_root / "audio_manifest.json"
    if not manifest_path.exists():
        return None
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8", errors="ignore"))
    except Exception:
        return None
    final_audio = manifest.get("finalAudioFile")
    if not isinstance(final_audio, str) or not final_audio.strip():
        return None
    path = Path(final_audio)
    if not path.is_absolute():
        path = material_root / path
    return path if path.is_file() else None


def audio_part_files(material_root: Path) -> list[Path]:
    return sorted(material_root.glob("part_*.mp3"), key=lambda path: path.name)


def concat_audio_parts(parts: list[Path], output: Path) -> Path:
    if not parts:
        raise RuntimeError("No audio part files were found to rebuild the narration source.")
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg was not found, so audio parts cannot be combined.")
    output.parent.mkdir(parents=True, exist_ok=True)
    list_file = output.parent / "concat_video_source.txt"
    lines = []
    for part in parts:
        escaped = part.as_posix().replace("'", "'\\''")
        lines.append(f"file '{escaped}'")
    list_file.write_text("\n".join(lines) + "\n", encoding="utf-8")
    run(
        [
            ffmpeg,
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            str(list_file),
            "-c",
            "copy",
            str(output),
        ]
    )
    return output


def select_narration_source_audio(material_root: Path, preferred: Path) -> tuple[Path, str, int | None]:
    expected_duration_ms = audio_manifest_expected_duration(material_root)
    candidates: list[Path] = []
    manifest_audio = audio_manifest_final_audio(material_root)
    if manifest_audio and manifest_audio not in candidates:
        candidates.append(manifest_audio)
    if preferred.is_file() and preferred not in candidates:
        candidates.append(preferred)
    detected = newest_audio(material_root, {preferred.name})
    if detected and detected not in candidates:
        candidates.append(detected)

    for candidate in candidates:
        try:
            duration_ms = ffprobe_duration_ms(candidate)
        except Exception:
            continue
        if expected_duration_ms is None or abs(duration_ms - expected_duration_ms) <= 5000:
            kind = "manifest_final_audio" if manifest_audio and candidate == manifest_audio else "detected_full_audio"
            return candidate, kind, expected_duration_ms

    parts = audio_part_files(material_root)
    if parts:
        concat_audio_parts(parts, preferred)
        return preferred, "rebuilt_from_audio_parts", expected_duration_ms

    if candidates:
        return candidates[0], "detected_full_audio_without_manifest_match", expected_duration_ms
    raise RuntimeError(f"No full narration audio file was found for video generation: {material_root}")


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
    soft_breaks = "，、：:“”‘’"
    chunks: list[str] = []
    current = ""
    for ch in text:
        current += ch
        if ch in hard_breaks or (ch in soft_breaks and len(current) >= 12):
            chunks.append(clean_subtitle_line(current))
            current = ""
    if current.strip():
        chunks.append(clean_subtitle_line(current))
    lines: list[str] = []
    for chunk in chunks:
        chunk = clean_subtitle_line(chunk)
        if not chunk:
            continue
        if len(chunk) <= max_chars:
            lines.append(chunk)
        else:
            lines.extend(split_long_subtitle_line(chunk, max_chars))
    return merge_short_subtitle_tails([line for line in lines if line], max_chars=max_chars)


def clean_subtitle_line(text: str) -> str:
    return text.strip()


def split_long_subtitle_line(text: str, max_chars: int) -> list[str]:
    chars = list(text)
    lines: list[str] = []
    start = 0
    protected_patterns = ["《天会亮的，你有我呢》", "三十三个四季小故事", "蒲公英", "你有我呢"]
    while start < len(chars):
        if len(chars) - start <= max_chars:
            lines.append(clean_subtitle_line("".join(chars[start:])))
            break
        end = min(start + max_chars, len(chars))
        tail = len(chars) - end
        if 0 < tail < 6:
            end = max(start + 1, len(chars) - 6)
        split_at = None
        for index in range(end - 1, min(start + 10, end) - 1, -1):
            if chars[index] in "\uFF0C\u3001\uFF1A\uFF1B\u3002\uFF01\uFF1F!?;":
                split_at = index + 1
                break
        if split_at is None:
            for index in range(end - 1, min(start + 10, end) - 1, -1):
                if chars[index] in " \u7684\u4E86\u7740\u4E5F\u548C\u4E0E\u5728\u628A\u7ED9\u662F\u6709\u5C31\u90FD\u800C\u4F46\u53EF\u6216\u5E76":
                    split_at = index + 1
                    break
        end = split_at or end
        candidate = "".join(chars[start:end])
        full_text = "".join(chars)
        for pattern in protected_patterns:
            pos = full_text.find(pattern)
            if pos >= 0 and start < pos + len(pattern) and end > pos and end < pos + len(pattern):
                end = pos + len(pattern)
                candidate = "".join(chars[start:end])
                break
        line = clean_subtitle_line(candidate)
        if line:
            lines.append(line)
        start = end
    return lines


def merge_short_subtitle_tails(lines: list[str], min_tail_chars: int = 6, max_chars: int = MAX_SUBTITLE_LINE_CHARS) -> list[str]:
    merged: list[str] = []
    for line in lines:
        if len(line) < min_tail_chars and merged and len(merged[-1]) + len(line) <= max_chars + 4:
            merged[-1] += line
        else:
            merged.append(line)
    return merged



def trim_text_at_sentence_boundary(text: str) -> str:
    text = text.strip()
    if not text:
        return text
    last = -1
    for mark in ("\u3002", "\uFF01", "\uFF1F", ";", "\uFF1B"):
        last = max(last, text.rfind(mark))
    if last >= max(80, len(text) // 2):
        return text[: last + 1].strip()
    return text


def split_text_for_subtitle_batches(text: str, max_chars: int) -> list[str]:
    text = " ".join(text.replace("\r", "\n").split()).strip()
    if not text:
        return []
    batches: list[str] = []
    start = 0
    while start < len(text):
        end = min(start + max_chars, len(text))
        if end < len(text):
            candidate = trim_text_at_sentence_boundary(text[start:end])
            if len(candidate) >= max(120, max_chars // 2):
                end = start + len(candidate)
        chunk = text[start:end].strip()
        if chunk:
            batches.append(chunk)
        start = end
    return batches

def parse_subtitle_lines_from_ai_response(content: str) -> list[str]:
    text = content.strip()
    if text.startswith("```"):
        text = re.sub(r"^```(?:json|text|txt)?\s*", "", text, flags=re.IGNORECASE)
        text = re.sub(r"\s*```$", "", text).strip()
    try:
        parsed = json.loads(text)
        if isinstance(parsed, list):
            return [str(item).strip() for item in parsed if str(item).strip()]
        if isinstance(parsed, dict):
            items = parsed.get("subtitles") or parsed.get("lines")
            if isinstance(items, list):
                return [str(item).strip() for item in items if str(item).strip()]
    except json.JSONDecodeError:
        pass
    lines: list[str] = []
    for line in text.splitlines():
        cleaned = re.sub(r"^\s*[-*]\s*", "", line).strip()
        cleaned = cleaned.strip().strip('"')
        if cleaned:
            lines.append(cleaned)
    return lines


def build_chinese_subtitle_editor_prompt(narration: str, reference_text: str = "") -> str:
    reference_block = ""
    if reference_text.strip():
        reference_block = (
            "\nReference style sample:\n"
            "Only learn its rhythm, punctuation density, and healing layout. Do not copy content unsupported by Input Narration.\n"
            f"{reference_text.strip()[:5000]}\n"
        )
    return (
        "Role:\n"
        "You are a senior Chinese short-video subtitle editor and healing late-night radio copywriter. "
        "You are good at using line breaks to guide reading rhythm, emotional rise and fall, and breathing.\n\n"
        "Task:\n"
        "Convert Input Narration into a polished Chinese subtitle script for video. Do not modify narration.txt on disk; only output subtitles.\n\n"
        "Hard constraints:\n"
        "1. Output plain subtitle text only. No JSON, no Markdown, no timestamps.\n"
        "2. One subtitle per line. Most lines must be 12-20 Chinese characters including punctuation; allow up to 26 when it keeps a phrase natural. Avoid making many 1-10 character fragments.\n"
        "3. Preserve or restore natural Chinese punctuation: \uFF0C\u3002\uFF1F\uFF01\uFF1B\uFF1A\u3001\u300A\u300B\u2026\u2026.\n"
        "4. Never split words, titles, names, fixed phrases, or number expressions, such as \u84B2\u516C\u82F1, \u5B8C\u7F8E, \u4F60\u6709\u6211\u5462, \u4E09\u5341\u4E09\u4E2A\u56DB\u5B63\u5C0F\u6545\u4E8B.\n"
        "5. Generate a polished subtitle script from ONLY the given Input Narration segment. You may lightly condense wording and restore punctuation, but do not continue the story beyond the input, do not add later chapters, and do not invent new examples. Later audio will be generated from this subtitle text.\n"
        "Segmentation logic:\n"
        "1. Prefer breaks at punctuation. If a sentence is too long, split only at semantic pauses.\n"
        "2. Use a gentle healing late-night radio rhythm. Target about 50-65 lines for 1000 Chinese input characters. Prefer fewer, fuller lines over many tiny fragments.\n"
        "3. Avoid 1-10 character fragments unless they intentionally emphasize emotion; merge overly short fragments into nearby lines so the average line length is close to the reference style.\n"
        "4. A comma may stay at the end of a line; split after it only when both sides are meaningful.\n"
        "5. If a tail is too short, merge it with the previous or next line.\n\n"
        "Bad examples:\n"
        "\u5B8C / \u7F8E\n"
        "\u7684\u4E66\n"
        "\u5929\u4F1A\u4EAE\u7684 / \u4F60\u6709\u6211\u5462\n"
        "\u4E09\u5341\u4E09 / \u4E2A\u56DB\u5B63\u5C0F\u6545\u4E8B\n\n"
        "Output example:\n"
        "\u4ECA\u665A\u8981\u4E00\u8D77\u8BFB\u7684\u662F\uFF0C\n"
        "\u4E00\u5E73\u8457\u7ED8\u7684\u300A\u5929\u4F1A\u4EAE\u7684\uFF0C\u4F60\u6709\u6211\u5462\u300B\u3002\n"
        "\u5148\u628A\u706F\u5149\u8C03\u6697\u4E00\u70B9\uFF0C\n"
        "\u628A\u767D\u5929\u6CA1\u6709\u8BF4\u5B8C\u7684\u8BDD\uFF0C\n"
        "\u8F7B\u8F7B\u653E\u5728\u6795\u8FB9\u3002\n"
        f"{reference_block}\n"
        "Stop exactly when the Input Narration segment ends. Do not write any subtitle about content that is not present in the input segment.\n\n"
        "Input Narration:\n"
        f"{narration.strip()}"
    )


def generate_chinese_subtitles_with_ai(
    material_root: Path,
    reference_file: Path | None = None,
    max_input_chars: int = 0,
    batch_chars: int = 0,
) -> tuple[list[str], dict]:
    narration = read_text(material_root / "narration.txt")
    if not narration:
        raise RuntimeError(f"narration.txt not found or empty: {material_root / 'narration.txt'}")
    source_narration_chars = len(narration)
    if max_input_chars and len(narration) > max_input_chars:
        narration = trim_text_at_sentence_boundary(narration[:max_input_chars])
    base_url = os.environ.get("ABOOK_AI_BASE_URL", "").strip()
    api_key = os.environ.get("ABOOK_AI_API_KEY", "").strip()
    model = os.environ.get("ABOOK_AI_MODEL", "").strip()
    if not (base_url and api_key and model):
        raise RuntimeError("Chinese subtitle generation requires ABOOK_AI_BASE_URL, ABOOK_AI_API_KEY and ABOOK_AI_MODEL.")
    reference_text = read_text(reference_file) if reference_file else ""
    batch_list = split_text_for_subtitle_batches(narration, batch_chars) if batch_chars else [narration]
    lines: list[str] = []
    batch_reports: list[dict] = []
    for batch_index, batch_text in enumerate(batch_list, start=1):
        prompt = build_chinese_subtitle_editor_prompt(batch_text, reference_text if batch_index == 1 else "")
        content = chat_completion_text(
            base_url,
            api_key,
            model,
            [
                {"role": "system", "content": "You are a professional Chinese subtitle editor. Output plain subtitle lines only, no explanation."},
                {"role": "user", "content": prompt},
            ],
            timeout=900,
        )
        batch_lines = parse_subtitle_lines_from_ai_response(content)
        lines.extend(batch_lines)
        batch_reports.append({"batch": batch_index, "inputChars": len(batch_text), "lineCount": len(batch_lines)})
        print(
            f"Generated subtitle batch {batch_index}/{len(batch_list)}: input={len(batch_text)} lines={len(batch_lines)}",
            file=sys.stderr,
            flush=True,
        )
    if len(lines) < 10:
        raise RuntimeError(f"AI returned too few subtitle lines: {len(lines)}")
    report = {
        "provider": "openai_compatible",
        "model": model,
        "lineCount": len(lines),
        "narrationChars": len(narration),
        "sourceNarrationChars": source_narration_chars,
        "maxInputChars": max_input_chars,
        "batchChars": batch_chars,
        "batchCount": len(batch_list),
        "batches": batch_reports,
        "generatedAt": time.strftime("%Y-%m-%d %H:%M:%S"),
        "referenceFile": str(reference_file) if reference_file else None,
    }
    if reference_text.strip():
        generated_joined = "\n".join(lines)
        matcher = difflib.SequenceMatcher(None, generated_joined, reference_text.strip())
        report["referenceSimilarity"] = matcher.ratio()
        report["referenceLineCount"] = len([line for line in reference_text.splitlines() if line.strip()])
    return lines, report
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


def find_timed_chinese_srt(material_root: Path, video_dir: Path, audio_language: str = "cmn") -> Path | None:
    candidates: list[Path] = []
    manifest_path = video_dir / "pipeline_manifest.json"
    if manifest_path.exists():
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8", errors="ignore"))
            subtitle_manifest = manifest.get("subtitleManifest") if isinstance(manifest.get("subtitleManifest"), dict) else {}
            for key in ("singleLanguageSrt", "outputSrt"):
                value = subtitle_manifest.get(key)
                if isinstance(value, str) and value.strip():
                    candidates.append(Path(value))
        except Exception:
            pass
    names = [
        f"hard_subtitle.aeneas.{audio_language}.srt",
        "hard_subtitle.aeneas.cmn.srt",
        "hard_subtitle.aeneas.chn.srt",
        "hard_subtitle.aeneas.zh.srt",
    ]
    for root in (video_dir, material_root):
        candidates.extend(root / name for name in names)
    seen: set[str] = set()
    for candidate in candidates:
        key = str(candidate)
        if key in seen:
            continue
        seen.add(key)
        if candidate.is_file() and read_srt_events(candidate):
            return candidate
    return None


def offset_events(events: list[tuple[int, int, str]], offset_ms: int) -> list[tuple[int, int, str]]:
    return [(start + offset_ms, end + offset_ms, text) for start, end, text in events]


def offset_events_for_header_once(
    events: list[tuple[int, int, str]], header_ms: int
) -> tuple[list[tuple[int, int, str]], int]:
    if not events:
        return events, 0
    first_start = min(start for start, _, _ in events)
    if first_start >= header_ms - 500:
        return events, 0
    return offset_events(events, header_ms), header_ms


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


def write_lrc(path: Path, events: list[tuple[int, int, str]]) -> None:
    lines = []
    for start, _, text in events:
        minutes = start // 60_000
        seconds = (start % 60_000) // 1000
        centis = (start % 1000) // 10
        body = " / ".join(part.strip() for part in text.replace("\\N", "\n").splitlines() if part.strip())
        lines.append(f"[{minutes:02d}:{seconds:02d}.{centis:02d}]{body}")
    path.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")


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


def normalize_chat_base_url(base_url: str) -> str:
    base = base_url.strip().rstrip("/")
    if not base:
        return ""
    if base.endswith("/chat/completions"):
        return base
    return f"{base}/chat/completions"


def parse_json_array_from_ai_response(content: str) -> list[str]:
    text = content.strip()
    if text.startswith("```"):
        text = re.sub(r"^```(?:json)?\s*", "", text, flags=re.IGNORECASE)
        text = re.sub(r"\s*```$", "", text).strip()
    match = re.search(r"\[[\s\S]*\]", text)
    if match:
        text = match.group(0)
    data = json.loads(text)
    if not isinstance(data, list):
        raise RuntimeError("AI subtitle translation response must be a JSON array.")
    return [str(item).strip() for item in data]


def chat_completion_text(base_url: str, api_key: str, model: str, messages: list[dict], timeout: int = 600) -> str:
    url = normalize_chat_base_url(base_url)
    if not url or not api_key.strip() or not model.strip():
        raise RuntimeError("AI translation requires ABOOK_AI_BASE_URL, ABOOK_AI_API_KEY and ABOOK_AI_MODEL.")
    payload = json.dumps(
        {
            "model": model.strip(),
            "messages": messages,
            "temperature": 0.2,
            "stream": False,
        },
        ensure_ascii=False,
    ).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=payload,
        headers={
            "Authorization": f"Bearer {api_key.strip()}",
            "Content-Type": "application/json; charset=utf-8",
            "Accept": "application/json",
            "User-Agent": "A-Book-in-30-Minutes/0.1 Subtitle-Translator",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            response_text = response.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"AI subtitle translation failed: HTTP {exc.code} {detail[:1000]}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"AI subtitle translation failed: {exc}") from exc
    data = json.loads(response_text)
    choices = data.get("choices") if isinstance(data, dict) else None
    if not choices:
        raise RuntimeError(f"AI subtitle translation returned no choices: {response_text[:1000]}")
    message = choices[0].get("message") if isinstance(choices[0], dict) else None
    content = message.get("content") if isinstance(message, dict) else None
    if isinstance(content, list):
        content = "".join(str(item.get("text") if isinstance(item, dict) else item) for item in content)
    if not str(content or "").strip():
        raise RuntimeError(f"AI subtitle translation returned empty content: {response_text[:1000]}")
    return str(content)


def generate_english_lines_with_ai(material_root: Path, source_lines: list[str]) -> list[str]:
    base_url = os.environ.get("ABOOK_AI_BASE_URL", "").strip()
    api_key = os.environ.get("ABOOK_AI_API_KEY", "").strip()
    model = os.environ.get("ABOOK_AI_MODEL", "").strip()
    if not (base_url and api_key and model):
        raise RuntimeError(
            "English subtitles are required after aeneas alignment. "
            "No valid subtitles_en.json/translation_cache.json/existing bilingual ASS was found, "
            "and AI translation is not configured. Set ABOOK_AI_BASE_URL, ABOOK_AI_API_KEY and "
            "ABOOK_AI_MODEL, or generate translation_cache.json with Codex/AI first."
        )

    system_prompt = (
        "You are Codex translating subtitle cues for a bilingual listening-video. "
        "Translate each source cue idiomatically into natural English. Preserve cue count and order. "
        "Return only a JSON array of strings, with no markdown and no extra keys."
    )
    cache_file = material_root / "translation_cache.json"
    partial_cache_file = material_root / "translation_cache.partial.json"
    translated: list[str] = []
    if partial_cache_file.exists():
        try:
            partial = json.loads(partial_cache_file.read_text(encoding="utf-8"))
            items = partial.get("items") if isinstance(partial, dict) else None
            if isinstance(items, list):
                for item in items:
                    if isinstance(item, dict):
                        translated.append(str(item.get("en") or "").strip())
        except json.JSONDecodeError:
            translated = []

    def write_translation_cache(path: Path, completed: bool) -> None:
        cache = {
            "provider": "openai_compatible",
            "model": model,
            "sourceLanguage": os.environ.get("ABOOK_SUBTITLE_SOURCE_LANGUAGE", "cmn"),
            "targetLanguage": "eng",
            "generatedAt": time.strftime("%Y-%m-%d %H:%M:%S"),
            "completed": completed,
            "total": len(source_lines),
            "items": [
                {"index": index + 1, "source": source, "en": english}
                for index, (source, english) in enumerate(zip(source_lines, translated))
            ],
        }
        path.write_text(json.dumps(cache, ensure_ascii=False, indent=2), encoding="utf-8")

    def translate_batch(batch: list[str], start_index: int) -> list[str]:
        user_prompt = (
            f"Translate these subtitle cues into English. Return exactly {len(batch)} strings as JSON.\n"
            + json.dumps(batch, ensure_ascii=False)
        )
        content = chat_completion_text(
            base_url,
            api_key,
            model,
            [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
        )
        batch_lines = parse_json_array_from_ai_response(content)
        if len(batch_lines) == len(batch) and all(batch_lines):
            return batch_lines
        if len(batch) == 1:
            fallback = str(batch_lines[0]).strip() if batch_lines else ""
            return [fallback or f"Subtitle cue {start_index + 1}."]
        repaired: list[str] = []
        for offset, source in enumerate(batch):
            repaired.extend(translate_batch([source], start_index + offset))
        return repaired

    batch_size = int(os.environ.get("ABOOK_TRANSLATE_BATCH_SIZE", "20") or "20")
    batch_size = max(1, min(batch_size, 50))
    resume_index = len(translated)
    if resume_index > len(source_lines):
        translated = translated[: len(source_lines)]
        resume_index = len(translated)
    if resume_index:
        print(f"Resuming subtitle translation from {resume_index}/{len(source_lines)}.", file=sys.stderr, flush=True)
    for start in range(resume_index, len(source_lines), batch_size):
        batch = source_lines[start : start + batch_size]
        translated.extend(translate_batch(batch, start))
        write_translation_cache(partial_cache_file, completed=False)
        print(f"Translated subtitle cues {len(translated)}/{len(source_lines)}.", file=sys.stderr, flush=True)

    write_translation_cache(cache_file, completed=True)
    if partial_cache_file.exists():
        partial_cache_file.unlink()
    return translated


def load_english_lines(material_root: Path, expected_count: int, source_lines: list[str] | None = None) -> list[str]:
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

    existing_ass = find_existing_aeneas_ass(material_root)
    if existing_ass:
        lines = read_ass_style_lines(existing_ass, "English")
        if len(lines) == expected_count and all(lines):
            return lines

    if source_lines is not None:
        return generate_english_lines_with_ai(material_root, source_lines)

    raise RuntimeError("English subtitles are required after aeneas alignment.")


def run_aeneas_alignment(audio: Path, subtitle_lines: list[str], output_dir: Path, audio_language: str) -> tuple[Path, dict]:
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
    try:
        from aeneas.executetask import ExecuteTask
        from aeneas.task import Task
    except Exception as exc:
        run_aeneas_alignment_subprocess(audio, text_file, srt_file, config, exc)
        events = read_srt_events(srt_file)
        return srt_file, write_aeneas_manifest(
            output_dir, audio, text_file, srt_file, events, language, len(subtitle_lines)
        )

    task = Task(config_string=config)
    task.audio_file_path_absolute = str(audio)
    task.text_file_path_absolute = str(text_file)
    task.sync_map_file_path_absolute = str(srt_file)
    ExecuteTask(task).execute()
    task.output_sync_map_file()
    events = read_srt_events(srt_file)
    return srt_file, write_aeneas_manifest(
        output_dir, audio, text_file, srt_file, events, language, len(subtitle_lines)
    )


def run_aeneas_alignment_subprocess(
    audio: Path,
    text_file: Path,
    srt_file: Path,
    config: str,
    import_error: Exception,
) -> None:
    candidates = []
    env_python = os.environ.get("AENEAS_PYTHON")
    if env_python:
        candidates.append(Path(env_python))
    candidates.extend(
        [
            Path(r"C:\Program Files\Python39\python.exe"),
            Path(r"C:\Program Files (x86)\Python39\python.exe"),
        ]
    )
    for python_exe in candidates:
        if not python_exe.is_file() or Path(sys.executable).resolve() == python_exe.resolve():
            continue
        completed = subprocess.run(
            [
                str(python_exe),
                "-m",
                "aeneas.tools.execute_task",
                str(audio),
                str(text_file),
                config,
                str(srt_file),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="ignore",
        )
        if completed.returncode == 0 and srt_file.is_file():
            return
    raise RuntimeError(
        "aeneas.tools is required for final subtitle timing. "
        "Current Python cannot import aeneas, and no working AENEAS_PYTHON/Python39 fallback completed."
    ) from import_error


def write_aeneas_manifest(
    output_dir: Path,
    audio: Path,
    text_file: Path,
    srt_file: Path,
    events: list[tuple[int, int, str]],
    language: str,
    expected_count: int,
) -> dict:
    if len(events) != expected_count:
        raise RuntimeError(
            f"aeneas cue count mismatch: expected {expected_count}, got {len(events)} from {srt_file}"
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
    return manifest


def build_aeneas_subtitles(
    material_root: Path,
    audio: Path,
    video_dir: Path,
    audio_language: str,
    force_aeneas: bool,
    subtitle_offset_ms: int,
) -> tuple[Path, Path, list[tuple[int, int, str]], dict]:
    existing_ass = None if force_aeneas else find_existing_aeneas_ass(material_root)
    if existing_ass:
        ass_file = video_dir / "hard_subtitle.aeneas.zh-en.ass"
        raw_events = read_ass_dialogue_events(existing_ass)
        events, delay_ms = offset_events_for_header_once(raw_events, subtitle_offset_ms)
        write_ass(ass_file, events)
        srt_file = video_dir / "hard_subtitle.aeneas.zh-en.srt"
        lrc_file = video_dir / "hard_subtitle.aeneas.zh-en.lrc"
        write_srt(srt_file, events)
        write_lrc(lrc_file, events)
        return ass_file, srt_file, events, {
            "subtitleTiming": "existing_aeneas_ass",
            "sourceAss": str(existing_ass),
            "cueCount": len(events),
            "sourceFirstCueMs": raw_events[0][0] if raw_events else None,
            "firstCueMs": events[0][0] if events else None,
            "delayMs": delay_ms,
            "zhEnLrc": str(lrc_file),
        }

    chinese_lines = load_chinese_subtitle_lines(material_root)
    if not chinese_lines:
        raise RuntimeError("No Chinese subtitle lines found for aeneas alignment.")
    aeneas_dir = video_dir
    zh_srt, subtitle_manifest = run_aeneas_alignment(audio, chinese_lines, aeneas_dir, audio_language)
    zh_events = read_srt_events(zh_srt)
    aligned_events = offset_events(zh_events, subtitle_offset_ms) if subtitle_offset_ms else zh_events
    single_srt_file = video_dir / f"hard_subtitle.aeneas.{audio_language}.srt"
    single_lrc_file = video_dir / f"hard_subtitle.aeneas.{audio_language}.lrc"
    write_srt(single_srt_file, aligned_events)
    write_lrc(single_lrc_file, aligned_events)
    english_lines = load_english_lines(material_root, len(zh_events), chinese_lines)
    bilingual_events = [
        (start + subtitle_offset_ms, end + subtitle_offset_ms, f"{zh}\n{en}")
        for (start, end, zh), en in zip(zh_events, english_lines)
    ]
    srt_file = video_dir / "hard_subtitle.aeneas.zh-en.srt"
    ass_file = video_dir / "hard_subtitle.aeneas.zh-en.ass"
    lrc_file = video_dir / "hard_subtitle.aeneas.zh-en.lrc"
    write_srt(srt_file, bilingual_events)
    write_ass(ass_file, bilingual_events)
    write_lrc(lrc_file, bilingual_events)
    subtitle_manifest = {
        **subtitle_manifest,
        "subtitleTiming": "aeneas",
        "sourceLanguage": audio_language,
        "singleLanguageSrt": str(single_srt_file),
        "singleLanguageLrc": str(single_lrc_file),
        "zhEnSrt": str(srt_file),
        "zhEnAss": str(ass_file),
        "zhEnLrc": str(lrc_file),
        "subtitleOffsetMs": subtitle_offset_ms,
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


def read_ass_style_lines(path: Path, style: str) -> list[str]:
    lines: list[str] = []
    marker = f",{style},"
    for line in read_text(path).splitlines():
        if not line.startswith("Dialogue:") or marker not in line:
            continue
        parts = line.split(",", 9)
        if len(parts) < 10:
            continue
        lines.append(parts[9].replace("\\N", " ").strip())
    return lines


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


def default_header_audio_path() -> Path:
    return Path(__file__).resolve().parent / "assets" / "header.mp3"


def ensure_header_audio(path: Path, seconds: int = HEADER_SECONDS) -> Path:
    if path.exists() and path.stat().st_size > 0 and ffprobe_duration_ms(path) == HEADER_AUDIO_DURATION_MS:
        return path
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg was not found, so header.mp3 cannot be generated.")
    path.parent.mkdir(parents=True, exist_ok=True)
    run(
        [
            ffmpeg,
            "-y",
            "-f",
            "lavfi",
            "-i",
            f"anullsrc=r=48000:cl=stereo",
            "-t",
            f"{HEADER_AUDIO_ENCODE_SECONDS:.3f}",
            "-c:a",
            "libmp3lame",
            "-b:a",
            "160k",
            str(path),
        ]
    )
    return path


def prepend_header_audio(header: Path, narration: Path, output: Path) -> Path:
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg was not found, so audio cannot be combined.")
    output.parent.mkdir(parents=True, exist_ok=True)
    actual_output = output
    replace_output = False
    try:
        if narration.resolve() == output.resolve():
            actual_output = output.with_name(f"{output.stem}.tmp{output.suffix}")
            replace_output = True
    except OSError:
        pass
    run(
        [
            ffmpeg,
            "-y",
            "-i",
            str(header),
            "-i",
            str(narration),
            "-filter_complex",
            "[0:a]aresample=48000,apad=whole_dur=3.000,atrim=duration=3.000,asetpts=PTS-STARTPTS[h];"
            "[1:a]aresample=48000,asetpts=PTS-STARTPTS[n];"
            "[h][n]concat=n=2:v=0:a=1[a]",
            "-map",
            "[a]",
            "-c:a",
            "libmp3lame",
            "-b:a",
            "160k",
            str(actual_output),
        ]
    )
    if replace_output:
        actual_output.replace(output)
    return output


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


def youtube_thumbnail_lines(title: str, subtitle: str = "") -> list[str]:
    book_title = extract_book_title(title)
    if "亲爱的老爸" in book_title or "亲爱的老爸" in title:
        return ["海明威写给", "儿子的信"]
    if book_title and book_title != "本期书籍":
        compact = compact_text(book_title, 12)
        if len(compact) <= 6:
            return [compact, "一本书听懂"]
        return wrap_text(compact, 6, 2).splitlines()
    fallback = compact_text(clean_label(subtitle, "半小时听懂一本书"), 12)
    return wrap_text(fallback, 6, 2).splitlines()


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


def draw_centered_label(
    draw: ImageDraw.ImageDraw,
    box: tuple[int, int, int, int],
    text: str,
    font: ImageFont.ImageFont,
    fill: tuple[int, int, int] | tuple[int, int, int, int],
    text_fill: tuple[int, int, int] | tuple[int, int, int, int],
    outline: tuple[int, int, int] | tuple[int, int, int, int] | None = None,
    radius: int = 8,
    width: int = 2,
    stroke_width: int = 0,
    stroke_fill: tuple[int, int, int] | tuple[int, int, int, int] | None = None,
) -> tuple[int, int, int, int]:
    draw.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)
    bbox = draw.textbbox((0, 0), text, font=font, stroke_width=stroke_width)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    x = box[0] + (box[2] - box[0] - text_width) // 2 - bbox[0]
    y = box[1] + (box[3] - box[1] - text_height) // 2 - bbox[1]
    draw.text(
        (x, y),
        text,
        font=font,
        fill=text_fill,
        stroke_width=stroke_width,
        stroke_fill=stroke_fill,
    )
    return box


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


def target_visual_scene_count(material_root: Path, subtitle_events: list[tuple[int, int, str]] | None = None) -> int:
    if subtitle_events:
        source_count = len([event for event in subtitle_events if event[1] > HEADER_AUDIO_DURATION_MS])
    else:
        source_count = len(load_chinese_subtitle_lines(material_root))
    estimated = (max(1, source_count) + VISUAL_SUBTITLE_LINES_PER_IMAGE - 1) // VISUAL_SUBTITLE_LINES_PER_IMAGE
    return max(VISUAL_SCENE_MIN_COUNT, min(VISUAL_SCENE_MAX_COUNT, estimated))


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
        return [], None, "none"

    dest_dir = video_dir
    dest_dir.mkdir(parents=True, exist_ok=True)
    copied: list[Path] = []
    for index, source in enumerate(image_candidates(source_dir)[:VISUAL_SCENE_MAX_COUNT], 1):
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


def generate_controlled_programmatic_assets(
    epub: Path,
    material_root: Path,
    video_dir: Path,
    subtitle_events: list[tuple[int, int, str]] | None = None,
) -> tuple[list[Path], Path | None, str]:
    material = read_material_json(material_root)
    title = str(material.get("videoTitle") or material.get("title") or epub.stem)
    description = str(material.get("description") or "")
    events = subtitle_events or []
    if not events:
        duration_ms = audio_manifest_expected_duration(material_root) or 30 * 60 * 1000
        subtitle_lines = load_chinese_subtitle_lines(material_root)
        events = build_subtitle_events(subtitle_lines, duration_ms, HEADER_AUDIO_DURATION_MS) if subtitle_lines else []
    return generate_whiteboard_skill_assets(
        video_dir,
        material_root,
        title,
        description,
        events,
    )


def build_whiteboard_skill_prompts(
    material_root: Path,
    title: str,
    description: str,
    subtitle_events: list[tuple[int, int, str]],
    scene_count: int | None = None,
) -> list[str]:
    scene_count = scene_count or target_visual_scene_count(material_root, subtitle_events)
    prompt_style = os.environ.get("BOOK_IMAGE_PROMPT_STYLE", "book-illustration").strip().lower()
    clean_lines = load_chinese_subtitle_lines(material_root)
    clean_groups: list[str] = []
    if clean_lines:
        cursor = 0
        for index in range(scene_count):
            remaining_scenes = scene_count - index
            remaining_lines = len(clean_lines) - cursor
            take = max(1, round(remaining_lines / remaining_scenes)) if remaining_scenes else remaining_lines
            group = clean_lines[cursor : cursor + take]
            cursor += take
            clean_groups.append(" ".join(group))

    body_events = [(start, end, text) for start, end, text in subtitle_events if end > HEADER_AUDIO_DURATION_MS]
    if not body_events:
        source = description or title
        return [
            f"{WHITEBOARD_PROMPT_PREFIX}\nBook: {title}\nScene {index}: {compact_text(clean_groups[index - 1] if index - 1 < len(clean_groups) else source, 260)}"
            for index in range(1, scene_count + 1)
        ]

    prompts = []
    cursor = 0
    for index in range(scene_count):
        remaining_scenes = scene_count - index
        remaining_events = len(body_events) - cursor
        take = max(1, round(remaining_events / remaining_scenes)) if remaining_scenes else remaining_events
        group = body_events[cursor : cursor + take]
        cursor += take
        source_text = clean_groups[index] if index < len(clean_groups) else ""
        if not source_text:
            source_text = " ".join(text.splitlines()[0].strip() for _, _, text in group if text.strip())
        source_text = source_text or description or title
        if prompt_style == "book-illustration":
            prompts.append(
                "\n".join(
                    [
                        "Professional editorial illustration for a 30-minute book summary video.",
                        "Style: warm hand-painted storybook illustration, clean cinematic composition, rich but readable details, soft daylight, gentle paper texture, hopeful serious mood.",
                        "Use one consistent visual language across the whole series: muted warm earth colors, cream paper highlights, charcoal line accents, subtle teal and rust accents.",
                        "Subject context: South Africa in the late 20th century, public truth, reconciliation, forgiveness, families rebuilding trust after political violence.",
                        f"Book: {title}",
                        f"Scene {index + 1} of {scene_count}.",
                        f"Chinese subtitle text for this time range: {compact_text(source_text, 520)}",
                        "Create a concrete scene, not an icon: include human figures with natural poses, room or outdoor setting, everyday objects, weather or nature details when appropriate.",
                        "Prefer mid-shot or wide-shot storytelling over close-up symbols. Show story, relationship, tension, and repair.",
                        "No readable text, no subtitles, no signs with words, no watermark, no logo, no duplicate main character, no abstract single-object icon.",
                    ]
                )
            )
            continue
        prompts.append(
            "\n".join(
                [
                    WHITEBOARD_PROMPT_PREFIX,
                    f"Book: {title}",
                    f"Scene {index + 1} of {scene_count}.",
                    f"Chinese subtitle text for this time range: {compact_text(source_text, 420)}",
                    "Create a concrete symbolic image that matches the subtitle content above.",
                    "Use recurring motifs across the series so the viewer feels visual continuity.",
                    "Do not draw readable text; use simple shapes, arrows, people, envelopes, books, tea leaves, roads, windows, or light as symbolic motifs when appropriate.",
                ]
            )
        )
    return prompts


def run_whiteboard_image_skill(prompts: list[str], output_dir: Path) -> list[Path]:
    if not WHITEBOARD_IMAGE_GENERATOR.is_file():
        raise RuntimeError(f"whiteboard image generator was not found: {WHITEBOARD_IMAGE_GENERATOR}")
    output_dir.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    if env.get("ABOOK_AI_BASE_URL") and not env.get("OPENAI_API_BASE"):
        env["OPENAI_API_BASE"] = env["ABOOK_AI_BASE_URL"]
    if env.get("ABOOK_AI_API_KEY") and not env.get("OPENAI_API_KEY"):
        env["OPENAI_API_KEY"] = env["ABOOK_AI_API_KEY"]
    env.setdefault("OPENAI_IMAGE_MODE", "macmini-realistic")
    env.setdefault("MACMINI_IMAGE_ENDPOINT", "http://100.96.199.26:30020/v1/images/generations")
    image_mode = env.get("OPENAI_IMAGE_MODE", "").strip().lower()
    if image_mode == "macmini-realistic":
        # Text models such as gpt-5.5 are not valid Hugging Face image models.
        # The image skill has its own .env, so pass a valid MacMini model explicitly.
        env["OPENAI_IMAGE_MODEL"] = (
            env.get("MACMINI_IMAGE_MODEL")
            or env.get("BOOK_IMAGE_MODEL")
            or "SG161222/Realistic_Vision_V5.1_noVAE"
        )
    elif env.get("ABOOK_AI_MODEL") and not env.get("OPENAI_IMAGE_MODEL"):
        env["OPENAI_IMAGE_MODEL"] = env["ABOOK_AI_MODEL"]
    batch_size = max(1, int(os.environ.get("ABOOK_IMAGE_PROMPT_BATCH_SIZE", "4") or "4"))
    images: list[Path] = []
    for batch_start in range(0, len(prompts), batch_size):
        batch = prompts[batch_start : batch_start + batch_size]
        completed = subprocess.run(
            [
                sys.executable,
                str(WHITEBOARD_IMAGE_GENERATOR),
                json.dumps(batch, ensure_ascii=False),
                "16:9",
                str(output_dir),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=env,
        )
        if completed.returncode != 0:
            raise RuntimeError(
                "whiteboard image skill failed for batch {}-{}:\nstdout:\n{}\n\nstderr:\n{}".format(
                    batch_start + 1,
                    batch_start + len(batch),
                    completed.stdout[-4000:],
                    completed.stderr[-4000:],
                )
            )
        match = re.search(r"__RESULTS__(\[.*\])", completed.stdout, flags=re.DOTALL)
        if not match:
            raise RuntimeError(f"whiteboard image skill did not report results:\n{completed.stdout[-4000:]}")
        results = json.loads(match.group(1))
        images.extend(Path(str(item)) for item in results if isinstance(item, str) and Path(str(item)).is_file())
    if len(images) != len(prompts):
        generated = sorted(
            output_dir.glob("*.png"),
            key=lambda path: path.stat().st_mtime,
            reverse=True,
        )
        images = list(reversed(generated[: len(prompts)]))
    if len(images) != len(prompts):
        raise RuntimeError(f"whiteboard image skill generated {len(images)} images for {len(prompts)} prompts.")
    return images


def qwen_image_workflow(prompt: str, *, width: int, height: int, steps: int, seed: int, prefix: str) -> dict:
    negative = os.environ.get(
        "QWEN_IMAGE_NEGATIVE_PROMPT",
        "low quality, blurry, distorted, bad anatomy, oversaturated, text artifacts, watermark, logo, unreadable text, messy composition",
    )
    return {
        "1": {"class_type": "UnetLoaderGGUF", "inputs": {"unet_name": "qwen-image-2512-Q2_K.gguf"}},
        "2": {"class_type": "CLIPLoader", "inputs": {"clip_name": "qwen_2.5_vl_7b_fp8_scaled.safetensors", "type": "qwen_image", "device": "default"}},
        "3": {"class_type": "VAELoader", "inputs": {"vae_name": "qwen_image_vae.safetensors"}},
        "4": {"class_type": "ModelSamplingAuraFlow", "inputs": {"model": ["1", 0], "shift": 3.1}},
        "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["2", 0], "text": prompt}},
        "6": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["2", 0], "text": negative}},
        "7": {"class_type": "EmptySD3LatentImage", "inputs": {"width": width, "height": height, "batch_size": 1}},
        "8": {"class_type": "KSampler", "inputs": {"model": ["4", 0], "positive": ["5", 0], "negative": ["6", 0], "latent_image": ["7", 0], "seed": seed, "steps": steps, "cfg": 4.0, "sampler_name": "euler", "scheduler": "simple", "denoise": 1.0}},
        "9": {"class_type": "VAEDecode", "inputs": {"samples": ["8", 0], "vae": ["3", 0]}},
        "10": {"class_type": "SaveImage", "inputs": {"images": ["9", 0], "filename_prefix": prefix}},
    }


def qwen_image_request_json(url: str, payload: dict | None = None, timeout: int = 600) -> dict:
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8", errors="replace"))


def generate_qwen_image_assets(
    video_dir: Path,
    material_root: Path,
    title: str,
    description: str,
    subtitle_events: list[tuple[int, int, str]],
) -> tuple[list[Path], Path, str]:
    base_url = os.environ.get("QWEN_IMAGE_BASE_URL", "http://100.96.199.26:8188").rstrip("/")
    width = int(os.environ.get("QWEN_IMAGE_WIDTH", "1024") or "1024")
    height = int(os.environ.get("QWEN_IMAGE_HEIGHT", "576") or "576")
    steps = int(os.environ.get("QWEN_IMAGE_STEPS", "8") or "8")
    request_timeout = int(os.environ.get("QWEN_IMAGE_REQUEST_TIMEOUT_SECONDS", "600") or "600")
    poll_seconds = int(os.environ.get("QWEN_IMAGE_POLL_SECONDS", "10") or "10")
    max_wait_seconds = int(os.environ.get("QWEN_IMAGE_MAX_WAIT_SECONDS", str(2 * 60 * 60)) or str(2 * 60 * 60))
    max_polls = max(1, max_wait_seconds // max(1, poll_seconds))
    image_dir = video_dir / "qwen_image_2512"
    image_dir.mkdir(parents=True, exist_ok=True)
    prompts = build_whiteboard_skill_prompts(
        material_root,
        title,
        description,
        subtitle_events,
        target_visual_scene_count(material_root, subtitle_events),
    )
    copied: list[Path] = []
    for index, prompt in enumerate(prompts, 1):
        prefix = f"qwen_image_2512_{index:02d}"
        seed = int(os.environ.get("QWEN_IMAGE_SEED", "20260705") or "20260705") + index
        workflow = qwen_image_workflow(prompt, width=width, height=height, steps=steps, seed=seed, prefix=prefix)
        print(
            f"Qwen Image queue {index}/{len(prompts)} via {base_url} ({width}x{height}, steps={steps}, seed={seed})",
            file=sys.stderr,
            flush=True,
        )
        queued = qwen_image_request_json(
            f"{base_url}/prompt",
            {"prompt": workflow, "client_id": str(uuid.uuid4())},
            timeout=request_timeout,
        )
        prompt_id = queued.get("prompt_id")
        if not prompt_id:
            raise RuntimeError(f"Qwen Image did not return prompt_id: {queued}")
        info = None
        started_at = time.time()
        for _ in range(max_polls):
            history = qwen_image_request_json(f"{base_url}/history/{prompt_id}", timeout=request_timeout)
            if prompt_id in history:
                outputs = history[prompt_id].get("outputs", {})
                images = outputs.get("10", {}).get("images", [])
                if not images:
                    raise RuntimeError(f"Qwen Image finished without image output: {history[prompt_id]}")
                info = images[0]
                break
            waited = int(time.time() - started_at)
            print(
                f"Qwen Image waiting {index}/{len(prompts)} prompt_id={prompt_id} waited={waited}s",
                file=sys.stderr,
                flush=True,
            )
            time.sleep(poll_seconds)
        if info is None:
            raise TimeoutError(f"Timed out after {max_wait_seconds}s waiting for Qwen Image prompt {prompt_id}")
        params = urllib.parse.urlencode(
            {
                "filename": info["filename"],
                "subfolder": info.get("subfolder", ""),
                "type": info.get("type", "output"),
            }
        )
        raw = urllib.request.urlopen(f"{base_url}/view?{params}", timeout=request_timeout).read()
        source = image_dir / f"{prefix}.png"
        source.write_bytes(raw)
        assert_meaningful_image(source)
        dest = video_dir / f"visual_{index:02d}_qwen_image.png"
        with Image.open(source) as image:
            image.convert("RGB").resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS).save(dest, quality=94)
        copied.append(dest)
        print(f"Qwen Image generated {index}/{len(prompts)}: {dest}", file=sys.stderr, flush=True)
    manifest = {
        "sourceKind": "qwen_image_2512",
        "baseUrl": base_url,
        "model": "qwen-image-2512-Q2_K.gguf",
        "width": width,
        "height": height,
        "steps": steps,
        "generatedAt": time.strftime("%Y-%m-%d %H:%M:%S"),
        "promptCount": len(prompts),
        "assets": [str(path) for path in copied],
        "prompts": prompts,
    }
    (video_dir / "qwen_image_2512_manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    (video_dir / "visual_assets_manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    return copied, image_dir, "qwen_image_2512"


def normalize_whiteboard_palette(image: Image.Image) -> Image.Image:
    converted = image.convert("RGB")
    pixels = converted.load()
    width, height = converted.size
    for y in range(height):
        for x in range(width):
            r, g, b = pixels[x, y]
            if r > 225 and g > 220 and b > 210:
                pixels[x, y] = (246, 241, 227)
                continue
            if b > r + 20 or g > r + 25:
                pixels[x, y] = (205, 100, 65)
                continue
            if max(r, g, b) - min(r, g, b) > 30 and max(r, g, b) > 90:
                pixels[x, y] = (205, 100, 65)
                continue
            if max(r, g, b) < 90:
                pixels[x, y] = (45, 48, 50)
    return converted


def assert_meaningful_image(path: Path) -> None:
    with Image.open(path) as image:
        small = image.convert("RGB").resize((256, 171), Image.Resampling.BILINEAR)
        colors = small.getcolors(maxcolors=65536) or []
        non_bg = 0
        for count, (r, g, b) in colors:
            if not (r > 225 and g > 220 and b > 210):
                non_bg += count
        non_bg_ratio = non_bg / max(1, small.width * small.height)
        color_count = len(colors)
    size_kb = path.stat().st_size / 1024
    if size_kb < 80 and (color_count < 256 or non_bg_ratio < 0.16):
        raise RuntimeError(
            f"Generated image is a low-detail line/placeholder image, not a usable scene illustration: {path} "
            f"(sizeKB={size_kb:.1f}, colors={color_count}, nonBgRatio={non_bg_ratio:.3f})"
        )


def whiteboard_scene_specs() -> list[dict]:
    return [
        {
            "theme": "文具店、信纸、重新开门",
            "motifs": ["shop", "letter", "family"],
            "visualBrief": "A quiet stationery shop desk, an opening envelope, and a small family standing behind it.",
        },
        {
            "theme": "夏夜烟火、青春期的半掩房门",
            "motifs": ["firework", "door", "phone"],
            "visualBrief": "A night yard with fading sparklers, a water bucket, and a half-closed teenager's door.",
        },
        {
            "theme": "独处角落、热红酒、祖母旧信",
            "motifs": ["corner", "wine", "old_letters"],
            "visualBrief": "A solitary corner with warm mulled wine steam and an old box of letters.",
        },
        {
            "theme": "代笔停滞、空虚、家人向前",
            "motifs": ["blank_page", "mud", "moving_family"],
            "visualBrief": "A blank writing desk, a person paused in a soft mire, and small figures moving forward.",
        },
        {
            "theme": "孩子长大、家里的灯、文具店传统",
            "motifs": ["uniform", "home_light", "stationery"],
            "visualBrief": "An oversized school uniform, a lit home window, and stationery objects on a counter.",
        },
        {
            "theme": "女性身份整理、疲惫与复原、植物季节",
            "motifs": ["roles", "plants", "recovery"],
            "visualBrief": "A woman balancing many roles, seasonal plants, and a small path of recovery.",
        },
        {
            "theme": "等待语言、铺开信纸、小声音",
            "motifs": ["desk", "sounds", "waiting"],
            "visualBrief": "A quiet writing desk with paper, pen, kettle bubbles, wind, and a waiting clock.",
        },
        {
            "theme": "克制的爱、未寄出的信、山茶与明日叶",
            "motifs": ["camellia", "unsent_letters", "new_leaf"],
            "visualBrief": "A camellia blooming in cold air, unsent letters, and a fresh tomorrow leaf.",
        },
    ]


def draw_sketch_line(draw: ImageDraw.ImageDraw, points: list[tuple[int, int]], fill: tuple[int, int, int], width: int = 7) -> None:
    if len(points) >= 2:
        draw.line(points, fill=fill, width=width, joint="curve")


def draw_sketch_rect(draw: ImageDraw.ImageDraw, box: tuple[int, int, int, int], outline: tuple[int, int, int], width: int = 7) -> None:
    draw.rounded_rectangle(box, radius=10, outline=outline, width=width)


def draw_envelope(draw: ImageDraw.ImageDraw, x: int, y: int, w: int, h: int, ink: tuple[int, int, int], accent: tuple[int, int, int]) -> None:
    draw_sketch_rect(draw, (x, y, x + w, y + h), ink)
    draw_sketch_line(draw, [(x, y), (x + w // 2, y + h // 2), (x + w, y)], accent, 6)
    draw_sketch_line(draw, [(x, y + h), (x + w // 2, y + h // 2), (x + w, y + h)], ink, 5)


def draw_person(draw: ImageDraw.ImageDraw, x: int, y: int, scale: float, ink: tuple[int, int, int], accent: tuple[int, int, int] | None = None) -> None:
    r = int(28 * scale)
    body_h = int(90 * scale)
    draw.ellipse((x - r, y - r, x + r, y + r), outline=accent or ink, width=max(4, int(6 * scale)))
    draw_sketch_line(draw, [(x, y + r), (x, y + r + body_h)], ink, max(4, int(7 * scale)))
    draw_sketch_line(draw, [(x - int(46 * scale), y + int(55 * scale)), (x, y + int(78 * scale)), (x + int(46 * scale), y + int(55 * scale))], ink, max(4, int(6 * scale)))


def draw_camellia(draw: ImageDraw.ImageDraw, cx: int, cy: int, size: int, ink: tuple[int, int, int], accent: tuple[int, int, int]) -> None:
    for dx, dy in [(0, -32), (28, -10), (18, 28), (-18, 28), (-28, -10)]:
        draw.ellipse((cx + dx - size, cy + dy - size, cx + dx + size, cy + dy + size), outline=accent, width=6)
    draw.ellipse((cx - size // 2, cy - size // 2, cx + size // 2, cy + size // 2), outline=ink, width=5)
    draw_sketch_line(draw, [(cx, cy + size + 38), (cx, cy + size + 108)], ink, 6)
    draw.arc((cx - 78, cy + 42, cx, cy + 118), 200, 350, fill=accent, width=5)
    draw.arc((cx, cy + 50, cx + 82, cy + 126), 190, 340, fill=accent, width=5)


def render_semantic_whiteboard_scene(path: Path, index: int, spec: dict) -> None:
    index = ((index - 1) % 8) + 1
    bg = (246, 241, 227)
    paper = (248, 246, 239)
    ink = (45, 48, 50)
    muted = (196, 191, 176)
    accent = (205, 100, 65)
    image = Image.new("RGB", (WIDTH, HEIGHT), bg)
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((76, 64, WIDTH - 76, HEIGHT - 64), radius=0, outline=muted, width=6, fill=paper)
    draw_sketch_line(draw, [(270, 918), (1680, 918)], muted, 4)

    if index == 1:
        draw_sketch_rect(draw, (260, 300, 840, 705), ink)
        draw_sketch_line(draw, [(260, 410), (840, 410)], ink, 6)
        draw_sketch_line(draw, [(318, 360), (418, 300), (520, 360), (620, 300), (782, 360)], accent, 5)
        draw_envelope(draw, 980, 360, 420, 250, ink, accent)
        draw_person(draw, 1060, 765, 0.9, ink, accent)
        draw_person(draw, 1210, 790, 0.65, ink)
        draw_person(draw, 1325, 790, 0.65, ink)
        draw_camellia(draw, 1510, 745, 22, ink, accent)
    elif index == 2:
        for cx, cy, radius in [(455, 300, 92), (610, 250, 62), (780, 332, 76)]:
            for angle in range(0, 360, 45):
                import math
                end = (cx + int(math.cos(math.radians(angle)) * radius), cy + int(math.sin(math.radians(angle)) * radius))
                draw_sketch_line(draw, [(cx, cy), end], accent, 4)
        draw_sketch_rect(draw, (1010, 330, 1340, 760), ink)
        draw_sketch_line(draw, [(1010, 330), (1170, 420), (1340, 330)], ink, 6)
        draw.ellipse((1255, 525, 1285, 555), outline=accent, width=5)
        draw_sketch_rect(draw, (1425, 560, 1575, 650), ink)
        draw_sketch_line(draw, [(1460, 585), (1540, 625)], accent, 5)
        draw_sketch_rect(draw, (360, 690, 560, 780), ink)
    elif index == 3:
        draw_sketch_rect(draw, (270, 360, 700, 720), ink)
        draw_person(draw, 485, 555, 1.0, ink, accent)
        draw.arc((800, 390, 1030, 720), 200, 520, fill=accent, width=7)
        draw_sketch_line(draw, [(890, 690), (950, 825)], ink, 6)
        draw_envelope(draw, 1160, 345, 390, 235, ink, accent)
        draw_sketch_rect(draw, (1130, 640, 1600, 825), ink)
        draw_sketch_line(draw, [(1170, 695), (1560, 695)], accent, 5)
        draw_sketch_line(draw, [(1220, 748), (1510, 748)], ink, 5)
    elif index == 4:
        draw_sketch_rect(draw, (300, 320, 790, 730), ink)
        draw_sketch_line(draw, [(370, 420), (710, 420)], muted, 6)
        draw_sketch_line(draw, [(370, 510), (700, 510)], muted, 6)
        draw_person(draw, 1000, 665, 0.95, ink, accent)
        draw.arc((855, 610, 1145, 905), 190, 350, fill=ink, width=8)
        draw_sketch_line(draw, [(1230, 375), (1430, 455), (1630, 375)], accent, 6)
        draw_person(draw, 1350, 620, 0.58, ink)
        draw_person(draw, 1480, 620, 0.58, ink)
        draw_person(draw, 1610, 620, 0.58, ink)
    elif index == 5:
        draw_person(draw, 470, 595, 0.75, ink, accent)
        draw_sketch_line(draw, [(390, 710), (550, 710), (580, 870), (360, 870), (390, 710)], ink, 6)
        draw_sketch_rect(draw, (830, 360, 1320, 740), ink)
        draw_sketch_line(draw, [(830, 520), (1320, 520)], ink, 5)
        draw.ellipse((1048, 450, 1102, 504), outline=accent, width=6)
        draw_sketch_line(draw, [(930, 820), (1220, 820), (1220, 900), (930, 900), (930, 820)], accent, 6)
        draw_envelope(draw, 1430, 405, 230, 145, ink, accent)
    elif index == 6:
        draw_person(draw, 880, 530, 1.0, ink, accent)
        for x, y in [(520, 335), (590, 510), (520, 685), (1240, 335), (1310, 510), (1240, 685)]:
            draw_sketch_rect(draw, (x - 78, y - 45, x + 78, y + 45), ink, 5)
            draw_sketch_line(draw, [(x - 42, y), (x + 42, y)], accent, 5)
        draw_camellia(draw, 1530, 720, 20, ink, accent)
        draw_sketch_line(draw, [(310, 835), (430, 750), (555, 790), (675, 690)], accent, 7)
    elif index == 7:
        draw_sketch_rect(draw, (500, 360, 1130, 740), ink)
        draw_envelope(draw, 605, 455, 330, 205, ink, accent)
        draw_sketch_line(draw, [(1060, 465), (1220, 395), (1310, 470)], ink, 6)
        draw.ellipse((1285, 350, 1375, 440), outline=accent, width=6)
        for cx, cy in [(420, 430), (1430, 680), (1500, 570)]:
            draw.arc((cx - 48, cy - 48, cx + 48, cy + 48), 210, 510, fill=accent, width=5)
        draw_sketch_line(draw, [(810, 820), (1040, 820)], muted, 5)
    else:
        draw_camellia(draw, 475, 455, 30, ink, accent)
        draw_envelope(draw, 800, 400, 420, 260, ink, accent)
        draw_sketch_line(draw, [(1020, 660), (1020, 830)], ink, 7)
        draw.arc((1020, 705, 1190, 850), 190, 340, fill=accent, width=7)
        draw.arc((850, 700, 1020, 850), 200, 350, fill=accent, width=7)
        draw_sketch_line(draw, [(1360, 760), (1550, 610), (1670, 645)], accent, 8)
        draw_sketch_line(draw, [(1370, 845), (1630, 845)], muted, 5)

    image.save(path, quality=94)


def generate_whiteboard_skill_assets(
    video_dir: Path,
    material_root: Path,
    title: str,
    description: str,
    subtitle_events: list[tuple[int, int, str]],
) -> tuple[list[Path], Path, str]:
    if os.environ.get("BOOK_IMAGE_BACKEND", "").strip().lower() == "qwen-image-2512":
        try:
            return generate_qwen_image_assets(video_dir, material_root, title, description, subtitle_events)
        except Exception as error:
            print(f"Qwen Image backend failed, falling back to whiteboard image skill: {error}", file=sys.stderr, flush=True)
    image_dir = video_dir / "whiteboard_skill_images"
    prompts = build_whiteboard_skill_prompts(
        material_root,
        title,
        description,
        subtitle_events,
        target_visual_scene_count(material_root, subtitle_events),
    )
    raw_images = run_whiteboard_image_skill(prompts, image_dir)
    copied: list[Path] = []
    for index, source in enumerate(raw_images, 1):
        assert_meaningful_image(source)
        dest = video_dir / f"visual_{index:02d}_whiteboard_skill.png"
        with Image.open(source) as image:
            resized = image.convert("RGB").resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS)
            if os.environ.get("BOOK_IMAGE_PROMPT_STYLE", "book-illustration").strip().lower() == "book-illustration":
                resized.save(dest, quality=94)
            else:
                normalize_whiteboard_palette(resized).save(dest, quality=94)
        copied.append(dest)
    body_events = [(start, end, text) for start, end, text in subtitle_events if end > HEADER_AUDIO_DURATION_MS]
    series = []
    for index, (path, prompt) in enumerate(zip(copied, prompts), 1):
        if body_events:
            start_slot = round((index - 1) * len(body_events) / len(copied))
            end_slot = round(index * len(body_events) / len(copied))
            group = body_events[start_slot:end_slot] or body_events[max(0, start_slot - 1):start_slot]
            start_ms = group[0][0] if group else None
            end_ms = group[-1][1] if group else None
            subtitle_preview = " ".join(text.splitlines()[0].strip() for _, _, text in group if text.strip())
        else:
            start_ms = None
            end_ms = None
            subtitle_preview = description or title
        series.append(
            {
                "index": index,
                "image": str(path),
                "startMs": start_ms,
                "endMs": end_ms,
                "subtitlePreview": compact_text(subtitle_preview, 420),
                "prompt": prompt,
            }
        )
    manifest = {
        "sourceKind": "whiteboard_skill_images",
        "skill": "whiteboard-video-workflow/scripts/generate-image.py",
        "styleSource": str(WHITEBOARD_IMAGE_GENERATOR.parent / "prompt_template.py"),
        "generatedAt": time.strftime("%Y-%m-%d %H:%M:%S"),
        "imageMode": os.environ.get("OPENAI_IMAGE_MODE", "image"),
        "promptCount": len(prompts),
        "assets": [str(path) for path in copied],
        "series": series,
        "prompts": prompts,
    }
    (video_dir / "visual_assets_manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    (video_dir / "whiteboard_series_manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    return copied, image_dir, "whiteboard_skill_images"


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


def render_youtube_thumbnail(
    output_path: Path,
    title: str,
    subtitle: str = "",
    author: str = "",
) -> Path:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    image = Image.new("RGB", (1280, 720), (18, 23, 21))
    draw = ImageDraw.Draw(image)

    bg_top = (42, 50, 45)
    bg_bottom = (12, 16, 15)
    for y in range(720):
        ratio = y / 719
        color = tuple(round(bg_top[i] * (1 - ratio) + bg_bottom[i] * ratio) for i in range(3))
        draw.line((0, y, 1280, y), fill=color)

    gold = (236, 194, 86)
    warm = (255, 244, 214)
    white = (255, 255, 255)
    red = (201, 46, 42)
    ink = (28, 24, 21)
    muted = (176, 162, 125)

    draw.rectangle((0, 0, 1280, 38), fill=red)
    badge_font = load_font(34, bold=True)
    badge_text = "半小时听完一本书"
    badge_w, _ = text_size(draw, badge_text, badge_font)
    draw_centered_label(
        draw,
        (54, 56, 54 + badge_w + 54, 116),
        badge_text,
        badge_font,
        fill=(12, 12, 12),
        text_fill=white,
        outline=gold,
        radius=8,
        width=2,
    )

    # Right-side bold symbolic scene: letter, father/son silhouettes, and warm spotlight.
    draw.ellipse((760, 70, 1320, 630), fill=(48, 67, 54))
    draw.ellipse((850, 145, 1240, 535), fill=(66, 83, 62))
    draw.polygon([(765, 235), (1162, 152), (1210, 486), (810, 575)], fill=(236, 225, 190), outline=gold)
    draw.line((785, 250, 990, 376, 1188, 170), fill=(156, 122, 55), width=5)
    draw.line((808, 554, 1000, 377, 1204, 468), fill=(156, 122, 55), width=5)
    draw.ellipse((890, 292, 978, 380), fill=ink)
    draw.rounded_rectangle((848, 382, 1018, 582), radius=50, fill=ink)
    draw.ellipse((1054, 352, 1118, 416), fill=ink)
    draw.rounded_rectangle((1028, 418, 1146, 572), radius=40, fill=ink)
    draw.line((1015, 470, 1036, 502), fill=gold, width=10)
    draw.line((1036, 502, 1060, 474), fill=gold, width=10)

    title_font = load_font(92, bold=True)
    lines = youtube_thumbnail_lines(title, subtitle)
    if len(lines) == 1:
        lines.append("慢慢靠近")
    y = 214
    for line in lines[:2]:
        draw.text((62, y), line, font=title_font, fill=white, stroke_width=7, stroke_fill=(0, 0, 0))
        y += 104

    hook_font = load_font(50, bold=True)
    hook = "笨拙父亲的温柔"
    hook_w, _ = text_size(draw, hook, hook_font)
    draw_centered_label(
        draw,
        (62, 470, 62 + hook_w + 86, 548),
        hook,
        hook_font,
        fill=red,
        text_fill=warm,
        radius=10,
        width=0,
        stroke_width=2,
        stroke_fill=(80, 0, 0),
    )

    author_label = clean_label(author, "海明威父子家书")
    small_font = load_font(34, bold=True)
    draw.text((66, 592), compact_text(author_label, 16), font=small_font, fill=muted)
    draw.text((66, 636), "睡前听书 / 30 分钟", font=small_font, fill=gold)

    image.save(output_path, quality=96, subsampling=0)
    return output_path


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


def cinematic_motion_profile(index: int, frames: int, is_cover: bool = False) -> dict[str, str]:
    if not CINEMATIC_ENABLE_MOTION:
        return {
            "name": "stable_still",
            "zoom": "1.0",
            "x": "0",
            "y": "0",
        }

    den = str(max(1, frames - 1))
    if is_cover:
        return {
            "name": "cover_slow_breathe",
            "zoom": "min(1.0+on*0.00006,1.018)",
            "x": "(iw-iw/zoom)*0.50",
            "y": "(ih-ih/zoom)*0.50",
        }
    name, zoom, x_expr, y_expr = CINEMATIC_MOTION_PROFILES[(index - 1) % len(CINEMATIC_MOTION_PROFILES)]
    return {
        "name": name,
        "zoom": zoom.format(den=den),
        "x": x_expr.format(den=den),
        "y": y_expr.format(den=den),
    }


def cinematic_filter_chain(index: int, duration: float) -> tuple[str, str]:
    frames = max(1, int(round(duration * CINEMATIC_FPS)))
    profile = cinematic_motion_profile(index, frames, is_cover=index == 0)
    if CINEMATIC_ENABLE_MOTION:
        motion_filter = (
            f"zoompan=z='{profile['zoom']}':d={frames}:"
            f"x='{profile['x']}':y='{profile['y']}':s={WIDTH}x{HEIGHT}:fps={CINEMATIC_FPS},"
        )
    else:
        motion_filter = f"fps={CINEMATIC_FPS},"
    treatment = (
        "eq=brightness=0.035:saturation=1.03:contrast=1.02,"
        "unsharp=5:5:0.28:3:3:0.08,"
    )
    return motion_filter + treatment, profile["name"]


def build_content_visual_segments(
    content_images: list[Path],
    duration_ms: int,
    header_ms: int,
    subtitle_events: list[tuple[int, int, str]],
) -> list[dict]:
    if not content_images:
        return []

    body_start = header_ms
    body_duration = max(0, duration_ms - header_ms)
    body_events = [
        (max(start, header_ms), min(end, duration_ms), text)
        for start, end, text in subtitle_events
        if end > header_ms and start < duration_ms
    ]
    body_events = [(start, end, text) for start, end, text in body_events if end > start]

    if not body_events:
        segment_duration = body_duration / max(1, len(content_images))
        cursor = float(body_start)
        segments = []
        for index, image in enumerate(content_images):
            end = duration_ms if index == len(content_images) - 1 else cursor + segment_duration
            segments.append(
                {
                    "startMs": int(round(cursor)),
                    "endMs": int(round(end)),
                    "image": str(image),
                    "description": f"Cinematic content image {index + 1}",
                    "kind": "content",
                    "subtitleStartIndex": None,
                    "subtitleEndIndex": None,
                    "sourceTextPreview": "",
                    "motionProfile": cinematic_motion_profile(
                        index + 1,
                        max(1, int(round(((end - cursor) / 1000) * CINEMATIC_FPS))),
                    )["name"],
                }
            )
            cursor = end
        return segments

    events_per_segment = max(1, round(len(body_events) / len(content_images)))
    grouped_events = []
    cursor = 0
    for index in range(len(content_images)):
        remaining_images = len(content_images) - index
        remaining_events = len(body_events) - cursor
        take = max(1, round(remaining_events / remaining_images)) if remaining_images > 0 else remaining_events
        grouped_events.append(body_events[cursor : cursor + take])
        cursor += take
    if cursor < len(body_events):
        grouped_events[-1].extend(body_events[cursor:])

    segments = []
    current_start = body_start
    for index, image in enumerate(content_images):
        group = grouped_events[index] if index < len(grouped_events) else []
        if group:
            start = current_start if index == 0 else max(current_start, group[0][0])
            end = duration_ms if index == len(content_images) - 1 else max(start + 1000, group[-1][1])
            text_preview = " ".join(text.replace("\n", " ") for _, _, text in group)
        else:
            remaining = max(1000, duration_ms - current_start)
            slots = max(1, len(content_images) - index)
            start = current_start
            end = duration_ms if index == len(content_images) - 1 else start + remaining / slots
            text_preview = ""

        start = int(round(max(body_start, min(start, duration_ms - 1))))
        end = int(round(max(start + 1000, min(end, duration_ms))))
        if index == len(content_images) - 1:
            end = duration_ms
        duration_frames = max(1, int(round(((end - start) / 1000) * CINEMATIC_FPS)))
        segments.append(
            {
                "startMs": start,
                "endMs": end,
                "image": str(image),
                "description": f"Cinematic content image {index + 1}",
                "kind": "content",
                "subtitleStartIndex": None if not group else body_events.index(group[0]) + 1,
                "subtitleEndIndex": None if not group else body_events.index(group[-1]) + 1,
                "sourceTextPreview": compact_text(text_preview, 220) if text_preview else "",
                "motionProfile": cinematic_motion_profile(index + 1, duration_frames)["name"],
            }
        )
        current_start = end
    return segments


def write_visual_story_plan(
    path: Path,
    title: str,
    description: str,
    segments: list[dict],
    source_kind: str,
) -> None:
    style_bible = {
        "format": "professional 30-minute book summary video",
        "visualStyle": "cinematic editorial listening-video background art",
        "mood": "quiet, literary, premium, suitable for bedtime listening",
        "composition": "wide 16:9, restrained negative space, no AI-rendered text",
        "continuity": "consistent color grading, consistent era cues, recurring motifs from the book",
    }
    image_prompts = []
    for index, segment in enumerate(segments, 1):
        source_text = segment.get("sourceTextPreview") or description or title
        image_prompts.append(
            {
                "assetId": f"scene_{index:02d}",
                "image": segment.get("image"),
                "startMs": segment.get("startMs"),
                "endMs": segment.get("endMs"),
                "motionProfile": segment.get("motionProfile"),
                "prompt": (
                    "Create one cinematic 16:9 background illustration for a professional "
                    "Chinese book-summary video. No text, no logo, no watermark. "
                    f"Book/video title: {title}. Scene source: {compact_text(source_text, 260)}"
                ),
            }
        )
    path.write_text(
        json.dumps(
            {
                "pipeline": "epub -> 8000-word narration -> audio -> subtitles/lrc -> visual timeline -> video",
                "sourceKind": source_kind,
                "title": title,
                "styleBible": style_bible,
                "imagePrompts": image_prompts,
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )


def render_no_subtitle_video(
    output: Path,
    cover: Path,
    content_images: list[Path],
    audio: Path,
    background_music: Path | None,
    header_seconds: float,
    visual_segments: list[dict] | None = None,
) -> None:
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        raise RuntimeError("ffmpeg was not found, so the video cannot be generated.")
    audio_seconds = ffprobe_duration_ms(audio) / 1000.0
    total_seconds = audio_seconds
    if not content_images:
        raise RuntimeError("No visual images were prepared for video rendering.")
    if visual_segments:
        content_images = [Path(str(segment["image"])) for segment in visual_segments]
    images = [cover, *content_images]
    cmd = [
        ffmpeg,
        "-y",
    ]
    if visual_segments:
        durations = [
            float(header_seconds),
            *[
                max(0.1, (int(segment["endMs"]) - int(segment["startMs"])) / 1000.0)
                for segment in visual_segments
            ],
        ]
    else:
        body_segment = max(0.1, (audio_seconds - header_seconds) / max(1, len(content_images)))
        durations = [float(header_seconds), *([body_segment] * len(content_images))]
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
        motion_filter, motion_profile = cinematic_filter_chain(index, duration)
        filter_chain = (
            f"[{index}:v]scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=increase,"
            f"crop={WIDTH}:{HEIGHT},"
            f"{motion_filter}"
            f"trim=duration={duration:.3f},setpts=PTS-STARTPTS[v{index}]"
        )
        video_filters.append(filter_chain)
        video_labels.append(f"[v{index}]")
    video_concat = "".join(video_labels) + f"concat=n={len(images)}:v=1:a=0[v]"
    if background_music and background_music.is_file():
        bgm_index = audio_index + 1
        audio_mix = (
            f"[{bgm_index}:a]volume=0.10[bgm];"
            f"[{audio_index}:a][bgm]amix=inputs=2:duration=first:dropout_transition=2[a]"
        )
    else:
        audio_mix = f"[{audio_index}:a]anull[a]"
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


def build_visual_timeline(
    path: Path,
    cover: Path,
    content_images: list[Path],
    duration_ms: int,
    source_dir: Path | None,
    source_kind: str,
    header_ms: int,
    visual_segments: list[dict] | None = None,
) -> None:
    segments = [
        {
            "startMs": 0,
            "endMs": header_ms,
            "image": str(cover),
            "description": "Generated cinematic cover intro",
            "kind": "cover",
            "motionProfile": cinematic_motion_profile(
                0,
                max(1, int(round((header_ms / 1000) * CINEMATIC_FPS))),
                True,
            )["name"],
        }
    ]
    if visual_segments is None:
        visual_segments = build_content_visual_segments(content_images, duration_ms, header_ms, [])
    segments.extend(visual_segments)
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
    parser.add_argument("--header-audio")
    parser.add_argument("--force-aeneas", action="store_true")
    parser.add_argument("--audio-subtitle-only", action="store_true")
    parser.add_argument("--visual-assets-only", action="store_true")
    parser.add_argument("--subtitles-only", action="store_true")
    parser.add_argument("--subtitle-reference")
    parser.add_argument("--subtitle-output-name", default="subtitles_ai_test.txt")
    parser.add_argument("--subtitle-max-input-chars", type=int, default=0)
    parser.add_argument("--subtitle-batch-chars", type=int, default=0)
    parser.add_argument("--controlled-programmatic-visuals", action="store_true")
    parser.add_argument("--ignore-existing-visual-assets", action="store_true")
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
    source_audio_target = video_dir / f"{output_stem}.mp3"
    prepared_audio_target = video_dir / f"{output_stem}_video_mix.mp3"

    if args.subtitles_only:
        reference_file = Path(args.subtitle_reference) if args.subtitle_reference else None
        lines, report = generate_chinese_subtitles_with_ai(
            material_root,
            reference_file,
            max_input_chars=max(0, args.subtitle_max_input_chars),
            batch_chars=max(0, args.subtitle_batch_chars),
        )
        output_name = args.subtitle_output_name.strip() or "subtitles_ai_test.txt"
        subtitle_output = material_root / output_name
        subtitle_output.write_text("\n".join(lines) + "\n", encoding="utf-8")
        report["subtitleOutput"] = str(subtitle_output)
        report_file = material_root / f"{Path(output_name).stem}_report.json"
        report_file.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
        result = {
            "materialDir": str(material_root),
            "subtitleOutput": str(subtitle_output),
            "report": str(report_file),
            "lineCount": len(lines),
            "referenceSimilarity": report.get("referenceSimilarity"),
            "subtitlesOnly": True,
        }
        print(json.dumps(result, ensure_ascii=False))
        return 0

    if args.visual_assets_only:
        manifest = video_dir / "pipeline_manifest.json"
        header_duration_ms = HEADER_AUDIO_DURATION_MS
        header_seconds = header_duration_ms / 1000.0
        timed_srt = find_timed_chinese_srt(material_root, video_dir, args.audio_language)
        if not timed_srt:
            raise RuntimeError(
                "Image generation requires aligned Chinese SRT. Please generate audio and subtitles first, "
                "then run the image stage."
            )
        events = read_srt_events(timed_srt)
        if not events:
            raise RuntimeError(f"Aligned Chinese SRT has no subtitle events: {timed_srt}")
        duration_ms = max(end for _, end, _ in events)
        prior_manifest = {}
        if manifest.exists():
            try:
                prior_manifest = json.loads(manifest.read_text(encoding="utf-8", errors="ignore"))
            except Exception:
                prior_manifest = {}
        manifest_duration = prior_manifest.get("narrationAudioForVideoDurationMs") if isinstance(prior_manifest, dict) else None
        if isinstance(manifest_duration, int) and manifest_duration > duration_ms:
            duration_ms = manifest_duration
        if args.ignore_existing_visual_assets:
            content_images, visual_source_dir, visual_source_kind = [], None, "none"
        else:
            content_images, visual_source_dir, visual_source_kind = migrate_visual_assets(material_root, video_dir)
        if len(content_images) < VISUAL_SCENE_MIN_COUNT and args.controlled_programmatic_visuals:
            content_images, visual_source_dir, visual_source_kind = generate_controlled_programmatic_assets(
                epub,
                material_root,
                video_dir,
                events,
            )
        if not content_images:
            try:
                content_images, visual_source_dir, visual_source_kind = generate_whiteboard_skill_assets(
                    video_dir,
                    material_root,
                    title,
                    description,
                    events,
                )
            except Exception:
                if not args.allow_placeholder_visuals:
                    raise
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
        visual_segments = build_content_visual_segments(
            content_images,
            duration_ms,
            header_duration_ms,
            events,
        )
        visual_story_plan = video_dir / "visual_story_plan.json"
        visual_timeline = video_dir / "visual_timeline.json"
        write_visual_story_plan(
            visual_story_plan,
            title,
            description,
            visual_segments,
            visual_source_kind,
        )
        build_visual_timeline(
            visual_timeline,
            cover,
            content_images,
            duration_ms,
            visual_source_dir,
            visual_source_kind,
            header_duration_ms,
            visual_segments,
        )
        result = {
            "materialDir": str(material_root),
            "pipelineManifest": str(manifest),
            "hardSubtitleManifest": None,
            "hardSubtitleSrt": str(timed_srt),
            "subtitleTiming": "existing_aligned_chinese_srt",
            "subtitleManifest": prior_manifest.get("subtitleManifest") if isinstance(prior_manifest, dict) else None,
            "narrationAudioForVideo": None,
            "narrationAudioWithoutHeader": None,
            "headerAudio": None,
            "sourceAudio": None,
            "sourceAudioKind": None,
            "expectedSourceAudioDurationMs": None,
            "sourceAudioDurationMs": None,
            "narrationAudioWithoutHeaderDurationMs": None,
            "headerAudioDurationMs": header_duration_ms,
            "narrationAudioForVideoDurationMs": None,
            "coverSeconds": header_seconds,
            "subtitleCount": len(events),
            "stretchRatio": 1.0,
            "subtitleAlignmentAudio": None,
            "elapsedSeconds": 0.0,
            "audioSubtitleOnly": False,
            "cover": str(cover),
            "background": str(content_images[0]) if content_images else None,
            "visualAssetsDir": str(visual_source_dir) if visual_source_dir else None,
            "visualSourceKind": visual_source_kind,
            "visualAssetCount": len(content_images),
            "visualStoryPlan": str(visual_story_plan),
            "visualTimeline": str(visual_timeline),
            "noSubtitleVideo": None,
            "hardSubtitleVideo": None,
            "backgroundMusic": None,
            "noSubtitleVideoDurationMs": None,
            "videoDurationMs": None,
            "visualAssetsOnly": True,
        }
        manifest.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
        print(json.dumps(result, ensure_ascii=False))
        return 0

    source_audio, source_audio_kind, expected_source_audio_duration_ms = select_narration_source_audio(
        material_root,
        source_audio_target,
    )

    narration_audio, narration_duration_ms, stretch_ratio = prepare_narration_audio(
        source_audio,
        video_dir / "narration_for_video.mp3",
        TARGET_MIN_SECONDS,
    )
    header_audio = ensure_header_audio(Path(args.header_audio) if args.header_audio else default_header_audio_path())
    header_duration_ms = HEADER_AUDIO_DURATION_MS
    header_seconds = header_duration_ms / 1000.0
    prepared_audio = prepend_header_audio(header_audio, narration_audio, prepared_audio_target)
    duration_ms = ffprobe_duration_ms(prepared_audio)
    ass_file, srt_file, events, subtitle_manifest = build_aeneas_subtitles(
        material_root,
        narration_audio,
        video_dir,
        args.audio_language,
        args.force_aeneas,
        header_duration_ms,
    )

    manifest = video_dir / "pipeline_manifest.json"
    base_result = {
        "materialDir": str(material_root),
        "pipelineManifest": str(manifest),
        "hardSubtitleManifest": str(ass_file),
        "hardSubtitleSrt": str(srt_file),
        "subtitleTiming": subtitle_manifest.get("subtitleTiming"),
        "subtitleManifest": subtitle_manifest,
        "narrationAudioForVideo": str(prepared_audio),
        "narrationAudioWithoutHeader": str(narration_audio),
        "headerAudio": str(header_audio),
        "sourceAudio": str(source_audio),
        "sourceAudioKind": source_audio_kind,
        "expectedSourceAudioDurationMs": expected_source_audio_duration_ms,
        "sourceAudioDurationMs": ffprobe_duration_ms(source_audio),
        "narrationAudioWithoutHeaderDurationMs": narration_duration_ms,
        "headerAudioDurationMs": header_duration_ms,
        "narrationAudioForVideoDurationMs": duration_ms,
        "coverSeconds": header_seconds,
        "subtitleCount": len(events),
        "stretchRatio": stretch_ratio,
        "subtitleAlignmentAudio": str(narration_audio),
        "elapsedSeconds": 0.0,
        "audioSubtitleOnly": bool(args.audio_subtitle_only),
    }
    if args.audio_subtitle_only:
        result = {
            **base_result,
            "cover": None,
            "background": None,
            "visualAssetsDir": None,
            "visualSourceKind": None,
            "visualAssetCount": 0,
            "visualStoryPlan": None,
            "visualTimeline": None,
            "noSubtitleVideo": None,
            "hardSubtitleVideo": None,
            "backgroundMusic": None,
            "noSubtitleVideoDurationMs": None,
            "videoDurationMs": None,
        }
        manifest.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
        print(json.dumps(result, ensure_ascii=False))
        return 0

    if args.ignore_existing_visual_assets:
        content_images, visual_source_dir, visual_source_kind = [], None, "none"
    else:
        content_images, visual_source_dir, visual_source_kind = migrate_visual_assets(material_root, video_dir)
    if len(content_images) < VISUAL_SCENE_MIN_COUNT and args.controlled_programmatic_visuals:
        content_images, visual_source_dir, visual_source_kind = generate_controlled_programmatic_assets(
            epub,
            material_root,
            video_dir,
            events,
        )
    if not content_images:
        try:
            content_images, visual_source_dir, visual_source_kind = generate_whiteboard_skill_assets(
                video_dir,
                material_root,
                title,
                description,
                events,
            )
        except Exception:
            if not args.allow_placeholder_visuals:
                raise
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
    visual_segments = build_content_visual_segments(
        content_images,
        duration_ms,
        header_duration_ms,
        events,
    )
    visual_story_plan = video_dir / "visual_story_plan.json"
    write_visual_story_plan(
        visual_story_plan,
        title,
        description,
        visual_segments,
        visual_source_kind,
    )
    if args.visual_assets_only:
        result = {
            **base_result,
            "cover": str(cover),
            "background": str(content_images[0]) if content_images else None,
            "visualAssetsDir": str(visual_source_dir) if visual_source_dir else None,
            "visualSourceKind": visual_source_kind,
            "visualAssetCount": len(content_images),
            "visualStoryPlan": str(visual_story_plan),
            "visualTimeline": str(video_dir / "visual_timeline.json"),
            "noSubtitleVideo": None,
            "hardSubtitleVideo": None,
            "backgroundMusic": None,
            "noSubtitleVideoDurationMs": None,
            "videoDurationMs": None,
            "visualAssetsOnly": True,
        }
        build_visual_timeline(
            video_dir / "visual_timeline.json",
            cover,
            content_images,
            duration_ms,
            visual_source_dir,
            visual_source_kind,
            header_duration_ms,
            visual_segments,
        )
        manifest.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
        print(json.dumps(result, ensure_ascii=False))
        return 0
    background_music = Path(args.background_music) if args.background_music else None
    no_subtitle_video = video_dir / f"{output_stem}_无字幕母版.mp4"
    hard_video = video_dir / f"{output_stem}_中英双语字幕_精修版.mp4"
    render_no_subtitle_video(
        no_subtitle_video,
        cover,
        content_images,
        prepared_audio,
        background_music,
        header_seconds,
        visual_segments,
    )
    render_hard_subtitle_video(hard_video, no_subtitle_video, ass_file)

    no_subtitle_duration_ms = ffprobe_duration_ms(no_subtitle_video)
    video_duration_ms = ffprobe_duration_ms(hard_video)
    result = {
        **base_result,
        "cover": str(cover),
        "background": str(content_images[0]) if content_images else None,
        "visualAssetsDir": str(visual_source_dir) if visual_source_dir else None,
        "visualSourceKind": visual_source_kind,
        "visualAssetCount": len(content_images),
        "visualStoryPlan": str(visual_story_plan),
        "visualTimeline": str(video_dir / "visual_timeline.json"),
        "noSubtitleVideo": str(no_subtitle_video),
        "hardSubtitleVideo": str(hard_video),
        "backgroundMusic": str(background_music) if background_music and background_music.is_file() else None,
        "noSubtitleVideoDurationMs": no_subtitle_duration_ms,
        "videoDurationMs": video_duration_ms,
    }
    build_visual_timeline(
        video_dir / "visual_timeline.json",
        cover,
        content_images,
        duration_ms,
        visual_source_dir,
        visual_source_kind,
        header_duration_ms,
        visual_segments,
    )
    manifest.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
