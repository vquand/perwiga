# Genshin Impact data fetching

This directory contains a title-owned, review-first fetch pipeline for the official Genshin Impact website character directory. It does not use account credentials, player data, or an undocumented game-client endpoint, and it never opens Perwiga's SQLite database.

## Snapshot boundary

[`data/characters.json`](data/characters.json) is a `perwiga.genshin-impact.characters.v1` snapshot of the records returned by the official regional character channels for Mondstadt, Liyue, Inazuma, Sumeru, Fontaine, Natlan, and Snezhnaya.

The 2026-08-24 snapshot contains 118 records:

| Region | Records |
| --- | ---: |
| Mondstadt | 26 |
| Liyue | 22 |
| Inazuma | 16 |
| Sumeru | 14 |
| Fontaine | 14 |
| Natlan | 12 |
| Snezhnaya | 14 |

Each record retains the official content ID, region channel, English name and description, official Traditional Chinese name, official Vietnamese name, availability timestamps, and first-party thumbnail source. The global website does not supply Simplified Chinese; the Traditional Chinese localization is deliberately not labeled as the original mainland name.

[`data/character-presentation.json`](data/character-presentation.json) is a separately reviewable, fully covered presentation snapshot keyed by those same stable HoYoverse content IDs. It records the current 4-star/5-star classification from `genshin-db` v5.2.13 without treating that community dataset as naming evidence. The UI renders 4-star Characters with a purple border and 5-star Characters with a gold border, keeps the star label in the thumbnail, and displays the source-confirmed region as a local white-on-transparent emblem in a compact trapezoid beside the Character type. The region name remains in the accessibility tree and becomes the visible fallback if its emblem cannot load. Aloy retains her 5-star classification but receives a red border because HoYoverse identifies her as a limited-time collaboration Character.

The seven bundled region emblems and their exact source URLs/checksums are recorded in [`assets/regions/README.md`](assets/regions/README.md). They are presentation-only community-indexed game UI assets; they are not used as evidence for names or affiliations, and the running app never fetches them from the Internet.

## Refresh the catalog

From the repository root:

```text
python3 games/genshin-impact/scripts/fetch-characters.py
```

The refresh is atomic and refuses to replace an existing snapshot with fewer records. If an official source intentionally removes records, inspect the generated change first and rerun with `--allow-shrink` only when the contraction is confirmed.

The content service can be slow. Each request has bounded timeouts, retries, and response size, and the script fetches at most four locale/region pages concurrently by default.

## Fetch local thumbnails

```text
python3 games/genshin-impact/scripts/fetch-character-thumbnails.py
```

The downloader reads only the validated snapshot, accepts trusted HoYoverse HTTPS hosts, checks image magic, and writes new files without replacing existing assets. The current 118 PNG files occupy about 2.8 MB.

## Verify the fetch logic

```text
python3 -m unittest discover -s games/genshin-impact/scripts/tests -v
```

These tests use local fixtures only; they do not require the network or a database.

## Import into Perwiga

The UAT setup imports the bundled snapshot automatically. To import it explicitly into an existing Genshin library work, use its stable work ID:

```text
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-characters --work <genshin-work-id>
```

The import is transactional, additive, and idempotent. Stable HoYoverse source keys prevent duplicate imports, existing records are never replaced, and official Traditional Chinese names are stored as provenance-labeled aliases. The confirmed global source does not provide Simplified Chinese, so the English catalog anchor is repeated in the schema-required original-name field rather than mislabeling Traditional Chinese.

## Sources

- Official global character directory: <https://genshin.hoyoverse.com/en/character/mondstadt>
- Official site bundle containing the current content-service contract: <https://genshin.hoyoverse.com/_nuxt/446ceb2.js>
- HoYoverse public website content service: <https://sg-public-api-static.hoyoverse.com/content_v2_user>
- Rarity presentation dataset: <https://github.com/theBowja/genshin-db> (`v5.2.13`, checked 2026-08-24)
- Region emblem catalog: <https://genshin-impact.fandom.com/wiki/Category:Region_Emblems> (presentation assets only, checked 2026-08-24)
- Official Aloy collaboration status: <https://support.hoyoverse.com/hc/en-us/articles/50333967030681-How-can-I-get-the-character-Aloy>

The content service is treated as a website source, not as a promised public developer API. Its contract identifiers are therefore recorded in the snapshot and validated on every refresh rather than hidden in an opaque importer.
