from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-hoyowiki-weapon-thumbnails.py"
SPEC = importlib.util.spec_from_file_location("fetch_hoyowiki_weapon_thumbnails", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
hoyowiki = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(hoyowiki)


class HoyowikiWeaponThumbnailTests(unittest.TestCase):
    def catalog(self) -> dict[str, object]:
        return {
            "schema": hoyowiki.CATALOG_SCHEMA,
            "catalog_scope": {"included_records": 2},
            "weapons": [
                {
                    "source_key": "genshin-data-weapon-12502",
                    "official_english_name": "Wolf's Gravestone",
                    "thumbnail_asset": "genshin-data-weapon-12502.png",
                },
                {
                    "source_key": "genshin-data-weapon-11412",
                    "official_english_name": "Sword of Descension",
                    "thumbnail_asset": "genshin-data-weapon-11412.png",
                },
            ],
        }

    def entry(self) -> dict[str, object]:
        return {
            "entry_page_id": "246",
            "name": "Wolf's Gravestone",
            "icon_url": "https://act-webstatic.hoyoverse.com/event-static-hoyowiki-admin/weapon.png",
        }

    def test_snapshot_records_exact_matches_and_explicit_gaps(self) -> None:
        snapshot = hoyowiki.build_snapshot(
            self.catalog(), [self.entry()], checked_at="2026-08-26"
        )

        self.assertEqual(snapshot["catalog_scope"]["wiki_matches"], 1)
        self.assertEqual(
            snapshot["catalog_scope"]["missing_from_hoyowiki"],
            ["genshin-data-weapon-11412"],
        )
        self.assertEqual(snapshot["weapons"][0]["official_english_name"], "Wolf's Gravestone")

    def test_duplicate_wiki_names_are_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate HoYoWiki weapon name"):
            hoyowiki.build_snapshot(
                self.catalog(), [self.entry(), self.entry()], checked_at="2026-08-26"
            )

    def test_typographic_apostrophe_variants_match(self) -> None:
        catalog = self.catalog()
        catalog["weapons"] = [
            {
                "source_key": "genshin-data-weapon-13512",
                "official_english_name": "Crimson Moon's Semblance",
                "thumbnail_asset": "genshin-data-weapon-13512.png",
            }
        ]
        catalog["catalog_scope"]["included_records"] = 1
        entry = self.entry()
        entry["name"] = "Crimson Moon’s Semblance"

        snapshot = hoyowiki.build_snapshot(catalog, [entry], checked_at="2026-08-26")

        self.assertEqual(snapshot["catalog_scope"]["wiki_matches"], 1)

    def test_catalog_title_quotes_do_not_block_a_wiki_match(self) -> None:
        catalog = self.catalog()
        catalog["weapons"] = [
            {
                "source_key": "genshin-data-weapon-12425",
                "official_english_name": '"Ultimate Overlord\'s Mega Magic Sword"',
                "thumbnail_asset": "genshin-data-weapon-12425.png",
            }
        ]
        catalog["catalog_scope"]["included_records"] = 1
        entry = self.entry()
        entry["name"] = "Ultimate Overlord's Mega Magic Sword"

        snapshot = hoyowiki.build_snapshot(catalog, [entry], checked_at="2026-08-26")

        self.assertEqual(snapshot["catalog_scope"]["wiki_matches"], 1)

    def test_foreign_icon_host_is_rejected(self) -> None:
        entry = self.entry()
        entry["icon_url"] = "https://example.com/weapon.png"
        with self.assertRaisesRegex(ValueError, "untrusted HoYoWiki icon URL"):
            hoyowiki.build_snapshot(self.catalog(), [entry], checked_at="2026-08-26")

    def test_unicode_icon_paths_are_percent_encoded_for_requests(self) -> None:
        self.assertEqual(
            hoyowiki.requestable_url("https://bbs.hoyolab.com/hoyowiki/picture/weapon/「渔获」.png"),
            "https://bbs.hoyolab.com/hoyowiki/picture/weapon/%E3%80%8C%E6%B8%94%E8%8E%B7%E3%80%8D.png",
        )

    def test_page_fetch_uses_all_pages_and_validates_total(self) -> None:
        pages = {
            1: {"retcode": 0, "data": {"total": "2", "list": [self.entry()]}},
            2: {
                "retcode": 0,
                "data": {
                    "total": "2",
                    "list": [
                        {
                            "entry_page_id": "247",
                            "name": "Sword of Descension",
                            "icon_url": "https://act-webstatic.hoyoverse.com/event-static-hoyowiki-admin/sword.png",
                        }
                    ],
                },
            },
        }

        entries = hoyowiki.fetch_entries(
            lambda page_number, _page_size: pages[page_number], page_size=1
        )

        self.assertEqual([entry["entry_page_id"] for entry in entries], ["246", "247"])


if __name__ == "__main__":
    unittest.main()
