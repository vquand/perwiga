import importlib.util
import unittest
from pathlib import Path


def load(name):
    path = Path(__file__).parents[1] / name
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


MODULE = load("fetch-npc-han-viet.py")


class NpcHanVietTests(unittest.TestCase):
    def test_generated_alias_stays_separate_and_missing_chinese_is_explicit(self):
        catalog = {
            "schema": MODULE.CATALOG_SCHEMA,
            "catalog_scope": {"included_records": 2},
            "npcs": [
                {"source_key": "npc-1", "official_english_name": "Liben", "official_simplified_chinese_name": "立本"},
                {"source_key": "npc-2", "official_english_name": "Aarav", "official_simplified_chinese_name": ""},
            ],
        }
        response = {"message": "OK", "mode": "trans", "lang": "1", "result": [
            {"i": "立", "t": 3, "o": ["lì"]},
            {"i": "本", "t": 3, "o": ["běn"]},
            {"i": "\r"},
        ]}
        snapshot = MODULE.build_snapshot(catalog, [response], checked_at="2026-08-27")
        self.assertEqual(snapshot["catalog_scope"]["aliases"], 1)
        self.assertEqual(snapshot["catalog_scope"]["missing_chinese_names"], 1)
        self.assertEqual(snapshot["npcs"][0]["han_viet_name"], "Lì Běn")
        self.assertEqual(snapshot["confidence"]["han_viet_names"], "E")


if __name__ == "__main__":
    unittest.main()
