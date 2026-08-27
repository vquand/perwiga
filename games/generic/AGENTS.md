# Generic game module

- Module ID: `generic` within the `game` work family.
- Scope: manual fallback for any game without dedicated support.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Keep this module free of title-specific entity types, APIs, feed endpoints, custom parsers, and conditionals.
- It must support all common manual features. It may use the shared standards-compliant RSS/Atom capability for feeds configured by the user, without adding title-specific interpretation.
- It must expose NPC and Region as explicit multilingual wiki entity types with the common CRUD, search, and stable-reference behavior.
- Source-backed NPC quests, events, and actions may use the shared entity-context tables, including multiple locations. Keep unverified relationships out of the fallback module; a dedicated game module owns title-specific relation vocabularies and sources.
- It uses the shared neutral theme fallback. Dedicated title modules may override semantic theme tokens without adding title checks to shared UI.
- Moving a game from `generic` to a dedicated module must preserve all common entities, links, folders, notes, checklists, attachments, feed sources/items, and manual events.
- Current state: scaffold only; no module-specific schema or integration is defined.
