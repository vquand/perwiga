from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-weapon-han-viet.py"
SPEC = importlib.util.spec_from_file_location("fetch_weapon_han_viet", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
weapon_han_viet = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(weapon_han_viet)


def result(character: str, *alternatives: str) -> dict[str, object]:
    return {"t": 3, "i": character, "o": list(alternatives)}


class WeaponHanVietTests(unittest.TestCase):
    def catalog(self) -> dict[str, object]:
        return {
            "schema": "perwiga.genshin-impact.weapons.v1",
            "catalog_scope": {
                "included_records": 1,
                "excluded_non_obtainable_records": 3,
            },
            "weapons": [
                {
                    "source_key": "genshin-data-weapon-12502",
                    "official_english_name": "Wolf's Gravestone",
                    "official_simplified_chinese_name": "狼的末路",
                }
            ],
        }

    def test_build_snapshot_generates_separate_confidence_e_alias(self) -> None:
        response = {
            "message": "OK",
            "mode": "trans",
            "lang": "1",
            "result": [
                result("狼", "lang"),
                result("的", "đích"),
                result("末", "mạt"),
                result("路", "lộ"),
            ],
        }

        snapshot = weapon_han_viet.build_snapshot(
            self.catalog(), response, checked_at="2026-08-26"
        )

        self.assertEqual(
            snapshot["schema"], "perwiga.genshin-impact.weapon-han-viet.v1"
        )
        self.assertEqual(snapshot["confidence"]["han_viet_names"], "E")
        self.assertEqual(
            snapshot["weapons"],
            [
                {
                    "source_key": "genshin-data-weapon-12502",
                    "official_english_name": "Wolf's Gravestone",
                    "official_simplified_chinese_name": "狼的末路",
                    "han_viet_name": "Lang Đích Mạt Lộ",
                }
            ],
        )

    def test_catalog_requires_complete_unique_weapon_source_keys(self) -> None:
        catalog = self.catalog()
        catalog["weapons"] = [catalog["weapons"][0], catalog["weapons"][0]]
        catalog["catalog_scope"]["included_records"] = 2

        with self.assertRaisesRegex(ValueError, "duplicate"):
            weapon_han_viet.validate_catalog(catalog)

    def test_dictionary_fetch_is_batched_and_round_tripped(self) -> None:
        weapons = [
            {
                "source_key": f"genshin-data-weapon-{index}",
                "official_english_name": f"Weapon {index}",
                "official_simplified_chinese_name": name,
            }
            for index, name in enumerate(["甲", "乙", "丙"], start=1)
        ]
        calls: list[str] = []

        def post(text: str, **_: object) -> dict[str, object]:
            calls.append(text)
            names = text.splitlines()
            results: list[dict[str, object]] = []
            for index, name in enumerate(names):
                if index:
                    results.append({"t": 0, "i": "\n", "o": []})
                results.append(result(name, "reading"))
            return {"message": "OK", "mode": "trans", "lang": "1", "result": results}

        lines = weapon_han_viet.fetch_result_lines(
            weapons,
            batch_size=2,
            timeout=1.0,
            attempts=1,
            post_dictionary=post,
        )

        self.assertEqual(calls, ["甲\n乙", "丙"])
        self.assertEqual(
            [[token["i"] for token in line] for line in lines],
            [["甲"], ["乙"], ["丙"]],
        )

    def test_chinese_title_quotes_remain_literal_not_missing_readings(self) -> None:
        tokens = [
            {"t": 3, "i": "「", "o": []},
            result("渔", "ngư"),
            result("获", "hoạch"),
            {"t": 3, "i": "」", "o": []},
        ]

        self.assertEqual(
            weapon_han_viet.format_weapon_han_viet("「渔获」", tokens),
            "Ngư Hoạch",
        )

    def test_weapon_specific_unattested_ibis_character_is_explicit(self) -> None:
        tokens = [
            {"t": 3, "i": "鹮", "o": []},
            result("穿", "xuyên"),
            result("之", "chi"),
            result("喙", "hối"),
        ]

        self.assertEqual(
            weapon_han_viet.format_weapon_han_viet("鹮穿之喙", tokens),
            "Hoàn Xuyên Chi Hối",
        )
        self.assertEqual(weapon_han_viet.EXPLICIT_FALLBACKS, {"鹮": "hoàn"})


if __name__ == "__main__":
    unittest.main()
