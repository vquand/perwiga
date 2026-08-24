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

The content service is treated as a website source, not as a promised public developer API. Its contract identifiers are therefore recorded in the snapshot and validated on every refresh rather than hidden in an opaque importer.
