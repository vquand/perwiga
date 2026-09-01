import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).parents[1] / "build-lore-appearance-snapshot.py"


def load_script():
    spec = importlib.util.spec_from_file_location("build_lore_appearance_snapshot", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class LoreAppearanceSnapshotTests(unittest.TestCase):
    def test_every_catalog_character_is_covered_without_inventing_placements(self):
        module = load_script()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tables = root / "tableCfg"
            mission = root / "dialog" / "json" / "mission" / "sm1l5m1"
            tables.mkdir(parents=True)
            mission.mkdir(parents=True)

            (tables / "CharacterTable.json").write_text(json.dumps({
                "chr_0012_avywen": {"name": {"en": "Avywenna"}},
                "chr_0099_unseen": {"name": {"en": "Unseen Operator"}},
            }), encoding="utf-8")
            (tables / "NpcTable.json").write_text(json.dumps({
                "avywen": {"dataKey": "npc_chr_0012_avywen", "name": {"en": "Avywenna"}},
                "fabian": {"dataKey": "npc_301_fabian", "name": {"en": "Fabian Collins"}},
                "unseen": {"dataKey": "npc_999_unseen", "name": {"en": "Unseen NPC"}},
            }), encoding="utf-8")
            (tables / "TextTable.json").write_text(json.dumps({
                "sm1l5m1_name": {"en": "Visitor from the Band I"},
                "sm1l5m1_desc_001": {"en": "Meet Fabian Collins."},
            }), encoding="utf-8")
            (mission / "dlg_sm1l5m1_1.json").write_text(json.dumps({
                "derivedData": {"privateActors": [
                    {"templateId": "npc_chr_0012_avywen"},
                    {"templateId": "npc_301_fabian"},
                ], "actors": []},
            }), encoding="utf-8")

            operators = {
                "operators": [
                    {"source_key": "avywenna", "official_english_name": "Avywenna"},
                    {"source_key": "unseen-operator", "official_english_name": "Unseen Operator"},
                ]
            }
            npcs = {
                "npcs": [
                    {"source_key": "fabian-collins", "official_english_name": "Fabian Collins"},
                    {"source_key": "unseen-npc", "official_english_name": "Unseen NPC"},
                ]
            }

            snapshot = module.build_snapshot(root, operators, npcs, checked_at="2026-09-01")

        self.assertEqual(snapshot["catalog_scope"], {
            "operators": 2,
            "npcs": 2,
            "characters": 4,
            "placed_characters": 2,
            "unplaced_characters": 2,
            "missions": 1,
        })
        mission_record = snapshot["missions"][0]
        self.assertEqual(mission_record["mission_id"], "sm1l5m1")
        self.assertEqual(mission_record["title_en"], "Visitor from the Band I")
        self.assertEqual(mission_record["period_key"], "valley-iv-after-main")
        self.assertEqual(
            {(actor["entity_type"], actor["source_key"]) for actor in mission_record["characters"]},
            {("operator", "avywenna"), ("npc", "fabian-collins")},
        )
        self.assertEqual(
            {(actor["entity_type"], actor["source_key"]) for actor in snapshot["unplaced_characters"]},
            {("operator", "unseen-operator"), ("npc", "unseen-npc")},
        )
        self.assertIn("dialog/json/mission/sm1l5m1", mission_record["source_locator"])


if __name__ == "__main__":
    unittest.main()
