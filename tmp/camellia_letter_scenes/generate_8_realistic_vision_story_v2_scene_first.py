import base64
import json
import re
import subprocess
import time
from pathlib import Path
from urllib import request as urlrequest

from PIL import Image, ImageDraw


ENDPOINT = "http://192.168.1.9:30020/v1/images/generations"
SRT_PATH = Path(r"D:\books\0625新书四本\2025-01《山茶的情书》\output\hard_subtitle.aeneas.zh-en.srt")
OUT_DIR = Path(r"tmp\camellia_letter_scenes\8_realistic_vision_scene_first_v2")
RAW_DIR = OUT_DIR / "raw"
FINAL_DIR = OUT_DIR / "final_1080p"
LOG_PATH = OUT_DIR / "generation.log"
MANIFEST_PATH = OUT_DIR / "manifest.json"
TARGET_SIZE = (1920, 1080)

STYLE = (
    "photorealistic cinematic Japanese drama film still, environmental storytelling, professional book-summary video visual, "
    "warm natural light, quiet emotional realism, realistic lived-in details, 35mm lens, 16:9, no readable text"
)

CHARACTER = (
    "one East Asian woman, early 30s, short black bob haircut, cream cardigan, white top, dark skirt"
)

NEGATIVE = (
    "close-up portrait, headshot, studio portrait, passport photo, plain wall background, empty beige wall, glamour shot, "
    "two people, three people, crowd, duplicate person, identical twins, clone, same face twice, extra woman, extra protagonist, "
    "kimono, yukata, red robe, ancient costume, school uniform on protagonist, readable text, letters with readable text, signboard text, "
    "watermark, logo, caption, subtitles, random characters, garbled characters, anime, cartoon, illustration, painting, low detail, blurry, "
    "bad anatomy, deformed hands, extra fingers, distorted face, plastic skin"
)

BEATS = [
    {
        "title": "Opening the letter shop",
        "scene": (
            "wide exterior morning shot of a narrow Kamakura side street; a tiny stationery shop door is half open; shelves of envelopes visible inside; "
            "red camellia pot near the threshold; the woman is small in frame, turning the key, not looking at camera"
        ),
        "theme": "story opens quietly, the letter shop becomes a doorway into memory",
    },
    {
        "title": "Desk and tools",
        "scene": (
            "medium environmental shot inside the shop; wooden desk covered with blank envelopes, fountain pen, ink bottle, tea cup, red camellia vase; "
            "the woman writes at the desk in three-quarter side view, room details dominate the frame"
        ),
        "theme": "letters and tools carry unsaid feelings",
    },
    {
        "title": "Breakfast tension",
        "scene": (
            "wide kitchen morning shot; table with two untouched rice bowls, miso soup, school bag on a chair, unopened letter beside a cup; "
            "the woman stands by the sink in the background, small in frame, domestic pressure implied by objects"
        ),
        "theme": "family roles and silence create pressure",
    },
    {
        "title": "Doorway distance",
        "scene": (
            "wide tatami room shot with a half-open sliding door; school shoes and a school bag sit near the doorway; "
            "the woman sits alone by a low table, separated from the doorway by empty space"
        ),
        "theme": "mother and daughter distance is shown without another visible person",
    },
    {
        "title": "Old letter box",
        "scene": (
            "medium low-angle tatami storage room; old wooden box opened on the floor, bundles of blank aged letters tied with string, camellia-pattern cloth; "
            "the woman kneels beside the box, hands reaching toward the letters, face not centered"
        ),
        "theme": "old letters reveal family memory and grief",
    },
    {
        "title": "Ferry journey",
        "scene": (
            "wide ferry deck shot; grey-blue sea, railings, wind, distant island silhouette; the woman stands alone near the railing holding a wrapped wooden box; "
            "her body is full length or half length, the deck and sea are important"
        ),
        "theme": "journey from shop life toward the island and the past",
    },
    {
        "title": "Blocked night letter",
        "scene": (
            "medium night room shot; brass desk lamp, rain-streaked window, blank paper, fountain pen paused, tea cup, scattered camellia petals; "
            "the woman sits at the desk in side view, the blank page is the focal point"
        ),
        "theme": "respect for words, being unable to write is also honest",
    },
    {
        "title": "Sea healing",
        "scene": (
            "wide sunrise seaside overlook; wooden rail, open sea, low orange light, red camellias in foreground; "
            "the woman is seen from behind, small in frame, facing the horizon"
        ),
        "theme": "quiet healing and unsent letters finding a place",
    },
]


