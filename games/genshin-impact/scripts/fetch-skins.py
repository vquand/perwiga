#!/usr/bin/env python3
"""Build a reviewable Genshin Skin snapshot from outfits and HoYoWiki.

Non-default localized outfit records come from a pinned genshin-db checkout.
The official Genshin HoYoWiki Outfit catalog supplies stable page IDs, rarity,
and first-party thumbnail URLs. The script never opens SQLite.
"""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen


SCHEMA = "perwiga.genshin-impact.skins.v1"
SOURCE_URL = "https://github.com/theBowja/genshin-db"
HOYOWIKI_ARCHIVE_URL = "https://wiki.hoyolab.com/pc/genshin/aggregate/34"
HOYOWIKI_API_URL = (
    "https://sg-wiki-api.hoyolab.com/hoyowiki/wapi/get_entry_page_list"
)
HOYOWIKI_MENU_ID = "34"
MAX_RESPONSE_BYTES = 4 * 1024 * 1024
LOCALES = {
    "official_simplified_chinese_name": "ChineseSimplified",
    "official_traditional_chinese_name": "ChineseTraditional",
    "official_vietnamese_name": "Vietnamese",
}


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"unable to read valid JSON from {path}: {error}") from error


def required_text(value: Any, field: str, source_name: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{source_name} has invalid {field}")
    return value.strip()


def normalized_name(value: str) -> str:
    return value.replace("’", "'").replace("‘", "'")


def validate_icon_url(value: Any, source_name: str) -> tuple[str, str]:
    url = required_text(value, "outfit icon URL", source_name)
    parsed = urlparse(url)
    host = parsed.hostname or ""
    extension = Path(parsed.path).suffix.lower()
    if (
        parsed.scheme != "https"
        or not (host.endswith(".hoyoverse.com") or host.endswith(".hoyolab.com"))
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
        or extension not in {".png", ".gif"}
    ):
        raise ValueError(f"{source_name} has untrusted outfit icon URL: {url}")
    return url, extension


def wiki_rarity(entry: dict[str, Any]) -> int:
    filter_values = entry.get("filter_values")
    if not isinstance(filter_values, dict):
        raise ValueError("HoYoWiki outfit has no rarity filter")
    star_values = []
    for value in filter_values.values():
        if isinstance(value, dict) and isinstance(value.get("values"), list):
            star_values.extend(
                candidate
                for candidate in value["values"]
                if isinstance(candidate, str) and set(candidate) == {"★"}
            )
    if len(star_values) != 1 or len(star_values[0]) not in {4, 5}:
        raise ValueError("HoYoWiki outfit has an unsupported rarity")
    return len(star_values[0])


def fetch_hoyowiki_entries(
    *, timeout: float = 60.0, attempts: int = 3
) -> list[dict[str, Any]]:
    request_body = json.dumps(
        {
            "filters": [],
            "menu_id": HOYOWIKI_MENU_ID,
            "page_num": 1,
            "page_size": 50,
            "use_es": True,
        },
        separators=(",", ":"),
    ).encode("utf-8")
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = Request(
            HOYOWIKI_API_URL,
            data=request_body,
            method="POST",
            headers={
                "Content-Type": "application/json",
                "Origin": "https://wiki.hoyolab.com",
                "Referer": "https://wiki.hoyolab.com/",
                "User-Agent": "Perwiga Genshin Skin curator/1.0",
                "x-rpc-client_type": "4",
                "x-rpc-language": "en-us",
                "x-rpc-wiki_app": "genshin",
            },
        )
        try:
            with urlopen(request, timeout=timeout) as response:
                payload = response.read(MAX_RESPONSE_BYTES + 1)
            if len(payload) > MAX_RESPONSE_BYTES:
                raise ValueError("HoYoWiki outfit response exceeds the size limit")
            result = json.loads(payload)
            data = result.get("data") if isinstance(result, dict) else None
            entries = data.get("list") if isinstance(data, dict) else None
            total = int(data.get("total")) if isinstance(data, dict) else -1
            if result.get("retcode") != 0 or not isinstance(entries, list):
                raise ValueError("HoYoWiki outfit response is unsuccessful")
            if total != len(entries) or total > 50:
                raise ValueError("HoYoWiki outfit pagination is inconsistent")
            return entries
        except (HTTPError, URLError, TimeoutError, json.JSONDecodeError, ValueError) as error:
            last_error = error
            if attempt < attempts:
                time.sleep(min(attempt * 2, 6))
    raise RuntimeError(f"unable to fetch HoYoWiki outfits: {last_error}")


def localized_payload(source_root: Path, locale: str, filename: str) -> dict[str, Any]:
    payload = read_json(source_root / "src" / "data" / locale / "outfits" / filename)
    if not isinstance(payload, dict):
        raise ValueError(f"{locale}/{filename} must contain an object")
    return payload


def build_snapshot(
    source_root: Path,
    wiki_entries: list[dict[str, Any]],
    *,
    checked_at: str,
    source_commit: str,
) -> dict[str, Any]:
    english_dir = source_root / "src" / "data" / "English" / "outfits"
    if not english_dir.is_dir():
        raise ValueError("source root has no English outfit catalog")
    if len(source_commit) != 40 or not all(character in "0123456789abcdef" for character in source_commit.lower()):
        raise ValueError("source commit must be a 40-character hexadecimal hash")

    grouped: dict[str, list[dict[str, Any]]] = {}
    excluded_defaults = 0
    for path in sorted(english_dir.glob("*.json")):
        payload = read_json(path)
        if not isinstance(payload, dict):
            raise ValueError(f"{path} must contain an object")
        if payload.get("isDefault") is True:
            excluded_defaults += 1
            continue
        if payload.get("isDefault") is not False:
            raise ValueError(f"{path.stem} has an invalid default-outfit marker")
        outfit_id = payload.get("id")
        if not isinstance(outfit_id, int) or outfit_id <= 0:
            raise ValueError(f"{path.stem} has an invalid outfit ID")
        english_name = required_text(payload.get("name"), "English name", path.stem)
        record = {
            "filename": path.name,
            "genshin_db_key": path.stem,
            "genshin_data_id": outfit_id,
            "official_english_name": english_name,
            "official_english_description": required_text(
                payload.get("description"), "English description", path.stem
            ),
            "character": required_text(payload.get("characterName"), "character", path.stem),
        }
        for field, locale in LOCALES.items():
            localized = localized_payload(source_root, locale, path.name)
            record[field] = required_text(localized.get("name"), f"{locale} name", path.stem)
        grouped.setdefault(normalized_name(english_name), []).append(record)

    wiki_by_name: dict[str, dict[str, Any]] = {}
    entry_ids: set[str] = set()
    for entry in wiki_entries:
        if not isinstance(entry, dict):
            raise ValueError("HoYoWiki outfit must be an object")
        entry_id = entry.get("entry_page_id")
        name = required_text(entry.get("name"), "English name", "HoYoWiki outfit")
        if not isinstance(entry_id, str) or not entry_id.isdigit() or entry_id in entry_ids:
            raise ValueError(f"invalid or duplicate HoYoWiki outfit ID: {entry_id!r}")
        match_name = normalized_name(name)
        if match_name in wiki_by_name:
            raise ValueError(f"duplicate HoYoWiki outfit name: {name}")
        icon_url, extension = validate_icon_url(entry.get("icon_url"), name)
        wiki_by_name[match_name] = {
            "entry_id": entry_id,
            "name": name,
            "rarity": wiki_rarity(entry),
            "icon_url": icon_url,
            "extension": extension,
        }
        entry_ids.add(entry_id)

    if set(grouped) != set(wiki_by_name):
        missing = sorted(set(grouped) - set(wiki_by_name))
        unknown = sorted(set(wiki_by_name) - set(grouped))
        raise ValueError(f"Genshin Skin catalog mismatch; missing={missing}, unknown={unknown}")

    skins = []
    for match_name, records in grouped.items():
        wiki = wiki_by_name[match_name]
        first = records[0]
        for record in records[1:]:
            for field in [
                "official_english_name",
                "official_english_description",
                *LOCALES,
            ]:
                if record[field] != first[field]:
                    raise ValueError(f"multi-character Skin has inconsistent {field}: {match_name}")
        source_key = f"hoyowiki-outfit-{wiki['entry_id']}"
        skins.append(
            {
                "source_key": source_key,
                "hoyowiki_entry_page_id": wiki["entry_id"],
                "genshin_data_ids": sorted(record["genshin_data_id"] for record in records),
                "genshin_db_keys": sorted(record["genshin_db_key"] for record in records),
                "official_english_name": first["official_english_name"],
                "official_simplified_chinese_name": first[
                    "official_simplified_chinese_name"
                ],
                "official_traditional_chinese_name": first[
                    "official_traditional_chinese_name"
                ],
                "official_vietnamese_name": first["official_vietnamese_name"],
                "official_english_description": first["official_english_description"],
                "rarity": wiki["rarity"],
                "application_kind": "character",
                "applicable_characters": sorted(record["character"] for record in records),
                "applicable_weapons": [],
                "applicable_weapon_types": [],
                "thumbnail_asset": f"{source_key}{wiki['extension']}",
                "thumbnail_source_url": wiki["icon_url"],
            }
        )
    skins.sort(key=lambda skin: (skin["official_english_name"].casefold(), skin["source_key"]))
    return {
        "schema": SCHEMA,
        "checked_at": checked_at,
        "sources": {
            "localized_outfit_data": {"url": SOURCE_URL, "commit": source_commit},
            "rarity_identity_thumbnail": {
                "provider": "Official Genshin HoYoWiki Outfit catalog",
                "url": HOYOWIKI_ARCHIVE_URL,
                "menu_id": HOYOWIKI_MENU_ID,
            },
        },
        "catalog_scope": {
            "included_records": len(skins),
            "excluded_default_outfits": excluded_defaults,
            "application_kinds_present": ["character"],
            "confirmed_weapon_skins": 0,
        },
        "skins": skins,
    }


def write_snapshot(snapshot: dict[str, Any], output: Path, *, allow_shrink: bool) -> None:
    if output.exists() and not allow_shrink:
        existing = read_json(output)
        previous = existing.get("catalog_scope", {}).get("included_records")
        current = snapshot["catalog_scope"]["included_records"]
        if isinstance(previous, int) and current < previous:
            raise ValueError(f"refusing to shrink Skin snapshot from {previous} to {current}")
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
    parser.add_argument("--wiki-response", type=Path)
    parser.add_argument("--timeout", type=float, default=60.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--allow-shrink", action="store_true")
    parser.add_argument("--output", type=Path, default=module_root / "data/skins.json")
    args = parser.parse_args()

    if args.wiki_response:
        response = read_json(args.wiki_response)
        data = response.get("data") if isinstance(response, dict) else None
        entries = data.get("list") if isinstance(data, dict) else None
        if not isinstance(entries, list):
            raise ValueError("--wiki-response has no HoYoWiki outfit list")
    else:
        entries = fetch_hoyowiki_entries(timeout=args.timeout, attempts=args.attempts)
    snapshot = build_snapshot(
        args.source_root.resolve(),
        entries,
        checked_at=args.checked_at,
        source_commit=args.source_commit,
    )
    write_snapshot(snapshot, args.output.resolve(), allow_shrink=args.allow_shrink)
    print(f"wrote {len(snapshot['skins'])} Genshin Skins to {args.output}")


if __name__ == "__main__":
    main()
