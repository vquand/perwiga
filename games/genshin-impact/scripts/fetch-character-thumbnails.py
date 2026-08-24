#!/usr/bin/env python3
"""Download module-local thumbnails from a validated Genshin snapshot.

Existing assets are verified and never overwritten. Downloads use bounded
timeouts, retries, response sizes, trusted first-party hosts, and image-magic
validation before an atomic no-clobber write.
"""

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


SCHEMA = "perwiga.genshin-impact.characters.v1"
MAX_IMAGE_BYTES = 12 * 1024 * 1024
ASSET_PATTERN = re.compile(r"^hoyoverse-content-([1-9][0-9]*)\.(png|webp|jpg|jpeg)$")
ALLOWED_THUMBNAIL_HOSTS = frozenset(
    {
        "act-webstatic.hoyoverse.com",
        "fastcdn.hoyoverse.com",
        "upload-static.hoyoverse.com",
        "webstatic.hoyoverse.com",
        "webstatic.mihoyo.com",
    }
)


def validate_snapshot(snapshot: Any) -> list[dict[str, Any]]:
    if not isinstance(snapshot, dict) or snapshot.get("schema") != SCHEMA:
        raise ValueError(f"snapshot must use schema {SCHEMA}")
    characters = snapshot.get("characters")
    included = snapshot.get("catalog_scope", {}).get("included_records")
    if not isinstance(characters, list) or included != len(characters):
        raise ValueError("snapshot record count is inconsistent")
    assets: set[str] = set()
    source_keys: set[str] = set()
    for record in characters:
        if not isinstance(record, dict):
            raise ValueError("snapshot character must be an object")
        asset = record.get("thumbnail_asset")
        source_key = record.get("source_key")
        if asset in assets:
            raise ValueError(f"duplicate thumbnail asset: {asset}")
        if source_key in source_keys:
            raise ValueError(f"duplicate character source key: {source_key}")
        assets.add(asset)
        source_keys.add(source_key)
    return characters


def _validated_asset(record: dict[str, Any]) -> tuple[str, str, str]:
    info_id = record.get("official_content_id")
    source_key = record.get("source_key")
    asset = record.get("thumbnail_asset")
    url = record.get("thumbnail_source_url")
    if not isinstance(info_id, int) or info_id <= 0:
        raise ValueError("character has an invalid official content ID")
    expected_source_key = f"hoyoverse-content-{info_id}"
    if source_key != expected_source_key:
        raise ValueError(f"character {info_id} has an inconsistent source key")
    if not isinstance(asset, str):
        raise ValueError(f"character {info_id} has no thumbnail asset")
    match = ASSET_PATTERN.fullmatch(asset)
    if match is None or int(match.group(1)) != info_id:
        raise ValueError(f"unsafe thumbnail asset for character {info_id}: {asset}")
    if not isinstance(url, str):
        raise ValueError(f"character {info_id} has no thumbnail URL")
    parsed = urlparse(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname not in ALLOWED_THUMBNAIL_HOSTS
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
    ):
        raise ValueError(f"untrusted thumbnail URL for character {info_id}: {url}")
    url_extension = Path(parsed.path).suffix.lower().removeprefix(".")
    asset_extension = match.group(2)
    if url_extension != asset_extension:
        raise ValueError(f"thumbnail type mismatch for character {info_id}")
    return asset, asset_extension, url


def _validate_image(body: bytes, extension: str) -> None:
    valid = False
    label = extension.upper()
    if extension == "png":
        valid = body.startswith(b"\x89PNG\r\n\x1a\n")
    elif extension == "webp":
        valid = len(body) >= 12 and body[:4] == b"RIFF" and body[8:12] == b"WEBP"
        label = "WebP"
    elif extension in {"jpg", "jpeg"}:
        valid = len(body) >= 4 and body[:3] == b"\xff\xd8\xff" and body[-2:] == b"\xff\xd9"
        label = "JPEG"
    if not valid:
        raise ValueError(f"download is not a valid {label} image")


def write_validated_asset(
    record: dict[str, Any], body: bytes, output_dir: Path
) -> str:
    asset, extension, _ = _validated_asset(record)
    output_dir.mkdir(parents=True, exist_ok=True)
    target = output_dir / asset
    if target.exists():
        if not target.is_file():
            raise ValueError(f"thumbnail target is not a regular file: {target}")
        _validate_image(target.read_bytes(), extension)
        return "skipped"
    if len(body) > MAX_IMAGE_BYTES:
        raise ValueError(f"thumbnail exceeds {MAX_IMAGE_BYTES} bytes: {asset}")
    _validate_image(body, extension)

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
            _validate_image(target.read_bytes(), extension)
            return "skipped"
        return "downloaded"
    finally:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass


def _download(url: str, *, timeout: float, attempts: int) -> bytes:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = Request(
            url,
            headers={
                "Accept": "image/avif,image/webp,image/png,image/jpeg,*/*;q=0.8",
                "Referer": "https://genshin.hoyoverse.com/",
                "User-Agent": "Perwiga Genshin asset curator/1.0",
            },
        )
        try:
            with urlopen(request, timeout=timeout) as response:
                final = urlparse(response.geturl())
                if final.scheme != "https" or final.hostname not in ALLOWED_THUMBNAIL_HOSTS:
                    raise ValueError(f"thumbnail redirected to an untrusted URL: {response.geturl()}")
                content_length = response.headers.get("Content-Length")
                if content_length is not None and int(content_length) > MAX_IMAGE_BYTES:
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


def _fetch_one(
    record: dict[str, Any], output_dir: Path, *, timeout: float, attempts: int
) -> str:
    asset, extension, url = _validated_asset(record)
    target = output_dir / asset
    if target.exists():
        if not target.is_file():
            raise ValueError(f"thumbnail target is not a regular file: {target}")
        _validate_image(target.read_bytes(), extension)
        return "skipped"
    return write_validated_asset(record, _download(url, timeout=timeout, attempts=attempts), output_dir)


def fetch_assets(
    characters: list[dict[str, Any]],
    output_dir: Path,
    *,
    timeout: float,
    attempts: int,
    workers: int,
) -> dict[str, int]:
    output_dir.mkdir(parents=True, exist_ok=True)
    counts = {"downloaded": 0, "skipped": 0}
    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {
            executor.submit(
                _fetch_one,
                record,
                output_dir,
                timeout=timeout,
                attempts=attempts,
            ): record["source_key"]
            for record in characters
        }
        for future in as_completed(futures):
            result = future.result()
            counts[result] += 1
    return counts


def main() -> None:
    module_dir = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--snapshot",
        type=Path,
        default=module_dir / "data" / "characters.json",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=module_dir / "assets" / "characters",
    )
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--workers", type=int, default=6)
    args = parser.parse_args()
    if args.timeout <= 0 or args.attempts <= 0 or not 1 <= args.workers <= 8:
        parser.error("timeout and attempts must be positive; workers must be between 1 and 8")

    snapshot_path = args.snapshot.resolve(strict=True)
    with snapshot_path.open(encoding="utf-8") as handle:
        characters = validate_snapshot(json.load(handle))
    counts = fetch_assets(
        characters,
        args.output_dir.resolve(),
        timeout=args.timeout,
        attempts=args.attempts,
        workers=args.workers,
    )
    print(
        f"Character thumbnails ready: {counts['downloaded']} downloaded, "
        f"{counts['skipped']} already present, {len(characters)} total"
    )


if __name__ == "__main__":
    main()
