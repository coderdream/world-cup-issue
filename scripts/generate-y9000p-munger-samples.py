import json
import os
import shutil
import time
import urllib.parse
import urllib.request
import uuid
from pathlib import Path


BASE_URL = "http://127.0.0.1:8188"
CHECKPOINT = os.environ.get("Y9000P_SAMPLE_CHECKPOINT", "v1-5-pruned-emaonly.safetensors")
WIDTH = int(os.environ.get("Y9000P_SAMPLE_WIDTH", "768"))
HEIGHT = int(os.environ.get("Y9000P_SAMPLE_HEIGHT", "432"))
STEPS = int(os.environ.get("Y9000P_SAMPLE_STEPS", "16"))
CFG = float(os.environ.get("Y9000P_SAMPLE_CFG", "7.0"))
SAMPLER = os.environ.get("Y9000P_SAMPLE_SAMPLER", "euler")
SCHEDULER = os.environ.get("Y9000P_SAMPLE_SCHEDULER", "normal")
OUTPUT_ROOT = Path(r"D:\AI\outputs\ComfyUI")
SAMPLE_DIR = Path(os.environ.get("Y9000P_SAMPLE_DIR", r"D:\books\0701新书四本\芒格传\output\04_images\_samples"))
FILE_PREFIX = os.environ.get("Y9000P_SAMPLE_FILE_PREFIX", "munger_sample")

POSITIVE_BASE = (
    os.environ.get(
        "Y9000P_SAMPLE_POSITIVE_PREFIX",
        "clean white background, black ink line art, chinese editorial illustration, "
        "funny visual metaphor, expressive little black character, investment thinking, "
        "rational decision, books, checklist, magnifying glass, tiny casino chip, "
        "large blank lower area for Chinese subtitles, simple composition, no readable text",
    )
)
NEGATIVE = (
    os.environ.get(
        "Y9000P_SAMPLE_NEGATIVE_PROMPT",
        "photorealistic, 3d, oil painting, dense background, messy text, watermark, logo, "
        "bad hands, extra fingers, horror, dark scene, cluttered layout, readable words",
    )
)

SAMPLES = [
    (
        f"{FILE_PREFIX}_01",
        "Charlie Munger refuses to be dragged by emotion, calmly studies the facts, then places one careful bet. "
        "A small black ink character holds a magnifying glass over papers and a single casino chip.",
    ),
    (
        f"{FILE_PREFIX}_02",
        "Charlie Munger says a good idea makes him jump like a little trout, a calm person suddenly childlike. "
        "A small black ink character springs beside a glowing idea card, books, and a tiny fish-like motion metaphor.",
    ),
]


def request_json(path: str, payload: dict | None = None, timeout: int = 600) -> dict:
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    request = urllib.request.Request(
        f"{BASE_URL}{path}",
        data=data,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8", errors="replace"))


def workflow(prefix: str, prompt: str, seed: int) -> dict:
    return {
        "1": {
            "class_type": "CheckpointLoaderSimple",
            "inputs": {"ckpt_name": CHECKPOINT},
        },
        "2": {
            "class_type": "CLIPTextEncode",
            "inputs": {"clip": ["1", 1], "text": f"{POSITIVE_BASE}. {prompt}"},
        },
        "3": {
            "class_type": "CLIPTextEncode",
            "inputs": {"clip": ["1", 1], "text": NEGATIVE},
        },
        "4": {
            "class_type": "EmptyLatentImage",
            "inputs": {"width": WIDTH, "height": HEIGHT, "batch_size": 1},
        },
        "5": {
            "class_type": "KSampler",
            "inputs": {
                "model": ["1", 0],
                "positive": ["2", 0],
                "negative": ["3", 0],
                "latent_image": ["4", 0],
                "seed": seed,
                "steps": STEPS,
                "cfg": CFG,
                "sampler_name": SAMPLER,
                "scheduler": SCHEDULER,
                "denoise": 1.0,
            },
        },
        "6": {
            "class_type": "VAEDecode",
            "inputs": {"samples": ["5", 0], "vae": ["1", 2]},
        },
        "7": {
            "class_type": "SaveImage",
            "inputs": {"images": ["6", 0], "filename_prefix": f"munger_samples/{prefix}"},
        },
    }


def download_image(info: dict, dest: Path) -> None:
    params = urllib.parse.urlencode(
        {
            "filename": info["filename"],
            "subfolder": info.get("subfolder", ""),
            "type": info.get("type", "output"),
        }
    )
    with urllib.request.urlopen(f"{BASE_URL}/view?{params}", timeout=120) as response:
        dest.write_bytes(response.read())


def main() -> None:
    SAMPLE_DIR.mkdir(parents=True, exist_ok=True)
    generated = []
    for index, (prefix, prompt) in enumerate(SAMPLES, 1):
        queued = request_json(
            "/prompt",
            {"prompt": workflow(prefix, prompt, 2026070800 + index), "client_id": str(uuid.uuid4())},
        )
        prompt_id = queued.get("prompt_id")
        if not prompt_id:
            raise RuntimeError(f"ComfyUI did not return prompt_id: {queued}")
        print(f"queued {index}/{len(SAMPLES)} prompt_id={prompt_id}", flush=True)
        image_info = None
        started_at = time.time()
        while time.time() - started_at < 900:
            history = request_json(f"/history/{prompt_id}", timeout=120)
            if prompt_id in history:
                images = history[prompt_id].get("outputs", {}).get("7", {}).get("images", [])
                if not images:
                    raise RuntimeError(f"ComfyUI finished without image output: {history[prompt_id]}")
                image_info = images[0]
                break
            time.sleep(3)
        if image_info is None:
            raise TimeoutError(f"Timed out waiting for prompt {prompt_id}")
        local_copy = SAMPLE_DIR / f"{prefix}.png"
        download_image(image_info, local_copy)
        generated.append(str(local_copy))
        print(f"generated {local_copy}", flush=True)

    manifest = {
        "sourceKind": "y9000p-comfyui-smoke",
        "baseUrl": BASE_URL,
        "checkpoint": CHECKPOINT,
        "width": WIDTH,
        "height": HEIGHT,
        "steps": STEPS,
        "cfg": CFG,
        "sampler": SAMPLER,
        "scheduler": SCHEDULER,
        "generatedAt": time.strftime("%Y-%m-%d %H:%M:%S"),
        "images": generated,
    }
    (SAMPLE_DIR / f"{FILE_PREFIX}_manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    source_dir = OUTPUT_ROOT / "munger_samples"
    if source_dir.exists():
        shutil.copytree(source_dir, SAMPLE_DIR / "_comfyui_raw", dirs_exist_ok=True)


if __name__ == "__main__":
    main()
