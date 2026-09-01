#!/usr/bin/env python3
"""Generate reviewable Hán-Việt search aliases for the HSR character snapshot.

The input name is the snapshot's official Simplified Chinese localization. The
Hán-Việt result is an additive, generated alias: it never replaces any official
English, Chinese, or Vietnamese field. Source placeholders are kept literal and
are never sent through a personal-name inference step.
"""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import re
import tempfile
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode, urlparse
from urllib.request import Request, urlopen


CATALOG_SCHEMA = "perwiga.honkai-star-rail.characters.v1"
SNAPSHOT_SCHEMA = "perwiga.honkai-star-rail.character-han-viet.v1"
STAR_RAIL_RES_REPOSITORY = "https://github.com/Mar-7th/StarRailRes"
STAR_RAIL_DATA_REPOSITORY = "https://github.com/Dimbreath/StarRailData"
STAR_RAIL_RES_COMMIT = "d226befe3db13f2ec15f4161d5f34b1b607643fe"
DICTIONARY_URL = "https://hvdic.thivien.net/transcript.php#trans"
DICTIONARY_ENDPOINT = "https://hvdic.thivien.net/transcript-query.json.php"
STAR_RAIL_SOURCE = (
    "https://raw.githubusercontent.com/Mar-7th/StarRailRes/"
    "d226befe3db13f2ec15f4161d5f34b1b607643fe/index_new/cn/characters.json"
)
TRAILBLAZER_FALLBACK_SOURCE = "https://wiki.hoyolab.com/pc/hsr/entry/880?crawler=Googlebot"
UNATTESTED_CHARACTER_SOURCE = "https://en.wiktionary.org/wiki/%E8%8F%88"
FALLBACK_COGNATE_SOURCE = "https://hvdic.thivien.net/whv/%E6%8B%89"
MAX_RESPONSE_BYTES = 2 * 1024 * 1024
PLACEHOLDER_RE = re.compile(r"\{[^{}]+\}")
HAN_RE = re.compile(r"[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]")

