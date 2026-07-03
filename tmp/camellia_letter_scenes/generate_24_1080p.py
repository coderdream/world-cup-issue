import base64
import json
import re
import subprocess
import time
from pathlib import Path
from urllib import request as urlrequest
from urllib.error import HTTPError, URLError

from PIL import Image, ImageDraw


ENDPOINT = "http://100.96.199.26:30019/v1/images/generations"
SRT_PATH = Path(r"D:\books\0625新书四本\2025-01《山茶的情书》\output\hard_subtitle.aeneas.zh-en.srt")
OUT_DIR = Path(r"tmp\camellia_letter_scenes\24_1080p_v1")
RAW_DIR = OUT_DIR / "raw"
FINAL_DIR = OUT_DIR / "final_1080p"
LOG_PATH = OUT_DIR / "generation.log"
MANIFEST_PATH = OUT_DIR / "manifest.json"

TARGET_SIZE = (1920, 1080)
RAW_SIZE = "1024x576"

STYLE = (
    "professional cinematic realistic watercolor illustration for a Chinese 30-minute book-summary video, "
    "consistent young East Asian woman with shoulder-length black hair and soft cream cardigan, "
    "quiet Japanese seaside town and warm stationery-room atmosphere, camellia flowers as recurring motif, "
    "letters and notebooks with no readable text, emotional healing tone, elegant composition, soft film lighting, "
    "detailed background, 16:9, no readable text, no logo, no watermark"
)

NEGATIVE = (
    "street exterior only, empty building, storefront only, no people, readable text, watermark, logo, caption, UI, "
    "flowchart, diagram, simple geometric lines, low detail, blurry, black image, blank image, monochrome, "
    "deformed hands, extra fingers, bad anatomy, distorted face, ugly, oversaturated, plastic skin, duplicate face, "
    "comic panel, split screen"
)

VISUAL_BEATS = [
    "inside the small letter-writing stationery shop at night, she writes a letter under a brass desk lamp, envelope and camellia vase on the wooden desk",
    "morning by a rain-streaked window, she opens an old envelope carefully, tea cup and camellia petals nearby, surprise and unease on her face",
    "quiet tatami room memory, mother holding a letter near a low table while daughter stands behind a half-open sliding door, camellia branch between them",
    "close emotional desk scene, pen hovering over an unfinished page, camellia shadow across the paper, restrained sadness",
    "seaside cafe window in Kamakura morning, she reads a letter beside tea, misty harbor and green hills outside, hopeful healing light",
    "walking along a seaside path after rain, holding the envelope close, camellia bushes and wet reflections, contemplative transition",
    "library-like corner with shelves of paper, she compares old letters and notebook pages, piecing together family memories",
    "warm evening room, two women seated at a low wooden table, old letters between them, soft lamp light, forgiveness beginning",
    "symbolic still life, open letter, notebook, fountain pen and pink camellias on wooden desk, seaside light through window, peaceful closure",
]


def log(message):
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    line = f"{timestamp} {message}"
    print(line, flush=True)
    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
    with LOG_PATH.open("a", encoding="utf-8") as f:
        f.write(line + "\n")


def read_srt_text():
    text = SRT_PATH.read_text(encoding="utf-8", errors="replace")
    blocks = re.split(r"\n\s*\n", text.replace("\r\n", "\n").replace("\r", "\n"))
    cues = []
    for block in blocks:
        lines = [line.strip() for line in block.split("\n") if line.strip()]
        if len(lines) < 3:
            continue
        if "-->" not in lines[1]:
            continue
        body_lines = []
        for line in lines[2:]:
            if re.search(r"[\u4e00-\u9fff]", line):
                body_lines.append(line)
        if body_lines:
            cue_text = re.sub(r"\s+", "", "".join(body_lines))
            cues.append(cue_text)
    return cues


def split_into_segments(cues, count=24):
    if not cues:
        raise RuntimeError(f"No Chinese cues parsed from {SRT_PATH}")
    segments = []
    total = len(cues)
    for i in range(count):
        start = round(i * total / count)
        end = round((i + 1) * total / count)
        if end <= start:
            end = min(total, start + 1)
        text = "".join(cues[start:end])
        if len(text) > 260:
            text = text[:260]
        segments.append({"index": i + 1, "cueStart": start + 1, "cueEnd": end, "text": text})
    return segments


def theme_hint(segment_text, index):
    t = segment_text
    if any(word in t for word in ["信", "代笔", "写", "纸", "字"]):
        return VISUAL_BEATS[0 if index < 5 else 6]
    if any(word in t for word in ["母", "妈妈", "女儿", "家", "亲"]):
        return VISUAL_BEATS[2 if index < 16 else 7]
    if any(word in t for word in ["海", "雨", "窗", "街", "路"]):
        return VISUAL_BEATS[4 if index < 18 else 5]
    if any(word in t for word in ["原谅", "理解", "告别", "释怀", "温柔"]):
        return VISUAL_BEATS[7]
    if index >= 22:
        return VISUAL_BEATS[8]
    return VISUAL_BEATS[index % len(VISUAL_BEATS)]


