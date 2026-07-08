import json
import time
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

from PIL import Image, ImageDraw


BASE_URL = "http://127.0.0.1:8188"
CHECKPOINT = "DreamShaper8_LCM.safetensors"
WIDTH = 768
HEIGHT = 432
STEPS = 8
CFG = 1.7
SAMPLER = "lcm"
SCHEDULER = "sgm_uniform"
DENOISE = 0.42
COMFY_INPUT_DIR = Path(r"D:\AI\apps\ComfyUI\input\munger_guides")
SAMPLE_DIR = Path(r"D:\books\0701新书四本\芒格传\output\04_images\_samples")

POSITIVE = (
    "flat 2D editorial doodle, black ink line art, white background, simple little black character, "
    "minimal visual metaphor for a Chinese book summary video, clean thick outline, lots of empty lower space, "
    "no text, no letters, no logo"
)
NEGATIVE = (
    "photorealistic, realistic photo, 3d render, glossy, glass, gradients, detailed documents, readable text, "
    "watermark, logo, red stamp, complex background, anime, chibi, human face"
)


def sketch_line(draw: ImageDraw.ImageDraw, points: list[tuple[int, int]], width: int = 7) -> None:
    draw.line(points, fill=(20, 20, 20), width=width, joint="curve")


def draw_character(draw: ImageDraw.ImageDraw, x: int, y: int, scale: float = 1.0) -> None:
    r = int(38 * scale)
    draw.ellipse((x - r, y - r, x + r, y + r), fill=(18, 18, 18))
    eye = int(6 * scale)
    draw.ellipse((x - 13 * scale - eye, y - 8 * scale - eye, x - 13 * scale + eye, y - 8 * scale + eye), fill=(245, 245, 245))
    draw.ellipse((x + 13 * scale - eye, y - 8 * scale - eye, x + 13 * scale + eye, y - 8 * scale + eye), fill=(245, 245, 245))
    draw.arc((x - 13 * scale, y + 4 * scale, x + 14 * scale, y + 24 * scale), 5, 175, fill=(245, 245, 245), width=max(2, int(3 * scale)))
    sketch_line(draw, [(x - 22, y + r), (x - 40, y + r + 55)], max(3, int(4 * scale)))
    sketch_line(draw, [(x + 22, y + r), (x + 45, y + r + 50)], max(3, int(4 * scale)))
    sketch_line(draw, [(x - 12, y + r + 35), (x - 30, y + r + 82)], max(3, int(4 * scale)))
    sketch_line(draw, [(x + 12, y + r + 35), (x + 30, y + r + 82)], max(3, int(4 * scale)))


def guide_one(path: Path) -> None:
    image = Image.new("RGB", (WIDTH, HEIGHT), (250, 250, 246))
    draw = ImageDraw.Draw(image)
    draw.ellipse((96, 82, 438, 424), outline=(18, 18, 18), width=18)
    sketch_line(draw, [(360, 342), (514, 396)], 18)
    draw.rectangle((505, 338, 635, 401), outline=(18, 18, 18), width=7)
    draw.line((520, 360, 620, 360), fill=(18, 18, 18), width=4)
    draw.line((520, 382, 600, 382), fill=(18, 18, 18), width=4)
    draw_character(draw, 160, 270, 0.85)
    draw.ellipse((650, 250, 704, 304), outline=(18, 18, 18), width=7)
    sketch_line(draw, [(676, 304), (676, 360)], 6)
    image.save(path)


def guide_two(path: Path) -> None:
    image = Image.new("RGB", (WIDTH, HEIGHT), (250, 250, 246))
    draw = ImageDraw.Draw(image)
    draw_character(draw, 210, 238, 1.05)
    draw.rectangle((384, 112, 574, 240), outline=(18, 18, 18), width=8)
    sketch_line(draw, [(410, 150), (450, 116), (500, 150), (542, 116)], 7)
    for offset in [0, 56, 112]:
        sketch_line(draw, [(260 + offset, 122), (288 + offset, 88), (316 + offset, 122)], 6)
    draw.arc((590, 154, 710, 294), 105, 255, fill=(18, 18, 18), width=9)
    draw.ellipse((638, 254, 688, 304), outline=(18, 18, 18), width=7)
    sketch_line(draw, [(210, 315), (285, 360), (350, 336)], 7)
    image.save(path)


