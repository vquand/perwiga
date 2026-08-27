# Game module instructions

## Scope

These rules apply to every module under `games/`. The root [AGENTS.md](../AGENTS.md) remains mandatory, especially its SQLite backup and no-data-loss rules. Follow [ADR-002](../docs/decisions/ADR-002-games-and-novels-as-library-works.md). Each module directory must also have its own `AGENTS.md`; read it before making module changes.

## Common versus module-specific

Keep behavior in the shared core only when its meaning, ownership, persistence, and failure behavior are genuinely the same for every library work. Notes, checklists, attachments, linked folders, stable base entities, entity references, normalized feeds/events, transport, updates, and the universal calendar are common.

Keep behavior in the owning game module when it depends on a title’s entity taxonomy, structured fields, relations, source URLs, credentials, rate limits, payload formats, parsing, refresh rules, deduplication, or time-zone semantics.

## Baseline entity types

Every game module, including `generic`, must make NPC and Region available as explicit multilingual wiki entity types.

- Give both types the common multilingual names, description, aliases/other information, timestamps, ownership, search, and stable-reference behavior.
- Keep NPC distinguishable from a playable-character type. Preserve the entity’s stable ID and references if its classification changes.
- Keep Region available for named geographic, administrative, world, and map areas. Add title-specific hierarchy or relations through normalized module data without encoding a mutable hierarchy into the display name.
- Modules may extend these types with namespaced fields or subtypes, but must not replace their common fields or hide them in opaque data.

## Entity context records

The shared core provides normalized source-backed context records for any entity, not only NPCs. A record has a module-owned relation kind, a related title, source provider/identity, optional notes, and zero or more location rows. Use the common kinds `quest`, `event`, and `action` when they have the same meaning across modules. If a title has a genuinely different structured context such as a chapter, scene, raid, or codex entry, keep that vocabulary in the owning module’s namespaced extension and map only the common portion into the shared record.

Do not encode a repeated list of quests, events, locations, or encounters in `other_information`. Preserve each source relationship separately so a character can belong to many regions and many contexts, and so refreshes remain additive and source-keyed. A relationship row is evidence of a source-backed association, not proof that the entity is physically present in every related location at every time.

- Modules depend on shared contracts; shared core code must not import a concrete module.
- Only the composition root may enumerate and register concrete modules.
- Do not put `if game == ...`, title matching, or source-specific parsing in shared core/calendar code.
- Prefer capability-driven shared UI. Keep unavoidable custom views inside the owning module.
- Keep title palettes in module-owned semantic theme metadata. Shared UI may consume those tokens but must not branch on a module ID to choose colors.
- Use stable, namespaced module, entity-type, source, and table identifiers. Renaming one requires a migration and compatibility plan.
- Put queryable game-specific fields in normalized namespaced tables with foreign keys to common records; do not create a global table full of nullable title-specific columns.
- Validate all RSS/API/page responses inside the owning module before normalization. Keep fixture-based parser tests independent from live Internet access.
- Map a source publication to a feed item by default. Create a calendar event only when the source supplies genuine schedule semantics; publication timestamps alone are not schedules.
- Module migrations run after common migrations in deterministic order and must obey the root no-data-loss rules.

## Local `AGENTS.md` requirements

Each game’s file should stay focused and record:

- Stable module ID and exact game/series scope.
- Entity types, including how the required NPC and Region types are extended, plus relationships and title-specific fields once known.
- Confirmed external sources, provenance, authentication, refresh, deduplication, feed/event mapping, and time-zone rules.
- Module-specific migrations, fixtures, custom UI, commands, and known gotchas.
- Any private franchise helper it may use without coupling sibling modules directly.

Do not invent these details. Keep unknown items marked as unconfirmed until researched or supplied by the user.
