#!/usr/bin/env python3
import argparse
import json
import subprocess
import tempfile
from pathlib import Path

from PIL import Image


def main() -> None:
    parser = argparse.ArgumentParser(description="Render a still-image timeline MP4 from image_timeline.json.")
    parser.add_argument("--timeline", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--fps", type=int, default=30)
    parser.add_argument("--crf", type=int, default=20)
    parser.add_argument("--preset", default="veryfast")
    args = parser.parse_args()

    timeline = json.loads(args.timeline.read_text(encoding="utf-8"))
    root = args.timeline.parent
    if not timeline:
        raise SystemExit("timeline is empty")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="book_timeline_") as tmp:
        tmp_dir = Path(tmp)
        concat_file = tmp_dir / "concat.txt"
        lines: list[str] = []
        for item in timeline:
            src = root / item["image"]
            if not src.exists():
                raise FileNotFoundError(src)
            # Keep ffmpeg paths ASCII and short to avoid quoting issues with concat files.
            frame = tmp_dir / f"scene_{item['index']:03d}.png"
            with Image.open(src) as im:
                im.convert("RGB").save(frame)
            duration_s = item["duration_ms"] / 1000
            lines.append(f"file '{frame.as_posix()}'")
            lines.append(f"duration {duration_s:.6f}")
        last_frame = tmp_dir / f"scene_{timeline[-1]['index']:03d}.png"
        lines.append(f"file '{last_frame.as_posix()}'")
        concat_file.write_text("\n".join(lines) + "\n", encoding="utf-8")

        cmd = [
            "ffmpeg",
            "-y",
            "-hide_banner",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            str(concat_file),
            "-vf",
            f"fps={args.fps},format=yuv420p",
            "-c:v",
            "libx264",
            "-preset",
            args.preset,
            "-crf",
            str(args.crf),
            "-movflags",
            "+faststart",
            str(args.output),
        ]
        subprocess.run(cmd, check=True)

    print(args.output.resolve())


if __name__ == "__main__":
    main()
