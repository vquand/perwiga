from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-artifacts.py"
SPEC = importlib.util.spec_from_file_location("fetch_artifacts", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
artifacts = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(artifacts)


class ArtifactSnapshotTests(unittest.TestCase):
    def localized_set(self, set_name: str, piece_name: str) -> dict[str, object]:
        return {
            "id": 15003,
            "rarityList": [4, 5],
            "name": set_name,
            "effect2Pc": "Increases Elemental Mastery by 80.",
            "effect4Pc": "Increases Charged Attack DMG by 35%.",
            "flower": {
                "name": piece_name,
                "relicType": "EQUIP_BRACER",
                "relicText": "Flower of Life",
                "description": "A flower description.",
                "story": "A flower story.",
            },
        }

    def write_fixture(self, root: Path) -> None:
        names = {
            "English": ("Wanderer's Troupe", "Troupe's Dawnlight"),
            "ChineseSimplified": ("流浪大地的乐团", "乐团的晨光"),
            "ChineseTraditional": ("流浪大地的樂團", "樂團的晨光"),
            "Vietnamese": ("Đoàn Hát Lang Thang Đại Lục", "Ánh Bình Minh Của Đoàn Hát"),
        }
        import json

        for locale, (set_name, piece_name) in names.items():
            directory = root / "src" / "data" / locale / "artifacts"
            directory.mkdir(parents=True, exist_ok=True)
            (directory / "wandererstroupe.json").write_text(
                json.dumps(self.localized_set(set_name, piece_name)), encoding="utf-8"
            )
        image_dir = root / "src" / "data" / "image"
        image_dir.mkdir(parents=True)
        (image_dir / "artifacts.json").write_text(
            json.dumps(
                {
                    "wandererstroupe": {
                        "flower": "https://upload-os-bbs.mihoyo.com/game_record/genshin/equip/UI_RelicIcon_15003_4.png"
                    }
                }
            ),
            encoding="utf-8",
        )

    def wiki_entry(self) -> dict[str, object]:
        return {
            "entry_page_id": "231",
            "name": "Wanderer's Troupe",
            "icon_url": "https://act-webstatic.hoyoverse.com/event-static-hoyowiki-admin/set.png",
            "filter_values": {
                "rarity": {"values": ["★★★★", "★★★★★"]},
                "effect": {"values": ["Elemental Mastery", "Charged Attack"]},
            },
        }

    def test_snapshot_preserves_set_piece_hierarchy_and_locales(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.write_fixture(root)
            snapshot = artifacts.build_snapshot(
                root,
                [self.wiki_entry()],
                checked_at="2026-08-26",
                source_commit="a" * 40,
            )

        self.assertEqual(snapshot["catalog_scope"]["artifact_sets"], 1)
        self.assertEqual(snapshot["catalog_scope"]["artifact_pieces"], 1)
        artifact_set = snapshot["artifact_sets"][0]
        piece = snapshot["artifact_pieces"][0]
        self.assertEqual(artifact_set["source_key"], "genshin-data-artifact-set-15003")
        self.assertEqual(artifact_set["hoyowiki_entry_page_id"], "231")
        self.assertEqual(piece["source_key"], "genshin-data-artifact-piece-15003-flower")
        self.assertEqual(piece["artifact_set_source_key"], artifact_set["source_key"])
        self.assertEqual(piece["slot"], "Flower of Life")
        self.assertEqual(piece["official_simplified_chinese_name"], "乐团的晨光")
        self.assertEqual(piece["official_vietnamese_name"], "Ánh Bình Minh Của Đoàn Hát")
        self.assertEqual(piece["thumbnail_asset"], f"{piece['source_key']}.png")

    def test_unmatched_wiki_entry_and_foreign_thumbnail_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.write_fixture(root)
            entry = self.wiki_entry()
            entry["name"] = "Unknown Set"
            with self.assertRaisesRegex(ValueError, "catalog mismatch"):
                artifacts.build_snapshot(root, [entry], checked_at="2026-08-26", source_commit="a" * 40)
            entry = self.wiki_entry()
            entry["icon_url"] = "https://example.com/set.png"
            with self.assertRaisesRegex(ValueError, "untrusted"):
                artifacts.build_snapshot(root, [entry], checked_at="2026-08-26", source_commit="a" * 40)


if __name__ == "__main__":
    unittest.main()
