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
import time
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen


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
BASE_FACTIONS = (
    "castle",
    "rampart",
    "tower",
    "inferno",
    "necropolis",
    "dungeon",
    "stronghold",
    "fortress",
    "conflux",
)
CLASS_TO_FACTION = {
    "knight": "castle",
    "cleric": "castle",
    "ranger": "rampart",
    "druid": "rampart",
    "alchemist": "tower",
    "wizard": "tower",
    "demoniac": "inferno",
    "heretic": "inferno",
    "deathknight": "necropolis",
    "necromancer": "necropolis",
    "overlord": "dungeon",
    "warlock": "dungeon",
    "barbarian": "stronghold",
    "battlemage": "stronghold",
    "beastmaster": "fortress",
    "witch": "fortress",
    "planeswalker": "conflux",
    "elementalist": "conflux",
}
HERO_TITLE_OVERRIDES = {"undeadHaart": "Lord Haart the Death Knight"}
CREATURE_TITLE_OVERRIDES = {
    "efreet": "Efreeti",
    "zombieLord": "Zombie",
    "goblinWolfRider": "Wolf Rider",
    "hobgoblinWolfRider": "Wolf Raider",
    "cyclop": "Cyclops",
    "cyclopKing": "Cyclops King",
    "fireDragonFly": "Dragon Fly",
    "fairieDragon": "Faerie Dragon",
}
SPELL_TITLE_OVERRIDES = {
    "inferno": "Inferno (spell)",
    "protectAir": "Protection from Air",
    "protectFire": "Protection from Fire",
    "protectWater": "Protection from Water",
    "protectEarth": "Protection from Earth",
    "titanBolt": "Titan's Lightning Bolt",
    "fireElemental": "Summon Fire Elemental",
    "earthElemental": "Summon Earth Elemental",
    "waterElemental": "Summon Water Elemental",
    "airElemental": "Summon Air Elemental",
}
ARTIFACT_TITLE_OVERRIDES = {
    "centaurAxe": "Centaur's Axe",
    "tomeOfFireMagic": "Tome of Fire",
    "tomeOfAirMagic": "Tome of Air",
    "tomeOfWaterMagic": "Tome of Water",
    "tomeOfEarthMagic": "Tome of Earth",
}
ARTIFACT_TABLE_TITLES = (
    "Template:Artifact table - Weapon",
    "Template:Artifact table - Shield",
    "Template:Artifact table - Torso",
    "Template:Artifact table - Helmet",
    "Template:Artifact table - Feet",
    "Template:Artifact table - Cape",
    "Template:Artifact table - Necklace",
    "Template:Artifact table - Ring",
    "Template:Artifact table - Miscellaneous",
)
SPECIAL_ARTIFACT_SOURCES = {
    "spellBook": ("Spell Book", None),
    "spellScroll": ("Spell scroll", None),
    "grail": ("Grail", None),
    "catapult": ("War Machine", "Catapult"),
    "ballista": ("War Machine", "Ballista"),
    "ammoCart": ("War Machine", "Ammo Cart"),
    "firstAidTent": ("War Machine", "First Aid Tent"),
}
SPECIAL_ARTIFACT_NAMES = {
    "spellBook": "Spell Book",
    "spellScroll": "Spell Scroll",
    "grail": "Grail",
    "catapult": "Catapult",
    "ballista": "Ballista",
    "ammoCart": "Ammo Cart",
    "firstAidTent": "First Aid Tent",
}
SPELL_CONFIG_FILES = ("adventure.json", "offensive.json", "other.json", "timed.json")


def normalize_identifier(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.casefold())


def wiki_url(title: str, anchor: str | None = None) -> str:
    url = WIKI_BASE_URL + quote(title.replace(" ", "_"), safe="()'/-")
    return f"{url}#{quote(anchor.replace(' ', '_'))}" if anchor else url


def title_from_key(value: str) -> str:
    words = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", value).split()
    return " ".join(word[:1].upper() + word[1:] for word in words)


