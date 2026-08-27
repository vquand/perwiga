from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-skin-thumbnails.py"
SPEC = importlib.util.spec_from_file_location("fetch_skin_thumbnails", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
thumbnails = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(thumbnails)


class SkinThumbnailTests(unittest.TestCase):
    def record(self, extension: str = "png") -> dict[str, object]:
        return {
            "source_key": "hoyowiki-outfit-5774",
            "hoyowiki_entry_page_id": "5774",
            "thumbnail_asset": f"hoyowiki-outfit-5774.{extension}",
            "thumbnail_source_url": (
                "https://act-webstatic.hoyoverse.com/event-static-hoyowiki-admin/"
                f"outfit.{extension}"
            ),
        }

    def test_accepts_png_and_gif(self) -> None:
        thumbnails.validated_asset(self.record("png"))
        thumbnails.validated_asset(self.record("gif"))

    def test_existing_valid_asset_is_never_overwritten(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            target = output / "hoyowiki-outfit-5774.png"
            original = b"\x89PNG\r\n\x1a\nexisting"
            target.write_bytes(original)
            result = thumbnails.write_validated_asset(
                self.record(), b"\x89PNG\r\n\x1a\nreplacement", output
            )
            self.assertEqual(result, "skipped")
            self.assertEqual(target.read_bytes(), original)

    def test_rejects_path_traversal_and_foreign_hosts(self) -> None:
        record = self.record()
        record["thumbnail_asset"] = "../outfit.png"
        with self.assertRaisesRegex(ValueError, "unsafe thumbnail asset"):
            thumbnails.validated_asset(record)
        record = self.record()
        record["thumbnail_source_url"] = "https://example.com/outfit.png"
        with self.assertRaisesRegex(ValueError, "untrusted thumbnail URL"):
            thumbnails.validated_asset(record)


if __name__ == "__main__":
    unittest.main()
