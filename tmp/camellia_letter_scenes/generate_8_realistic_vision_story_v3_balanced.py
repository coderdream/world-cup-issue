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
OUT_DIR = Path(r"tmp\camellia_letter_scenes\8_realistic_vision_balanced_v3")
RAW_DIR = OUT_DIR / "raw"
FINAL_DIR = OUT_DIR / "final_1080p"
LOG_PATH = OUT_DIR / "generation.log"
MANIFEST_PATH = OUT_DIR / "manifest.json"
TARGET_SIZE = (1920, 1080)

STYLE = (
    "photorealistic cinematic Japanese drama film still, medium-wide environmental shot, professional book-summary video visual, "
    "warm natural light, quiet emotional realism, realistic lived-in details, 35mm lens, 16:9, no readable text"
)

CHARACTER = (
    "same woman visible in every scene, East Asian woman early 30s, short black bob haircut with soft bangs, cream knit cardigan, white top, dark skirt"
)

FRAME_RULE = (
    "The woman must be clearly visible as a full-body or half-body figure, occupying about 20 to 35 percent of the frame. "
    "She must be interacting with the environment, not posing for camera, not a close-up portrait."
)

NEGATIVE = (
    "no person, empty room, empty landscape, close-up portrait, headshot, studio portrait, passport photo, plain wall background, "
    "face filling frame, looking directly at camera, glamour shot, two people, three people, crowd, duplicate person, identical twins, clone, "
    "same face twice, extra woman, extra protagonist, kimono, yukata, red robe, ancient costume, readable text, letters with readable text, "
    "signboard text, watermark, logo, caption, subtitles, random characters, garbled characters, anime, cartoon, illustration, painting, "
    "low detail, blurry, bad anatomy, deformed hands, extra fingers, distorted face, plastic skin"
)

BEATS = [
    ("Opening the letter shop", "the woman unlocks a tiny Kamakura stationery shop at dawn; narrow side street, paper door, red camellia pot, shelves of envelopes visible inside"),
    ("Desk and tools", "the woman writes at a wooden shop desk; blank envelopes, fountain pen, ink bottle, tea cup, red camellia vase, window light"),
    ("Breakfast tension", "the woman stands in a small kitchen holding an unopened letter; two untouched rice bowls and a school bag on a chair imply family pressure"),
    ("Doorway distance", "the woman sits alone at a low table in a tatami room; half-open sliding door, school shoes and bag near the doorway create emotional distance"),
    ("Old letter box", "the woman kneels beside an old wooden box full of tied blank letters on tatami; camellia-pattern cloth, dust in warm side light"),
    ("Ferry journey", "the woman stands alone on a ferry deck holding a wrapped wooden box; grey-blue sea, railings, distant island silhouette, windy cardigan"),
    ("Blocked night letter", "the woman sits at a night desk under a brass lamp; blank paper is central, rain-streaked window, tea cup, camellia petals"),
    ("Sea healing", "the woman stands at a seaside overlook at sunrise, seen from behind or three-quarter back; wooden rail, open sea, red camellias in foreground"),
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
    return [
        {"index": i + 1, "cueStart": round(i * total / count) + 1, "cueEnd": round((i + 1) * total / count), "text": "".join(cues[round(i * total / count):round((i + 1) * total / count)])[:120]}
        for i in range(count)
    ]


def build_prompts():
    prompts = []
    for seg, (title, scene) in zip(split_segments(), BEATS):
        prompt = (
            f"{STYLE}. Scene {seg['index']:02d}/8, {title}. "
            f"Primary scene action: {scene}. {CHARACTER}. {FRAME_RULE} "
            "Use blank paper and blank envelopes only, no readable words. Keep the composition cinematic and story-driven."
        )
        prompts.append({**seg, "title": title, "prompt": prompt, "negativePrompt": NEGATIVE})
    return prompts


def call_image(info):
    payload = {
        "prompt": info["prompt"],
        "negative_prompt": info["negativePrompt"],
        "size": "768x432",
        "n": 1,
        "steps": 30,
        "guidance_scale": 7.8,
        "seed": 950000 + info["index"] * 71,
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