def wiki_page_content(page: dict[str, Any]) -> str:
    revisions = page.get("revisions")
    if not isinstance(revisions, list) or not revisions:
        raise ValueError(f"wiki page {page.get('title')!r} has no revision")
    revision = revisions[0]
    main = revision.get("slots", {}).get("main", {})
    content = main.get("*") if isinstance(main, dict) else None
    if content is None and isinstance(main, dict):
        content = main.get("content")
    if not isinstance(content, str):
        raise ValueError(f"wiki page {page.get('title')!r} has no main-slot content")
    return content


def wiki_page_source(page: dict[str, Any], *, anchor: str | None = None) -> dict[str, Any]:
    revisions = page.get("revisions")
    if not isinstance(revisions, list) or not revisions:
        raise ValueError(f"wiki page {page.get('title')!r} has no revision metadata")
    revision = revisions[0]
    title = page.get("title")
    page_id = page.get("pageid")
    revision_id = revision.get("revid")
    timestamp = revision.get("timestamp")
    if not isinstance(title, str) or not isinstance(page_id, int):
        raise ValueError("wiki page has invalid identity metadata")
    if not isinstance(revision_id, int) or not isinstance(timestamp, str):
        raise ValueError(f"wiki page {title!r} has invalid revision metadata")
    return {
        "title": title,
        "url": wiki_url(title, anchor),
        "page_id": page_id,
        "revision_id": revision_id,
        "revision_timestamp": timestamp,
    }


def facts_from_template(
    text: str,
    template_name: str,
    selections: tuple[tuple[str, str | tuple[str, ...]], ...],
) -> list[dict[str, str]]:
    fields = template_fields(text, template_name)
    facts: list[dict[str, str]] = []
    for label, keys in selections:
        candidates = (keys,) if isinstance(keys, str) else keys
        value = next((fields.get(key.lower()) for key in candidates if fields.get(key.lower())), None)
        if value is None:
            continue
        cleaned = clean_wikitext(value)
        if cleaned:
            facts.append({"label": label, "value": cleaned})
    return facts


def validate_catalog_records(
    records: list[dict[str, Any]], expected_counts: dict[str, int]
) -> None:
    source_keys: set[str] = set()
    counts = {entity_type: 0 for entity_type in expected_counts}
    for record in records:
        source_key = record.get("source_key")
        entity_type = record.get("entity_type")
        if not isinstance(source_key, str) or not source_key:
            raise ValueError("catalog record has an invalid source key")
        if source_key in source_keys:
            raise ValueError(f"duplicate source key: {source_key}")
        source_keys.add(source_key)
        if entity_type not in expected_counts:
            raise ValueError(f"catalog record has unsupported entity type: {entity_type!r}")
        counts[entity_type] += 1
    for entity_type, expected in expected_counts.items():
        actual = counts[entity_type]
        if actual != expected:
            raise ValueError(
                f"{entity_type} catalog has {actual} records; expected {expected}"
            )