def build_prompts(segments):
    prompts = []
    for seg in segments:
        hint = theme_hint(seg["text"], seg["index"])
        visual_text = seg["text"][:120]
        prompt = (
            f"{STYLE}, scene {seg['index']:02d} of 24, {hint}, "
            f"visualize this subtitle meaning without drawing readable text: {visual_text}"
        )
        prompts.append({**seg, "prompt": prompt, "negativePrompt": NEGATIVE})
    return prompts


def mac_pressure():
    try:
        cmd = (
            "vm_stat | head -8; "
            "ps -Ao pid,pcpu,pmem,rss,comm | sort -nrk2 | head -8; "
            "lsof -nP -iTCP:30019 -sTCP:LISTEN || true"
        )
        result = subprocess.run(
            ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=8", "macmini4", cmd],
            capture_output=True,
            text=True,
            timeout=20,
            encoding="utf-8",
            errors="replace",
        )
        return result.stdout.strip()
    except Exception as exc:
        return f"pressure_unavailable: {exc!r}"


def call_image(prompt_info):
    payload = {
        "prompt": prompt_info["prompt"],
        "negative_prompt": prompt_info["negativePrompt"],
        "size": RAW_SIZE,
        "n": 1,
        "steps": 10,
        "guidance_scale": 2.2,
        "seed": 340000 + prompt_info["index"],
    }
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    req = urlrequest.Request(ENDPOINT, data=body, headers={"Content-Type": "application/json"}, method="POST")
    start = time.time()
    last_error = None
    for attempt in range(1, 4):
        try:
            with urlrequest.urlopen(req, timeout=240) as resp:
                raw = resp.read()
                status = resp.status
            break
        except HTTPError as exc:
            raw = exc.read()
            status = exc.code
            break
        except Exception as exc:
            last_error = exc
            log(f"[{prompt_info['index']:02d}/24] request attempt {attempt} failed: {exc!r}")
            time.sleep(10 * attempt)
    else:
        raise RuntimeError(f"request failed after retries: {last_error!r}") from last_error
    elapsed = round(time.time() - start, 2)
    data = json.loads(raw.decode("utf-8"))
    item = data.get("data", [{}])[0]
    b64 = item.get("b64_json")
    if not b64:
        raise RuntimeError(f"no image returned, status={status}, data={data}")
    return status, elapsed, item, base64.b64decode(b64), data


def image_metrics(path):
    im = Image.open(path).convert("RGB")
    small = im.resize((256, 144))
    colors = small.getcolors(maxcolors=65536) or []
    total = small.width * small.height
    near_black = sum(c for c, (r, g, b) in colors if r < 12 and g < 12 and b < 12) / total
    near_white = sum(c for c, (r, g, b) in colors if r > 245 and g > 245 and b > 245) / total
    return {
        "width": im.width,
        "height": im.height,
        "colors": len(colors),
        "nearBlackRatio": round(near_black, 3),
        "nearWhiteRatio": round(near_white, 3),
    }


def upscale_to_1080p(raw_path, final_path):
    im = Image.open(raw_path).convert("RGB")
    upscaled = im.resize(TARGET_SIZE, Image.Resampling.LANCZOS)
    final_path.parent.mkdir(parents=True, exist_ok=True)
    upscaled.save(final_path, quality=95)


