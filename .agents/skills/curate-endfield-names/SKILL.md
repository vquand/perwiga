---
name: curate-endfield-names
description: "Research, verify, normalize, and safely persist multilingual Arknights: Endfield names and aliases in Perwiga. Use for official English, Simplified Chinese, official or generated Vietnamese, Hán-Việt forms, Chinese community or fanfiction aliases, reverse lookup of converter text, glossary curation, and Endfield entity imports or upserts into SQLite or another project database."
---

# Curate Endfield Names

Curate multilingual Arknights: Endfield entity data without presenting fan terminology, machine conversion, or inference as official text. Support research-only answers, import-ready records, and safe writes to Perwiga's actual persistence layer.

## Load Project Context

Before researching or changing data:

1. Read the repository root `AGENTS.md`.
2. Read `games/AGENTS.md` and `games/arknights-endfield/AGENTS.md`.
3. Read [references/naming-policy.md](references/naming-policy.md) in full.
4. For persistence work, inspect the current migrations, schema, models, database configuration, and existing Endfield records. Do not invent table names, columns, IDs, or a database path.

Treat the naming policy as a research and normalization procedure, not as a source of game facts. Reverify names against current external sources because localization and release data can change.

## Select the Operation

- **Research:** Return a verified name record with sources and confidence. Do not mutate the database.
- **Prepare:** Produce a normalized, import-ready record mapped conceptually to Perwiga fields. Do not mutate the database.
- **Persist:** Research, normalize, then add or non-destructively update records only when the user's request includes saving, filling, importing, or updating project data.
- **Reverse lookup:** Resolve a likely Chinese source name from broken Hán-Việt, machine-converted, or copied text. Do not persist unresolved candidates.

## Research and Normalize

1. Identify the entity and entity type. Keep character, place, faction, weapon, ability, item, enemy, ecology, lore concept, archive, and quest terms distinct.
2. Search current sources using the hierarchy and query patterns in the naming policy. Prefer first-party sources and in-game text.
3. Verify official Simplified Chinese and official English independently. Record direct source URLs and the date checked.
4. Search Chinese community and web-novel usage separately. Include an alias only when it is attested; otherwise record no common alias.
5. Verify official Vietnamese independently. If none is found, keep the official field empty. Put generated Vietnamese in the generated-translation field and Hán-Việt or converter forms in aliases or other information with explicit labels.
6. Preserve current official names as primary. Keep beta, legacy, affectionate, mocking, meme, and fanfiction forms as labeled aliases.
7. Preserve Unicode, scripts, accents, punctuation, and mixed-language forms exactly. Never silently normalize away meaningful spelling differences.
8. Assign confidence per assertion, not just per entity. Distinguish sourced facts from inference.

Use this conceptual record even if the implemented schema uses different names:

- Game and entity identity
- Entity type
- Official English name
- Official original-language name, normally Simplified Chinese for this module
- Official Vietnamese name
- Generated Vietnamese translation
- English description
- Pinyin and other romanization
- Aliases and other information, each with language, kind, label, and notes
- Current, beta, or legacy status
- Source URL, source class, date checked, and confidence for each assertion
- User-selected project-preferred display name, if one exists, without replacing official forms

## Persist Safely

For every database write:

1. Resolve the actual database and schema from repository configuration. If they do not exist yet, stop at an import-ready record and explain what is missing.
2. Enable SQLite foreign keys for the connection and use parameterized statements inside a transaction.
3. Read the owning game and matching entity or aliases before writing. Avoid duplicates using the schema's stable identity rules.
4. Prefer additive inserts and merges. Never use `INSERT OR REPLACE`, `REPLACE`, `DELETE`, `DROP`, destructive cascades, or statements that clear non-empty cells.
5. Never overwrite official data with community terminology, generated text, converter output, or lower-confidence evidence. Preserve superseded non-empty values as labeled historical aliases when the schema supports it.
6. If a requested change would delete, clear, or destructively replace existing data, follow the root `AGENTS.md` backup and explicit-authorization procedure before proceeding.
7. Do not create or alter schemas unless the user requested implementation work. Do not commit or push database changes unless the user requested Git operations.
8. After writing, query the affected records and run appropriate SQLite integrity and foreign-key checks. Report inserted, updated, unchanged, skipped, and unresolved records.

## Handle Reverse Lookup

When given suspicious converter text:

1. Preserve the input exactly and split it into meaningful components.
2. Generate two to five plausible Chinese candidates, clearly marked as candidates.
3. Search each candidate with Endfield-specific terms.
4. Accept a match only after verifying official Chinese and cross-checking official English or Vietnamese when available.
5. Return the original input, likely source form, official names, confidence, and a short explanation.
6. Do not save a candidate while ambiguity remains. Keep E-grade reconstructions as research notes rather than identified entities.

## Report Results

For a single entity, return the normalized record, sources, confidence, unresolved questions, and database action. For batches, use a compact table plus a write summary. Clearly label every `[Hán-Việt]`, `[Convert]`, `[Machine translation]`, `[meme]`, `[fanfic]`, `[nickname]`, `[beta name]`, `[affectionate]`, and `[mocking]` value.
