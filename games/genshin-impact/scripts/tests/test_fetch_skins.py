from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-skins.py"
SPEC = importlib.util.spec_from_file_location("fetch_skins", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
skins = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(skins)


class FetchSkinsTests(unittest.TestCase):
    def make_source(self, root: Path) -> None:
        records = {
            "traveler-aether.json": {
                "id": 200501,
                "name": "As Heaven and Earth Are Made Anew",
                "description": "Traveler outfit.",
                "isDefault": False,
                "characterName": "Aether",
            },
            "traveler-lumine.json": {
                "id": 200701,
                "name": "As Heaven and Earth Are Made Anew",
                "description": "Traveler outfit.",
                "isDefault": False,
                "characterName": "Lumine",
            },
            "diluc.json": {
                "id": 201601,
                "name": "Red Dead of Night",
                "description": "Diluc outfit.",
                "isDefault": False,
                "characterName": "Diluc",
            },
            "default.json": {
                "id": 201602,
                "name": "Darknight Blaze",
                "description": "Default outfit.",
                "isDefault": True,
                "characterName": "Diluc",
            },
        }
        localized_names = {
            "ChineseSimplified": {
                "traveler-aether.json": "复地重天",
                "traveler-lumine.json": "复地重天",
                "diluc.json": "殷红终夜",
                "default.json": "暗夜微光",
            },
            "ChineseTraditional": {
                "traveler-aether.json": "復地重天",
                "traveler-lumine.json": "復地重天",
                "diluc.json": "殷紅終夜",
                "default.json": "暗夜微光",
            },
            "Vietnamese": {
                "traveler-aether.json": "Phục Địa Trùng Thiên",
                "traveler-lumine.json": "Phục Địa Trùng Thiên",
                "diluc.json": "Đêm Đỏ Rực Lửa",
                "default.json": "Ánh Lửa Đêm Tối",
            },
        }
        for locale in ["English", *localized_names]:
            directory = root / "src" / "data" / locale / "outfits"
            directory.mkdir(parents=True)
            for filename, record in records.items():
                payload = dict(record)
                if locale != "English":
                    payload["name"] = localized_names[locale][filename]
                (directory / filename).write_text(
                    json.dumps(payload, ensure_ascii=False), encoding="utf-8"
                )

    def wiki_entries(self) -> list[dict[str, object]]:
        return [
            {
                "entry_page_id": "9251",
                "name": "As Heaven and Earth Are Made Anew",
                "icon_url": "https://act-webstatic.hoyoverse.com/event-static-hoyowiki-admin/traveler.gif",
                "filter_values": {"rarity": {"values": ["★★★★"]}},
            },
            {
                "entry_page_id": "5777",
                "name": "Red Dead of Night",
                "icon_url": "https://act-webstatic.hoyoverse.com/event-static-hoyowiki-admin/diluc.png",
                "filter_values": {"rarity": {"values": ["★★★★★"]}},
            },
        ]

    def test_snapshot_groups_multi_character_skin_and_excludes_defaults(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            self.make_source(source)

            snapshot = skins.build_snapshot(
                source,
                self.wiki_entries(),
                checked_at="2026-08-26",
                source_commit="8b15995fa220c88a4d0d7ffe1e21b041d0b32588",
            )

        self.assertEqual(snapshot["catalog_scope"]["included_records"], 2)
        traveler = next(
            skin for skin in snapshot["skins"] if skin["hoyowiki_entry_page_id"] == "9251"
        )
        self.assertEqual(traveler["source_key"], "hoyowiki-outfit-9251")
        self.assertEqual(traveler["rarity"], 4)
        self.assertEqual(traveler["applicable_characters"], ["Aether", "Lumine"])
        self.assertEqual(traveler["applicable_weapons"], [])
        self.assertEqual(traveler["applicable_weapon_types"], [])
        self.assertEqual(traveler["thumbnail_asset"], "hoyowiki-outfit-9251.gif")

        diluc = next(skin for skin in snapshot["skins"] if skin["hoyowiki_entry_page_id"] == "5777")
        self.assertEqual(diluc["rarity"], 5)
        self.assertEqual(diluc["official_simplified_chinese_name"], "殷红终夜")
        self.assertEqual(diluc["official_traditional_chinese_name"], "殷紅終夜")
        self.assertEqual(diluc["official_vietnamese_name"], "Đêm Đỏ Rực Lửa")

    def test_unmatched_wiki_or_untrusted_icon_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            self.make_source(source)
            entries = self.wiki_entries()
            entries[0]["name"] = "Unknown Outfit"
            with self.assertRaisesRegex(ValueError, "catalog mismatch"):
                skins.build_snapshot(
                    source,
                    entries,
                    checked_at="2026-08-26",
                    source_commit="8b15995fa220c88a4d0d7ffe1e21b041d0b32588",
                )

            entries = self.wiki_entries()
            entries[0]["icon_url"] = "https://example.com/traveler.gif"
            with self.assertRaisesRegex(ValueError, "untrusted outfit icon URL"):
                skins.build_snapshot(
                    source,
                    entries,
                    checked_at="2026-08-26",
                    source_commit="8b15995fa220c88a4d0d7ffe1e21b041d0b32588",
                )


if __name__ == "__main__":
    unittest.main()
