#!/usr/bin/env python3
"""Generate provenance-preserving Hán-Việt aliases for Star Rail Light Cones.

The generator reads only the reviewable Light Cone snapshot, submits its
Simplified Chinese names to the public Thi Viện Hán-Nôm transcription endpoint,
validates that every response round-trips to the submitted source text, and
writes a separate alias snapshot.  It never opens or modifies SQLite and never
changes the official catalog.
"""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import re
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode, urlparse
from urllib.request import Request, urlopen


CATALOG_SCHEMA = "perwiga.honkai-star-rail.light-cones.v1"
SNAPSHOT_SCHEMA = "perwiga.honkai-star-rail.light-cone-han-viet.v1"
EXPECTED_RECORDS = 169
STAR_RAIL_RES_REPOSITORY = "https://github.com/Mar-7th/StarRailRes"
STAR_RAIL_DATA_REPOSITORY = "https://github.com/Dimbreath/StarRailData"
STAR_RAIL_RES_COMMIT = "d226befe3db13f2ec15f4161d5f34b1b607643fe"
DICTIONARY_URL = "https://hvdic.thivien.net/transcript.php#trans"
DICTIONARY_ENDPOINT = "https://hvdic.thivien.net/transcript-query.json.php"
MAX_RESPONSE_BYTES = 2 * 1024 * 1024
QUOTE_MARKS = "「」『』《》〈〉"
SEPARATOR_MARKS = "·•—"
PUNCTUATION = "，。！？：；、（）〔〕【】,.!?;:()[]"


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def is_han_character(value: str) -> bool:
    return any(
        "\u3400" <= character <= "\u4dbf"
        or "\u4e00" <= character <= "\u9fff"
        or "\uf900" <= character <= "\ufaff"
        for character in value
    )


def validate_catalog(catalog: Any) -> list[dict[str, Any]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unsupported Star Rail Light Cone catalog schema")
    records = catalog.get("light_cones")
    count = catalog.get("catalog_scope", {}).get("included_records")
    if not isinstance(records, list) or count != len(records):
        raise ValueError("Light Cone catalog record count does not match")
    if len(records) != EXPECTED_RECORDS:
        raise ValueError(f"expected {EXPECTED_RECORDS} Light Cones, found {len(records)}")

    source_keys: set[str] = set()
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("Light Cone catalog entry must be an object")
        source_key = record.get("source_key")
        simplified = record.get("official_simplified_chinese_name")
        traditional = record.get("official_traditional_chinese_name")
        english = record.get("official_english_name")
        if not isinstance(source_key, str) or not source_key.startswith(
            "starrailres-light-cone-"
        ):
            raise ValueError("invalid Light Cone source key")
        if source_key in source_keys:
            raise ValueError(f"duplicate Light Cone source key {source_key}")
        source_keys.add(source_key)
        for field, value in (
            ("official_english_name", english),
            ("official_simplified_chinese_name", simplified),
            ("official_traditional_chinese_name", traditional),
        ):
            if not isinstance(value, str) or not value.strip():
                raise ValueError(f"missing {field} for {source_key}")
            if "\n" in value or "\r" in value:
                raise ValueError(f"multiline {field} for {source_key}")
    return records


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
            "User-Agent": "Perwiga HSR Light Cone Han-Viet curator/1.0",
        },
    )


def post_dictionary(text: str, *, timeout: float, attempts: int) -> dict[str, Any]:
    if urlparse(DICTIONARY_ENDPOINT).hostname != "hvdic.thivien.net":
        raise ValueError("untrusted Hán-Việt dictionary endpoint")
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
    raise RuntimeError(f"unable to fetch Hán-Việt readings after {attempts} attempts: {last_error}")


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
            "dictionary response did not round-trip every Light Cone name: "
            f"expected {len(names)} lines, got {len(reconstructed)}"
        )
    return lines


def title_reading(value: str) -> str:
    return value[:1].upper() + value[1:]


def format_han_viet(
    name: str, results: list[dict[str, Any]]
) -> tuple[str | None, list[str]]:
    tokens: list[tuple[str, bool]] = []
    unresolved: list[str] = []
    for result in results:
        source = result["i"]
        alternatives = result.get("o", [])
        if not isinstance(alternatives, list) or not all(
            isinstance(value, str) and value for value in alternatives
        ):
            alternatives = []
        if result.get("t") == 3 and not is_han_character(source):
            tokens.append((source, False))
        elif result.get("t") == 3:
            if alternatives:
                tokens.append((title_reading(alternatives[0]), True))
            else:
                unresolved.append(source)
                tokens.append((source, True))
        else:
            tokens.append((source, False))

    if "".join(result["i"] for result in results) != name:
        raise ValueError(f"dictionary result did not round-trip {name!r}")
    if unresolved:
        return None, list(dict.fromkeys(unresolved))

    output = ""
    previous_han = False
    for value, is_han in tokens:
        if value in QUOTE_MARKS:
            continue
        if is_han:
            if output and not output.endswith((" ", *SEPARATOR_MARKS)):
                output += " "
            output += value
        elif value in SEPARATOR_MARKS:
            output = output.rstrip() + f" {value} "
        elif value.isspace():
            if output and not output.endswith(" "):
                output += " "
        elif value in PUNCTUATION:
            output = output.rstrip() + value
        else:
            if previous_han and output and not output.endswith(" "):
                output += " "
            output += value
        previous_han = is_han
    return re.sub(r"\s+", " ", output).strip(), []


