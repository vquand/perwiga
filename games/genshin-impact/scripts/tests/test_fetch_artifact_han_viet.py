from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-artifact-han-viet.py"
SPEC = importlib.util.spec_from_file_location("fetch_artifact_han_viet", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
han_viet = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(han_viet)


class ArtifactHanVietTests(unittest.TestCase):
    def catalog(self) -> dict[str, object]:
        return {
            "schema": han_viet.CATALOG_SCHEMA,
            "catalog_scope": {"artifact_sets": 1, "artifact_pieces": 1},
            "artifact_sets": [{"source_key": "genshin-data-artifact-set-1", "official_english_name": "Crimson Witch", "official_simplified_chinese_name": "炽烈的炎之魔女"}],
            "artifact_pieces": [{"source_key": "genshin-data-artifact-piece-1-flower", "official_english_name": "Witch's Flower", "official_simplified_chinese_name": "魔女的炎之花"}],
        }

    def test_aliases_are_separate_and_confidence_e(self) -> None:
        results = [{"i": character, "t": 3, "o": [reading]} for character, reading in [("炽", "sí"), ("烈", "liệt"), ("的", "đích"), ("炎", "viêm"), ("之", "chi"), ("魔", "ma"), ("女", "nữ"), ("魔", "ma"), ("女", "nữ"), ("的", "đích"), ("炎", "viêm"), ("之", "chi"), ("花", "hoa")]]
        snapshot = han_viet.build_snapshot_from_lines(self.catalog(), [results[:7], results[7:]], checked_at="2026-08-26")
        self.assertEqual(snapshot["confidence"]["han_viet_names"], "E")
        self.assertEqual(snapshot["catalog_scope"], {"artifact_sets": 1, "artifact_pieces": 1})
        self.assertEqual(snapshot["artifact_pieces"][0]["han_viet_name"], "Ma Nữ Đích Viêm Chi Hoa")

    def test_rejects_duplicate_source_keys(self) -> None:
        catalog = self.catalog()
        catalog["artifact_pieces"][0]["source_key"] = "genshin-data-artifact-set-1"
        with self.assertRaisesRegex(ValueError, "invalid|duplicate"):
            han_viet.validate_catalog(catalog)


if __name__ == "__main__":
    unittest.main()
