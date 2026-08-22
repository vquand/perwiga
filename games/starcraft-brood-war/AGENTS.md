# StarCraft and Brood War module

- Module ID: `starcraft-brood-war`.
- Scope: StarCraft and StarCraft: Brood War; exclude StarCraft II data.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Keep Brood War-specific entity types, relations, fields, sources, parsers, fixtures, migrations, and custom UI in this module.
- A private StarCraft franchise helper may be shared, but this module must not import the `starcraft-ii` module directly.
- Do not assume an event source or entity taxonomy until it is explicitly documented.
- Current state: scaffold only; sources and title-specific schemas are unconfirmed.
