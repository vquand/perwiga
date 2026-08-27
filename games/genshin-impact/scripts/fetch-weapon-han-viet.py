#!/usr/bin/env python3
"""Generate reviewable Hán-Việt aliases for curated Genshin weapon names.

The input is the weapon snapshot's Simplified Chinese original names. Generated
readings remain separate confidence-E aliases and never replace official names.
This script never opens or mutates SQLite.
"""

from __future__ import annotations

import argparse
from datetime import date
import importlib.util
import json
from pathlib import Path
from typing import Any, Callable


CATALOG_SCHEMA = "perwiga.genshin-impact.weapons.v1"
SNAPSHOT_SCHEMA = "perwiga.genshin-impact.weapon-han-viet.v1"
GENSHIN_DB_URL = "https://github.com/theBowja/genshin-db"
EXPLICIT_FALLBACKS = {"鹮": "hoàn"}
FALLBACK_SOURCE_URL = "https://js.vnu.edu.vn/FS/article/download/4175/3893"


def _load_transcriber():
    path = Path(__file__).with_name("fetch-character-han-viet.py")
    spec = importlib.util.spec_from_file_location("genshin_han_viet_transcriber", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load shared transcriber {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


TRANSCRIBER = _load_transcriber()


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def validate_catalog(catalog: Any) -> list[dict[str, Any]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unsupported Genshin weapon catalog schema")
    weapons = catalog.get("weapons")
    count = catalog.get("catalog_scope", {}).get("included_records")
    if not isinstance(weapons, list) or count != len(weapons):
        raise ValueError("Genshin weapon catalog record count does not match")

    source_keys: set[str] = set()
    for weapon in weapons:
        if not isinstance(weapon, dict):
            raise ValueError("Genshin weapon catalog entry must be an object")
        source_key = weapon.get("source_key")
        english = weapon.get("official_english_name")
        chinese = weapon.get("official_simplified_chinese_name")
        if not isinstance(source_key, str) or not source_key.startswith(
            "genshin-data-weapon-"
        ):
            raise ValueError("invalid Genshin weapon source key")
        if source_key in source_keys:
            raise ValueError(f"duplicate Genshin weapon source key {source_key}")
        source_keys.add(source_key)
        if not isinstance(english, str) or not english.strip():
            raise ValueError(f"missing English name for {source_key}")
        if not isinstance(chinese, str) or not chinese.strip():
            raise ValueError(f"missing Simplified Chinese name for {source_key}")
        if "\n" in chinese or "\r" in chinese:
            raise ValueError(f"multiline Simplified Chinese name for {source_key}")
    return weapons


def dictionary_input(weapons: list[dict[str, Any]]) -> str:
    return "\n".join(
        weapon["official_simplified_chinese_name"] for weapon in weapons
    )


def fetch_result_lines(
    weapons: list[dict[str, Any]],
    *,
    batch_size: int,
    timeout: float,
    attempts: int,
    post_dictionary: Callable[..., dict[str, Any]] = TRANSCRIBER._post_dictionary,
) -> list[list[dict[str, Any]]]:
    if batch_size < 1 or batch_size > 100:
        raise ValueError("dictionary batch size must be between 1 and 100")
    result_lines: list[list[dict[str, Any]]] = []
    for start in range(0, len(weapons), batch_size):
        batch = weapons[start : start + batch_size]
        names = [weapon["official_simplified_chinese_name"] for weapon in batch]
        response = post_dictionary(
            "\n".join(names), timeout=timeout, attempts=attempts
        )
        result_lines.extend(TRANSCRIBER.split_dictionary_response(names, response))
    if len(result_lines) != len(weapons):
        raise ValueError("dictionary batches did not cover every weapon")
    return result_lines


def _is_cjk_ideograph(value: str) -> bool:
    return any(
        "\u3400" <= character <= "\u4dbf"
        or "\u4e00" <= character <= "\u9fff"
        or "\uf900" <= character <= "\ufaff"
        for character in value
    )


def format_weapon_han_viet(
    name: str, results: list[dict[str, Any]]
) -> str:
    normalized = []
    for result in results:
        token = dict(result)
        alternatives = token.get("o")
        if (
            token.get("t") == 3
            and (not isinstance(alternatives, list) or not alternatives)
            and not _is_cjk_ideograph(token["i"])
        ):
            token["t"] = 0
        elif token.get("t") == 3 and (
            not isinstance(alternatives, list) or not alternatives
        ):
            fallback = EXPLICIT_FALLBACKS.get(token["i"])
            if fallback is not None:
                token["o"] = [fallback]
        normalized.append(token)
    return TRANSCRIBER.format_han_viet(name, normalized).strip(
        "「」『』《》〈〉 "
    )


def build_snapshot_from_lines(
    catalog: Any, result_lines: list[list[dict[str, Any]]], *, checked_at: str
) -> dict[str, Any]:
    weapons = validate_catalog(catalog)
    if len(result_lines) != len(weapons):
        raise ValueError("dictionary result count does not match weapon catalog")
    return _snapshot_payload(weapons, result_lines, checked_at=checked_at)


def build_snapshot(
    catalog: Any, response: dict[str, Any], *, checked_at: str
) -> dict[str, Any]:
    weapons = validate_catalog(catalog)
    names = [weapon["official_simplified_chinese_name"] for weapon in weapons]
    result_lines = TRANSCRIBER.split_dictionary_response(names, response)
    return build_snapshot_from_lines(catalog, result_lines, checked_at=checked_at)


def _snapshot_payload(
    weapons: list[dict[str, Any]],
    result_lines: list[list[dict[str, Any]]],
    *,
    checked_at: str,
) -> dict[str, Any]:
    aliases = []
    for weapon, results in zip(weapons, result_lines, strict=True):
        aliases.append(
            {
                "source_key": weapon["source_key"],
                "official_english_name": weapon["official_english_name"],
                "official_simplified_chinese_name": weapon[
                    "official_simplified_chinese_name"
                ],
                "han_viet_name": format_weapon_han_viet(
                    weapon["official_simplified_chinese_name"], results
                ),
            }
        )
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {"included_records": len(aliases)},
        "confidence": {"han_viet_names": "E"},
        "sources": {
            "simplified_chinese_names": GENSHIN_DB_URL,
            "han_viet_dictionary": TRANSCRIBER.DICTIONARY_URL,
            "weapon_fallback_evidence": FALLBACK_SOURCE_URL,
        },
        "generation": {
            "selection": "First dictionary alternative unless a context override is recorded.",
            "reading_overrides": TRANSCRIBER.READING_OVERRIDES,
            "explicit_fallbacks": EXPLICIT_FALLBACKS,
        },
        "notes": [
            "Every Hán-Việt value is a generated search alias from the extracted Simplified Chinese original name; none is an official Vietnamese localization.",
            "Ambiguous readings use the dictionary's first alternative unless an explicit context override is recorded.",
        ],
        "weapons": aliases,
    }


def main() -> None:
    module_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--catalog", type=Path, default=module_root / "data" / "weapons.json"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=module_root / "data" / "weapon-han-viet.json",
    )
    parser.add_argument("--dictionary-response", type=Path)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--batch-size", type=int, default=60)
    args = parser.parse_args()

    catalog = read_json(args.catalog)
    weapons = validate_catalog(catalog)
    if args.dictionary_response is None:
        result_lines = fetch_result_lines(
            weapons,
            batch_size=args.batch_size,
            timeout=args.timeout,
            attempts=args.attempts,
        )
        snapshot = build_snapshot_from_lines(
            catalog, result_lines, checked_at=date.today().isoformat()
        )
    else:
        response = read_json(args.dictionary_response)
        snapshot = build_snapshot(
            catalog, response, checked_at=date.today().isoformat()
        )
    TRANSCRIBER.write_json_atomic(args.output, snapshot)
    print(
        f"Wrote {len(snapshot['weapons'])} generated Hán-Việt aliases to {args.output}"
    )


if __name__ == "__main__":
    main()
