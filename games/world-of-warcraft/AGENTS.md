# World of Warcraft module

- Module ID: `world-of-warcraft`.
- Scope: World of Warcraft only; exclude Warcraft strategy-game data.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Keep World of Warcraft-specific entity types, relations, fields, sources, parsers, fixtures, migrations, and custom UI in this module.
- A private Warcraft franchise helper may be shared, but this module must not import the `warcraft` module directly.
- Do not assume an API, RSS feed, supported game version, region schedule, or entity taxonomy until it is explicitly documented.
- Current state: scaffold only; versions, sources, and schemas are unconfirmed.
