#!/usr/bin/env python3
"""Fetch a reviewable Genshin NPC catalog from the public Fandom MediaWiki API.

The source is community-maintained. This script intentionally stores only
source-backed relationships found in NPC infoboxes, page categories, explicit
Quest/Event/Action templates, and explicit linked quest/event titles; it does
not infer appearances from free-form lore prose.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
from urllib.error import HTTPError, URLError
from pathlib import Path
from urllib.parse import urlencode
from urllib.request import Request, urlopen

API_URL = "https://genshin-impact.fandom.com/api.php"
WIKI_URL = "https://genshin-impact.fandom.com/wiki/"
SCHEMA = "perwiga.genshin-impact.npcs.v1"

REGION_CATEGORIES = (
    "Mondstadt Characters", "Liyue Characters", "Inazuma Characters",
    "Sumeru Characters", "Fontaine Characters", "Natlan Characters",
    "Nod-Krai Characters", "Snezhnaya Characters",
    "Golden Apple Archipelago Characters", "Khaenri'ah Characters",
)
TYPE_CATEGORIES = (
    "Open-World NPCs", "Quest-Exclusive NPCs", "Event-Exclusive NPCs",
    "Quest and Event-Exclusive NPCs", "Mentioned Characters",
)
RELATION_RE = re.compile(r"\{\{\s*(Quest|Event|Action)\s*\|\s*([^{}|]+?)\s*\}}", re.I)
FIELD_RE = re.compile(r"^\s*\|\s*([A-Za-z][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$")
LINKED_RELATION_RE = re.compile(r"(?:(?:appears?|appeared)(?:\s+in)?|exclusive to|participates? in|featured in)\s+(?:the\s+)?\[\[([^\]|#]+)", re.I)
LANGUAGE_FIELD_RE = re.compile(r"^\s*\|\s*(zhs|zht|vi)\s*=\s*(.*?)\s*$", re.M)
LINK_RE = re.compile(r"\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|[^\]]+)?\]\]")


def clean(value: str) -> str:
    value = re.sub(r"<ref[^>]*>.*?</ref>", "", value, flags=re.I | re.S)
    value = re.sub(r"<[^>]+>", "", value)
    value = re.sub(r"\{\{[^{}]*\}\}", "", value)
    return re.sub(r"\s+", " ", value).strip()


def values(fields: dict[str, str], prefix: str) -> list[str]:
    result = []
    for key, value in fields.items():
        if key == prefix or re.fullmatch(prefix + r"\d+", key):
            for part in re.split(r"<br\s*/?>|\n|;", value, flags=re.I):
                value = clean(part)
                value = LINK_RE.sub(lambda match: match.group(1).strip(), value)
                if value and value not in result:
                    result.append(value)
    return result


def parse_page(title: str, page_id: int, wikitext: str, categories: list[str]) -> dict:
    infobox = wikitext.split("{{Character Infobox", 1)[-1].split("\n}}", 1)[0]
    fields = {}
    for line in infobox.splitlines():
        match = FIELD_RE.match(line)
        if match:
            fields[match.group(1).lower()] = match.group(2).strip()

    regions = values(fields, "region")
    locations = values(fields, "location")
    category_regions = [
        category.removeprefix("Category:NPCs Located in ")
        for category in categories
        if category.startswith("Category:NPCs Located in ")
    ]
    for region in category_regions:
        if region not in regions:
            regions.append(region)

    relationships = []
    for match in RELATION_RE.finditer(wikitext):
        kind = match.group(1).lower()
        title_value = clean(match.group(2))
        if title_value and not any(item["kind"] == kind and item["title"] == title_value for item in relationships):
            relationships.append({
                "kind": kind,
                "title": title_value,
                "locations": sorted(set(locations)),
                "regions": sorted(set(regions)),
                "source": "explicit-template",
            })

    category_text = " ".join(categories).lower()
    linked_kind = "event" if "event-exclusive npcs" in category_text else "quest"
    for match in LINKED_RELATION_RE.finditer(wikitext):
        title_value = clean(match.group(1))
        if title_value and not any(item["title"] == title_value for item in relationships):
            relationships.append({
                "kind": linked_kind,
                "title": title_value,
                "locations": sorted(set(locations)),
                "regions": sorted(set(regions)),
                "source": "explicit-linked-title",
            })

    excluded_categories = {
        "Category:Characters", "Category:Characters by Region", "Category:NPCs",
        "Category:Pages Using DPL Parser Function", "Category:Version",
        "Category:Event-Exclusive NPCs", "Category:Quest-Exclusive NPCs",
        "Category:Quest and Event-Exclusive NPCs", "Category:Open-World NPCs",
        "Category:Mentioned Characters",
    }
    relationship_categories = []
    for category in categories:
        if category in excluded_categories or "NPCs Located in " in category:
            continue
        label = category.removeprefix("Category:").strip()
        if not label or label.startswith(("Characters", "Released in Version", "Pages with ", "Pages Using ")):
            continue
        if any(word in label.lower() for word in ("quest", "event", "commission", "hangout", "story")):
            relationship_categories.append(label)
    for label in relationship_categories:
        kind = "quest" if any(word in label.lower() for word in ("quest", "commission", "hangout", "story")) else "event"
        if not any(item["kind"] == kind and item["title"] == label for item in relationships):
            relationships.append({
                "kind": kind,
                "title": label,
                "locations": sorted(set(locations)),
                "regions": sorted(set(regions)),
                "source": "page-category",
            })

    language_fields = {
        key: clean(value)
        for key, value in LANGUAGE_FIELD_RE.findall(wikitext)
    }
    zhs = language_fields.get("zhs", "")
    zht = language_fields.get("zht", "")
    vi = language_fields.get("vi", "")
    description_match = re.search(r"\{\{Description\|(.*?)\}\}", wikitext, re.I | re.S)
    description = clean(description_match.group(1)) if description_match else ""
    return {
        "source_key": f"genshin-fandom-npc-{page_id}",
        "page_id": page_id,
        "page_title": title,
        "page_url": WIKI_URL + title.replace(" ", "_"),
        "official_english_name": title,
        "official_simplified_chinese_name": zhs,
        "official_traditional_chinese_name": zht,
        "official_vietnamese_name": vi,
        "official_english_description": description,
        "regions": sorted(set(regions)),
        "locations": sorted(set(locations)),
        "relationships": relationships,
        "categories": sorted(category.removeprefix("Category:") for category in categories),
    }


def api_request(params: dict[str, str]) -> dict:
    request = Request(API_URL + "?" + urlencode(params), headers={"User-Agent": "Perwiga NPC catalog (private local wiki)"})
    last_error = None
    for attempt in range(1, 5):
        try:
            with urlopen(request, timeout=30) as response:
                return json.load(response)
        except (HTTPError, URLError, TimeoutError, ConnectionResetError, json.JSONDecodeError) as error:
            last_error = error
            if attempt < 4:
                time.sleep(min(attempt * 2, 8))
    raise RuntimeError(f"Fandom API request failed after 4 attempts: {last_error}")


def category_members(category: str) -> list[tuple[int, str]]:
    result = []
    continuation = None
    while True:
        params = {"action": "query", "list": "categorymembers", "cmtitle": "Category:" + category, "cmtype": "page", "cmlimit": "max", "format": "json"}
        if continuation:
            params["cmcontinue"] = continuation
        payload = api_request(params)
        result.extend((item["pageid"], item["title"]) for item in payload["query"]["categorymembers"])
        continuation = payload.get("continue", {}).get("cmcontinue")
        if not continuation:
            return result


def fetch_pages(page_ids: list[int]) -> list[dict]:
    pages = []
    for offset in range(0, len(page_ids), 20):
        batch = "|".join(str(value) for value in page_ids[offset:offset + 20])
        payload = None
        for attempt in range(3):
            candidate = api_request({"action": "query", "pageids": batch, "prop": "revisions|categories", "rvprop": "content", "rvslots": "main", "cllimit": "max", "format": "json"})
            candidate_pages = list(candidate.get("query", {}).get("pages", {}).values())
            if len(candidate_pages) == len(batch.split("|")) and all(
                page.get("revisions", [{}])[0].get("slots", {}).get("main", {}).get("*", "")
                for page in candidate_pages
            ):
                payload = candidate
                break
            if attempt < 2:
                time.sleep(2 * (attempt + 1))
        if payload is None:
            raise RuntimeError(f"NPC page batch did not return complete wikitext: {batch[:120]}")
        for page in payload["query"]["pages"].values():
            revision = page.get("revisions", [{}])[0]
            text = revision.get("slots", {}).get("main", {}).get("*", "")
            categories = [item["title"] for item in page.get("categories", [])]
            pages.append(parse_page(page["title"], int(page["pageid"]), text, categories))
        time.sleep(0.05)
    return pages


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path(__file__).parents[1] / "data/npcs.json")
    parser.add_argument("--checked-at", required=True)
    parser.add_argument("--allow-shrink", action="store_true")
    args = parser.parse_args()

    membership = {}
    for category in REGION_CATEGORIES + TYPE_CATEGORIES:
        for page_id, title in category_members(category):
            membership[page_id] = title
    records = fetch_pages(sorted(membership))
    records.sort(key=lambda item: (item["official_english_name"].casefold(), item["page_id"]))
    snapshot = {
        "schema": SCHEMA,
        "checked_at": args.checked_at,
        "source": {"url": API_URL, "wiki_url": "https://genshin-impact.fandom.com/wiki/NPC/List", "community": "Genshin Impact Wiki contributors"},
        "catalog_scope": {"included_records": len(records), "source_page_count": len(records)},
        "notes": [
            "Relationships are source-backed only: explicit Quest/Event/Action templates, selected page categories, and explicit linked quest/event titles.",
            "Locations are NPC infobox locations and are repeated on each related appearance as the source does not expose a normalized location history.",
            "Empty original-language fields remain explicit when an NPC page has no Simplified Chinese localization.",
        ],
        "npcs": records,
    }
    if args.output.exists() and not args.allow_shrink:
        previous = json.loads(args.output.read_text(encoding="utf-8"))
        previous_count = previous.get("catalog_scope", {}).get("included_records", 0)
        if len(records) < previous_count:
            raise SystemExit(f"refusing catalog shrink from {previous_count} to {len(records)}; review and use --allow-shrink")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(args.output)
    relationships = sum(len(item["relationships"]) for item in records)
    print(f"wrote {len(records)} NPC records and {relationships} source-backed relationships to {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
