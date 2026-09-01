#!/usr/bin/env python3
"""Build source-backed Endfield mission appearances for the curated cast.

Dialogue actor templates attest that a character is staged in a mission. They do
not by themselves prove that the character speaks, nor do mission identifiers
establish an exact in-world date. Characters without such evidence are retained
as explicitly unplaced catalog members.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from datetime import date
import json
from pathlib import Path
import re
import subprocess
from typing import Any


CLIENT_DATA_COMMIT = "90bc55768143f1998ad02d03ed0042919135979f"


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def client_commit(root: Path) -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=root, check=True, text=True,
        stdout=subprocess.PIPE,
    ).stdout.strip()


def natural_key(value: str) -> list[str | int]:
    return [int(part) if part.isdigit() else part for part in re.split(r"(\d+)", value)]


def mission_period(mission_id: str) -> str | None:
    """Return only broad placements supported by the mission family."""
    if mission_id.startswith("sm1l5"):
        return "valley-iv-after-main"
    main = re.fullmatch(r"e(\d+)m.*", mission_id)
    if main:
        episode = int(main.group(1))
        if episode <= 4:
            return "chapter-i-valley-iv"
        if episode <= 9:
            return "chapter-ii-wuling"
        return "homecoming"
    if mission_id.startswith("sm2") or mission_id.startswith("gm02"):
        return "chapter-ii-wuling"
    if mission_id.startswith(("sm1", "gm01")):
        return "chapter-i-valley-iv"
    return None


def actor_template_names(root: Path) -> tuple[dict[str, set[str]], dict[str, set[str]]]:
    characters = read_json(root / "tableCfg" / "CharacterTable.json")
    npcs = read_json(root / "tableCfg" / "NpcTable.json")
    operator_templates: dict[str, set[str]] = defaultdict(set)
    npc_templates: dict[str, set[str]] = defaultdict(set)

    for source_key, record in characters.items():
        name = record.get("name", {}).get("en")
        if name:
            operator_templates[name].add(f"npc_{source_key}")
    for source_key, record in npcs.items():
        name = record.get("name", {}).get("en")
        if not name:
            continue
        templates = {
            source_key,
            record.get("npcId"),
            record.get("dataKey"),
        }
        npc_templates[name].update(value for value in templates if value)
        # Playable characters are often staged through their NPC template.
        operator_templates[name].update(value for value in templates if value)
    return operator_templates, npc_templates


def mission_actor_templates(mission_dir: Path) -> tuple[set[str], list[str]]:
    templates: set[str] = set()
    files = sorted(mission_dir.glob("*.json"), key=lambda path: natural_key(path.name))
    for path in files:
        document = read_json(path)
        derived = document.get("derivedData", {})
        for actor in [*derived.get("privateActors", []), *derived.get("actors", [])]:
            template = actor.get("templateId")
            if template:
                templates.add(template)
    return templates, [path.name for path in files]


def roster_records(snapshot: dict[str, Any], key: str, entity_type: str) -> list[dict[str, str]]:
    return [
        {
            "entity_type": entity_type,
            "source_key": record["source_key"],
            "name_en": record["official_english_name"],
        }
        for record in snapshot[key]
    ]


def build_snapshot(
    root: Path,
    operators: dict[str, Any],
    npcs: dict[str, Any],
    checked_at: str | None = None,
) -> dict[str, Any]:
    text = read_json(root / "tableCfg" / "TextTable.json")
    operator_templates, npc_templates = actor_template_names(root)
    roster = [
        *roster_records(operators, "operators", "operator"),
        *roster_records(npcs, "npcs", "npc"),
    ]
    templates_by_character: dict[tuple[str, str], set[str]] = {}
    for character in roster:
        templates = operator_templates if character["entity_type"] == "operator" else npc_templates
        templates_by_character[(character["entity_type"], character["source_key"])] = set(
            templates.get(character["name_en"], set())
        )

    placed: set[tuple[str, str]] = set()
    missions = []
    mission_root = root / "dialog" / "json" / "mission"
    for mission_dir in sorted(
        (path for path in mission_root.iterdir() if path.is_dir()),
        key=lambda path: natural_key(path.name),
    ):
        actor_templates, files = mission_actor_templates(mission_dir)
        mission_characters = []
        for character in roster:
            identity = (character["entity_type"], character["source_key"])
            if not templates_by_character[identity].intersection(actor_templates):
                continue
            placed.add(identity)
            mission_characters.append(character)
        if not mission_characters:
            continue
        mission_id = mission_dir.name
        title = text.get(f"{mission_id}_name", {}).get("en") or mission_id
        description = text.get(f"{mission_id}_desc_001", {}).get("en") or None
        missions.append({
            "mission_id": mission_id,
            "title_en": title,
            "summary_en": description,
            "period_key": mission_period(mission_id),
            "source_locator": f"dialog/json/mission/{mission_id} ({len(files)} dialogue files)",
            "characters": sorted(
                mission_characters,
                key=lambda item: (item["entity_type"], item["name_en"], item["source_key"]),
            ),
        })

    unplaced = [
        character for character in roster
        if (character["entity_type"], character["source_key"]) not in placed
    ]
    source_base = f"https://github.com/555me/beyondGameData/tree/{CLIENT_DATA_COMMIT}"
    operator_count = len(operators["operators"])
    npc_count = len(npcs["npcs"])
    return {
        "schema": "perwiga.endfield.lore-appearances.v1",
        "checked_at": checked_at or date.today().isoformat(),
        "client_data_commit": CLIENT_DATA_COMMIT,
        "catalog_scope": {
            "operators": operator_count,
            "npcs": npc_count,
            "characters": operator_count + npc_count,
            "placed_characters": len(placed),
            "unplaced_characters": len(unplaced),
            "missions": len(missions),
        },
        "sources": {
            "client_data": source_base,
            "dialogue": f"{source_base}/dialog/json/mission",
            "mission_text": f"{source_base}/tableCfg/TextTable.json",
            "operator_templates": f"{source_base}/tableCfg/CharacterTable.json",
            "npc_templates": f"{source_base}/tableCfg/NpcTable.json",
        },
        "evidence_note": (
            "A dialogue actor template directly supports association with the mission, but does "
            "not alone prove that the character speaks or is physically present in every scene."
        ),
        "missions": missions,
        "unplaced_characters": sorted(
            unplaced,
            key=lambda item: (item["entity_type"], item["name_en"], item["source_key"]),
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--client-data", required=True, type=Path)
    parser.add_argument("--operators", required=True, type=Path)
    parser.add_argument("--npcs", required=True, type=Path)
    parser.add_argument("--checked-at")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    actual = client_commit(args.client_data)
    if actual != CLIENT_DATA_COMMIT:
        raise SystemExit(f"Expected client commit {CLIENT_DATA_COMMIT}, found {actual}")
    snapshot = build_snapshot(
        args.client_data,
        read_json(args.operators),
        read_json(args.npcs),
        checked_at=args.checked_at,
    )
    args.output.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
