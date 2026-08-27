#!/usr/bin/env python3
"""Build first-class AKE Gear Set and production Essence Wiki snapshots.

The source checkout is read only. Internal test presets and tutorial-only presets
are deliberately excluded; ordinary Item and Gear-piece rows remain untouched.
"""

from __future__ import annotations

import argparse
from datetime import date
import importlib.util
import json
from pathlib import Path
import subprocess
from typing import Any


CLIENT_DATA_COMMIT = "90bc55768143f1998ad02d03ed0042919135979f"
WEAPON_TYPES = {
    "sword": "Sword",
    "claym": "Greatsword",
    "lance": "Polearm",
    "pistol": "Handcannon",
    "funnel": "Arts Unit",
}
TERM_KINDS = {
    "0": "Primary attribute",
    "1": "Secondary attribute",
    "2": "Skill effect",
}


def read_table(root: Path, filename: str) -> dict[str, Any]:
    return json.loads((root / "tableCfg" / filename).read_text(encoding="utf-8"))


def client_commit(root: Path) -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=root, check=True, text=True,
        stdout=subprocess.PIPE,
    ).stdout.strip()


def localization_index(root: Path) -> dict[tuple[str, str], dict[str, str]]:
    files = {
        "en": read_table(root, "I18nTextTable_EN.json"),
        "cn": read_table(root, "I18nTextTable_CN.json"),
        "vi": read_table(root, "I18nTextTable_VN.json"),
        "tc": read_table(root, "I18nTextTable_TC.json"),
    }
    index: dict[tuple[str, str], dict[str, str]] = {}
    for key, english in files["en"].items():
        chinese = files["cn"].get(key)
        if not chinese:
            continue
        pair = (english, chinese)
        localized = index.setdefault(pair, {})
        for language in ("vi", "tc"):
            value = files[language].get(key)
            if value and value not in (english, chinese):
                localized.setdefault(language, value)
    return index


def localized(index: dict, names: dict[str, str]) -> dict[str, str | None]:
    values = index.get((names["en"], names["cn"]), {})
    return {
        "official_vietnamese_name": values.get("vi"),
        "official_traditional_chinese_name": values.get("tc"),
    }


def clean_markup(value: str) -> str:
    # Preserve source placeholders but remove the client's rich-text tags.
    import re
    return re.sub(r"<@[^>]+>|</>", "", value).strip()


