#!/usr/bin/env python3
"""Crawl and generate provenance-preserving Hán-Việt aliases for HSR relics.

This script reads the reviewable relic-set and relic-piece snapshots, submits
their official Traditional Chinese names to the Thi Viện Hán-Nôm transcription
endpoint, validates response round-trips, and writes a separate alias
snapshot. It never edits the official catalogs or opens SQLite.
"""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode, urlparse
from urllib.request import Request, urlopen


MODULE_DIR = Path(__file__).resolve().parents[1]
SETS_DEFAULT = MODULE_DIR / "data" / "relic-sets.json"
PIECES_DEFAULT = MODULE_DIR / "data" / "relics.json"
OUTPUT_DEFAULT = MODULE_DIR / "data" / "relic-han-viet.json"

SETS_SCHEMA = "perwiga.honkai-star-rail.relic-sets.v1"
PIECES_SCHEMA = "perwiga.honkai-star-rail.relics.v1"
SNAPSHOT_SCHEMA = "perwiga.honkai-star-rail.relic-han-viet.v1"
EXPECTED_SETS = 60
EXPECTED_PIECES = 742

STAR_RAIL_RES_REPOSITORY = "https://github.com/Mar-7th/StarRailRes"
STAR_RAIL_DATA_REPOSITORY = "https://github.com/Dimbreath/StarRailData"
STAR_RAIL_RES_COMMIT = "d226befe3db13f2ec15f4161d5f34b1b607643fe"
DICTIONARY_URL = "https://hvdic.thivien.net/transcript.php#trans"
DICTIONARY_ENDPOINT = "https://hvdic.thivien.net/transcript-query.json.php"
MAX_RESPONSE_BYTES = 2 * 1024 * 1024
HAN_RE = re.compile(
    r"[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\U00020000-\U0002fa1f]"
)
OPENING_MARKS = "「『《〈【〔（([{"
CLOSING_MARKS = "」』》〉】〕）》)]}"
SEPARATOR_MARKS = "·•—/@|"
PUNCTUATION_MARKS = "，。！？：；、,.!?;:%"


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def is_han_character(value: str) -> bool:
    return HAN_RE.fullmatch(value) is not None


