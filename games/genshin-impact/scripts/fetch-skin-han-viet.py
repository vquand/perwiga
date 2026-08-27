#!/usr/bin/env python3
"""Generate confidence-E Hán-Việt aliases for curated Genshin Skin names."""

from __future__ import annotations

import argparse
from datetime import date
import importlib.util
import json
import os
from pathlib import Path
import tempfile
from typing import Any


CATALOG_SCHEMA = "perwiga.genshin-impact.skins.v1"
SNAPSHOT_SCHEMA = "perwiga.genshin-impact.skin-han-viet.v1"


def load_script(filename: str, module_name: str):
    path = Path(__file__).with_name(filename)
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


TRANSCRIBER = load_script("fetch-character-han-viet.py", "genshin_skin_transcriber")
WEAPON_TRANSCRIBER = load_script("fetch-weapon-han-viet.py", "genshin_skin_formatter")


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def validate_catalog(catalog: Any) -> list[dict[str, Any]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unsupported Genshin Skin catalog schema")
    skins = catalog.get("skins")
    count = catalog.get("catalog_scope", {}).get("included_records")
    if not isinstance(skins, list) or count != len(skins):
        raise ValueError("Genshin Skin catalog record count does not match")
    source_keys: set[str] = set()
    for skin in skins:
        if not isinstance(skin, dict):
            raise ValueError("Genshin Skin entry must be an object")
        source_key = skin.get("source_key")
        english = skin.get("official_english_name")
        chinese = skin.get("official_simplified_chinese_name")
        if not isinstance(source_key, str) or not source_key.startswith("hoyowiki-outfit-"):
            raise ValueError("invalid Genshin Skin source key")
        if source_key in source_keys:
            raise ValueError(f"duplicate Genshin Skin source key {source_key}")
        source_keys.add(source_key)
        if not isinstance(english, str) or not english.strip():
            raise ValueError(f"missing English name for {source_key}")
        if not isinstance(chinese, str) or not chinese.strip() or "\n" in chinese or "\r" in chinese:
            raise ValueError(f"invalid Simplified Chinese name for {source_key}")
    return skins


def build_snapshot(
    catalog: Any, response: dict[str, Any], *, checked_at: str
) -> dict[str, Any]:
    skins = validate_catalog(catalog)
    names = [skin["official_simplified_chinese_name"] for skin in skins]
    lines = TRANSCRIBER.split_dictionary_response(names, response)
    return build_snapshot_from_lines(catalog, lines, checked_at=checked_at)


def build_snapshot_from_lines(
    catalog: Any, lines: list[list[dict[str, Any]]], *, checked_at: str
) -> dict[str, Any]:
    skins = validate_catalog(catalog)
    if len(lines) != len(skins):
        raise ValueError("dictionary result count does not match Skin catalog")
    aliases = []
    for skin, results in zip(skins, lines, strict=True):
        aliases.append(
            {
                "source_key": skin["source_key"],
                "official_english_name": skin["official_english_name"],
                "official_simplified_chinese_name": skin[
                    "official_simplified_chinese_name"
                ],
                "han_viet_name": WEAPON_TRANSCRIBER.format_weapon_han_viet(
                    skin["official_simplified_chinese_name"], results
                ),
            }
        )
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {"included_records": len(aliases)},
        "confidence": {"han_viet_names": "E"},
        "sources": {
            "simplified_chinese_names": "Pinned genshin-db outfit localizations",
            "han_viet_dictionary": TRANSCRIBER.DICTIONARY_URL,
        },
        "notes": [
            "Every value is a generated search alias from the Simplified Chinese Skin name; none is an official Vietnamese localization."
        ],
        "skins": aliases,
    }


def fetch_lines(
    skins: list[dict[str, Any]], *, batch_size: int, timeout: float, attempts: int
) -> list[list[dict[str, Any]]]:
    if batch_size < 1 or batch_size > 100:
        raise ValueError("dictionary batch size must be between 1 and 100")
    lines = []
    for start in range(0, len(skins), batch_size):
        batch = skins[start : start + batch_size]
        names = [skin["official_simplified_chinese_name"] for skin in batch]
        response = TRANSCRIBER._post_dictionary(
            "\n".join(names), timeout=timeout, attempts=attempts
        )
        lines.extend(TRANSCRIBER.split_dictionary_response(names, response))
    return lines


def write_snapshot(snapshot: dict[str, Any], output: Path) -> None:
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
    finally:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass


def main() -> None:
    module_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=module_root / "data/skins.json")
    parser.add_argument(
        "--output", type=Path, default=module_root / "data/skin-han-viet.json"
    )
    parser.add_argument("--dictionary-response", type=Path)
    parser.add_argument("--batch-size", type=int, default=40)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    args = parser.parse_args()
    catalog = read_json(args.catalog)
    skins = validate_catalog(catalog)
    if args.dictionary_response:
        snapshot = build_snapshot(
            catalog, read_json(args.dictionary_response), checked_at=date.today().isoformat()
        )
    else:
        snapshot = build_snapshot_from_lines(
            catalog,
            fetch_lines(
                skins,
                batch_size=args.batch_size,
                timeout=args.timeout,
                attempts=args.attempts,
            ),
            checked_at=date.today().isoformat(),
        )
    write_snapshot(snapshot, args.output.resolve())
    print(f"wrote {len(snapshot['skins'])} generated Skin Hán-Việt aliases to {args.output}")


if __name__ == "__main__":
    main()
