import base64
import json
import shutil
import time
from pathlib import Path
from urllib import request as urlrequest

from PIL import Image, ImageDraw


ENDPOINT = "http://100.96.199.26:30019/v1/images/generations"
SRC_DIR = Path(r"tmp\camellia_letter_scenes\24_1080p_v4_single_subject")
OUT_DIR = Path(r"tmp\camellia_letter_scenes\24_1080p_v4_revised")
RAW_DIR = OUT_DIR / "raw"
FINAL_DIR = OUT_DIR / "final_1080p"
LOG_PATH = OUT_DIR / "generation.log"
TARGET_SIZE = (1920, 1080)

NEGATIVE = (
    "two people, three people, crowd, duplicate person, identical twins, clone, same face twice, extra woman, extra protagonist, "
    "kimono, yukata, red robe, red kimono, ancient costume, readable text, letters with readable text, signboard text, watermark, logo, "
    "caption, calligraphy, random characters, garbled characters, comic, anime, cartoon, low detail, blurry, deformed hands, extra fingers"
)

REVISIONS = {
    8: {
        "title": "Ferry crossing revised",
        "prompt": (
            "photorealistic cinematic modern Japanese drama film still, 16:9. One East Asian woman in her early 30s, shoulder-length black hair, "
            "cream knit cardigan and dark skirt, stands alone on a ferry deck holding a small wooden box wrapped in cloth. Windy grey-blue sea, island silhouette, "
            "emotional journey mood. Exactly one visible person, no readable text, no kimono."
        ),
    },
    9: {
        "title": "Harbor arrival revised",
        "prompt": (
            "photorealistic cinematic modern Japanese drama film still, 16:9. One East Asian woman in cream cardigan steps from a ferry onto a wet island pier, "
            "holding a wrapped wooden box. Fishing ropes, small boats, cloudy harbor light, close medium shot with real narrative action. Exactly one visible person, no readable text."
        ),
    },
    15: {
        "title": "Seaside breath revised",
        "prompt": (
            "photorealistic cinematic modern Japanese drama film still, 16:9. One East Asian woman in cream cardigan walks alone on a rainy seaside path, "
            "holding a sealed envelope against her chest. Wet pavement reflections, camellia bushes, coastline softly behind her. Exactly one visible person, no readable text."
        ),
    },
    23: {
        "title": "Healing sea revised",
        "prompt": (
            "photorealistic cinematic modern Japanese drama film still, 16:9. One East Asian woman in cream cardigan seen from behind at a seaside overlook at sunrise, "
            "hands resting on a wooden rail, open sky and quiet sea ahead, red camellias in foreground. Exactly one visible person, no readable text, no signboards."
        ),
    },
    24: {
        "title": "Closing still life revised",
        "prompt": (
            "photorealistic cinematic still life for a book-summary video, 16:9. No people. A sealed blank envelope with no writing, a closed notebook with plain cover, "
            "fountain pen, red camellia blossom, small ferry ticket shape with no text, warm sunrise through window. Elegant closing image, no readable text, no characters."
        ),
    },
}


def log(message):
    line = f"{time.strftime('%Y-%m-%d %H:%M:%S')} {message}"
    print(line, flush=True)
    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
    with LOG_PATH.open("a", encoding="utf-8") as f:
        f.write(line + "\n")


def call_image(index, info):
    payload = {
        "prompt": info["prompt"],
        "negative_prompt": NEGATIVE,
        "size": "1024x576",
        "n": 1,
        "steps": 16,
        "guidance_scale": 3.8,
        "seed": 730000 + index * 131,
    }
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    req = urlrequest.Request(ENDPOINT, data=body, headers={"Content-Type": "application/json"}, method="POST")
    start = time.time()
    with urlrequest.urlopen(req, timeout=320) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    elapsed = round(time.time() - start, 2)
    item = data["data"][0]
    image_bytes = base64.b64decode(item["b64_json"])
    item.pop("b64_json", None)
    return elapsed, item, image_bytes, data


def copy_v4():
    FINAL_DIR.mkdir(parents=True, exist_ok=True)
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    for src in (SRC_DIR / "final_1080p").glob("*.jpg"):
        shutil.copy2(src, FINAL_DIR / src.name)
    for src in (SRC_DIR / "raw").glob("*.png"):
        shutil.copy2(src, RAW_DIR / src.name)
    for name in ["prompts_24.json", "manifest.json"]:
        src = SRC_DIR / name
        if src.exists():
            shutil.copy2(src, OUT_DIR / f"source_{name}")


def make_sheet():
    files = sorted(FINAL_DIR.glob("*.jpg"))
    cols = 4
    thumb_w, thumb_h = 480, 270
    label_h = 34
    rows = (len(files) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * thumb_w, rows * (thumb_h + label_h)), "white")
    draw = ImageDraw.Draw(sheet)
    for i, path in enumerate(files):
        img = Image.open(path).convert("RGB").resize((thumb_w, thumb_h))
        x = (i % cols) * thumb_w
        y = (i // cols) * (thumb_h + label_h)
        sheet.paste(img, (x, y))
        draw.rectangle([x, y + thumb_h, x + thumb_w, y + thumb_h + label_h], fill=(245, 245, 242))
        draw.text((x + 8, y + thumb_h + 9), path.name[:36], fill=(20, 20, 20))
    out = OUT_DIR / "contact_sheet.jpg"
    sheet.save(out, quality=92)
    return out


def main():
    copy_v4()
    results = []
    prompts_path = OUT_DIR / "revision_prompts.json"
    prompts_path.write_text(json.dumps(REVISIONS, ensure_ascii=False, indent=2), encoding="utf-8")
    for index, info in REVISIONS.items():
        existing = sorted(FINAL_DIR.glob(f"{index:02d}_*_1080p.jpg"))[0]
        raw_existing = sorted(RAW_DIR.glob(f"{index:02d}_*.png"))[0]
        raw_path = raw_existing.with_name(raw_existing.stem + "_revised.png")
        final_path = existing.with_name(existing.stem + "_revised.jpg")
        log(f"[{index:02d}] regenerating {info['title']}")
        elapsed, item, image_bytes, response_data = call_image(index, info)
        raw_path.write_bytes(image_bytes)
        Image.open(raw_path).convert("RGB").resize(TARGET_SIZE, Image.Resampling.LANCZOS).save(final_path, quality=95)
        response_path = OUT_DIR / f"{index:02d}_revised_response.json"
        response_path.write_text(json.dumps(response_data, ensure_ascii=False, indent=2), encoding="utf-8")
        existing.unlink()
        shutil.move(str(final_path), str(existing))
        raw_existing.unlink()
        shutil.move(str(raw_path), str(raw_existing))
        results.append({"index": index, **info, "elapsedSeconds": elapsed, "serverElapsedSeconds": item.get("elapsedSeconds"), "finalPath": str(existing.resolve())})
        log(f"[{index:02d}] done elapsed={elapsed}s server={item.get('elapsedSeconds')}s final={existing.name}")
    sheet = make_sheet()
    (OUT_DIR / "manifest.json").write_text(
        json.dumps({"source": str(SRC_DIR), "revisions": results, "contactSheet": str(sheet.resolve())}, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    log(f"finished contact_sheet={sheet.resolve()}")


if __name__ == "__main__":
    main()
