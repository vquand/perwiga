# Genshin Impact module

- Module ID: `genshin-impact`.
- Scope: Genshin Impact only.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Keep Genshin-specific entity types, relations, fields, event sources, parsers, fixtures, migrations, and custom UI in this module.
- Do not assume an API, RSS feed, entity taxonomy, region schedule, or time zone until it is documented from a confirmed source.
- Shared HoYoverse helpers are allowed only for behavior proven common; do not depend directly on the Honkai: Star Rail module.
- Current state: scaffold only; sources and title-specific schemas are unconfirmed.
