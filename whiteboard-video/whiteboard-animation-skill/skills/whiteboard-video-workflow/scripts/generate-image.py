#!/usr/bin/env python3

import asyncio
import base64
import json
import os
import random
import re
import struct
import sys

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8")
import time
import zlib
from http.client import IncompleteRead
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import ProxyHandler, Request, build_opener, urlopen

sys.path.insert(0, str(Path(__file__).resolve().parent))
from prompt_template import whiteboard_prompt_template

MAX_RETRIES = 3
RETRY_BASE_DELAY_S = 3.0
CHAT_FALLBACK_SYSTEM = """You create simple whiteboard storyboard art as JSON.
Return JSON only, with this shape:
{"title":"short title","elements":[{"type":"rect|circle|line|label","x":0.1,"y":0.1,"w":0.3,"h":0.2,"x2":0.8,"y2":0.8,"text":"short label"}]}
Use normalized coordinates from 0 to 1. Use 8 to 18 elements. Prefer simple iconic shapes, scene layout, labels, arrows, and motion cues. Do not include markdown."""

SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
        "AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/126.0.0.0 Safari/537.36"
    ),
    "Accept": "application/json, text/plain, */*",
    "Accept-Encoding": "identity",
    "Connection": "close",
}


class RetryableError(Exception):
    def __init__(self, message, *, is_rate_limit=False):
        super().__init__(message)
        self.is_rate_limit = is_rate_limit


class FatalError(Exception):
    pass


def load_env():
    env_path = SCRIPT_DIR.parent / ".env"
    if not env_path.exists():
        return
    for line in env_path.read_text(encoding="utf-8").splitlines():
        trimmed = line.strip()
        if not trimmed or trimmed.startswith("#") or "=" not in trimmed:
            continue
        key, value = trimmed.split("=", 1)
        key = key.strip()
        value = value.strip().strip('"').strip("'")
        if not value:
            continue
        os.environ.setdefault(key, value)
        normalized = key.lower()
        if normalized == "url":
            os.environ.setdefault("OPENAI_API_BASE", value)
        elif normalized == "model":
            os.environ.setdefault("OPENAI_IMAGE_MODEL", value)
        elif normalized == "key":
            os.environ.setdefault("OPENAI_API_KEY", value)
        elif key in (
            "OPENAI_API_BASE",
            "OPENAI_BASE_URL",
            "OPENAI_IMAGE_MODEL",
            "OPENAI_MODEL",
            "OPENAI_API_KEY",
            "CODEX_API_KEY",
            "OPENAI_IMAGE_CONCURRENCY",
            "OPENAI_IMAGE_MODE",
            "HTTPS_PROXY",
            "HTTP_PROXY",
            "ALL_PROXY",
            "proxy",
        ):
            os.environ.setdefault(key, value)


def api_base():
    return (
        os.environ.get("OPENAI_API_BASE")
        or os.environ.get("OPENAI_BASE_URL")
        or os.environ.get("url")
        or ""
    ).rstrip("/")


def image_model():
    return os.environ.get("OPENAI_IMAGE_MODEL") or os.environ.get("OPENAI_MODEL") or os.environ.get("model") or ""


def image_concurrency():
    return int(os.environ.get("OPENAI_IMAGE_CONCURRENCY", "3"))


def image_mode():
    return os.environ.get("OPENAI_IMAGE_MODE", "auto").strip().lower()


def macmini_endpoint():
    return (
        os.environ.get("MACMINI_IMAGE_ENDPOINT")
        or os.environ.get("BOOK_IMAGE_ENDPOINT")
        or "http://192.168.1.9:30020/v1/images/generations"
    ).rstrip("/")


