import base64
import json
import re
import subprocess
import time
from pathlib import Path
from urllib import request as urlrequest

from PIL import Image, ImageDraw


ENDPOINT = "http://100.96.199.26:30019/v1/images/generations"
SRT_PATH = Path(r"D:\books\0625新书四本\2025-01《山茶的情书》\output\hard_subtitle.aeneas.zh-en.srt")
OUT_DIR = Path(r"tmp\camellia_letter_scenes\24_1080p_v4_single_subject")
RAW_DIR = OUT_DIR / "raw"
FINAL_DIR = OUT_DIR / "final_1080p"
LOG_PATH = OUT_DIR / "generation.log"
MANIFEST_PATH = OUT_DIR / "manifest.json"
TARGET_SIZE = (1920, 1080)

STYLE = (
    "photorealistic cinematic Japanese drama film still, professional 30-minute book-summary video visual, "
    "warm natural movie lighting, detailed lived-in interior, emotional quiet realism, Kamakura letter shop mood, "
    "red camellia motif, paper letters and fountain pen, 16:9, no readable text"
)

CHARACTER = (
    "One visible main protagonist only: East Asian woman in her early 30s, shoulder-length black hair, cream cardigan, calm tired eyes. "
    "Do not show any second person. Do not show twins. Do not show crowds."
)

NEGATIVE = (
    "two people, three people, crowd, extra person, extra woman, extra protagonist, duplicate person, identical twins, clone, same face twice, "
    "mother visible, daughter visible, group portrait, landscape painting, empty postcard panorama, mountain panorama, temple panorama, "
    "readable text, watermark, logo, caption, comic, anime, cartoon, simple line drawing, low detail, blurry, bad anatomy, deformed hands, extra fingers"
)

