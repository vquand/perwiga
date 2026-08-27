#!/usr/bin/env python3
"""Build a reviewable multilingual Artifact Set and Artifact Piece snapshot."""

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


SCHEMA = "perwiga.genshin-impact.artifacts.v1"
SOURCE_URL = "https://github.com/theBowja/genshin-db"
HOYOWIKI_ARCHIVE_URL = "https://wiki.hoyolab.com/pc/genshin/aggregate/5"
HOYOWIKI_API_URL = "https://sg-wiki-api.hoyolab.com/hoyowiki/wapi/get_entry_page_list"
HOYOWIKI_MENU_ID = "5"
MAX_RESPONSE_BYTES = 6 * 1024 * 1024
LOCALES = {
    "official_simplified_chinese_name": "ChineseSimplified",
    "official_traditional_chinese_name": "ChineseTraditional",
    "official_vietnamese_name": "Vietnamese",
}
SLOTS = ("flower", "plume", "sands", "goblet", "circlet")
HOYOWIKI_PIECE_ICON_FIELDS = {
    "flower": "flower_of_life_icon_url",
    "plume": "plume_of_death_icon_url",
    "sands": "sands_of_eon_icon_url",
    "goblet": "goblet_of_eonothem_icon_url",
    "circlet": "circlet_of_logos_icon_url",
}
ALLOWED_IMAGE_HOSTS = frozenset(
    {
        "act-webstatic.hoyoverse.com",
        "act-upload.hoyoverse.com",
        "bbs.hoyolab.com",
        "upload-os-bbs.mihoyo.com",
        "upload-static.hoyoverse.com",
    }
)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"unable to read valid JSON from {path}: {error}") from error


