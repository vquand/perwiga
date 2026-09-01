#!/usr/bin/env python3
"""Crawl the official HoYoWiki NPC index and generate Hán-Việt aliases.

The catalog boundary is the Simplified Chinese NPC index. English names are
joined by the official HoYoWiki ``entry_page_id`` and are never inferred from
translation or display text. Hán-Việt values are generated from the captured
Simplified Chinese names with HVDic and remain separate from official fields.

This script never opens SQLite. It writes only ``data/npcs.json`` and
``data/npc-han-viet.json`` through atomic replacements.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import os
from pathlib import Path
import re
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode, urlparse
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parents[1]
DATA_DIR = ROOT / "data"
NPC_OUTPUT = DATA_DIR / "npcs.json"
HAN_VIET_OUTPUT = DATA_DIR / "npc-han-viet.json"

API_URL = "https://sg-act-public-api.hoyolab.com/hoyowiki/hsr/wapi/get_entry_page_list"
INDEX_URL = "https://wiki.hoyolab.com/pc/hsr/aggregate/npc?lang=zh-cn"
ENGLISH_INDEX_URL = "https://wiki.hoyolab.com/pc/hsr/aggregate/npc?lang=en-us"
ENTRY_URL_TEMPLATE = "https://wiki.hoyolab.com/pc/hsr/entry/{entry_page_id}?lang=zh-cn"
DICTIONARY_URL = "https://hvdic.thivien.net/transcript.php#trans"
DICTIONARY_ENDPOINT = "https://hvdic.thivien.net/transcript-query.json.php"

API_HOST = "sg-act-public-api.hoyolab.com"
DICTIONARY_HOST = "hvdic.thivien.net"
MENU_ID = 105
PAGE_SIZE = 30
DICTIONARY_BATCH_SIZE = 80
MAX_API_RESPONSE_BYTES = 4 * 1024 * 1024
MAX_DICTIONARY_RESPONSE_BYTES = 2 * 1024 * 1024
CATALOG_SCHEMA = "perwiga.honkai-star-rail.npcs.v1"
HAN_VIET_SCHEMA = "perwiga.honkai-star-rail.npc-han-viet.v1"
HAN_RE = re.compile(r"[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]")
PLACEHOLDER_RE = re.compile(r"\{[^{}]+\}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-at", required=True, help="Snapshot date in YYYY-MM-DD")
    parser.add_argument("--output-dir", type=Path, default=DATA_DIR)
    parser.add_argument("--delay", type=float, default=0.15, help="Seconds between page requests")
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=4)
    parser.add_argument(
        "--allow-shrink",
        action="store_true",
        help="Allow the new source catalog to contain fewer entries than an existing snapshot",
    )
    return parser.parse_args()


def validate_date(value: str) -> str:
    try:
        parsed = dt.date.fromisoformat(value)
    except ValueError as error:
        raise ValueError(f"invalid --checked-at date: {value}") from error
    return parsed.isoformat()


def bounded_json_request(
    url: str,
    *,
    body: bytes | None,
    headers: dict[str, str],
    timeout: float,
    attempts: int,
    max_bytes: int,
) -> dict[str, Any]:
    parsed = urlparse(url)
    if parsed.scheme != "https" or not parsed.hostname:
        raise ValueError(f"untrusted HTTPS endpoint: {url}")

    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = Request(url, data=body, method="POST" if body is not None else "GET", headers=headers)
        try:
            with urlopen(request, timeout=timeout) as response:
                content_length = response.headers.get("Content-Length")
                if content_length is not None and int(content_length) > max_bytes:
                    raise ValueError(f"response exceeds {max_bytes} bytes: {url}")
                raw = response.read(max_bytes + 1)
            if len(raw) > max_bytes:
                raise ValueError(f"response exceeds {max_bytes} bytes: {url}")
            payload = json.loads(raw.decode("utf-8"))
            if not isinstance(payload, dict):
                raise ValueError(f"response must be a JSON object: {url}")
            return payload
        except HTTPError as error:
            last_error = error
            if 400 <= error.code < 500:
                break
        except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
            last_error = error
        if attempt < attempts:
            time.sleep(min(attempt * 2, 6))

    raise RuntimeError(f"unable to fetch {url} after {attempts} attempts: {last_error}")


def validate_page_record(record: Any, *, locale: str) -> dict[str, Any]:
    if not isinstance(record, dict):
        raise ValueError(f"{locale} NPC list contains a non-object record")
    entry_page_id = record.get("entry_page_id")
    name = record.get("name")
    if not isinstance(entry_page_id, str) or not entry_page_id.isdigit():
        raise ValueError(f"{locale} NPC list contains an invalid entry_page_id")
    if not isinstance(name, str) or not name.strip():
        raise ValueError(f"{locale} NPC {entry_page_id} has no non-empty name")
    return {
        "entry_page_id": entry_page_id,
        "name": name.strip(),
    }


def fetch_page(
    locale: str,
    page_num: int,
    *,
    timeout: float,
    attempts: int,
) -> tuple[list[dict[str, str]], int]:
    body = json.dumps(
        {
            "filters": [],
            "menu_id": MENU_ID,
            "page_num": page_num,
            "page_size": PAGE_SIZE,
            "use_es": True,
        }
    ).encode("utf-8")
    payload = bounded_json_request(
        API_URL,
        body=body,
        headers={
            "Accept": "application/json, text/plain, */*",
            "Content-Type": "application/json",
            "Origin": "https://wiki.hoyolab.com",
            "Referer": INDEX_URL,
            "User-Agent": "Perwiga-HSR-NPC-Crawler/1.0",
            "x-rpc-client_type": "4",
            "x-rpc-language": locale,
            "x-rpc-wiki_app": "hsr",
        },
        timeout=timeout,
        attempts=attempts,
        max_bytes=MAX_API_RESPONSE_BYTES,
    )
    if payload.get("retcode") != 0 or payload.get("message") != "OK":
        raise ValueError(f"HoYoWiki returned an unsuccessful response for {locale} page {page_num}")
    data = payload.get("data")
    if not isinstance(data, dict) or not isinstance(data.get("list"), list):
        raise ValueError(f"HoYoWiki response has an invalid list for {locale} page {page_num}")
    try:
        total = int(data.get("total"))
    except (TypeError, ValueError) as error:
        raise ValueError(f"HoYoWiki response has an invalid total for {locale}") from error
    if total <= 0:
        raise ValueError(f"HoYoWiki returned an empty {locale} NPC catalog")
    return [validate_page_record(record, locale=locale) for record in data["list"]], total


def crawl_locale(
    locale: str,
    *,
    delay: float,
    timeout: float,
    attempts: int,
) -> list[dict[str, str]]:
    records, total = fetch_page(locale, 1, timeout=timeout, attempts=attempts)
    page_count = math.ceil(total / PAGE_SIZE)
    for page_num in range(2, page_count + 1):
        if delay > 0:
            time.sleep(delay)
        page_records, page_total = fetch_page(locale, page_num, timeout=timeout, attempts=attempts)
        if page_total != total:
            raise RuntimeError(
                f"HoYoWiki {locale} catalog total changed during crawl: {total} -> {page_total}"
            )
        records.extend(page_records)

    if len(records) != total:
        raise RuntimeError(f"HoYoWiki {locale} catalog expected {total} records, received {len(records)}")
    ids = [record["entry_page_id"] for record in records]
    if len(ids) != len(set(ids)):
        raise RuntimeError(f"HoYoWiki {locale} catalog contains duplicate entry_page_id values")
    return records


def source_metadata(checked_at: str, english_count: int, chinese_count: int) -> dict[str, Any]:
    return {
        "provider": "HoYoWiki",
        "hosted_by": "HoYoverse HoYoLAB",
        "api_url": API_URL,
        "index_url": INDEX_URL,
        "english_index_url": ENGLISH_INDEX_URL,
        "entry_url_template": ENTRY_URL_TEMPLATE,
        "game_route": "hsr",
        "npc_menu_id": str(MENU_ID),
        "snapshot_locales": ["en-us", "zh-cn"],
        "english_index_records": english_count,
        "simplified_chinese_index_records": chinese_count,
        "retrieved_at": checked_at,
        "identity_join": "HoYoWiki entry_page_id",
    }


def build_catalog(
    english_records: list[dict[str, str]],
    chinese_records: list[dict[str, str]],
    *,
    checked_at: str,
) -> dict[str, Any]:
    english_by_id = {record["entry_page_id"]: record for record in english_records}
    chinese_ids = {record["entry_page_id"] for record in chinese_records}
    english_only = sorted(set(english_by_id) - chinese_ids, key=int)
    records: list[dict[str, Any]] = []
    english_name_gaps: list[str] = []
    for chinese in chinese_records:
        entry_page_id = chinese["entry_page_id"]
        english = english_by_id.get(entry_page_id)
        english_name = english["name"] if english else None
        source_key = f"hoyowiki-npc-{entry_page_id}"
        if english_name is None:
            english_name_gaps.append(source_key)
        records.append(
            {
                "source_key": source_key,
                "hoyowiki_entry_page_id": entry_page_id,
                "page_url": ENTRY_URL_TEMPLATE.format(entry_page_id=entry_page_id),
                "official_english_name": english_name,
                "official_simplified_chinese_name": chinese["name"],
            }
        )

    return {
        "schema": CATALOG_SCHEMA,
        "checked_at": checked_at,
        "source": source_metadata(checked_at, len(english_records), len(chinese_records)),
        "catalog_scope": {
            "included_records": len(records),
            "source_locale": "zh-cn",
            "source_index_records": len(chinese_records),
            "english_joined_records": len(records) - len(english_name_gaps),
            "english_name_gaps": english_name_gaps,
            "english_only_records": len(english_only),
            "english_only_entry_page_ids": english_only,
            "stable_identity_field": "hoyowiki_entry_page_id",
        },
        "notes": [
            "The catalog scope is the official HoYoWiki Simplified Chinese NPC index, joined to the English index by entry_page_id.",
            "Names are copied from HoYoWiki locale responses; no names are fabricated from page titles, transliteration, or inference.",
            "The English and Simplified Chinese index totals are retained because the source currently exposes different locale counts.",
            "No Traditional Chinese field is included: the crawlable zh-tw NPC index is materially incomplete compared with zh-cn.",
        ],
        "npcs": records,
    }


def dictionary_request(text: str) -> Request:
    if urlparse(DICTIONARY_ENDPOINT).hostname != DICTIONARY_HOST:
        raise ValueError("untrusted Hán-Việt dictionary endpoint")
    body = urlencode({"mode": "trans", "lang": "1", "capitalize": "1", "input": text}).encode("utf-8")
    return Request(
        DICTIONARY_ENDPOINT,
        data=body,
        method="POST",
        headers={
            "Accept": "application/json",
            "Content-Type": "application/x-www-form-urlencoded",
            "Referer": DICTIONARY_URL.split("#", 1)[0],
            "User-Agent": "Perwiga-HSR-NPC-Crawler/1.0",
        },
    )


def fetch_dictionary(
    text: str,
    *,
    timeout: float,
    attempts: int,
) -> dict[str, Any]:
    request = dictionary_request(text)
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            with urlopen(request, timeout=timeout) as response:
                content_length = response.headers.get("Content-Length")
                if content_length is not None and int(content_length) > MAX_DICTIONARY_RESPONSE_BYTES:
                    raise ValueError("dictionary response exceeds the size limit")
                raw = response.read(MAX_DICTIONARY_RESPONSE_BYTES + 1)
            if len(raw) > MAX_DICTIONARY_RESPONSE_BYTES:
                raise ValueError("dictionary response exceeds the size limit")
            payload = json.loads(raw.decode("utf-8"))
            if not isinstance(payload, dict):
                raise ValueError("dictionary response must be a JSON object")
            return payload
        except HTTPError as error:
            last_error = error
            if 400 <= error.code < 500:
                break
        except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
            last_error = error
        if attempt < attempts:
            time.sleep(min(attempt * 2, 6))
    raise RuntimeError(f"unable to fetch Hán-Việt readings after {attempts} attempts: {last_error}")


def split_dictionary_response(names: list[str], response: dict[str, Any]) -> list[list[dict[str, Any]]]:
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
            if lines[-1]:
                lines.append([])
        else:
            lines[-1].append(result)
    while lines and not lines[-1]:
        lines.pop()
    reconstructed = ["".join(token["i"] for token in line) for line in lines]
    if reconstructed != names:
        raise ValueError(
            f"dictionary response did not round-trip every queried name: expected {len(names)} lines, got {len(reconstructed)}"
        )
    return lines


def title_reading(value: str) -> str:
    return value[:1].upper() + value[1:]


def format_han_viet(name: str, results: list[dict[str, Any]]) -> tuple[str | None, list[dict[str, Any]], list[str]]:
    if "".join(result["i"] for result in results) != name:
        raise ValueError(f"dictionary result did not round-trip {name!r}")

    tokens: list[tuple[str, bool]] = []
    trace: list[dict[str, Any]] = []
    unresolved: list[str] = []
    for result in results:
        source = result["i"]
        alternatives = result.get("o", [])
        if not isinstance(alternatives, list) or not all(isinstance(value, str) and value for value in alternatives):
            alternatives = []
        is_han = result.get("t") == 3 and HAN_RE.fullmatch(source) is not None
        selected = alternatives[0] if alternatives else None
        if is_han:
            trace.append({"source": source, "selected": selected, "alternatives": alternatives})
            if selected is None:
                unresolved.append(source)
                tokens.append((source, False))
            else:
                tokens.append((title_reading(selected), True))
        else:
            tokens.append((source, False))

    if unresolved:
        return None, trace, sorted(set(unresolved))

    output = ""
    previous_han = False
    for value, is_han in tokens:
        if is_han:
            if output and not output.endswith((" ", "·", "•", "—", "-", "&", "/")):
                output += " "
            output += value
        elif value in {"·", "•", "—"}:
            output = output.rstrip() + f" {value} "
        elif value in {"&", "/"}:
            output = output.rstrip() + f" {value} "
        elif value.isspace():
            if output and not output.endswith(" "):
                output += " "
        else:
            if previous_han and output and not output.endswith(" "):
                output += " "
            output += value
        previous_han = is_han
    return re.sub(r"\s+", " ", output).strip(), trace, []


def validate_catalog(catalog: dict[str, Any]) -> list[dict[str, Any]]:
    if catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unexpected HSR NPC catalog schema")
    records = catalog.get("npcs")
    if not isinstance(records, list) or catalog.get("catalog_scope", {}).get("included_records") != len(records):
        raise ValueError("HSR NPC catalog count does not match metadata")
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("HSR NPC catalog record must be an object")
        source_key = record.get("source_key")
        entry_page_id = record.get("hoyowiki_entry_page_id")
        chinese_name = record.get("official_simplified_chinese_name")
        if (
            not isinstance(source_key, str)
            or not isinstance(entry_page_id, str)
            or source_key != f"hoyowiki-npc-{entry_page_id}"
            or not entry_page_id.isdigit()
            or not isinstance(chinese_name, str)
            or not chinese_name.strip()
        ):
            raise ValueError(f"invalid HSR NPC identity or Chinese name: {source_key}")
    return records


def base_han_viet_record(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "source_key": record["source_key"],
        "hoyowiki_entry_page_id": record["hoyowiki_entry_page_id"],
        "official_english_name": record.get("official_english_name"),
        "official_simplified_chinese_name": record["official_simplified_chinese_name"],
        "generated_from": "official_simplified_chinese_name",
    }


def build_han_viet_snapshot(
    catalog: dict[str, Any],
    *,
    checked_at: str,
    timeout: float,
    attempts: int,
    delay: float,
) -> dict[str, Any]:
    records = validate_catalog(catalog)
    queryable = [
        record
        for record in records
        if HAN_RE.search(record["official_simplified_chinese_name"])
        and not PLACEHOLDER_RE.search(record["official_simplified_chinese_name"])
    ]
    dictionary_results: dict[str, list[dict[str, Any]]] = {}
    for offset in range(0, len(queryable), DICTIONARY_BATCH_SIZE):
        batch = queryable[offset : offset + DICTIONARY_BATCH_SIZE]
        if offset and delay > 0:
            time.sleep(delay)
        response = fetch_dictionary(
            "\n".join(record["official_simplified_chinese_name"] for record in batch),
            timeout=timeout,
            attempts=attempts,
        )
        lines = split_dictionary_response(
            [record["official_simplified_chinese_name"] for record in batch], response
        )
        dictionary_results.update(
            {
                record["source_key"]: line
                for record, line in zip(batch, lines, strict=True)
            }
        )

    aliases: list[dict[str, Any]] = []
    status_counts = {"generated": 0, "unresolved": 0, "source_literal": 0, "placeholder": 0}
    unresolved_by_source: dict[str, list[str]] = {}
    for record in records:
        alias = base_han_viet_record(record)
        chinese_name = record["official_simplified_chinese_name"]
        placeholders = PLACEHOLDER_RE.findall(chinese_name)
        if placeholders:
            alias.update(
                {
                    "han_viet_name": None,
                    "alias_status": "placeholder",
                    "unresolved_tokens": placeholders,
                    "dictionary_readings": [],
                    "note": "Source placeholder preserved; no personal name was inferred.",
                }
            )
        elif not HAN_RE.search(chinese_name):
            alias.update(
                {
                    "han_viet_name": None,
                    "alias_status": "source_literal",
                    "unresolved_tokens": [],
                    "dictionary_readings": [],
                    "note": "Source name contains no Han characters, so no Hán-Việt alias was generated.",
                }
            )
        else:
            value, trace, unresolved = format_han_viet(chinese_name, dictionary_results[record["source_key"]])
            alias.update(
                {
                    "han_viet_name": value,
                    "alias_status": "generated" if value is not None else "unresolved",
                    "unresolved_tokens": unresolved,
                    "dictionary_readings": trace,
                }
            )
            if unresolved:
                unresolved_by_source[record["source_key"]] = unresolved
        status_counts[alias["alias_status"]] += 1
        aliases.append(alias)

    return {
        "schema": HAN_VIET_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {
            "source_catalog": "games/honkai-star-rail/data/npcs.json",
            "source_catalog_schema": CATALOG_SCHEMA,
            "included_records": len(records),
            "output_records": len(aliases),
            "queried_records": len(queryable),
            "status_counts": status_counts,
            "source_key_coverage": {
                "catalog_records": len(records),
                "output_records": len(aliases),
                "missing_source_keys": [],
                "unexpected_source_keys": [],
            },
        },
        "confidence": {
            "han_viet_names": "E",
            "notes": {
                "E": "Exploratory generated search aliases, not official Vietnamese localization or human-reviewed proper-noun readings."
            },
        },
        "sources": {
            "npc_catalog": INDEX_URL,
            "npc_catalog_api": API_URL,
            "han_viet_dictionary": DICTIONARY_URL,
            "han_viet_dictionary_endpoint": DICTIONARY_ENDPOINT,
        },
        "generation": {
            "method": "Query each non-placeholder Simplified Chinese name with HVDic mode=trans and lang=1; select the first returned reading per Han character and preserve non-Han source text.",
            "placeholder_policy": "Preserve brace-delimited source placeholders and do not infer a personal name.",
            "official_fields_unchanged": True,
            "unresolved": unresolved_by_source,
        },
        "limitations": [
            "Hán-Việt values are generated search aliases only; they are not official Vietnamese names.",
            "The deterministic first dictionary alternative can be wrong for proper-noun context; all returned alternatives remain reviewable per record.",
            "The source index is official HoYoWiki content co-built by HoYoLAB editors and players, not a game-client NPC dump.",
        ],
        "npcs": aliases,
    }


def existing_count(path: Path, key: str) -> int | None:
    if not path.is_file():
        return None
    try:
        with path.open(encoding="utf-8") as handle:
            payload = json.load(handle)
        value = payload.get("catalog_scope", {}).get(key)
        return int(value) if value is not None else None
    except (OSError, ValueError, TypeError, json.JSONDecodeError) as error:
        raise RuntimeError(f"unable to inspect existing snapshot {path}: {error}") from error


def write_json_atomic(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
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
    args = parse_args()
    checked_at = validate_date(args.checked_at)
    if args.attempts < 1:
        raise ValueError("--attempts must be positive")
    if args.delay < 0:
        raise ValueError("--delay cannot be negative")

    english_records = crawl_locale(
        "en-us", delay=args.delay, timeout=args.timeout, attempts=args.attempts
    )
    chinese_records = crawl_locale(
        "zh-cn", delay=args.delay, timeout=args.timeout, attempts=args.attempts
    )
    catalog = build_catalog(english_records, chinese_records, checked_at=checked_at)
    han_viet = build_han_viet_snapshot(
        catalog,
        checked_at=checked_at,
        timeout=args.timeout,
        attempts=args.attempts,
        delay=args.delay,
    )

    output_catalog = args.output_dir / "npcs.json"
    output_han_viet = args.output_dir / "npc-han-viet.json"
    previous_catalog_count = existing_count(output_catalog, "included_records")
    if previous_catalog_count is not None and len(catalog["npcs"]) < previous_catalog_count and not args.allow_shrink:
        raise RuntimeError(
            f"refusing NPC catalog shrink from {previous_catalog_count} to {len(catalog['npcs'])}; review and pass --allow-shrink"
        )
    previous_alias_count = existing_count(output_han_viet, "output_records")
    if previous_alias_count is not None and len(han_viet["npcs"]) < previous_alias_count and not args.allow_shrink:
        raise RuntimeError(
            f"refusing Hán-Việt snapshot shrink from {previous_alias_count} to {len(han_viet['npcs'])}; review and pass --allow-shrink"
        )

    write_json_atomic(output_catalog, catalog)
    write_json_atomic(output_han_viet, han_viet)
    print(
        f"wrote {len(catalog['npcs'])} NPC records and {len(han_viet['npcs'])} Hán-Việt records; "
        f"statuses={han_viet['catalog_scope']['status_counts']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
