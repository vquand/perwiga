from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT_PATH = Path(__file__).parents[1] / "fetch-characters.py"
SPEC = importlib.util.spec_from_file_location("fetch_characters", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load {SCRIPT_PATH}")
fetch_characters = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fetch_characters)


def record(
    info_id: int,
    title: str,
    *,
    icon_url: str = "https://fastcdn.hoyoverse.com/content-v2/hk4e/1/icon.png",
    description: str = "<p>A brave <strong>knight</strong>.&nbsp;</p>",
) -> dict:
    return {
        "sChanId": ["403", "407"],
        "sTitle": title,
        "sExt": (
            '{"407_0":[{"name":"icon.png","url":"'
            + icon_url
            + '"}],"407_7":"'
            + description.replace('"', '\\"')
            + '"}'
        ),
        "dtStartTime": "2026-06-08 12:00:00",
        "dtEndTime": "2035-01-01 00:00:00",
        "dtCreateTime": "2026-05-14 15:25:06",
        "iInfoId": info_id,
    }


def catalogs(*, vietnamese: bool = True, icon_url: str | None = None) -> dict:
    result = {
        locale: {region: [] for region, _, _ in fetch_characters.REGION_CHANNELS}
        for _, locale in fetch_characters.LOCALES
    }
    english_options = {} if icon_url is None else {"icon_url": icon_url}
    result["en-us"]["Mondstadt"] = [record(163984, "Lohen", **english_options)]
    result["zh-tw"]["Mondstadt"] = [record(163984, "洛恩")]
    if vietnamese:
        result["vi-vn"]["Mondstadt"] = [record(163984, "Lohen")]
    return result


class CharacterSnapshotTests(unittest.TestCase):
    def test_build_snapshot_joins_official_locales_by_content_id(self) -> None:
        fetched = catalogs()

        snapshot = fetch_characters.build_snapshot(
            fetched,
            checked_at="2026-08-24",
            generated_at="2026-08-24T12:00:00Z",
        )

        self.assertEqual(snapshot["schema"], "perwiga.genshin-impact.characters.v1")
        self.assertEqual(snapshot["catalog_scope"]["included_records"], 1)
        self.assertEqual(snapshot["catalog_scope"]["locale_gaps"], [])
        self.assertEqual(
            snapshot["sources"]["official_character_directory"],
            "https://genshin.hoyoverse.com/en/character/mondstadt",
        )
        self.assertEqual(
            snapshot["sources"]["official_content_api"],
            "https://sg-public-api-static.hoyoverse.com/content_v2_user",
        )

        character = snapshot["characters"][0]
        self.assertEqual(character["source_key"], "hoyoverse-content-163984")
        self.assertEqual(character["official_content_id"], 163984)
        self.assertEqual(character["official_english_name"], "Lohen")
        self.assertEqual(character["official_traditional_chinese_name"], "洛恩")
        self.assertEqual(character["official_vietnamese_name"], "Lohen")
        self.assertEqual(character["official_english_description"], "A brave knight.")
        self.assertEqual(character["region"], "Mondstadt")
        self.assertEqual(character["region_channel_id"], 403)
        self.assertEqual(character["thumbnail_asset"], "hoyoverse-content-163984.png")

    def test_missing_official_locale_is_preserved_as_an_explicit_gap(self) -> None:
        fetched = catalogs(vietnamese=False)

        snapshot = fetch_characters.build_snapshot(
            fetched,
            checked_at="2026-08-24",
            generated_at="2026-08-24T12:00:00Z",
        )

        self.assertIsNone(snapshot["characters"][0]["official_vietnamese_name"])
        self.assertEqual(
            snapshot["catalog_scope"]["locale_gaps"],
            [
                {
                    "source_key": "hoyoverse-content-163984",
                    "locale": "vi-vn",
                    "field": "official_vietnamese_name",
                }
            ],
        )

    def test_duplicate_content_id_is_rejected_at_the_external_boundary(self) -> None:
        fetched = catalogs()
        fetched["en-us"]["Mondstadt"].append(record(163984, "Duplicate"))

        with self.assertRaisesRegex(ValueError, "duplicate official content ID 163984"):
            fetch_characters.build_snapshot(
                fetched,
                checked_at="2026-08-24",
                generated_at="2026-08-24T12:00:00Z",
            )

    def test_untrusted_thumbnail_host_is_rejected(self) -> None:
        fetched = catalogs(icon_url="https://example.com/icon.png")

        with self.assertRaisesRegex(ValueError, "untrusted thumbnail URL"):
            fetch_characters.build_snapshot(
                fetched,
                checked_at="2026-08-24",
                generated_at="2026-08-24T12:00:00Z",
            )

    def test_existing_snapshot_cannot_shrink_without_explicit_override(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "characters.json"
            output.write_text(
                '{"schema":"perwiga.genshin-impact.characters.v1",'
                '"catalog_scope":{"included_records":2},"characters":[{},{}]}\n',
                encoding="utf-8",
            )
            smaller = {
                "schema": "perwiga.genshin-impact.characters.v1",
                "catalog_scope": {"included_records": 1},
                "characters": [{}],
            }

            with self.assertRaisesRegex(ValueError, "would shrink.*2 to 1"):
                fetch_characters.validate_replacement(output, smaller, allow_shrink=False)

            fetch_characters.validate_replacement(output, smaller, allow_shrink=True)


class OfficialContentApiTests(unittest.TestCase):
    def test_request_url_uses_the_published_app_and_channel_contract(self) -> None:
        url = fetch_characters.content_list_url(
            channel_id=403,
            locale="en-us",
            page=2,
            page_size=100,
        )

        self.assertEqual(
            url,
            "https://sg-public-api-static.hoyoverse.com/content_v2_user/"
            "app/a1b1f9d3315447cc/getContentList?"
            "iAppId=32&iChanId=403&iPageSize=100&iPage=2&sLangKey=en-us",
        )

    def test_invalid_api_shape_is_rejected_before_transformation(self) -> None:
        with self.assertRaisesRegex(ValueError, "retcode"):
            fetch_characters.validate_page({"retcode": -1, "message": "busy"})

        with self.assertRaisesRegex(ValueError, "data.list"):
            fetch_characters.validate_page(
                {"retcode": 0, "message": "OK", "data": {"iTotal": 1, "list": {}}}
            )


if __name__ == "__main__":
    unittest.main()
