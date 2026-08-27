from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SCRIPT_PATH = Path(__file__).parents[1] / "fetch-character-han-viet.py"
SPEC = importlib.util.spec_from_file_location("fetch_character_han_viet", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load {SCRIPT_PATH}")
fetch_han_viet = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fetch_han_viet)


def result(character: str, *alternatives: str, capitalize: bool = False) -> dict:
    return {
        "t": 3,
        "i": character,
        "o": list(alternatives),
        "c": capitalize,
    }


class HanVietSnapshotTests(unittest.TestCase):
    def test_dictionary_request_headers_are_http_encodable(self) -> None:
        request = fetch_han_viet.dictionary_request("阿貝多")

        for _, value in request.header_items():
            value.encode("latin-1")

    def test_build_snapshot_uses_dictionary_readings_and_labels_generation(self) -> None:
        catalog = {
            "schema": "perwiga.genshin-impact.characters.v1",
            "catalog_scope": {"included_records": 1},
            "characters": [
                {
                    "source_key": "hoyoverse-content-104816",
                    "official_english_name": "Albedo",
                    "official_traditional_chinese_name": "阿貝多",
                }
            ],
        }
        response = {
            "message": "OK",
            "mode": "trans",
            "lang": "1",
            "result": [
                result("阿", "a", "á", capitalize=True),
                result("貝", "bối"),
                result("多", "đa"),
            ],
        }

        snapshot = fetch_han_viet.build_snapshot(
            catalog, response, checked_at="2026-08-25"
        )

        self.assertEqual(
            snapshot["schema"],
            "perwiga.genshin-impact.character-han-viet.v1",
        )
        self.assertEqual(snapshot["catalog_scope"]["included_records"], 1)
        self.assertEqual(snapshot["confidence"]["han_viet_names"], "E")
        self.assertEqual(
            snapshot["characters"],
            [
                {
                    "source_key": "hoyoverse-content-104816",
                    "official_english_name": "Albedo",
                    "official_traditional_chinese_name": "阿貝多",
                    "han_viet_name": "A Bối Đa",
                }
            ],
        )

    def test_genshin_specific_fallback_covers_unattested_transliteration_character(self) -> None:
        catalog = {
            "schema": "perwiga.genshin-impact.characters.v1",
            "catalog_scope": {"included_records": 1},
            "characters": [
                {
                    "source_key": "hoyoverse-content-104818",
                    "official_english_name": "Eula",
                    "official_traditional_chinese_name": "優菈",
                }
            ],
        }
        response = {
            "message": "OK",
            "mode": "trans",
            "lang": "1",
            "result": [result("優", "ưu", capitalize=True), result("菈")],
        }

        snapshot = fetch_han_viet.build_snapshot(
            catalog, response, checked_at="2026-08-25"
        )

        self.assertEqual(snapshot["characters"][0]["han_viet_name"], "Ưu Lạp")
        self.assertEqual(
            snapshot["generation"]["explicit_fallbacks"], {"菈": "lạp"}
        )

    def test_response_must_round_trip_every_catalog_name(self) -> None:
        catalog = {
            "schema": "perwiga.genshin-impact.characters.v1",
            "catalog_scope": {"included_records": 1},
            "characters": [
                {
                    "source_key": "hoyoverse-content-104816",
                    "official_english_name": "Albedo",
                    "official_traditional_chinese_name": "阿貝多",
                }
            ],
        }
        response = {
            "message": "OK",
            "mode": "trans",
            "lang": "1",
            "result": [result("阿", "a", capitalize=True), result("貝", "bối")],
        }

        with self.assertRaisesRegex(ValueError, "round-trip"):
            fetch_han_viet.build_snapshot(
                catalog, response, checked_at="2026-08-25"
            )


if __name__ == "__main__":
    unittest.main()
