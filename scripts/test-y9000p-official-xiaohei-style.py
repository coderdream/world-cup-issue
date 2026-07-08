import json
import time
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

from PIL import Image


BASE_URL = "http://127.0.0.1:8188"
CHECKPOINT = "DreamShaper8_LCM.safetensors"
WIDTH = 768
HEIGHT = 432
STEPS = 8
CFG = 1.7
SAMPLER = "lcm"
SCHEDULER = "sgm_uniform"
DENOISE_VALUES = (0.28, 0.36, 0.44)

SOURCE = Path(r"C:\Users\ADMINI~1\AppData\Local\Temp\codex-clipboard-34b7ccc4-cc8b-493a-b144-5e01e72bd027.png")
WORK_DIR = Path(r"D:\AI\tests\official-xiaohei-style")
REFERENCE_DIR = WORK_DIR / "references"
OUTPUT_DIR = WORK_DIR / "outputs"
COMFY_INPUT_DIR = Path(r"D:\AI\apps\ComfyUI\input\official_xiaohei_style")

POSITIVE = (
    "replicate the official Xiaohei hand-drawn educational doodle style, white background, "
    "simple black blob character, thin black sketch lines, sparse red blue orange handwritten annotations, "
    "conceptual visual metaphor, lots of empty space, clean 2D line art, same composition discipline"
)
NEGATIVE = (
    "photorealistic, 3d, anime, glossy, painterly, colorful background, dense texture, watermark, logo, "
    "extra typography, messy text, complex scene, gradient, realistic face, detailed hands"
)


def request_json(path: str, payload: dict | None = None, timeout: int = 600) -> dict:
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = urllib.request.Request(f"{BASE_URL}{path}", data=data, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8", errors="replace"))


def crop_reference_panels() -> list[Path]:
    REFERENCE_DIR.mkdir(parents=True, exist_ok=True)
    COMFY_INPUT_DIR.mkdir(parents=True, exist_ok=True)
    source = Image.open(SOURCE).convert("RGB")
    rows: list[tuple[int, int]] = []
    in_panel = False
    start = 0
    sample_x = list(range(20, source.width - 20, 10))
    for y in range(source.height):
        white_pixels = 0
        for x in sample_x:
            r, g, b = source.getpixel((x, y))
            if r > 235 and g > 235 and b > 235:
                white_pixels += 1
        is_white_row = white_pixels / max(1, len(sample_x)) > 0.65
        if is_white_row and not in_panel:
            start = y
            in_panel = True
        elif not is_white_row and in_panel:
            if y - start > 250:
                rows.append((start, y))
            in_panel = False
    if in_panel and source.height - start > 250:
        rows.append((start, source.height))

    panels: list[Path] = []
    for index, (top, bottom) in enumerate(rows[:3], 1):
        panel = source.crop((20, max(0, top - 2), source.width - 20, min(source.height, bottom + 2)))
        panel.thumbnail((WIDTH, HEIGHT), Image.Resampling.LANCZOS)
        canvas = Image.new("RGB", (WIDTH, HEIGHT), (255, 255, 255))
        x = (WIDTH - panel.width) // 2
        y = (HEIGHT - panel.height) // 2
        canvas.paste(panel, (x, y))
        ref_path = REFERENCE_DIR / f"official_panel_{index:02d}.png"
        input_path = COMFY_INPUT_DIR / ref_path.name
        canvas.save(ref_path, quality=95)
        canvas.save(input_path, quality=95)
        panels.append(ref_path)
    if not panels:
        raise RuntimeError("No official white panels were detected in the source screenshot.")
    return panels


def workflow(prefix: str, ref_name: str, denoise: float, seed: int) -> dict:
    return {
        "1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": CHECKPOINT}},
        "2": {"class_type": "LoadImage", "inputs": {"image": f"official_xiaohei_style/{ref_name}"}},
        "3": {"class_type": "VAEEncode", "inputs": {"pixels": ["2", 0], "vae": ["1", 2]}},
        "4": {"class_type": "CLIPTextEncode", "inputs": {"clip": ["1", 1], "text": POSITIVE}},
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
                "denoise": denoise,
            },
        },
        "7": {"class_type": "VAEDecode", "inputs": {"samples": ["6", 0], "vae": ["1", 2]}},
        "8": {"class_type": "SaveImage", "inputs": {"images": ["7", 0], "filename_prefix": f"official_xiaohei_style/{prefix}"}},
    }


def download_image(info: dict, dest: Path) -> None:
    params = urllib.parse.urlencode(
        {"filename": info["filename"], "subfolder": info.get("subfolder", ""), "type": info.get("type", "output")}
    )
    with urllib.request.urlopen(f"{BASE_URL}/view?{params}", timeout=120) as response:
        dest.write_bytes(response.read())


def run_prompt(prompt: dict) -> dict:
    queued = request_json("/prompt", {"prompt": prompt, "client_id": str(uuid.uuid4())})
    prompt_id = queued.get("prompt_id")
    if not prompt_id:
        raise RuntimeError(f"ComfyUI did not return prompt_id: {queued}")
    started = time.time()
    while time.time() - started < 900:
        history = request_json(f"/history/{prompt_id}", timeout=120)
        if prompt_id in history:
            if history[prompt_id].get("status", {}).get("status_str") == "error":
                raise RuntimeError(f"ComfyUI prompt failed: {history[prompt_id]}")
            images = history[prompt_id].get("outputs", {}).get("8", {}).get("images", [])
            if images:
                return images[0]
        time.sleep(2)
    raise TimeoutError(f"Timed out waiting for prompt {prompt_id}")


def main() -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    panels = crop_reference_panels()
    generated = []
    for panel_index, panel in enumerate(panels, 1):
        for denoise in DENOISE_VALUES:
            prefix = f"official_panel_{panel_index:02d}_denoise_{int(denoise * 100):02d}"
            print(f"queue {prefix}", flush=True)
            info = run_prompt(workflow(prefix, panel.name, denoise, 2026070800 + panel_index * 100 + int(denoise * 100)))
            dest = OUTPUT_DIR / f"{prefix}.png"
            download_image(info, dest)
            generated.append({"reference": str(panel), "denoise": denoise, "output": str(dest)})
            print(f"generated {dest}", flush=True)
    manifest = {
        "sourceKind": "official-xiaohei-style-reference-img2img",
        "checkpoint": CHECKPOINT,
        "steps": STEPS,
        "cfg": CFG,
        "sampler": SAMPLER,
        "scheduler": SCHEDULER,
        "generatedAt": time.strftime("%Y-%m-%d %H:%M:%S"),
        "items": generated,
    }
    (WORK_DIR / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(manifest, ensure_ascii=False, indent=2), flush=True)


if __name__ == "__main__":
    main()