def validate_catalog(
    catalog: Any,
    *,
    record_key: str,
    schema: str,
    expected_count: int,
    label: str,
) -> list[dict[str, Any]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != schema:
        raise ValueError(f"unsupported {label} catalog schema")
    records = catalog.get(record_key)
    declared_count = catalog.get("catalog_scope", {}).get("included_records")
    if not isinstance(records, list) or declared_count != len(records):
        raise ValueError(f"{label} catalog record count does not match")
    if len(records) != expected_count:
        raise ValueError(f"expected {expected_count} {label}, found {len(records)}")

    source_keys: set[str] = set()
    game_data_ids: set[str] = set()
    for record in records:
        if not isinstance(record, dict):
            raise ValueError(f"{label} catalog entry must be an object")
        source_key = record.get("source_key")
        game_data_id = record.get("game_data_id")
        if not isinstance(source_key, str) or not source_key:
            raise ValueError(f"invalid {label} source key")
        if source_key in source_keys:
            raise ValueError(f"duplicate {label} source key {source_key}")
        source_keys.add(source_key)
        if not isinstance(game_data_id, str) or not game_data_id:
            raise ValueError(f"missing game_data_id for {source_key}")
        if game_data_id in game_data_ids:
            raise ValueError(f"duplicate {label} game_data_id {game_data_id}")
        game_data_ids.add(game_data_id)

        for field in (
            "official_english_name",
            "official_simplified_chinese_name",
            "official_traditional_chinese_name",
            "official_vietnamese_name",
        ):
            value = record.get(field)
            if not isinstance(value, str) or not value.strip():
                raise ValueError(f"missing {field} for {source_key}")
            if "\n" in value or "\r" in value:
                raise ValueError(f"multiline {field} for {source_key}")
    return records


def validate_parent_links(
    sets: list[dict[str, Any]], pieces: list[dict[str, Any]]
) -> None:
    parent_ids = {
        record["source_key"]: record["game_data_id"] for record in sets
    }
    for record in pieces:
        parent_key = record.get("relic_set_source_key")
        parent_id = record.get("relic_set_game_data_id")
        if parent_key not in parent_ids:
            raise ValueError(
                f"piece {record['source_key']} has unknown parent set {parent_key}"
            )
        if parent_id != parent_ids[parent_key]:
            raise ValueError(
                f"piece {record['source_key']} has mismatched parent-set ID"
            )


def dictionary_request(text: str) -> Request:
    body = urlencode(
        {"mode": "trans", "lang": "1", "capitalize": "1", "input": text}
    ).encode("utf-8")
    return Request(
        DICTIONARY_ENDPOINT,
        data=body,
        method="POST",
        headers={
            "Accept": "application/json",
            "Content-Type": "application/x-www-form-urlencoded",
            "Referer": "https://hvdic.thivien.net/transcript.php",
            "User-Agent": "Perwiga HSR Relic Han-Viet curator/1.0",
        },
    )


def post_dictionary(text: str, *, timeout: float, attempts: int) -> dict[str, Any]:
    if urlparse(DICTIONARY_ENDPOINT).hostname != "hvdic.thivien.net":
        raise ValueError("untrusted Hán-Việt dictionary endpoint")
    if attempts < 1:
        raise ValueError("dictionary attempts must be positive")

    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            with urlopen(dictionary_request(text), timeout=timeout) as response:
                content_length = response.headers.get("Content-Length")
                if content_length is not None and int(content_length) > MAX_RESPONSE_BYTES:
                    raise ValueError("dictionary response exceeds the size limit")
                body = response.read(MAX_RESPONSE_BYTES + 1)
                if len(body) > MAX_RESPONSE_BYTES:
                    raise ValueError("dictionary response exceeds the size limit")
            payload = json.loads(body.decode("utf-8"))
            if not isinstance(payload, dict):
                raise ValueError("dictionary response must be a JSON object")
            return payload
        except (HTTPError, URLError, TimeoutError, json.JSONDecodeError, ValueError) as error:
            last_error = error
            if attempt < attempts:
                time.sleep(min(attempt * 2, 6))
    raise RuntimeError(
        f"unable to fetch Hán-Việt readings after {attempts} attempts: {last_error}"
    )


def split_dictionary_response(
    names: list[str], response: dict[str, Any]
) -> list[list[dict[str, Any]]]:
    if (
        response.get("message") != "OK"
        or response.get("mode") != "trans"
        or str(response.get("lang")) != "1"
        or not isinstance(response.get("result"), list)
    ):
        raise ValueError("dictionary response is not a successful Hán-Việt result")

    lines: list[list[dict[str, Any]]] = [[]]
    for result in response["result"]:
        if not isinstance(result, dict) or not isinstance(result.get("i"), str):
            raise ValueError("dictionary result has an invalid shape")
        source = result["i"]
        if source in {"\r", "\n", "\r\n"}:
            if source != "\r":
                lines.append([])
        else:
            lines[-1].append(result)
    while lines and not lines[-1]:
        lines.pop()

    reconstructed = ["".join(result["i"] for result in line) for line in lines]
    if reconstructed != names:
        raise ValueError(
            "dictionary response did not round-trip every relic name: "
            f"expected {len(names)} lines, got {len(reconstructed)}"
        )
    return lines


def title_reading(value: str) -> str:
    return value[:1].upper() + value[1:]


def format_han_viet(
    name: str, results: list[dict[str, Any]]
) -> tuple[str | None, list[dict[str, Any]], list[str]]:
    tokens: list[tuple[str, bool]] = []
    readings: list[dict[str, Any]] = []
    unresolved: list[str] = []

    for result in results:
        source = result["i"]
        alternatives = result.get("o", [])
        if not isinstance(alternatives, list) or not all(
            isinstance(value, str) and value for value in alternatives
        ):
            alternatives = []

        if result.get("t") == 3 and is_han_character(source):
            selected = alternatives[0] if alternatives else None
            readings.append(
                {
                    "source": source,
                    "selected": selected,
                    "alternatives": alternatives,
                    "explicit_fallback": False,
                }
            )
            if selected is None:
                unresolved.append(source)
                tokens.append((source, True))
            else:
                tokens.append((title_reading(selected), True))
        else:
            tokens.append((source, False))

    if "".join(result["i"] for result in results) != name:
        raise ValueError(f"dictionary result did not round-trip {name!r}")
    if unresolved:
        return None, readings, list(dict.fromkeys(unresolved))

    output = ""
    previous_han = False
    for value, is_han in tokens:
        if is_han:
            if output and not output.endswith((" ", *OPENING_MARKS, *SEPARATOR_MARKS)):
                output += " "
            output += value
        elif value in OPENING_MARKS:
            output = output.rstrip() + value
        elif value in CLOSING_MARKS or value in PUNCTUATION_MARKS:
            output = output.rstrip() + value
        elif value in SEPARATOR_MARKS:
            output = output.rstrip() + value
        elif value.isspace():
            if output and not output.endswith(" "):
                output += " "
        else:
            if previous_han and output and not output.endswith(" "):
                output += " "
            output += value
        previous_han = is_han
    return re.sub(r"\s+", " ", output).strip(), readings, []


def fetch_result_lines(
    records: list[dict[str, Any]],
    *,
    batch_size: int,
    timeout: float,
    attempts: int,
) -> list[list[dict[str, Any]]]:
    if not 1 <= batch_size <= 100:
        raise ValueError("dictionary batch size must be between 1 and 100")
    result_lines: list[list[dict[str, Any]]] = []
    for start in range(0, len(records), batch_size):
        batch = records[start : start + batch_size]
        names = [record["official_traditional_chinese_name"] for record in batch]
        response = post_dictionary(
            "\n".join(names), timeout=timeout, attempts=attempts
        )
        result_lines.extend(split_dictionary_response(names, response))
        print(
            f"Fetched dictionary batch {start + 1}-{start + len(batch)} "
            f"of {len(records)}",
            file=sys.stderr,
        )
    if len(result_lines) != len(records):
        raise ValueError("dictionary batches did not cover every relic")
    return result_lines


def unique_han_characters(records: list[dict[str, Any]]) -> list[str]:
    return sorted(
        {
            character
            for record in records
            for character in record["official_traditional_chinese_name"]
            if is_han_character(character)
        }
    )


def make_alias_record(
    record: dict[str, Any], results: list[dict[str, Any]]
) -> dict[str, Any]:
    alias, readings, unresolved = format_han_viet(
        record["official_traditional_chinese_name"], results
    )
    generated_fields = {
        "generated_from",
        "han_viet_name",
        "confidence",
        "generated_aliases",
        "unresolved_tokens",
    }
    if generated_fields & record.keys():
        raise ValueError(
            f"source record already contains generated fields: {record['source_key']}"
        )

    alias_record = dict(record)
    alias_record.update(
        {
            "generated_from": "official_traditional_chinese_name",
            "han_viet_name": alias,
            "confidence": "E",
            "generated_aliases": [
                {
                    "value": alias,
                    "language": "vi",
                    "kind": "han-viet",
                    "official": False,
                    "confidence": "E",
                    "source_text": record["official_traditional_chinese_name"],
                    "character_readings": readings,
                    "unresolved_tokens": unresolved,
                    "dictionary_lookup_urls": [
                        f"https://hvdic.thivien.net/whv/{quote(character)}"
                        for character in unique_han_characters([record])
                    ],
                    "limitations": [
                        "Generated alias only; not an official HoYoverse Vietnamese localization.",
                        "The Thi Viện endpoint returns character readings; the first alternative is selected deterministically and may not be the preferred context-specific reading.",
                        "Character-level readings do not guarantee an idiomatic Vietnamese compound or an established reading for invented, transliterated, or proper-noun terms.",
                        "The alias is derived from official_traditional_chinese_name; official English, Simplified Chinese, Traditional Chinese, and Vietnamese fields remain unchanged.",
                    ],
                }
            ],
            "unresolved_tokens": unresolved,
        }
    )
    if unresolved:
        alias_record["generated_aliases"][0]["limitations"].append(
            "Unresolved Han characters are reported and no alias value is fabricated."
        )
    return alias_record


def source_descriptor(
    catalog: dict[str, Any], *, source_path: str, locale: str
) -> dict[str, Any]:
    return {
        "repository": STAR_RAIL_RES_REPOSITORY,
        "source_path": source_path,
        "commit": STAR_RAIL_RES_COMMIT,
        "raw_snapshot": f"https://raw.githubusercontent.com/Mar-7th/StarRailRes/{STAR_RAIL_RES_COMMIT}/{source_path}",
        "locale": locale,
        "local_catalog_source": catalog.get("catalog_source"),
        "local_catalog_checked_at": catalog.get("catalog_checked_at"),
        "upstream_data": STAR_RAIL_DATA_REPOSITORY,
    }


def coverage_for(
    source_records: list[dict[str, Any]], alias_records: list[dict[str, Any]]
) -> dict[str, Any]:
    source_keys = [record["source_key"] for record in source_records]
    alias_keys = [record["source_key"] for record in alias_records]
    source_set = set(source_keys)
    alias_set = set(alias_keys)
    return {
        "source_records": len(source_keys),
        "alias_records": len(alias_keys),
        "matched": len(source_set & alias_set),
        "missing": sorted(source_set - alias_set),
        "extra": sorted(alias_set - source_set),
    }


def make_snapshot(
    sets_catalog: dict[str, Any],
    pieces_catalog: dict[str, Any],
    set_lines: list[list[dict[str, Any]]],
    piece_lines: list[list[dict[str, Any]]],
    *,
    checked_at: str,
) -> dict[str, Any]:
    sets = validate_catalog(
        sets_catalog,
        record_key="relic_sets",
        schema=SETS_SCHEMA,
        expected_count=EXPECTED_SETS,
        label="relic-set",
    )
    pieces = validate_catalog(
        pieces_catalog,
        record_key="relics",
        schema=PIECES_SCHEMA,
        expected_count=EXPECTED_PIECES,
        label="relic-piece",
    )
    validate_parent_links(sets, pieces)
    if len(set_lines) != len(sets) or len(piece_lines) != len(pieces):
        raise ValueError("dictionary result count does not match relic catalogs")

    set_aliases = [
        make_alias_record(record, results)
        for record, results in zip(sets, set_lines, strict=True)
    ]
    piece_aliases = [
        make_alias_record(record, results)
        for record, results in zip(pieces, piece_lines, strict=True)
    ]
    all_source_records = sets + pieces
    all_alias_records = set_aliases + piece_aliases
    unresolved_records = [
        record
        for record in all_alias_records
        if record["unresolved_tokens"]
    ]
    unresolved_tokens = sorted(
        {
            token
            for record in unresolved_records
            for token in record["unresolved_tokens"]
        }
    )
    all_names = {
        record["official_traditional_chinese_name"] for record in all_source_records
    }

    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {
            "source_records": len(all_source_records),
            "alias_records": len(all_alias_records),
            "source_arrays": {
                "relic_sets": {
                    "source_records": len(sets),
                    "alias_records": len(set_aliases),
                },
                "relics": {
                    "source_records": len(pieces),
                    "alias_records": len(piece_aliases),
                },
            },
            "unique_traditional_chinese_names": len(all_names),
            "unique_traditional_han_characters": len(
                unique_han_characters(all_source_records)
            ),
        },
        "coverage": {
            "source_key_coverage": {
                "relic_sets": coverage_for(sets, set_aliases),
                "relics": coverage_for(pieces, piece_aliases),
            },
            "generated_aliases": len(all_alias_records) - len(unresolved_records),
            "unresolved_records": len(unresolved_records),
            "unresolved_token_occurrences": sum(
                len(record["unresolved_tokens"]) for record in unresolved_records
            ),
            "unresolved_tokens": unresolved_tokens,
        },
        "confidence": {
            "han_viet_names": "E",
            "scale": {
                "E": "Generated dictionary reading suitable as a search alias; not an official localization and not a semantic translation."
            },
        },
        "sources": {
            "official_simplified_chinese_names": {
                "relic_sets": source_descriptor(
                    sets_catalog,
                    source_path="index_new/cn/relic_sets.json",
                    locale="cn",
                ),
                "relics": source_descriptor(
                    pieces_catalog,
                    source_path="index_new/cn/relics.json",
                    locale="cn",
                ),
            },
            "official_traditional_chinese_names": {
                "relic_sets": source_descriptor(
                    sets_catalog,
                    source_path="index_new/cht/relic_sets.json",
                    locale="cht",
                ),
                "relics": source_descriptor(
                    pieces_catalog,
                    source_path="index_new/cht/relics.json",
                    locale="cht",
                ),
            },
            "han_viet_dictionary": DICTIONARY_URL,
            "han_viet_query_endpoint": DICTIONARY_ENDPOINT,
        },
        "generation": {
            "input_locale": "official_traditional_chinese_name",
            "selection": "First Hán-Việt alternative returned by the Thi Viện transcription endpoint for each Han character.",
            "capitalization": "Capitalize the first letter of each generated syllable.",
            "punctuation": "Preserve source punctuation and ASCII symbols; separate generated syllables with spaces.",
            "reading_overrides": {},
            "explicit_fallbacks": {},
            "dictionary_request_records": len(all_source_records),
            "dictionary_batch_size": 60,
            "official_fields_unchanged": True,
        },
        "limitations": [
            "Hán-Việt values are generated, unofficial reading aliases; they never replace official English, Chinese, or Vietnamese names.",
            "The dictionary returns character readings rather than a context-reviewed title translation; polyphonic characters retain the endpoint's first alternative.",
            "Character-level readings do not guarantee an idiomatic Vietnamese compound or an established reading for invented, transliterated, or proper-noun terms.",
            "The StarRailRes source is a community-maintained game-data extract, not an official HoYoverse API.",
        ],
        "notes": [
            "Every output record copies its source catalog record and adds generated alias metadata; source keys and relic_set_source_key parent links are preserved exactly.",
            "Unresolved characters, if any, remain explicit in coverage and on the affected alias record; no unknown reading is fabricated.",
        ],
        "unresolved_tokens": unresolved_tokens,
        "relic_sets": set_aliases,
        "relics": piece_aliases,
    }