def han_viet_tools():
    path = Path(__file__).with_name("build-items-snapshot.py")
    spec = importlib.util.spec_from_file_location("endfield_item_snapshot_tools", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    # Contextual selection: 幕 means a curtain/veil in 夜幕, hence "mạc".
    module.HAN_VIET_READING_OVERRIDES[("夜幕", 1)] = "mạc"
    return module


def dictionary_names(root: Path) -> list[str]:
    suits = read_table(root, "EquipSuitTable.json")
    items = read_table(root, "ItemTable.json")
    presets = read_table(root, "GemPresetTable.json")
    terms = read_table(root, "GemTable.json")
    values = [value["list"][0]["suitName"]["cn"] for value in suits.values()]
    for source_key, preset in presets.items():
        if source_key.startswith(("gem_test_", "gem_guide_")):
            continue
        values.append(items[source_key]["name"]["cn"])
        values.extend(terms[entry["termId"]]["tagName"]["cn"] for entry in preset["termList"])
    return list(dict.fromkeys(values))


def build_snapshot(
    root: Path,
    checked_at: str | None = None,
    dictionary_response: Path | None = None,
) -> dict[str, Any]:
    suits = read_table(root, "EquipSuitTable.json")
    items = read_table(root, "ItemTable.json")
    skills = read_table(root, "SkillPatchTable.json")
    presets = read_table(root, "GemPresetTable.json")
    terms = read_table(root, "GemTable.json")
    compatibility = read_table(root, "GemTagKeyToWeaponTable.json")
    try:
        locale_index = localization_index(root)
    except FileNotFoundError:
        locale_index = {}

    transcriptions = {}
    if dictionary_response is not None:
        tools = han_viet_tools()
        names = dictionary_names(root)
        transcriptions = tools.split_dictionary_response(
            names, tools.read_dictionary_response(dictionary_response)
        )

    def transcribe(value: str) -> str | None:
        if value not in transcriptions:
            return None
        generated, unresolved = han_viet_tools().format_han_viet(value, transcriptions[value])
        if unresolved:
            raise ValueError(f"Unresolved Hán-Việt characters for {value}: {unresolved}")
        return generated

    gear_sets = []
    for source_key, value in sorted(suits.items()):
        definition = value["list"][0]
        names = definition["suitName"]
        skill_bundle = skills[definition["skillID"]]["SkillPatchDataBundle"]
        skill = next((entry for entry in skill_bundle if str(entry.get("level")) == "1"), skill_bundle[0])
        members = value.get("equipList", [])
        record = {
            "source_key": source_key,
            "official_english_name": names["en"],
            "official_original_name": names["cn"],
            **localized(locale_index, names),
            "han_viet_name": transcribe(names["cn"]),
            "set_size": int(definition["equipCnt"]),
            "client_logo_id": definition["suitLogoName"],
            "skill_id": definition["skillID"],
            "official_english_effect": clean_markup(skill["description"]["en"]),
            "official_original_effect": clean_markup(skill["description"]["cn"]),
            "member_item_ids": members,
            "member_names": [items[item_id]["name"] for item_id in members],
            "rarities": sorted({int(items[item_id]["rarity"]) for item_id in members}),
        }
        gear_sets.append(record)

    excluded_test = sum(source_key.startswith("gem_test_") for source_key in presets)
    excluded_guide = sum(source_key.startswith("gem_guide_") for source_key in presets)
    essences = []
    for source_key, preset in sorted(presets.items()):
        if source_key.startswith(("gem_test_", "gem_guide_")):
            continue
        item = items[source_key]
        term_records = []
        for entry in preset["termList"]:
            term = terms[entry["termId"]]
            term_records.append({
                "term_id": entry["termId"],
                "tag_id": term["tagId"],
                "level": int(entry["level"]),
                "kind": TERM_KINDS[str(term["termType"])],
                "official_english_name": term["tagName"]["en"],
                "official_original_name": term["tagName"]["cn"],
                **localized(locale_index, term["tagName"]),
                "han_viet_name": transcribe(term["tagName"]["cn"]),
            })
        tag_key = "+".join(term["tag_id"] for term in term_records)
        prefix = source_key.split("_", 2)[1]
        weapon_suffix = source_key.rsplit("_", 1)[0].removeprefix(f"gem_{prefix}_")
        weapon_id = f"wpn_{prefix}_{weapon_suffix}"
        weapon_name = items[weapon_id]["name"]["en"]
        names = item["name"]
        effect_label = " · ".join(
            f"{term['official_english_name']} Lv.{term['level']}" for term in term_records
        )
        han_viet_name = transcribe(names["cn"])
        catalog_han_viet_label = None
        if han_viet_name and all(term["han_viet_name"] for term in term_records):
            catalog_han_viet_label = f"{han_viet_name} — " + " · ".join(
                term["han_viet_name"] for term in term_records
            )
        essences.append({
            "source_key": source_key,
            "official_english_name": names["en"],
            "official_original_name": names["cn"],
            **localized(locale_index, names),
            "han_viet_name": han_viet_name,
            "catalog_label": f"{names['en']} — {weapon_name} — {effect_label}",
            "catalog_han_viet_label": catalog_han_viet_label,
            "rarity": int(preset["rarity"]),
            "client_icon_id": item["iconId"],
            "domain_id": preset["domainId"],
            "weapon_type": WEAPON_TYPES[prefix],
            "weapon_id": weapon_id,
            "weapon_name": weapon_name,
            "tag_key": tag_key,
            "compatible_weapon_ids": compatibility.get(tag_key, {}).get("list", []),
            "term_kinds": [term["kind"] for term in term_records],
            "terms": term_records,
        })

    source_base = f"https://github.com/555me/beyondGameData/blob/{CLIENT_DATA_COMMIT}/tableCfg"
    return {
        "schema": "perwiga.endfield.gear-essence.v1",
        "checked_at": checked_at or date.today().isoformat(),
        "client_data_commit": CLIENT_DATA_COMMIT,
        "catalog_scope": {
            "gear_sets": len(gear_sets),
            "essences": len(essences),
            "excluded_test_essences": excluded_test,
            "excluded_guide_essences": excluded_guide,
        },
        "sources": {
            "client_data": "https://github.com/555me/beyondGameData",
            "gear_sets": f"{source_base}/EquipSuitTable.json",
            "gear_pieces": f"{source_base}/EquipTable.json",
            "items": f"{source_base}/ItemTable.json",
            "set_effects": f"{source_base}/SkillPatchTable.json",
            "essence_presets": f"{source_base}/GemPresetTable.json",
            "essence_terms": f"{source_base}/GemTable.json",
            "essence_weapon_compatibility": f"{source_base}/GemTagKeyToWeaponTable.json",
            "han_viet_dictionary": "https://hvdic.thivien.net/transcript.php#trans",
        },
        "notes": [
            "Gear Sets are first-class suit records; existing individual Item/Gear rows are retained unchanged.",
            "Essences include production GemPresetTable rows only. Internal gem_test_* and tutorial-only gem_guide_* rows are excluded.",
            "catalog_label is a UI label composed from independently official English rarity and effect-term names; it is not claimed as an official item name.",
        ],
        "gear_sets": gear_sets,
        "essences": essences,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--client-data", required=True, type=Path)
    parser.add_argument("--dictionary-response", type=Path)
    parser.add_argument("--emit-dictionary-input", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    actual = client_commit(args.client_data)
    if actual != CLIENT_DATA_COMMIT:
        raise SystemExit(f"Expected client commit {CLIENT_DATA_COMMIT}, found {actual}")
    if args.emit_dictionary_input:
        args.emit_dictionary_input.write_text(
            "\n".join(dictionary_names(args.client_data)), encoding="utf-8"
        )
    if args.output:
        if args.dictionary_response is None:
            raise SystemExit("--output requires --dictionary-response")
        snapshot = build_snapshot(args.client_data, dictionary_response=args.dictionary_response)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"Wrote {len(snapshot['gear_sets'])} gear sets and {len(snapshot['essences'])} essences")
    if not args.emit_dictionary_input and not args.output:
        raise SystemExit("Select --emit-dictionary-input and/or --output")


if __name__ == "__main__":
    main()