class WikiClient:
    def _request(self, params: dict[str, str]) -> dict[str, Any]:
        request = Request(
            WIKI_API_URL + "?" + urlencode(params),
            headers={"User-Agent": "Perwiga Heroes III catalog (private local wiki)"},
        )
        last_error: Exception | None = None
        for attempt in range(1, 5):
            try:
                with urlopen(request, timeout=60) as response:
                    if response.geturl().split("?", 1)[0] != WIKI_API_URL:
                        raise ValueError("Heroes 3 Wiki API redirected to an untrusted endpoint")
                    body = response.read(8_000_001)
                    if len(body) > 8_000_000:
                        raise ValueError("Heroes 3 Wiki API response exceeded 8 MB")
                    payload = json.loads(body)
                    if not isinstance(payload, dict) or "error" in payload:
                        raise ValueError("Heroes 3 Wiki API returned an invalid payload")
                    return payload
            except (
                HTTPError,
                URLError,
                TimeoutError,
                ConnectionResetError,
                json.JSONDecodeError,
                ValueError,
            ) as error:
                last_error = error
                if attempt < 4:
                    time.sleep(min(attempt * 2, 8))
        raise RuntimeError(f"Heroes 3 Wiki API failed after 4 attempts: {last_error}")

    def paginated(
        self, params: dict[str, str], *, result_key: str, continuation_key: str
    ) -> list[dict[str, Any]]:
        results: list[dict[str, Any]] = []
        continuation: str | None = None
        while True:
            request_params = {"action": "query", "format": "json", **params}
            if continuation:
                request_params[continuation_key] = continuation
            payload = self._request(request_params)
            values = payload.get("query", {}).get(result_key)
            if not isinstance(values, list):
                raise ValueError(f"Heroes 3 Wiki response has no {result_key} list")
            results.extend(values)
            continuation = payload.get("continue", {}).get(continuation_key)
            if not continuation:
                return results

    def pages(self, *, page_ids: list[int] | None = None, titles: list[str] | None = None) -> list[dict[str, Any]]:
        if (page_ids is None) == (titles is None):
            raise ValueError("request page IDs or titles, but not both")
        requested: list[int | str] = page_ids if page_ids is not None else titles or []
        pages: list[dict[str, Any]] = []
        for offset in range(0, len(requested), 20):
            batch = requested[offset : offset + 20]
            params = {
                "action": "query",
                "prop": "revisions",
                "rvprop": "ids|timestamp|content",
                "rvslots": "main",
                "format": "json",
                ("pageids" if page_ids is not None else "titles"): "|".join(map(str, batch)),
            }
            payload = self._request(params)
            values = payload.get("query", {}).get("pages", {})
            if not isinstance(values, dict):
                raise ValueError("Heroes 3 Wiki response has no pages object")
            pages.extend(values.values())
        if len(pages) != len(requested):
            raise ValueError(
                f"Heroes 3 Wiki returned {len(pages)} pages for {len(requested)} requests"
            )
        return pages


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
    without_comments = "".join(output)
    without_trailing_commas: list[str] = []
    in_string = False
    escaped = False
    for index, character in enumerate(without_comments):
        if in_string:
            without_trailing_commas.append(character)
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
            continue
        if character == '"':
            in_string = True
            without_trailing_commas.append(character)
            continue
        if character == ",":
            lookahead = index + 1
            while lookahead < len(without_comments) and without_comments[lookahead].isspace():
                lookahead += 1
            if lookahead < len(without_comments) and without_comments[lookahead] in "]}":
                continue
        without_trailing_commas.append(character)
    try:
        return json.loads("".join(without_trailing_commas))
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


def _add_fact(facts: list[dict[str, str]], label: str, value: Any) -> None:
    if value is None:
        return
    if isinstance(value, (list, tuple)):
        rendered = ", ".join(str(item) for item in value if str(item).strip())
    else:
        rendered = str(value).strip()
    if rendered and not any(fact["label"] == label for fact in facts):
        facts.append({"label": label, "value": rendered})


def _record(
    entity_type: str,
    source_key: str,
    index: int,
    name: str,
    page: dict[str, Any],
    facts: list[dict[str, str]],
    *,
    source_file: str,
    source_anchor: str | None = None,
    entry_url: str | None = None,
) -> dict[str, Any]:
    source = wiki_page_source(page, anchor=source_anchor)
    if entry_url:
        source["entry_url"] = entry_url
    return {
        "source_key": f"homm3-complete-{entity_type}-{index}",
        "entity_type": entity_type,
        "vcmi_key": source_key,
        "vcmi_index": index,
        "official_english_name": name,
        "official_original_name": name,
        "official_vietnamese_name": None,
        "vcmi_source_file": source_file,
        "wiki_source": source,
        "facts": facts,
    }


