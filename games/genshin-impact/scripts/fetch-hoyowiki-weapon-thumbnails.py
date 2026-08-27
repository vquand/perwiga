#!/usr/bin/env python3
"""Snapshot and download weapon icons from the official Genshin HoYoWiki."""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import date
import json
import os
from pathlib import Path
import tempfile
import time
from typing import Any, Callable
import unicodedata
from urllib.error import HTTPError, URLError
from urllib.parse import quote, unquote, urlparse, urlsplit, urlunsplit
from urllib.request import Request, urlopen


CATALOG_SCHEMA = "perwiga.genshin-impact.weapons.v1"
SNAPSHOT_SCHEMA = "perwiga.genshin-impact.weapon-thumbnails-hoyowiki.v1"
API_URL = "https://sg-wiki-api.hoyolab.com/hoyowiki/wapi/get_entry_page_list"
ARCHIVE_URL = "https://wiki.hoyolab.com/pc/genshin/aggregate/weapon"
ICON_DOMAINS = ("hoyoverse.com", "hoyolab.com")
MENU_ID = "4"
MAX_RESPONSE_BYTES = 8 * 1024 * 1024
MAX_IMAGE_BYTES = 8 * 1024 * 1024


def validate_catalog(catalog: Any) -> list[dict[str, Any]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError(f"catalog must use schema {CATALOG_SCHEMA}")
    weapons = catalog.get("weapons")
    included = catalog.get("catalog_scope", {}).get("included_records")
    if not isinstance(weapons, list) or included != len(weapons):
        raise ValueError("weapon catalog count is inconsistent")
    names: set[str] = set()
    keys: set[str] = set()
    for weapon in weapons:
        if not isinstance(weapon, dict):
            raise ValueError("catalog weapon must be an object")
        name = weapon.get("official_english_name")
        source_key = weapon.get("source_key")
        asset = weapon.get("thumbnail_asset")
        if not isinstance(name, str) or not name or name in names:
            raise ValueError(f"invalid or duplicate catalog weapon name: {name!r}")
        if not isinstance(source_key, str) or source_key in keys:
            raise ValueError(f"invalid or duplicate catalog weapon key: {source_key!r}")
        if asset != f"{source_key}.png":
            raise ValueError(f"unsafe catalog thumbnail asset: {asset!r}")
        names.add(name)
        keys.add(source_key)
    return weapons


def validate_icon_url(url: Any) -> str:
    if not isinstance(url, str):
        raise ValueError("HoYoWiki weapon has no icon URL")
    parsed = urlparse(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname is None
        or not any(parsed.hostname.endswith(f".{domain}") for domain in ICON_DOMAINS)
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
        or not parsed.path.lower().endswith((".png", ".webp"))
    ):
        raise ValueError(f"untrusted HoYoWiki icon URL: {url}")
    return url


def requestable_url(url: str) -> str:
    parts = urlsplit(validate_icon_url(url))
    return urlunsplit((parts.scheme, parts.netloc, quote(unquote(parts.path), safe="/%"), "", ""))


def normalized_name(name: str) -> str:
    normalized = unicodedata.normalize("NFKC", name).replace("’", "'").replace("‘", "'")
    if len(normalized) >= 2 and normalized[0] == normalized[-1] == '"':
        normalized = normalized[1:-1]
    return normalized


def request_page(page_number: int, page_size: int, *, timeout: float = 30.0) -> Any:
    body = json.dumps(
        {
            "filters": [],
            "menu_id": MENU_ID,
            "page_num": page_number,
            "page_size": page_size,
            "use_es": True,
        },
        separators=(",", ":"),
    ).encode("utf-8")
    request = Request(
        API_URL,
        data=body,
        method="POST",
        headers={
            "Content-Type": "application/json",
            "Origin": "https://wiki.hoyolab.com",
            "Referer": f"{ARCHIVE_URL}?lang=en-us",
            "User-Agent": "Perwiga Genshin HoYoWiki curator/1.0",
            "x-rpc-client_type": "4",
            "x-rpc-language": "en-us",
            "x-rpc-wiki_app": "genshin",
        },
    )
    with urlopen(request, timeout=timeout) as response:
        payload = response.read(MAX_RESPONSE_BYTES + 1)
    if len(payload) > MAX_RESPONSE_BYTES:
        raise ValueError("HoYoWiki response exceeds the size limit")
    return json.loads(payload)


def fetch_entries(
    get_page: Callable[[int, int], Any] = request_page, *, page_size: int = 50
) -> list[dict[str, Any]]:
    if page_size < 1 or page_size > 50:
        raise ValueError("HoYoWiki page size must be between 1 and 50")
    entries: list[dict[str, Any]] = []
    expected_total: int | None = None
    page_number = 1
    while expected_total is None or len(entries) < expected_total:
        response = get_page(page_number, page_size)
        if not isinstance(response, dict) or response.get("retcode") != 0:
            raise ValueError(f"HoYoWiki returned an unsuccessful response on page {page_number}")
        data = response.get("data")
        page_entries = data.get("list") if isinstance(data, dict) else None
        try:
            total = int(data.get("total")) if isinstance(data, dict) else -1
        except (TypeError, ValueError):
            total = -1
        if total < 0 or not isinstance(page_entries, list):
            raise ValueError(f"HoYoWiki returned malformed pagination on page {page_number}")
        if expected_total is None:
            expected_total = total
        elif total != expected_total:
            raise ValueError("HoYoWiki total changed during pagination")
        if not page_entries and len(entries) < expected_total:
            raise ValueError("HoYoWiki pagination ended before the advertised total")
        entries.extend(page_entries)
        page_number += 1
    if len(entries) != expected_total:
        raise ValueError("HoYoWiki returned more entries than its advertised total")
    return entries


def build_snapshot(
    catalog: Any, entries: list[dict[str, Any]], *, checked_at: str
) -> dict[str, Any]:
    weapons = validate_catalog(catalog)
    by_name: dict[str, dict[str, Any]] = {}
    entry_ids: set[str] = set()
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("HoYoWiki weapon entry must be an object")
        name = entry.get("name")
        entry_id = entry.get("entry_page_id")
        if not isinstance(name, str) or not name:
            raise ValueError("HoYoWiki weapon has no English name")
        match_name = normalized_name(name)
        if match_name in by_name:
            raise ValueError(f"duplicate HoYoWiki weapon name: {name}")
        if not isinstance(entry_id, str) or not entry_id or entry_id in entry_ids:
            raise ValueError(f"invalid or duplicate HoYoWiki entry ID: {entry_id!r}")
        icon_url = validate_icon_url(entry.get("icon_url"))
        by_name[match_name] = {
            "entry_page_id": entry_id,
            "icon_url": icon_url,
            "official_english_name": name,
        }
        entry_ids.add(entry_id)

    records = []
    matched_names = set()
    missing = []
    for weapon in weapons:
        name = weapon["official_english_name"]
        match_name = normalized_name(name)
        wiki = by_name.get(match_name)
        if wiki is None:
            missing.append(weapon["source_key"])
            continue
        matched_names.add(match_name)
        records.append(
            {
                "source_key": weapon["source_key"],
                "official_english_name": name,
                "hoyowiki_english_name": wiki["official_english_name"],
                "hoyowiki_entry_page_id": wiki["entry_page_id"],
                "thumbnail_asset": weapon["thumbnail_asset"],
                "thumbnail_source_url": wiki["icon_url"],
            }
        )
    unknown = sorted(set(by_name) - matched_names)
    if unknown:
        raise ValueError(f"HoYoWiki includes names absent from the pinned catalog: {unknown}")
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "source": {"provider": "Official Genshin HoYoWiki", "url": ARCHIVE_URL},
        "catalog_scope": {
            "catalog_records": len(weapons),
            "wiki_matches": len(records),
            "missing_from_hoyowiki": missing,
        },
        "weapons": records,
    }


