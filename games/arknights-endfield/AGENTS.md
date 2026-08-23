# Arknights: Endfield module

- Module ID: `arknights-endfield`.
- Scope: Arknights: Endfield only.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Use the project-local [`$curate-endfield-names`](../../.agents/skills/curate-endfield-names/SKILL.md) skill for Endfield multilingual naming research, normalization, reverse lookup, and database imports or upserts.
- Keep Endfield-specific entity types, relations, fields, event sources, parsers, fixtures, migrations, and custom UI in this module.
- Do not assume an API, RSS feed, entity taxonomy, server schedule, or time zone until it is documented from a confirmed source.
- The first implementation exposes `operator`, `npc`, `region`, `weapon`, `enemy`, `mission`, `event`, and `item` entity keys. The common multilingual fields and stable references remain in the shared core; no Endfield-specific fields or namespaced tables are needed yet.
- The module exposes a dark graphite theme with an acid-green accent through semantic theme tokens. Shared UI and a future desktop shell consume these tokens without Endfield conditionals.
- Confirmed source boundaries checked on 2026-08-23: the official [operator directory](https://endfield.gryphline.com/en-us/operator) and official [Homecoming version notes](https://endfield.gryphline.com/en-us/news/5200). These sources confirm operator, weapon, enemy, area, mission, event/activity, item, and NPC terminology. They are provenance references, not an automatic importer.
- The curated [`data/operators.json`](data/operators.json) snapshot contains the 33 entries verified across the official English, Simplified Chinese, and Vietnamese operator directories on 2026-08-23. Treat it as module-owned import source data; preserve its source keys and the two distinct Endministrator entries during additive refreshes.
- Operator presentation is module-owned. The curated source key resolves rarity metadata and an embedded local WebP thumbnail generated from the snapshot's official portrait URL; unknown or malformed keys must fall back safely without filesystem access. Keep 4-star blue, 5-star gold, and 6-star crimson presentation metadata distinct.
- The module currently has no automatic feed or scheduled-event adapter. Users can enter manual calendar events; a future adapter must document source identity, refresh behavior, deduplication, and time-zone semantics before registration.
- Current state: common persistence and manual workflows are implemented, and a manually verified operator roster snapshot is available. Title-specific tables and automatic integrations remain unconfirmed and intentionally unimplemented.
