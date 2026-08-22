# Generic game module

- Module ID: `generic`.
- Scope: manual fallback for any game without dedicated support.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Keep this module free of title-specific entity types, APIs, RSS feeds, parsers, and conditionals.
- It must support all common manual features without an external provider.
- Moving a game from `generic` to a dedicated module must preserve all common entities, links, folders, notes, checklists, attachments, and manual events.
- Current state: scaffold only; no module-specific schema or integration is defined.