def _pages_by_id(client: WikiClient, page_ids: list[int]) -> dict[int, dict[str, Any]]:
    pages = client.pages(page_ids=page_ids)
    by_id = {page.get("pageid"): page for page in pages}
    if any(page_id not in by_id for page_id in page_ids):
        missing = [page_id for page_id in page_ids if page_id not in by_id]
        raise ValueError(f"Heroes 3 Wiki omitted requested page IDs: {missing}")
    return by_id


def _normalized_page_index(
    pages: list[dict[str, Any]], *, strip_prefix: str = ""
) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for page in pages:
        title = page.get("title")
        if not isinstance(title, str):
            continue
        if strip_prefix and title.startswith(strip_prefix):
            title = title[len(strip_prefix) :]
        normalized = normalize_identifier(title)
        if normalized in result:
            raise ValueError(f"ambiguous Heroes 3 Wiki title after normalization: {title}")
        result[normalized] = page
    return result


def _crawl_heroes(
    source_root: Path, client: WikiClient
) -> list[dict[str, Any]]:
    candidates = client.paginated(
        {
            "list": "embeddedin",
            "eititle": "Template:HeroNew",
            "einamespace": "0",
            "eilimit": "max",
        },
        result_key="embeddedin",
        continuation_key="eicontinue",
    )
    candidate_index = _normalized_page_index(candidates)
    configs: list[tuple[str, str, dict[str, Any]]] = []
    for faction in (*BASE_FACTIONS, "special"):
        source_file = f"config/heroes/{faction}.json"
        payload = read_jsonc(source_root / source_file)
        for key, config in payload.items():
            if isinstance(config, dict) and isinstance(config.get("index"), int):
                configs.append((key, source_file, config))
    configs.sort(key=lambda item: item[2]["index"])
    if len(configs) != EXPECTED_COUNTS["hero"]:
        raise ValueError(f"VCMI hero catalog has {len(configs)} records")

    matches: list[tuple[str, str, dict[str, Any], dict[str, Any]]] = []
    for key, source_file, config in configs:
        title = HERO_TITLE_OVERRIDES.get(key, title_from_key(key))
        candidate = candidate_index.get(normalize_identifier(title))
        if candidate is None:
            raise ValueError(f"no Heroes 3 Wiki hero page matches VCMI key {key!r}")
        matches.append((key, source_file, config, candidate))
    pages = _pages_by_id(client, [match[3]["pageid"] for match in matches])

    records: list[dict[str, Any]] = []
    for key, source_file, config, candidate in matches:
        page = pages[candidate["pageid"]]
        facts = facts_from_template(
            wiki_page_content(page),
            "HeroNew",
            (
                ("Town", "town"),
                ("Class", "class"),
                ("Gender", "gender"),
                ("Race", "race"),
                ("Starting skill 1", "skill_1"),
                ("Starting skill 2", "skill_2"),
                ("Starting spell", "spell"),
                ("Specialty", "specialty"),
            ),
        )
        hero_class = config.get("class")
        faction = CLASS_TO_FACTION.get(hero_class)
        _add_fact(facts, "Town", title_from_key(faction) if faction else None)
        _add_fact(facts, "Class", title_from_key(hero_class) if hero_class else None)
        _add_fact(facts, "Gender", "Female" if config.get("female") else "Male")
        skills = [
            f"{title_from_key(skill.get('level', ''))} {title_from_key(skill.get('skill', ''))}".strip()
            for skill in config.get("skills", [])
            if isinstance(skill, dict)
        ]
        _add_fact(facts, "Starting skills", skills)
        _add_fact(
            facts,
            "Starting spells",
            [title_from_key(spell) for spell in config.get("spellbook", [])],
        )
        _add_fact(facts, "Availability", "Special/campaign hero" if config.get("special") else "Standard roster")
        records.append(
            _record(
                "hero",
                key,
                config["index"],
                page["title"],
                page,
                facts,
                source_file=source_file,
            )
        )
    return records


