#!/usr/bin/env python3
"""Group Artifact challenge stages into stable multilingual Domain entrances."""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import tempfile
from typing import Any


SCHEMA = "perwiga.genshin-impact.artifact-domains.v1"
ARTIFACT_SCHEMA = "perwiga.genshin-impact.artifacts.v1"
SOURCE_URL = "https://github.com/theBowja/genshin-db"
LOCALES = {
    "official_simplified_chinese_name": "ChineseSimplified",
    "official_traditional_chinese_name": "ChineseTraditional",
    "official_vietnamese_name": "Vietnamese",
}


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"unable to read valid JSON from {path}: {error}") from error


def required_text(value: Any, field: str, source: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{source} has invalid {field}")
    return value.strip()


def is_artifact_challenge(payload: dict[str, Any]) -> bool:
    return payload.get("domainType") == "UI_ABYSSUS_RELIC"


def build_snapshot(
    source_root: Path,
    *,
    artifact_set_names: dict[str, str],
    checked_at: str,
    source_commit: str,
) -> dict[str, Any]:
    english_dir = source_root / "src" / "data" / "English" / "domains"
    if not english_dir.is_dir():
        raise ValueError("source root has no English Domain catalog")
    if len(source_commit) != 40 or not all(character in "0123456789abcdef" for character in source_commit.lower()):
        raise ValueError("source commit must be a 40-character hexadecimal hash")
    if not artifact_set_names or len(set(artifact_set_names.values())) != len(artifact_set_names):
        raise ValueError("Artifact Set name map must be non-empty and unique")

    groups: dict[int, list[tuple[Path, dict[str, Any], dict[str, dict[str, Any]]]]] = {}
    challenge_ids: set[int] = set()
    for path in sorted(english_dir.glob("*.json")):
        english = read_json(path)
        if not isinstance(english, dict) or not is_artifact_challenge(english):
            continue
        challenge_id = english.get("id")
        entrance_id = english.get("entranceId")
        if (
            not isinstance(challenge_id, int)
            or challenge_id <= 0
            or challenge_id in challenge_ids
            or not isinstance(entrance_id, int)
            or entrance_id <= 0
        ):
            raise ValueError(f"{path.stem} has invalid Domain identity")
        localized = {}
        for locale in LOCALES.values():
            payload = read_json(source_root / "src" / "data" / locale / "domains" / path.name)
            if not isinstance(payload, dict) or payload.get("id") != challenge_id or payload.get("entranceId") != entrance_id:
                raise ValueError(f"{locale}/{path.name} has mismatched Domain identity")
            localized[locale] = payload
        groups.setdefault(entrance_id, []).append((path, english, localized))
        challenge_ids.add(challenge_id)

    domains = []
    for entrance_id, stages in groups.items():
        stages.sort(key=lambda item: (item[1].get("recommendedLevel", 0), item[1]["id"]))
        highest = stages[-1][1]
        english_name = required_text(highest.get("entranceName"), "English entrance name", str(entrance_id))
        region = required_text(highest.get("regionName"), "region", english_name)
        source_key = f"genshin-data-artifact-domain-{entrance_id}"
        localized_names = {}
        for field, locale in LOCALES.items():
            names = {
                required_text(localized[locale].get("entranceName"), f"{locale} entrance name", source_key)
                for _, _, localized in stages
            }
            if len(names) != 1:
                raise ValueError(f"{source_key} has inconsistent {locale} entrance names")
            localized_names[field] = names.pop()
        if any(required_text(stage.get("entranceName"), "entrance name", source_key) != english_name for _, stage, _ in stages):
            raise ValueError(f"{source_key} has inconsistent English entrance names")

        hosted_names = set()
        for _, stage, _ in stages:
            rewards = stage.get("rewardPreview")
            if not isinstance(rewards, list):
                raise ValueError(f"{source_key} has invalid reward preview")
            for reward in rewards:
                if isinstance(reward, dict) and reward.get("name") in artifact_set_names:
                    hosted_names.add(reward["name"])
        if not hosted_names:
            raise ValueError(f"{source_key} has no recognized hosted Artifact Sets")
        hosted_names = sorted(hosted_names, key=str.casefold)
        recommended_elements = sorted(
            {
                element
                for _, stage, _ in stages
                for element in stage.get("recommendedElements", [])
                if isinstance(element, str) and element.strip()
            },
            key=str.casefold,
        )
        domains.append(
            {
                "source_key": source_key,
                "game_data_entrance_id": entrance_id,
                "official_english_name": english_name,
                **localized_names,
                "official_english_description": required_text(
                    highest.get("description"), "English description", source_key
                ),
                "region": region,
                "challenge_stage_ids": [stage["id"] for _, stage, _ in stages],
                "challenge_stage_names": [
                    required_text(stage.get("name"), "challenge stage name", source_key)
                    for _, stage, _ in stages
                ],
                "recommended_levels": sorted(
                    {stage["recommendedLevel"] for _, stage, _ in stages if isinstance(stage.get("recommendedLevel"), int)}
                ),
                "minimum_unlock_rank": min(
                    stage["unlockRank"] for _, stage, _ in stages if isinstance(stage.get("unlockRank"), int)
                ),
                "recommended_elements": recommended_elements,
                "hosted_artifact_set_names": hosted_names,
                "hosted_artifact_set_source_keys": [artifact_set_names[name] for name in hosted_names],
            }
        )
    domains.sort(key=lambda record: (record["official_english_name"].casefold(), record["source_key"]))
    return {
        "schema": SCHEMA,
        "checked_at": checked_at,
        "source": {"url": SOURCE_URL, "commit": source_commit},
        "catalog_scope": {
            "domain_entrances": len(domains),
            "challenge_stages": len(challenge_ids),
            "domain_type": "UI_ABYSSUS_RELIC",
        },
        "domains": domains,
    }


def write_snapshot(snapshot: dict[str, Any], output: Path, *, allow_shrink: bool) -> None:
    if output.exists() and not allow_shrink:
        existing = read_json(output)
        for field in ("domain_entrances", "challenge_stages"):
            previous = existing.get("catalog_scope", {}).get(field)
            current = snapshot["catalog_scope"][field]
            if isinstance(previous, int) and current < previous:
                raise ValueError(f"refusing to shrink {field} from {previous} to {current}")
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{output.name}.", suffix=".tmp", dir=output.parent)
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
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--source-commit", default="8b15995fa220c88a4d0d7ffe1e21b041d0b32588")
    parser.add_argument("--artifact-snapshot", type=Path, default=module_root / "data" / "artifacts.json")
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--allow-shrink", action="store_true")
    parser.add_argument("--output", type=Path, default=module_root / "data" / "artifact-domains.json")
    args = parser.parse_args()
    artifact_snapshot = read_json(args.artifact_snapshot)
    if not isinstance(artifact_snapshot, dict) or artifact_snapshot.get("schema") != ARTIFACT_SCHEMA:
        raise ValueError("Artifact snapshot has an unsupported schema")
    sets = artifact_snapshot.get("artifact_sets")
    if not isinstance(sets, list):
        raise ValueError("Artifact snapshot has no Artifact Sets")
    name_map = {record["official_english_name"]: record["source_key"] for record in sets}
    snapshot = build_snapshot(
        args.source_root.resolve(),
        artifact_set_names=name_map,
        checked_at=args.checked_at,
        source_commit=args.source_commit,
    )
    write_snapshot(snapshot, args.output.resolve(), allow_shrink=args.allow_shrink)
    print(
        f"wrote {len(snapshot['domains'])} Artifact Domain entrances from "
        f"{snapshot['catalog_scope']['challenge_stages']} challenge stages to {args.output}"
    )


if __name__ == "__main__":
    main()
