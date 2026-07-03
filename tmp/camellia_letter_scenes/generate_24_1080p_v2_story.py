import base64
import json
import re
import subprocess
import time
from pathlib import Path
from urllib import request as urlrequest
from urllib.error import HTTPError

from PIL import Image, ImageDraw


ENDPOINT = "http://100.96.199.26:30019/v1/images/generations"
SRT_PATH = Path(r"D:\books\0625新书四本\2025-01《山茶的情书》\output\hard_subtitle.aeneas.zh-en.srt")
OUT_DIR = Path(r"tmp\camellia_letter_scenes\24_1080p_v2_story")
RAW_DIR = OUT_DIR / "raw"
FINAL_DIR = OUT_DIR / "final_1080p"
LOG_PATH = OUT_DIR / "generation.log"
MANIFEST_PATH = OUT_DIR / "manifest.json"

TARGET_SIZE = (1920, 1080)
RAW_SIZE = "1024x576"

STYLE = (
    "cinematic realistic watercolor book illustration, professional visual for a Chinese 30-minute book-summary video, "
    "quiet Kamakura and Izu Oshima atmosphere, Japanese seaside town, camellia flowers, paper letters, fountain pens, "
    "warm natural film lighting, detailed environment, emotional healing tone, soft but rich colors, storybook realism, "
    "consistent art direction across a 24-image sequence, 16:9, no readable text, no logo, no watermark"
)

CHARACTER_GUIDE = (
    "Main protagonist: one East Asian woman in her early 30s, shoulder-length black hair, soft cream cardigan, calm tired eyes. "
    "Mother: East Asian woman in her late 50s with short grey hair and muted green cardigan, clearly older than the protagonist. "
    "Daughter: teenage East Asian girl with school uniform and ponytail, clearly much younger. "
    "When a scene includes multiple people, make them different ages, different clothes, different hairstyles, different faces."
)

NEGATIVE = (
    "readable text, watermark, logo, caption, UI, comic panel, split screen, flowchart, diagram, simple geometric lines, "
    "low detail, blurry, black image, blank image, monochrome, oversaturated, plastic skin, deformed hands, extra fingers, "
    "bad anatomy, distorted face, ugly, identical twins, duplicate person, clone, same face twice, two identical women, "
    "mirrored person, repeated face, crowd of similar people, extra protagonist, extra woman beside protagonist"
)

