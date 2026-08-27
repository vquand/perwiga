#!/usr/bin/env python3
"""Download non-overwriting local thumbnails for Artifact sets and pieces."""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlparse
from urllib.request import Request, urlopen


SCHEMA = "perwiga.genshin-impact.artifacts.v1"
MAX_IMAGE_BYTES = 8 * 1024 * 1024
ASSET_PATTERN = re.compile(r"^genshin-data-artifact-(?:set|piece)-[a-z0-9-]+\.png$")
ALLOWED_HOSTS = frozenset({"act-webstatic.hoyoverse.com", "act-upload.hoyoverse.com", "bbs.hoyolab.com", "upload-static.hoyoverse.com", "upload-os-bbs.mihoyo.com"})


def validated_asset(record: dict[str, Any]) -> tuple[str, str]:
    source_key = record.get("source_key")
    asset = record.get("thumbnail_asset")
    url = record.get("thumbnail_source_url")
    if not isinstance(source_key, str) or asset != f"{source_key}.png" or not ASSET_PATTERN.fullmatch(str(asset)):
        raise ValueError(f"unsafe Artifact thumbnail asset: {asset!r}")
    if not isinstance(url, str):
        raise ValueError(f"Artifact {source_key} has no thumbnail URL")
    parsed = urlparse(url)
    if parsed.scheme != "https" or parsed.hostname not in ALLOWED_HOSTS or parsed.username is not None or parsed.password is not None or parsed.query or parsed.fragment or not parsed.path.lower().endswith(".png"):
        raise ValueError(f"untrusted Artifact thumbnail URL: {url}")
    return asset, url


def validate_snapshot(snapshot: Any) -> list[dict[str, Any]]:
    if not isinstance(snapshot, dict) or snapshot.get("schema") != SCHEMA:
        raise ValueError(f"snapshot must use schema {SCHEMA}")
    records = []
    assets = set()
    for collection in ("artifact_sets", "artifact_pieces"):
        entries = snapshot.get(collection)
        if not isinstance(entries, list) or snapshot.get("catalog_scope", {}).get(collection) != len(entries):
            raise ValueError(f"snapshot {collection} count is inconsistent")
        for record in entries:
            if not isinstance(record, dict):
                raise ValueError("Artifact thumbnail record must be an object")
            asset, _ = validated_asset(record)
            if asset in assets:
                raise ValueError(f"duplicate Artifact thumbnail asset: {asset}")
            assets.add(asset)
            records.append(record)
    return records


def validate_png(body: bytes) -> None:
    if not body.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("download is not a valid PNG image")


def normalize_png(body: bytes) -> bytes:
    if body.startswith(b"\x89PNG\r\n\x1a\n"):
        return body
    if not body.startswith(b"RIFF") or body[8:12] != b"WEBP":
        raise ValueError("download is neither a PNG nor WebP image")
    result = subprocess.run(
        ["magick", "webp:-", "png:-"],
        input=body,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        raise ValueError(f"unable to normalize mislabeled WebP thumbnail: {result.stderr.decode('utf-8', errors='replace').strip()}")
    validate_png(result.stdout)
    return result.stdout


def download(url: str, *, timeout: float, attempts: int) -> bytes:
    parsed_source = urlparse(url)
    url = parsed_source._replace(path=quote(parsed_source.path)).geturl()
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = Request(url, headers={"Accept": "image/png,*/*;q=0.8", "Referer": "https://wiki.hoyolab.com/", "User-Agent": "Perwiga Genshin Artifact asset curator/1.0"})
        try:
            with urlopen(request, timeout=timeout) as response:
                final = urlparse(response.geturl())
                if final.scheme != "https" or final.hostname not in ALLOWED_HOSTS:
                    raise ValueError(f"thumbnail redirected to an untrusted URL: {response.geturl()}")
                body = response.read(MAX_IMAGE_BYTES + 1)
                if len(body) > MAX_IMAGE_BYTES:
                    raise ValueError("thumbnail exceeds response size limit")
                return normalize_png(body)
        except (HTTPError, URLError, TimeoutError, ValueError) as error:
            last_error = error
            if attempt < attempts:
                time.sleep(min(attempt * 2, 6))
    raise RuntimeError(f"unable to download {url}: {last_error}")


def fetch_one(record: dict[str, Any], output_dir: Path, *, timeout: float, attempts: int) -> str:
    asset, url = validated_asset(record)
    target = output_dir / asset
    if target.exists():
        validate_png(target.read_bytes())
        return "skipped"
    body = download(url, timeout=timeout, attempts=attempts)
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


def main() -> None:
    module_dir = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", type=Path, default=module_dir / "data/artifacts.json")
    parser.add_argument("--output-dir", type=Path, default=module_dir / "assets/artifacts")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--workers", type=int, default=8)
    args = parser.parse_args()
    records = validate_snapshot(json.loads(args.snapshot.read_text(encoding="utf-8")))
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    counts = {"downloaded": 0, "skipped": 0}
    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(fetch_one, record, output_dir, timeout=args.timeout, attempts=args.attempts) for record in records]
        for future in as_completed(futures):
            counts[future.result()] += 1
    print(f"Artifact thumbnails: {counts['downloaded']} downloaded, {counts['skipped']} unchanged")


if __name__ == "__main__":
    main()
