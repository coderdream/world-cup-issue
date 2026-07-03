#!/usr/bin/env python3
"""
Whiteboard Video Workflow Helper

Provides three commands:
  1. init-dirs     - Create storyboard/image/video output directories
  2. gen-prompts   - Parse storyboard.json and generate image prompts with whiteboard style prefix
  3. merge-videos  - Merge video segments into one final video using PyAV

Usage:
    python workflow_helper.py init-dirs <output-dir>
    python workflow_helper.py gen-prompts <storyboard-json-path>
    python workflow_helper.py merge-videos <output-dir> <video1> <video2> ...
"""

import json
import os
import sys

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8")
import unicodedata
from datetime import datetime
from pathlib import Path


def ends_with_symbol(text: str) -> bool:
    """Return True when the stripped text already ends with punctuation or a symbol."""
    stripped = text.rstrip()
    if not stripped:
        return False
    return unicodedata.category(stripped[-1])[0] in {"P", "S"}


def ensure_ending(text: str, ending: str) -> str:
    """Append ending only when the stripped text does not already end with a symbol."""
    stripped = text.strip()
    if not stripped:
        return ""
    if ends_with_symbol(stripped):
        return stripped
    return f"{stripped}{ending}"


def join_scene_text(text_parts: list[str]) -> str:
    """Join scene segment texts while preserving existing ending symbols."""
    parts = [text.strip() for text in text_parts if text.strip()]
    if not parts:
        return ""

    pieces = [ensure_ending(text, "，") for text in parts[:-1]]
    pieces.append(ensure_ending(parts[-1], "。"))
    return "".join(pieces)


def init_dirs(output_dir: str):
    """Create storyboard, image, video subdirectories under output_dir."""
    base = Path(output_dir).resolve()
    for name in ("storyboard", "image", "video"):
        (base / name).mkdir(parents=True, exist_ok=True)
    print(json.dumps({
        "status": "ok",
        "storyboardDir": str(base / "storyboard"),
        "imageDir": str(base / "image"),
        "videoDir": str(base / "video"),
    }))


def gen_prompts(storyboard_path: str):
    """Parse storyboard.json and output a JSON array of image prompts."""
    sb = json.loads(Path(storyboard_path).read_text(encoding="utf-8"))
    prompts = []
    style = os.environ.get("BOOK_IMAGE_PROMPT_STYLE", "").strip().lower()
    scenes = sb.get("scenes", [])
    for index, scene in enumerate(scenes):
        if style == "book-realistic":
            visual_hint = scene.get("visualHint", "").strip()
            scene_text = join_scene_text([segment.get("text", "") for segment in scene.get("segments", [])])[:160]
            prompts.append(build_book_realistic_prompt(index, len(scenes), visual_hint, scene_text))
            continue
        if style == "book-illustration":
            visual_hint = scene.get("visualHint", "").strip()
            scene_text = join_scene_text([segment.get("text", "") for segment in scene.get("segments", [])])[:220]
            prompts.append(build_book_illustration_prompt(index, len(scenes), visual_hint, scene_text))
            continue
        if style == "book-illustration-character":
            visual_hint = scene.get("visualHint", "").strip()
            scene_text = join_scene_text([segment.get("text", "") for segment in scene.get("segments", [])])[:220]
            prompts.append(build_book_character_illustration_prompt(index, len(scenes), visual_hint, scene_text))
            continue
        if style == "book-xiaohei":
            prompts.append(build_book_xiaohei_prompt(index, len(scenes)))
            continue
        visual_hint = ensure_ending(scene.get("visualHint", ""), "。")
        if visual_hint:
            content = f'视觉元素建议：\n"{visual_hint}"'
            prompts.append(content)
        else:
            prompts.append("")
    print(json.dumps(prompts, ensure_ascii=False))


def build_book_realistic_prompt(index: int, total: int, visual_hint: str, scene_text: str) -> str:
    """Build a realistic book-video image prompt for MacMini4 Realistic Vision."""
    beat = visual_hint or scene_text or "quiet emotional reading scene with paper letters"
    return (
        "photorealistic cinematic Japanese drama film still, medium-wide environmental shot, "
        "professional 30-minute book-summary video visual, warm natural light, quiet emotional realism, "
        "realistic lived-in details, 35mm lens, 16:9, no readable text. "
        f"Scene {index + 1:02d}/{total}. Primary scene action: {beat}. "
        "Same woman visible in every scene: East Asian woman early 30s, short black bob haircut with soft bangs, "
        "cream knit cardigan, white top, dark skirt. "
        "The woman must be clearly visible as a full-body or half-body figure, occupying about 20 to 35 percent of the frame. "
        "She must be the only visible person in the entire image, with no companion, no customer, no family member, no reflection person. "
        "She must be interacting with the environment, not posing for camera, not a close-up portrait. "
        "Use blank paper and blank envelopes only. No signs, no labels, no posters, no numbers, no visible writing anywhere. "
        "Keep the composition cinematic and story-driven."
    )


