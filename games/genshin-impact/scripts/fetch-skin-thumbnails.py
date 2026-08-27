#!/usr/bin/env python3
"""Download validated official HoYoWiki Skin thumbnails without overwriting assets."""

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


SCHEMA = "perwiga.genshin-impact.skins.v1"
MAX_IMAGE_BYTES = 12 * 1024 * 1024
ASSET_PATTERN = re.compile(r"^hoyowiki-outfit-([1-9][0-9]*)\.(png|gif)$")
ALLOWED_HOSTS = frozenset(
    {
        "act-webstatic.hoyoverse.com",
        "fastcdn.hoyoverse.com",
        "upload-static.hoyoverse.com",
        "webstatic.hoyoverse.com",
    }
)


def validate_snapshot(snapshot: Any) -> list[dict[str, Any]]:
    if not isinstance(snapshot, dict) or snapshot.get("schema") != SCHEMA:
        raise ValueError(f"snapshot must use schema {SCHEMA}")
    records = snapshot.get("skins")
    if not isinstance(records, list) or snapshot.get("catalog_scope", {}).get(
        "included_records"
    ) != len(records):
        raise ValueError("snapshot record count is inconsistent")
    seen: set[str] = set()
    for record in records:
        asset, _, _ = validated_asset(record)
        if asset in seen:
            raise ValueError(f"duplicate thumbnail asset: {asset}")
        seen.add(asset)
    return records


def validated_asset(record: dict[str, Any]) -> tuple[str, str, str]:
    source_key = record.get("source_key")
    entry_id = record.get("hoyowiki_entry_page_id")
    asset = record.get("thumbnail_asset")
    url = record.get("thumbnail_source_url")
    if not isinstance(entry_id, str) or not entry_id.isdigit() or entry_id.startswith("0"):
        raise ValueError("Skin has an invalid HoYoWiki entry ID")
    if source_key != f"hoyowiki-outfit-{entry_id}":
        raise ValueError(f"Skin {entry_id} has an inconsistent source key")
    if not isinstance(asset, str):
        raise ValueError(f"Skin {entry_id} has no thumbnail asset")
    match = ASSET_PATTERN.fullmatch(asset)
    if match is None or match.group(1) != entry_id:
        raise ValueError(f"unsafe thumbnail asset for Skin {entry_id}: {asset}")
    if not isinstance(url, str):
        raise ValueError(f"Skin {entry_id} has no thumbnail URL")
    parsed = urlparse(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname not in ALLOWED_HOSTS
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
    ):
        raise ValueError(f"untrusted thumbnail URL for Skin {entry_id}: {url}")
    extension = match.group(2)
    if Path(parsed.path).suffix.lower() != f".{extension}":
        raise ValueError(f"thumbnail type mismatch for Skin {entry_id}")
    return asset, extension, url


def validate_image(body: bytes, extension: str) -> None:
    if extension == "png" and body.startswith(b"\x89PNG\r\n\x1a\n"):
        return
    if extension == "gif" and body.startswith((b"GIF87a", b"GIF89a")):
        return
    raise ValueError(f"download is not a valid {extension.upper()} image")


def write_validated_asset(record: dict[str, Any], body: bytes, output_dir: Path) -> str:
    asset, extension, _ = validated_asset(record)
    output_dir.mkdir(parents=True, exist_ok=True)
    target = output_dir / asset
    if target.exists():
        if not target.is_file():
            raise ValueError(f"thumbnail target is not a regular file: {target}")
        validate_image(target.read_bytes(), extension)
        return "skipped"
    if len(body) > MAX_IMAGE_BYTES:
        raise ValueError(f"thumbnail exceeds {MAX_IMAGE_BYTES} bytes: {asset}")
    validate_image(body, extension)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{asset}.", suffix=".tmp", dir=output_dir
    )
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(body)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary_name, target)
        except FileExistsError:
            validate_image(target.read_bytes(), extension)
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
                "Accept": "image/png,image/gif,*/*;q=0.8",
                "Referer": "https://wiki.hoyolab.com/",
                "User-Agent": "Perwiga Genshin Skin curator/1.0",
            },
        )
        try:
            with urlopen(request, timeout=timeout) as response:
                final = urlparse(response.geturl())
                if final.scheme != "https" or final.hostname not in ALLOWED_HOSTS:
                    raise ValueError("thumbnail redirected to an untrusted URL")
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


def fetch_one(record: dict[str, Any], output: Path, timeout: float, attempts: int) -> str:
    asset, extension, url = validated_asset(record)
    target = output / asset
    if target.exists():
        validate_image(target.read_bytes(), extension)
        return "skipped"
    return write_validated_asset(
        record, download(url, timeout=timeout, attempts=attempts), output
    )


def fetch_assets(
    records: list[dict[str, Any]], output: Path, *, timeout: float, attempts: int, workers: int
) -> dict[str, int]:
    counts = {"downloaded": 0, "skipped": 0}
    output.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = [
            executor.submit(fetch_one, record, output, timeout, attempts)
            for record in records
        ]
        for future in as_completed(futures):
            counts[future.result()] += 1
    return counts


def main() -> None:
    module_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, default=module_root / "data/skins.json")
    parser.add_argument("--output-dir", type=Path, default=module_root / "assets/skins")
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--workers", type=int, default=6)
    args = parser.parse_args()
    if args.timeout <= 0 or args.attempts <= 0 or not 1 <= args.workers <= 8:
        parser.error("timeout and attempts must be positive; workers must be between 1 and 8")
    with args.snapshot.resolve(strict=True).open(encoding="utf-8") as handle:
        records = validate_snapshot(json.load(handle))
    counts = fetch_assets(
        records,
        args.output_dir.resolve(),
        timeout=args.timeout,
        attempts=args.attempts,
        workers=args.workers,
    )
    print(
        f"Skin thumbnails ready: {counts['downloaded']} downloaded, "
        f"{counts['skipped']} already present, {len(records)} total"
    )


if __name__ == "__main__":
    main()
