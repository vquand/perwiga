# Generic novel module

- Module ID: `generic` within the `novel` work family.
- Scope: fallback for any novel without dedicated support.
- Follow [novels/AGENTS.md](../AGENTS.md), [ADR-002](../../docs/decisions/ADR-002-games-and-novels-as-library-works.md), and the root data-safety rules.
- Support all common manual features and user-configured standards-compliant RSS/Atom feeds.
- Keep this module free of title-specific entity types, source URLs, authentication, custom parsers, chapter heuristics, APIs, and title conditionals.
- Preserve the source feed title, URL, GUID, publication timestamp, and provenance. Do not guess chapter numbers or convert publication timestamps into calendar schedules.
- Moving a novel from `generic` to a dedicated module must preserve all common entities, links, folders, notes, checklists, attachments, feed sources/items, and manual events.
- Current state: scaffold only; no module-specific schema or integration is defined.
