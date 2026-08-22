# Game module instructions

## Scope

These rules apply to every module under `games/`. The root [AGENTS.md](../AGENTS.md) remains mandatory, especially its SQLite backup and no-data-loss rules. Each module directory must also have its own `AGENTS.md`; read it before making module changes.

## Common versus module-specific

Keep behavior in the shared core only when its meaning, ownership, persistence, and failure behavior are genuinely the same for every game. Notes, checklists, attachments, linked folders, stable base entities, entity references, normalized events, transport, and the universal calendar are common.

Keep behavior in the owning game module when it depends on a title’s entity taxonomy, structured fields, relations, source URLs, credentials, rate limits, payload formats, parsing, refresh rules, deduplication, or time-zone semantics.

- Modules depend on shared contracts; shared core code must not import a concrete module.
- Only the composition root may enumerate and register concrete modules.
- Do not put `if game == ...`, title matching, or source-specific parsing in shared core/calendar code.
- Prefer capability-driven shared UI. Keep unavoidable custom views inside the owning module.
- Use stable, namespaced module, entity-type, source, and table identifiers. Renaming one requires a migration and compatibility plan.
- Put queryable game-specific fields in normalized namespaced tables with foreign keys to common records; do not create a global table full of nullable title-specific columns.
- Validate all RSS/API/page responses inside the owning module before normalization. Keep fixture-based parser tests independent from live Internet access.
- Module migrations run after common migrations in deterministic order and must obey the root no-data-loss rules.

## Local `AGENTS.md` requirements

Each game’s file should stay focused and record:

- Stable module ID and exact game/series scope.
- Entity types, relationships, and title-specific fields once known.
- Confirmed external sources, provenance, authentication, refresh, deduplication, and time-zone rules.
- Module-specific migrations, fixtures, custom UI, commands, and known gotchas.
- Any private franchise helper it may use without coupling sibling modules directly.

Do not invent these details. Keep unknown items marked as unconfirmed until researched or supplied by the user.
