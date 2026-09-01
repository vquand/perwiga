# Heroes of Might and Magic III data

This module contains a review-first catalog for **Heroes of Might and Magic III Complete**. The crawler writes JSON only and never opens Perwiga's SQLite database. The Rust importer validates that bundled review boundary before performing an additive, source-keyed import.

## Snapshot boundary

[`data/heroes-iii.json`](data/heroes-iii.json) contains the original game's three Complete-edition releases: Restoration of Erathia, Armageddon's Blade, and Shadow of Death. It excludes Horn of the Abyss, Day of Reckoning, disabled VCMI creature slots, and VCMI-only internal spells.

The snapshot checked on 2026-09-01 contains:

| Catalog | Records |
| --- | ---: |
| Towns | 9 |
| Heroes | 156 |
| Creatures | 141 |
| Artifacts and war machines | 141 |
| Adventure and combat spells | 70 |
| **Total** | **517** |

Each record has a stable VCMI key and index, the exact VCMI source file, short factual fields, and the Heroes 3 Wiki page/revision used for review. The snapshot deliberately omits copied biographies and manual prose. `NPC` and `Region` remain normal module entity types for manual records, but the selected sources do not supply a canonical reviewed catalog for either type.

No confirmed official Vietnamese catalog was found, so official Vietnamese names remain absent. English is retained as the original name for this English-language source catalog.

## Refresh

Check out the pinned VCMI source revision, then run:

```text
python3 games/heroes-of-might-and-magic/scripts/fetch-heroes-iii.py \
  --source-root /path/to/vcmi \
  --checked-at YYYY-MM-DD
```

The checkout must be at commit `93dd4eeac1a9f41ff8c7f090c1d323633a32d69e`. The crawler reads the Heroes 3 Wiki MediaWiki API, stores revision provenance per record, rejects duplicate stable IDs or unexpected type counts, and refuses an unreviewed shrink of an existing snapshot. Run its offline parser and safety tests with:

```text
python3 -m unittest discover \
  -s games/heroes-of-might-and-magic/scripts/tests \
  -v
```

After reviewing the JSON diff, create a game work owned by `heroes-of-might-and-magic` and import it with:

```text
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-heroes-iii --work <work-id>
```

The import is additive and idempotent. It does not replace manually maintained entities and does not clear existing values.

## Sources

- [VCMI](https://github.com/vcmi/vcmi) — pinned open-source structural configuration for stable keys, indices, and edition scope.
- [Heroes 3 Wiki](https://heroes.thelazy.net/) — community-maintained English catalog facts accessed through its read-only MediaWiki API, with page revision provenance retained in every record.
- [Heroes of Might and Magic III manual on Steam](https://cdn.cloudflare.steamstatic.com/steam/apps/297000/manuals/Heroes3_Manual.pdf) — original manual used to confirm the game's world-reference categories and terminology boundary, not copied into the snapshot.
