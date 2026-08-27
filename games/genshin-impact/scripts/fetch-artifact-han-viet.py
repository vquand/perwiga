#!/usr/bin/env python3
"""Generate confidence-E Hán-Việt aliases for Genshin Artifact sets and pieces."""

from __future__ import annotations

import argparse
from datetime import date
import importlib.util
import json
import os
from pathlib import Path
import tempfile
from typing import Any


CATALOG_SCHEMA = "perwiga.genshin-impact.artifacts.v1"
SNAPSHOT_SCHEMA = "perwiga.genshin-impact.artifact-han-viet.v1"
COLLECTIONS = ("artifact_sets", "artifact_pieces")


def load_script(filename: str, module_name: str):
    path = Path(__file__).with_name(filename)
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


TRANSCRIBER = load_script("fetch-character-han-viet.py", "genshin_artifact_transcriber")
FORMATTER = load_script("fetch-weapon-han-viet.py", "genshin_artifact_formatter")


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def validate_catalog(catalog: Any) -> list[tuple[str, dict[str, Any]]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unsupported Genshin Artifact catalog schema")
    records: list[tuple[str, dict[str, Any]]] = []
    source_keys: set[str] = set()
    scope = catalog.get("catalog_scope", {})
    for collection in COLLECTIONS:
        entries = catalog.get(collection)
        if not isinstance(entries, list) or scope.get(collection) != len(entries):
            raise ValueError(f"Genshin Artifact {collection} count does not match")
        for entry in entries:
            if not isinstance(entry, dict):
                raise ValueError(f"Genshin Artifact {collection} entry must be an object")
            source_key = entry.get("source_key")
            english = entry.get("official_english_name")
            chinese = entry.get("official_simplified_chinese_name")
            expected = "genshin-data-artifact-set-" if collection == "artifact_sets" else "genshin-data-artifact-piece-"
            if not isinstance(source_key, str) or not source_key.startswith(expected):
                raise ValueError(f"invalid Genshin Artifact source key: {source_key!r}")
            if source_key in source_keys:
                raise ValueError(f"duplicate Genshin Artifact source key {source_key}")
            if not isinstance(english, str) or not english.strip():
                raise ValueError(f"missing English name for {source_key}")
            if not isinstance(chinese, str) or not chinese.strip() or "\n" in chinese or "\r" in chinese:
                raise ValueError(f"invalid Simplified Chinese name for {source_key}")
            source_keys.add(source_key)
            records.append((collection, entry))
    return records


def build_snapshot_from_lines(catalog: Any, lines: list[list[dict[str, Any]]], *, checked_at: str) -> dict[str, Any]:
    records = validate_catalog(catalog)
    if len(lines) != len(records):
        raise ValueError("dictionary result count does not match Artifact catalog")
    output: dict[str, list[dict[str, str]]] = {collection: [] for collection in COLLECTIONS}
    for (collection, entry), results in zip(records, lines, strict=True):
        output[collection].append({
            "source_key": entry["source_key"],
            "official_english_name": entry["official_english_name"],
            "official_simplified_chinese_name": entry["official_simplified_chinese_name"],
            "han_viet_name": FORMATTER.format_weapon_han_viet(entry["official_simplified_chinese_name"], results),
        })
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {collection: len(output[collection]) for collection in COLLECTIONS},
        "confidence": {"han_viet_names": "E"},
        "sources": {
            "simplified_chinese_names": "Pinned genshin-db Artifact localizations",
            "han_viet_dictionary": TRANSCRIBER.DICTIONARY_URL,
        },
        "notes": ["Every value is a generated search alias from the Simplified Chinese Artifact name; none is an official Vietnamese localization."],
        **output,
    }


def build_snapshot(catalog: Any, response: dict[str, Any], *, checked_at: str) -> dict[str, Any]:
    records = validate_catalog(catalog)
    names = [entry["official_simplified_chinese_name"] for _, entry in records]
    return build_snapshot_from_lines(catalog, TRANSCRIBER.split_dictionary_response(names, response), checked_at=checked_at)


def fetch_lines(records: list[tuple[str, dict[str, Any]]], *, batch_size: int, timeout: float, attempts: int) -> list[list[dict[str, Any]]]:
    if batch_size < 1 or batch_size > 100:
        raise ValueError("dictionary batch size must be between 1 and 100")
    lines = []
    for start in range(0, len(records), batch_size):
        batch = records[start:start + batch_size]
        names = [entry["official_simplified_chinese_name"] for _, entry in batch]
        response = TRANSCRIBER._post_dictionary("\n".join(names), timeout=timeout, attempts=attempts)
        lines.extend(TRANSCRIBER.split_dictionary_response(names, response))
    return lines


def write_snapshot(snapshot: dict[str, Any], output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{output.name}.", suffix=".tmp", dir=output.parent)
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
    parser.add_argument("--catalog", type=Path, default=module_root / "data/artifacts.json")
    parser.add_argument("--output", type=Path, default=module_root / "data/artifact-han-viet.json")
    parser.add_argument("--dictionary-response", type=Path)
    parser.add_argument("--batch-size", type=int, default=40)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    args = parser.parse_args()
    catalog = read_json(args.catalog)
    records = validate_catalog(catalog)
    snapshot = build_snapshot(catalog, read_json(args.dictionary_response), checked_at=date.today().isoformat()) if args.dictionary_response else build_snapshot_from_lines(catalog, fetch_lines(records, batch_size=args.batch_size, timeout=args.timeout, attempts=args.attempts), checked_at=date.today().isoformat())
    write_snapshot(snapshot, args.output.resolve())
    print(f"wrote {len(records)} generated Artifact Hán-Việt aliases to {args.output}")


if __name__ == "__main__":
    main()