def _crawl_towns(source_root: Path, client: WikiClient) -> list[dict[str, Any]]:
    pages = client.pages(titles=[title_from_key(key) for key in BASE_FACTIONS])
    page_index = _normalized_page_index(pages)
    records: list[dict[str, Any]] = []
    for key in BASE_FACTIONS:
        source_file = f"config/factions/{key}.json"
        config = read_jsonc(source_root / source_file)[key]
        page = page_index.get(normalize_identifier(key))
        if page is None:
            raise ValueError(f"no Heroes 3 Wiki town page matches VCMI key {key!r}")
        facts = facts_from_template(
            wiki_page_content(page),
            "TownNew",
            (
                ("Alignment", "alignment"),
                ("Hero class 1", "class_1"),
                ("Hero class 2", "class_2"),
                ("Country", "country"),
                ("Native terrain", "terrain"),
            ),
        )
        _add_fact(facts, "Alignment", title_from_key(config.get("alignment", "")))
        hero_classes = [
            title_from_key(hero_class)
            for hero_class, faction in CLASS_TO_FACTION.items()
            if faction == key
        ]
        _add_fact(facts, "Hero classes", hero_classes)
        _add_fact(
            facts,
            "Native terrain",
            [title_from_key(terrain) for terrain in config.get("nativeTerrain", [])],
        )
        records.append(
            _record(
                "town",
                key,
                config["index"],
                page["title"],
                page,
                facts,
                source_file=source_file,
            )
        )
    return records


def _crawl_creatures(
    source_root: Path, client: WikiClient
) -> list[dict[str, Any]]:
    candidates = client.paginated(
        {
            "list": "categorymembers",
            "cmtitle": "Category:Creature Database",
            "cmnamespace": "10",
            "cmtype": "page",
            "cmlimit": "max",
        },
        result_key="categorymembers",
        continuation_key="cmcontinue",
    )
    candidate_index = _normalized_page_index(candidates, strip_prefix="Template:Creature/")
    configs: list[tuple[str, str, dict[str, Any]]] = []
    creature_dir = source_root / "config/creatures"
    for path in sorted(creature_dir.glob("*.json")):
        if path.name == "special.json":
            continue
        source_file = f"config/creatures/{path.name}"
        for key, config in read_jsonc(path).items():
            if isinstance(config, dict) and not config.get("disabled"):
                configs.append((key, source_file, config))
    configs.sort(key=lambda item: item[2]["index"])
    if len(configs) != EXPECTED_COUNTS["creature"]:
        raise ValueError(f"VCMI creature catalog has {len(configs)} records")

    matches: list[tuple[str, str, dict[str, Any], dict[str, Any]]] = []
    for key, source_file, config in configs:
        title = CREATURE_TITLE_OVERRIDES.get(key, title_from_key(key))
        candidate = candidate_index.get(normalize_identifier(title))
        if candidate is None:
            raise ValueError(f"no Heroes 3 Wiki creature template matches VCMI key {key!r}")
        matches.append((key, source_file, config, candidate))
    pages = _pages_by_id(client, [match[3]["pageid"] for match in matches])

    records: list[dict[str, Any]] = []
    for key, source_file, config, candidate in matches:
        page = pages[candidate["pageid"]]
        text = wiki_page_content(page)
        fields = template_fields(text, "CreatureNew")
        name = clean_wikitext(fields.get("name", "")) or page["title"].removeprefix("Template:Creature/")
        facts = facts_from_template(
            text,
            "CreatureNew",
            (
                ("Attack", "attack"),
                ("Defense", "defense"),
                ("Damage", "damage"),
                ("Health", "health"),
                ("Speed", "speed"),
                ("Movement", ("move", "movement")),
                ("Growth", "growth"),
                ("AI value", "aivalue"),
                ("Fight value", "fightvalue"),
                ("Gold cost", "goldcost"),
                ("Abilities", "abilities"),
            ),
        )
        _add_fact(facts, "Faction", title_from_key(config.get("faction", "neutral")))
        _add_fact(facts, "Tier", config.get("level"))
        _add_fact(
            facts,
            "Upgrades to",
            [title_from_key(value) for value in config.get("upgrades", [])],
        )
        records.append(
            _record(
                "creature",
                key,
                config["index"],
                name,
                page,
                facts,
                source_file=source_file,
                entry_url=wiki_url(name),
            )
        )
    return records


