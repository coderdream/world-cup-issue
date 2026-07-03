import base64
import json
import re
import subprocess
import time
from pathlib import Path
from urllib import request as urlrequest

from PIL import Image, ImageDraw


ENDPOINT = "http://100.96.199.26:30020/v1/images/generations"
SRT_PATH = Path(r"D:\books\0625新书四本\2025-01《山茶的情书》\output\hard_subtitle.aeneas.zh-en.srt")
OUT_DIR = Path(r"tmp\camellia_letter_scenes\8_realistic_vision_character_v1")
RAW_DIR = OUT_DIR / "raw"
FINAL_DIR = OUT_DIR / "final_1080p"
LOG_PATH = OUT_DIR / "generation.log"
MANIFEST_PATH = OUT_DIR / "manifest.json"
TARGET_SIZE = (1920, 1080)

CHARACTER_BIBLE = (
    "same main character in every image: East Asian woman in her early 30s, shoulder-length straight black bob haircut with soft bangs, "
    "cream knit cardigan over a plain white top, dark skirt, calm tired eyes, natural face, no heavy makeup"
)

STYLE = (
    "photorealistic cinematic Japanese drama film still, professional book-summary video visual, warm natural light, "
    "quiet emotional realism, shallow depth of field, detailed lived-in environment, 16:9, no readable text"
)

NEGATIVE = (
    "two people, three people, crowd, duplicate person, identical twins, clone, same face twice, extra woman, extra protagonist, "
    "kimono, yukata, red robe, ancient costume, school uniform on protagonist, readable text, letters with readable text, signboard text, "
    "watermark, logo, caption, subtitles, random characters, garbled characters, anime, cartoon, illustration, painting, low detail, blurry, "
    "bad anatomy, deformed hands, extra fingers, distorted face, plastic skin"
)

BEATS = [
    (
        "Opening the letter shop",
        "medium shot from behind her shoulder as she unlocks a small Kamakura stationery shop at dawn, envelopes and fountain pens visible inside, a red camellia pot beside the door",
    ),
    (
        "Quiet shop interior",
        "single woman sitting at a wooden desk inside the stationery shop, writing on a blank sheet with fountain pen, sealed blank envelopes and red camellia in a small vase",
    ),
    (
        "Family pressure implied",
        "single woman in a small morning kitchen holding an unopened letter, two untouched rice bowls across the table imply family tension, soft window light",
    ),
    (
        "Daughter distance implied",
        "single woman seated near a half-open sliding door, a school bag and small shoes left in the doorway imply a teenage daughter leaving, quiet emotional distance",
    ),
    (
        "Old letters discovery",
        "single woman kneeling on tatami floor opening an old wooden box full of tied blank letters, dust in warm side light, red camellia-pattern cloth",
    ),
    (
        "Ferry to the island",
        "single woman standing alone on a ferry deck holding the wrapped wooden box close, grey-blue sea and island silhouette behind her, wind moves her cream cardigan",
    ),
    (
        "Blocked letter at night",
        "single woman at a night desk under a brass lamp, pen hovering above blank paper, rain on the dark window, one cup of tea, red camellia petals",
    ),
    (
        "Healing by the sea",
        "single woman seen from behind at a seaside overlook at sunrise, hands on wooden rail, open sea ahead, red camellias in foreground, hopeful ending mood",
    ),
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


def split_segments(cues, count=8):
    total = len(cues)
    segments = []
    for i in range(count):
        start = round(i * total / count)
        end = round((i + 1) * total / count)
        segments.append({"index": i + 1, "cueStart": start + 1, "cueEnd": end, "text": "".join(cues[start:end])[:180]})
    return segments


def build_prompts():
    prompts = []
    for seg, (title, shot) in zip(split_segments(read_cues(), 8), BEATS):
        prompt = (
            f"{STYLE}. {CHARACTER_BIBLE}. Scene {seg['index']:02d}/8, {title}. "
            f"Specific shot: {shot}. Exactly one visible person. Blank paper only, no readable words. "
            f"Connect to this Chinese subtitle theme without writing text in the image: {seg['text']}"
        )
        prompts.append({**seg, "title": title, "prompt": prompt, "negativePrompt": NEGATIVE})
    return prompts


def call_image(info):
    payload = {
        "prompt": info["prompt"],
        "negative_prompt": info["negativePrompt"],
        "size": "768x432",
        "n": 1,
        "steps": 28,
        "guidance_scale": 7.0,
        "seed": 930000 + info["index"] * 37,
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
    rows = 2
    sheet = Image.new("RGB", (cols * thumb_w, rows * (thumb_h + label_h)), "white")
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
