#!/usr/bin/env python3
"""Build a reviewable Genshin weapon snapshot from a pinned genshin-db checkout.

The checkout supplies game-data extracts, localized strings, release metadata,
and first-party HoYoverse image URLs. This script never opens SQLite. Snapshot
writes are atomic and refuse an unexpected catalog contraction by default.
"""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import subprocess
import tempfile
from typing import Any
from urllib.parse import urlparse


SCHEMA = "perwiga.genshin-impact.weapons.v1"
SOURCE_URL = "https://github.com/theBowja/genshin-db"
ALLOWED_IMAGE_HOSTS = frozenset({"upload-os-bbs.mihoyo.com", "enka.network"})
LOCALES = {
    "official_simplified_chinese_name": "ChineseSimplified",
    "official_traditional_chinese_name": "ChineseTraditional",
    "official_vietnamese_name": "Vietnamese",
}
SUBSTATS = {
    None: "None",
    "FIGHT_PROP_ATTACK_PERCENT": "ATK",
    "FIGHT_PROP_CHARGE_EFFICIENCY": "Energy Recharge",
    "FIGHT_PROP_CRITICAL": "CRIT Rate",
    "FIGHT_PROP_CRITICAL_HURT": "CRIT DMG",
    "FIGHT_PROP_DEFENSE_PERCENT": "DEF",
    "FIGHT_PROP_ELEMENT_MASTERY": "Elemental Mastery",
    "FIGHT_PROP_HP_PERCENT": "HP",
    "FIGHT_PROP_PHYSICAL_ADD_HURT": "Physical DMG Bonus",
}
ASCENSION_REGION_BY_FIRST_MATERIAL = {
    "Tile of Decarabian's Tower": "Mondstadt",
    "Boreal Wolf's Milk Tooth": "Mondstadt",
    "Fetters of the Dandelion Gladiator": "Mondstadt",
    "Luminous Sands from Guyun": "Liyue",
    "Mist Veiled Lead Elixir": "Liyue",
    "Grain of Aerosiderite": "Liyue",
    "Coral Branch of a Distant Sea": "Inazuma",
    "Narukami's Wisdom": "Inazuma",
    "Mask of the Wicked Lieutenant": "Inazuma",
    "Copper Talisman of the Forest Dew": "Sumeru",
    "Oasis Garden's Reminiscence": "Sumeru",
    "Echo of Scorching Might": "Sumeru",
    "Fragment of an Ancient Chord": "Fontaine",
    "Dross of Pure Sacred Dewdrop": "Fontaine",
    "Broken Goblet of the Pristine Sea": "Fontaine",
    "Blazing Sacrificial Heart's Terror": "Natlan",
    "Delirious Decadence of the Sacred Lord": "Natlan",
    "Night-Wind's Mystic Consideration": "Natlan",
    # Nod-Krai and the Far North are grouped under the module's broad
    # Snezhnaya region taxonomy, matching the official character directory.
    "Ember of Long Night Flint": "Snezhnaya",
    "Artful Device Fragment": "Snezhnaya",
    "Measured Pour of the Cellared Spiritual Nectar": "Snezhnaya",
    "Rise of the Pale Star Army": "Snezhnaya",
    "Sundered Glory of the Far-North Scions": "Snezhnaya",
    "The Frost Emperor's Revival": "Snezhnaya",
}


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"unable to read valid JSON from {path}: {error}") from error


