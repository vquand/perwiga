from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-weapon-thumbnails.py"
SPEC = importlib.util.spec_from_file_location("fetch_weapon_thumbnails", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
thumbnails = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(thumbnails)


class WeaponThumbnailTests(unittest.TestCase):
    def record(self) -> dict[str, object]:
        return {
            "source_key": "genshin-data-weapon-12502",
            "game_data_id": 12502,
            "thumbnail_asset": "genshin-data-weapon-12502.png",
            "thumbnail_source_url": "https://upload-os-bbs.mihoyo.com/game_record/genshin/equip/UI_EquipIcon_Claymore_Wolfmound.png",
        }

    def test_snapshot_validation_accepts_first_party_png(self) -> None:
        snapshot = {
            "schema": thumbnails.SCHEMA,
            "catalog_scope": {"included_records": 1},
            "weapons": [self.record()],
        }
        self.assertEqual(thumbnails.validate_snapshot(snapshot), snapshot["weapons"])

    def test_existing_valid_asset_is_never_overwritten(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            target = output / "genshin-data-weapon-12502.png"
            original = b"\x89PNG\r\n\x1a\nexisting"
            target.write_bytes(original)

            result = thumbnails.write_validated_asset(
                self.record(), b"\x89PNG\r\n\x1a\nreplacement", output
            )

            self.assertEqual(result, "skipped")
            self.assertEqual(target.read_bytes(), original)

    def test_path_traversal_and_foreign_hosts_are_rejected(self) -> None:
        record = self.record()
        record["thumbnail_asset"] = "../weapon.png"
        with self.assertRaisesRegex(ValueError, "unsafe thumbnail asset"):
            thumbnails.validated_asset(record)

        record = self.record()
        record["thumbnail_source_url"] = "https://example.com/weapon.png"
        with self.assertRaisesRegex(ValueError, "untrusted thumbnail URL"):
            thumbnails.validated_asset(record)


if __name__ == "__main__":
    unittest.main()