BEATS = [
    ("Opening the shop", "medium shot from behind shoulder, protagonist unlocks a small stationery shop at dawn, envelopes visible through the doorway, camellia pot beside the threshold"),
    ("Ready room", "no person still life, wooden counter with envelopes, ink bottle, fountain pen, old brass lamp, red camellia in a ceramic vase, morning light"),
    ("Breakfast pressure", "single protagonist at kitchen table holding an unopened letter, two untouched bowls across from her imply absent family, tense morning light"),
    ("Daughter distance", "single protagonist seated near a half-open sliding door, a school bag left in the doorway, emotional distance without showing the daughter"),
    ("Fireworks afterglow", "single protagonist on wooden porch at night after fireworks, folded letter on her lap, fading smoke and warm lantern light"),
    ("Hot wine winter", "single protagonist in winter kitchen warming hands around a mug, small pot with orange peel and cinnamon, letter beside the stove"),
    ("Wooden box", "single protagonist kneels on tatami opening an old wooden box of tied letters, dust in warm side light, camellia-pattern cloth"),
    ("Ferry crossing", "single protagonist on ferry deck holding the wooden box close, wind in her cardigan, grey-blue sea behind her, cinematic medium shot"),
    ("Harbor arrival", "single protagonist steps onto a wet island pier with the wrapped box, fishing ropes and small boats, cloudy light"),
    ("Camellia road", "single protagonist walks on a narrow winter island road, red camellia petals on dark volcanic soil, box under her arm"),
    ("Letter ritual", "no face close-up still life, hands arranging old letters beside incense, red camellia, ceramic bowl, farewell mood"),
    ("Blank paper", "single protagonist at night desk, pen hovering over blank paper, face reflected faintly in dark window, brass lamp light"),
    ("Spring leaving", "single protagonist stands inside front door, school shoes and a school bag just outside imply daughter leaving, spring light, camellia branch"),
    ("Afternoon burnout", "single protagonist behind shop counter in late afternoon, unanswered letters stacked neatly, long shadows, tired posture"),
    ("Seaside breath", "single protagonist on rainy seaside path holding an envelope, wet pavement close foreground, camellia bushes, not a wide panorama"),
    ("Unwritten request", "single protagonist alone in tea shop booth looking at a blank page and an untouched second teacup, gentle sea light"),
    ("Dictation memory", "close-up of protagonist writing with fountain pen, an older hand implied only by a folded cardigan on the chair, no second person visible"),
    ("Reconciliation object", "single protagonist sits before a low table with old letters and an empty chair across from her, softened expression, dusk light"),
    ("Bloom letter", "no person macro still life, red camellia blossom beside sealed envelope and fountain pen, soft window light"),
    ("Rain reflection", "single protagonist reflected in rainy window glass, unfinished letter on sill, reflection faint and clearly same person"),
    ("Self letter", "single protagonist at midnight desk writing to herself, candle, tea, camellia petals, intimate brave mood"),
    ("Reopening dawn", "single protagonist lifts shop curtain and opens the door at dawn, shelves glow warmly, new rhythm"),
    ("Healing sea", "single protagonist at seaside overlook seen from back, close foreground camellias, morning sea, hopeful but not postcard-like"),
    ("Closing still life", "no person final cover still life, sealed letter, notebook, fountain pen, red camellia, ferry ticket stub, sunrise window"),
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


def split_segments(cues, count=24):
    segments = []
    total = len(cues)
    for i in range(count):
        start = round(i * total / count)
        end = round((i + 1) * total / count)
        segments.append({"index": i + 1, "cueStart": start + 1, "cueEnd": end, "text": "".join(cues[start:end])[:140]})
    return segments


def build_prompts():
    prompts = []
    for seg, (title, scene) in zip(split_segments(read_cues()), BEATS):
        no_people = scene.startswith("no person")
        prompt = (
            f"{STYLE}. {'' if no_people else CHARACTER} Scene {seg['index']:02d}/24, {title}. "
            f"Specific shot: {scene}. Connect to subtitle theme without drawing readable words: {seg['text']}"
        )
        prompts.append({**seg, "title": title, "prompt": prompt, "negativePrompt": NEGATIVE})
    return prompts


def mac_pressure():
    try:
        cmd = "vm_stat | head -8; ps -Ao pid,pcpu,pmem,rss,comm | sort -nrk2 | head -8; lsof -nP -iTCP:30019 -sTCP:LISTEN || true"
        result = subprocess.run(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=8", "macmini4", cmd], capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
        return result.stdout.strip()
    except Exception as exc:
        return f"pressure_unavailable: {exc!r}"


def call_image(info):
    payload = {
        "prompt": info["prompt"],
        "negative_prompt": info["negativePrompt"],
        "size": "1024x576",
        "n": 1,
        "steps": 14,
        "guidance_scale": 3.4,
        "seed": 610000 + info["index"] * 113,
    }
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    req = urlrequest.Request(ENDPOINT, data=body, headers={"Content-Type": "application/json"}, method="POST")
    start = time.time()
    with urlrequest.urlopen(req, timeout=300) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    elapsed = round(time.time() - start, 2)
    item = data["data"][0]
    image_bytes = base64.b64decode(item["b64_json"])
    item.pop("b64_json", None)
    return elapsed, item, image_bytes, data


def metrics(path):
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


def make_sheet(results):
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
        draw.text((x + 8, y + thumb_h + 9), f"{result['index']:02d} {result['title']} {result['elapsedSecondsClient']}s", fill=(20, 20, 20))
    path = OUT_DIR / "contact_sheet.jpg"
    sheet.save(path, quality=92)
    return path


def write_manifest(results):
    MANIFEST_PATH.write_text(
        json.dumps(
            {
                "endpoint": ENDPOINT,
                "srtPath": str(SRT_PATH),
                "rawSize": "1024x576",
                "targetSize": "1920x1080",
                "completedCount": len(results),
                "allPassedMetrics": len(results) == 24 and all(r["ok"] for r in results),
                "results": results,
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )


def main():
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    FINAL_DIR.mkdir(parents=True, exist_ok=True)
    prompts = build_prompts()
    (OUT_DIR / "prompts_24.json").write_text(json.dumps(prompts, ensure_ascii=False, indent=2), encoding="utf-8")
    log(f"SRT={SRT_PATH}")
    log(f"prompts={OUT_DIR / 'prompts_24.json'}")
    log(f"start pressure snapshot:\n{mac_pressure()}")
    results = []
    for info in prompts:
        idx = info["index"]
        raw_path = RAW_DIR / f"{idx:02d}_{info['cueStart']:04d}_{info['cueEnd']:04d}.png"
        final_path = FINAL_DIR / f"{idx:02d}_{info['cueStart']:04d}_{info['cueEnd']:04d}_1080p.jpg"
        response_path = OUT_DIR / f"{idx:02d}_response.json"
        log(f"[{idx:02d}/24] generating {info['title']} cue={info['cueStart']}-{info['cueEnd']}")
        elapsed, item, image_bytes, response_data = call_image(info)
        raw_path.write_bytes(image_bytes)
        response_path.write_text(json.dumps(response_data, ensure_ascii=False, indent=2), encoding="utf-8")
        Image.open(raw_path).convert("RGB").resize(TARGET_SIZE, Image.Resampling.LANCZOS).save(final_path, quality=95)
        raw_metrics = metrics(raw_path)
        final_metrics = metrics(final_path)
        ok = final_metrics["width"] == 1920 and final_metrics["height"] == 1080 and final_path.stat().st_size >= 350000 and final_metrics["colors"] >= 4000
        results.append({
            **info,
            "ok": ok,
            "status": 200,
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
        })
        write_manifest(results)
        log(f"[{idx:02d}/24] done elapsed={elapsed}s server={item.get('elapsedSeconds')}s final={final_path.name} {final_path.stat().st_size//1024}KB ok={ok}")
    sheet = make_sheet(results)
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    manifest["contactSheet"] = str(sheet.resolve())
    manifest["finishedAt"] = time.strftime("%Y-%m-%d %H:%M:%S")
    MANIFEST_PATH.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    log(f"finished contact_sheet={sheet.resolve()}")
    log(f"final pressure snapshot:\n{mac_pressure()}")


if __name__ == "__main__":
    main()