def required_text(value: Any, field: str, source_name: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{source_name} has invalid {field}")
    return value.strip()


def localized_name(source_root: Path, locale: str, filename: str) -> str:
    payload = read_json(source_root / "src/data" / locale / "weapons" / filename)
    if not isinstance(payload, dict):
        raise ValueError(f"{locale}/{filename} must contain an object")
    return required_text(payload.get("name"), f"{locale} name", filename)


def validate_image_url(value: Any, source_name: str) -> str:
    url = required_text(value, "first-party image URL", source_name)
    parsed = urlparse(url)
    if (
        parsed.scheme != "https"
        or parsed.hostname not in ALLOWED_IMAGE_HOSTS
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
        or not parsed.path.endswith(".png")
    ):
        raise ValueError(f"{source_name} has untrusted image URL: {url}")
    return url


def build_snapshot(
    source_root: Path, *, checked_at: str, source_commit: str
) -> dict[str, Any]:
    english_dir = source_root / "src/data/English/weapons"
    image_index = read_json(source_root / "src/data/image/weapons.json")
    version_index = read_json(source_root / "src/data/version/weapons.json")
    if not english_dir.is_dir() or not isinstance(image_index, dict) or not isinstance(version_index, dict):
        raise ValueError("source root is not a complete genshin-db checkout")

    weapons: list[dict[str, Any]] = []
    excluded = 0
    ids: set[int] = set()
    source_names: set[str] = set()
    for path in sorted(english_dir.glob("*.json")):
        source_name = path.stem
        payload = read_json(path)
        if not isinstance(payload, dict):
            raise ValueError(f"{path} must contain an object")
        if "dupealias" in payload:
            excluded += 1
            continue

        weapon_id = payload.get("id")
        if not isinstance(weapon_id, int) or weapon_id <= 0 or weapon_id in ids:
            raise ValueError(f"{source_name} has invalid or duplicate weapon id")
        ids.add(weapon_id)
        if source_name in source_names:
            raise ValueError(f"duplicate weapon source name: {source_name}")
        source_names.add(source_name)

        rarity = payload.get("rarity")
        if rarity not in {1, 2, 3, 4, 5}:
            raise ValueError(f"{source_name} has invalid rarity")
        weapon_type = payload.get("weaponText")
        if weapon_type not in {"Sword", "Claymore", "Polearm", "Catalyst", "Bow"}:
            raise ValueError(f"{source_name} has invalid weapon type")
        base_attack_raw = payload.get("baseAtkValue")
        if not isinstance(base_attack_raw, (int, float)) or base_attack_raw <= 0:
            raise ValueError(f"{source_name} has invalid Base ATK")
        base_attack = round(base_attack_raw)
        stat_type = payload.get("mainStatType")
        if stat_type not in SUBSTATS:
            raise ValueError(f"{source_name} has unsupported substat {stat_type!r}")

        ascend1 = payload.get("costs", {}).get("ascend1")
        if not isinstance(ascend1, list) or len(ascend1) < 2 or not isinstance(ascend1[1], dict):
            raise ValueError(f"{source_name} has no ascension material")
        ascension_material = ascend1[1].get("name")
        if ascension_material not in ASCENSION_REGION_BY_FIRST_MATERIAL:
            raise ValueError(
                f"{source_name} has unmapped ascension material: {ascension_material!r}"
            )

        images = image_index.get(source_name)
        if not isinstance(images, dict):
            raise ValueError(f"{source_name} has no image metadata")
        image_url = validate_image_url(images.get("mihoyo_icon"), source_name)
        image_filename = required_text(images.get("filename_icon"), "image filename", source_name)
        if not image_filename.replace("_", "").isalnum():
            raise ValueError(f"{source_name} has unsafe image filename")
        fallback_image_url = validate_image_url(
            f"https://enka.network/ui/{image_filename}.png", source_name
        )
        version = required_text(version_index.get(source_name), "release version", source_name)
        localized = {
            field: localized_name(source_root, locale, path.name)
            for field, locale in LOCALES.items()
        }
        source_key = f"genshin-data-weapon-{weapon_id}"
        weapons.append(
            {
                "source_key": source_key,
                "game_data_id": weapon_id,
                "genshin_db_key": source_name,
                "official_english_name": required_text(payload.get("name"), "English name", source_name),
                **localized,
                "official_english_description": required_text(payload.get("description"), "English description", source_name),
                "weapon_type": weapon_type,
                "rarity": rarity,
                "base_attack": base_attack,
                "substat": SUBSTATS[stat_type],
                "base_substat_value": payload.get("baseStatText") or None,
                "ascension_region": ASCENSION_REGION_BY_FIRST_MATERIAL[ascension_material],
                "ascension_material": ascension_material,
                "release_version": version,
                "thumbnail_asset": f"{source_key}.png",
                "thumbnail_source_url": image_url,
                "thumbnail_fallback_url": fallback_image_url,
            }
        )

    weapons.sort(key=lambda weapon: (weapon["official_english_name"].casefold(), weapon["game_data_id"]))
    return {
        "schema": SCHEMA,
        "checked_at": checked_at,
        "source": {
            "url": SOURCE_URL,
            "commit": source_commit,
            "upstream": "GenshinData game-data extracts with genshin-db localization and image indexes",
        },
        "catalog_scope": {
            "included_records": len(weapons),
            "excluded_non_obtainable_records": excluded,
        },
        "field_provenance": {
            "names_stats_types_rarity": "GenshinData game-data extracts distributed by genshin-db",
            "release_version": "genshin-db version index",
            "thumbnail": "first-party HoYoverse image URL indexed by genshin-db, with an explicit Enka Network game-asset mirror fallback",
            "ascension_region": "derived from the first weapon-ascension material family; Nod-Krai/Far North are grouped under Snezhnaya",
        },
        "weapons": weapons,
    }


def write_snapshot(snapshot: dict[str, Any], output: Path, *, allow_shrink: bool) -> None:
    if output.exists() and not allow_shrink:
        existing = read_json(output)
        old_count = existing.get("catalog_scope", {}).get("included_records") if isinstance(existing, dict) else None
        new_count = snapshot["catalog_scope"]["included_records"]
        if isinstance(old_count, int) and new_count < old_count:
            raise ValueError(f"refusing to shrink weapon snapshot from {old_count} to {new_count} records")
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


def checkout_commit(source_root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(source_root), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError("--source-root must be a Git checkout, or pass --source-commit") from error


def main() -> None:
    module_dir = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--source-commit")
    parser.add_argument("--checked-at", default=date.today().isoformat())
    parser.add_argument("--output", type=Path, default=module_dir / "data/weapons.json")
    parser.add_argument("--allow-shrink", action="store_true")
    args = parser.parse_args()
    snapshot = build_snapshot(
        args.source_root.resolve(),
        checked_at=args.checked_at,
        source_commit=args.source_commit or checkout_commit(args.source_root.resolve()),
    )
    write_snapshot(snapshot, args.output.resolve(), allow_shrink=args.allow_shrink)
    print(f"wrote {snapshot['catalog_scope']['included_records']} weapons to {args.output}")


if __name__ == "__main__":
    main()