def build_book_illustration_prompt(index: int, total: int, visual_hint: str, scene_text: str) -> str:
    """Build a consistent illustrated book-video prompt focused on story meaning."""
    anchor = illustration_anchor(index)
    return (
        "16:9 soft watercolor literary illustration, ivory paper, muted teal shadows, camellia red accent, graphite outlines. "
        f"Scene {index + 1:02d}/{total}, main image: {anchor}. "
        "No people. One clear still-life action, foreground object storytelling, quiet Japanese coastal town mood. "
        "Looks like premium book-summary video B-roll, calm and emotionally specific. "
        "No empty room, no character, no face, no hands, no readable text, no signs, no numbers, no labels."
    )


def build_book_character_illustration_prompt(index: int, total: int, visual_hint: str, scene_text: str) -> str:
    """Build a character-led illustration prompt with restrained composition."""
    anchor = character_illustration_anchor(index)
    return (
        "16:9 soft watercolor literary illustration, ivory paper, muted teal shadows, camellia red accent, graphite outlines. "
        "One tiny recurring faceless adult woman only, modern Kamakura mother and stationery-shop letter writer, late 30s to early 40s, short natural black bob hair, oatmeal cardigan, linen apron, dark long skirt, back view or distant side view. "
        f"Scene {index + 1:02d}/{total}, main image: {anchor}. "
        "She occupies 4 to 8 percent of the frame, environmental wide shot, never close-up, never front-facing portrait, no visible facial details. "
        "Contemporary everyday life, phone and printer era but handwritten letters remain central, quiet Japanese coastal town mood, premium book-summary video still. "
        "No second person, no duplicate woman, no crowd, no schoolgirl, no teenager, no backpack, no kimono, no fantasy cloak, no readable text, no signs, no numbers, no labels."
    )


def build_book_xiaohei_prompt(index: int, total: int) -> str:
    """Build a white-background editorial doodle prompt like the reference skill videos."""
    anchor = book_xiaohei_anchor(index)
    return (
        "clean white background editorial doodle illustration, hand-drawn black ink lines, simple flat shapes, lots of negative space. "
        "Fixed IP character: faceless round-headed adult woman letter writer, black bob hair, small oatmeal apron, dark skirt, calm modern Kamakura mother. "
        f"Scene {index + 1:02d}/{total}: {anchor}. "
        "One complete illustration where character, objects, arrows, and symbols share the same hand-drawn style. "
        "Use only black, warm gray, camellia red accent, and muted paper beige. "
        "No realistic room, no watercolor scenery, no anime face, no text, no labels, no logo, no poster."
    )


def compact_visual_hint(text: str) -> str:
    """Keep prompt content short enough for SD1.5 CLIP while preserving the scene action."""
    cleaned = " ".join((text or "").replace("\n", " ").split())
    if not cleaned:
        return "the messenger opens an old box of blank letters beside one red camellia"
    replacements = {
        "the woman": "the messenger",
        "A young East Asian woman": "the messenger",
        "young East Asian woman": "the messenger",
        "Kamakura": "quiet town",
        "stationery shop": "letter shop",
        "fountain pen": "pen",
    }
    for src, dst in replacements.items():
        cleaned = cleaned.replace(src, dst)
    return cleaned[:170]


def illustration_anchor(index: int) -> str:
    """Return a short scene-specific object/action anchor for local SD1.5 models."""
    anchors = [
        "red camellia pot and brass key in foreground, quiet letter shop door behind it, shelves of sealed blank envelopes",
        "sealed blank envelope centered on a wooden writing desk, pen, ink bottle, tea steam, camellia vase, window light",
        "two untouched rice bowls and sealed envelope on a kitchen table, school bag on one chair, morning silence",
        "two empty cushions across a low tatami table, school shoes near a half-open sliding door, red camellia petal",
        "old wooden letter box centered on tatami, tied bundles of sealed blank envelopes, camellia-pattern cloth, dust light",
        "wrapped wooden letter box beside a ferry rail, grey-blue sea wind, distant island silhouette, no passengers",
        "blank paper centered under a brass lamp, rain-streaked window, tea cup, scattered red camellia petals",
        "sealed envelope and red camellias on a seaside wooden rail, sunrise over calm open sea, no people",
    ]
    if not anchors:
        return "sealed blank envelope beside one red camellia"
    return anchors[index % len(anchors)]


