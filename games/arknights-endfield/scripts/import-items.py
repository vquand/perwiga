#!/usr/bin/env python3
"""Add a curated Endfield item snapshot to an explicitly selected Perwiga DB.

The importer never updates or deletes existing wiki entities. A repeated import
only verifies source-marked rows and may add a missing alias.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from datetime import datetime, timezone
import json
from pathlib import Path
import sqlite3
import uuid


SOURCE_CHECKED_AT = "2026-08-24"
DICTIONARY_URL = "https://hvdic.thivien.net/transcript.php#trans"


def read_snapshot(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        snapshot = json.load(handle)
    if snapshot.get("schema") != "perwiga.endfield.items.v1":
        raise ValueError("Unsupported item snapshot schema")
    if len(snapshot.get("items", [])) != 731:
        raise ValueError("Expected the verified 731-record item snapshot")
    return snapshot


def now_timestamp() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace(
        "+00:00", "Z"
    )


def other_information(record: dict, snapshot: dict) -> str:
    showing = record.get("client_showing_type")
    showing_text = (
        f"{showing['english']} (client ID {showing['id']})"
        if showing is not None
        else "not assigned"
    )
    localization_keys = [record["localization_key"], *record["additional_localization_keys"]]
    classes = ", ".join(record["classifications"])
    return " ".join(
        [
            f"Curated source key: {record['source_key']}.",
            f"Catalog index: {record['catalog_index']:03d}/{snapshot['catalog_scope']['included_records']}.",
            f"Client item ID: {record['client_item_id']}.",
            (
                "Client taxonomy: "
                f"{record['client_item_type']['english']} item type "
                f"(client ID {record['client_item_type']['id']}); "
                f"{showing_text} showing type; "
                f"{record['client_database_group']['english']} database group."
            ),
            f"Rarity: {record['rarity']}-star; status: {record['status']}.",
            f"Classifications (multi-valued): {classes}.",
            (
                "Crafting evidence: "
                f"produced by {len(record['produced_by_formula_keys'])} explicit formula(s); "
                f"consumed by {len(record['consumed_by_formula_keys'])} explicit formula(s); "
                "formula keys are retained in the module snapshot."
            ),
            (
                f"Name evidence: {record['name_evidence']}; localization keys: "
                f"{', '.join(localization_keys)}."
            ),
            (
                f"Item classification confidence {record['classification_confidence']}; "
                f"official localized-name confidence {record['name_confidence']}."
            ),
            (
                "Hán-Việt is stored only as a generated alias with confidence E "
                "when every Han character has a dictionary-attested reading."
            ),
            (
                f"Sources checked {snapshot['checked_at']}: "
                f"{snapshot['sources']['structured_item_data']} ; "
                f"{snapshot['sources']['structured_database_entry_data']} ; "
                f"{snapshot['sources']['item_catalog_cross_check']}"
            ),
        ]
    )


def alias_rows(record: dict) -> list[tuple[str, str, str, str, str]]:
    aliases: list[tuple[str, str, str, str, str]] = []
    if record.get("han_viet_name"):
        aliases.append(
            (
                record["han_viet_name"],
                "vi",
                "han-viet",
                "Hán-Việt (generated)",
                (
                    "Generated from the verified Simplified Chinese item name using "
                    "readings returned by the Thi Viện Hán-Nôm dictionary; not an "
                    "official Vietnamese localization. Context-sensitive alternatives "
                    f"are explicitly selected by the snapshot builder. Source: {DICTIONARY_URL}. "
                    f"Checked {SOURCE_CHECKED_AT}. Confidence E (generated)."
                ),
            )
        )
    for variant in record.get("official_vietnamese_name_variants", []):
        aliases.append(
            (
                variant["value"],
                "vi",
                "official-localization-variant",
                "Official Vietnamese localization variant",
                (
                    "An alternate casing or numeral-style value aligned with the same "
                    f"English and Simplified Chinese item name at client localization key "
                    f"{variant['localization_key']}. Checked {SOURCE_CHECKED_AT}. Confidence B."
                ),
            )
        )
    return aliases


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--database", required=True, type=Path)
    parser.add_argument("--snapshot", required=True, type=Path)
    parser.add_argument("--work-id", required=True)
    args = parser.parse_args()

    database_path = args.database.resolve(strict=True)
    snapshot_path = args.snapshot.resolve(strict=True)
    if not database_path.is_file():
        raise SystemExit(f"Database is not a regular file: {database_path}")
    snapshot = read_snapshot(snapshot_path)

    connection = sqlite3.connect(f"file:{database_path}?mode=rw", uri=True)
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA foreign_keys = ON")
    if connection.execute("PRAGMA foreign_keys").fetchone()[0] != 1:
        raise RuntimeError("SQLite foreign-key enforcement did not enable")

    work = connection.execute(
        "SELECT work_kind, module_id, display_name FROM library_works WHERE id = ?",
        (args.work_id,),
    ).fetchone()
    if work is None or work["work_kind"] != "game" or work["module_id"] != "arknights-endfield":
        raise RuntimeError("The selected work is not the expected Arknights: Endfield game")

    existing_rows = connection.execute(
        """
        SELECT id, official_english_name, official_original_name,
               official_vietnamese_name, other_information
        FROM wiki_entities
        WHERE work_id = ? AND entity_type = 'item'
        """,
        (args.work_id,),
    ).fetchall()
    by_source: dict[str, list[sqlite3.Row]] = defaultdict(list)
    by_names: dict[tuple[str, str], list[sqlite3.Row]] = defaultdict(list)
    for row in existing_rows:
        by_names[(row["official_english_name"], row["official_original_name"])].append(row)
        information = row["other_information"] or ""
        for record in snapshot["items"]:
            marker = f"Curated source key: {record['source_key']}."
            if marker in information:
                by_source[record["source_key"]].append(row)

    inserted_entities = 0
    verified_entities = 0
    inserted_aliases = 0
    timestamp = now_timestamp()
    connection.execute("BEGIN IMMEDIATE")
    try:
        for record in snapshot["items"]:
            source_key = record["source_key"]
            marked_rows = by_source.get(source_key, [])
            expected_information = other_information(record, snapshot)
            if len(marked_rows) > 1:
                raise RuntimeError(f"Duplicate existing source marker: {source_key}")
            if marked_rows:
                entity = marked_rows[0]
                expected_names = (
                    record["official_english_name"],
                    record["official_original_name"],
                    record["official_vietnamese_name"],
                )
                stored_names = (
                    entity["official_english_name"],
                    entity["official_original_name"],
                    entity["official_vietnamese_name"],
                )
                if stored_names != expected_names or entity["other_information"] != expected_information:
                    raise RuntimeError(
                        f"Existing source-marked item differs from snapshot: {source_key}"
                    )
                entity_id = entity["id"]
                verified_entities += 1
            else:
                unmarked_name_matches = by_names.get(
                    (record["official_english_name"], record["official_original_name"]), []
                )
                if unmarked_name_matches:
                    raise RuntimeError(
                        "Refusing to attach curated metadata to an unmarked existing item "
                        f"with the same display names: {source_key}"
                    )
                entity_id = uuid.uuid4().hex
                connection.execute(
                    """
                    INSERT INTO wiki_entities (
                        id, work_id, entity_type, official_english_name,
                        official_original_name, official_vietnamese_name,
                        automatic_vietnamese_translation, english_description,
                        other_information, created_at, updated_at
                    ) VALUES (?, ?, 'item', ?, ?, ?, NULL, NULL, ?, ?, ?)
                    """,
                    (
                        entity_id,
                        args.work_id,
                        record["official_english_name"],
                        record["official_original_name"],
                        record["official_vietnamese_name"],
                        expected_information,
                        timestamp,
                        timestamp,
                    ),
                )
                inserted_entities += 1

            existing_aliases = {
                (row["value"], row["kind"])
                for row in connection.execute(
                    "SELECT value, kind FROM entity_aliases WHERE entity_id = ?",
                    (entity_id,),
                )
            }
            for value, language, kind, label, notes in alias_rows(record):
                if (value, kind) in existing_aliases:
                    continue
                connection.execute(
                    """
                    INSERT INTO entity_aliases (
                        id, entity_id, value, language, kind, label, notes
                    ) VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (uuid.uuid4().hex, entity_id, value, language, kind, label, notes),
                )
                existing_aliases.add((value, kind))
                inserted_aliases += 1

        foreign_key_failures = connection.execute("PRAGMA foreign_key_check").fetchall()
        if foreign_key_failures:
            raise RuntimeError(f"Foreign-key check failed: {foreign_key_failures}")
        connection.commit()
    except Exception:
        connection.rollback()
        raise

    item_count = connection.execute(
        "SELECT COUNT(*) FROM wiki_entities WHERE work_id = ? AND entity_type = 'item'",
        (args.work_id,),
    ).fetchone()[0]
    alias_count = connection.execute(
        """
        SELECT COUNT(*) FROM entity_aliases
        WHERE entity_id IN (
            SELECT id FROM wiki_entities WHERE work_id = ? AND entity_type = 'item'
        )
        """,
        (args.work_id,),
    ).fetchone()[0]
    integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
    foreign_keys = connection.execute("PRAGMA foreign_key_check").fetchall()
    connection.close()
    if integrity != "ok" or foreign_keys:
        raise RuntimeError("Post-commit SQLite verification failed")

    print(
        json.dumps(
            {
                "database": str(database_path),
                "work_id": args.work_id,
                "inserted_entities": inserted_entities,
                "verified_entities": verified_entities,
                "inserted_aliases": inserted_aliases,
                "item_count": item_count,
                "item_alias_count": alias_count,
                "integrity_check": integrity,
                "foreign_key_failures": len(foreign_keys),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
