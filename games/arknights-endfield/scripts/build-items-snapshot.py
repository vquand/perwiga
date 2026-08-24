#!/usr/bin/env python3
"""Build the curated Endfield Item Files snapshot from extracted client tables.

This script is intentionally read-only with respect to the client-data checkout. It
can first emit a newline-delimited dictionary request and then consume the JSON
response returned by the Thi Vien Han-Nom transcription endpoint.
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


CLIENT_DATA_COMMIT = "21d6f91ede6e91cbd4b3f2dde3f32774b6fbf45f"

CLASSIFICATION_ORDER = [
    "collectible",
    "consumable",
    "craftable",
    "crafting-ingredient",
    "equipment",
    "material",
    "progression-material",
    "gatherable",
    "natural-resource",
    "plant",
    "mineral",
    "aic-product",
    "facility",
    "logistics",
    "tactical-item",
    "functional-item",
    "personal-device",
    "gift",
    "ornamental",
    "mission-item",
    "currency",
    "permit",
    "seed",
    "essence-material",
    "valuable",
]

CLASSIFICATION_DEFINITIONS = [
    {
        "key": "collectible",
        "label": "Collectible",
        "derivation": "Official Collectible item type, or an Ornamental exposed by Item Files.",
    },
    {
        "key": "consumable",
        "label": "Consumable",
        "derivation": "Official Consumable item type, Usables showing type, or Usable Items database group.",
    },
    {
        "key": "craftable",
        "label": "Craftable",
        "derivation": "Explicit outcome of a current factory, manual-craft, gear-assembly, Dijiang manufacture, or cultivation formula.",
    },
    {
        "key": "crafting-ingredient",
        "label": "Crafting ingredient",
        "derivation": "Explicit material, seed, currency, or ingredient consumed by a current formula.",
    },
    {
        "key": "equipment",
        "label": "Equipment",
        "derivation": "Official Gear item type or gear database/showing group.",
    },
    {
        "key": "material",
        "label": "Material",
        "derivation": "Official material/progression type or a material, resource, gatherable, progression, or AIC-product client grouping.",
    },
    {
        "key": "progression-material",
        "label": "Progression material",
        "derivation": "Official Operator, Weapon, Essence, or Rare Progression Material type, or a progression/rare-material client grouping.",
    },
    {
        "key": "gatherable",
        "label": "Gatherable",
        "derivation": "Official Gatherables database/showing group, including mineral and plant showing types.",
    },
    {
        "key": "natural-resource",
        "label": "Natural resource",
        "derivation": "Official Natural Resources database group.",
    },
    {
        "key": "plant",
        "label": "Plant",
        "derivation": "Official Plants showing type.",
    },
    {
        "key": "mineral",
        "label": "Mineral",
        "derivation": "Official Minerals showing type.",
    },
    {
        "key": "aic-product",
        "label": "AIC product",
        "derivation": "Official Products showing type or AIC Products database group.",
    },
    {
        "key": "facility",
        "label": "Facility",
        "derivation": "Official Basic Facility item type, Production showing type, or any building database group.",
    },
    {
        "key": "logistics",
        "label": "Logistics",
        "derivation": "Official Logistics item type or Logistics Units database group.",
    },
    {
        "key": "tactical-item",
        "label": "Tactical item",
        "derivation": "Official Tactical item type.",
    },
    {
        "key": "functional-item",
        "label": "Functional item",
        "derivation": "Official Miscellaneous/functional item type or Functional Items database group.",
    },
    {
        "key": "personal-device",
        "label": "Personal device",
        "derivation": "Official Personal Device item type, showing type, or database group.",
    },
    {
        "key": "gift",
        "label": "Gift",
        "derivation": "Official Gift item type or Operator Gifts database group.",
    },
    {
        "key": "ornamental",
        "label": "Ornamental",
        "derivation": "Official Ornamental item type or Ornamental building database group.",
    },
    {
        "key": "mission-item",
        "label": "Mission item",
        "derivation": "Official Mission Item item type.",
    },
    {
        "key": "currency",
        "label": "Currency",
        "derivation": "Official Currency, Event Currency, or Stock Bill item type.",
    },
    {
        "key": "permit",
        "label": "Permit",
        "derivation": "Official Engraving Permit item type.",
    },
    {
        "key": "seed",
        "label": "Seed",
        "derivation": "Official Seed item type.",
    },
    {
        "key": "essence-material",
        "label": "Essence material",
        "derivation": "Official Essence Material item type.",
    },
    {
        "key": "valuable",
        "label": "Valuable",
        "derivation": "Official Valuable item type.",
    },
]

VIETNAMESE_NAME_OVERRIDES = {
    "item_device_xiranite_nexus_3": "Trung Tâm Xiranite XC-Ⅱ",
    "item_equip_t4_suit_crush_fracture_body_01": "Giáp Lão Luyện",
    "item_liquid_sewage": "Nước Thải",
    "item_originium_enr_powder": "Bột Originium Đặc",
}

# Contextual selections are keyed by (full Simplified Chinese item name,
# zero-based character index). Values must occur in the dictionary alternatives
# for that exact character; the builder rejects an unattested override.
HAN_VIET_READING_OVERRIDES: dict[tuple[str, int], str] = {}

# These characters occur only in the listed semantic reading throughout the
# scoped Item Files names. Keeping this map explicit avoids accepting the
# dictionary's popularity-first choice when the item-name context requires a
# different attested reading.
HAN_VIET_READING_OVERRIDES_BY_CHARACTER = {
    "仓": "thương",  # warehouse/storage
    "传": "truyền",  # transmission
    "使": "sứ",  # envoy/messenger; also 天使
    "参": "sâm",  # ginseng
    "单": "đơn",  # unit
    "受": "thụ",  # affected by
    "处": "xử",  # processing
    "存": "tồn",  # storage/existence (documents intent despite first match)
    "应": "ứng",  # response/emergency/Yinglung
    "弹": "đạn",  # projectile/grenade
    "微": "vi",  # micro-
    "时": "thời",  # time
    "摆": "bãi",  # swing
    "数": "số",  # points/count
    "摇": "diêu",  # swing
    "汇": "hội",  # confluence
    "爆": "bạo",  # explosive/burst
    "盾": "thuẫn",  # shield
    "瞭": "liệu",  # lookout
    "索": "sách",  # cable/zipline
    "调": "điều",  # scheduling
    "遴": "lân",  # careful selection
    "长": "trường",  # long
    "难": "nạn",  # difficulty/hardship
    "储": "trữ",  # storage
    "仿": "phỏng",  # imitate
    "耶": "da",  # phonetic Ye- in Yersh
    "重": "trọng",  # heavy/dense
}


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(value, handle, ensure_ascii=False, indent=2)
        handle.write("\n")


def write_text(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        handle.write(value)


def client_commit(client_root: Path) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=client_root,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def table(client_root: Path, name: str) -> Any:
    return read_json(client_root / "tableCfg" / name)


def add_evidence(
    evidence: dict[str, set[str]], item_id: str, table_name: str, formula_id: str
) -> None:
    if item_id:
        evidence[item_id].add(f"{table_name}:{formula_id}")


def recipe_evidence(client_root: Path) -> tuple[dict[str, set[str]], dict[str, set[str]]]:
    produced: dict[str, set[str]] = defaultdict(set)
    consumed: dict[str, set[str]] = defaultdict(set)

    for table_name in (
        "FactoryManualCraftTable.json",
        "FactoryHubCraftTable.json",
    ):
        for formula_id, formula in table(client_root, table_name).items():
            for outcome in formula.get("outcomes", []):
                add_evidence(produced, outcome.get("id", ""), table_name, formula_id)
            for ingredient in formula.get("ingredients", []):
                add_evidence(consumed, ingredient.get("id", ""), table_name, formula_id)

    machine_table_name = "FactoryMachineCraftTable.json"
    for formula_id, formula in table(client_root, machine_table_name).items():
        for outcome_group in formula.get("outcomes", []):
            for outcome in outcome_group.get("group", []):
                add_evidence(produced, outcome.get("id", ""), machine_table_name, formula_id)
        for ingredient_group in formula.get("ingredients", []):
            for ingredient in ingredient_group.get("group", []):
                add_evidence(consumed, ingredient.get("id", ""), machine_table_name, formula_id)

    equip_table_name = "EquipFormulaTable.json"
    equip_chains = table(client_root, "EquipFormulaChainTable.json")
    for formula_id, formula in table(client_root, equip_table_name).items():
        add_evidence(produced, formula.get("outcomeEquipId", ""), equip_table_name, formula_id)
        for chain in equip_chains.get(formula.get("level", ""), {}).get("chainList", []):
            for item_id in chain.get("costItemId", []):
                add_evidence(consumed, item_id, equip_table_name, formula_id)
            add_evidence(consumed, chain.get("costGoldId", ""), equip_table_name, formula_id)

    manufacture_table_name = "SpaceshipManufactureFormulaTable.json"
    for formula_id, formula in table(client_root, manufacture_table_name).items():
        add_evidence(
            produced,
            formula.get("outcomeItemId", ""),
            manufacture_table_name,
            formula_id,
        )

    grow_table_name = "SpaceshipGrowCabinFormulaTable.json"
    for formula_id, formula in table(client_root, grow_table_name).items():
        add_evidence(produced, formula.get("outcomeItemId", ""), grow_table_name, formula_id)
        add_evidence(consumed, formula.get("seedItemId", ""), grow_table_name, formula_id)

    seed_table_name = "SpaceshipGrowCabinSeedFormulaTable.json"
    for formula_id, formula in table(client_root, seed_table_name).items():
        add_evidence(produced, formula.get("outcomeseedItemId", ""), seed_table_name, formula_id)
        add_evidence(consumed, formula.get("materialItemId", ""), seed_table_name, formula_id)

    return produced, consumed


def group_definitions(client_root: Path) -> tuple[dict[str, dict[str, str]], dict[str, int]]:
    definitions: dict[str, dict[str, str]] = {}
    order: dict[str, int] = {}
    index = 0
    for wiki_type in ("wiki_type_item", "wiki_type_equip", "wiki_type_building"):
        for group in table(client_root, "WikiGroupTable.json")[wiki_type]["list"]:
            group_id = group["groupId"]
            definitions[group_id] = {
                "english": group["groupName"]["en"],
                "simplified_chinese": group["groupName"]["cn"],
            }
            order[group_id] = index
            index += 1
    return definitions, order


def aligned_localizations(client_root: Path) -> dict[tuple[str, str], list[tuple[str, str]]]:
    english = table(client_root, "I18nTextTable_EN.json")
    chinese = table(client_root, "I18nTextTable_CN.json")
    vietnamese = table(client_root, "I18nTextTable_VN.json")
    aligned: dict[tuple[str, str], list[tuple[str, str]]] = defaultdict(list)
    for key, english_value in english.items():
        chinese_value = chinese.get(key)
        vietnamese_value = vietnamese.get(key)
        if chinese_value is not None and vietnamese_value is not None:
            aligned[(english_value, chinese_value)].append((key, vietnamese_value))
    return aligned


def choose_vietnamese_name(
    item_id: str, candidates: list[tuple[str, str]]
) -> tuple[str, str, list[dict[str, str]]]:
    if not candidates:
        raise ValueError(f"No aligned Vietnamese localization for {item_id}")
    preferred = VIETNAMESE_NAME_OVERRIDES.get(item_id)
    normalized_candidates = [(key, value.strip()) for key, value in candidates]
    values = {value for _, value in normalized_candidates}
    if preferred is None:
        if len(values) != 1:
            raise ValueError(
                f"Ambiguous Vietnamese localizations for {item_id}: {sorted(values)}"
            )
        preferred = next(iter(values))
    if preferred not in values:
        raise ValueError(f"Vietnamese override for {item_id} is not client-attested")
    primary_key = next(key for key, value in normalized_candidates if value == preferred)
    variants = [
        {"localization_key": key, "value": value}
        for key, value in normalized_candidates
        if value != preferred
    ]
    return preferred, primary_key, variants


def normalized_classifications(
    item_type: str,
    showing_type: str,
    group_id: str,
    is_produced: bool,
    is_consumed: bool,
) -> list[str]:
    classes: set[str] = set()

    if item_type in {"6", "112"} or showing_type in {"8", "9", "10"} or group_id == "wiki_group_equip_default":
        classes.add("equipment")
    if item_type in {"7", "8", "25", "26", "27", "61", "87", "95"}:
        classes.add("material")
    if item_type in {"7", "25", "26", "27", "61", "87", "95"}:
        classes.add("progression-material")
    if item_type == "61":
        classes.add("essence-material")
    if item_type == "9" or showing_type == "17" or group_id.startswith("wiki_group_building_"):
        classes.add("facility")
    if item_type == "11" or group_id == "wiki_group_building_logistic":
        classes.add("logistics")
    if item_type == "48":
        classes.add("tactical-item")
    if item_type == "52" or showing_type == "5" or group_id == "wiki_group_item_usable":
        classes.add("consumable")
    if item_type == "54" or group_id == "wiki_group_item_other":
        classes.add("functional-item")
    if item_type == "101" or showing_type == "18" or group_id == "wiki_group_item_portable_device":
        classes.add("personal-device")
    if item_type == "33" or group_id == "wiki_group_item_gift":
        classes.add("gift")
    if item_type == "111" or group_id == "wiki_group_building_decorate":
        classes.update({"collectible", "ornamental"})
    if item_type == "15":
        classes.add("collectible")
    if item_type == "13":
        classes.add("mission-item")
    if item_type in {"1", "2", "3", "53", "79"}:
        classes.add("currency")
    if item_type == "78":
        classes.add("permit")
    if item_type == "34":
        classes.update({"material", "seed"})
    if item_type in {"14", "29", "49", "50", "96", "98"}:
        classes.add("valuable")

    if showing_type in {"1", "2", "15", "16"}:
        classes.add("material")
    if showing_type in {"1", "2", "15"}:
        classes.add("gatherable")
    if showing_type == "1":
        classes.add("mineral")
    if showing_type == "2":
        classes.add("plant")
    if showing_type == "16":
        classes.add("progression-material")
    if showing_type == "4" or group_id == "wiki_group_item_product":
        classes.update({"aic-product", "material"})

    if group_id == "wiki_group_item_material":
        classes.update({"gatherable", "material"})
    if group_id == "wiki_group_item_nature":
        classes.update({"material", "natural-resource"})
    if group_id in {"wiki_group_item_special", "wiki_group_item_nurturance"}:
        classes.update({"material", "progression-material"})

    if is_produced:
        classes.add("craftable")
    if is_consumed:
        classes.add("crafting-ingredient")

    if not classes:
        raise ValueError(
            f"No normalized classification for type={item_type}, showing={showing_type}, group={group_id}"
        )
    rank = {name: index for index, name in enumerate(CLASSIFICATION_ORDER)}
    return sorted(classes, key=lambda value: (rank.get(value, len(rank)), value))


def is_han_character(value: str) -> bool:
    return len(value) == 1 and (
        "\u3400" <= value <= "\u4dbf"
        or "\u4e00" <= value <= "\u9fff"
        or "\uf900" <= value <= "\ufaff"
    )


def dictionary_alternatives(result: dict[str, Any]) -> list[str]:
    output = result.get("o", [])
    if isinstance(output, dict):
        output = output.get("by_views", next(iter(output.values()), []))
    return [str(value) for value in output]


def title_reading(value: str) -> str:
    return value[:1].upper() + value[1:]


def format_han_viet(name: str, results: list[dict[str, Any]]) -> tuple[str | None, list[str]]:
    tokens: list[tuple[str, bool]] = []
    unresolved: list[str] = []
    source_index = 0
    for result in results:
        source = result.get("i", "")
        for character in source:
            if is_han_character(character):
                alternatives = dictionary_alternatives(result)
                if not alternatives:
                    unresolved.append(character)
                    tokens.append((character, True))
                else:
                    selected = HAN_VIET_READING_OVERRIDES.get(
                        (name, source_index),
                        HAN_VIET_READING_OVERRIDES_BY_CHARACTER.get(
                            character, alternatives[0]
                        ),
                    )
                    if selected not in alternatives:
                        raise ValueError(
                            f"Unattested Hán-Việt override {selected!r} for {name!r} index {source_index}"
                        )
                    tokens.append((title_reading(selected), True))
            else:
                tokens.append((character, False))
            source_index += 1

    if "".join(result.get("i", "") for result in results) != name:
        raise ValueError(f"Dictionary result did not round-trip {name!r}")
    if unresolved:
        return None, sorted(set(unresolved))
    if not any(is_han for _, is_han in tokens):
        return None, []

    punctuation = {
        "：": ":",
        "，": ",",
        "。": ".",
        "！": "!",
        "？": "?",
        "（": "(",
        "）": ")",
        "【": "[",
        "】": "]",
        "《": "«",
        "》": "»",
    }
    output = ""
    previous_han = False
    for value, han in tokens:
        value = punctuation.get(value, value)
        if han:
            if output and not output.endswith((" ", "(", "[", "«", "'", '"', "·", "-")):
                output += " "
            output += value
        elif value in {",", ".", ":", ";", "!", "?", ")", "]", "»"}:
            output = output.rstrip() + value
            if value in {",", ":", ";"}:
                output += " "
        elif value in {"(", "[", "«"}:
            if output and not output.endswith(" "):
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
        previous_han = han
    return re.sub(r"\s+", " ", output).strip(), []


def split_dictionary_response(
    names: list[str], response: dict[str, Any]
) -> dict[str, list[dict[str, Any]]]:
    if response.get("message") != "OK" or response.get("mode") != "trans":
        raise ValueError("Dictionary response is not a successful transcription result")
    lines: list[list[dict[str, Any]]] = [[]]
    for result in response["result"]:
        source = result.get("i", "")
        if source in {"\r", "\n", "\r\n"}:
            if source != "\r":
                lines.append([])
        else:
            lines[-1].append(result)
    while lines and not lines[-1]:
        lines.pop()
    reconstructed = ["".join(result.get("i", "") for result in line) for line in lines]
    if reconstructed != names:
        raise ValueError(
            f"Dictionary input mismatch: expected {len(names)} lines, got {len(reconstructed)}"
        )
    return dict(zip(names, lines, strict=True))


def read_dictionary_response(path: Path) -> dict[str, Any]:
    if path.is_file():
        return read_json(path)
    response_paths = sorted(path.glob("response-*.json"))
    if not response_paths:
        raise ValueError(f"No dictionary response batches found in {path}")
    combined: dict[str, Any] = {
        "mode": "trans",
        "lang": "1",
        "input": [],
        "message": "OK",
        "result": [],
    }
    for response_path in response_paths:
        response = read_json(response_path)
        if response.get("message") != "OK":
            raise ValueError(f"Unsuccessful dictionary batch {response_path}")
        combined["input"].extend(response.get("input", []))
        combined["result"].extend(response.get("result", []))
    return combined


def scoped_items(client_root: Path) -> tuple[list[dict[str, Any]], list[str]]:
    item_table = table(client_root, "ItemTable.json")
    item_types = table(client_root, "ItemTypeTable.json")
    showing_types = table(client_root, "ItemShowingTypeTable.json")
    wiki_entries = table(client_root, "WikiEntryDataTable.json")
    item_gather_text = table(client_root, "ItemGatherTextTable.json")
    domains = table(client_root, "DomainDataTable.json")
    group_defs, group_order = group_definitions(client_root)
    localized = aligned_localizations(client_root)
    produced, consumed = recipe_evidence(client_root)

    records: list[dict[str, Any]] = []
    for entry in wiki_entries.values():
        item_id = entry.get("refItemId", "")
        if not item_id or item_id.startswith("wpn_"):
            continue
        item = item_table.get(item_id)
        if item is None:
            raise ValueError(f"Wiki entry references missing item {item_id}")
        item_type_id = item["type"]
        item_type = item_types[item_type_id]["name"]
        showing_type_id = item.get("showingType", "")
        showing = showing_types.get(showing_type_id, {}).get("name")
        group_id = entry["groupId"]
        if group_id not in group_defs:
            raise ValueError(f"Unrecognized scoped wiki group {group_id}")

        raw_english_name = item["name"]["en"]
        raw_original_name = item["name"]["cn"]
        english_name = raw_english_name.strip()
        original_name = raw_original_name.strip()
        candidates = localized[(raw_english_name, raw_original_name)]
        vietnamese, localization_key, variants = choose_vietnamese_name(item_id, candidates)
        classes = normalized_classifications(
            item_type_id,
            showing_type_id,
            group_id,
            item_id in produced,
            item_id in consumed,
        )
        record: dict[str, Any] = {
            "source_key": item_id,
            "client_item_id": item_id,
            "client_icon_id": item["iconId"],
            "client_sort_id_primary": int(item["sortId1"]),
            "client_sort_id_secondary": int(item["sortId2"]),
            "client_database_group": {
                "key": group_id,
                "english": group_defs[group_id]["english"],
                "simplified_chinese": group_defs[group_id]["simplified_chinese"],
                "order": int(entry["order"]),
            },
            "client_item_type": {
                "id": int(item_type_id),
                "english": item_type["en"],
                "simplified_chinese": item_type["cn"],
            },
            "client_showing_type": None,
            "rarity": int(item["rarity"]),
            "classifications": classes,
            "produced_by_formula_keys": sorted(produced.get(item_id, set())),
            "consumed_by_formula_keys": sorted(consumed.get(item_id, set())),
            "official_english_name": english_name,
            "official_original_name": original_name,
            "official_vietnamese_name": vietnamese,
            "han_viet_name": None,
            "status": "current",
            "name_evidence": (
                "exact-localization-value-normalized-whitespace"
                if english_name != raw_english_name or original_name != raw_original_name
                else "exact-localization-value"
            ),
            "localization_key": localization_key,
            "additional_localization_keys": sorted(
                key
                for key, value in candidates
                if key != localization_key and value.strip() == vietnamese
            ),
            "name_confidence": "B",
            "classification_confidence": "B",
        }
        gather_text = item_gather_text.get(item_id)
        if gather_text is not None:
            domain_id = gather_text["domainId"]
            domain = domains.get(domain_id)
            if domain is None:
                raise ValueError(f"Unrecognized item region {domain_id} for {item_id}")
            record["region"] = {
                "id": domain_id,
                "english": domain["domainName"]["en"],
                "simplified_chinese": domain["domainName"]["cn"],
                "evidence": "ItemGatherTextTable.domainId",
            }
        if showing is not None:
            record["client_showing_type"] = {
                "id": int(showing_type_id),
                "english": showing["en"],
                "simplified_chinese": showing["cn"],
            }
        if variants:
            record["official_vietnamese_name_variants"] = variants
        records.append(record)

    records.sort(
        key=lambda record: (
            group_order[record["client_database_group"]["key"]],
            record["client_database_group"]["order"],
            record["official_english_name"].casefold(),
            record["client_item_id"],
        )
    )
    for index, record in enumerate(records, start=1):
        record["catalog_index"] = index

    if len(records) != 731:
        raise ValueError(f"Expected 731 scoped non-weapon Item Files records, got {len(records)}")
    source_keys = [record["source_key"] for record in records]
    if len(source_keys) != len(set(source_keys)):
        raise ValueError("Scoped source keys are not unique")
    if any(not record["client_icon_id"] for record in records):
        raise ValueError("Every scoped Item Files record must have a client icon ID")
    regional_count = sum("region" in record for record in records)
    if regional_count != 45:
        raise ValueError(
            f"Expected 45 explicitly region-bound gatherable/drop records, got {regional_count}"
        )
    names = list(dict.fromkeys(record["official_original_name"] for record in records))
    return records, names


def build_snapshot(
    client_root: Path, dictionary_response_path: Path | None
) -> dict[str, Any]:
    records, names = scoped_items(client_root)
    unresolved: list[dict[str, Any]] = []
    if dictionary_response_path is not None:
        dictionary_results = split_dictionary_response(
            names, read_dictionary_response(dictionary_response_path)
        )
        for record in records:
            han_viet_name, unresolved_characters = format_han_viet(
                record["official_original_name"],
                dictionary_results[record["official_original_name"]],
            )
            record["han_viet_name"] = han_viet_name
            if unresolved_characters:
                record["han_viet_unresolved_characters"] = unresolved_characters
                unresolved.append(
                    {
                        "source_key": record["source_key"],
                        "field": "han_viet_name",
                        "reason": "The dictionary returned no Hán-Việt reading for one or more Han characters.",
                        "characters": unresolved_characters,
                    }
                )
            elif han_viet_name is None:
                unresolved.append(
                    {
                        "source_key": record["source_key"],
                        "field": "han_viet_name",
                        "reason": "The official Simplified Chinese localization contains no Han characters.",
                    }
                )

    source_base = f"https://github.com/555me/beyondGameData/blob/{CLIENT_DATA_COMMIT}/tableCfg"
    return {
        "schema": "perwiga.endfield.items.v1",
        "entity_type": "item",
        "checked_at": date.today().isoformat(),
        "client_data_commit": CLIENT_DATA_COMMIT,
        "catalog_scope": {
            "name": "Database - Item Files",
            "included_records": 731,
            "rule": "Current WikiEntryDataTable records with a non-empty refItemId that resolves in ItemTable, excluding the 77 weapon IDs curated separately.",
            "excluded_internal_item_table_records": 1889,
            "excluded_weapon_records": 77,
        },
        "confidence": {
            "item_roster": "B",
            "item_classification": "B",
            "official_localized_names": "B",
            "han_viet_names": "E",
        },
        "sources": {
            "localized_client_data": "https://github.com/555me/beyondGameData",
            "structured_item_data": f"{source_base}/ItemTable.json",
            "structured_item_type_data": f"{source_base}/ItemTypeTable.json",
            "structured_item_showing_type_data": f"{source_base}/ItemShowingTypeTable.json",
            "structured_item_region_data": f"{source_base}/ItemGatherTextTable.json",
            "structured_region_data": f"{source_base}/DomainDataTable.json",
            "structured_database_entry_data": f"{source_base}/WikiEntryDataTable.json",
            "structured_database_group_data": f"{source_base}/WikiGroupTable.json",
            "structured_crafting_data": f"{source_base}/FactoryManualCraftTable.json",
            "english_localization_data": f"{source_base}/I18nTextTable_EN.json",
            "simplified_chinese_localization_data": f"{source_base}/I18nTextTable_CN.json",
            "vietnamese_localization_data": f"{source_base}/I18nTextTable_VN.json",
            "official_homecoming_notes": "https://endfield.gryphline.com/en-us/news/5200",
            "item_catalog_cross_check": "https://endfield.wiki.gg/wiki/Category%3AItems",
            "item_system_cross_check": "https://endfield.wiki.gg/wiki/Item",
            "aic_product_cross_check": "https://endfield.wiki.gg/wiki/Item/AIC_Products",
            "item_thumbnail_asset_index": "https://endfield-assets.fffdan.com/vfs/Bundle/search/",
            "item_thumbnail_asset_root": "https://endfield-assets.fffdan.com/vfs/Bundle/file/assets/beyond/dynamicassets/gameplay/ui/sprites/itemicon/",
            "han_viet_dictionary": "https://hvdic.thivien.net/transcript.php#trans",
        },
        "classification_model": {
            "cardinality": "many",
            "storage": "Each item has a classifications array; classes are additive and non-exclusive.",
            "definitions": CLASSIFICATION_DEFINITIONS,
        },
        "notes": [
            "This initial snapshot intentionally follows the current in-game Database - Item Files boundary. It does not import hidden/internal ItemTable rows, profile cosmetics, every mission prop, or the 77 weapons already owned by the Weapon dataset.",
            "The 731 records are distinct by stable client item ID. Names are not identity keys: several fluid and gas container records intentionally share the same English and Simplified Chinese display name.",
            "classifications is a many-valued set. Client-backed identity classes such as material, facility, and equipment can coexist with behavior classes such as craftable, crafting-ingredient, consumable, and collectible.",
            "craftable and crafting-ingredient are derived only from explicit current client formula outcomes and costs across factory, manual crafting, gear assembly, Dijiang manufacture, and cultivation tables. Acquisition text alone is not treated as a recipe.",
            "English, Simplified Chinese, and Vietnamese names were selected independently by matching exact values at the same client localization key. Four casing or numeral-style variants are preserved without treating them as different item identities.",
            "Hán-Việt names were generated from the verified Simplified Chinese localization using readings returned by the Thi Viện Hán-Nôm dictionary. They are search aliases, not official Vietnamese localization, and have confidence E.",
            "The community item catalog is broader than the in-game Item Files scope and is used only as a taxonomy and coverage cross-check, not as the identity boundary for this snapshot.",
            "region is present only when ItemGatherTextTable explicitly assigns a client domainId. The current 45 records resolve through DomainDataTable to Valley IV or Wuling; no region is inferred from an item name or general availability.",
            "client_icon_id is the client-backed presentation key. Bundled thumbnails are fetched non-destructively from the indexed extracted asset path and are not naming evidence.",
        ],
        "unresolved": unresolved,
        "items": records,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--client-data", required=True, type=Path)
    parser.add_argument("--dictionary-response", type=Path)
    parser.add_argument("--emit-dictionary-input", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    actual_commit = client_commit(args.client_data)
    if actual_commit != CLIENT_DATA_COMMIT:
        raise SystemExit(
            f"Expected client-data commit {CLIENT_DATA_COMMIT}, found {actual_commit}"
        )
    records, names = scoped_items(args.client_data)
    if args.emit_dictionary_input is not None:
        write_text(args.emit_dictionary_input, "\n".join(names))
    if args.output is not None:
        if args.dictionary_response is None:
            raise SystemExit("--output requires --dictionary-response")
        write_json(args.output, build_snapshot(args.client_data, args.dictionary_response))
    if args.emit_dictionary_input is None and args.output is None:
        print(f"Validated {len(records)} scoped item records and {len(names)} unique Chinese names")


if __name__ == "__main__":
    main()
