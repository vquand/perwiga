# Warcraft module

- Module ID: `warcraft`.
- Scope: the Warcraft strategy series; exclude World of Warcraft data.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Keep Warcraft strategy-specific entity types, relations, fields, sources, parsers, fixtures, migrations, and custom UI in this module.
- A private Warcraft franchise helper may be shared, but this module must not import the `world-of-warcraft` module directly.
- If individual strategy titles diverge materially, split them into dedicated modules through an ADR instead of accumulating title checks here.
- Current state: scaffold only; title coverage, sources, and schemas are unconfirmed.
