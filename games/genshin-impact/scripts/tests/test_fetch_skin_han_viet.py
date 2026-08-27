from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-skin-han-viet.py"
SPEC = importlib.util.spec_from_file_location("fetch_skin_han_viet", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
han_viet = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(han_viet)


class SkinHanVietTests(unittest.TestCase):
    def catalog(self) -> dict[str, object]:
        return {
            "schema": han_viet.CATALOG_SCHEMA,
            "catalog_scope": {"included_records": 1},
            "skins": [
                {
                    "source_key": "hoyowiki-outfit-5777",
                    "official_english_name": "Red Dead of Night",
                    "official_simplified_chinese_name": "殷红终夜",
                }
            ],
        }

    def test_generated_alias_remains_separate_and_confidence_e(self) -> None:
        response = {
            "message": "OK",
            "mode": "trans",
            "lang": "1",
            "result": [
                {"i": "殷", "t": 3, "o": ["ân"]},
                {"i": "红", "t": 3, "o": ["hồng"]},
                {"i": "终", "t": 3, "o": ["chung"]},
                {"i": "夜", "t": 3, "o": ["dạ"]},
            ],
        }

        snapshot = han_viet.build_snapshot(
            self.catalog(), response, checked_at="2026-08-26"
        )

        self.assertEqual(snapshot["confidence"]["han_viet_names"], "E")
        self.assertEqual(snapshot["skins"][0]["han_viet_name"], "Ân Hồng Chung Dạ")

    def test_catalog_requires_complete_unique_source_keys(self) -> None:
        catalog = self.catalog()
        catalog["skins"].append(dict(catalog["skins"][0]))
        catalog["catalog_scope"]["included_records"] = 2
        with self.assertRaisesRegex(ValueError, "duplicate"):
            han_viet.validate_catalog(catalog)


if __name__ == "__main__":
    unittest.main()