STORY_BEATS = [
    {
        "title": "Kamakura morning opening",
        "scene": (
            "wide exterior establishing shot of a narrow Kamakura street at early morning, the small Tsubaki stationery shop opens, "
            "red camellia shrubs near the door, the single protagonist stands outside holding a key and a paper bag, hopeful but uncertain"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Stationery shop interior",
        "scene": (
            "interior still life of the letter-writing shop, wooden shelves of envelopes, ink bottles, fountain pens, a camellia in a vase, "
            "sunlight through a paper window, no person, the room itself feels ready for stories"
        ),
        "people": "no people",
    },
    {
        "title": "Family breakfast pressure",
        "scene": (
            "small family kitchen in the morning, rice bowls and miso soup on the table, the protagonist tries to manage breakfast while "
            "the older mother sits nearby with a letter, domestic pressure and tenderness in the same frame"
        ),
        "people": "two women only, protagonist and clearly older mother",
    },
    {
        "title": "Teenage daughter at doorway",
        "scene": (
            "narrow hallway with sliding door half open, a teenage daughter in school uniform stands at the doorway with a guarded expression, "
            "the protagonist is farther inside the room, emotional distance shown by the architecture"
        ),
        "people": "two people only, adult protagonist and teenage daughter, very different ages",
    },
    {
        "title": "Fireworks afterglow",
        "scene": (
            "summer night porch after fireworks, smoke fading above distant rooftops, the protagonist watches alone from the wooden veranda, "
            "a folded letter on her lap and camellia leaves in shadow"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Winter hot wine kitchen",
        "scene": (
            "winter kitchen close scene, steam rising from a small pot of spiced hot wine, orange peel and cinnamon, the protagonist warms her hands, "
            "a letter waits unopened beside the stove"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Wooden box discovery",
        "scene": (
            "dusty storage corner, an old wooden box opened on tatami, bundles of aged letters tied with cord, camellia-pattern cloth, "
            "the protagonist kneels and discovers family history"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Ferry to Izu Oshima",
        "scene": (
            "wide ferry deck crossing grey-blue sea toward Izu Oshima, wind moves the protagonist's hair and cream cardigan, "
            "she holds the wooden box close, island silhouette ahead"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Cloudy harbor arrival",
        "scene": (
            "cloudy island harbor arrival, ropes, gulls, wet concrete pier, small fishing boats, the protagonist steps off the ferry with the box, "
            "the mood shifts from memory to journey"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Camellia island road",
        "scene": (
            "winter island road lined with camellia trees, fallen red petals on dark volcanic soil, the protagonist walks alone under a pale sky, "
            "the wooden box wrapped in cloth under her arm"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Old letters ritual",
        "scene": (
            "quiet ritual still life by a low table, old letters arranged carefully with incense smoke, red camellia blossoms and a ceramic bowl, "
            "hands only, the sense of farewell without showing faces"
        ),
        "people": "hands only, no faces",
    },
    {
        "title": "Blocked at blank paper",
        "scene": (
            "night desk scene from an oblique angle, blank paper under a brass lamp, fountain pen hovering but not writing, "
            "the protagonist's face reflected faintly in the dark window, creative block and respect for words"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Spring daughter leaving",
        "scene": (
            "spring morning front door, teenage daughter in school uniform leaves home with school bag, cherry and camellia branches outside, "
            "the protagonist watches from inside with complicated love"
        ),
        "people": "two people only, adult protagonist and teenage daughter, very different ages",
    },
    {
        "title": "Empty shop afternoon burnout",
        "scene": (
            "empty stationery shop in late afternoon, long shadows across shelves and envelopes, the protagonist sits behind the counter with tired posture, "
            "unanswered letters stacked neatly"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Seaside walk alone",
        "scene": (
            "wide seaside path after rain, Kamakura coastline and wet pavement, the protagonist walks alone holding an envelope, "
            "camellia bushes beside the path, quiet room to breathe"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Tea shop conversation",
        "scene": (
            "small tea shop near the sea, the protagonist sits across from an older man with silver hair and navy jacket, teacups between them, "
            "they speak gently about an unwritten letter"
        ),
        "people": "two people only, protagonist and clearly older man, different gender and age",
    },
    {
        "title": "Letter dictation hands",
        "scene": (
            "close-up of hands during letter dictation, one older hand rests near a teacup, the protagonist's hand writes with fountain pen, "
            "camellia petal on the table, faces out of frame"
        ),
        "people": "hands only, no full faces",
    },
    {
        "title": "Family reconciliation",
        "scene": (
            "warm family room at dusk, protagonist and older mother sit at opposite sides of a low table with old letters between them, "
            "their faces are visibly different ages, forgiveness begins quietly"
        ),
        "people": "two women only, protagonist and clearly older mother, not twins",
    },
    {
        "title": "Camellia bloom and letter",
        "scene": (
            "macro cinematic still life, red camellia blossom blooming beside a sealed envelope and fountain pen, soft sea light from a window, "
            "symbol of love that waits"
        ),
        "people": "no people",
    },
    {
        "title": "Rain window reflection",
        "scene": (
            "rainy evening window, the protagonist's subtle reflection appears in glass while raindrops blur the seaside street outside, "
            "an unfinished letter lies on the sill"
        ),
        "people": "single protagonist only, reflection is faint and not a second person",
    },
    {
        "title": "Night self-letter",
        "scene": (
            "quiet midnight desk, the protagonist writes a letter to herself, one candle, one cup of tea, camellia petals, "
            "the room feels intimate and brave"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Dawn shop reopening",
        "scene": (
            "dawn light entering the stationery shop, protagonist lifts the shop curtain and opens the door, shelves glow warmly, "
            "new day and recovered rhythm"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Seaside healing wide shot",
        "scene": (
            "wide final journey shot on a quiet seaside overlook, protagonist stands alone facing the morning sea, wind moving her cardigan, "
            "camellias in foreground, open sky suggests healing"
        ),
        "people": "single protagonist only",
    },
    {
        "title": "Final cover still life",
        "scene": (
            "cover-like final still life on a wooden desk by the window, sealed letter, notebook, fountain pen, red camellia, ferry ticket stub, "
            "warm sunrise over the sea outside, elegant book-video closing image"
        ),
        "people": "no people",
    },
]


def log(message):
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    line = f"{timestamp} {message}"
    print(line, flush=True)
    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
    with LOG_PATH.open("a", encoding="utf-8") as f:
        f.write(line + "\n")


def read_srt_text():
    text = SRT_PATH.read_text(encoding="utf-8", errors="strict")
    blocks = re.split(r"\n\s*\n", text.replace("\r\n", "\n").replace("\r", "\n"))
    cues = []
    for block in blocks:
        lines = [line.strip() for line in block.split("\n") if line.strip()]
        if len(lines) < 3 or "-->" not in lines[1]:
            continue
        chinese = "".join(line for line in lines[2:] if re.search(r"[\u4e00-\u9fff]", line))
        chinese = re.sub(r"\s+", "", chinese)
        if chinese:
            cues.append(chinese)
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
        segments.append({"index": i + 1, "cueStart": start + 1, "cueEnd": end, "text": text[:260]})
    return segments


def build_prompts(segments):
    prompts = []
    for segment, beat in zip(segments, STORY_BEATS):
        subtitle_hint = segment["text"][:150]
        prompt = (
            f"{STYLE}. {CHARACTER_GUIDE} Scene {segment['index']:02d}/24, story beat: {beat['title']}. "
            f"Composition: {beat['scene']}. People rule: {beat['people']}. "
            f"Keep this image narratively connected to the nearby subtitle meaning, but do not draw readable words: {subtitle_hint}"
        )
        prompts.append({**segment, "title": beat["title"], "prompt": prompt, "negativePrompt": NEGATIVE})
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
        "steps": 12,
        "guidance_scale": 2.6,
        "seed": 420000 + prompt_info["index"] * 17,
    }
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    req = urlrequest.Request(ENDPOINT, data=body, headers={"Content-Type": "application/json"}, method="POST")
    start = time.time()
    last_error = None
    for attempt in range(1, 4):
        try:
            with urlrequest.urlopen(req, timeout=300) as resp:
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
            f"{result['index']:02d} {result['title']} {result['elapsedSecondsClient']}s",
            fill=(20, 20, 20),
        )
    path = OUT_DIR / "contact_sheet.jpg"
    sheet.save(path, quality=92)
    return path


def write_manifest(results):
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
        log(f"[{idx:02d}/24] generating {prompt_info['title']} cue={prompt_info['cueStart']}-{prompt_info['cueEnd']}")
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
        write_manifest(results)
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