def fetch_result_lines(
    records: list[dict[str, Any]], *, batch_size: int, timeout: float, attempts: int
) -> list[list[dict[str, Any]]]:
    if batch_size < 1 or batch_size > 100:
        raise ValueError("dictionary batch size must be between 1 and 100")
    result_lines: list[list[dict[str, Any]]] = []
    for start in range(0, len(records), batch_size):
        batch = records[start : start + batch_size]
        names = [record["official_simplified_chinese_name"] for record in batch]
        response = post_dictionary(
            "\n".join(names), timeout=timeout, attempts=attempts
        )
        result_lines.extend(split_dictionary_response(names, response))
    if len(result_lines) != len(records):
        raise ValueError("dictionary batches did not cover every Light Cone")
    return result_lines


def snapshot_payload(
    catalog: Any, result_lines: list[list[dict[str, Any]]], *, checked_at: str
) -> dict[str, Any]:
    records = validate_catalog(catalog)
    if len(result_lines) != len(records):
        raise ValueError("dictionary result count does not match Light Cone catalog")

    aliases: list[dict[str, Any]] = []
    unresolved_records: list[dict[str, Any]] = []
    for record, results in zip(records, result_lines, strict=True):
        simplified = record["official_simplified_chinese_name"]
        han_viet_name, unresolved = format_han_viet(simplified, results)
        alias = {
            "source_key": record["source_key"],
            "game_data_id": record["game_data_id"],
            "official_english_name": record["official_english_name"],
            "official_simplified_chinese_name": simplified,
            "official_traditional_chinese_name": record[
                "official_traditional_chinese_name"
            ],
            "han_viet_name": han_viet_name,
            "confidence": "E",
        }
        if unresolved:
            alias["unresolved_tokens"] = unresolved
            unresolved_records.append(
                {"source_key": record["source_key"], "tokens": unresolved}
            )
        aliases.append(alias)

    source_keys = [record["source_key"] for record in records]
    alias_keys = [alias["source_key"] for alias in aliases]
    unresolved_tokens = sorted(
        {token for record in unresolved_records for token in record["tokens"]}
    )
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {
            "source_records": len(records),
            "alias_records": len(aliases),
            "unique_simplified_chinese_names": len(
                {record["official_simplified_chinese_name"] for record in records}
            ),
        },
        "coverage": {
            "source_key_coverage": {
                "source_records": len(source_keys),
                "alias_records": len(alias_keys),
                "matched": len(set(source_keys) & set(alias_keys)),
                "missing": sorted(set(source_keys) - set(alias_keys)),
                "extra": sorted(set(alias_keys) - set(source_keys)),
            },
            "generated_aliases": len(aliases) - len(unresolved_records),
            "unresolved_records": len(unresolved_records),
            "unresolved_token_occurrences": sum(
                len(record["tokens"]) for record in unresolved_records
            ),
            "unresolved_tokens": unresolved_tokens,
        },
        "confidence": {
            "han_viet_names": "E",
            "scale": {
                "E": "Generated dictionary reading suitable as a search alias; not an official localization and not a semantic translation.",
            },
        },
        "sources": {
            "official_simplified_chinese_names": {
                "repository": STAR_RAIL_RES_REPOSITORY,
                "source_path": "index_new/cn/light_cones.json",
                "commit": STAR_RAIL_RES_COMMIT,
                "upstream_data": STAR_RAIL_DATA_REPOSITORY,
            },
            "official_traditional_chinese_names": {
                "repository": STAR_RAIL_RES_REPOSITORY,
                "source_path": "index_new/cht/light_cones.json",
                "commit": STAR_RAIL_RES_COMMIT,
                "upstream_data": STAR_RAIL_DATA_REPOSITORY,
            },
            "han_viet_dictionary": DICTIONARY_URL,
            "han_viet_query_endpoint": DICTIONARY_ENDPOINT,
        },
        "generation": {
            "input_locale": "official_simplified_chinese_name",
            "selection": "First Hán-Việt alternative returned by the Thi Viện transcription endpoint for each Han character.",
            "capitalization": "Capitalize the first letter of each generated syllable.",
            "punctuation": "Preserve source punctuation, drop Chinese quote marks used only as title decoration, and separate generated syllables with spaces.",
            "reading_overrides": {},
            "explicit_fallbacks": {},
        },
        "limitations": [
            "Hán-Việt is a generated, unofficial reading alias; it never replaces official English, Chinese, or Vietnamese names.",
            "The dictionary returns character readings, not a context-reviewed translation of the Light Cone title; polyphonic characters therefore retain the endpoint's first alternative unless a future reviewed override is added.",
            "The Chinese locale values are preserved StarRailRes game-data extracts sourced from Dimbreath/StarRailData, not a direct official HoYoverse API response.",
            "Names that contain transliterated, invented, or newly coined terms may have a readable Hán-Việt form that is not an established Vietnamese name.",
        ],
        "notes": [
            "Every alias record is keyed to the source catalog's stable source_key and preserves both Chinese locale fields for review.",
            "Unresolved characters, if any, remain explicit in coverage and on the affected alias record; no unknown reading is fabricated.",
        ],
        "light_cones": aliases,
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


def main() -> None:
    module_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--catalog", type=Path, default=module_root / "data" / "light-cones.json"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=module_root / "data" / "light-cone-han-viet.json",
    )
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--batch-size", type=int, default=60)
    args = parser.parse_args()

    catalog = read_json(args.catalog)
    records = validate_catalog(catalog)
    result_lines = fetch_result_lines(
        records,
        batch_size=args.batch_size,
        timeout=args.timeout,
        attempts=args.attempts,
    )
    snapshot = snapshot_payload(catalog, result_lines, checked_at=args.checked_at)
    write_json_atomic(args.output, snapshot)
    coverage = snapshot["coverage"]
    print(
        f"Wrote {len(snapshot['light_cones'])} aliases; "
        f"unresolved records={coverage['unresolved_records']}, "
        f"unresolved tokens={coverage['unresolved_tokens']}"
    )


if __name__ == "__main__":
    main()
