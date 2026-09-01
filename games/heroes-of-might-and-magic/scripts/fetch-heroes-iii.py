#!/usr/bin/env python3
"""Build a reviewable Heroes III Complete catalog without opening SQLite.

VCMI supplies pinned structural identifiers and edition scope. The Heroes 3
Wiki MediaWiki API supplies verified English labels and factual table fields.
The generated snapshot is an offline review boundary, not a live application
integration.
"""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
from typing import Any


SCHEMA = "perwiga.heroes-of-might-and-magic.heroes-iii-complete.v1"
VCMI_SOURCE_URL = "https://github.com/vcmi/vcmi"
PINNED_VCMI_COMMIT = "93dd4eeac1a9f41ff8c7f090c1d323633a32d69e"
WIKI_API_URL = "https://heroes.thelazy.net/api.php"
WIKI_BASE_URL = "https://heroes.thelazy.net/index.php/"
EXPECTED_COUNTS = {
    "town": 9,
    "hero": 156,
    "creature": 141,
    "artifact": 141,
    "spell": 70,
}
COMPLETE_EDITIONS = frozenset({"roe", "ab", "sod"})


def parse_jsonc(text: str) -> Any:
    """Parse VCMI's JSON-with-comments format without touching string data."""

    output: list[str] = []
    index = 0
    in_string = False
    escaped = False
    while index < len(text):
        character = text[index]
        if in_string:
            output.append(character)
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
            index += 1
            continue
        if character == '"':
            in_string = True
            output.append(character)
            index += 1
            continue
        marker = text[index : index + 2]
        if marker == "//":
            newline = text.find("\n", index + 2)
            if newline < 0:
                break
            index = newline
            continue
        if marker == "/*":
            end = text.find("*/", index + 2)
            if end < 0:
                raise ValueError("unterminated JSON block comment")
            index = end + 2
            continue
        output.append(character)
        index += 1
    try:
        return json.loads("".join(output))
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSONC source: {error}") from error