def validate_png(body: bytes) -> None:
    if not body.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("download is not a valid PNG image")


def download_icon(url: str, *, timeout: float, attempts: int) -> bytes:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = Request(
            requestable_url(url),
            headers={
                "Accept": "image/png,*/*;q=0.8",
                "Referer": "https://wiki.hoyolab.com/",
                "User-Agent": "Perwiga Genshin HoYoWiki curator/1.0",
            },
        )
        try:
            with urlopen(request, timeout=timeout) as response:
                final_url = response.geturl()
                validate_icon_url(final_url)
                length = response.headers.get("Content-Length")
                if length is not None and int(length) > MAX_IMAGE_BYTES:
                    raise ValueError("HoYoWiki icon exceeds the size limit")
                body = response.read(MAX_IMAGE_BYTES + 1)
            if len(body) > MAX_IMAGE_BYTES:
                raise ValueError("HoYoWiki icon exceeds the size limit")
            validate_png(body)
            return body
        except (HTTPError, URLError, TimeoutError, ValueError) as error:
            last_error = error
            if attempt < attempts:
                time.sleep(min(attempt * 2, 6))
    raise RuntimeError(f"unable to download {url} after {attempts} attempts: {last_error}")


def write_new_asset(record: dict[str, Any], body: bytes, output_dir: Path) -> str:
    validate_png(body)
    if len(body) > MAX_IMAGE_BYTES:
        raise ValueError("HoYoWiki icon exceeds the size limit")
    asset = record["thumbnail_asset"]
    if asset != f"{record['source_key']}.png" or Path(asset).name != asset:
        raise ValueError(f"unsafe HoYoWiki asset path: {asset!r}")
    output_dir.mkdir(parents=True, exist_ok=True)
    target = output_dir / asset
    if target.exists():
        validate_png(target.read_bytes())
        return "skipped"
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{asset}.", suffix=".tmp", dir=output_dir)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(body)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary_name, target)
        except FileExistsError:
            validate_png(target.read_bytes())
            return "skipped"
        return "downloaded"
    finally:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass


