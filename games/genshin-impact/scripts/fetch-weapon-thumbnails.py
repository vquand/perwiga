#!/usr/bin/env python3
"""Download non-overwriting local weapon thumbnails from the weapon snapshot."""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
import os
from pathlib import Path
import re
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen


SCHEMA = "perwiga.genshin-impact.weapons.v1"
MAX_IMAGE_BYTES = 8 * 1024 * 1024
ASSET_PATTERN = re.compile(r"^genshin-data-weapon-([1-9][0-9]*)\.png$")
ALLOWED_HOSTS = frozenset({"upload-os-bbs.mihoyo.com", "enka.network"})


def validate_snapshot(snapshot: Any) -> list[dict[str, Any]]:
    if not isinstance(snapshot, dict) or snapshot.get("schema") != SCHEMA:
        raise ValueError(f"snapshot must use schema {SCHEMA}")
    weapons = snapshot.get("weapons")
    included = snapshot.get("catalog_scope", {}).get("included_records")
    if not isinstance(weapons, list) or included != len(weapons):
        raise ValueError("snapshot record count is inconsistent")
    assets: set[str] = set()
    keys: set[str] = set()
    for record in weapons:
        if not isinstance(record, dict):
            raise ValueError("snapshot weapon must be an object")
        asset = record.get("thumbnail_asset")
        key = record.get("source_key")
        if not isinstance(asset, str) or asset in assets:
            raise ValueError(f"invalid or duplicate thumbnail asset: {asset!r}")
        if not isinstance(key, str) or key in keys:
            raise ValueError(f"invalid or duplicate weapon source key: {key!r}")
        assets.add(asset)
        keys.add(key)
        validated_asset(record)
    return weapons


def validated_asset(record: dict[str, Any]) -> tuple[str, list[str]]:
    weapon_id = record.get("game_data_id")
    source_key = record.get("source_key")
    asset = record.get("thumbnail_asset")
    urls = [record.get("thumbnail_source_url")]
    if record.get("thumbnail_fallback_url") is not None:
        urls.append(record.get("thumbnail_fallback_url"))
    if not isinstance(weapon_id, int) or weapon_id <= 0:
        raise ValueError("weapon has an invalid game-data ID")
    expected_key = f"genshin-data-weapon-{weapon_id}"
    if source_key != expected_key or asset != f"{expected_key}.png" or not ASSET_PATTERN.fullmatch(str(asset)):
        raise ValueError(f"unsafe thumbnail asset for weapon {weapon_id}: {asset}")
    for url in urls:
        if not isinstance(url, str):
            raise ValueError(f"weapon {weapon_id} has no thumbnail URL")
        parsed = urlparse(url)
        if (
            parsed.scheme != "https"
            or parsed.hostname not in ALLOWED_HOSTS
            or parsed.username is not None
            or parsed.password is not None
            or parsed.query
            or parsed.fragment
            or not parsed.path.endswith(".png")
        ):
            raise ValueError(f"untrusted thumbnail URL for weapon {weapon_id}: {url}")
    return asset, urls


def validate_png(body: bytes) -> None:
    if not body.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("download is not a valid PNG image")


def write_validated_asset(record: dict[str, Any], body: bytes, output_dir: Path) -> str:
    asset, _ = validated_asset(record)
    output_dir.mkdir(parents=True, exist_ok=True)
    target = output_dir / asset
    if target.exists():
        if not target.is_file():
            raise ValueError(f"thumbnail target is not a regular file: {target}")
        validate_png(target.read_bytes())
        return "skipped"
    if len(body) > MAX_IMAGE_BYTES:
        raise ValueError(f"thumbnail exceeds {MAX_IMAGE_BYTES} bytes: {asset}")
    validate_png(body)
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


def download(url: str, *, timeout: float, attempts: int) -> bytes:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = Request(
            url,
            headers={
                "Accept": "image/png,*/*;q=0.8",
                "Referer": "https://wiki.hoyolab.com/",
                "User-Agent": "Perwiga Genshin weapon asset curator/1.0",
            },
        )
        try:
            with urlopen(request, timeout=timeout) as response:
                final = urlparse(response.geturl())
                if final.scheme != "https" or final.hostname not in ALLOWED_HOSTS:
                    raise ValueError(f"thumbnail redirected to an untrusted URL: {response.geturl()}")
                length = response.headers.get("Content-Length")
                if length is not None and int(length) > MAX_IMAGE_BYTES:
                    raise ValueError("thumbnail exceeds the response size limit")
                body = response.read(MAX_IMAGE_BYTES + 1)
                if len(body) > MAX_IMAGE_BYTES:
                    raise ValueError("thumbnail exceeds the response size limit")
                return body
        except (HTTPError, URLError, TimeoutError, ValueError) as error:
            last_error = error
            if attempt < attempts:
                time.sleep(min(attempt * 2, 6))
    raise RuntimeError(f"unable to download {url} after {attempts} attempts: {last_error}")


def fetch_one(record: dict[str, Any], output_dir: Path, *, timeout: float, attempts: int) -> str:
    asset, urls = validated_asset(record)
    target = output_dir / asset
    if target.exists():
        validate_png(target.read_bytes())
        return "skipped"
    errors = []
    for url in urls:
        try:
            return write_validated_asset(
                record, download(url, timeout=timeout, attempts=attempts), output_dir
            )
        except RuntimeError as error:
            errors.append(str(error))
    raise RuntimeError(f"all thumbnail sources failed for {asset}: {'; '.join(errors)}")


def fetch_assets(
    weapons: list[dict[str, Any]], output_dir: Path, *, timeout: float, attempts: int, workers: int
) -> dict[str, int]:
    output_dir.mkdir(parents=True, exist_ok=True)
    counts = {"downloaded": 0, "skipped": 0}
    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {
            executor.submit(fetch_one, record, output_dir, timeout=timeout, attempts=attempts): record["source_key"]
            for record in weapons
        }
        for future in as_completed(futures):
            counts[future.result()] += 1
    return counts


def main() -> None:
    module_dir = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, default=module_dir / "data/weapons.json")
    parser.add_argument("--output-dir", type=Path, default=module_dir / "assets/weapons")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--workers", type=int, default=6)
    args = parser.parse_args()
    snapshot = json.loads(args.snapshot.read_text(encoding="utf-8"))
    weapons = validate_snapshot(snapshot)
    counts = fetch_assets(
        weapons,
        args.output_dir.resolve(),
        timeout=args.timeout,
        attempts=args.attempts,
        workers=args.workers,
    )
    print(f"weapon thumbnails: {counts['downloaded']} downloaded, {counts['skipped']} unchanged")


if __name__ == "__main__":
    main()
