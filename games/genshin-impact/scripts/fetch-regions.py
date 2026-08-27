#!/usr/bin/env python3
"""Build a reviewable Genshin Region hierarchy from pinned Geography data.

The game-data Geography catalog stores one record per unlockable viewpoint.
This script collapses records sharing a stable regionId/areaId into one Region
or subregion entry, aligns four in-game locales by viewpoint ID, and writes a
reviewable snapshot atomically. It never opens SQLite.
"""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import tempfile
from typing import Any


SCHEMA = "perwiga.genshin-impact.regions.v1"
SOURCE_URL = "https://github.com/theBowja/genshin-db"
LOCALES = {
    "English": ("official_english_name", "regionName", "areaName"),
    "ChineseSimplified": (
        "official_simplified_chinese_name",
        "regionName",
        "areaName",
    ),
    "ChineseTraditional": (
        "official_traditional_chinese_name",
        "regionName",
        "areaName",
    ),
    "Vietnamese": ("official_vietnamese_name", "regionName", "areaName"),
}
WORLD_NAMES = {
    "official_english_name": "Teyvat",
    "official_simplified_chinese_name": "提瓦特",
    "official_traditional_chinese_name": "提瓦特",
    "official_vietnamese_name": "Teyvat",
}
REGION_PARENT_OVERRIDES = {7: 8}  # Nod-Krai is within Snezhnaya.
REGION_SUBTYPES = {7: "autonomous-region", 105: "special-region"}
PLACEHOLDER_AREA_NAMES = {"???"}


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"unable to read valid JSON from {path}: {error}") from error