# HVDic returned no reading for 菈 on 2026-09-01. Keep the same explicit,
# reviewable fallback used by the Genshin Hán-Việt pipeline: Wiktionary records
# the character's historical/phonetic relationship to 拉, while HVDic attests
# 拉 as "lạp". This is not a context-confirmed official character reading.
EXPLICIT_FALLBACKS = {"菈": "lạp"}


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def validate_catalog(catalog: Any) -> list[dict[str, Any]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unsupported HSR character catalog schema")

    characters = catalog.get("characters")
    count = catalog.get("catalog_scope", {}).get("included_records")
    if not isinstance(characters, list) or count != len(characters):
        raise ValueError("HSR character catalog record count does not match")

    source_keys: set[str] = set()
    game_data_ids: set[str] = set()
    for character in characters:
        if not isinstance(character, dict):
            raise ValueError("HSR character catalog entry must be an object")

        source_key = character.get("source_key")
        game_data_id = character.get("game_data_id")
        if not isinstance(source_key, str) or not source_key.startswith(
            "starrailres-character-"
        ):
            raise ValueError("invalid HSR character source key")
        if not isinstance(game_data_id, str) or not game_data_id.strip():
            raise ValueError(f"missing game_data_id for {source_key}")
        if source_key != f"starrailres-character-{game_data_id}":
            raise ValueError(f"source key does not stably join game_data_id for {source_key}")
        if source_key in source_keys:
            raise ValueError(f"duplicate HSR character source key {source_key}")
        if game_data_id in game_data_ids:
            raise ValueError(f"duplicate HSR character game_data_id {game_data_id}")
        source_keys.add(source_key)
        game_data_ids.add(game_data_id)

        for field in (
            "official_english_name",
            "official_simplified_chinese_name",
            "official_traditional_chinese_name",
            "official_vietnamese_name",
        ):
            value = character.get(field)
            if not isinstance(value, str) or not value.strip():
                raise ValueError(f"missing {field} for {source_key}")
            if "\n" in value or "\r" in value:
                raise ValueError(f"multiline {field} for {source_key}")

    return characters


def placeholder_tokens(name: str) -> list[str]:
    return PLACEHOLDER_RE.findall(name)


def queryable_characters(characters: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        character
        for character in characters
        if not placeholder_tokens(character["official_simplified_chinese_name"])
    ]


def dictionary_input(characters: list[dict[str, Any]]) -> str:
    return "\n".join(
        character["official_simplified_chinese_name"] for character in characters
    )


def dictionary_request(text: str) -> Request:
    body = urlencode(
        {"mode": "trans", "lang": "1", "capitalize": "1", "input": text}
    ).encode("utf-8")
    return Request(
        DICTIONARY_ENDPOINT,
        data=body,
        method="POST",
        headers={
            "Accept": "application/json",
            "Content-Type": "application/x-www-form-urlencoded",
            "Referer": "https://hvdic.thivien.net/transcript.php",
            "User-Agent": "Perwiga HSR Han-Viet curator/1.0",
        },
    )


def post_dictionary(text: str, *, timeout: float, attempts: int) -> dict[str, Any]:
    if urlparse(DICTIONARY_ENDPOINT).hostname != "hvdic.thivien.net":
        raise ValueError("untrusted Hán-Việt dictionary endpoint")

    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            with urlopen(dictionary_request(text), timeout=timeout) as response:
                content_length = response.headers.get("Content-Length")
                if content_length is not None and int(content_length) > MAX_RESPONSE_BYTES:
                    raise ValueError("dictionary response exceeds the size limit")
                response_body = response.read(MAX_RESPONSE_BYTES + 1)
                if len(response_body) > MAX_RESPONSE_BYTES:
                    raise ValueError("dictionary response exceeds the size limit")
            payload = json.loads(response_body.decode("utf-8"))
            if not isinstance(payload, dict):
                raise ValueError("dictionary response must be a JSON object")
            return payload
        except (HTTPError, URLError, TimeoutError, json.JSONDecodeError, ValueError) as error:
            last_error = error
            if attempt < attempts:
                time.sleep(min(attempt * 2, 6))

    raise RuntimeError(
        f"unable to fetch Hán-Việt readings after {attempts} attempts: {last_error}"
    )


def split_dictionary_response(
    names: list[str], response: dict[str, Any]
) -> list[list[dict[str, Any]]]:
    if (
        response.get("message") != "OK"
        or response.get("mode") != "trans"
        or str(response.get("lang")) != "1"
        or not isinstance(response.get("result"), list)
    ):
        raise ValueError("dictionary response is not a successful Hán-Việt result")

    lines: list[list[dict[str, Any]]] = [[]]
    for result in response["result"]:
        if not isinstance(result, dict) or not isinstance(result.get("i"), str):
            raise ValueError("dictionary result has an invalid shape")
        source = result["i"]
        if source in {"\r", "\n", "\r\n"}:
            if lines[-1]:
                lines.append([])
        else:
            lines[-1].append(result)
    while lines and not lines[-1]:
        lines.pop()

    reconstructed = ["".join(token["i"] for token in line) for line in lines]
    if reconstructed != names:
        raise ValueError(
            "dictionary response did not round-trip every queried name: "
            f"expected {len(names)} lines, got {len(reconstructed)}"
        )
    return lines


def _title_reading(value: str) -> str:
    return value[:1].upper() + value[1:]


def format_han_viet(
    name: str, results: list[dict[str, Any]]
) -> tuple[str | None, list[dict[str, Any]], list[str], list[str]]:
    if "".join(result["i"] for result in results) != name:
        raise ValueError(f"dictionary result did not round-trip {name!r}")

    tokens: list[tuple[str, bool]] = []
    trace: list[dict[str, Any]] = []
    unresolved: list[str] = []
    fallback_tokens: list[str] = []
    for result in results:
        source = result["i"]
        alternatives = result.get("o", [])
        if not isinstance(alternatives, list) or not all(
            isinstance(value, str) and value for value in alternatives
        ):
            alternatives = []

        if result.get("t") == 3:
            selected = alternatives[0] if alternatives else None
            used_fallback = False
            if selected is None:
                selected = EXPLICIT_FALLBACKS.get(source)
                used_fallback = selected is not None
            trace.append(
                {
                    "source": source,
                    "selected": selected,
                    "alternatives": alternatives,
                    "explicit_fallback": used_fallback,
                }
            )
            if selected is None:
                unresolved.append(source)
                tokens.append((source, False))
            else:
                if used_fallback:
                    fallback_tokens.append(source)
                tokens.append((_title_reading(selected), True))
        else:
            tokens.append((source, False))

    if unresolved:
        return None, trace, unresolved, fallback_tokens

    output = ""
    previous_han = False
    for value, is_han in tokens:
        if is_han:
            if output and not output.endswith((" ", "·", "•", "—", "-", "&", "/")):
                output += " "
            output += value
        elif value in {"·", "•", "—"}:
            output = output.rstrip() + f" {value} "
        elif value in {"&", "/"}:
            output = output.rstrip() + f" {value} "
        elif value.isspace():
            if output and not output.endswith(" "):
                output += " "
        else:
            if previous_han and output and not output.endswith(" "):
                output += " "
            output += value
        previous_han = is_han

    return re.sub(r"\s+", " ", output).strip(), trace, [], fallback_tokens


def base_record(character: dict[str, Any]) -> dict[str, Any]:
    return {
        "source_key": character["source_key"],
        "game_data_id": character["game_data_id"],
        "official_english_name": character["official_english_name"],
        "official_simplified_chinese_name": character[
            "official_simplified_chinese_name"
        ],
        "official_traditional_chinese_name": character[
            "official_traditional_chinese_name"
        ],
        "official_vietnamese_name": character["official_vietnamese_name"],
        "generated_from": "official_simplified_chinese_name",
    }


def build_snapshot(
    catalog: Any,
    response: dict[str, Any],
    *,
    checked_at: str,
) -> dict[str, Any]:
    characters = validate_catalog(catalog)
    queryable = queryable_characters(characters)
    names = [character["official_simplified_chinese_name"] for character in queryable]
    result_lines = split_dictionary_response(names, response)

    aliases: list[dict[str, Any]] = []
    result_by_source_key = {
        character["source_key"]: results
        for character, results in zip(queryable, result_lines, strict=True)
    }
    for character in characters:
        record = base_record(character)
        source_name = character["official_simplified_chinese_name"]
        placeholders = placeholder_tokens(source_name)
        if placeholders:
            record.update(
                {
                    "han_viet_name": None,
                    "alias_status": "placeholder",
                    "unresolved_tokens": placeholders,
                    "display_name_fallback": "Trailblazer",
                    "display_fallback_kind": "generic_role_label",
                    "display_fallback_is_personal_name": False,
                    "display_fallback_note": (
                        "User-facing role label only; the source {NICKNAME} value is "
                        "preserved and no personal name is inferred."
                    ),
                }
            )
        elif not HAN_RE.search(source_name):
            record.update(
                {
                    "han_viet_name": None,
                    "alias_status": "source_literal",
                    "unresolved_tokens": [],
                    "source_literal_note": (
                        "The source name contains no Han characters, so no Hán-Việt "
                        "transcription was generated."
                    ),
                }
            )
        else:
            alias, trace, unresolved, fallback_tokens = format_han_viet(
                source_name, result_by_source_key[character["source_key"]]
            )
            record.update(
                {
                    "han_viet_name": alias,
                    "alias_status": (
                        "generated_fallback"
                        if alias is not None and fallback_tokens
                        else "generated"
                        if alias is not None
                        else "unresolved"
                    ),
                    "unresolved_tokens": unresolved,
                    "dictionary_readings": trace,
                    "explicit_fallback_tokens": fallback_tokens,
                }
            )
        aliases.append(record)

    source_keys = [character["source_key"] for character in characters]
    output_keys = [record["source_key"] for record in aliases]
    if source_keys != output_keys:
        raise ValueError("output source-key order or coverage does not match the catalog")

    counts = {
        "catalog_records": len(characters),
        "output_records": len(aliases),
        "queried_records": len(queryable),
        "generated_han_viet_records": sum(
            record["alias_status"] in {"generated", "generated_fallback"}
            for record in aliases
        ),
        "dictionary_only_generated_records": sum(
            record["alias_status"] == "generated" for record in aliases
        ),
        "explicit_fallback_records": sum(
            record["alias_status"] == "generated_fallback" for record in aliases
        ),
        "source_literal_records": sum(
            record["alias_status"] == "source_literal" for record in aliases
        ),
        "placeholder_records": sum(
            record["alias_status"] == "placeholder" for record in aliases
        ),
        "unresolved_records": sum(
            record["alias_status"] == "unresolved" for record in aliases
        ),
        "unique_unresolved_tokens": sorted(
            {
                token
                for record in aliases
                for token in record["unresolved_tokens"]
            }
        ),
        "unique_explicit_fallback_tokens": sorted(
            {
                token
                for record in aliases
                for token in record.get("explicit_fallback_tokens", [])
            }
        ),
    }

    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {
            "source_catalog": "games/honkai-star-rail/data/characters.json",
            "source_catalog_schema": CATALOG_SCHEMA,
            "source_name_field": "official_simplified_chinese_name",
            "included_records": len(aliases),
            "source_records": len(characters),
            "alias_records": len(aliases),
            "unique_simplified_chinese_names": len(set(names)),
            "counts": counts,
            "source_key_coverage": {
                "catalog_records": len(source_keys),
                "output_records": len(output_keys),
                "missing_source_keys": [],
                "unexpected_source_keys": [],
            },
        },
        "coverage": {
            "source_key_coverage": {
                "source_records": len(source_keys),
                "alias_records": len(output_keys),
                "matched": len(set(source_keys) & set(output_keys)),
                "missing": sorted(set(source_keys) - set(output_keys)),
                "extra": sorted(set(output_keys) - set(source_keys)),
            },
            "generated_aliases": counts["generated_han_viet_records"],
            "dictionary_only_generated_aliases": counts[
                "dictionary_only_generated_records"
            ],
            "explicit_fallback_aliases": counts["explicit_fallback_records"],
            "source_literal_records": counts["source_literal_records"],
            "placeholder_records": counts["placeholder_records"],
            "unresolved_records": counts["unresolved_records"],
            "unresolved_token_occurrences": sum(
                len(record["unresolved_tokens"]) for record in aliases
            ),
            "unresolved_tokens": counts["unique_unresolved_tokens"],
        },
        "confidence": {
            "han_viet_names": "E",
            "display_name_fallbacks": "B",
            "scale": {
                "E": (
                    "Generated dictionary reading suitable as a search alias; not "
                    "an official localization and not a semantic translation."
                ),
                "B": (
                    "Source-backed generic display label; not a personal name and "
                    "not a Hán-Việt alias."
                ),
            },
            "notes": {
                "E": (
                    "Exploratory generated search aliases. They are selected from "
                    "the dictionary's first character-level reading and are not "
                    "official Vietnamese names or human-reviewed proper-noun readings."
                ),
                "B": (
                    "Source-backed generic display label, safe for the placeholder "
                    "fallback but not a personal name and not a Hán-Việt alias."
                ),
            },
        },
        "sources": {
            "official_simplified_chinese_names": {
                "repository": STAR_RAIL_RES_REPOSITORY,
                "source_path": "index_new/cn/characters.json",
                "commit": STAR_RAIL_RES_COMMIT,
                "raw_snapshot": STAR_RAIL_SOURCE,
                "upstream_data": STAR_RAIL_DATA_REPOSITORY,
            },
            "official_traditional_chinese_names": {
                "repository": STAR_RAIL_RES_REPOSITORY,
                "source_path": "index_new/cht/characters.json",
                "commit": STAR_RAIL_RES_COMMIT,
                "upstream_data": STAR_RAIL_DATA_REPOSITORY,
            },
            "han_viet_dictionary": DICTIONARY_URL,
            "han_viet_dictionary_endpoint": DICTIONARY_ENDPOINT,
            "unattested_character_fallback": UNATTESTED_CHARACTER_SOURCE,
            "fallback_han_viet_cognate": FALLBACK_COGNATE_SOURCE,
            "placeholder_display_fallback": TRAILBLAZER_FALLBACK_SOURCE,
        },
        "generation": {
            "input_locale": "official_simplified_chinese_name",
            "method": (
                "Query official_simplified_chinese_name values with HVDic mode=trans "
                "and lang=1; select the first returned Hán-Việt alternative per Han "
                "character; retain non-Han source text and punctuation."
            ),
            "selection": (
                "First Hán-Việt alternative returned by the Thi Viện transcription "
                "endpoint for each Han character."
            ),
            "capitalization": "Capitalize the first letter of each generated syllable.",
            "punctuation": "Preserve source punctuation and separate generated syllables with spaces.",
            "reading_overrides": {},
            "placeholder_policy": (
                "Do not query or transcribe brace-delimited source placeholders. "
                "Keep each placeholder in official fields and expose it in "
                "unresolved_tokens."
            ),
            "display_fallback_policy": (
                "For {NICKNAME}, expose Trailblazer only as a documented generic "
                "role label. It is separate from official names and han_viet_name."
            ),
            "explicit_fallbacks": EXPLICIT_FALLBACKS,
            "official_fields_unchanged": True,
            "dictionary_request_records": len(queryable),
        },
        "limitations": [
            "The Hán-Việt value is a search alias, not an official Vietnamese localization.",
            "Character-level dictionary readings can be semantically or name-context wrong for transliterations and proper nouns.",
            "The first dictionary alternative is deterministic but may not be the preferred reading; alternatives remain per record in dictionary_readings.",
            "菈 is absent from the Hán-Việt dictionary response; the one explicit lạp fallback is based on the documented phonetic cognate 拉 and remains flagged as generated_fallback.",
            "Latin-only source names such as Saber and Archer have no generated Hán-Việt value.",
            "The selected StarRailRes catalog is a community-maintained game-data extract, not an official HoYoverse API response.",
        ],
        "characters": aliases,
    }


