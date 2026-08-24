#!/usr/bin/env python3
"""Fetch the official Genshin character directory into a reviewable snapshot.

The script reads HoYoverse's public website content service only. It never opens
or mutates a Perwiga SQLite database. Refreshes are atomic and refuse to replace
an existing snapshot with fewer records unless ``--allow-shrink`` is explicit.
"""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import date, datetime, timezone
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import tempfile
import time
from typing import Any, Callable
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode, urlparse
from urllib.request import Request, urlopen


SCHEMA = "perwiga.genshin-impact.characters.v1"
# Contract identifiers are published by the official site bundle recorded in
# OFFICIAL_SITE_BUNDLE_URL and are re-exposed in every generated snapshot.
API_BASE = "https://sg-public-api-static.hoyoverse.com/content_v2_user"
CONTENT_KEY = "a1b1f9d3315447cc"
APP_ID = 32
DETAIL_CHANNEL_ID = 407
PAGE_SIZE = 100
MAX_RESPONSE_BYTES = 20 * 1024 * 1024
OFFICIAL_DIRECTORY_URL = "https://genshin.hoyoverse.com/en/character/mondstadt"
OFFICIAL_SITE_BUNDLE_URL = "https://genshin.hoyoverse.com/_nuxt/446ceb2.js"

REGION_CHANNELS: tuple[tuple[str, int, str], ...] = (
    ("Mondstadt", 403, "mondstadt"),
    ("Liyue", 404, "liyue"),
    ("Inazuma", 405, "inazuma"),
    ("Sumeru", 406, "sumeru"),
    ("Fontaine", 653, "fontaine"),
    ("Natlan", 934, "natlan"),
    ("Snezhnaya", 1221, "snezhnaya"),
)
LOCALES: tuple[tuple[str, str], ...] = (
    ("english", "en-us"),
    ("traditional_chinese", "zh-tw"),
    ("vietnamese", "vi-vn"),
)
ALLOWED_THUMBNAIL_HOSTS = frozenset(
    {
        "act-webstatic.hoyoverse.com",
        "fastcdn.hoyoverse.com",
        "upload-static.hoyoverse.com",
        "webstatic.hoyoverse.com",
        "webstatic.mihoyo.com",
    }
)


class _PlainTextParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []

    def handle_data(self, data: str) -> None:
        self.parts.append(data)


def html_to_text(value: str) -> str:
    parser = _PlainTextParser()
    parser.feed(value)
    parser.close()
    return " ".join("".join(parser.parts).split())


def content_list_url(
    *, channel_id: int, locale: str, page: int, page_size: int = PAGE_SIZE
) -> str:
    query = urlencode(
        {
            "iAppId": APP_ID,
            "iChanId": channel_id,
            "iPageSize": page_size,
            "iPage": page,
            "sLangKey": locale,
        }
    )
    return f"{API_BASE}/app/{CONTENT_KEY}/getContentList?{query}"


def validate_page(payload: Any) -> dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("official API response must be an object")
    if payload.get("retcode") != 0:
        raise ValueError(
            f"official API retcode was {payload.get('retcode')!r}: "
            f"{payload.get('message', 'unknown error')}"
        )
    data = payload.get("data")
    if not isinstance(data, dict):
        raise ValueError("official API data must be an object")
    if not isinstance(data.get("iTotal"), int) or data["iTotal"] < 0:
        raise ValueError("official API data.iTotal must be a non-negative integer")
    if not isinstance(data.get("list"), list):
        raise ValueError("official API data.list must be an array")
    return data


def _http_get_json(url: str, *, timeout: float, attempts: int) -> dict[str, Any]:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = Request(
            url,
            headers={
                "Accept": "application/json",
                "Referer": "https://genshin.hoyoverse.com/",
                "User-Agent": "Perwiga Genshin snapshot curator/1.0",
            },
        )
        try:
            with urlopen(request, timeout=timeout) as response:
                content_length = response.headers.get("Content-Length")
                if content_length is not None and int(content_length) > MAX_RESPONSE_BYTES:
                    raise ValueError("official API response exceeds the size limit")
                body = response.read(MAX_RESPONSE_BYTES + 1)
                if len(body) > MAX_RESPONSE_BYTES:
                    raise ValueError("official API response exceeds the size limit")
            payload = json.loads(body.decode("utf-8"))
            if not isinstance(payload, dict):
                raise ValueError("official API response must be a JSON object")
            return payload
        except (HTTPError, URLError, TimeoutError, json.JSONDecodeError, ValueError) as error:
            last_error = error
            if attempt < attempts:
                time.sleep(min(attempt * 2, 6))
    raise RuntimeError(f"unable to fetch {url} after {attempts} attempts: {last_error}")


