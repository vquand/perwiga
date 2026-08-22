# StarCraft II module

- Module ID: `starcraft-ii`.
- Scope: the StarCraft II series; exclude StarCraft and Brood War data.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Keep StarCraft II-specific entity types, relations, fields, sources, parsers, fixtures, migrations, and custom UI in this module.
- A private StarCraft franchise helper may be shared, but this module must not import the `starcraft-brood-war` module directly.
- Do not assume an event source, supported expansion, or entity taxonomy until it is explicitly documented.
- Current state: scaffold only; sources and title-specific schemas are unconfirmed.
