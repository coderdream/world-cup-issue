import base64
import io
import os
import time
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

import torch
from diffusers import DiffusionPipeline, LCMScheduler
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field


DEFAULT_MODEL = os.environ.get("BOOK_IMAGE_MODEL", "Lykon/dreamshaper-8-lcm")
DEFAULT_WIDTH = int(os.environ.get("BOOK_IMAGE_WIDTH", "768"))
DEFAULT_HEIGHT = int(os.environ.get("BOOK_IMAGE_HEIGHT", "432"))
DEFAULT_STEPS = int(os.environ.get("BOOK_IMAGE_STEPS", "8"))
DEFAULT_GUIDANCE = float(os.environ.get("BOOK_IMAGE_GUIDANCE", "1.8"))
DEFAULT_DTYPE = os.environ.get("BOOK_IMAGE_DTYPE", "auto").lower()
OUTPUT_DIR = Path(os.environ.get("BOOK_IMAGE_OUTPUT_DIR", "./outputs")).resolve()

app = FastAPI(title="Book Image Service", version="0.1.0")
pipe: Optional[DiffusionPipeline] = None
loaded_model = ""


class ImageGenerationRequest(BaseModel):
    model: Optional[str] = None
    prompt: str
    negative_prompt: Optional[str] = Field(
        default=(
            "text, letters, watermark, logo, caption, UI, flowchart, diagram, "
            "simple geometric lines, empty background, low detail, blurry"
        )
    )
    size: Optional[str] = None
    n: int = 1
    steps: Optional[int] = None
    guidance_scale: Optional[float] = None
    seed: Optional[int] = None
    response_format: str = "b64_json"


def parse_size(size: Optional[str]) -> Tuple[int, int]:
    if not size:
        return DEFAULT_WIDTH, DEFAULT_HEIGHT
    try:
        width, height = [int(part) for part in size.lower().split("x", 1)]
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Invalid size: {size}") from exc
    width = max(256, min(1024, width))
    height = max(256, min(1024, height))
    width = (width // 8) * 8
    height = (height // 8) * 8
    return width, height


def device_name() -> str:
    if torch.backends.mps.is_available():
        return "mps"
    if torch.cuda.is_available():
        return "cuda"
    return "cpu"


def dtype_for_device(device: str) -> torch.dtype:
    if DEFAULT_DTYPE in {"float32", "fp32"}:
        return torch.float32
    if DEFAULT_DTYPE in {"float16", "fp16", "half"}:
        return torch.float16
    # MPS float16 can produce NaNs on some Stable Diffusion pipelines.
    if device == "mps":
        return torch.float32
    if device == "cuda":
        return torch.float16
    return torch.float32


def load_pipeline(model_id: str) -> DiffusionPipeline:
    global pipe, loaded_model
    if pipe is not None and loaded_model == model_id:
        return pipe

    device = device_name()
    dtype = dtype_for_device(device)
    local_pipe = DiffusionPipeline.from_pretrained(
        model_id,
        torch_dtype=dtype,
        safety_checker=None,
        requires_safety_checker=False,
    )
    if hasattr(local_pipe, "safety_checker"):
        local_pipe.safety_checker = None
    if hasattr(local_pipe, "requires_safety_checker"):
        local_pipe.requires_safety_checker = False
    if "lcm" in model_id.lower():
        local_pipe.scheduler = LCMScheduler.from_config(local_pipe.scheduler.config)
    local_pipe = local_pipe.to(device)
    try:
        local_pipe.enable_attention_slicing()
    except Exception:
        pass
    pipe = local_pipe
    loaded_model = model_id
    return local_pipe


def quality_metrics(image) -> Dict[str, Any]:
    small = image.convert("RGB").resize((256, 144))
    colors = small.getcolors(maxcolors=65536) or []
    non_bg = 0
    for count, (r, g, b) in colors:
        if not (r > 225 and g > 220 and b > 210):
            non_bg += count
    ratio = non_bg / max(1, small.width * small.height)
    return {"colors": len(colors), "nonBgRatio": round(ratio, 3)}


@app.get("/health")
def health() -> Dict[str, Any]:
    return {
        "ok": True,
        "device": device_name(),
        "model": loaded_model or DEFAULT_MODEL,
        "dtype": str(dtype_for_device(device_name())).replace("torch.", ""),
        "torch": torch.__version__,
    }


@app.post("/v1/images/generations")
def generate_image(request: ImageGenerationRequest) -> Dict[str, Any]:
    if not request.prompt.strip():
        raise HTTPException(status_code=400, detail="prompt is required")
    if request.n != 1:
        raise HTTPException(status_code=400, detail="Only n=1 is supported on the local Mac service")

    model_id = request.model or DEFAULT_MODEL
    width, height = parse_size(request.size)
    generator = None
    if request.seed is not None:
        generator = torch.Generator(device=device_name()).manual_seed(request.seed)

    pipeline = load_pipeline(model_id)
    start = time.time()
    with torch.inference_mode():
        image = pipeline(
            prompt=request.prompt,
            negative_prompt=request.negative_prompt,
            width=width,
            height=height,
            num_inference_steps=request.steps or DEFAULT_STEPS,
            guidance_scale=request.guidance_scale or DEFAULT_GUIDANCE,
            generator=generator,
        ).images[0]

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    filename = f"book_image_{int(time.time())}.png"
    output_path = OUTPUT_DIR / filename
    image.save(output_path)
    metrics = quality_metrics(image)

    buffer = io.BytesIO()
    image.save(buffer, format="PNG")
    encoded = base64.b64encode(buffer.getvalue()).decode("ascii")
    return {
        "created": int(time.time()),
        "data": [
            {
                "b64_json": encoded,
                "url": f"/outputs/{filename}",
                "path": str(output_path),
                "revised_prompt": request.prompt,
                "metrics": metrics,
                "elapsedSeconds": round(time.time() - start, 2),
                "model": model_id,
                "device": device_name(),
            }
        ],
    }