def macmini_negative_prompt():
    if os.environ.get("BOOK_IMAGE_PROMPT_STYLE", "").strip().lower() == "book-illustration":
        return os.environ.get(
            "MACMINI_NEGATIVE_PROMPT",
            (
                "readable text, writing, open book pages, signs, numbers, labels, posters, subtitles, watermark, logo, "
                "photorealistic, real photo, 3d render, human portrait, anime face, crowd, duplicate character, "
                "complex landscape, postcard panorama, blurry, low detail, bad anatomy"
            ),
        )
    return os.environ.get(
        "MACMINI_NEGATIVE_PROMPT",
        (
            "no person, empty room, empty landscape, close-up portrait, headshot, studio portrait, passport photo, "
            "plain wall background, face filling frame, looking directly at camera, glamour shot, two people, three people, "
            "second person, another person, companion, sitting together, crowd, duplicate person, identical twins, clone, "
            "same face twice, extra woman, extra protagonist, kimono, yukata, red robe, ancient costume, readable text, "
            "letters with readable text, signboard text, storefront sign, shop sign, poster text, label text, numbers, digits, "
            "watermark, logo, caption, subtitles, random characters, garbled characters, anime, cartoon, illustration, painting, low detail, blurry, "
            "bad anatomy, deformed hands, extra fingers, distorted face, plastic skin"
        ),
    )


def macmini_steps():
    return int(os.environ.get("MACMINI_IMAGE_STEPS", "30"))


def macmini_guidance():
    return float(os.environ.get("MACMINI_IMAGE_GUIDANCE", "7.8"))


def macmini_seed_base():
    return int(os.environ.get("MACMINI_IMAGE_SEED_BASE", "960000"))


def api_key():
    return os.environ.get("OPENAI_API_KEY") or os.environ.get("CODEX_API_KEY") or os.environ.get("key")


def proxy_url():
    return os.environ.get("proxy") or os.environ.get("HTTPS_PROXY") or os.environ.get("HTTP_PROXY") or os.environ.get("ALL_PROXY")


def open_url(req, timeout):
    configured_proxy = proxy_url()
    if configured_proxy:
        proxy = configured_proxy
        if "://" not in proxy:
            proxy = f"http://{proxy}"
        opener = build_opener(ProxyHandler({"http": proxy, "https": proxy}))
        return opener.open(req, timeout=timeout)
    return urlopen(req, timeout=timeout)


def calc_backoff(attempt, *, is_rate_limit=False):
    multiplier = 2.0 if is_rate_limit else 1.0
    delay = RETRY_BASE_DELAY_S * (2 ** (attempt - 1)) * multiplier
    return delay * random.uniform(0.5, 1.5)


def image_size_for_aspect_ratio(aspect_ratio):
    normalized = aspect_ratio.strip()
    if normalized == "1:1":
        return "1024x1024"
    if normalized == "9:16":
        return "1024x1536"
    if image_mode() == "macmini-realistic":
        return "768x432"
    return "1536x1024"


def request_image_sync(prompt, aspect_ratio):
    key = api_key()
    if not key:
        raise FatalError(
            "API key not found. Set key=..., OPENAI_API_KEY=..., or CODEX_API_KEY=... in the skill .env file."
        )
    base = api_base()
    if not base:
        raise FatalError("API base URL not found. Set url=... or OPENAI_API_BASE=... in the skill .env file.")
    model = image_model()
    if not model:
        raise FatalError("Image model not found. Set model=... or OPENAI_IMAGE_MODEL=... in the skill .env file.")

    body = {
        "model": model,
        "prompt": prompt,
        "size": image_size_for_aspect_ratio(aspect_ratio),
        "n": 1,
    }
    payload = json.dumps(body).encode("utf-8")
    req = Request(f"{base}/images/generations", data=payload, method="POST")
    for header, value in DEFAULT_HEADERS.items():
        req.add_header(header, value)
    req.add_header("Content-Type", "application/json")
    req.add_header("Authorization", f"Bearer {key}")

    try:
        with open_url(req, timeout=180) as resp:
            return json.loads(read_response_text(resp))
    except HTTPError as e:
        body_text = e.read().decode("utf-8", errors="replace")
        if e.code in (400, 401, 403, 404):
            raise FatalError(f"HTTP {e.code}: {body_text}")
        if e.code == 429:
            raise RetryableError(f"HTTP 429 (rate limited): {body_text}", is_rate_limit=True)
        raise RetryableError(f"HTTP {e.code}: {body_text}")
    except json.JSONDecodeError as e:
        raise RetryableError(f"Failed to parse OpenAI response: {e}")
    except FatalError:
        raise
    except Exception as e:
        raise RetryableError(str(e))