def character_illustration_anchor(index: int) -> str:
    """Return a short person-plus-object anchor for local SD1.5 character smoke tests."""
    anchors = [
        "tiny back-view woman at the quiet letter shop door, brass key and red camellia pot large in foreground, shelves of sealed blank envelopes inside",
        "tiny side-view woman at far end of a wooden writing desk, sealed blank envelope centered in foreground, pen, tea steam, camellia vase",
        "tiny back-view woman in a small kitchen doorway, two untouched rice bowls and sealed envelope large on the table, school bag on one chair",
        "tiny side-view woman beside a low tatami table, half-open sliding door, school shoes near doorway, one empty cushion across from her",
        "tiny back-view woman beside an old wooden letter box on tatami, tied blank envelopes, camellia-pattern cloth, warm dust light",
        "tiny back-view woman alone by a ferry rail, wrapped wooden letter box large in foreground, grey-blue sea wind, distant island silhouette",
        "tiny side-view woman at far side of a night desk, blank paper centered under brass lamp, rain window, tea cup, red camellia petals",
        "tiny back-view woman at a seaside wooden rail, sealed envelope and red camellias large in foreground, sunrise over calm sea",
    ]
    return anchors[index % len(anchors)]


def book_xiaohei_anchor(index: int) -> str:
    """Return a compact xiaohei-style book-summary illustration anchor."""
    anchors = [
        "she opens a tiny letter shop; oversized brass key, red camellia pot, and blank envelopes float around her",
        "she writes at a desk; sealed envelope, fountain pen, tea cup, and camellia form a simple composition",
        "family pressure metaphor; two rice bowls, school bag, small house outline, and a blank letter between them",
        "distance at home; half-open sliding door, two cushions, school shoes, and a small lonely character",
        "old letters discovered; wooden box, tied blank envelopes, camellia cloth, dust sparkles, character kneeling beside it",
        "journey and farewell; ferry rail, wrapped letter box, sea wave icon, distant island, character looking outward",
        "blocked writing night; brass lamp, blank paper, rain window, tea cup, camellia petals around the character",
        "quiet repair; seaside rail, sealed envelope, red camellias, sunrise arc, character facing the open sea",
    ]
    return anchors[index % len(anchors)]


def merge_videos(output_dir: str, video_paths: list[str]):
    """Merge multiple video segments into one final video using PyAV (re-encode via H.264)."""
    if not video_paths:
        print(json.dumps({"status": "error", "error": "没有视频片段可合并"}))
        sys.exit(1)

    # 检查所有视频文件是否存在
    for vp in video_paths:
        if not Path(vp).exists():
            print(json.dumps({"status": "error", "error": f"视频文件不存在: {vp}"}))
            sys.exit(1)

    # 生成输出文件名
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    output_path = Path(output_dir).resolve() / f"whiteboard_{timestamp}.mp4"

    import av
    from fractions import Fraction

    try:
        # 从第一个片段获取编码参数
        first_input = av.open(video_paths[0], mode="r")
        in_stream = first_input.streams.video[0]
        width = in_stream.codec_context.width
        height = in_stream.codec_context.height
        fps = in_stream.average_rate
        first_input.close()

        # 创建输出容器
        time_base = Fraction(1, int(fps))

        output_container = av.open(str(output_path), mode="w")
        out_stream = output_container.add_stream("h264", rate=fps)
        out_stream.width = width
        out_stream.height = height
        out_stream.pix_fmt = "yuv420p"
        out_stream.time_base = time_base
        out_stream.options = {"crf": "18"}

        # 逐个读取输入片段，解码后重新编码写入输出
        # 使用帧计数器生成单调递增的 PTS，避免多段拼接时时间戳倒退
        frame_count = 0
        for vp in video_paths:
            input_container = av.open(vp, mode="r")
            for frame in input_container.decode(video=0):
                frame.pts = frame_count
                frame.time_base = time_base
                frame_count += 1
                packet = out_stream.encode(frame)
                if packet:
                    for p in packet:
                        output_container.mux(p)
            input_container.close()

        # flush
        packet = out_stream.encode(None)
        if packet:
            for p in packet:
                output_container.mux(p)
        output_container.close()
    except Exception as e:
        # 清理可能残留的输出文件
        if output_path.exists():
            output_path.unlink()
        print(json.dumps({"status": "error", "error": f"视频合并失败: {e}"}))
        sys.exit(1)

    output_size_mb = output_path.stat().st_size / (1024 * 1024)
    print(json.dumps({
        "status": "ok",
        "mergedVideo": str(output_path),
        "totalSegments": len(video_paths),
        "sizeMB": round(output_size_mb, 1),
    }, ensure_ascii=False))


def main():
    if len(sys.argv) < 3:
        print("Usage:")
        print("  workflow_helper.py init-dirs <output-dir>")
        print("  workflow_helper.py gen-prompts <storyboard-json-path>")
        print("  workflow_helper.py merge-videos <output-dir> <video1> <video2> ...")
        sys.exit(1)

    cmd = sys.argv[1]
    if cmd == "init-dirs":
        init_dirs(sys.argv[2])
    elif cmd == "gen-prompts":
        gen_prompts(sys.argv[2])
    elif cmd == "merge-videos":
        if len(sys.argv) < 4:
            print("Error: merge-videos requires output-dir and at least one video path")
            sys.exit(1)
        merge_videos(sys.argv[2], sys.argv[3:])
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)


if __name__ == "__main__":
    main()