def required_text(value: Any, field: str, source: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{source} has invalid {field}")
    return value.strip()


def load_locale(source_root: Path, locale: str) -> dict[int, dict[str, Any]]:
    directory = source_root / "src" / "data" / locale / "geographies"
    if not directory.is_dir():
        raise ValueError(f"source root has no {locale} Geography catalog")
    records: dict[int, dict[str, Any]] = {}
    for path in sorted(directory.glob("*.json")):
        payload = read_json(path)
        if not isinstance(payload, dict):
            raise ValueError(f"{path} is not a Geography object")
        identity = payload.get("id")
        if not isinstance(identity, int) or identity <= 0 or identity in records:
            raise ValueError(f"{path} has invalid Geography identity")
        records[identity] = payload
    if not records:
        raise ValueError(f"{locale} Geography catalog is empty")
    return records


def one_localized_value(
    records: dict[int, dict[str, Any]],
    ids: list[int],
    field: str,
    source_key: str,
) -> str:
    values = {
        required_text(records[identity].get(field), field, source_key)
        for identity in ids
    }
    if len(values) != 1:
        raise ValueError(f"{source_key} has inconsistent {field} values: {values}")
    return values.pop()


def build_snapshot(
    source_root: Path,
    *,
    checked_at: str,
    source_commit: str,
) -> dict[str, Any]:
    if len(source_commit) != 40 or not all(
        character in "0123456789abcdef" for character in source_commit.lower()
    ):
        raise ValueError("source commit must be a 40-character hexadecimal hash")
    localized = {locale: load_locale(source_root, locale) for locale in LOCALES}
    english = localized["English"]
    english_ids = set(english)
    for locale, records in localized.items():
        if set(records) != english_ids:
            raise ValueError(f"{locale} Geography identities do not match English")
        for identity, payload in records.items():
            anchor = english[identity]
            if (
                payload.get("id") != identity
                or payload.get("regionId") != anchor.get("regionId")
                or payload.get("areaId") != anchor.get("areaId")
            ):
                raise ValueError(
                    f"{locale}/{identity} has mismatched geography identity"
                )

    region_groups: dict[int, list[int]] = {}
    area_groups: dict[tuple[int, int], list[int]] = {}
    unnamed_viewpoints = 0
    placeholder_areas: set[tuple[int, int]] = set()
    for identity, payload in english.items():
        region_id = payload.get("regionId")
        area_id = payload.get("areaId")
        if not isinstance(region_id, int) or region_id <= 0:
            raise ValueError(f"English/{identity} has invalid regionId")
        if not isinstance(area_id, int) or area_id <= 0:
            raise ValueError(f"English/{identity} has invalid areaId")
        region_groups.setdefault(region_id, []).append(identity)
        area_name = payload.get("areaName")
        if not isinstance(area_name, str) or not area_name.strip():
            unnamed_viewpoints += 1
            continue
        if area_name.strip() in PLACEHOLDER_AREA_NAMES:
            placeholder_areas.add((region_id, area_id))
            continue
        area_groups.setdefault((region_id, area_id), []).append(identity)

    records: list[dict[str, Any]] = [
        {
            "source_key": "genshin-world-teyvat",
            **WORLD_NAMES,
            "subtype": "world",
            "parent_source_key": None,
            "region_id": None,
            "area_id": None,
            "viewpoint_ids": [],
            "official_english_description": "The world containing the mapped Genshin Impact regions in this catalog.",
            "source_table": "curated-world-anchor",
        }
    ]
    for region_id in sorted(region_groups):
        ids = sorted(region_groups[region_id])
        source_key = f"genshin-data-region-{region_id}"
        names = {}
        for locale, (output_field, region_field, _) in LOCALES.items():
            names[output_field] = one_localized_value(
                localized[locale], ids, region_field, source_key
            )
        parent_id = REGION_PARENT_OVERRIDES.get(region_id)
        records.append(
            {
                "source_key": source_key,
                **names,
                "subtype": REGION_SUBTYPES.get(region_id, "nation"),
                "parent_source_key": (
                    f"genshin-data-region-{parent_id}"
                    if parent_id is not None
                    else "genshin-world-teyvat"
                ),
                "region_id": region_id,
                "area_id": None,
                "viewpoint_ids": ids,
                "official_english_description": "A top-level region represented in the in-game Geography Archive.",
                "source_table": "Geography",
            }
        )

    area_records = []
    for (region_id, area_id), ids in area_groups.items():
        ids.sort()
        source_key = f"genshin-data-area-{region_id}-{area_id}"
        names = {}
        for locale, (output_field, _, area_field) in LOCALES.items():
            names[output_field] = one_localized_value(
                localized[locale], ids, area_field, source_key
            )
        parent_key = f"genshin-data-region-{region_id}"
        parent_name = next(
            record["official_english_name"]
            for record in records
            if record["source_key"] == parent_key
        )
        area_records.append(
            {
                "source_key": source_key,
                **names,
                "subtype": "subregion",
                "parent_source_key": parent_key,
                "region_id": region_id,
                "area_id": area_id,
                "viewpoint_ids": ids,
                "official_english_description": (
                    f"A mapped area in {parent_name}, represented by "
                    f"{len(ids)} in-game Geography Archive viewpoint record(s)."
                ),
                "source_table": "Geography",
            }
        )
    area_records.sort(
        key=lambda record: (
            record["region_id"],
            record["official_english_name"].casefold(),
            record["area_id"],
        )
    )
    records.extend(area_records)
    for index, record in enumerate(records, start=1):
        record["catalog_index"] = index

    return {
        "schema": SCHEMA,
        "checked_at": checked_at,
        "source": {
            "url": SOURCE_URL,
            "commit": source_commit,
            "upstream": "GenshinData game-data extracts distributed by genshin-db",
        },
        "catalog_scope": {
            "viewpoint_records": len(english),
            "regions": len(region_groups),
            "subregions": len(area_records),
            "included_records": len(records),
            "excluded_unnamed_viewpoints": unnamed_viewpoints,
            "excluded_placeholder_areas": len(placeholder_areas),
        },
        "notes": [
            "The hierarchy collapses in-game Geography Archive viewpoints by stable regionId and areaId.",
            "Subregions are areas attested by at least one permanent Geography Archive viewpoint; this is not a claim that every map label is present.",
            "Nod-Krai is parented to Snezhnaya while retaining its distinct in-game Geography region identity.",
            "Unnamed and placeholder area labels are excluded rather than invented.",
        ],
        "regions": records,
    }


def write_snapshot(snapshot: dict[str, Any], output: Path, *, allow_shrink: bool) -> None:
    if output.exists() and not allow_shrink:
        existing = read_json(output)
        previous = existing.get("catalog_scope", {}).get("included_records")
        current = snapshot["catalog_scope"]["included_records"]
        if isinstance(previous, int) and current < previous:
            raise ValueError(
                f"refusing to shrink Region catalog from {previous} to {current}"
            )
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{output.name}.", suffix=".tmp", dir=output.parent
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(snapshot, handle, ensure_ascii=False, indent=2)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_name, output)
    finally:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass


def main() -> None:
    module_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument(
        "--source-commit",
        default="8b15995fa220c88a4d0d7ffe1e21b041d0b32588",
    )
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--allow-shrink", action="store_true")
    parser.add_argument(
        "--output", type=Path, default=module_root / "data" / "regions.json"
    )
    arguments = parser.parse_args()
    snapshot = build_snapshot(
        arguments.source_root.resolve(),
        checked_at=arguments.checked_at,
        source_commit=arguments.source_commit,
    )
    write_snapshot(snapshot, arguments.output.resolve(), allow_shrink=arguments.allow_shrink)
    print(
        f"wrote {snapshot['catalog_scope']['included_records']} Region records "
        f"from {snapshot['catalog_scope']['viewpoint_records']} viewpoints to {arguments.output}"
    )


if __name__ == "__main__":
    main()