def request_macmini_image_sync(prompt, aspect_ratio, index):
    model = image_model()
    body = {
        "prompt": prompt,
        "negative_prompt": macmini_negative_prompt(),
        "size": image_size_for_aspect_ratio(aspect_ratio),
        "n": 1,
        "steps": macmini_steps(),
        "guidance_scale": macmini_guidance(),
        "seed": macmini_seed_base() + (index + 1) * 71,
    }
    if model:
        body["model"] = model
    payload = json.dumps(body, ensure_ascii=False).encode("utf-8")
    req = Request(macmini_endpoint(), data=payload, method="POST")
    for header, value in DEFAULT_HEADERS.items():
        req.add_header(header, value)
    req.add_header("Content-Type", "application/json")

    try:
        with open_url(req, timeout=600) as resp:
            return json.loads(read_response_text(resp))
    except HTTPError as e:
        body_text = e.read().decode("utf-8", errors="replace")
        if e.code in (400, 401, 403, 404):
            raise FatalError(f"HTTP {e.code}: {body_text}")
        if e.code == 429:
            raise RetryableError(f"HTTP 429 (rate limited): {body_text}", is_rate_limit=True)
        raise RetryableError(f"HTTP {e.code}: {body_text}")
    except json.JSONDecodeError as e:
        raise RetryableError(f"Failed to parse MacMini image response: {e}")
    except FatalError:
        raise
    except Exception as e:
        raise RetryableError(str(e))


async def with_retry(fn, context=""):
    for attempt in range(1, MAX_RETRIES + 1):
        try:
            return await fn()
        except FatalError:
            raise
        except RetryableError as e:
            if attempt == MAX_RETRIES:
                raise
            delay = calc_backoff(attempt, is_rate_limit=e.is_rate_limit)
            print(f"{context}Attempt {attempt}/{MAX_RETRIES} failed: {e}. Retrying in {delay:.1f}s...")
            await asyncio.sleep(delay)


def save_image_response(res, output_dir, index, total):
    data = res.get("data") or []
    if not data:
        raise RetryableError(f"No image data in response: {json.dumps(res)[:500]}")

    item = data[0]
    timestamp = int(time.time() * 1000)
    suffix = f"_{str(index + 1).zfill(len(str(total)))}" if total > 1 else ""
    filepath = Path(output_dir) / f"img_{timestamp}{suffix}.png"

    if item.get("b64_json"):
        filepath.write_bytes(base64.b64decode(item["b64_json"]))
        return str(filepath)

    if item.get("url"):
        with urlopen(item["url"], timeout=120) as resp:
            filepath.write_bytes(resp.read())
        return str(filepath)

    raise RetryableError(f"Image response has neither b64_json nor url: {json.dumps(item)[:500]}")


def request_chat_scene_sync(prompt):
    key = api_key()
    if not key:
        raise FatalError(
            "API key not found. Set key=..., OPENAI_API_KEY=..., or CODEX_API_KEY=... in the skill .env file."
        )
    base = api_base()
    if not base:
        raise FatalError("API base URL not found. Set url=... or OPENAI_API_BASE=... in the skill .env file.")
    model = image_model()
    if not model:
        raise FatalError("Model not found. Set model=... or OPENAI_IMAGE_MODEL=... in the skill .env file.")

    body = {
        "model": model,
        "messages": [
            {"role": "system", "content": CHAT_FALLBACK_SYSTEM},
            {"role": "user", "content": prompt},
        ],
        "temperature": 0.2,
    }
    payload = json.dumps(body).encode("utf-8")
    req = Request(f"{base}/chat/completions", data=payload, method="POST")
    for header, value in DEFAULT_HEADERS.items():
        req.add_header(header, value)
    req.add_header("Content-Type", "application/json")
    req.add_header("Authorization", f"Bearer {key}")

    try:
        with open_url(req, timeout=180) as resp:
            return json.loads(read_response_text(resp))
    except HTTPError as e:
        body_text = e.read().decode("utf-8", errors="replace")
        if e.code in (400, 401, 403, 404):
            raise FatalError(f"HTTP {e.code}: {body_text}")
        if e.code == 429:
            raise RetryableError(f"HTTP 429 (rate limited): {body_text}", is_rate_limit=True)
        raise RetryableError(f"HTTP {e.code}: {body_text}")
    except json.JSONDecodeError as e:
        raise RetryableError(f"Failed to parse chat response: {e}")
    except FatalError:
        raise
    except Exception as e:
        raise RetryableError(str(e))