def log(message):
    line = f"{time.strftime('%Y-%m-%d %H:%M:%S')} {message}"
    print(line, flush=True)
    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
    with LOG_PATH.open("a", encoding="utf-8") as f:
        f.write(line + "\n")


def read_cues():
    text = SRT_PATH.read_text(encoding="utf-8", errors="strict")
    cues = []
    for block in re.split(r"\n\s*\n", text.replace("\r\n", "\n").replace("\r", "\n")):
        lines = [line.strip() for line in block.split("\n") if line.strip()]
        if len(lines) < 3 or "-->" not in lines[1]:
            continue
        body = "".join(line for line in lines[2:] if re.search(r"[\u4e00-\u9fff]", line))
        body = re.sub(r"\s+", "", body)
        if body:
            cues.append(body)
    return cues


def split_segments(count=8):
    cues = read_cues()
    total = len(cues)
    segments = []
    for i in range(count):
        start = round(i * total / count)
        end = round((i + 1) * total / count)
        segments.append({"index": i + 1, "cueStart": start + 1, "cueEnd": end, "text": "".join(cues[start:end])[:180]})
    return segments


def build_prompts():
    prompts = []
    for seg, beat in zip(split_segments(8), BEATS):
        prompt = (
            f"{STYLE}. Scene {seg['index']:02d}/8, {beat['title']}. "
            f"Primary instruction: {beat['scene']}. "
            f"Visible character rule: {CHARACTER}; exactly one visible person, never a close-up portrait, never looking directly at camera. "
            f"Emotional theme: {beat['theme']}. Blank paper only, no readable words."
        )
        prompts.append({**seg, "title": beat["title"], "prompt": prompt, "negativePrompt": NEGATIVE})
    return prompts


def call_image(info):
    payload = {
        "prompt": info["prompt"],
        "negative_prompt": info["negativePrompt"],
        "size": "768x432",
        "n": 1,
        "steps": 30,
        "guidance_scale": 7.5,
        "seed": 940000 + info["index"] * 53,
    }
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    req = urlrequest.Request(ENDPOINT, data=body, headers={"Content-Type": "application/json"}, method="POST")
    start = time.time()
    with urlrequest.urlopen(req, timeout=600) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    elapsed = round(time.time() - start, 2)
    item = data["data"][0]
    image_bytes = base64.b64decode(item["b64_json"])
    item.pop("b64_json", None)
    return elapsed, item, image_bytes, data


def image_metrics(path):
    im = Image.open(path).convert("RGB")
    small = im.resize((256, 144))
    colors = small.getcolors(maxcolors=65536) or []
    total = small.width * small.height
    return {
        "width": im.width,
        "height": im.height,
        "colors": len(colors),
        "nearBlackRatio": round(sum(c for c, (r, g, b) in colors if r < 12 and g < 12 and b < 12) / total, 3),
        "nearWhiteRatio": round(sum(c for c, (r, g, b) in colors if r > 245 and g > 245 and b > 245) / total, 3),
    }