def required_text(value: Any, field: str, source: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{source} has invalid {field}")
    return value.strip()


def normalized_name(value: str) -> str:
    return value.replace("’", "'").replace("‘", "'")


def validate_image_url(value: Any, source: str) -> str:
    url = required_text(value, "thumbnail URL", source)
    parsed = urlparse(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname not in ALLOWED_IMAGE_HOSTS
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
        or Path(parsed.path).suffix.lower() != ".png"
    ):
        raise ValueError(f"{source} has untrusted thumbnail URL: {url}")
    return url


def wiki_rarities(entry: dict[str, Any]) -> list[int]:
    values = []
    filters = entry.get("filter_values")
    if isinstance(filters, dict):
        for value in filters.values():
            if isinstance(value, dict) and isinstance(value.get("values"), list):
                values.extend(
                    len(candidate)
                    for candidate in value["values"]
                    if isinstance(candidate, str) and candidate and set(candidate) == {"★"}
                )
    rarities = sorted(set(values))
    if not rarities or any(rarity not in range(1, 6) for rarity in rarities):
        raise ValueError("HoYoWiki Artifact has no supported rarity classification")
    return rarities


def wiki_effect_tags(entry: dict[str, Any]) -> list[str]:
    filters = entry.get("filter_values")
    if not isinstance(filters, dict):
        return []
    tags = []
    for key, value in filters.items():
        if key == "filter_key_43" or not isinstance(value, dict):
            continue
        candidates = value.get("values")
        if isinstance(candidates, list):
            tags.extend(candidate.strip() for candidate in candidates if isinstance(candidate, str) and candidate.strip())
    return sorted(set(tags), key=str.casefold)


def fetch_hoyowiki_entries(*, timeout: float, attempts: int) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    expected_total: int | None = None
    page = 1
    while expected_total is None or len(entries) < expected_total:
        body = json.dumps(
            {"filters": [], "menu_id": HOYOWIKI_MENU_ID, "page_num": page, "page_size": 50, "use_es": True},
            separators=(",", ":"),
        ).encode()
        last_error: Exception | None = None
        for attempt in range(1, attempts + 1):
            request = Request(
                HOYOWIKI_API_URL,
                data=body,
                method="POST",
                headers={
                    "Content-Type": "application/json",
                    "Origin": "https://wiki.hoyolab.com",
                    "Referer": "https://wiki.hoyolab.com/",
                    "User-Agent": "Perwiga Genshin Artifact curator/1.0",
                    "x-rpc-client_type": "4",
                    "x-rpc-language": "en-us",
                    "x-rpc-wiki_app": "genshin",
                },
            )
            try:
                with urlopen(request, timeout=timeout) as response:
                    payload = response.read(MAX_RESPONSE_BYTES + 1)
                if len(payload) > MAX_RESPONSE_BYTES:
                    raise ValueError("HoYoWiki Artifact response exceeds the size limit")
                result = json.loads(payload)
                data = result.get("data") if isinstance(result, dict) else None
                page_entries = data.get("list") if isinstance(data, dict) else None
                total = int(data.get("total")) if isinstance(data, dict) else -1
                if result.get("retcode") != 0 or not isinstance(page_entries, list) or total < 0:
                    raise ValueError("HoYoWiki Artifact response is unsuccessful")
                if expected_total is not None and expected_total != total:
                    raise ValueError("HoYoWiki Artifact total changed during pagination")
                expected_total = total
                entries.extend(page_entries)
                break
            except (HTTPError, URLError, TimeoutError, json.JSONDecodeError, ValueError) as error:
                last_error = error
                if attempt < attempts:
                    time.sleep(min(attempt * 2, 6))
        else:
            raise RuntimeError(f"unable to fetch HoYoWiki Artifacts: {last_error}")
        page += 1
    if len(entries) != expected_total:
        raise ValueError("HoYoWiki Artifact pagination is inconsistent")
    return entries


def localized_payload(source_root: Path, locale: str, filename: str) -> dict[str, Any]:
    payload = read_json(source_root / "src" / "data" / locale / "artifacts" / filename)
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
    english_dir = source_root / "src" / "data" / "English" / "artifacts"
    image_map = read_json(source_root / "src" / "data" / "image" / "artifacts.json")
    if not english_dir.is_dir() or not isinstance(image_map, dict):
        raise ValueError("source root has no Artifact catalog or image index")
    if len(source_commit) != 40 or not all(character in "0123456789abcdef" for character in source_commit.lower()):
        raise ValueError("source commit must be a 40-character hexadecimal hash")

    local_sets: dict[str, tuple[str, dict[str, Any], dict[str, dict[str, Any]]]] = {}
    for path in sorted(english_dir.glob("*.json")):
        english = read_json(path)
        if not isinstance(english, dict):
            raise ValueError(f"{path} must contain an object")
        artifact_id = english.get("id")
        name = required_text(english.get("name"), "English set name", path.stem)
        if not isinstance(artifact_id, int) or artifact_id <= 0:
            raise ValueError(f"{path.stem} has invalid Artifact ID")
        localized = {locale: localized_payload(source_root, locale, path.name) for locale in LOCALES.values()}
        match_name = normalized_name(name)
        if match_name in local_sets:
            raise ValueError(f"duplicate Artifact set name: {name}")
        local_sets[match_name] = (path.stem, english, localized)

    wiki_by_name: dict[str, dict[str, Any]] = {}
    entry_ids: set[str] = set()
    for entry in wiki_entries:
        if not isinstance(entry, dict):
            raise ValueError("HoYoWiki Artifact entry must be an object")
        entry_id = entry.get("entry_page_id")
        name = required_text(entry.get("name"), "English name", "HoYoWiki Artifact")
        if not isinstance(entry_id, str) or not entry_id.isdigit() or entry_id in entry_ids:
            raise ValueError(f"invalid or duplicate HoYoWiki Artifact ID: {entry_id!r}")
        match_name = normalized_name(name)
        if match_name in wiki_by_name:
            raise ValueError(f"duplicate HoYoWiki Artifact name: {name}")
        wiki_by_name[match_name] = {
            "entry_page_id": entry_id,
            "name": name,
            "icon_url": validate_image_url(entry.get("icon_url"), name),
            "rarities": wiki_rarities(entry),
            "effect_tags": wiki_effect_tags(entry),
            "display_field": entry.get("display_field") if isinstance(entry.get("display_field"), dict) else {},
        }
        entry_ids.add(entry_id)
    if set(local_sets) != set(wiki_by_name):
        missing = sorted(set(local_sets) - set(wiki_by_name))
        unknown = sorted(set(wiki_by_name) - set(local_sets))
        raise ValueError(f"Genshin Artifact catalog mismatch; missing={missing}, unknown={unknown}")

    artifact_sets = []
    artifact_pieces = []
    for match_name, (db_key, english, localized) in local_sets.items():
        wiki = wiki_by_name[match_name]
        artifact_id = english["id"]
        rarity_list = english.get("rarityList")
        if not isinstance(rarity_list, list) or not rarity_list or any(not isinstance(value, int) or value not in range(1, 6) for value in rarity_list):
            raise ValueError(f"{db_key} has invalid rarity list")
        rarity_list = sorted(set(rarity_list))
        if rarity_list != wiki["rarities"]:
            raise ValueError(f"HoYoWiki rarity mismatch for {english['name']}")
        set_source_key = f"genshin-data-artifact-set-{artifact_id}"
        set_record = {
            "source_key": set_source_key,
            "game_data_id": artifact_id,
            "genshin_db_key": db_key,
            "hoyowiki_entry_page_id": wiki["entry_page_id"],
            "official_english_name": english["name"],
            "rarity_list": rarity_list,
            "minimum_rarity": min(rarity_list),
            "maximum_rarity": max(rarity_list),
            "effect_1_piece": english.get("effect1Pc"),
            "effect_2_piece": english.get("effect2Pc"),
            "effect_4_piece": english.get("effect4Pc"),
            "effect_tags": wiki["effect_tags"],
            "artifact_piece_source_keys": [],
            "thumbnail_asset": f"{set_source_key}.png",
            "thumbnail_source_url": wiki["icon_url"],
        }
        for field, locale in LOCALES.items():
            set_record[field] = required_text(localized[locale].get("name"), f"{locale} set name", db_key)

        images = image_map.get(db_key)
        if not isinstance(images, dict):
            raise ValueError(f"{db_key} has no Artifact image index")
        for slot_key in SLOTS:
            piece = english.get(slot_key)
            localized_pieces = {locale: payload.get(slot_key) for locale, payload in localized.items()}
            if piece is None:
                if any(value is not None for value in localized_pieces.values()):
                    raise ValueError(f"{db_key}/{slot_key} has inconsistent locale coverage")
                continue
            if not isinstance(piece, dict) or any(not isinstance(value, dict) for value in localized_pieces.values()):
                raise ValueError(f"{db_key}/{slot_key} has invalid piece data")
            piece_source_key = f"genshin-data-artifact-piece-{artifact_id}-{slot_key}"
            piece_record = {
                "source_key": piece_source_key,
                "artifact_set_source_key": set_source_key,
                "artifact_set_english_name": english["name"],
                "game_data_set_id": artifact_id,
                "slot_key": slot_key,
                "slot": required_text(piece.get("relicText"), "slot", piece_source_key),
                "relic_type": required_text(piece.get("relicType"), "relic type", piece_source_key),
                "official_english_name": required_text(piece.get("name"), "English piece name", piece_source_key),
                "official_english_description": required_text(piece.get("description"), "English description", piece_source_key),
                "official_english_story": required_text(piece.get("story"), "English story", piece_source_key),
                "rarity_list": rarity_list,
                "minimum_rarity": min(rarity_list),
                "maximum_rarity": max(rarity_list),
                "thumbnail_asset": f"{piece_source_key}.png",
                "thumbnail_source_url": validate_image_url(
                    wiki["display_field"].get(HOYOWIKI_PIECE_ICON_FIELDS[slot_key])
                    or images.get(slot_key),
                    piece_source_key,
                ),
            }
            for field, locale in LOCALES.items():
                piece_record[field] = required_text(localized_pieces[locale].get("name"), f"{locale} piece name", piece_source_key)
            artifact_pieces.append(piece_record)
            set_record["artifact_piece_source_keys"].append(piece_source_key)
        artifact_sets.append(set_record)

    artifact_sets.sort(key=lambda record: (record["official_english_name"].casefold(), record["source_key"]))
    artifact_pieces.sort(key=lambda record: (record["artifact_set_english_name"].casefold(), SLOTS.index(record["slot_key"])))
    return {
        "schema": SCHEMA,
        "checked_at": checked_at,
        "sources": {
            "localized_game_data_and_piece_thumbnails": {"url": SOURCE_URL, "commit": source_commit},
            "set_identity_rarity_and_thumbnail": {
                "provider": "Official Genshin HoYoWiki Artifact archive",
                "url": HOYOWIKI_ARCHIVE_URL,
                "menu_id": HOYOWIKI_MENU_ID,
            },
        },
        "catalog_scope": {
            "artifact_sets": len(artifact_sets),
            "artifact_pieces": len(artifact_pieces),
            "piece_slots": list(SLOTS),
        },
        "artifact_sets": artifact_sets,
        "artifact_pieces": artifact_pieces,
    }


def write_snapshot(snapshot: dict[str, Any], output: Path, *, allow_shrink: bool) -> None:
    if output.exists() and not allow_shrink:
        existing = read_json(output)
        for field in ("artifact_sets", "artifact_pieces"):
            previous = existing.get("catalog_scope", {}).get(field)
            current = snapshot["catalog_scope"][field]
            if isinstance(previous, int) and current < previous:
                raise ValueError(f"refusing to shrink {field} from {previous} to {current}")
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{output.name}.", suffix=".tmp", dir=output.parent)
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
    parser.add_argument("--source-commit", default="8b15995fa220c88a4d0d7ffe1e21b041d0b32588")
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--wiki-response", type=Path)
    parser.add_argument("--timeout", type=float, default=60.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--allow-shrink", action="store_true")
    parser.add_argument("--output", type=Path, default=module_root / "data" / "artifacts.json")
    args = parser.parse_args()
    if args.wiki_response:
        response = read_json(args.wiki_response)
        data = response.get("data") if isinstance(response, dict) else None
        entries = data.get("list") if isinstance(data, dict) else None
        total = data.get("total") if isinstance(data, dict) else None
        try:
            total = int(total)
        except (TypeError, ValueError):
            total = None
        if not isinstance(entries, list) or total != len(entries):
            raise ValueError("--wiki-response has incomplete HoYoWiki Artifact pagination")
    else:
        entries = fetch_hoyowiki_entries(timeout=args.timeout, attempts=args.attempts)
    snapshot = build_snapshot(
        args.source_root.resolve(), entries, checked_at=args.checked_at, source_commit=args.source_commit
    )
    write_snapshot(snapshot, args.output.resolve(), allow_shrink=args.allow_shrink)
    print(
        f"wrote {len(snapshot['artifact_sets'])} Artifact Sets and "
        f"{len(snapshot['artifact_pieces'])} Artifact Pieces to {args.output}"
    )


if __name__ == "__main__":
    main()