def read_response_text(resp):
    try:
        data = resp.read()
    except IncompleteRead as e:
        data = e.partial
        if not data:
            raise
    return data.decode("utf-8", errors="replace")


def extract_chat_content(res):
    choices = res.get("choices") or []
    if not choices:
        raise RetryableError(f"No choices in chat response: {json.dumps(res)[:500]}")
    message = choices[0].get("message") or {}
    content = message.get("content")
    if isinstance(content, list):
        parts = []
        for item in content:
            if isinstance(item, dict) and item.get("text"):
                parts.append(item["text"])
            elif isinstance(item, str):
                parts.append(item)
        content = "\n".join(parts)
    if not isinstance(content, str) or not content.strip():
        raise RetryableError(f"No text content in chat response: {json.dumps(res)[:500]}")
    return content.strip()


def parse_scene_json(text):
    cleaned = re.sub(r"^```(?:json)?\s*|\s*```$", "", text.strip(), flags=re.IGNORECASE | re.MULTILINE)
    try:
        return json.loads(cleaned)
    except json.JSONDecodeError:
        match = re.search(r"\{.*\}", cleaned, flags=re.DOTALL)
        if not match:
            raise RetryableError(f"Chat fallback did not return JSON: {text[:500]}")
        return json.loads(match.group(0))


def png_chunk(chunk_type, data):
    chunk = chunk_type + data
    return struct.pack(">I", len(data)) + chunk + struct.pack(">I", zlib.crc32(chunk) & 0xFFFFFFFF)


def write_png(path, width, height, pixels):
    raw = bytearray()
    stride = width * 3
    for y in range(height):
        raw.append(0)
        start = y * stride
        raw.extend(pixels[start:start + stride])
    data = b"\x89PNG\r\n\x1a\n"
    data += png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
    data += png_chunk(b"IDAT", zlib.compress(bytes(raw), 6))
    data += png_chunk(b"IEND", b"")
    Path(path).write_bytes(data)


def set_pixel(pixels, width, height, x, y, color):
    if 0 <= x < width and 0 <= y < height:
        idx = (y * width + x) * 3
        pixels[idx:idx + 3] = bytes(color)