def fetch_assets(
    records: list[dict[str, Any]], output_dir: Path, *, timeout: float, attempts: int, workers: int
) -> dict[str, int]:
    counts = {"downloaded": 0, "skipped": 0}

    def fetch_one(record: dict[str, Any]) -> str:
        target = output_dir / record["thumbnail_asset"]
        if target.exists():
            validate_png(target.read_bytes())
            return "skipped"
        return write_new_asset(
            record,
            download_icon(record["thumbnail_source_url"], timeout=timeout, attempts=attempts),
            output_dir,
        )

    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = [executor.submit(fetch_one, record) for record in records]
        for future in as_completed(futures):
            counts[future.result()] += 1
    return counts


def main() -> None:
    module_dir = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=module_dir / "data/weapons.json")
    parser.add_argument(
        "--snapshot",
        type=Path,
        default=module_dir / "data/weapon-thumbnails-hoyowiki.json",
    )
    parser.add_argument(
        "--output-dir", type=Path, default=module_dir / "assets/weapons-hoyowiki"
    )
    parser.add_argument("--snapshot-only", action="store_true")
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--workers", type=int, default=6)
    args = parser.parse_args()

    catalog = json.loads(args.catalog.read_text(encoding="utf-8"))
    snapshot = build_snapshot(catalog, fetch_entries(), checked_at=date.today().isoformat())
    args.snapshot.write_text(
        json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(
        f"HoYoWiki snapshot: {snapshot['catalog_scope']['wiki_matches']} matched, "
        f"{len(snapshot['catalog_scope']['missing_from_hoyowiki'])} explicit gaps"
    )
    if not args.snapshot_only:
        counts = fetch_assets(
            snapshot["weapons"],
            args.output_dir.resolve(),
            timeout=args.timeout,
            attempts=args.attempts,
            workers=args.workers,
        )
        print(
            f"HoYoWiki weapon thumbnails: {counts['downloaded']} downloaded, "
            f"{counts['skipped']} unchanged"
        )


if __name__ == "__main__":
    main()
