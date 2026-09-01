#!/usr/bin/env python3
"""Generate reviewable Honkai: Star Rail catalog snapshots.

The default source is the public StarRailStaticAPI mirror of StarRailRes. A
StarRailRes checkout can be supplied with --source-dir for reproducible local
refreshes. This script never opens or modifies Perwiga's SQLite database.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import subprocess
import tempfile
import urllib.request
from pathlib import Path
from typing import Any


LANGUAGES = ("en", "vi", "cn", "cht")
ENDPOINTS = ("characters", "light_cones", "paths", "elements", "relic_sets", "relics")
OUTPUTS = {
    "characters": "characters.json",
    "light_cones": "light-cones.json",
    "paths": "paths.json",
    "elements": "elements.json",
    "relic_sets": "relic-sets.json",
    "relics": "relics.json",
}
DEFAULT_BASE_URL = "https://vizualabstract.github.io/StarRailStaticAPI/db"
MAX_RESPONSE_BYTES = 25 * 1024 * 1024
SCHEMA_PREFIX = "perwiga.honkai-star-rail"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source-dir",
        type=Path,
        help="StarRailRes checkout, extracted locale directory, or static-API cache",
    )
    parser.add_argument("--source-commit", help="Pinned upstream commit when --source-dir has no .git")
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--checked-at", help="Snapshot date in YYYY-MM-DD; defaults to today")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "data",
    )
    parser.add_argument(
        "--allow-shrink",
        action="store_true",
        help="Allow a refreshed catalog to contain fewer records than the existing snapshot",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    try:
        with path.open(encoding="utf-8") as handle:
            payload = json.load(handle)
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"unable to read JSON source {path}: {error}") from error
    validate_payload(payload, str(path))
    return payload


def validate_payload(payload: Any, label: str) -> None:
    if not isinstance(payload, dict) or not payload:
        raise RuntimeError(f"{label} must be a non-empty JSON object keyed by source ID")
    for key, value in payload.items():
        if not isinstance(key, str) or not key.strip():
            raise RuntimeError(f"{label} contains an invalid source key")
        if not isinstance(value, dict):
            raise RuntimeError(f"{label} record {key} is not a JSON object")
        if str(value.get("id", "")) != key:
            raise RuntimeError(f"{label} record {key} has a mismatched id")


def read_source(source_dir: Path | None, base_url: str, language: str, endpoint: str) -> dict[str, Any]:
    if source_dir is not None:
        candidates = [
            source_dir / "index_new" / language / f"{endpoint}.json",
            source_dir / "index_min" / language / f"{endpoint}.json",
            source_dir / "db" / language / f"{endpoint}.json",
            source_dir / language / f"{endpoint}.json",
        ]
        for candidate in candidates:
            if candidate.is_file():
                return load_json(candidate)
        searched = ", ".join(str(candidate) for candidate in candidates)
        raise RuntimeError(f"missing {language}/{endpoint}.json; searched {searched}")

    url = f"{base_url.rstrip('/')}/{language}/{endpoint}.json"
    request = urllib.request.Request(url, headers={"User-Agent": "Perwiga-HSR-Catalog/1.0"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = response.read(MAX_RESPONSE_BYTES + 1)
    except OSError as error:
        raise RuntimeError(f"unable to fetch {url}: {error}") from error
    if len(body) > MAX_RESPONSE_BYTES:
        raise RuntimeError(f"source response is larger than {MAX_RESPONSE_BYTES} bytes: {url}")
    try:
        payload = json.loads(body.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RuntimeError(f"source response is not valid UTF-8 JSON: {url}: {error}") from error
    validate_payload(payload, url)
    return payload


def source_commit(source_dir: Path | None, explicit: str | None) -> str | None:
    if explicit:
        return explicit
    if source_dir is None:
        return None
    try:
        return subprocess.check_output(
            ["git", "-C", str(source_dir), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def required_text(record: dict[str, Any], field: str, label: str) -> str:
    value = record.get(field)
    if not isinstance(value, str) or not value.strip():
        raise RuntimeError(f"{label} record {record.get('id', '?')} has no non-empty {field}")
    return value.strip()


def optional_text(record: dict[str, Any], field: str) -> str | None:
    value = record.get(field)
    if value is None:
        return None
    if not isinstance(value, str):
        raise RuntimeError(f"record {record.get('id', '?')} field {field} is not text")
    value = value.strip()
    return value or None


def sorted_items(payload: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    return sorted(payload.items(), key=lambda item: (int(item[0]) if item[0].isdigit() else item[0]))


def locale_gap(ids: set[str], localized: dict[str, dict[str, Any]]) -> list[str]:
    return sorted(set(localized) - ids)


def common_record(
    source_id: str,
    localized: dict[str, dict[str, Any]],
    *,
    include_descriptions: bool = False,
) -> dict[str, Any]:
    english = localized["en"]
    vietnamese = localized["vi"]
    simplified = localized["cn"]
    traditional = localized["cht"]
    result: dict[str, Any] = {
        "source_key": source_id,
        "game_data_id": english["id"],
        "official_english_name": required_text(english, "name", "English"),
        "official_simplified_chinese_name": required_text(simplified, "name", "Simplified Chinese"),
        "official_traditional_chinese_name": required_text(traditional, "name", "Traditional Chinese"),
        "official_vietnamese_name": required_text(vietnamese, "name", "Vietnamese"),
    }
    if include_descriptions:
        result.update(
            {
                "official_english_description": optional_text(english, "desc"),
                "official_simplified_chinese_description": optional_text(simplified, "desc"),
                "official_traditional_chinese_description": optional_text(traditional, "desc"),
                "official_vietnamese_description": optional_text(vietnamese, "desc"),
            }
        )
    return result


def source_metadata(commit: str | None, base_url: str) -> dict[str, Any]:
    return {
        "repository": "https://github.com/Mar-7th/StarRailRes",
        "source_data": "https://github.com/Dimbreath/StarRailData",
        "static_api": base_url.rstrip("/") + "/",
        "commit": commit,
        "upstream": "StarRailRes index_new multilingual game-data extracts; not an official HoYoverse API",
    }


def make_header(kind: str, checked_at: str, source: dict[str, Any], count: int) -> dict[str, Any]:
    return {
        "schema": f"{SCHEMA_PREFIX}.{kind}.v1",
        "checked_at": checked_at,
        "source": source,
        "catalog_scope": {
            "included_records": count,
            "locale_set": list(LANGUAGES),
        },
    }


def build_catalogs(raw: dict[str, dict[str, dict[str, Any]]], checked_at: str, source: dict[str, Any]) -> dict[str, dict[str, Any]]:
    paths = raw["paths"]
    elements = raw["elements"]
    path_names = {
        language: {key: required_text(value, "name", f"{language} path") for key, value in paths[language].items()}
        for language in LANGUAGES
    }
    path_english_labels = {
        key: required_text(value, "name", "English path") for key, value in paths["en"].items()
    }
    element_names = {
        language: {key: required_text(value, "name", f"{language} element") for key, value in elements[language].items()}
        for language in LANGUAGES
    }
    element_english_labels = {
        key: required_text(value, "name", "English element") for key, value in elements["en"].items()
    }

    result: dict[str, dict[str, Any]] = {}
    for endpoint in ENDPOINTS:
        records_by_language = raw[endpoint]
        ids = set(records_by_language["en"])
        for language in LANGUAGES[1:]:
            if set(records_by_language[language]) != ids:
                raise RuntimeError(
                    f"{endpoint} locale IDs differ for {language}; missing={sorted(ids - set(records_by_language[language]))} "
                    f"extra={locale_gap(ids, records_by_language[language])}"
                )

        records: list[dict[str, Any]] = []
        for item_id, english in sorted_items(records_by_language["en"]):
            localized = {language: records_by_language[language][item_id] for language in LANGUAGES}
            if endpoint == "characters":
                path_key = required_text(english, "path", "Character")
                element_key = required_text(english, "element", "Character")
                if path_key not in path_english_labels or element_key not in element_english_labels:
                    raise RuntimeError(f"Character {item_id} refers to an unknown path or element")
                record = common_record(f"starrailres-character-{item_id}", localized)
                record.update(
                    {
                        "rarity": english.get("rarity"),
                        "path_key": path_key,
                        "path_english_name": path_english_labels[path_key],
                        "path_vietnamese_name": path_names["vi"][path_key],
                        "element_key": element_key,
                        "element_english_name": element_english_labels[element_key],
                        "element_vietnamese_name": element_names["vi"][element_key],
                        "max_sp": english.get("max_sp"),
                        "icon_path": required_text(english, "icon", "Character"),
                        "thumbnail_path": required_text(english, "portrait", "Character"),
                    }
                )
                records.append(record)
            elif endpoint == "light_cones":
                path_key = required_text(english, "path", "Light Cone")
                if path_key not in path_english_labels:
                    raise RuntimeError(f"Light Cone {item_id} refers to an unknown path")
                record = common_record(f"starrailres-light-cone-{item_id}", localized, include_descriptions=True)
                record.update(
                    {
                        "rarity": english.get("rarity"),
                        "path_key": path_key,
                        "path_english_name": path_english_labels[path_key],
                        "path_vietnamese_name": path_names["vi"][path_key],
                        "icon_path": required_text(english, "icon", "Light Cone"),
                        "thumbnail_path": required_text(english, "portrait", "Light Cone"),
                    }
                )
                records.append(record)
            elif endpoint == "relic_sets":
                record = common_record(f"starrail-relic-set-{item_id}", localized)
                descriptions: dict[str, list[str]] = {}
                for language in LANGUAGES:
                    description = localized[language].get("desc")
                    if not isinstance(description, list) or not all(isinstance(value, str) for value in description):
                        raise RuntimeError(f"Relic Set {item_id} has an invalid {language} description")
                    descriptions[language] = [value.strip() for value in description if value.strip()]
                record.update(
                    {
                        "descriptions": {
                            "en": descriptions["en"],
                            "vi": descriptions["vi"],
                            "cn": descriptions["cn"],
                            "cht": descriptions["cht"],
                        },
                        "icon_path": required_text(english, "icon", "Relic Set"),
                    }
                )
                records.append(record)
            elif endpoint == "relics":
                set_id = required_text(english, "set_id", "Relic")
                record = common_record(f"starrail-relic-{item_id}", localized)
                relic_type = required_text(english, "type", "Relic")
                slot_labels = {
                    "HEAD": "Head",
                    "HAND": "Hands",
                    "BODY": "Body",
                    "FOOT": "Feet",
                    "NECK": "Planar Sphere",
                    "OBJECT": "Link Rope",
                }
                if relic_type not in slot_labels:
                    raise RuntimeError(f"Relic {item_id} has unknown slot type {relic_type}")
                record.update(
                    {
                        "relic_set_source_key": f"starrail-relic-set-{set_id}",
                        "relic_set_game_data_id": set_id,
                        "slot_key": relic_type,
                        "slot": slot_labels[relic_type],
                        "rarity": english.get("rarity"),
                        "max_level": english.get("max_level"),
                        "main_affix_id": english.get("main_affix_id"),
                        "sub_affix_id": english.get("sub_affix_id"),
                        "icon_path": required_text(english, "icon", "Relic"),
                        "thumbnail_path": "icon/relic/" + set_id + "_0.png",
                    }
                )
                records.append(record)
            elif endpoint in ("paths", "elements"):
                kind = "path" if endpoint == "paths" else "element"
                record = common_record(f"starrail-{kind}-{item_id}", localized, include_descriptions=True)
                record.update(
                    {
                        "source_key_name": item_id,
                        "color": english.get("color") if endpoint == "elements" else None,
                        "icon_path": required_text(english, "icon", kind.title()),
                    }
                )
                records.append(record)
            else:
                raise AssertionError(endpoint)

        if endpoint == "characters":
            field_provenance = {
                "names_facets_rarity": "StarRailRes index_new extracts sourced from Dimbreath/StarRailData",
                "description": "Not supplied by the StarRailRes basic character endpoint; remains null rather than inferred",
                "thumbnail": "StarRailRes asset path, served by its public static mirror",
            }
        elif endpoint == "light_cones":
            field_provenance = {
                "names_descriptions_facets_rarity": "StarRailRes index_new extracts sourced from Dimbreath/StarRailData",
                "thumbnail": "StarRailRes asset path, served by its public static mirror",
            }
        elif endpoint == "relic_sets":
            field_provenance = {
                "names_descriptions": "StarRailRes index_new extracts sourced from Dimbreath/StarRailData",
                "thumbnail": "StarRailRes asset path, served by its public static mirror",
            }
        elif endpoint == "relics":
            field_provenance = {
                "names_slots_rarity": "StarRailRes index_new extracts sourced from Dimbreath/StarRailData",
                "parent_set": "StarRailRes set_id relationship",
                "thumbnail": "StarRailRes asset path, served by its public static mirror",
            }
        else:
            field_provenance = {
                "names_descriptions": "StarRailRes index_new extracts sourced from Dimbreath/StarRailData",
                "thumbnail": "StarRailRes asset path, served by its public static mirror",
            }
        snapshot = make_header(endpoint.replace("_", "-"), checked_at, source, len(records))
        snapshot["field_provenance"] = field_provenance
        snapshot[OUTPUTS[endpoint].split(".")[0].replace("-", "_")] = records
        result[endpoint] = snapshot
    return result


def assert_no_shrink(output_dir: Path, catalogs: dict[str, dict[str, Any]], allow_shrink: bool) -> None:
    if allow_shrink:
        return
    for endpoint, snapshot in catalogs.items():
        output_path = output_dir / OUTPUTS[endpoint]
        if not output_path.is_file():
            continue
        try:
            with output_path.open(encoding="utf-8") as handle:
                previous = json.load(handle)
        except (OSError, json.JSONDecodeError) as error:
            raise RuntimeError(f"existing snapshot is not readable: {output_path}: {error}") from error
        previous_count = previous.get("catalog_scope", {}).get("included_records")
        current_count = snapshot["catalog_scope"]["included_records"]
        if isinstance(previous_count, int) and current_count < previous_count:
            raise RuntimeError(
                f"refusing to shrink {output_path.name} from {previous_count} to {current_count}; review and pass --allow-shrink"
            )


def write_atomically(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    encoded = json.dumps(payload, ensure_ascii=False, indent=2) + "\n"
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", dir=path.parent, delete=False) as handle:
        handle.write(encoded)
        temporary = Path(handle.name)
    os.replace(temporary, path)


def main() -> int:
    args = parse_args()
    checked_at = args.checked_at or dt.date.today().isoformat()
    try:
        dt.date.fromisoformat(checked_at)
    except ValueError as error:
        raise RuntimeError("--checked-at must be YYYY-MM-DD") from error

    raw = {
        endpoint: {
            language: read_source(args.source_dir, args.base_url, language, endpoint)
            for language in LANGUAGES
        }
        for endpoint in ENDPOINTS
    }
    source = source_metadata(source_commit(args.source_dir, args.source_commit), args.base_url)
    catalogs = build_catalogs(raw, checked_at, source)
    assert_no_shrink(args.output_dir, catalogs, args.allow_shrink)
    for endpoint, snapshot in catalogs.items():
        write_atomically(args.output_dir / OUTPUTS[endpoint], snapshot)
        print(f"wrote {OUTPUTS[endpoint]} ({snapshot['catalog_scope']['included_records']} records)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as error:
        raise SystemExit(f"error: {error}")