def _crawl_artifacts(
    source_root: Path, client: WikiClient
) -> list[dict[str, Any]]:
    table_pages = client.pages(titles=list(ARTIFACT_TABLE_TITLES))
    row_index: dict[str, tuple[dict[str, str | None], dict[str, Any]]] = {}
    for page in table_pages:
        for row in parse_artifact_rows(wiki_page_content(page)):
            normalized = normalize_identifier(str(row["name"]))
            if normalized in row_index:
                raise ValueError(f"duplicate artifact table row: {row['name']}")
            row_index[normalized] = (row, page)

    special_titles = sorted({title for title, _ in SPECIAL_ARTIFACT_SOURCES.values()})
    special_pages = client.pages(titles=special_titles)
    special_page_index = _normalized_page_index(special_pages)
    payload = read_jsonc(source_root / "config/artifacts.json")
    configs = [
        (key, config)
        for key, config in payload.items()
        if isinstance(config, dict)
        and isinstance(config.get("index"), int)
        and config["index"] <= 140
        and not key.startswith("unused")
    ]
    configs.sort(key=lambda item: item[1]["index"])
    if len(configs) != EXPECTED_COUNTS["artifact"]:
        raise ValueError(f"VCMI artifact catalog has {len(configs)} records")

    edition_names = {
        "roe": "Restoration of Erathia",
        "ab": "Armageddon's Blade",
        "sod": "Shadow of Death",
    }
    records: list[dict[str, Any]] = []
    for key, config in configs:
        if key in SPECIAL_ARTIFACT_SOURCES:
            source_title, anchor = SPECIAL_ARTIFACT_SOURCES[key]
            page = special_page_index.get(normalize_identifier(source_title))
            if page is None:
                raise ValueError(f"no Heroes 3 Wiki page matches special artifact {key!r}")
            name = SPECIAL_ARTIFACT_NAMES[key]
            facts: list[dict[str, str]] = []
            _add_fact(facts, "Artifact type", "War Machine" if anchor else "Special artifact")
            records.append(
                _record(
                    "artifact",
                    key,
                    config["index"],
                    name,
                    page,
                    facts,
                    source_file="config/artifacts.json",
                    source_anchor=anchor,
                )
            )
            continue

        proposed = ARTIFACT_TITLE_OVERRIDES.get(key, title_from_key(key))
        match = row_index.get(normalize_identifier(proposed))
        if match is None:
            raise ValueError(f"no Heroes 3 Wiki artifact row matches VCMI key {key!r}")
        row, page = match
        name = str(row["name"])
        facts = []
        _add_fact(facts, "Edition", edition_names.get(str(row["edition"]), row["edition"]))
        _add_fact(facts, "Slot", row["slot"])
        _add_fact(facts, "Class", row["class"])
        _add_fact(facts, "Value", row["value"])
        _add_fact(facts, "Effect", row["effect"])
        _add_fact(facts, "Components", row["components"])
        records.append(
            _record(
                "artifact",
                key,
                config["index"],
                name,
                page,
                facts,
                source_file="config/artifacts.json",
                entry_url=wiki_url(name),
            )
        )
    return records


