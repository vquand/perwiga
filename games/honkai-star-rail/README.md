# Honkai: Star Rail data

This directory contains a title-owned, review-first catalog pipeline for Honkai: Star Rail. It never uses account credentials, player data, or Perwiga's SQLite database.

## Snapshot boundary

[`data/characters.json`](data/characters.json), [`data/light-cones.json`](data/light-cones.json), [`data/relic-sets.json`](data/relic-sets.json), [`data/relics.json`](data/relics.json), [`data/paths.json`](data/paths.json), and [`data/elements.json`](data/elements.json) are generated from the `index_new` files in StarRailRes. Each catalog is joined by stable game-data ID across English (`en`), Vietnamese (`vi`), Simplified Chinese (`cn`), and Traditional Chinese (`cht`).

[`data/npcs.json`](data/npcs.json) is a separate, source-bound catalog from the official HoYoWiki NPC index. It contains 436 Simplified Chinese entries joined to 433 English entries by HoYoWiki `entry_page_id`; the three entries without an English join remain in the snapshot for review and are not fabricated into runtime entities.

The snapshot checked on 2026-09-01 contains:

| Catalog | Records |
| --- | ---: |
| Characters | 97 |
| Light Cones | 169 |
| Relic Sets | 60 |
| Relic Pieces | 742 |
| Paths | 9 |
| Elements | 7 |
| HoYoWiki NPC source records | 436 |
| HoYoWiki NPC records imported with English joins | 433 |

The separate Hán-Việt snapshots provide generated search aliases without changing official names: 97 Character records (85 aliases; placeholders and Latin-only names remain unresolved), 169 Light Cones, 60 Relic Sets, 742 Relic Pieces, 9 Paths, 7 Elements, and 436 NPC source records (432 generated readings, 3 unresolved readings, and 1 source-literal name). These aliases are explicitly marked as generated and unofficial; they are not Vietnamese localization fields.

Character records preserve rarity, Path, Element, Max Energy when supplied, localized names, and StarRailRes portrait paths. Character icon URLs are pinned to the reviewed StarRailRes commit and crawled into `assets/characters/` for local and public serving; the source URL remains in each database row's provenance metadata. The upstream catalog includes multiple playable variants, both protagonist genders/path variants, crossover records, and source placeholders such as `{NICKNAME}`; these are preserved as source records and are not silently collapsed by display name. The runtime uses a contextual `Trailblazer (Male/Female)` label for those placeholder records while retaining `{NICKNAME}` in the official source fields. The basic Character endpoint does not supply biographies, so the snapshot keeps descriptions absent rather than inferring them from another source.

Light Cone records preserve localized names and descriptions, rarity, Path, and portrait paths. Relic Sets and Relic Pieces remain separate: each Piece retains its source parent Set key, slot, rarity, maximum level, and localized names. No missing pieces are fabricated. Paths and Elements retain their localized names, descriptions, source keys, and Element presentation colors.

The runtime module exposes `Character`, `NPC`, `Light Cone`, `Relic Set`, `Relic`, `Path`, and `Element`, plus the required shared `Region` type. It provides Path, Element, and Relic-slot facets and additive importers for the StarRailRes catalogs, Hán-Việt alias snapshots, and the HoYoWiki NPC snapshot. Region and calendar records are not inferred because no reviewed source snapshot is included for those categories.

## Refresh

For a reproducible refresh from a local StarRailRes checkout:

```text
python3 games/honkai-star-rail/scripts/fetch-data.py \
  --source-dir /path/to/StarRailRes \
  --source-commit <reviewed-commit> \
  --checked-at YYYY-MM-DD

python3 games/honkai-star-rail/scripts/fetch-character-thumbnails.py
```

The script also supports the public static mirror when `--source-dir` is omitted. Review changes before importing them. It refuses an unexpected shrink in any existing catalog unless `--allow-shrink` is explicitly added.

The dedicated alias and NPC crawlers are independently reviewable and never open SQLite:

```text
python3 games/honkai-star-rail/scripts/fetch-npcs.py --checked-at YYYY-MM-DD
python3 games/honkai-star-rail/scripts/fetch-character-han-viet.py
python3 games/honkai-star-rail/scripts/fetch-light-cone-han-viet.py
python3 games/honkai-star-rail/scripts/crawl-relic-han-viet.py
```

## Sources

- [StarRailRes](https://github.com/Mar-7th/StarRailRes) — multilingual index and asset paths.
- [Dimbreath/StarRailData](https://github.com/Dimbreath/StarRailData) — upstream game-data source identified by StarRailRes.
- [StarRailStaticAPI](https://github.com/VizualAbstract/StarRailStaticAPI) — public static mirror used as the crawler's default transport source.
- [Honkai: Star Rail HoYoWiki announcement](https://www.hoyolab.com/article/17984857) — official HoYoLAB announcement that the game's HoYoWiki archive exists. This module does not treat the archive as a general public API.
- [HoYoWiki NPC index](https://wiki.hoyolab.com/pc/hsr/aggregate/npc?lang=zh-cn) — official HoYoLAB-hosted NPC index used for the source-bound NPC snapshot.
- [Thi Viện Hán-Nôm dictionary](https://hvdic.thivien.net/transcript.php#trans) — character-reading evidence for generated Hán-Việt search aliases.

The generated snapshots are community-distributed game-data extracts. They are provenance-labeled for local reference and are not presented as official HoYoverse API responses.
