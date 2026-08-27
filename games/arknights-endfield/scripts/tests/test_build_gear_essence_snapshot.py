import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).parents[1] / "build-gear-essence-snapshot.py"


def load_script():
    spec = importlib.util.spec_from_file_location("build_gear_essence_snapshot", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class GearEssenceSnapshotTests(unittest.TestCase):
    def test_catalog_excludes_internal_essences_and_preserves_official_names(self):
        module = load_script()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tables = root / "tableCfg"
            tables.mkdir()
            fixtures = {
                "EquipSuitTable.json": {
                    "suit_agi01": {
                        "equipList": ["gear_a"],
                        "list": [{
                            "equipCnt": "3",
                            "skillID": "skill_set",
                            "suitID": "suit_agi01",
                            "suitLogoName": "icon_set",
                            "suitName": {"en": "Roving MSGR", "cn": "巡行信使"},
                        }],
                    }
                },
                "ItemTable.json": {
                    "gear_a": {"rarity": "4", "name": {"en": "Cap", "cn": "帽"}},
                    "gem_sword_0001_442": {"rarity": "5", "name": {"en": "Flawless Essence", "cn": "无瑕基质"}, "iconId": "item_gem_rarity_5"},
                    "gem_test_1": {"rarity": "5", "name": {"en": "Flawless Essence", "cn": "无瑕基质"}, "iconId": "item_gem_rarity_5"},
                    "gem_guide_1": {"rarity": "5", "name": {"en": "Flawless Essence", "cn": "无瑕基质"}, "iconId": "item_gem_rarity_5"},
                    "wpn_sword_0001": {"name": {"en": "Test Sword", "cn": "测试剑"}},
                },
                "GemPresetTable.json": {
                    key: {
                        "domainId": "domain_1", "gemItemId": key, "rarity": "5",
                        "realItemId": "item_gem_rarity_5",
                        "termList": [
                            {"level": "4", "termId": "primary"},
                            {"level": "4", "termId": "secondary"},
                            {"level": "2", "termId": "skill"},
                        ],
                    }
                    for key in ("gem_sword_0001_442", "gem_test_1", "gem_guide_1")
                },
                "GemTable.json": {
                    "primary": {"tagId": "attr_agi", "tagName": {"en": "Agility Boost", "cn": "敏捷提升"}, "termType": "0", "isSkillTerm": False},
                    "secondary": {"tagId": "attr_atk", "tagName": {"en": "Attack Boost", "cn": "攻击提升"}, "termType": "1", "isSkillTerm": False},
                    "skill": {"tagId": "burst", "tagName": {"en": "Ultimate Gain", "cn": "终结技增幅"}, "termType": "2", "isSkillTerm": True},
                },
                "SkillPatchTable.json": {
                    "skill_set": {"SkillPatchDataBundle": [{"level": "1", "description": {"en": "3-piece set effect", "cn": "3件套组效果"}}]}
                },
                "GemTagKeyToWeaponTable.json": {
                    "attr_agi+attr_atk+burst": {"list": ["wpn_sword_0001"]}
                },
            }
            for filename, value in fixtures.items():
                (tables / filename).write_text(json.dumps(value), encoding="utf-8")

            snapshot = module.build_snapshot(root, checked_at="2026-08-26")

        self.assertEqual(snapshot["catalog_scope"]["gear_sets"], 1)
        self.assertEqual(snapshot["catalog_scope"]["essences"], 1)
        self.assertEqual(snapshot["catalog_scope"]["excluded_test_essences"], 1)
        self.assertEqual(snapshot["catalog_scope"]["excluded_guide_essences"], 1)
        gear = snapshot["gear_sets"][0]
        self.assertEqual(gear["official_english_name"], "Roving MSGR")
        self.assertEqual(gear["member_item_ids"], ["gear_a"])
        self.assertEqual(gear["rarities"], [4])
        essence = snapshot["essences"][0]
        self.assertEqual(essence["official_english_name"], "Flawless Essence")
        self.assertEqual(
            essence["catalog_label"],
            "Flawless Essence — Test Sword — Agility Boost Lv.4 · Attack Boost Lv.4 · Ultimate Gain Lv.2",
        )
        self.assertEqual(essence["weapon_type"], "Sword")
        self.assertEqual(essence["term_kinds"], ["Primary attribute", "Secondary attribute", "Skill effect"])


if __name__ == "__main__":
    unittest.main()
