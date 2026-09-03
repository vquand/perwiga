#!/usr/bin/env python3
"""Crawl Genshin books and quests, then build their reviewed lore join.

The Genshin Impact Wiki is a community-maintained source. This crawler keeps
the boundary conservative: it extracts infobox fields, the readable Text
section from book pages, release-version categories, explicit Quest/Book
templates, quest reward books, predecessor links, and the explicit
character/appearance categories on quest pages. It does not turn arbitrary
prose links into confirmed appearances or dates.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import re
import sys
import time
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen


API_URL = "https://genshin-impact.fandom.com/api.php"
WIKI_URL = "https://genshin-impact.fandom.com/wiki/"
BOOK_CATEGORY = "Category:Books"
QUEST_CATEGORY = "Category:Quests"
BOOK_SCHEMA = "perwiga.genshin-impact.books.v1"
QUEST_SCHEMA = "perwiga.genshin-impact.quests.v1"
LORE_SCHEMA = "perwiga-lore-candidates/v1"
NPC_PROVIDER = "Genshin Impact Wiki NPC catalog"
CHARACTER_PROVIDER = "Genshin Impact official global character directory"
BOOK_PROVIDER = "Genshin Impact Wiki Book catalog"
QUEST_PROVIDER = "Genshin Impact Wiki Quest catalog"

FIELD_RE = re.compile(r"^\s*\|\s*([A-Za-z][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$")
INFOBOX_RE = re.compile(r"\{\{\s*([A-Za-z ]+?)\s+Infobox\b", re.I)
LANGUAGE_FIELD_RE = re.compile(r"^\s*\|\s*(en|zhs|zht|vi)\s*=\s*(.*?)\s*$", re.I | re.M)
LINK_RE = re.compile(r"\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|([^\]]+))?\]\]")
TEMPLATE_RE = re.compile(r"\{\{\s*(Quest|Book)\s*\|\s*([^{}|]+?)(?:\|[^{}]*)?\}\}", re.I)
VERSION_RE = re.compile(r"^Released in Version (\d+(?:\.\d+)*)$", re.I)
APPEARANCE_CATEGORY_RE = re.compile(r"^(.+?) Appearances$", re.I)
BOOK_TEXT_HEADING_RE = re.compile(r"^={2,6}\s*Text(?:\[\])?\s*={2,6}\s*$", re.I | re.M)
SECTION_HEADING_RE = re.compile(r"^={2,6}\s*[^=\n]+?\s*={2,6}\s*$", re.M)


def compact(value: str) -> str:
    value = re.sub(r"<ref[^>]*>.*?</ref>", "", value, flags=re.I | re.S)
    value = re.sub(r"<!--.*?-->", "", value, flags=re.S)
    value = re.sub(r"<[^>]+>", "", value)
    value = re.sub(r"'{2,5}", "", value)
    value = html.unescape(value)
    return re.sub(r"\s+", " ", value).strip()


def title_value(value: str) -> str:
    links = list(LINK_RE.finditer(value))
    if links:
        value = links[0].group(1)
    value = re.sub(r"\{\{[^{}]*\}\}", "", value)
    return compact(value)


def plain_wikitext(value: str) -> str:
    value = re.sub(r"<ref[^>]*>.*?</ref>", "", value, flags=re.I | re.S)
    value = re.sub(r"<!--.*?-->", "", value, flags=re.S)
    for _ in range(5):
        replacement = re.sub(r"\{\{[^{}]*\}\}", "", value)
        if replacement == value:
            break
        value = replacement
    value = re.sub(r"\[\[([^\]|#]+)(?:#[^\]|]*)?\|([^\]]+)\]\]", r"\2", value)
    value = re.sub(r"\[\[([^\]|#]+)(?:#[^\]|]*)?\]\]", r"\1", value)
    value = re.sub(r"\[https?://\S+(?:\s+([^\]]+))?\]", lambda match: match.group(1) or "", value)
    value = re.sub(r"<br\s*/?>", "\n", value, flags=re.I)
    value = re.sub(r"<[^>]+>", "", value)
    value = re.sub(r"'{2,5}", "", value)
    value = html.unescape(value)
    return re.sub(r"[ \t]+", " ", value)


def book_content(wikitext: str) -> list[str]:
    heading = BOOK_TEXT_HEADING_RE.search(wikitext)
    if not heading:
        return []
    section = wikitext[heading.end():]
    next_heading = SECTION_HEADING_RE.search(section)
    if next_heading:
        section = section[:next_heading.start()]
    section = plain_wikitext(section)
    paragraphs: list[str] = []
    for block in re.split(r"\n\s*\n+", section):
        lines = []
        for line in block.splitlines():
            line = re.sub(r"^\s*[*#;:]\s*", "", line).strip()
            if line:
                lines.append(line)
        paragraph = re.sub(r"\s+", " ", " ".join(lines)).strip()
        if paragraph and paragraph not in paragraphs:
            paragraphs.append(paragraph)
    return paragraphs


def page_url(title: str) -> str:
    return WIKI_URL + quote(title.replace(" ", "_"), safe="()'!,-_.")


def parse_infobox(wikitext: str) -> tuple[str, dict[str, str]]:
    match = INFOBOX_RE.search(wikitext)
    if not match:
        return "", {}
    body = wikitext[match.end():].split("\n}}", 1)[0]
    # A small number of maintained pages have two infobox fields on one line
    # (for example ``next`` immediately followed by ``rewards``). Preserve
    # both fields without trying to parse pipes nested inside templates.
    body = re.sub(
        r"(\|(?:next|prev)\s*=\s*[^|\n]+)(?=\|(?:rewards|otherRewards)\s*=)",
        r"\1\n",
        body,
        flags=re.I,
    )
    fields: dict[str, str] = {}
    for line in body.splitlines():
        field = FIELD_RE.match(line)
        if field:
            fields[field.group(1).lower()] = field.group(2).strip()
    return compact(match.group(1)).lower(), fields


def languages(wikitext: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for key, value in LANGUAGE_FIELD_RE.findall(wikitext):
        cleaned = compact(value)
        if cleaned:
            result[key.lower()] = cleaned
    return result


def release_version(categories: list[str]) -> str | None:
    for category in categories:
        match = VERSION_RE.match(category.removeprefix("Category:"))
        if match:
            return match.group(1)
    return None


def explicit_templates(wikitext: str, kind: str) -> list[str]:
    values: list[str] = []
    pattern = re.compile(r"\{\{\s*" + re.escape(kind) + r"\s*\|\s*([^{}|]+)", re.I)
    for match in pattern.finditer(wikitext):
        value = title_value(match.group(1))
        if value and value not in values:
            values.append(value)
    return values


def linked_titles(value: str) -> list[str]:
    result: list[str] = []
    for match in LINK_RE.finditer(value):
        target = compact(match.group(1))
        if target and target not in result:
            result.append(target)
    return result


def parse_book_page(title: str, page_id: int, wikitext: str, categories: list[str]) -> dict:
    infobox_kind, fields = parse_infobox(wikitext)
    if "collection" in infobox_kind:
        kind = "collection"
    elif infobox_kind == "book":
        kind = "book"
    else:
        kind = "index"
    langs = languages(wikitext)
    quest_links = explicit_templates(wikitext, "Quest")
    source_values = [value for key, value in fields.items() if key == "source" or key.startswith("source")]
    for source_value in source_values:
        for item in explicit_templates(source_value, "Quest"):
            if item not in quest_links:
                quest_links.append(item)
    volume_count = fields.get("volumes")
    return {
        "source_key": f"genshin-fandom-book-{page_id}",
        "page_id": page_id,
        "page_title": title,
        "page_url": page_url(title),
        "official_english_name": langs.get("en", title),
        "official_simplified_chinese_name": langs.get("zhs", ""),
        "official_traditional_chinese_name": langs.get("zht", ""),
        "official_vietnamese_name": langs.get("vi", ""),
        "book_kind": kind,
        "quality": int(fields["quality"]) if fields.get("quality", "").isdigit() else None,
        "lore_region": title_value(fields.get("region_lore", "")),
        "obtain_region": title_value(fields.get("region_location", "")),
        "volume_count": int(volume_count) if volume_count and volume_count.isdigit() else None,
        "description": title_value(fields.get("description", "")),
        "content": book_content(wikitext),
        "source_note": "; ".join(title_value(value) for value in source_values if title_value(value)),
        "release_version": release_version(categories),
        "quest_links": quest_links,
        "related_quest_keys": [],
        "unresolved_quest_links": [],
    }


def normalized_name(value: str) -> str:
    value = title_value(value).casefold()
    value = re.sub(r"\s+\((?:quest|event quest|act|chapter)\)$", "", value)
    return value.strip()


def record_kind(infobox_kind: str, fields: dict[str, str], categories: list[str]) -> str:
    if infobox_kind == "act":
        return "act"
    if infobox_kind == "chapter":
        return "chapter"
    category_text = {item.removeprefix("Category:") for item in categories}
    if "Event Quest" in category_text or "Event Quests" in category_text:
        return "event_quest"
    if infobox_kind == "quest":
        return "quest"
    if infobox_kind in {"hidden exploration objectives", "quest-like"}:
        return "quest_like"
    if fields:
        return "index"
    return "index"


def known_entity_link(
    name: str,
    character_names: set[str] | dict[str, dict],
    npc_names: set[str] | dict[str, dict],
) -> dict:
    clean_name = title_value(name)
    canonical = normalized_name(clean_name)
    character_by_name = (
        character_names
        if isinstance(character_names, dict)
        else {normalized_name(item): item for item in character_names}
    )
    npc_by_name = (
        npc_names
        if isinstance(npc_names, dict)
        else {normalized_name(item): item for item in npc_names}
    )
    if canonical in character_by_name:
        record = character_by_name[canonical]
        return {"entity_type": "character", "name": record if isinstance(record, str) else record["official_english_name"],
                "source_key": None if isinstance(record, str) else record["source_key"], "source": "catalog"}
    if canonical in npc_by_name:
        record = npc_by_name[canonical]
        return {"entity_type": "npc", "name": record if isinstance(record, str) else record["official_english_name"],
                "source_key": None if isinstance(record, str) else record["source_key"], "source": "catalog"}
    return {"entity_type": "unknown", "name": clean_name, "source_key": None, "source": "explicit-quest-field"}


def quest_entity_links(
    fields: dict[str, str],
    categories: list[str],
    character_names: set[str] | dict[str, dict],
    npc_names: set[str] | dict[str, dict],
) -> list[dict]:
    names: list[tuple[str, str]] = []
    for key in ("characters", "character"):
        for item in re.split(r";|<br\s*/?>|\n", fields.get(key, ""), flags=re.I):
            cleaned = title_value(item)
            if cleaned:
                names.append((cleaned, "infobox"))
    for category in categories:
        match = APPEARANCE_CATEGORY_RE.match(category.removeprefix("Category:"))
        if match:
            cleaned = title_value(match.group(1))
            if cleaned:
                names.append((cleaned, "appearance-category"))
    result: list[dict] = []
    seen: set[tuple[str, str]] = set()
    for name, source in names:
        link = known_entity_link(name, character_names, npc_names)
        link["source"] = source
        identity = (link["entity_type"], link["name"])
        if identity not in seen:
            seen.add(identity)
            result.append(link)
    return result


def split_reward_names(value: str) -> list[str]:
    result: list[str] = []
    for item in re.split(r";|<br\s*/?>|\n", value, flags=re.I):
        item = re.sub(r"\*\s*\d+(?:\.\d+)?$", "", item).strip()
        cleaned = title_value(item)
        if cleaned and cleaned not in result:
            result.append(cleaned)
    return result


def parse_quest_page(
    title: str,
    page_id: int,
    wikitext: str,
    categories: list[str],
    *,
    book_titles: set[str],
    character_names: set[str] | dict[str, dict],
    npc_names: set[str] | dict[str, dict],
    redirect_titles: list[str] | None = None,
) -> dict:
    infobox_kind, fields = parse_infobox(wikitext)
    langs = languages(wikitext)
    known_books = {normalized_name(item): item for item in book_titles}
    book_links: list[dict] = []

    def add_book(name: str, source: str) -> None:
        canonical = known_books.get(normalized_name(name))
        if canonical is None:
            return
        if not any(item["title"] == canonical for item in book_links):
            book_links.append({"title": canonical, "source": source, "source_key": None})

    for item in split_reward_names(fields.get("rewards", "")):
        add_book(item, "reward")
    for item in explicit_templates(wikitext, "Book"):
        add_book(item, "explicit-reference")
    for item in linked_titles(wikitext):
        add_book(item, "explicit-link")

    description_match = re.search(r"\{\{\s*Quest Description\s*\|([^{}]*)\}\}", wikitext, re.I | re.S)
    description = title_value(description_match.group(1)) if description_match else ""
    regions = [title_value(fields.get(key, "")) for key in ("region", "area", "subarea") if fields.get(key)]
    regions = list(dict.fromkeys(item for item in regions if item))
    return {
        "source_key": f"genshin-fandom-quest-{page_id}",
        "page_id": page_id,
        "page_title": title,
        "page_url": page_url(title),
        "redirect_titles": redirect_titles or [],
        "official_english_name": langs.get("en", title),
        "official_simplified_chinese_name": langs.get("zhs", ""),
        "official_traditional_chinese_name": langs.get("zht", ""),
        "official_vietnamese_name": langs.get("vi", ""),
        "record_kind": record_kind(infobox_kind, fields, categories),
        "quest_type": title_value(fields.get("type", "")),
        "chapter": title_value(fields.get("chapter", "")),
        "act": title_value(fields.get("act", "")),
        "act_number": title_value(fields.get("actnum", "")),
        "part": title_value(fields.get("part", "")),
        "region": regions[0] if regions else "",
        "locations": regions,
        "description": description,
        "requirement": title_value(fields.get("requirement", "")),
        "previous_quest_title": title_value(fields.get("prev", "")) or None,
        "next_quest_title": title_value(fields.get("next", "")) or None,
        "release_version": release_version(categories),
        "entity_links": quest_entity_links(fields, categories, character_names, npc_names),
        "book_links": book_links,
        "previous_quest_source_key": None,
        "next_quest_source_key": None,
        "unresolved_quest_links": [],
    }


def resolve_title(value: str | None, by_name: dict[str, list[dict]]) -> dict | None:
    if not value:
        return None
    clean_value = title_value(value)
    candidates = by_name.get(normalized_name(clean_value), [])
    exact_page = [item for item in candidates if item["page_title"] == clean_value]
    if len(exact_page) == 1:
        return exact_page[0]
    exact_page_casefold = [item for item in candidates if item["page_title"].casefold() == clean_value.casefold()]
    if len(exact_page_casefold) == 1:
        return exact_page_casefold[0]
    exact_official = [item for item in candidates if item["official_english_name"] == clean_value]
    if len(exact_official) == 1:
        return exact_official[0]
    exact_official_casefold = [item for item in candidates if item["official_english_name"].casefold() == clean_value.casefold()]
    if len(exact_official_casefold) == 1:
        return exact_official_casefold[0]
    if len(candidates) == 1:
        return candidates[0]
    exact = [item for item in candidates if normalized_name(item["page_title"]) == normalized_name(clean_value)]
    return exact[0] if len(exact) == 1 else None


def index_by_title(records: list[dict]) -> dict[str, list[dict]]:
    result: dict[str, list[dict]] = {}
    for record in records:
        names = [record["page_title"], record["official_english_name"], *record.get("redirect_titles", [])]
        for name in names:
            candidates = result.setdefault(normalized_name(name), [])
            if not any(item["source_key"] == record["source_key"] for item in candidates):
                candidates.append(record)
    return result


def join_catalogs(books: list[dict], quests: list[dict]) -> None:
    quest_index = index_by_title(quests)
    book_index = index_by_title(books)
    for book in books:
        book["related_quest_keys"] = []
        book["unresolved_quest_links"] = []
        for title in book["quest_links"]:
            target = resolve_title(title, quest_index)
            if target is None:
                book["unresolved_quest_links"].append(title)
            elif target["source_key"] not in book["related_quest_keys"]:
                book["related_quest_keys"].append(target["source_key"])
    for quest in quests:
        quest["previous_quest_source_key"] = None
        quest["next_quest_source_key"] = None
        quest["unresolved_quest_links"] = []
        for field, output in (("previous_quest_title", "previous_quest_source_key"), ("next_quest_title", "next_quest_source_key")):
            target = resolve_title(quest[field], quest_index)
            if target is None and quest[field]:
                quest["unresolved_quest_links"].append(quest[field])
            elif target is not None:
                quest[output] = target["source_key"]
        for item in quest["book_links"]:
            target = resolve_title(item["title"], book_index)
            if target is None:
                item["source_key"] = None
                if item["title"] not in quest["unresolved_quest_links"]:
                    quest["unresolved_quest_links"].append(item["title"])
            else:
                item["source_key"] = target["source_key"]


def validate_cross_catalog_links(books: list[dict], quests: list[dict]) -> None:
    book_keys = {item["source_key"] for item in books}
    quest_keys = {item["source_key"] for item in quests}
    errors: list[str] = []
    for book in books:
        errors.extend(f"{book['source_key']} unresolved quest {value}" for value in book.get("unresolved_quest_links", []))
        errors.extend(f"{book['source_key']} unknown quest {value}" for value in book.get("related_quest_keys", []) if value not in quest_keys)
    for quest in quests:
        errors.extend(f"{quest['source_key']} unresolved link {value}" for value in quest.get("unresolved_quest_links", []))
        for item in quest.get("book_links", []):
            if item.get("source_key") is None or item["source_key"] not in book_keys:
                errors.append(f"{quest['source_key']} unknown book {item['title']}")
        for field in ("previous_quest_source_key", "next_quest_source_key"):
            value = quest.get(field)
            if value is not None and value not in quest_keys:
                errors.append(f"{quest['source_key']} unknown quest {value}")
    if errors:
        raise ValueError("cross-catalog links are unresolved: " + "; ".join(errors[:12]))


def sha256(value: object) -> str:
    encoded = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def source_ref(record: dict, provider: str) -> dict:
    return {"provider": provider, "identity": record["source_key"]}


def build_lore_batch(
    books: list[dict],
    quests: list[dict],
    *,
    characters: list[dict],
    npcs: list[dict],
    checked_at: str,
) -> dict:
    sources = [
        {"key": "books-catalog", "kind": "community-wiki-catalog", "provider": BOOK_PROVIDER,
         "identity": BOOK_CATEGORY, "title": "Genshin Impact Wiki Book list", "language": "en",
         "url": "https://genshin-impact.fandom.com/wiki/Category:Books", "content_sha256": sha256(books), "checked_at": checked_at},
        {"key": "quests-catalog", "kind": "community-wiki-catalog", "provider": QUEST_PROVIDER,
         "identity": QUEST_CATEGORY, "title": "Genshin Impact Wiki Quest list", "language": "en",
         "url": "https://genshin-impact.fandom.com/wiki/Category:Quests", "content_sha256": sha256(quests), "checked_at": checked_at},
        {"key": "npc-catalog", "kind": "community-wiki-catalog", "provider": NPC_PROVIDER,
         "identity": "data/npcs.json", "title": "Genshin Impact Wiki NPC catalog", "language": "en",
         "url": "https://genshin-impact.fandom.com/wiki/NPC/List", "content_sha256": sha256(npcs), "checked_at": checked_at},
        {"key": "character-catalog", "kind": "official-character-catalog", "provider": CHARACTER_PROVIDER,
         "identity": "data/characters.json", "title": "Genshin Impact official Character directory", "language": "en",
         "url": "https://genshin.hoyoverse.com/en/character/mondstadt", "content_sha256": sha256(characters), "checked_at": checked_at},
    ]
    versions = sorted({item["release_version"] for item in [*books, *quests] if item.get("release_version")}, key=lambda value: [int(part) for part in value.split(".")])
    periods = [{
        "key": "release-version-" + version.replace(".", "-"),
        "name_en": f"Version {version} release anchor",
        "description_en": "Publication/release ordering only; this is not an in-world chronological date.",
        "order": index * 10,
        "parent_key": None,
    } for index, version in enumerate(versions, start=1)]

    subjects: list[dict] = []
    for book in books:
        subjects.append({"key": "book-" + book["source_key"], "attested_name": book["official_english_name"], "proposed_type": "book",
                         "entity_ref": source_ref(book, BOOK_PROVIDER), "evidence": [{"source_key": "books-catalog", "locator": f"page:{book['page_id']}",
                         "excerpt": f"The Book catalog records {book['official_english_name']}.", "stance": "supports"}]})
    for record, proposed_type, source_key, provider, evidence_key in [
        *[(item, "character", "character-catalog", CHARACTER_PROVIDER, "character-catalog") for item in characters],
        *[(item, "npc", "npc-catalog", NPC_PROVIDER, "npc-catalog") for item in npcs],
    ]:
        subjects.append({"key": f"{proposed_type}-{record['source_key']}", "attested_name": record["official_english_name"],
                         "proposed_type": proposed_type, "entity_ref": {"provider": provider, "identity": record["source_key"]},
                         "evidence": [{"source_key": evidence_key, "locator": f"catalog:{record['source_key']}",
                         "excerpt": f"The curated catalog records {record['official_english_name']} as a {proposed_type}.", "stance": "supports"}]})
    for quest in quests:
        subjects.append({"key": "quest-" + quest["source_key"], "attested_name": quest["official_english_name"], "proposed_type": "quest",
                         "entity_ref": source_ref(quest, QUEST_PROVIDER), "evidence": [{"source_key": "quests-catalog", "locator": f"page:{quest['page_id']}",
                         "excerpt": f"The Quest catalog records {quest['official_english_name']}.", "stance": "supports"}]})

    subject_keys = {item["key"] for item in subjects}
    events: list[dict] = []
    for quest in quests:
        if quest.get("record_kind") == "index":
            continue
        period_key = "release-version-" + quest["release_version"].replace(".", "-") if quest.get("release_version") else None
        involvements = [{"subject_key": "quest-" + quest["source_key"], "role": "subject"}]
        for link in quest.get("entity_links", []):
            if link["entity_type"] in {"character", "npc"} and link.get("source_key"):
                key = f"{link['entity_type']}-{link['source_key']}"
            elif link["entity_type"] == "unknown":
                key = "unknown-" + re.sub(r"[^a-z0-9]+", "-", link["name"].casefold()).strip("-")
                if key not in subject_keys:
                    subjects.append({"key": key, "attested_name": link["name"], "proposed_type": "unknown", "entity_ref": None,
                                     "evidence": [{"source_key": "quests-catalog", "locator": f"page:{quest['page_id']}",
                                     "excerpt": f"The Quest infobox or appearance category names {link['name']}.", "stance": "supports"}]})
                    subject_keys.add(key)
            else:
                continue
            if key in subject_keys:
                involvements.append({"subject_key": key, "role": "participant"})
        for book in quest.get("book_links", []):
            if book.get("source_key"):
                involvements.append({"subject_key": "book-" + book["source_key"], "role": "document"})
        relations = []
        if quest.get("previous_quest_source_key"):
            relations.append({"kind": "after", "event_key": "quest-" + quest["previous_quest_source_key"]})
        label = "In-world date unknown"
        if quest.get("release_version"):
            label += f"; first indexed in Version {quest['release_version']}"
        events.append({
            "key": "quest-" + quest["source_key"],
            "title_en": quest["official_english_name"],
            "summary_en": quest.get("description") or None,
            "time": {"kind": "moment", "precision": "unknown", "label": label,
                      "start_period_key": period_key, "end_period_key": None, "relations": relations},
            "involvements": involvements,
            "claims": [{"key": "catalog-record", "text_en": f"The reviewed Quest catalog records {quest['official_english_name']} with explicit links to the listed books and named participants; its in-world date remains unresolved.",
                        "assertion_kind": "direct_fact", "certainty": "confirmed", "branch_group": None,
                        "involvements": [item for item in involvements if item["role"] != "subject"],
                        "evidence": [{"source_key": "quests-catalog", "locator": f"page:{quest['page_id']}",
                                      "excerpt": f"Quest page {quest['official_english_name']} contains the extracted infobox, category, and explicit cross-links.", "stance": "supports"}]}],
        })
    return {"schema": LORE_SCHEMA, "module_id": "genshin-impact",
            "corpus": {"version": f"reviewed-wiki-catalog-{checked_at}", "manifest_sha256": sha256({"books": books, "quests": quests})},
            "generator": {"name": "perwiga-genshin-lore-crawl", "version": "1", "generated_at": checked_at + "T00:00:00Z"},
            "sources": sources, "periods": periods, "subjects": subjects, "events": events}


def api_request(params: dict[str, str]) -> dict:
    request = Request(API_URL + "?" + urlencode(params), headers={"User-Agent": "Perwiga Genshin lore catalog (private local wiki)"})
    last_error = None
    for attempt in range(1, 5):
        try:
            with urlopen(request, timeout=30) as response:
                payload = json.load(response)
                if not isinstance(payload, dict):
                    raise ValueError("API response is not an object")
                return payload
        except (HTTPError, URLError, TimeoutError, ConnectionResetError, json.JSONDecodeError, ValueError) as error:
            last_error = error
            if attempt < 4:
                time.sleep(min(attempt * 2, 8))
    raise RuntimeError(f"Fandom API request failed after 4 attempts: {last_error}")


def category_members(category: str) -> list[tuple[int, str]]:
    result: list[tuple[int, str]] = []
    continuation = None
    while True:
        params = {"action": "query", "list": "categorymembers", "cmtitle": category, "cmtype": "page", "cmlimit": "max", "format": "json"}
        if continuation:
            params["cmcontinue"] = continuation
        payload = api_request(params)
        result.extend((int(item["pageid"]), item["title"]) for item in payload.get("query", {}).get("categorymembers", []))
        continuation = payload.get("continue", {}).get("cmcontinue")
        if not continuation:
            return result


def fetch_pages(page_ids: list[int]) -> list[dict]:
    pages: list[dict] = []
    for offset in range(0, len(page_ids), 20):
        batch = "|".join(str(value) for value in page_ids[offset:offset + 20])
        payload = api_request({"action": "query", "pageids": batch, "prop": "revisions|categories", "rvprop": "ids|content", "rvslots": "main", "cllimit": "max", "format": "json"})
        for page in payload.get("query", {}).get("pages", {}).values():
            revision = page.get("revisions", [{}])[0]
            wikitext = revision.get("slots", {}).get("main", {}).get("*", "")
            if not wikitext:
                raise RuntimeError(f"Fandom page did not return wikitext: {page.get('title')}")
            pages.append({"page_id": int(page["pageid"]), "title": page["title"], "wikitext": wikitext,
                          "categories": [item["title"] for item in page.get("categories", [])], "revision_id": revision.get("revid")})
        time.sleep(0.05)
    return pages


def fetch_pages_by_titles(titles: list[str]) -> list[dict]:
    pages: list[dict] = []
    for offset in range(0, len(titles), 20):
        batch = "|".join(titles[offset:offset + 20])
        payload = api_request({"action": "query", "titles": batch, "redirects": "1", "prop": "revisions|categories", "rvprop": "ids|content", "rvslots": "main", "cllimit": "max", "format": "json"})
        redirect_titles_by_target: dict[str, list[str]] = {}
        for redirect in payload.get("query", {}).get("redirects", []):
            redirect_titles_by_target.setdefault(redirect["to"], []).append(redirect["from"])
        for page in payload.get("query", {}).get("pages", {}).values():
            revision = page.get("revisions", [{}])[0]
            wikitext = revision.get("slots", {}).get("main", {}).get("*", "")
            if not wikitext or "missing" in page:
                continue
            pages.append({"page_id": int(page["pageid"]), "title": page["title"], "wikitext": wikitext,
                          "categories": [item["title"] for item in page.get("categories", [])],
                          "revision_id": revision.get("revid"),
                          "redirect_titles": redirect_titles_by_target.get(page["title"], [])})
        time.sleep(0.05)
    return pages


def atomic_write(path: Path, document: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def validate_replacement(path: Path, document: dict, *, allow_shrink: bool) -> None:
    if not path.exists() or allow_shrink:
        return
    previous = json.loads(path.read_text(encoding="utf-8"))
    old_count = previous.get("catalog_scope", {}).get("included_records", 0)
    new_count = document.get("catalog_scope", {}).get("included_records", 0)
    if new_count < old_count:
        raise ValueError(f"refusing catalog shrink from {old_count} to {new_count}; review and use --allow-shrink")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--checked-at", required=True)
    parser.add_argument("--books-output", type=Path, default=Path(__file__).parents[1] / "data/books.json")
    parser.add_argument("--quests-output", type=Path, default=Path(__file__).parents[1] / "data/quests.json")
    parser.add_argument("--lore-output", type=Path, default=Path(__file__).parents[1] / "data/lore.json")
    parser.add_argument("--allow-shrink", action="store_true")
    parser.add_argument("--allow-unresolved", action="store_true")
    args = parser.parse_args()

    book_members = category_members(BOOK_CATEGORY)
    quest_members = category_members(QUEST_CATEGORY)
    pages = fetch_pages(sorted({page_id for page_id, _ in [*book_members, *quest_members]}))
    book_ids = {page_id for page_id, _ in book_members}
    quest_ids = {page_id for page_id, _ in quest_members}
    books = [parse_book_page(page["title"], page["page_id"], page["wikitext"], page["categories"]) for page in pages if page["page_id"] in book_ids]
    books = [item for item in books if item["book_kind"] in {"book", "collection"}]
    character_snapshot = json.loads((Path(__file__).parents[1] / "data/characters.json").read_text(encoding="utf-8"))
    npc_snapshot = json.loads((Path(__file__).parents[1] / "data/npcs.json").read_text(encoding="utf-8"))
    character_names = {
        normalized_name(item["official_english_name"]): item
        for item in character_snapshot["characters"]
    }
    npc_names = {
        normalized_name(item["official_english_name"]): item
        for item in npc_snapshot["npcs"]
    }
    book_titles = {item["page_title"] for item in books} | {item["official_english_name"] for item in books}
    quests = [parse_quest_page(page["title"], page["page_id"], page["wikitext"], page["categories"], book_titles=book_titles,
                               character_names=character_names, npc_names=npc_names) for page in pages if page["page_id"] in quest_ids]
    quests = [item for item in quests if item["record_kind"] != "index"]
    join_catalogs(books, quests)
    initial_quest_count = len(quests)
    expanded_titles = sorted({value for quest in quests for value in quest.get("unresolved_quest_links", [])})
    if expanded_titles:
        expanded_pages = fetch_pages_by_titles(expanded_titles)
        known_page_ids = {quest["page_id"] for quest in quests}
        for page in expanded_pages:
            if page["page_id"] in known_page_ids:
                existing = next(quest for quest in quests if quest["page_id"] == page["page_id"])
                existing["redirect_titles"] = list(dict.fromkeys([*existing.get("redirect_titles", []), *page.get("redirect_titles", [])]))
                continue
            quest = parse_quest_page(page["title"], page["page_id"], page["wikitext"], page["categories"], book_titles=book_titles,
                                     character_names=character_names, npc_names=npc_names,
                                     redirect_titles=page.get("redirect_titles"))
            if quest["record_kind"] != "index":
                quests.append(quest)
                known_page_ids.add(quest["page_id"])
        join_catalogs(books, quests)
    expanded_quest_count = len(quests) - initial_quest_count
    try:
        validate_cross_catalog_links(books, quests)
    except ValueError:
        if not args.allow_unresolved:
            raise
    books_document = {"schema": BOOK_SCHEMA, "checked_at": args.checked_at,
                      "source": {"api_url": API_URL, "wiki_url": "https://genshin-impact.fandom.com/wiki/Category:Books", "community": "Genshin Impact Wiki contributors"},
                      "catalog_scope": {"included_records": len(books), "source_page_count": len(book_members)},
                      "notes": ["Book-to-Quest links come only from explicit Quest templates in Book pages."], "books": books}
    quests_document = {"schema": QUEST_SCHEMA, "checked_at": args.checked_at,
                       "source": {"api_url": API_URL, "wiki_url": "https://genshin-impact.fandom.com/wiki/Category:Quests", "community": "Genshin Impact Wiki contributors"},
                       "catalog_scope": {"included_records": len(quests), "source_page_count": len(quest_members), "expanded_linked_records": expanded_quest_count},
                       "notes": ["Release versions are publication anchors. They are not asserted as in-world dates.", "Character and NPC links come from Quest Infobox characters and explicit Appearances categories; arbitrary prose links are not treated as appearances."],
                       "quests": quests}
    for path, document in ((args.books_output, books_document), (args.quests_output, quests_document)):
        validate_replacement(path, document, allow_shrink=args.allow_shrink)
        atomic_write(path, document)
    atomic_write(args.lore_output, build_lore_batch(books, quests, characters=character_snapshot["characters"], npcs=npc_snapshot["npcs"], checked_at=args.checked_at))
    print(f"wrote {len(books)} books, {len(quests)} quests, and the connected lore batch")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (RuntimeError, ValueError, KeyError) as error:
        raise SystemExit(str(error))
