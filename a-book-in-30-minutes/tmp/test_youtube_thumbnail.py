import importlib.util
import tempfile
import unittest
from pathlib import Path

from PIL import Image


PIPELINE = Path(__file__).with_name("book_video_pipeline.py")
SPEC = importlib.util.spec_from_file_location("book_video_pipeline", PIPELINE)
pipeline = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(pipeline)


class YoutubeThumbnailTests(unittest.TestCase):
    def test_hemingway_title_uses_short_hook_lines(self):
        lines = pipeline.youtube_thumbnail_lines("《亲爱的老爸：海明威父子家书》")
        self.assertEqual(lines, ["海明威写给", "儿子的信"])

    def test_thumbnail_renders_1280x720_with_detail(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "thumb.jpg"
            pipeline.render_youtube_thumbnail(
                output,
                "睡前听书｜《亲爱的老爸：海明威父子家书》：在信里，父亲和儿子慢慢靠近",
                "在信里，父亲和儿子慢慢靠近",
                "海明威父子家书",
            )
            self.assertTrue(output.is_file())
            with Image.open(output) as image:
                self.assertEqual(image.size, (1280, 720))
                preview = image.resize((320, 180), Image.Resampling.LANCZOS).convert("RGB")
                colors = preview.getcolors(maxcolors=65536) or []
                self.assertGreater(len(colors), 500)
                bright_pixels = sum(
                    count for count, (r, g, b) in colors
                    if r > 220 and g > 210 and b > 190
                )
                self.assertGreater(bright_pixels / (320 * 180), 0.03)


if __name__ == "__main__":
    unittest.main()
