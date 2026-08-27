#!/usr/bin/env python3
"""Generate confidence-E Hán-Việt aliases for the Genshin Region snapshot."""

from __future__ import annotations

import argparse
from datetime import date
import importlib.util
import json
import os
from pathlib import Path
import re
import tempfile
from typing import Any
import unicodedata


CATALOG_SCHEMA = "perwiga.genshin-impact.regions.v1"
SNAPSHOT_SCHEMA = "perwiga.genshin-impact.region-han-viet.v1"
DICTIONARY_URL = "https://hvdic.thivien.net/transcript.php#trans"
REGION_FALLBACKS = {
    "峠": "đèo",  # Japanese-coined character meaning mountain pass.
    "哐": "khuông",  # Modern onomatopoeic character; phonetic fallback from kuāng.
}


def load_transcription_module() -> Any:
    path = Path(__file__).with_name("fetch-character-han-viet.py")
    spec = importlib.util.spec_from_file_location("genshin_han_viet_common", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load Hán-Việt helper {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HAN_VIET = load_transcription_module()


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def validate_catalog(catalog: Any) -> list[dict[str, Any]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unsupported Genshin Region catalog schema")
    records = catalog.get("regions")
    expected = catalog.get("catalog_scope", {}).get("included_records")
    if not isinstance(records, list) or expected != len(records):
        raise ValueError("Genshin Region catalog record count does not match")
    source_keys: set[str] = set()
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("Region catalog entry must be an object")
        source_key = record.get("source_key")
        english = record.get("official_english_name")
        chinese = record.get("official_simplified_chinese_name")
        if not isinstance(source_key, str) or not source_key.strip() or source_key in source_keys:
            raise ValueError("invalid or duplicate Region source key")
        source_keys.add(source_key)
        if not isinstance(english, str) or not english.strip():
            raise ValueError(f"missing English name for {source_key}")
        if not isinstance(chinese, str) or not chinese.strip() or "\n" in chinese:
            raise ValueError(f"missing Simplified Chinese name for {source_key}")
    return records


def dictionary_batches(names: list[str], *, batch_size: int = 40) -> list[list[str]]:
    if batch_size <= 0:
        raise ValueError("dictionary batch size must be positive")
    return [names[index : index + batch_size] for index in range(0, len(names), batch_size)]


def format_region_han_viet(name: str, results: list[dict[str, Any]]) -> str:
    tokens: list[tuple[str, bool]] = []
    source_index = 0
    for result in results:
        source = result["i"]
        alternatives = result.get("o", [])
        if not isinstance(alternatives, list):
            alternatives = []
        if result.get("t") == 3 and alternatives:
            selected = HAN_VIET.READING_OVERRIDES.get((name, source_index), alternatives[0])
            if selected not in alternatives:
                raise ValueError(f"unattested Hán-Việt override {selected!r} for {name!r}")
            tokens.append((selected[:1].upper() + selected[1:], True))
        elif result.get("t") == 3 and source in {
            **HAN_VIET.EXPLICIT_FALLBACKS,
            **REGION_FALLBACKS,
        }:
            selected = {**HAN_VIET.EXPLICIT_FALLBACKS, **REGION_FALLBACKS}[source]
            tokens.append((selected[:1].upper() + selected[1:], True))
        elif result.get("t") == 3 and all(
            unicodedata.category(character).startswith(("P", "S"))
            for character in source
        ):
            tokens.append((source, False))
        elif result.get("t") == 3:
            raise ValueError(f"no Hán-Việt reading for {source!r} in {name!r}")
        else:
            tokens.append((source, False))
        source_index += len(source)

    if "".join(result["i"] for result in results) != name:
        raise ValueError(f"dictionary result did not round-trip {name!r}")

    output = ""
    previous_han = False
    opening = {"「", "『", "“", "‘", "(", "[", "{"}
    closing = {"」", "』", "”", "’", ")", "]", "}", ",", ".", ":", ";", "!", "?"}
    for value, is_han in tokens:
        if is_han:
            if output and not output.endswith((" ", "·", "-", *opening)):
                output += " "
            output += value
        elif value in opening:
            output += value
        elif value in closing:
            output = output.rstrip() + value
        elif value in {"·", "—"}:
            output = output.rstrip() + f" {value} "
        elif value.isspace():
            if output and not output.endswith(" "):
                output += " "
        else:
            if previous_han and output and not output.endswith(" "):
                output += " "
            output += value
        previous_han = is_han
    return re.sub(r"\s+", " ", output).strip()


def unattested_sources(lines: list[list[dict[str, Any]]]) -> list[str]:
    known = {**HAN_VIET.EXPLICIT_FALLBACKS, **REGION_FALLBACKS}
    missing = {
        result["i"]
        for line in lines
        for result in line
        if result.get("t") == 3
        and not result.get("o")
        and result.get("i") not in known
        and not all(
            unicodedata.category(character).startswith(("P", "S"))
            for character in result.get("i", "")
        )
    }
    return sorted(missing)


def snapshot_from_lines(
    records: list[dict[str, Any]],
    lines: list[list[dict[str, Any]]],
    *,
    checked_at: str,
) -> dict[str, Any]:
    if len(lines) != len(records):
        raise ValueError("dictionary result count does not match Region catalog")
    aliases = [
        {
            "source_key": record["source_key"],
            "official_english_name": record["official_english_name"],
            "official_simplified_chinese_name": record[
                "official_simplified_chinese_name"
            ],
            "han_viet_name": format_region_han_viet(
                record["official_simplified_chinese_name"], line
            ),
        }
        for record, line in zip(records, lines, strict=True)
    ]
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {"included_records": len(aliases)},
        "confidence": {"han_viet_names": "E"},
        "sources": {
            "simplified_chinese_names": "Pinned genshin-db Geography localizations",
            "han_viet_dictionary": DICTIONARY_URL,
        },
        "generation": {
            "selection": "First dictionary alternative unless an explicit shared override is recorded.",
            "reading_overrides": HAN_VIET.READING_OVERRIDES,
            "explicit_fallbacks": HAN_VIET.EXPLICIT_FALLBACKS,
            "region_semantic_fallbacks": REGION_FALLBACKS,
            "region_fallback_evidence": {
                "峠": "https://en.wiktionary.org/wiki/%E5%B3%A0",
                "哐": "https://en.wiktionary.org/wiki/%E5%93%90",
            },
        },
        "notes": [
            "Every Hán-Việt value is generated and unofficial.",
            "Generated readings remain aliases and never replace official Vietnamese names.",
        ],
        "regions": aliases,
    }


def build_snapshot(
    catalog: Any, response: dict[str, Any], *, checked_at: str
) -> dict[str, Any]:
    records = validate_catalog(catalog)
    names = [record["official_simplified_chinese_name"] for record in records]
    lines = HAN_VIET.split_dictionary_response(names, response)
    return snapshot_from_lines(records, lines, checked_at=checked_at)


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
    parser.add_argument(
        "--catalog", type=Path, default=module_root / "data" / "regions.json"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=module_root / "data" / "region-han-viet.json",
    )
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--attempts", type=int, default=3)
    arguments = parser.parse_args()
    catalog = read_json(arguments.catalog)
    records = validate_catalog(catalog)
    names = [record["official_simplified_chinese_name"] for record in records]
    lines = []
    for batch in dictionary_batches(names):
        response = HAN_VIET._post_dictionary(
            "\n".join(batch),
            timeout=arguments.timeout,
            attempts=arguments.attempts,
        )
        lines.extend(HAN_VIET.split_dictionary_response(batch, response))
    missing = unattested_sources(lines)
    if missing:
        raise ValueError(f"unattested Region characters require review: {missing}")
    snapshot = snapshot_from_lines(records, lines, checked_at=arguments.checked_at)
    write_snapshot(snapshot, arguments.output.resolve())
    print(f"wrote {len(snapshot['regions'])} Hán-Việt Region aliases to {arguments.output}")


if __name__ == "__main__":
    main()