def draw_line(pixels, width, height, x1, y1, x2, y2, color, thickness=3):
    dx = abs(x2 - x1)
    dy = -abs(y2 - y1)
    sx = 1 if x1 < x2 else -1
    sy = 1 if y1 < y2 else -1
    err = dx + dy
    x, y = x1, y1
    radius = max(1, thickness // 2)
    while True:
        for oy in range(-radius, radius + 1):
            for ox in range(-radius, radius + 1):
                set_pixel(pixels, width, height, x + ox, y + oy, color)
        if x == x2 and y == y2:
            break
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x += sx
        if e2 <= dx:
            err += dx
            y += sy


def draw_rect(pixels, width, height, x, y, w, h, color):
    x2 = x + w
    y2 = y + h
    draw_line(pixels, width, height, x, y, x2, y, color)
    draw_line(pixels, width, height, x2, y, x2, y2, color)
    draw_line(pixels, width, height, x2, y2, x, y2, color)
    draw_line(pixels, width, height, x, y2, x, y, color)


def draw_circle(pixels, width, height, cx, cy, r, color):
    steps = max(24, int(r * 0.8))
    prev = None
    for i in range(steps + 1):
        angle = 2 * 3.141592653589793 * i / steps
        x = int(cx + r * __import__("math").cos(angle))
        y = int(cy + r * __import__("math").sin(angle))
        if prev:
            draw_line(pixels, width, height, prev[0], prev[1], x, y, color, 2)
        prev = (x, y)


def draw_label_placeholder(pixels, width, height, x, y, text, color):
    length = max(3, min(16, len(text or "label")))
    line_w = min(width - x - 8, 14 * length)
    for i in range(3):
        yy = y + i * 9
        draw_line(pixels, width, height, x, yy, x + max(20, line_w - i * 16), yy, color, 2)


def clamp01(value, default):
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return min(1.0, max(0.0, number))


def render_scene_png(scene, output_dir, index, total, aspect_ratio):
    size = image_size_for_aspect_ratio(aspect_ratio)
    width, height = [int(part) for part in size.split("x")]
    bg = (248, 246, 239)
    ink = (28, 35, 38)
    accent = (38, 98, 160)
    pixels = bytearray(bg * (width * height))

    margin = int(width * 0.05)
    draw_rect(pixels, width, height, margin, margin, width - margin * 2, height - margin * 2, (210, 205, 190))

    elements = scene.get("elements") if isinstance(scene, dict) else []
    if not isinstance(elements, list):
        elements = []
    if not elements:
        elements = [
            {"type": "rect", "x": 0.18, "y": 0.25, "w": 0.28, "h": 0.2},
            {"type": "circle", "x": 0.68, "y": 0.34, "w": 0.18, "h": 0.18},
            {"type": "line", "x": 0.46, "y": 0.35, "x2": 0.62, "y2": 0.35},
            {"type": "label", "x": 0.22, "y": 0.55, "text": scene.get("title", "scene") if isinstance(scene, dict) else "scene"},
        ]

    for i, item in enumerate(elements[:24]):
        if not isinstance(item, dict):
            continue
        typ = str(item.get("type", "rect")).lower()
        x = int(clamp01(item.get("x"), 0.1) * width)
        y = int(clamp01(item.get("y"), 0.1) * height)
        w = int(max(0.03, clamp01(item.get("w"), 0.18)) * width)
        h = int(max(0.03, clamp01(item.get("h"), 0.12)) * height)
        color = accent if i % 5 == 0 else ink
        if typ == "line":
            x2 = int(clamp01(item.get("x2"), clamp01(item.get("x"), 0.1) + 0.2) * width)
            y2 = int(clamp01(item.get("y2"), clamp01(item.get("y"), 0.1) + 0.1) * height)
            draw_line(pixels, width, height, x, y, x2, y2, color, 4)
        elif typ == "circle":
            draw_circle(pixels, width, height, x, y, max(12, min(w, h) // 2), color)
        elif typ == "label":
            draw_label_placeholder(pixels, width, height, x, y, str(item.get("text", "")), color)
        else:
            draw_rect(pixels, width, height, x, y, w, h, color)

    timestamp = int(time.time() * 1000)
    suffix = f"_{str(index + 1).zfill(len(str(total)))}" if total > 1 else ""
    filepath = Path(output_dir) / f"img_{timestamp}{suffix}.png"
    write_png(filepath, width, height, pixels)
    return str(filepath)


def generate_chat_fallback_sync(prompt, aspect_ratio, output_dir, index, total):
    res = request_chat_scene_sync(prompt)
    content = extract_chat_content(res)
    scene = parse_scene_json(content)
    return render_scene_png(scene, output_dir, index, total, aspect_ratio)


def generate_local_fallback_sync(prompt, aspect_ratio, output_dir, index, total):
    seed = sum(ord(ch) for ch in prompt)
    random.seed(seed)
    elements = []
    for i in range(10):
        typ = ["rect", "circle", "line", "label"][i % 4]
        x = 0.12 + (random.random() * 0.72)
        y = 0.16 + (random.random() * 0.62)
        item = {
            "type": typ,
            "x": x,
            "y": y,
            "w": 0.08 + random.random() * 0.18,
            "h": 0.06 + random.random() * 0.16,
        }
        if typ == "line":
            item["x2"] = min(0.92, x + 0.12 + random.random() * 0.2)
            item["y2"] = min(0.86, y + (random.random() - 0.5) * 0.18)
        if typ == "label":
            item["text"] = prompt[:24]
        elements.append(item)
    scene = {"title": prompt[:40], "elements": elements}
    return render_scene_png(scene, output_dir, index, total, aspect_ratio)


async def generate_single(prompt, aspect_ratio, output_dir, index, total):
    tag = f"[{index + 1}/{total}] " if total > 1 else ""
    full_prompt = whiteboard_prompt_template + prompt

    async def _do():
        mode = image_mode()
        target = macmini_endpoint() if mode == "macmini-realistic" else api_base()
        print(f"{tag}Generating image with {image_model()} via {target} ({mode})...")
        if mode == "chat-json":
            path = await asyncio.to_thread(generate_chat_fallback_sync, prompt, aspect_ratio, output_dir, index, total)
        elif mode == "macmini-realistic":
            res = await asyncio.to_thread(request_macmini_image_sync, prompt, aspect_ratio, index)
            path = await asyncio.to_thread(save_image_response, res, output_dir, index, total)
        else:
            try:
                res = await asyncio.to_thread(request_image_sync, full_prompt, aspect_ratio)
                path = await asyncio.to_thread(save_image_response, res, output_dir, index, total)
            except FatalError as e:
                if mode != "auto":
                    raise
                print(f"{tag}Images endpoint failed; gateway compatibility looks incomplete, falling back to chat JSON renderer: {e}")
                try:
                    path = await asyncio.to_thread(generate_chat_fallback_sync, prompt, aspect_ratio, output_dir, index, total)
                except Exception as chat_error:
                    print(f"{tag}Chat renderer also failed; using local deterministic whiteboard renderer: {chat_error}")
                    path = await asyncio.to_thread(generate_local_fallback_sync, prompt, aspect_ratio, output_dir, index, total)
        print(f"{tag}Image saved: {path}")
        return path

    return await with_retry(_do, context=tag)


async def run_batch(tasks, concurrency):
    semaphore = asyncio.Semaphore(concurrency)
    results = [None] * len(tasks)

    async def worker(i, task):
        async with semaphore:
            try:
                results[i] = await generate_single(
                    task["prompt"],
                    task["aspectRatio"],
                    task["outputDir"],
                    task["index"],
                    task["total"],
                )
            except Exception as e:
                results[i] = {"error": str(e)}

    await asyncio.gather(*(worker(i, t) for i, t in enumerate(tasks)))
    return results


async def main():
    load_env()
    args = sys.argv[1:]
    prompt_arg = args[0] if len(args) > 0 else ""
    aspect_ratio = args[1] if len(args) > 1 else "16:9"
    output_dir = args[2] if len(args) > 2 else os.getcwd()

    if not prompt_arg.strip():
        print("Error: prompt is required and cannot be empty.")
        sys.exit(1)

    Path(output_dir).mkdir(parents=True, exist_ok=True)

    prompts = None
    try:
        parsed = json.loads(prompt_arg)
        if isinstance(parsed, list) and parsed and isinstance(parsed[0], str):
            prompts = parsed
    except (json.JSONDecodeError, ValueError):
        pass
    if not prompts:
        prompts = [prompt_arg]

    total = len(prompts)
    if total > 1:
        print(f"Batch mode: generating {total} images (concurrency: {image_concurrency()})...")

    tasks = [
        {
            "prompt": prompt,
            "aspectRatio": aspect_ratio,
            "outputDir": output_dir,
            "index": i,
            "total": total,
        }
        for i, prompt in enumerate(prompts)
    ]

    results = await run_batch(tasks, image_concurrency())

    succeeded = [r for r in results if isinstance(r, str)]
    failed = [r for r in results if isinstance(r, dict) and r.get("error")]
    if total > 1:
        print(f"\nBatch complete: {len(succeeded)} succeeded, {len(failed)} failed.")
    if failed:
        for f in failed:
            print(f"  Error: {f['error']}")

    print(f"\n__RESULTS__{json.dumps(results, ensure_ascii=False)}")
    sys.exit(0 if not failed else 1)


if __name__ == "__main__":
    asyncio.run(main())