def _crawl_spells(source_root: Path, client: WikiClient) -> list[dict[str, Any]]:
    candidates = client.paginated(
        {
            "list": "categorymembers",
            "cmtitle": "Category:Spells",
            "cmnamespace": "0",
            "cmtype": "page",
            "cmlimit": "max",
        },
        result_key="categorymembers",
        continuation_key="cmcontinue",
    )
    candidate_index = _normalized_page_index(candidates)
    configs: list[tuple[str, str, dict[str, Any]]] = []
    for filename in SPELL_CONFIG_FILES:
        source_file = f"config/spells/{filename}"
        for key, config in read_jsonc(source_root / source_file).items():
            index = config.get("index") if isinstance(config, dict) else None
            if isinstance(index, int) and 0 <= index <= 69:
                configs.append((key, source_file, config))
    configs.sort(key=lambda item: item[2]["index"])
    if len(configs) != EXPECTED_COUNTS["spell"]:
        raise ValueError(f"VCMI spell catalog has {len(configs)} records")

    matches: list[tuple[str, str, dict[str, Any], dict[str, Any]]] = []
    for key, source_file, config in configs:
        title = SPELL_TITLE_OVERRIDES.get(key, title_from_key(key))
        candidate = candidate_index.get(normalize_identifier(title))
        if candidate is None:
            raise ValueError(f"no Heroes 3 Wiki spell page matches VCMI key {key!r}")
        matches.append((key, source_file, config, candidate))
    pages = _pages_by_id(client, [match[3]["pageid"] for match in matches])

    records: list[dict[str, Any]] = []
    for key, source_file, config, candidate in matches:
        page = pages[candidate["pageid"]]
        text = wiki_page_content(page)
        fields = template_fields(text, "Spell")
        name = clean_wikitext(fields.get("name", "")) or re.sub(r"\s*\(spell\)$", "", page["title"], flags=re.I)
        facts = facts_from_template(
            text,
            "Spell",
            (
                ("Type", "type"),
                ("School", ("school", "schools")),
                ("Level", "level"),
                ("Cost", "cost"),
                ("Duration", "duration"),
                ("Basic effect", ("basic", "basic_effect")),
                ("Advanced effect", ("advanced", "advanced_effect")),
                ("Expert effect", ("expert", "expert_effect")),
                ("Effect", "effect"),
            ),
        )
        _add_fact(facts, "Target type", title_from_key(config.get("targetType", "")))
        records.append(
            _record(
                "spell",
                key,
                config["index"],
                name,
                page,
                facts,
                source_file=source_file,
            )
        )
    return records


def build_snapshot(source_root: Path, *, checked_at: str) -> dict[str, Any]:
    commit = checkout_commit(source_root)
    if commit != PINNED_VCMI_COMMIT:
        raise ValueError(
            f"VCMI checkout is at {commit}; expected pinned commit {PINNED_VCMI_COMMIT}"
        )
    client = WikiClient()
    records = [
        *_crawl_towns(source_root, client),
        *_crawl_heroes(source_root, client),
        *_crawl_creatures(source_root, client),
        *_crawl_artifacts(source_root, client),
        *_crawl_spells(source_root, client),
    ]
    validate_catalog_records(records, EXPECTED_COUNTS)
    return {
        "schema": SCHEMA,
        "checked_at": checked_at,
        "module_id": "heroes-of-might-and-magic",
        "edition": "Heroes of Might and Magic III Complete",
        "sources": {
            "structure": {
                "provider": "VCMI",
                "url": VCMI_SOURCE_URL,
                "commit": commit,
            },
            "labels_and_facts": {
                "provider": "Heroes 3 Wiki",
                "url": WIKI_API_URL,
                "revision_provenance": "stored per record",
            },
        },
        "catalog_scope": {
            "included_records": len(records),
            "counts": EXPECTED_COUNTS,
            "included_editions": [
                "Restoration of Erathia",
                "Armageddon's Blade",
                "Shadow of Death",
            ],
            "excluded": [
                "Horn of the Abyss",
                "Day of Reckoning",
                "VCMI-only internal spells and disabled creatures",
            ],
        },
        "notes": [
            "The snapshot contains short catalog facts, not copied biographies or manual prose.",
            "Heroes 3 Wiki is a community source; every selected page revision is retained for review.",
            "No official Vietnamese source was confirmed, so official Vietnamese names remain null.",
            "NPC and region entity types remain available for manual use but are not inferred by this crawl.",
        ],
        "records": records,
    }


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