def write_json_atomic(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(payload, handle, ensure_ascii=False, indent=2)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_name, path)
    except Exception:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sets", type=Path, default=SETS_DEFAULT)
    parser.add_argument("--pieces", type=Path, default=PIECES_DEFAULT)
    parser.add_argument("--output", type=Path, default=OUTPUT_DEFAULT)
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--batch-size", type=int, default=60)
    args = parser.parse_args()

    try:
        sets_catalog = read_json(args.sets)
        pieces_catalog = read_json(args.pieces)
        sets = validate_catalog(
            sets_catalog,
            record_key="relic_sets",
            schema=SETS_SCHEMA,
            expected_count=EXPECTED_SETS,
            label="relic-set",
        )
        pieces = validate_catalog(
            pieces_catalog,
            record_key="relics",
            schema=PIECES_SCHEMA,
            expected_count=EXPECTED_PIECES,
            label="relic-piece",
        )
        validate_parent_links(sets, pieces)
        all_records = sets + pieces
        split_at = len(sets)
        result_lines = fetch_result_lines(
            all_records,
            batch_size=args.batch_size,
            timeout=args.timeout,
            attempts=args.attempts,
        )
        snapshot = make_snapshot(
            sets_catalog,
            pieces_catalog,
            result_lines[:split_at],
            result_lines[split_at:],
            checked_at=args.checked_at,
        )
        write_json_atomic(args.output, snapshot)
        coverage = snapshot["coverage"]
        print(
            f"Wrote {len(snapshot['relic_sets'])} set aliases and "
            f"{len(snapshot['relics'])} piece aliases; "
            f"unresolved records={coverage['unresolved_records']}, "
            f"unresolved tokens={coverage['unresolved_tokens']}"
        )
        return 0
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
