# Genshin Impact data fetching

This directory contains title-owned, review-first fetch pipelines for the official Genshin Impact website character directory and the community-maintained Book/Quest lore catalog. They do not use account credentials, player data, or undocumented game-client endpoints, and they never open Perwiga's SQLite database.

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

[`data/character-han-viet.json`](data/character-han-viet.json) adds one generated Hán-Việt search alias for every catalog character. The readings are generated from the verified official Traditional Chinese localization with the Thi Viện Hán-Nôm dictionary and carry confidence E. They are not official Vietnamese names, and the Traditional Chinese input is not presented as the original mainland localization. Ambiguous dictionary choices and the explicit fallback for `菈` remain visible in the reviewable snapshot metadata.

The seven bundled region emblems and their exact source URLs/checksums are recorded in [`assets/regions/README.md`](assets/regions/README.md). They are presentation-only community-indexed game UI assets; they are not used as evidence for names or affiliations, and the running app never fetches them from the Internet.

[`data/weapons.json`](data/weapons.json) is a pinned, reviewable snapshot of 246 obtainable weapons from GenshinData-derived data distributed by `genshin-db`. It excludes three explicitly marked, non-obtainable Prized Isshin Blade quest variants. Each record retains the stable game-data ID, English, Simplified Chinese, Traditional Chinese, and Vietnamese names, English description, rarity, weapon type, level-1 Base ATK, substat, first ascension material, release version, and local-thumbnail metadata.

[`data/weapon-han-viet.json`](data/weapon-han-viet.json) adds one generated confidence-E Hán-Việt search alias for every catalog weapon. The generator transcribes the original Simplified Chinese name; it never replaces the official Vietnamese localization. Dictionary ambiguity and the cited `鹮` fallback remain reviewable in the snapshot metadata.

[`data/weapon-thumbnails-hoyowiki.json`](data/weapon-thumbnails-hoyowiki.json) joins the catalog to the official Genshin HoYoWiki weapon archive. The 2026-08-26 snapshot matches 245 of 246 weapons and records each HoYoWiki entry ID and first-party image URL. HoYoWiki does not yet list the 7.0 weapon Exaiphanes Blade, so that one explicit gap retains its existing verified game-asset icon; it is not misattributed to the wiki.

The weapon `ascension_region` facet is derived from the first ascension-material family. It answers “which region supplies this weapon's ascension material?” and does not claim a lore origin. Nod-Krai and Far North material families are grouped under the module's broad Snezhnaya taxonomy. Generic rarity filtering/sorting uses the module's 1-star gray, 2-star green, 3-star blue, 4-star purple, and 5-star gold presentation colors.

[`data/skins.json`](data/skins.json) is a 2026-08-26 snapshot of all 29 non-default entries in the official HoYoWiki Outfit catalog, joined to the pinned multilingual `genshin-db` outfit records. It excludes 120 default outfits, groups the shared Traveler outfit under both Aether and Lumine, retains official English, Simplified Chinese, Traditional Chinese, and Vietnamese names, and bundles each first-party HoYoWiki thumbnail. All current entries apply to Characters; the snapshot records zero confirmed weapon or weapon-type skins. The schema reserves those applicability fields for future source-confirmed entries, and the UI does not fabricate any.

Skin presentation uses the same 4-star purple and 5-star gold rarity borders as other Genshin entities. Filters cover rarity, applicability kind, applicable Character, and the title-owned weapon-type taxonomy. [`data/skin-han-viet.json`](data/skin-han-viet.json) contains a separate generated confidence-E Hán-Việt search alias for each Skin name; it never replaces the official Vietnamese localization.

[`data/artifacts.json`](data/artifacts.json) separates the 63 current Artifact Sets from their 299 actual Artifact Pieces. Each Set keeps its effects, rarity range, stable game-data ID, official HoYoWiki identity, and piece links. Each Piece keeps its parent Set, equipment slot, lore, multilingual in-game names, and local first-party thumbnail. Four legacy one-piece Sets contain only a Circlet; the snapshot preserves that source structure instead of fabricating missing slots. [`data/artifact-han-viet.json`](data/artifact-han-viet.json) adds 362 separate confidence-E Hán-Việt search aliases generated from Simplified Chinese names.

[`data/artifact-domains.json`](data/artifact-domains.json) contains 22 stable Artifact Domain entrances grouped from 92 difficulty stages. It keeps multilingual entrance names, region, unlock rank, stage IDs/names, recommended levels/elements, and links to hosted Artifact Sets. Domain entrances are records; difficulty stages remain nested source metadata so the Wiki does not show four near-duplicate entries for one place. Artifact Sets and Pieces use rarity filtering/sorting and rarity-colored borders, Pieces add an Artifact-slot filter, and Domains add a title-owned region filter including Nod-Krai.

