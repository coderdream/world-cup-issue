import importlib.util
import json
import shutil
from pathlib import Path


REPO = Path(r"D:\04_GitHub\world-cup-issue")
PIPELINE = REPO / "a-book-in-30-minutes" / "tmp" / "book_video_pipeline.py"
EPUB = Path(r"D:\books\0701新书四本\芒格传\芒格传.epub")
MATERIAL_ROOT = Path(r"D:\books\0701新书四本\芒格传\output")
SMOKE_DIR = Path(r"D:\AI\tests\official-xiaohei-style\munger-book-smoke")


def load_pipeline():
    spec = importlib.util.spec_from_file_location("book_video_pipeline", PIPELINE)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def main() -> None:
    pipeline = load_pipeline()
    timed_srt = pipeline.find_timed_chinese_srt(MATERIAL_ROOT, MATERIAL_ROOT, "cmn")
    if not timed_srt:
        raise RuntimeError(f"No aligned Chinese SRT found in {MATERIAL_ROOT}")
    events = pipeline.read_srt_events(timed_srt)
    if not events:
        raise RuntimeError(f"No SRT events found: {timed_srt}")

    if SMOKE_DIR.exists():
        shutil.rmtree(SMOKE_DIR)
    SMOKE_DIR.mkdir(parents=True, exist_ok=True)

    material = pipeline.read_material_json(MATERIAL_ROOT)
    title = str(material.get("videoTitle") or pipeline.read_text(MATERIAL_ROOT / "title.txt", EPUB.stem)).strip()
    description = str(material.get("description") or pipeline.read_text(MATERIAL_ROOT / "description.txt", "")).strip()
    assets, source_dir, source_kind = pipeline.generate_y9000p_comfyui_assets(
        SMOKE_DIR,
        MATERIAL_ROOT,
        title,
        description,
        events,
    )
    result = {
        "title": title,
        "timedSrt": str(timed_srt),
        "smokeDir": str(SMOKE_DIR),
        "sourceDir": str(source_dir),
        "sourceKind": source_kind,
        "assetCount": len(assets),
        "assets": [str(path) for path in assets],
    }
    (SMOKE_DIR / "smoke_result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
