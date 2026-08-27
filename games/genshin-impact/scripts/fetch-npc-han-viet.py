#!/usr/bin/env python3
"""Generate confidence-E Hán-Việt aliases for NPCs with source Chinese names."""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import re


ROOT = Path(__file__).parents[1]
SPEC = importlib.util.spec_from_file_location("character_han_viet", Path(__file__).with_name("fetch-character-han-viet.py"))
HAN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HAN)

SNAPSHOT_SCHEMA = "perwiga.genshin-impact.npc-han-viet.v1"
CATALOG_SCHEMA = "perwiga.genshin-impact.npcs.v1"
BATCH_SIZE = 80


def validate_catalog(catalog: dict) -> list[dict]:
    if catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unsupported Genshin NPC catalog schema")
    records = catalog.get("npcs")
    if not isinstance(records, list) or catalog.get("catalog_scope", {}).get("included_records") != len(records):
        raise ValueError("Genshin NPC catalog record count does not match")
    return records


def build_snapshot(catalog: dict, responses: list[dict], *, checked_at: str) -> dict:
    records = [record for record in validate_catalog(catalog) if record.get("official_simplified_chinese_name", "").strip()]
    names = [record["official_simplified_chinese_name"] for record in records]
    if len(responses) != (len(names) + BATCH_SIZE - 1) // BATCH_SIZE:
        raise ValueError("unexpected number of dictionary batches")
    lines = []
    for batch_names, response in zip((names[i:i + BATCH_SIZE] for i in range(0, len(names), BATCH_SIZE)), responses, strict=True):
        lines.extend(HAN.split_dictionary_response(batch_names, response))
    aliases = []
    for record, result in zip(records, lines, strict=True):
        aliases.append({
            "source_key": record["source_key"],
            "official_english_name": record["official_english_name"],
            "official_simplified_chinese_name": record["official_simplified_chinese_name"],
            "han_viet_name": HAN.format_han_viet(record["official_simplified_chinese_name"], result),
        })
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {"catalog_records": len(records), "aliases": len(aliases), "missing_chinese_names": len(validate_catalog(catalog)) - len(records)},
        "confidence": {"han_viet_names": "E"},
        "sources": {"npc_catalog": "https://genshin-impact.fandom.com/wiki/NPC/List", "han_viet_dictionary": HAN.DICTIONARY_URL},
        "notes": ["Every Hán-Việt value is generated and unofficial.", "NPC pages without a Simplified Chinese name remain explicit catalog gaps and are not silently guessed."],
        "npcs": aliases,
    }


def build_snapshot_from_mapping(catalog: dict, mapping: dict[str, list[str]], *, checked_at: str) -> dict:
    records = validate_catalog(catalog)
    aliases = []
    unresolved = {}
    for record in records:
        chinese = record.get("official_simplified_chinese_name", "").strip()
        if not chinese:
            continue
        words = []
        missing = []
        for char in chinese:
            readings = mapping.get(char, [])
            if readings:
                words.append(readings[0][:1].upper() + readings[0][1:])
            elif re.match(r"[\u3400-\u9fff]", char):
                missing.append(char)
            else:
                words.append(char)
        if missing:
            unresolved[record["source_key"]] = sorted(set(missing))
            continue
        aliases.append({
            "source_key": record["source_key"],
            "official_english_name": record["official_english_name"],
            "official_simplified_chinese_name": chinese,
            "han_viet_name": " ".join(words).strip(),
        })
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {"catalog_records": len(records), "aliases": len(aliases), "missing_chinese_names": sum(not record.get("official_simplified_chinese_name", "").strip() for record in records), "unresolved_readings": len(unresolved)},
        "confidence": {"han_viet_names": "E"},
        "sources": {"npc_catalog": "https://genshin-impact.fandom.com/wiki/NPC/List", "han_viet_dictionary": "https://github.com/ph0ngp/hanviet-pinyin-words"},
        "generation": {"selection": "First local Sino-Vietnamese character reading; unresolved characters remain explicit.", "unresolved": unresolved},
        "notes": ["Every Hán-Việt value is generated and unofficial.", "Character-by-character fallback is less context-sensitive than Thi Viện and should be reviewed before relying on it as a canonical reading."],
        "npcs": aliases,
    }


def load_mapping(path: Path) -> dict[str, list[str]]:
    text = path.read_text(encoding="utf-8")
    payload = text.split("=", 1)[1].strip()
    if payload.endswith(";"):
        payload = payload[:-1]
    data = json.loads(payload)
    mapping = {}
    for char, readings in data.items():
        values = []
        for candidates in readings.values():
            if isinstance(candidates, list):
                values.extend(value for value in candidates if isinstance(value, str) and value)
        if values:
            mapping[char] = values
    return mapping


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--catalog", type=Path, default=ROOT / "data/npcs.json")
    parser.add_argument("--output", type=Path, default=ROOT / "data/npc-han-viet.json")
    parser.add_argument("--checked-at", required=True)
    parser.add_argument("--mapping", type=Path, help="local hanvietData.js from hanviet-pinyin-words")
    args = parser.parse_args()
    catalog = json.loads(args.catalog.read_text(encoding="utf-8"))
    records = [record for record in validate_catalog(catalog) if record.get("official_simplified_chinese_name", "").strip()]
    if args.mapping:
        snapshot = build_snapshot_from_mapping(catalog, load_mapping(args.mapping), checked_at=args.checked_at)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        temporary = args.output.with_suffix(args.output.suffix + ".tmp")
        temporary.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        temporary.replace(args.output)
        print(f"wrote {snapshot['catalog_scope']['aliases']} NPC Hán-Việt aliases; {snapshot['catalog_scope']['unresolved_readings']} unresolved readings")
        return 0
    responses = []
    for offset in range(0, len(records), BATCH_SIZE):
        batch = records[offset:offset + BATCH_SIZE]
        responses.append(HAN._post_dictionary("\n".join(record["official_simplified_chinese_name"] for record in batch), timeout=30, attempts=4))
    snapshot = build_snapshot(catalog, responses, checked_at=args.checked_at)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(args.output)
    print(f"wrote {snapshot['catalog_scope']['aliases']} NPC Hán-Việt aliases; {snapshot['catalog_scope']['missing_chinese_names']} Chinese-name gaps")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