[`data/regions.json`](data/regions.json) contains a world anchor, nine top-level regions, and 207 subregions collapsed from 268 multilingual Geography Archive viewpoint records in pinned `genshin-db`. The hierarchy retains stable region/area IDs, parents Nod-Krai to Snezhnaya, and excludes seven unnamed viewpoints plus one `???` placeholder instead of inventing names. It describes Geography Archive-backed areas, not every possible map label. [`data/region-han-viet.json`](data/region-han-viet.json) adds one generated confidence-E Hán-Việt search alias per entry from the Simplified Chinese name. Region type and parent region are title-owned filters; duplicate display names such as Mondstadt (nation and city) retain separate source identities.

[`data/npcs.json`](data/npcs.json) contains a 2026-08-27 snapshot of 4,967 named NPC/person pages from the community-maintained [Genshin Impact Wiki NPC index](https://genshin-impact.fandom.com/wiki/NPC/List). It preserves page IDs, multilingual page fields when present, infobox regions and locations, page categories, and source-backed quest/event/action relationships. One NPC may have many locations and many related contexts; those relationships are imported into the shared normalized context tables rather than being flattened into a text field on the NPC. The crawler captures explicit Quest/Event/Action templates, selected relationship categories, and explicit linked titles, so it does not claim to infer every appearance from free-form prose.

[`data/npc-han-viet.json`](data/npc-han-viet.json) contains generated confidence-E Hán-Việt search aliases for the subset with usable Chinese names. The local reading source is [hanviet-pinyin-words](https://github.com/ph0ngp/hanviet-pinyin-words); unresolved characters and pages without Chinese names remain explicit in the snapshot metadata. These values are search aids only and never replace official Vietnamese text.

## Books, quests, and lore timeline

[`data/books.json`](data/books.json) and [`data/quests.json`](data/quests.json) are source-checked snapshots from the community-maintained [Books category](https://genshin-impact.fandom.com/wiki/Category:Books) and [Quests category](https://genshin-impact.fandom.com/wiki/Category:Quests). The 2026-09-02 crawl contains 47 book or book-collection records and 2,545 quest, act, chapter, event-quest, and explicitly linked quest-like records. The category's `Book` terminology/index page is retained in the source-page count but excluded from the entity catalog. Four linked records were expanded from quest pages outside the main category; redirect aliases and reused quest titles remain source-keyed rather than being merged by display name.

The quest crawler extracts only bounded, reviewable fields: infobox names and classifications, explicit predecessor/successor links, release-version categories, explicit book/reward links, and Quest Infobox or Appearances-category Character/NPC links. It does not infer a character appearance from arbitrary prose. Every book and quest record retains its page ID, URL, multilingual fields when available, stable source key, and unresolved-link field. The checked snapshot has zero unresolved book-to-quest, quest-to-book, predecessor, or successor links. Unknown names such as the Traveler are kept as provisional lore subjects until a dedicated source identity is available.

[`data/lore.json`](data/lore.json) is the connected Genshin lore candidate batch. It links each quest event to its quest subject, explicitly named Characters/NPCs, and explicitly linked Books; book links from either direction are preserved in the normalized context import. It contains 2,545 quest events, 8,002 subjects, and 44 release-version ordering anchors. Release versions provide publication ordering only; every quest's in-world date remains explicitly unknown unless a future source-backed chronology is reviewed. The core validates the batch's evidence, subject references, event relations, and temporal graph before the module imports it into the lore timeline.

Refresh the three snapshots together after reviewing the live category pages:

```text
python3 games/genshin-impact/scripts/fetch-lore.py \
  --checked-at YYYY-MM-DD
```

The crawler uses the public Fandom MediaWiki API, follows linked redirects, validates every cross-catalog link, writes atomically, refuses an unexpected catalog shrink unless `--allow-shrink` is explicitly reviewed, and never opens SQLite. The local script tests use fixtures only:

```text
python3 -m unittest discover -s games/genshin-impact/scripts/tests -v
```

## Event schedule snapshot

[`data/events.json`](data/events.json) is a reviewed, title-owned snapshot of 109 Genshin Impact events from Version 6.3 (Luna IV) through Version 7.0. It includes version boundaries, story and exploration activities, combat challenges, TCG events, resource events, skins, permanent content, and Character Wish records with their featured Character source keys. Date-only historical windows are stored as all-day events; Version 7.0 notices with published Asia-server times retain RFC3339 timestamps and the `Asia server (UTC+8)` timezone.

Historical rows use the community-maintained [Genshin Impact Wiki Event History](https://genshin-impact.fandom.com/wiki/Event/History/Song_of_the_Welkin_Moon). Current and foreseeable Version 7.0 rows use official [HoYoLAB Version 7.0 notices](https://www.hoyolab.com/article/46186330) and linked official announcements. The snapshot was checked on 2026-08-27. It is a reviewable import boundary, not an always-on crawler: a refresh must recheck the cited source, preserve stable `source_key` values, keep source provenance on every row, and use additive imports. Permanent or not-yet-ended events retain a null `ends_at` rather than receiving a guessed date.

The shared calendar renders one swimlane per module-defined `event_type`. Character Wish and Test Run records resolve their `featured_character_keys` through the existing HoYoverse Character snapshot, so the hover preview can show local Character thumbnails, rarity colors, and previous-banner gaps where a prior limited banner is recorded. The event importer is available through UAT setup and the CLI; it never opens a remote service or mutates SQLite destructively.

To import the reviewed Event snapshot into an existing Genshin work:

```text
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \\
  entity import-genshin-events --work <genshin-work-id>
```

## Refresh Regions

Use the same reviewed, pinned `genshin-db` checkout:

```text
python3 games/genshin-impact/scripts/fetch-regions.py \
  --source-root /path/to/pinned/genshin-db
python3 games/genshin-impact/scripts/fetch-region-han-viet.py
```

The catalog builder joins four locales by stable Geography identity, writes atomically, refuses unexpected shrinkage, and never opens SQLite. The Hán-Việt generator keeps Japanese-kokuji/phonetic fallback evidence in snapshot metadata and never promotes generated names to official Vietnamese.

## Refresh NPCs and NPC Hán-Việt aliases

```text
python3 games/genshin-impact/scripts/fetch-npcs.py \
  --checked-at YYYY-MM-DD
python3 games/genshin-impact/scripts/fetch-npc-han-viet.py
```

The NPC crawler uses the public Fandom MediaWiki API, validates page content in bounded batches, writes atomically, and refuses an unexpected catalog shrink unless `--allow-shrink` is reviewed explicitly. It never opens SQLite. The Hán-Việt generator is intentionally partial when a Chinese name or dictionary reading is unavailable; it records those gaps instead of fabricating a translation. Importing NPCs is additive and idempotent, including their separately stored context and location rows.

## Refresh Artifacts and Domains

Use a reviewed checkout of the pinned `genshin-db` source and a complete official HoYoWiki Artifact archive response:

```text
python3 games/genshin-impact/scripts/fetch-artifacts.py \
  --source-root /path/to/pinned/genshin-db \
  --wiki-response /path/to/complete-hoyowiki-artifact-response.json
python3 games/genshin-impact/scripts/fetch-artifact-domains.py \
  --source-root /path/to/pinned/genshin-db
python3 games/genshin-impact/scripts/fetch-artifact-han-viet.py
python3 games/genshin-impact/scripts/fetch-artifact-thumbnails.py
```

The catalog builder requires exact name and rarity coverage between the multilingual game-data extract and official HoYoWiki. The Domain builder groups only `UI_ABYSSUS_RELIC` stages by stable entrance ID and links recognized rewards to Set source keys. The thumbnail downloader allowlists first-party hosts, validates or normalizes image signatures, and never overwrites valid assets. None of these scripts opens SQLite.

## Refresh Skins

Use the same reviewed pinned `genshin-db` checkout as the weapon pipeline:

```text
python3 games/genshin-impact/scripts/fetch-skins.py \
  --source-root /path/to/pinned/genshin-db \
  --source-commit 8b15995fa220c88a4d0d7ffe1e21b041d0b32588
python3 games/genshin-impact/scripts/fetch-skin-thumbnails.py
python3 games/genshin-impact/scripts/fetch-skin-han-viet.py
```

The catalog builder joins official HoYoWiki identities, rarity, and thumbnail URLs to pinned multilingual game-data extracts by normalized English name. It validates complete 29-entry coverage and refuses unexpected shrinkage. The downloader accepts only allowlisted first-party HoYoVerse hosts, validates PNG/GIF signatures, and never overwrites a valid asset. All scripts write reviewable files only and never open SQLite.

## Refresh weapons and thumbnails

Use a reviewed, pinned checkout of `genshin-db`:

```text
python3 games/genshin-impact/scripts/fetch-weapons.py \
  --source-root /path/to/pinned/genshin-db
python3 games/genshin-impact/scripts/fetch-weapon-thumbnails.py
python3 games/genshin-impact/scripts/fetch-hoyowiki-weapon-thumbnails.py
python3 games/genshin-impact/scripts/fetch-weapon-han-viet.py
```

The snapshot builder joins all locales by stable filename and game-data ID, validates every filter value, writes atomically, and refuses unexpected shrinkage. The first thumbnail command maintains the complete fallback set from indexed game assets. The HoYoWiki command snapshots the official archive, normalizes only decorative title quotes and apostrophe typography for matching, downloads the 245 confirmed wiki icons into a separate primary asset set, and leaves explicit gaps visible. Both downloaders validate hosts, paths, sizes, and PNG signatures and never overwrite an existing valid asset.

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

## Refresh generated Hán-Việt aliases

After reviewing an updated character catalog, run:

```text
python3 games/genshin-impact/scripts/fetch-character-han-viet.py
```

The generator submits only the catalog's public Traditional Chinese names to the allowlisted Thi Viện transcription endpoint, validates that every response round-trips to its source name, and atomically replaces only the reviewable alias snapshot. It never opens SQLite. Import remains additive and idempotent through the Rust module.

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
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-regions --work <genshin-work-id>
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-weapons --work <genshin-work-id>
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-skins --work <genshin-work-id>
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-artifacts --work <genshin-work-id>
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-artifact-domains --work <genshin-work-id>
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-npcs --work <genshin-work-id>
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-books --work <genshin-work-id>
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-quests --work <genshin-work-id>
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity import-genshin-lore --work <genshin-work-id>
```

The import is transactional, additive, and idempotent. `entity import-genshin-lore` is the connected convenience command: it ensures the Character, NPC, Book, and Quest source entities exist before materializing the reviewed lore batch. Quest links also populate normalized source-backed context rows for Characters, NPCs, and Books. Stable source keys prevent duplicate imports, existing records are never replaced, official Traditional Chinese names remain provenance-labeled aliases, and generated Hán-Việt values are separate confidence-E search aliases. The confirmed global source does not provide Simplified Chinese, so the English catalog anchor is repeated in the schema-required original-name field rather than mislabeling Traditional Chinese.

## Sources

- Official global character directory: <https://genshin.hoyoverse.com/en/character/mondstadt>
- Official site bundle containing the current content-service contract: <https://genshin.hoyoverse.com/_nuxt/446ceb2.js>
- HoYoverse public website content service: <https://sg-public-api-static.hoyoverse.com/content_v2_user>
- Thi Viện Hán-Nôm transcription tool: <https://hvdic.thivien.net/transcript.php#trans>
- Rarity presentation dataset: <https://github.com/theBowja/genshin-db> (`v5.2.13`, checked 2026-08-24)
- Official HoYoWiki weapon directory and its rarity/type/substat taxonomy: <https://wiki.hoyolab.com/pc/genshin/aggregate/weapon>
- Official HoYoWiki Outfit catalog: <https://wiki.hoyolab.com/pc/genshin/aggregate/34>
- Official HoYoWiki Artifact archive: <https://wiki.hoyolab.com/pc/genshin/aggregate/5>
- Official HoYoLAB introduction to Artifact and Domain archive updates: <https://www.hoyolab.com/article/7159996>
- Official HoYoWiki introduction and Weapon Archive description: <https://www.hoyolab.com/article/4626755>
- Weapon data/localizations and indexed first-party image URLs: <https://github.com/theBowja/genshin-db> (commit `8b15995fa220c88a4d0d7ffe1e21b041d0b32588`, checked 2026-08-25)
- Game-asset thumbnail fallback: <https://enka.network/> (used only when an indexed first-party URL is unavailable)
- Community-maintained Book catalog: <https://genshin-impact.fandom.com/wiki/Category:Books>
- Community-maintained Quest catalog: <https://genshin-impact.fandom.com/wiki/Category:Quests>
- Quest article and menu taxonomy: <https://genshin-impact.fandom.com/wiki/Quest>
- `鹮` Hán-Việt fallback evidence: <https://js.vnu.edu.vn/FS/article/download/4175/3893>
- Region emblem catalog: <https://genshin-impact.fandom.com/wiki/Category:Region_Emblems> (presentation assets only, checked 2026-08-24)
- Geography Archive names and localizations: <https://github.com/theBowja/genshin-db> (commit `8b15995fa220c88a4d0d7ffe1e21b041d0b32588`, checked 2026-08-26)
- Official Aloy collaboration status: <https://support.hoyoverse.com/hc/en-us/articles/50333967030681-How-can-I-get-the-character-Aloy>

The content service is treated as a website source, not as a promised public developer API. Its contract identifiers are therefore recorded in the snapshot and validated on every refresh rather than hidden in an opaque importer.