def mac_pressure():
    try:
        cmd = "vm_stat | head -8; ps -Ao pid,pcpu,pmem,rss,comm | sort -nrk2 | head -10; curl -sS http://127.0.0.1:30020/health || true"
        result = subprocess.run(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=8", "macmini4", cmd], capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
        return result.stdout.strip()
    except Exception as exc:
        return f"pressure_unavailable: {exc!r}"


def make_sheet(results):
    cols = 4
    thumb_w, thumb_h = 480, 270
    label_h = 34
    sheet = Image.new("RGB", (cols * thumb_w, 2 * (thumb_h + label_h)), "white")
    draw = ImageDraw.Draw(sheet)
    for i, result in enumerate(results):
        img = Image.open(result["finalPath"]).convert("RGB").resize((thumb_w, thumb_h))
        x = (i % cols) * thumb_w
        y = (i // cols) * (thumb_h + label_h)
        sheet.paste(img, (x, y))
        draw.rectangle([x, y + thumb_h, x + thumb_w, y + thumb_h + label_h], fill=(245, 245, 242))
        draw.text((x + 8, y + thumb_h + 9), f"{result['index']:02d} {result['title']} {result['elapsedSecondsClient']}s", fill=(20, 20, 20))
    path = OUT_DIR / "contact_sheet.jpg"
    sheet.save(path, quality=92)
    return path


def main():
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    FINAL_DIR.mkdir(parents=True, exist_ok=True)
    prompts = build_prompts()
    (OUT_DIR / "prompts_8.json").write_text(json.dumps(prompts, ensure_ascii=False, indent=2), encoding="utf-8")
    log(f"SRT={SRT_PATH}")
    log(f"endpoint={ENDPOINT}")
    log(f"prompts={OUT_DIR / 'prompts_8.json'}")
    log(f"start pressure snapshot:\n{mac_pressure()}")
    results = []
    for info in prompts:
        idx = info["index"]
        raw_path = RAW_DIR / f"{idx:02d}_{info['cueStart']:04d}_{info['cueEnd']:04d}.png"
        final_path = FINAL_DIR / f"{idx:02d}_{info['cueStart']:04d}_{info['cueEnd']:04d}_1080p.jpg"
        response_path = OUT_DIR / f"{idx:02d}_response.json"
        log(f"[{idx:02d}/8] generating {info['title']} cue={info['cueStart']}-{info['cueEnd']}")
        elapsed, item, image_bytes, response_data = call_image(info)
        raw_path.write_bytes(image_bytes)
        response_path.write_text(json.dumps(response_data, ensure_ascii=False, indent=2), encoding="utf-8")
        Image.open(raw_path).convert("RGB").resize(TARGET_SIZE, Image.Resampling.LANCZOS).save(final_path, quality=95)
        raw_metrics = image_metrics(raw_path)
        final_metrics = image_metrics(final_path)
        ok = final_metrics["width"] == 1920 and final_metrics["height"] == 1080 and final_path.stat().st_size >= 250000 and final_metrics["colors"] >= 4000
        result = {
            **info,
            "ok": ok,
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
            "serviceMetrics": item.get("metrics"),
            "model": item.get("model"),
            "pressureAfter": mac_pressure(),
        }
        results.append(result)
        MANIFEST_PATH.write_text(
            json.dumps(
                {
                    "endpoint": ENDPOINT,
                    "srtPath": str(SRT_PATH),
                    "rawSize": "768x432",
                    "targetSize": "1920x1080",
                    "completedCount": len(results),
                    "allPassedMetrics": len(results) == 8 and all(r["ok"] for r in results),
                    "results": results,
                },
                ensure_ascii=False,
                indent=2,
            ),
            encoding="utf-8",
        )
        log(f"[{idx:02d}/8] done elapsed={elapsed}s server={item.get('elapsedSeconds')}s final={final_path.name} {final_path.stat().st_size//1024}KB ok={ok}")
    sheet = make_sheet(results)
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    manifest["contactSheet"] = str(sheet.resolve())
    manifest["finishedAt"] = time.strftime("%Y-%m-%d %H:%M:%S")
    manifest["allPassedMetrics"] = all(r["ok"] for r in results)
    MANIFEST_PATH.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    log(f"finished contact_sheet={sheet.resolve()}")
    log(f"final pressure snapshot:\n{mac_pressure()}")


if __name__ == "__main__":
    main()