def request_json(path: str, payload: dict | None = None, timeout: int = 600) -> dict:
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = urllib.request.Request(f"{BASE_URL}{path}", data=data, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8", errors="replace"))


def workflow(prefix: str, guide_name: str, prompt: str, seed: int) -> dict:
    return {
        "1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": CHECKPOINT}},
        "2": {"class_type": "LoadImage", "inputs": {"image": f"munger_guides/{guide_name}"}},
        "3": {"class_type": "VAEEncode", "inputs": {"pixels": ["2", 0], "vae": ["1", 2]}},
        "4": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["1", 1], "text": f"{POSITIVE}. {prompt}"}},
        "5": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["1", 1], "text": NEGATIVE}},
        "6": {
            "class_type": "KSampler",
            "inputs": {
                "model": ["1", 0],
                "positive": ["4", 0],
                "negative": ["5", 0],
                "latent_image": ["3", 0],
                "seed": seed,
                "steps": STEPS,
                "cfg": CFG,
                "sampler_name": SAMPLER,
                "scheduler": SCHEDULER,
                "denoise": DENOISE,
            },
        },
        "7": {"class_type": "VAEDecode", "inputs": {"samples": ["6", 0], "vae": ["1", 2]}},
        "8": {"class_type": "SaveImage", "inputs": {"images": ["7", 0], "filename_prefix": f"munger_controlled/{prefix}"}},
    }


def download_image(info: dict, dest: Path) -> None:
    params = urllib.parse.urlencode(
        {"filename": info["filename"], "subfolder": info.get("subfolder", ""), "type": info.get("type", "output")}
    )
    with urllib.request.urlopen(f"{BASE_URL}/view?{params}", timeout=120) as response:
        dest.write_bytes(response.read())


def main() -> None:
    COMFY_INPUT_DIR.mkdir(parents=True, exist_ok=True)
    SAMPLE_DIR.mkdir(parents=True, exist_ok=True)
    guides = [
        ("munger_control_01", "munger_control_01.png", guide_one, "A rational investor studies facts with a magnifying glass and makes one careful decision."),
        ("munger_control_02", "munger_control_02.png", guide_two, "A good idea makes a calm thinker jump with childlike energy beside books and a bright idea card."),
    ]
    generated = []
    for index, (prefix, guide_name, guide_fn, prompt) in enumerate(guides, 1):
        guide_path = COMFY_INPUT_DIR / guide_name
        guide_fn(guide_path)
        queued = request_json(
            "/prompt",
            {"prompt": workflow(prefix, guide_name, prompt, 2026070810 + index), "client_id": str(uuid.uuid4())},
        )
        prompt_id = queued.get("prompt_id")
        if not prompt_id:
            raise RuntimeError(f"ComfyUI did not return prompt_id: {queued}")
        print(f"queued {index}/{len(guides)} prompt_id={prompt_id}", flush=True)
        image_info = None
        started_at = time.time()
        while time.time() - started_at < 900:
            history = request_json(f"/history/{prompt_id}", timeout=120)
            if prompt_id in history:
                if history[prompt_id].get("status", {}).get("status_str") == "error":
                    raise RuntimeError(f"ComfyUI prompt failed: {history[prompt_id]}")
                images = history[prompt_id].get("outputs", {}).get("8", {}).get("images", [])
                if images:
                    image_info = images[0]
                    break
            time.sleep(2)
        if image_info is None:
            raise TimeoutError(f"Timed out waiting for prompt {prompt_id}")
        local_copy = SAMPLE_DIR / f"{prefix}.png"
        download_image(image_info, local_copy)
        generated.append(str(local_copy))
        print(f"generated {local_copy}", flush=True)
    manifest = {
        "sourceKind": "y9000p-comfyui-img2img-controlled-smoke",
        "checkpoint": CHECKPOINT,
        "steps": STEPS,
        "cfg": CFG,
        "sampler": SAMPLER,
        "scheduler": SCHEDULER,
        "denoise": DENOISE,
        "generatedAt": time.strftime("%Y-%m-%d %H:%M:%S"),
        "images": generated,
    }
    (SAMPLE_DIR / "munger_controlled_manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
