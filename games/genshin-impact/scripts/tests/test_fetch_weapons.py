from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-weapons.py"
SPEC = importlib.util.spec_from_file_location("fetch_weapons", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
fetch_weapons = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fetch_weapons)


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload), encoding="utf-8")


class FetchWeaponsTests(unittest.TestCase):
    def fixture_source(self, root: Path) -> None:
        weapon = {
            "id": 12502,
            "name": "Wolf's Gravestone",
            "description": "A longsword used by the Wolf Knight.",
            "weaponText": "Claymore",
            "rarity": 5,
            "baseAtkValue": 45.9364,
            "mainStatType": "FIGHT_PROP_ATTACK_PERCENT",
            "mainStatText": "ATK",
            "baseStatText": "10.8%",
            "costs": {
                "ascend1": [
                    {"id": 202, "name": "Mora", "count": 10000},
                    {
                        "id": 114009,
                        "name": "Fetters of the Dandelion Gladiator",
                        "count": 5,
                    },
                ]
            },
        }
        write_json(root / "src/data/English/weapons/wolfsgravestone.json", weapon)
        write_json(
            root / "src/data/ChineseSimplified/weapons/wolfsgravestone.json",
            {**weapon, "name": "狼的末路"},
        )
        write_json(
            root / "src/data/ChineseTraditional/weapons/wolfsgravestone.json",
            {**weapon, "name": "狼的末路"},
        )
        write_json(
            root / "src/data/Vietnamese/weapons/wolfsgravestone.json",
            {**weapon, "name": "Đường Cùng Của Sói"},
        )
        write_json(
            root / "src/data/image/weapons.json",
            {
                "wolfsgravestone": {
                    "mihoyo_icon": "https://upload-os-bbs.mihoyo.com/game_record/genshin/equip/UI_EquipIcon_Claymore_Wolfmound.png",
                    "filename_icon": "UI_EquipIcon_Claymore_Wolfmound",
                }
            },
        )
        write_json(root / "src/data/version/weapons.json", {"wolfsgravestone": "1.0"})

    def test_build_snapshot_joins_locales_and_derives_filter_fields(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture_source(root)

            snapshot = fetch_weapons.build_snapshot(
                root, checked_at="2026-08-25", source_commit="fixture-commit"
            )

        self.assertEqual(snapshot["schema"], "perwiga.genshin-impact.weapons.v1")
        self.assertEqual(snapshot["catalog_scope"]["included_records"], 1)
        weapon = snapshot["weapons"][0]
        self.assertEqual(weapon["source_key"], "genshin-data-weapon-12502")
        self.assertEqual(weapon["official_simplified_chinese_name"], "狼的末路")
        self.assertEqual(weapon["official_vietnamese_name"], "Đường Cùng Của Sói")
        self.assertEqual(weapon["weapon_type"], "Claymore")
        self.assertEqual(weapon["rarity"], 5)
        self.assertEqual(weapon["base_attack"], 46)
        self.assertEqual(weapon["substat"], "ATK")
        self.assertEqual(weapon["ascension_region"], "Mondstadt")
        self.assertEqual(weapon["thumbnail_asset"], "genshin-data-weapon-12502.png")
        self.assertEqual(
            weapon["thumbnail_fallback_url"],
            "https://enka.network/ui/UI_EquipIcon_Claymore_Wolfmound.png",
        )

    def test_build_snapshot_excludes_non_obtainable_duplicate_variants(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture_source(root)
            path = root / "src/data/English/weapons/prizedisshinblade.json"
            write_json(
                path,
                {
                    "id": 11420,
                    "name": "Prized Isshin Blade",
                    "dupealias": "Prized Isshin Blade 1",
                    "description": "Unobtainable quest state.",
                    "weaponText": "Sword",
                    "rarity": 4,
                    "baseAtkValue": 42.401,
                    "mainStatType": "FIGHT_PROP_ATTACK_PERCENT",
                    "mainStatText": "ATK",
                    "baseStatText": "NaN",
                    "costs": {"ascend1": []},
                },
            )

            snapshot = fetch_weapons.build_snapshot(
                root, checked_at="2026-08-25", source_commit="fixture-commit"
            )

        self.assertEqual(snapshot["catalog_scope"]["included_records"], 1)
        self.assertEqual(snapshot["catalog_scope"]["excluded_non_obtainable_records"], 1)

    def test_unmapped_ascension_material_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture_source(root)
            weapon_path = root / "src/data/English/weapons/wolfsgravestone.json"
            weapon = json.loads(weapon_path.read_text(encoding="utf-8"))
            weapon["costs"]["ascend1"][1]["name"] = "Unknown Future Material"
            write_json(weapon_path, weapon)

            with self.assertRaisesRegex(ValueError, "unmapped ascension material"):
                fetch_weapons.build_snapshot(
                    root, checked_at="2026-08-25", source_commit="fixture-commit"
                )


if __name__ == "__main__":
    unittest.main()