def read_jsonc(path: Path) -> Any:
    try:
        return parse_jsonc(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ValueError(f"unable to read VCMI source {path}: {error}") from error


def _template_call(text: str, template_name: str) -> str:
    pattern = re.compile(r"\{\{\s*" + re.escape(template_name) + r"(?=\s|\||\}\})", re.I)
    match = pattern.search(text)
    if match is None:
        raise ValueError(f"missing {{{{{template_name}}}}} template")
    start = match.start()
    index = start + 2
    depth = 1
    while index < len(text) and depth:
        marker = text[index : index + 2]
        if marker == "{{":
            depth += 1
            index += 2
        elif marker == "}}":
            depth -= 1
            index += 2
        else:
            index += 1
    if depth:
        raise ValueError(f"unterminated {{{{{template_name}}}}} template")
    return text[start + 2 : index - 2]


def _split_top_level(value: str) -> list[str]:
    fields: list[str] = []
    start = 0
    index = 0
    template_depth = 0
    link_depth = 0
    while index < len(value):
        marker = value[index : index + 2]
        if marker == "{{":
            template_depth += 1
            index += 2
            continue
        if marker == "}}":
            template_depth -= 1
            index += 2
            continue
        if marker == "[[":
            link_depth += 1
            index += 2
            continue
        if marker == "]]":
            link_depth -= 1
            index += 2
            continue
        if value[index] == "|" and template_depth == 0 and link_depth == 0:
            fields.append(value[start:index])
            start = index + 1
        index += 1
    fields.append(value[start:])
    return fields


def template_fields(text: str, template_name: str) -> dict[str, str]:
    call = _template_call(text, template_name)
    parts = _split_top_level(call)
    fields: dict[str, str] = {}
    position = 1
    for part in parts[1:]:
        key, separator, value = part.partition("=")
        if separator and re.fullmatch(r"\s*[A-Za-z][A-Za-z0-9_]*\s*", key):
            fields[key.strip().lower()] = value.strip()
        else:
            fields[str(position)] = part.strip()
            position += 1
    return fields


def _replace_simple_template(match: re.Match[str]) -> str:
    parts = _split_top_level(match.group(1))
    positional = [part.strip() for part in parts[1:] if "=" not in part]
    if not positional:
        return ""
    return positional[-1]


def clean_wikitext(value: str) -> str:
    value = re.sub(r"<ref\b[^>]*>.*?</ref>", "", value, flags=re.I | re.S)
    value = re.sub(r"<ref\b[^>]*/>", "", value, flags=re.I)
    value = re.sub(
        r"\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|([^\]]+))?\]\]",
        lambda match: (match.group(2) or match.group(1)).strip(),
        value,
    )
    simple_template = re.compile(r"\{\{([^{}]*)\}\}")
    previous = None
    while value != previous and simple_template.search(value):
        previous = value
        value = simple_template.sub(_replace_simple_template, value)
    value = re.sub(r"<[^>]+>", " ", value)
    value = value.replace("'''", "").replace("''", "")
    value = value.replace("&times;", "×").replace("&nbsp;", " ")
    return re.sub(r"\s+", " ", value).strip()


def _template_calls(text: str, template_name: str) -> list[str]:
    calls: list[str] = []
    position = 0
    pattern = re.compile(r"\{\{\s*" + re.escape(template_name) + r"(?=\s|\||\}\})", re.I)
    while True:
        match = pattern.search(text, position)
        if match is None:
            return calls
        start = match.start()
        index = start + 2
        depth = 1
        while index < len(text) and depth:
            marker = text[index : index + 2]
            if marker == "{{":
                depth += 1
                index += 2
            elif marker == "}}":
                depth -= 1
                index += 2
            else:
                index += 1
        if depth:
            raise ValueError(f"unterminated {{{{{template_name}}}}} template")
        calls.append(text[start + 2 : index - 2])
        position = index


def parse_artifact_rows(text: str) -> list[dict[str, str | None]]:
    rows: list[dict[str, str | None]] = []
    for call in _template_calls(text, "Ar"):
        fields = _split_top_level(call)
        if len(fields) < 7:
            raise ValueError("artifact table row has too few fields")
        edition = fields[2].strip().lower()
        if edition not in COMPLETE_EDITIONS:
            continue
        rows.append(
            {
                "name": clean_wikitext(fields[1]),
                "edition": edition,
                "slot": clean_wikitext(fields[3]),
                "class": clean_wikitext(fields[4]),
                "value": clean_wikitext(fields[5]) or None,
                "effect": clean_wikitext(fields[6]) or None,
                "components": clean_wikitext(fields[7]) if len(fields) > 7 else None,
            }
        )
    return rows


def write_snapshot(snapshot: dict[str, Any], output: Path, *, allow_shrink: bool) -> None:
    if output.exists() and not allow_shrink:
        try:
            current = json.loads(output.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise ValueError(f"unable to inspect existing snapshot {output}: {error}") from error
        old_count = current.get("catalog_scope", {}).get("included_records")
        new_count = snapshot.get("catalog_scope", {}).get("included_records")
        if isinstance(old_count, int) and isinstance(new_count, int) and new_count < old_count:
            raise ValueError(
                f"refusing to shrink Heroes III snapshot from {old_count} to {new_count} records"
            )
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


def checkout_commit(source_root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(source_root), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError("--source-root must be a Git checkout") from error


def build_snapshot(source_root: Path, *, checked_at: str) -> dict[str, Any]:
    raise NotImplementedError("catalog crawl is implemented in the next increment")


def main() -> None:
    module_dir = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--output", type=Path, default=module_dir / "data/heroes-iii.json")
    parser.add_argument("--allow-shrink", action="store_true")
    args = parser.parse_args()
    snapshot = build_snapshot(args.source_root.resolve(), checked_at=args.checked_at)
    write_snapshot(snapshot, args.output.resolve(), allow_shrink=args.allow_shrink)
    print(
        f"wrote {snapshot['catalog_scope']['included_records']} Heroes III records "
        f"to {args.output}"
    )


if __name__ == "__main__":
    main()
