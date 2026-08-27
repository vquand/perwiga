from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SCRIPT_PATH = Path(__file__).parents[1] / "fetch-region-han-viet.py"
SPEC = importlib.util.spec_from_file_location("fetch_region_han_viet", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load {SCRIPT_PATH}")
fetch_han_viet = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fetch_han_viet)


def result(character: str, *alternatives: str) -> dict:
    return {"t": 3, "i": character, "o": list(alternatives)}


class RegionHanVietSnapshotTests(unittest.TestCase):
    def test_dictionary_batches_preserve_order_and_bound_request_size(self) -> None:
        names = [f"名称{index}" for index in range(85)]

        batches = fetch_han_viet.dictionary_batches(names, batch_size=40)

        self.assertEqual([len(batch) for batch in batches], [40, 40, 5])
        self.assertEqual([name for batch in batches for name in batch], names)

    def test_build_snapshot_keeps_generated_reading_separate(self) -> None:
        catalog = {
            "schema": "perwiga.genshin-impact.regions.v1",
            "catalog_scope": {"included_records": 1},
            "regions": [
                {
                    "source_key": "genshin-data-area-1-1201",
                    "official_english_name": "Windrise",
                    "official_simplified_chinese_name": "风起地",
                }
            ],
        }
        response = {
            "message": "OK",
            "mode": "trans",
            "lang": "1",
            "result": [
                result("风", "phong"),
                result("起", "khởi"),
                result("地", "địa"),
            ],
        }

        snapshot = fetch_han_viet.build_snapshot(
            catalog, response, checked_at="2026-08-26"
        )

        self.assertEqual(snapshot["schema"], "perwiga.genshin-impact.region-han-viet.v1")
        self.assertEqual(snapshot["confidence"]["han_viet_names"], "E")
        self.assertEqual(
            snapshot["regions"][0]["han_viet_name"],
            "Phong Khởi Địa",
        )
        self.assertNotIn("official_vietnamese_name", snapshot["regions"][0])

    def test_unattested_decorative_punctuation_is_preserved(self) -> None:
        tokens = [
            {"t": 3, "i": "「", "o": []},
            result("清", "thanh"),
            result("籁", "lại"),
            result("丸", "hoàn"),
            {"t": 3, "i": "」", "o": []},
        ]

        self.assertEqual(
            fetch_han_viet.format_region_han_viet("「清籁丸」", tokens),
            "「Thanh Lại Hoàn」",
        )

    def test_japanese_coined_pass_character_uses_reviewable_semantic_fallback(self) -> None:
        tokens = [result("天", "thiên"), result("云", "vân"), {"t": 3, "i": "峠", "o": []}]

        self.assertEqual(
            fetch_han_viet.format_region_han_viet("天云峠", tokens),
            "Thiên Vân Đèo",
        )
        self.assertEqual(
            fetch_han_viet.REGION_FALLBACKS,
            {"峠": "đèo", "哐": "khuông"},
        )


if __name__ == "__main__":
    unittest.main()
