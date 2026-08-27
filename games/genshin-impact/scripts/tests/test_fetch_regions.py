from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT_PATH = Path(__file__).parents[1] / "fetch-regions.py"
SPEC = importlib.util.spec_from_file_location("fetch_regions", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load {SCRIPT_PATH}")
fetch_regions = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fetch_regions)


class RegionSnapshotTests(unittest.TestCase):
    def write_geography(
        self,
        root: Path,
        locale: str,
        filename: str,
        *,
        geography_id: int,
        region_id: int,
        region_name: str,
        area_id: int,
        area_name: str | None,
    ) -> None:
        target = root / "src" / "data" / locale / "geographies" / filename
        target.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "id": geography_id,
            "name": f"Viewpoint {geography_id}",
            "regionId": region_id,
            "regionName": region_name,
            "areaId": area_id,
            "description": "A verified viewpoint description.",
            "sortOrder": geography_id,
        }
        if area_name is not None:
            payload["areaName"] = area_name
        target.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")

    def add_localized_record(
        self,
        root: Path,
        filename: str,
        *,
        geography_id: int,
        region_id: int,
        area_id: int,
        names: dict[str, tuple[str, str | None]],
    ) -> None:
        for locale, (region_name, area_name) in names.items():
            self.write_geography(
                root,
                locale,
                filename,
                geography_id=geography_id,
                region_id=region_id,
                region_name=region_name,
                area_id=area_id,
                area_name=area_name,
            )

    def test_snapshot_collapses_viewpoints_and_preserves_region_hierarchy(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mondstadt = {
                "English": ("Mondstadt", "Windrise"),
                "ChineseSimplified": ("蒙德", "风起地"),
                "ChineseTraditional": ("蒙德", "風起地"),
                "Vietnamese": ("Mondstadt", "Phong Khởi Địa"),
            }
            self.add_localized_record(
                root,
                "windswept.json",
                geography_id=70100005,
                region_id=1,
                area_id=1201,
                names=mondstadt,
            )
            self.add_localized_record(
                root,
                "second-view.json",
                geography_id=70100006,
                region_id=1,
                area_id=1201,
                names=mondstadt,
            )
            self.add_localized_record(
                root,
                "nodkrai.json",
                geography_id=70800001,
                region_id=7,
                area_id=70101,
                names={
                    "English": ("Nod-Krai", "Nasha Town"),
                    "ChineseSimplified": ("挪德卡莱", "那夏镇"),
                    "ChineseTraditional": ("挪德卡萊", "那夏鎮"),
                    "Vietnamese": ("Nod-Krai", "Thị Trấn Nasha"),
                },
            )
            self.add_localized_record(
                root,
                "snezhnaya.json",
                geography_id=70800040,
                region_id=8,
                area_id=80100,
                names={
                    "English": ("Snezhnaya", "Volkodlak Tundra"),
                    "ChineseSimplified": ("至冬", "沃尔科达拉克苔原"),
                    "ChineseTraditional": ("至冬", "沃爾科達拉克苔原"),
                    "Vietnamese": ("Snezhnaya", "Lãnh Nguyên Volkodlak"),
                },
            )

            snapshot = fetch_regions.build_snapshot(
                root,
                checked_at="2026-08-26",
                source_commit="8b15995fa220c88a4d0d7ffe1e21b041d0b32588",
            )

        self.assertEqual(snapshot["schema"], "perwiga.genshin-impact.regions.v1")
        self.assertEqual(snapshot["catalog_scope"]["viewpoint_records"], 4)
        self.assertEqual(snapshot["catalog_scope"]["included_records"], 7)
        records = {record["source_key"]: record for record in snapshot["regions"]}
        self.assertEqual(records["genshin-data-area-1-1201"]["viewpoint_ids"], [70100005, 70100006])
        self.assertEqual(records["genshin-data-area-1-1201"]["parent_source_key"], "genshin-data-region-1")
        self.assertEqual(records["genshin-data-region-7"]["parent_source_key"], "genshin-data-region-8")
        self.assertEqual(records["genshin-data-region-8"]["parent_source_key"], "genshin-world-teyvat")
        self.assertEqual(records["genshin-data-area-1-1201"]["official_simplified_chinese_name"], "风起地")

    def test_snapshot_rejects_locale_identity_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.add_localized_record(
                root,
                "windrise.json",
                geography_id=70100005,
                region_id=1,
                area_id=1201,
                names={
                    "English": ("Mondstadt", "Windrise"),
                    "ChineseSimplified": ("蒙德", "风起地"),
                    "ChineseTraditional": ("蒙德", "風起地"),
                    "Vietnamese": ("Mondstadt", "Phong Khởi Địa"),
                },
            )
            vietnamese = root / "src/data/Vietnamese/geographies/windrise.json"
            payload = json.loads(vietnamese.read_text(encoding="utf-8"))
            payload["areaId"] = 9999
            vietnamese.write_text(json.dumps(payload), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "mismatched geography identity"):
                fetch_regions.build_snapshot(
                    root,
                    checked_at="2026-08-26",
                    source_commit="8b15995fa220c88a4d0d7ffe1e21b041d0b32588",
                )


if __name__ == "__main__":
    unittest.main()
