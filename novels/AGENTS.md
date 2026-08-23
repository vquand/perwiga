# Novel module instructions

## Scope

These rules apply to every module under `novels/`. The root [AGENTS.md](../AGENTS.md) remains mandatory, especially its SQLite backup and no-data-loss rules. Follow [ADR-002](../docs/decisions/ADR-002-games-and-novels-as-library-works.md). Each module directory must also have its own `AGENTS.md`; read it before making module changes.

## Common versus module-specific

Keep the wiki, notes, checklists, attachments, linked folders, stable entity references, standards-compliant RSS/Atom transport, normalized feed items, and provider status in the shared core.

Keep behavior in the owning novel module when it depends on a title’s entity taxonomy, chapter/volume model, structured fields, source URLs, authentication, rate limits, custom HTML/API parsing, refresh rules, deduplication, or publication-time semantics.

- Modules depend on shared contracts; shared core code must not import a concrete novel module.
- Only the composition root may enumerate and register concrete modules.
- Do not put title matching or source-specific chapter parsing in shared core code.
- Prefer capability-driven shared UI. Keep unavoidable custom views inside the owning module.
- Use stable, namespaced entity-type, source, chapter-extension, and table identifiers. Renaming one requires a migration and compatibility plan.
- Validate all feed/API/page responses before normalization. Keep parser tests fixture-based and independent from live Internet access.
- Prefer a stable RSS/Atom GUID for feed identity, then a canonical URL, then a documented title-specific fallback.
- Treat a chapter publication timestamp as feed activity, not a future schedule. Do not create a calendar event unless a confirmed source explicitly supplies schedule semantics or the user creates one manually.
- Never delete stored chapters or feed items merely because an upstream feed is shortened, reordered, unavailable, or corrected.
- Module migrations run after common migrations in deterministic order and must obey the root no-data-loss rules.

## Local `AGENTS.md` requirements

Each novel module’s file should stay focused and record:

- Stable module ID and exact novel or series scope.
- Entity types, chapter/volume relationships, and title-specific fields once known.
- Confirmed external sources, provenance, authentication, refresh, deduplication, and timestamp rules.
- How source records map to feed items, chapter metadata, and any genuine scheduled events.
- Module-specific migrations, fixtures, custom UI, commands, and known gotchas.

Do not invent these details. Keep unknown items marked as unconfirmed until researched or supplied by the user.
