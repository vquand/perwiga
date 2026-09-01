#!/usr/bin/env python3
"""Download validated local character thumbnails from the HSR snapshot.

The snapshot stores the upstream StarRailRes icon URL. This crawler verifies
that each URL is the expected pinned character asset, downloads it without
overwriting existing files, and writes the result under the title module's
assets directory for local and public serving.
"""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
import os
from pathlib import Path
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen


SCHEMA = "perwiga.honkai-star-rail.characters.v1"
ASSET_HOSTS = frozenset({"raw.githubusercontent.com", "vizualabstract.github.io"})
ASSET_PREFIXES = (
    "/Mar-7th/StarRailRes/d226befe3db13f2ec15f4161d5f34b1b607643fe/icon/character/",
    "/StarRailStaticAPI/assets/icon/character/",
)
MAX_IMAGE_BYTES = 2 * 1024 * 1024


def validate_snapshot(payload: Any) -> list[dict[str, Any]]:
    if not isinstance(payload, dict) or payload.get("schema") != SCHEMA:
        raise ValueError(f"snapshot must use schema {SCHEMA}")
    characters = payload.get("characters")
    included = payload.get("catalog_scope", {}).get("included_records")
    if not isinstance(characters, list) or included != len(characters):
        raise ValueError("snapshot record count is inconsistent")

    source_keys: set[str] = set()
    for record in characters:
        if not isinstance(record, dict):
            raise ValueError("snapshot character must be an object")
        source_key = record.get("source_key")
        if not isinstance(source_key, str) or source_key in source_keys:
            raise ValueError(f"duplicate or invalid character source key: {source_key}")
        source_keys.add(source_key)
        _validated_record(record)
    return characters


def _validated_record(record: dict[str, Any]) -> tuple[str, str]:
    source_key = record.get("source_key")
    game_data_id = record.get("game_data_id")
    url = record.get("thumbnail_url")
    if not isinstance(source_key, str) or not source_key.startswith("starrailres-character-"):
        raise ValueError(f"invalid character source key: {source_key}")
    if not isinstance(game_data_id, str) or not game_data_id.isdigit():
        raise ValueError(f"invalid game data ID for {source_key}")
    if source_key != f"starrailres-character-{game_data_id}":
        raise ValueError(f"source key and game data ID disagree: {source_key}")
    if not isinstance(url, str):
        raise ValueError(f"character {game_data_id} has no thumbnail URL")
    parsed = urlparse(url)
    expected_paths = tuple(f"{prefix}{game_data_id}.png" for prefix in ASSET_PREFIXES)
    if (
        parsed.scheme != "https"
        or parsed.hostname not in ASSET_HOSTS
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
        or parsed.path not in expected_paths
    ):
        raise ValueError(f"untrusted thumbnail URL for character {game_data_id}: {url}")
    return source_key, url


def _validate_png(body: bytes) -> None:
    if not body.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("download is not a PNG image")


def _download(url: str, *, timeout: float, attempts: int) -> bytes:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = Request(
            url,
            headers={
                "Accept": "image/png",
                "Referer": "https://github.com/Mar-7th/StarRailRes",
                "User-Agent": "Perwiga HSR asset curator/1.0",
            },
        )
        try:
            with urlopen(request, timeout=timeout) as response:
                final = urlparse(response.geturl())
                if final.scheme != "https" or final.hostname not in ASSET_HOSTS:
                    raise ValueError(f"thumbnail redirected to an untrusted URL: {response.geturl()}")
                content_type = response.headers.get_content_type()
                if content_type != "image/png":
                    raise ValueError(f"thumbnail has unexpected content type: {content_type}")
                content_length = response.headers.get("Content-Length")
                if content_length is not None and int(content_length) > MAX_IMAGE_BYTES:
                    raise ValueError("thumbnail exceeds the response size limit")
                body = response.read(MAX_IMAGE_BYTES + 1)
                if len(body) > MAX_IMAGE_BYTES:
                    raise ValueError("thumbnail exceeds the response size limit")
                _validate_png(body)
                return body
        except (HTTPError, URLError, TimeoutError, ValueError) as error:
            last_error = error
            if attempt < attempts:
                time.sleep(min(attempt * 2, 6))
    raise RuntimeError(f"unable to download {url} after {attempts} attempts: {last_error}")


def _write_if_missing(target: Path, body: bytes) -> str:
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        if not target.is_file():
            raise ValueError(f"thumbnail target is not a regular file: {target}")
        _validate_png(target.read_bytes())
        return "skipped"

    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{target.name}.", suffix=".tmp", dir=target.parent)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(body)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary_name, target)
        except FileExistsError:
            _validate_png(target.read_bytes())
            return "skipped"
        return "downloaded"
    finally:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass


def _fetch_one(record: dict[str, Any], output_dir: Path, *, timeout: float, attempts: int) -> str:
    source_key, url = _validated_record(record)
    return _write_if_missing(
        output_dir / f"{source_key}.png",
        _download(url, timeout=timeout, attempts=attempts),
    )


def main() -> None:
    module_dir = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, default=module_dir / "data" / "characters.json")
    parser.add_argument("--output-dir", type=Path, default=module_dir / "assets" / "characters")
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--workers", type=int, default=6)
    args = parser.parse_args()
    if args.timeout <= 0 or args.attempts <= 0 or not 1 <= args.workers <= 8:
        parser.error("timeout and attempts must be positive; workers must be between 1 and 8")

    with args.snapshot.resolve(strict=True).open(encoding="utf-8") as handle:
        characters = validate_snapshot(json.load(handle))
    output_dir = args.output_dir.resolve()
    counts = {"downloaded": 0, "skipped": 0}
    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = {
            executor.submit(
                _fetch_one,
                record,
                output_dir,
                timeout=args.timeout,
                attempts=args.attempts,
            ): record["source_key"]
            for record in characters
        }
        for future in as_completed(futures):
            counts[future.result()] += 1
    print(
        f"Character thumbnails ready: {counts['downloaded']} downloaded, "
        f"{counts['skipped']} already present, {len(characters)} total"
    )


if __name__ == "__main__":
    main()
