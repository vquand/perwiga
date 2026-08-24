from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT_PATH = Path(__file__).parents[1] / "fetch-character-thumbnails.py"
SPEC = importlib.util.spec_from_file_location("fetch_character_thumbnails", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load {SCRIPT_PATH}")
fetch_thumbnails = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fetch_thumbnails)


PNG = b"\x89PNG\r\n\x1a\n" + b"test-image"


def character(*, asset: str = "hoyoverse-content-163984.png") -> dict:
    return {
        "source_key": "hoyoverse-content-163984",
        "official_content_id": 163984,
        "official_english_name": "Lohen",
        "thumbnail_source_url": (
            "https://fastcdn.hoyoverse.com/content-v2/hk4e/163984/icon.png"
        ),
        "thumbnail_asset": asset,
    }


class CharacterThumbnailTests(unittest.TestCase):
    def test_valid_first_party_png_is_written_without_overwriting(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)

            first = fetch_thumbnails.write_validated_asset(character(), PNG, output)
            second = fetch_thumbnails.write_validated_asset(
                character(), PNG + b"replacement", output
            )

            self.assertEqual(first, "downloaded")
            self.assertEqual(second, "skipped")
            self.assertEqual(
                (output / "hoyoverse-content-163984.png").read_bytes(), PNG
            )

    def test_invalid_image_signature_is_rejected_before_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)

            with self.assertRaisesRegex(ValueError, "not a valid PNG"):
                fetch_thumbnails.write_validated_asset(
                    character(), b"<html>not an image</html>", output
                )

            self.assertEqual(list(output.iterdir()), [])

    def test_snapshot_cannot_escape_the_asset_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)

            with self.assertRaisesRegex(ValueError, "unsafe thumbnail asset"):
                fetch_thumbnails.write_validated_asset(
                    character(asset="../outside.png"), PNG, output
                )

            self.assertFalse((output.parent / "outside.png").exists())

    def test_snapshot_schema_and_count_are_validated(self) -> None:
        valid = {
            "schema": "perwiga.genshin-impact.characters.v1",
            "catalog_scope": {"included_records": 1},
            "characters": [character()],
        }
        self.assertEqual(fetch_thumbnails.validate_snapshot(valid), valid["characters"])

        invalid = dict(valid)
        invalid["catalog_scope"] = {"included_records": 2}
        with self.assertRaisesRegex(ValueError, "record count"):
            fetch_thumbnails.validate_snapshot(invalid)


if __name__ == "__main__":
    unittest.main()