def make_contact_sheet(results):
    cols = 4
    thumb_w, thumb_h = 480, 270
    label_h = 34
    rows = (len(results) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * thumb_w, rows * (thumb_h + label_h)), "white")
    draw = ImageDraw.Draw(sheet)
    for i, result in enumerate(results):
        img = Image.open(result["finalPath"]).convert("RGB").resize((thumb_w, thumb_h))
        x = (i % cols) * thumb_w
        y = (i // cols) * (thumb_h + label_h)
        sheet.paste(img, (x, y))
        draw.rectangle([x, y + thumb_h, x + thumb_w, y + thumb_h + label_h], fill=(245, 245, 242))
        draw.text(
            (x + 8, y + thumb_h + 9),
            f"{result['index']:02d} {result['elapsedSecondsClient']}s {result['finalBytes']//1024}KB ok={result['ok']}",
            fill=(20, 20, 20),
        )
    path = OUT_DIR / "contact_sheet.jpg"
    sheet.save(path, quality=92)
    return path


def main():
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    FINAL_DIR.mkdir(parents=True, exist_ok=True)
    cues = read_srt_text()
    prompts = build_prompts(split_into_segments(cues, 24))
    results = []
    prompt_path = OUT_DIR / "prompts_24.json"
    prompt_path.write_text(json.dumps(prompts, ensure_ascii=False, indent=2), encoding="utf-8")
    log(f"SRT={SRT_PATH}")
    log(f"parsed_cues={len(cues)} prompts={prompt_path}")
    log(f"start pressure snapshot:\n{mac_pressure()}")

    for prompt_info in prompts:
        idx = prompt_info["index"]
        raw_path = RAW_DIR / f"{idx:02d}_{prompt_info['cueStart']:04d}_{prompt_info['cueEnd']:04d}.png"
        final_path = FINAL_DIR / f"{idx:02d}_{prompt_info['cueStart']:04d}_{prompt_info['cueEnd']:04d}_1080p.jpg"
        response_path = OUT_DIR / f"{idx:02d}_response.json"
        if final_path.exists() and raw_path.exists():
            raw_metrics = image_metrics(raw_path)
            final_metrics = image_metrics(final_path)
            ok = (
                final_metrics["width"] == 1920
                and final_metrics["height"] == 1080
                and final_path.stat().st_size >= 350000
                and final_metrics["colors"] >= 4000
                and final_metrics["nearBlackRatio"] < 0.8
                and final_metrics["nearWhiteRatio"] < 0.9
            )
            result = {
                **prompt_info,
                "ok": ok,
                "status": "reused",
                "elapsedSecondsClient": 0,
                "serverElapsedSeconds": 0,
                "rawPath": str(raw_path.resolve()),
                "finalPath": str(final_path.resolve()),
                "responsePath": str(response_path.resolve()),
                "rawBytes": raw_path.stat().st_size,
                "finalBytes": final_path.stat().st_size,
                "rawMetrics": raw_metrics,
                "finalMetrics": final_metrics,
                "remotePath": None,
                "pressureAfter": "reused_existing_file",
            }
            results.append(result)
            log(f"[{idx:02d}/24] reused existing final={final_path.name} {final_path.stat().st_size//1024}KB ok={ok}")
            continue
        log(f"[{idx:02d}/24] generating cue={prompt_info['cueStart']}-{prompt_info['cueEnd']}")
        status, elapsed, item, image_bytes, response_data = call_image(prompt_info)
        raw_path.write_bytes(image_bytes)
        item.pop("b64_json", None)
        response_path.write_text(json.dumps(response_data, ensure_ascii=False, indent=2), encoding="utf-8")
        upscale_to_1080p(raw_path, final_path)
        raw_metrics = image_metrics(raw_path)
        final_metrics = image_metrics(final_path)
        ok = (
            status == 200
            and final_metrics["width"] == 1920
            and final_metrics["height"] == 1080
            and final_path.stat().st_size >= 350000
            and final_metrics["colors"] >= 4000
            and final_metrics["nearBlackRatio"] < 0.8
            and final_metrics["nearWhiteRatio"] < 0.9
        )
        result = {
            **prompt_info,
            "ok": ok,
            "status": status,
            "elapsedSecondsClient": elapsed,
            "serverElapsedSeconds": item.get("elapsedSeconds"),
            "rawPath": str(raw_path.resolve()),
            "finalPath": str(final_path.resolve()),
            "responsePath": str(response_path.resolve()),
            "rawBytes": raw_path.stat().st_size,
            "finalBytes": final_path.stat().st_size,
            "rawMetrics": raw_metrics,
            "finalMetrics": final_metrics,
            "remotePath": item.get("path"),
            "pressureAfter": mac_pressure(),
        }
        results.append(result)
        MANIFEST_PATH.write_text(
            json.dumps(
                {
                    "endpoint": ENDPOINT,
                    "srtPath": str(SRT_PATH),
                    "rawSize": RAW_SIZE,
                    "targetSize": "1920x1080",
                    "createdAt": time.strftime("%Y-%m-%d %H:%M:%S"),
                    "completedCount": len(results),
                    "allPassedMetrics": len(results) == 24 and all(r["ok"] for r in results),
                    "results": results,
                },
                ensure_ascii=False,
                indent=2,
            ),
            encoding="utf-8",
        )
        log(
            f"[{idx:02d}/24] done elapsed={elapsed}s server={item.get('elapsedSeconds')}s "
            f"final={final_path.name} {final_path.stat().st_size//1024}KB ok={ok}"
        )

    contact_sheet = make_contact_sheet(results)
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    manifest["contactSheet"] = str(contact_sheet.resolve())
    manifest["allPassedMetrics"] = all(r["ok"] for r in results)
    manifest["finishedAt"] = time.strftime("%Y-%m-%d %H:%M:%S")
    MANIFEST_PATH.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    log(f"finished contact_sheet={contact_sheet.resolve()}")
    log(f"final pressure snapshot:\n{mac_pressure()}")


if __name__ == "__main__":
    main()
