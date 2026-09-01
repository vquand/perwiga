# Heroes of Might and Magic module

- Module ID: `heroes-of-might-and-magic`.
- Confirmed runtime scope: Heroes of Might and Magic III Complete, comprising Restoration of Erathia, Armageddon's Blade, and Shadow of Death. Horn of the Abyss, Day of Reckoning, and other fan expansions are excluded from the bundled snapshot.
- Follow [games/AGENTS.md](../AGENTS.md) and the root data-safety rules.
- Entity types are Hero, NPC, Region, Town, Creature, Artifact, and Spell. NPC and Region are available for manual records but have no bundled crawl because the confirmed sources do not define a reviewed canonical catalog for them.
- The generated snapshot uses VCMI commit `93dd4eeac1a9f41ff8c7f090c1d323633a32d69e` for stable structural keys and Complete-edition scope, and the read-only Heroes 3 Wiki MediaWiki API for English labels and short factual catalog fields. Preserve each selected wiki page ID, revision ID, revision timestamp, and URL.
- Treat Heroes 3 Wiki as a community source. Do not relabel its text as official, and do not add biographies or long copied prose to the snapshot.
- No confirmed official Vietnamese catalog source is used. Keep official Vietnamese names null rather than generating or inferring them.
- The crawler must never open Perwiga's SQLite database. Refresh into reviewable JSON, enforce the exact reviewed counts, and import only through the title-owned additive source-keyed importer.
- Keep series-specific entity types, relations, fields, sources, parsers, fixtures, migrations, and custom UI in this module.
- If title schemas or integrations diverge materially, create separate modules and an ADR instead of accumulating title checks here.
- No feed or calendar semantics are confirmed for this snapshot. Do not infer scheduled events from wiki revision or publication timestamps.