def fetch_region(
    *,
    region: str,
    channel_id: int,
    locale: str,
    timeout: float,
    attempts: int,
    get_json: Callable[..., dict[str, Any]] = _http_get_json,
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    page = 1
    expected_total: int | None = None
    while expected_total is None or len(records) < expected_total:
        payload = get_json(
            content_list_url(channel_id=channel_id, locale=locale, page=page),
            timeout=timeout,
            attempts=attempts,
        )
        data = validate_page(payload)
        if expected_total is None:
            expected_total = data["iTotal"]
        elif data["iTotal"] != expected_total:
            raise ValueError(
                f"official {region} total changed during pagination for {locale}"
            )
        page_records = data["list"]
        if not page_records and len(records) < expected_total:
            raise ValueError(
                f"official {region} catalog ended before iTotal for {locale}"
            )
        records.extend(page_records)
        page += 1
    if len(records) != expected_total:
        raise ValueError(
            f"official {region} catalog returned {len(records)} records, expected {expected_total}"
        )
    return records


def fetch_catalogs(
    *, timeout: float, attempts: int, workers: int
) -> dict[str, dict[str, list[dict[str, Any]]]]:
    fetched = {
        locale: {region: [] for region, _, _ in REGION_CHANNELS}
        for _, locale in LOCALES
    }
    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {
            executor.submit(
                fetch_region,
                region=region,
                channel_id=channel_id,
                locale=locale,
                timeout=timeout,
                attempts=attempts,
            ): (locale, region)
            for _, locale in LOCALES
            for region, channel_id, _ in REGION_CHANNELS
        }
        for future in as_completed(futures):
            locale, region = futures[future]
            fetched[locale][region] = future.result()
    return fetched


def _validated_record(record: Any, *, locale: str, region: str) -> dict[str, Any]:
    if not isinstance(record, dict):
        raise ValueError(f"official {locale} {region} record must be an object")
    info_id = record.get("iInfoId")
    title = record.get("sTitle")
    if not isinstance(info_id, int) or info_id <= 0:
        raise ValueError(f"official {locale} {region} iInfoId must be a positive integer")
    if not isinstance(title, str) or not title.strip():
        raise ValueError(f"official content ID {info_id} has no {locale} title")
    channels = record.get("sChanId")
    if not isinstance(channels, list) or not all(isinstance(value, str) for value in channels):
        raise ValueError(f"official content ID {info_id} has invalid channel IDs")
    extension = record.get("sExt")
    if not isinstance(extension, str):
        raise ValueError(f"official content ID {info_id} has no extension payload")
    try:
        parsed_extension = json.loads(extension)
    except json.JSONDecodeError as error:
        raise ValueError(
            f"official content ID {info_id} has invalid extension JSON"
        ) from error
    if not isinstance(parsed_extension, dict):
        raise ValueError(f"official content ID {info_id} extension must be an object")
    result = dict(record)
    result["_extension"] = parsed_extension
    return result


def _index_catalogs(
    catalogs: dict[str, dict[str, list[dict[str, Any]]]]
) -> dict[str, dict[int, tuple[str, dict[str, Any]]]]:
    indexed: dict[str, dict[int, tuple[str, dict[str, Any]]]] = {}
    for _, locale in LOCALES:
        locale_catalogs = catalogs.get(locale)
        if not isinstance(locale_catalogs, dict):
            raise ValueError(f"missing fetched catalog for locale {locale}")
        by_id: dict[int, tuple[str, dict[str, Any]]] = {}
        for region, channel_id, _ in REGION_CHANNELS:
            records = locale_catalogs.get(region)
            if not isinstance(records, list):
                raise ValueError(f"missing fetched catalog for {locale} {region}")
            for raw_record in records:
                record = _validated_record(raw_record, locale=locale, region=region)
                info_id = record["iInfoId"]
                if info_id in by_id:
                    raise ValueError(
                        f"duplicate official content ID {info_id} in {locale} catalog"
                    )
                expected_channel = str(channel_id)
                if expected_channel not in record["sChanId"]:
                    raise ValueError(
                        f"official content ID {info_id} is missing region channel {channel_id}"
                    )
                by_id[info_id] = (region, record)
        indexed[locale] = by_id
    return indexed


def _thumbnail_url(record: dict[str, Any]) -> str:
    info_id = record["iInfoId"]
    icons = record["_extension"].get(f"{DETAIL_CHANNEL_ID}_0")
    if not isinstance(icons, list) or not icons or not isinstance(icons[0], dict):
        raise ValueError(f"official content ID {info_id} has no thumbnail")
    url = icons[0].get("url")
    if not isinstance(url, str):
        raise ValueError(f"official content ID {info_id} has no thumbnail URL")
    parsed = urlparse(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname not in ALLOWED_THUMBNAIL_HOSTS
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
    ):
        raise ValueError(f"untrusted thumbnail URL for official content ID {info_id}: {url}")
    suffix = Path(parsed.path).suffix.lower()
    if suffix not in {".png", ".webp", ".jpg", ".jpeg"}:
        raise ValueError(f"unsupported thumbnail type for official content ID {info_id}")
    return url


def _description(record: dict[str, Any]) -> str | None:
    value = record["_extension"].get(f"{DETAIL_CHANNEL_ID}_7")
    if not isinstance(value, str):
        return None
    plain = html_to_text(value)
    return plain or None


def build_snapshot(
    catalogs: dict[str, dict[str, list[dict[str, Any]]]],
    *,
    checked_at: str,
    generated_at: str,
) -> dict[str, Any]:
    indexed = _index_catalogs(catalogs)
    english = indexed["en-us"]
    english_ids = set(english)
    for locale in ("zh-tw", "vi-vn"):
        unexpected = sorted(set(indexed[locale]) - english_ids)
        if unexpected:
            raise ValueError(
                f"{locale} catalog contains content IDs absent from English: {unexpected}"
            )

    region_order = {region: position for position, (region, _, _) in enumerate(REGION_CHANNELS)}
    channel_by_region = {region: channel for region, channel, _ in REGION_CHANNELS}
    locale_gaps: list[dict[str, str]] = []
    characters: list[dict[str, Any]] = []
    for info_id, (region, english_record) in english.items():
        source_key = f"hoyoverse-content-{info_id}"
        traditional_chinese_record = indexed["zh-tw"].get(info_id)
        vietnamese_record = indexed["vi-vn"].get(info_id)
        if (
            traditional_chinese_record is not None
            and traditional_chinese_record[0] != region
        ):
            raise ValueError(f"official content ID {info_id} changes region in zh-tw")
        if vietnamese_record is not None and vietnamese_record[0] != region:
            raise ValueError(f"official content ID {info_id} changes region in vi-vn")

        traditional_chinese_name = (
            traditional_chinese_record[1]["sTitle"].strip()
            if traditional_chinese_record
            else None
        )
        vietnamese_name = (
            vietnamese_record[1]["sTitle"].strip() if vietnamese_record else None
        )
        if traditional_chinese_name is None:
            locale_gaps.append(
                {
                    "source_key": source_key,
                    "locale": "zh-tw",
                    "field": "official_traditional_chinese_name",
                }
            )
        if vietnamese_name is None:
            locale_gaps.append(
                {
                    "source_key": source_key,
                    "locale": "vi-vn",
                    "field": "official_vietnamese_name",
                }
            )

        thumbnail_url = _thumbnail_url(english_record)
        thumbnail_suffix = Path(urlparse(thumbnail_url).path).suffix.lower()
        characters.append(
            {
                "source_key": source_key,
                "official_content_id": info_id,
                "region": region,
                "region_channel_id": channel_by_region[region],
                "official_english_name": english_record["sTitle"].strip(),
                "official_traditional_chinese_name": traditional_chinese_name,
                "official_vietnamese_name": vietnamese_name,
                "official_english_description": _description(english_record),
                "thumbnail_source_url": thumbnail_url,
                "thumbnail_asset": f"{source_key}{thumbnail_suffix}",
                "source_channels": sorted(int(value) for value in english_record["sChanId"]),
                "available_from": english_record.get("dtStartTime"),
                "available_until": english_record.get("dtEndTime"),
                "source_created_at": english_record.get("dtCreateTime"),
            }
        )

    characters.sort(
        key=lambda item: (
            region_order[item["region"]],
            item["official_english_name"].casefold(),
            item["official_content_id"],
        )
    )
    locale_gaps.sort(key=lambda item: (item["source_key"], item["locale"]))
    region_counts = {
        region: sum(character["region"] == region for character in characters)
        for region, _, _ in REGION_CHANNELS
    }
    return {
        "schema": SCHEMA,
        "checked_at": checked_at,
        "generated_at": generated_at,
        "sources": {
            "official_character_directory": OFFICIAL_DIRECTORY_URL,
            "official_content_api": API_BASE,
            "api_contract_discovered_from": OFFICIAL_SITE_BUNDLE_URL,
            "api_contract": {
                "app_id": APP_ID,
                "content_key": CONTENT_KEY,
                "detail_channel_id": DETAIL_CHANNEL_ID,
                "locales": {label: locale for label, locale in LOCALES},
                "region_channels": {
                    region: channel for region, channel, _ in REGION_CHANNELS
                },
            },
        },
        "catalog_scope": {
            "boundary": (
                "Playable-character records returned by the official Genshin Impact "
                "website's seven regional character-directory channels."
            ),
            "included_records": len(characters),
            "region_counts": region_counts,
            "locale_gaps": locale_gaps,
        },
        "characters": characters,
    }


def validate_replacement(
    output: Path, snapshot: dict[str, Any], *, allow_shrink: bool
) -> None:
    if snapshot.get("schema") != SCHEMA:
        raise ValueError(f"new snapshot must use schema {SCHEMA}")
    characters = snapshot.get("characters")
    included = snapshot.get("catalog_scope", {}).get("included_records")
    if not isinstance(characters, list) or included != len(characters):
        raise ValueError("new snapshot record count is inconsistent")
    if not output.exists():
        return
    with output.open(encoding="utf-8") as handle:
        existing = json.load(handle)
    if existing.get("schema") != SCHEMA:
        raise ValueError(f"existing output does not use schema {SCHEMA}")
    existing_count = existing.get("catalog_scope", {}).get("included_records")
    if not isinstance(existing_count, int) or existing_count < 0:
        raise ValueError("existing snapshot record count is invalid")
    if included < existing_count and not allow_shrink:
        raise ValueError(
            f"refresh would shrink the character catalog from {existing_count} to {included}; "
            "inspect the source change and pass --allow-shrink only if intentional"
        )


def write_snapshot(output: Path, snapshot: dict[str, Any]) -> None:
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
    except Exception:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass
        raise


def utc_timestamp() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def main() -> None:
    module_dir = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=module_dir / "data" / "characters.json",
        help="snapshot path (default: module data/characters.json)",
    )
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument(
        "--allow-shrink",
        action="store_true",
        help="allow a reviewed upstream catalog contraction",
    )
    args = parser.parse_args()
    if args.timeout <= 0 or args.attempts <= 0 or not 1 <= args.workers <= 8:
        parser.error("timeout and attempts must be positive; workers must be between 1 and 8")

    output = args.output.resolve()
    fetched = fetch_catalogs(
        timeout=args.timeout,
        attempts=args.attempts,
        workers=args.workers,
    )
    snapshot = build_snapshot(
        fetched,
        checked_at=args.checked_at,
        generated_at=utc_timestamp(),
    )
    validate_replacement(output, snapshot, allow_shrink=args.allow_shrink)
    write_snapshot(output, snapshot)
    gaps = len(snapshot["catalog_scope"]["locale_gaps"])
    print(
        f"Wrote {snapshot['catalog_scope']['included_records']} official Genshin "
        f"characters to {output} ({gaps} locale gaps)"
    )


if __name__ == "__main__":
    main()