def write_json_atomic(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(payload, handle, ensure_ascii=False, indent=2)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_name, path)
    except Exception:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass
        raise


def parse_args() -> argparse.Namespace:
    module_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--catalog", type=Path, default=module_root / "data" / "characters.json"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=module_root / "data" / "character-han-viet.json",
    )
    parser.add_argument(
        "--dictionary-response",
        type=Path,
        help="Use a saved HVDic JSON response instead of making a live request",
    )
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--attempts", type=int, default=3)
    parser.add_argument("--checked-at", default=date.today().isoformat())
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    catalog = read_json(args.catalog)
    characters = validate_catalog(catalog)
    if args.dictionary_response is None:
        response = post_dictionary(
            dictionary_input(queryable_characters(characters)),
            timeout=args.timeout,
            attempts=args.attempts,
        )
    else:
        response = read_json(args.dictionary_response)
    snapshot = build_snapshot(catalog, response, checked_at=args.checked_at)
    write_json_atomic(args.output, snapshot)
    print(
        f"Wrote {len(snapshot['characters'])} HSR Hán-Việt records to {args.output}; "
        f"generated={snapshot['catalog_scope']['counts']['generated_han_viet_records']}, "
        f"fallbacks={snapshot['catalog_scope']['counts']['explicit_fallback_records']}, "
        f"placeholders={snapshot['catalog_scope']['counts']['placeholder_records']}"
    )


if __name__ == "__main__":
    main()
