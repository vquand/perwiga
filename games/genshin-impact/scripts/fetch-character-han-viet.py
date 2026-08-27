#!/usr/bin/env python3
"""Generate reviewable Genshin Hán-Việt aliases from the curated zh-Hant names.

The script reads the title-owned character snapshot, sends only its official
Traditional Chinese names to the Thi Viện Hán-Nôm transcription tool, validates
that the response round-trips every name, and writes a JSON snapshot atomically.
It never opens or mutates a Perwiga SQLite database.
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
from typing import Any, Callable
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode, urlparse
from urllib.request import Request, urlopen


CATALOG_SCHEMA = "perwiga.genshin-impact.characters.v1"
SNAPSHOT_SCHEMA = "perwiga.genshin-impact.character-han-viet.v1"
DICTIONARY_URL = "https://hvdic.thivien.net/transcript.php#trans"
DICTIONARY_ENDPOINT = "https://hvdic.thivien.net/transcript-query.json.php"
OFFICIAL_DIRECTORY_URL = "https://genshin.hoyoverse.com/zh-tw/character/mondstadt"
MAX_RESPONSE_BYTES = 2 * 1024 * 1024

# 菈 is used phonetically in three current character names. Thi Viện returned no
# entry when checked on 2026-08-25. Lạp is retained as an explicit, reviewable
# fallback based on the historical reading and phonetic relationship documented
# for 菈 and the dictionary-attested Hán-Việt reading of its cognate 拉.
EXPLICIT_FALLBACKS = {"菈": "lạp"}

# The converter orders alternatives by common usage. Keep any future deviations
# explicit so a refresh can never silently change a context-sensitive reading.
READING_OVERRIDES: dict[tuple[str, int], str] = {}


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def validate_catalog(catalog: Any) -> list[dict[str, Any]]:
    if not isinstance(catalog, dict) or catalog.get("schema") != CATALOG_SCHEMA:
        raise ValueError("unsupported Genshin character catalog schema")
    characters = catalog.get("characters")
    count = catalog.get("catalog_scope", {}).get("included_records")
    if not isinstance(characters, list) or count != len(characters):
        raise ValueError("Genshin character catalog record count does not match")

    source_keys: set[str] = set()
    for character in characters:
        if not isinstance(character, dict):
            raise ValueError("Genshin character catalog entry must be an object")
        source_key = character.get("source_key")
        english = character.get("official_english_name")
        chinese = character.get("official_traditional_chinese_name")
        if not isinstance(source_key, str) or not source_key.startswith(
            "hoyoverse-content-"
        ):
            raise ValueError("invalid Genshin character source key")
        if source_key in source_keys:
            raise ValueError(f"duplicate Genshin character source key {source_key}")
        source_keys.add(source_key)
        if not isinstance(english, str) or not english.strip():
            raise ValueError(f"missing English name for {source_key}")
        if not isinstance(chinese, str) or not chinese.strip():
            raise ValueError(f"missing Traditional Chinese name for {source_key}")
        if "\n" in chinese or "\r" in chinese:
            raise ValueError(f"multiline Traditional Chinese name for {source_key}")
    return characters


def dictionary_input(characters: list[dict[str, Any]]) -> str:
    return "\n".join(
        character["official_traditional_chinese_name"] for character in characters
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
            "User-Agent": "Perwiga Genshin Han-Viet curator/1.0",
        },
    )


def _post_dictionary(
    text: str, *, timeout: float, attempts: int
) -> dict[str, Any]:
    if urlparse(DICTIONARY_ENDPOINT).hostname != "hvdic.thivien.net":
        raise ValueError("untrusted Hán-Việt dictionary endpoint")
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        request = dictionary_request(text)
        try:
            with urlopen(request, timeout=timeout) as response:
                content_length = response.headers.get("Content-Length")
                if (
                    content_length is not None
                    and int(content_length) > MAX_RESPONSE_BYTES
                ):
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
            if source != "\r":
                lines.append([])
        else:
            lines[-1].append(result)
    while lines and not lines[-1]:
        lines.pop()

    reconstructed = ["".join(token["i"] for token in line) for line in lines]
    if reconstructed != names:
        raise ValueError(
            "dictionary response did not round-trip every catalog name: "
            f"expected {len(names)} lines, got {len(reconstructed)}"
        )
    return lines


def _title_reading(value: str) -> str:
    return value[:1].upper() + value[1:]


def format_han_viet(name: str, results: list[dict[str, Any]]) -> str:
    tokens: list[tuple[str, bool]] = []
    source_index = 0
    for result in results:
        source = result["i"]
        alternatives = result.get("o", [])
        if not isinstance(alternatives, list) or not all(
            isinstance(value, str) and value for value in alternatives
        ):
            alternatives = []
        if result.get("t") == 3:
            selected = READING_OVERRIDES.get((name, source_index))
            if selected is not None and selected not in alternatives:
                raise ValueError(
                    f"unattested Hán-Việt override {selected!r} for {name!r}"
                )
            if selected is None and alternatives:
                selected = alternatives[0]
            if selected is None:
                selected = EXPLICIT_FALLBACKS.get(source)
            if selected is None:
                raise ValueError(
                    f"no Hán-Việt reading for {source!r} in {name!r}"
                )
            tokens.append((_title_reading(selected), True))
        else:
            tokens.append((source, False))
        source_index += len(source)

    if "".join(result["i"] for result in results) != name:
        raise ValueError(f"dictionary result did not round-trip {name!r}")

    output = ""
    previous_han = False
    for value, is_han in tokens:
        if is_han:
            if output and not output.endswith((" ", "·", "-")):
                output += " "
            output += value
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


def build_snapshot(
    catalog: Any, response: dict[str, Any], *, checked_at: str
) -> dict[str, Any]:
    characters = validate_catalog(catalog)
    names = [character["official_traditional_chinese_name"] for character in characters]
    result_lines = split_dictionary_response(names, response)
    aliases = []
    for character, results in zip(characters, result_lines, strict=True):
        aliases.append(
            {
                "source_key": character["source_key"],
                "official_english_name": character["official_english_name"],
                "official_traditional_chinese_name": character[
                    "official_traditional_chinese_name"
                ],
                "han_viet_name": format_han_viet(
                    character["official_traditional_chinese_name"], results
                ),
            }
        )
    return {
        "schema": SNAPSHOT_SCHEMA,
        "checked_at": checked_at,
        "catalog_scope": {"included_records": len(aliases)},
        "confidence": {"han_viet_names": "E"},
        "sources": {
            "official_traditional_chinese_names": OFFICIAL_DIRECTORY_URL,
            "han_viet_dictionary": DICTIONARY_URL,
            "unattested_character_fallback": "https://en.wiktionary.org/wiki/%E8%8F%88",
            "fallback_han_viet_cognate": "https://dictionary.zim.vn/trung-viet/%E6%8B%89%E9%BB%91",
        },
        "generation": {
            "selection": "First dictionary alternative unless a context override is recorded.",
            "reading_overrides": {},
            "explicit_fallbacks": EXPLICIT_FALLBACKS,
        },
        "notes": [
            "Every Hán-Việt value is a generated search alias from the verified official Traditional Chinese localization; none is an official Vietnamese localization.",
            "Traditional Chinese is the global website localization and is not presented as the original mainland Simplified Chinese name.",
            "Ambiguous readings use the dictionary's first alternative unless this snapshot records an explicit context override.",
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


def main() -> None:
    module_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--catalog", type=Path, default=module_root / "data" / "characters.json"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=module_root / "data" / "character-han-viet.json",
    )
    parser.add_argument("--dictionary-response", type=Path)
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--attempts", type=int, default=3)
    args = parser.parse_args()

    catalog = read_json(args.catalog)
    characters = validate_catalog(catalog)
    if args.dictionary_response is None:
        response = _post_dictionary(
            dictionary_input(characters),
            timeout=args.timeout,
            attempts=args.attempts,
        )
    else:
        response = read_json(args.dictionary_response)
    snapshot = build_snapshot(catalog, response, checked_at=date.today().isoformat())
    write_json_atomic(args.output, snapshot)
    print(f"Wrote {len(snapshot['characters'])} generated Hán-Việt aliases to {args.output}")


if __name__ == "__main__":
    main()
