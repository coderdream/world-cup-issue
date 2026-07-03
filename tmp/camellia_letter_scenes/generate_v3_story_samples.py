import base64
import json
import time
from pathlib import Path
from urllib import request as urlrequest

from PIL import Image, ImageDraw


ENDPOINT = "http://100.96.199.26:30019/v1/images/generations"
OUT_DIR = Path(r"tmp\camellia_letter_scenes\24_1080p_v3_people_first_samples")
RAW_DIR = OUT_DIR / "raw"
FINAL_DIR = OUT_DIR / "final_1080p"
LOG_PATH = OUT_DIR / "generation.log"
TARGET_SIZE = (1920, 1080)

NEGATIVE = (
    "landscape painting, empty scenery, postcard, mountain panorama, temple panorama, duplicate person, identical twins, clone, "
    "same face twice, crowd, extra woman, extra protagonist, readable text, watermark, logo, caption, comic, anime, cartoon, "
    "simple lines, low detail, blurry, bad anatomy, deformed hands, extra fingers, distorted face"
)

SAMPLES = [
    {
        "index": 1,
        "title": "shop opening close action",
        "prompt": (
            "photorealistic cinematic Japanese drama film still, 16:9. A single East Asian woman in her early 30s, shoulder length black hair, "
            "cream cardigan, opens a small letter-writing stationery shop in Kamakura at dawn. Medium shot from behind her shoulder, one hand on the key, "
            "paper envelopes visible inside the doorway, red camellia pot beside the door. Emotional, quiet, professional movie lighting. "
            "Only one person in frame, no readable text."
        ),
    },
    {
        "index": 2,
        "title": "mother conflict kitchen",
        "prompt": (
            "photorealistic cinematic Japanese drama film still, 16:9. Small warm kitchen breakfast scene. A young East Asian woman in cream cardigan "
            "stands beside the table holding an unopened letter. Her mother, clearly older late 50s with short grey hair and green cardigan, sits across the table. "
            "Rice bowls, tea, morning light, tense but tender family mood. Exactly two people, visibly different ages and faces, no readable text."
        ),
    },
    {
        "index": 3,
        "title": "daughter doorway distance",
        "prompt": (
            "photorealistic cinematic Japanese drama film still, 16:9. A teenage East Asian daughter in school uniform and ponytail stands in a half-open sliding doorway, "
            "looking away with guarded emotion. The adult protagonist in cream cardigan is seated deeper inside the room, slightly out of focus. "
            "The doorway creates emotional distance. Exactly two people, clearly adult and teenager, no readable text."
        ),
    },
    {
        "index": 4,
        "title": "old letter discovery",
        "prompt": (
            "photorealistic cinematic Japanese drama film still, 16:9. Close medium shot in a tatami storage room. The single protagonist kneels on the floor, "
            "opening an old wooden box full of tied paper letters. Her face shows surprise and sadness. One red camellia-pattern cloth, dust in warm side light. "
            "Only one person in frame, no readable text."
        ),
    },
]


def log(message):
    line = f"{time.strftime('%Y-%m-%d %H:%M:%S')} {message}"
    print(line, flush=True)
    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
    with LOG_PATH.open("a", encoding="utf-8") as f:
        f.write(line + "\n")


def call_image(sample):
    payload = {
        "prompt": sample["prompt"],
        "negative_prompt": NEGATIVE,
        "size": "1024x576",
        "n": 1,
        "steps": 14,
        "guidance_scale": 3.2,
        "seed": 530000 + sample["index"] * 101,
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


def upscale(raw_path, final_path):
    im = Image.open(raw_path).convert("RGB")
    im.resize(TARGET_SIZE, Image.Resampling.LANCZOS).save(final_path, quality=95)


def make_contact_sheet(results):
    thumb_w, thumb_h = 640, 360
    label_h = 34
    sheet = Image.new("RGB", (2 * thumb_w, 2 * (thumb_h + label_h)), "white")
    draw = ImageDraw.Draw(sheet)
    for i, result in enumerate(results):
        img = Image.open(result["finalPath"]).convert("RGB").resize((thumb_w, thumb_h))
        x = (i % 2) * thumb_w
        y = (i // 2) * (thumb_h + label_h)
        sheet.paste(img, (x, y))
        draw.rectangle([x, y + thumb_h, x + thumb_w, y + thumb_h + label_h], fill=(245, 245, 242))
        draw.text((x + 8, y + thumb_h + 9), f"{result['index']:02d} {result['title']} {result['elapsedSeconds']}s", fill=(20, 20, 20))
    path = OUT_DIR / "contact_sheet.jpg"
    sheet.save(path, quality=92)
    return path


def main():
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    FINAL_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "prompts.json").write_text(json.dumps(SAMPLES, ensure_ascii=False, indent=2), encoding="utf-8")
    results = []
    for sample in SAMPLES:
        idx = sample["index"]
        raw_path = RAW_DIR / f"{idx:02d}.png"
        final_path = FINAL_DIR / f"{idx:02d}_1080p.jpg"
        response_path = OUT_DIR / f"{idx:02d}_response.json"
        log(f"[{idx}/4] generating {sample['title']}")
        elapsed, item, image_bytes, response_data = call_image(sample)
        raw_path.write_bytes(image_bytes)
        response_path.write_text(json.dumps(response_data, ensure_ascii=False, indent=2), encoding="utf-8")
        upscale(raw_path, final_path)
        results.append({**sample, "elapsedSeconds": elapsed, "serverElapsedSeconds": item.get("elapsedSeconds"), "finalPath": str(final_path.resolve())})
        log(f"[{idx}/4] done elapsed={elapsed}s server={item.get('elapsedSeconds')}s final={final_path}")
    contact_sheet = make_contact_sheet(results)
    (OUT_DIR / "manifest.json").write_text(
        json.dumps({"results": results, "contactSheet": str(contact_sheet.resolve())}, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    log(f"finished contact_sheet={contact_sheet.resolve()}")


if __name__ == "__main__":
    main()
