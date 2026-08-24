# Perwiga

Perwiga is a personal, local-first wiki for games and novels. It stores information about the works a person plays or reads in a SQLite database and organizes that information into categories that can grow over time.

The first planned category is a multilingual wiki for named content such as playable characters, NPCs, regions, places, items, factions, and other entities. The app will also connect folders, notes, checklists, feeds, and supported schedule data to each library work.

## Status

The first executable vertical slice is implemented in Rust. It provides a local SQLite core, a family-aware module registry, a CLI composition root, and the common wiki, notes, folders, checklists, feeds, and calendar persistence workflows. Arknights: Endfield and Genshin Impact are dedicated runtime game modules. Genshin has a title-owned, review-first official character-directory fetch pipeline and an additive, idempotent SQLite importer for its bundled snapshot.

An Endfield-focused localhost UAT interface now exercises the multilingual wiki workflow in a real browser while keeping all domain logic and SQLite access in Rust. It is an evaluation surface, not the final distribution model. The planned product remains a lightweight cross-platform desktop application. Tauri 2 is the leading shell candidate because it can reuse this interface with the operating system's WebView, but the desktop choice will be recorded in a separate ADR after packaging prerequisites and platform behavior are verified. See [ADR-003](docs/decisions/ADR-003-rust-core-and-cli-first-slice.md).

## Quick start

Start the Endfield UAT interface with an explicit temporary database path:

```text
cargo run -p perwiga-web -- \
  --database /tmp/perwiga-endfield-uat.sqlite \
  --bind 127.0.0.1:5178
```

Then open `http://127.0.0.1:5178`. The server rejects non-loopback bind addresses. Do not point UAT at the tracked personal database; use a fresh path outside the repository.

The interface sets up Endfield and Genshin as switchable library games, imports Genshin's bundled Character snapshot, supports adding more games, and consumes module-specific themes and entity schemas. It provides type and rarity filtering, rarity sorting, multilingual name and alias search, record creation, editing, detail inspection, aliases, module-owned Endfield Operator, Weapon, and Item thumbnails with rarity presentation, and module-owned Genshin Character thumbnails with region filtering. It also exposes a shared horizontally scrollable event timeline with title-defined event-type swimlanes and past, ongoing, and upcoming filters. Title modules can declare additional structured facets; Endfield currently provides weapon-type filtering for Operators and Weapons, Operator role, element, race, and verified subrace filters, and Item classification and evidence-backed region filters. Item classifications remain multi-valued, so a material can also be craftable, gatherable, or another applicable type. Titles without a dedicated module use the generic game fallback. Official and automatic Vietnamese translations remain separate throughout the workflow.

The CLI remains available for all implemented common workflows. List the registered modules:

```text
cargo run -p perwiga -- --database /path/to/perwiga.sqlite modules
```

Create an Endfield work and add a multilingual entry:

```text
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  work add --kind game --module arknights-endfield --name "Arknights: Endfield"
cargo run -p perwiga -- --database /path/to/perwiga.sqlite \
  entity add --work <work-id> --type operator \
  --english "<verified official English name>" \
  --original "<verified official original-language name>"
```

The database path is explicit so a personal tracked database is never confused with a temporary test database. Tests use in-memory or temporary SQLite databases.

## Verified development commands

| Command | Purpose |
| --- | --- |
| `cargo fmt --all -- --check` | Check Rust formatting |
| `cargo build --workspace` | Build the core, CLI, UAT server, and registered modules |
| `cargo test --workspace` | Run unit and workflow self-tests |
| `cargo run -p perwiga-web -- --help` | Show the localhost Endfield UAT server options |

The CLI also exposes `work`, `entity`, `note`, `folder`, `checklist`, `feed`, and `event` subcommands. Run `cargo run -p perwiga -- --help` for the complete interface.

## Implemented architecture

`crates/perwiga-core` owns the stable contracts, additive SQLite migration, boundary validation, normalized RSS/Atom parsing, and common workflows. Concrete modules under `games/` and `novels/` implement the `LibraryModule` contract. A module may also supply semantic presentation tokens and entity facet definitions, allowing shared UI surfaces to adopt title-specific colors and filter vocabularies without title checks in shared code. The CLI and localhost UAT server are composition roots that register concrete modules, preserving the dependency direction from [ADR-002](docs/decisions/ADR-002-games-and-novels-as-library-works.md).

The current Endfield module exposes source-confirmed baseline types for operators, NPCs, regions, weapons, enemies, missions, events, and items. Module-owned data includes the [official 33-Operator roster snapshot](games/arknights-endfield/data/operators.json), a [provenance-labeled initial catalog of 134 named NPCs/persons](games/arknights-endfield/data/npcs.json), a [scoped hierarchical snapshot of 43 regions and locations](games/arknights-endfield/data/regions.json), a [current snapshot of 77 weapons with rarity, type, multilingual names, and generated Hán-Việt aliases](games/arknights-endfield/data/weapons.json), a [current Item Files snapshot of 731 non-weapon records](games/arknights-endfield/data/items.json) with client icon IDs, additive multi-class taxonomy, recipe evidence, and optional client-attested Valley IV/Wuling region metadata for 45 gatherable/drop records, and a [source-checked Asia-server event schedule](games/arknights-endfield/data/events.json) spanning launch through announcements checked on 2026-08-24. The NPC, Region, and Item catalogs are explicitly scoped rather than presented as complete official rosters. The shared feed capability supports HTTP/HTTPS RSS and Atom refreshes with bounded retries, response limits, and persisted status, but Endfield does not declare that capability until a confirmed title source exists. The Endfield event integration is a bundled snapshot importer with explicit UTC+8 semantics, not an unconfirmed live event API.

The [Genshin Impact data pipeline](games/genshin-impact/README.md) fetches the official website's seven regional character catalogs in English, Traditional Chinese, and Vietnamese, joins them by HoYoverse content ID, and caches first-party thumbnails locally without touching SQLite. Its 2026-08-24 snapshot contains 118 character-directory records with no locale gaps. A separate transactional importer persists that reviewed snapshot using stable source identities without replacing existing records. It does not mislabel Traditional Chinese as the original Simplified Chinese localization, and it does not assume that the website content service is a general game-data, account, event, or schedule API.

## Product goals

- Add a new game or novel to the local library.
- Open a library work and manage information belonging to it.
- Organize information by category rather than putting every field directly on the work record.
- Start with a multilingual wiki that supports adding, editing, and viewing a list of named entries.
- Include NPC and Region as explicit baseline multilingual entity types for every game.
- Link one or more local folders to a specific work and browse their images.
- Create simple rich-text notes with local images, remote image URLs, thumbnails, and YouTube embeds.
- Track work-specific checklists with click-to-complete items.
- Read supported RSS or Atom feeds so a novel can show newly published chapters and other title updates.
- Fetch supported game events when possible and show all events in one universal calendar.
- Provide the same core experience on macOS, Windows, and Linux as far as platform capabilities allow.
- Keep data available locally through SQLite so the app can work without a remote service.

## Planned title support

Each library record has a work kind (`game` or `novel`) and is represented by a registered module in that family. A module is identified by the pair of work kind and stable module identifier, so game and novel modules can evolve independently.

### Games

The initial dedicated game support list is:

| Module identifier | Game or series |
| --- | --- |
| `genshin-impact` | Genshin Impact |
| `honkai-star-rail` | Honkai: Star Rail |
| `arknights-endfield` | Arknights: Endfield |
| `heroes-of-might-and-magic` | Heroes of Might and Magic series |
| `starcraft-brood-war` | StarCraft and StarCraft: Brood War |
| `starcraft-ii` | StarCraft II series |
| `warcraft` | Warcraft strategy series |
| `world-of-warcraft` | World of Warcraft |

Series modules may share private franchise code, but they remain separate modules when their entities, source data, release cycles, or integrations differ. This list describes planned support; it does not imply that an RSS feed or API has already been selected for any game.

Games without a dedicated module use the built-in game `generic` module. They still support the common wiki, folders, notes, checklists, feeds configured by the user, and manually entered events. Assigning a dedicated module later must preserve that existing common data.

### Novels

Novels initially use the built-in novel `generic` module. It supports the common wiki, folders, notes, checklists, and user-configured standard RSS or Atom feeds. Add a dedicated novel module when a title requires custom entity types, chapter metadata, authentication, HTML parsing, a non-standard API, or other title-specific behavior.

Novel feeds normally report that a chapter has already been published; they do not imply a future publication schedule. A novel should participate in the universal calendar only when the user enters an event manually or a confirmed source provides genuine future schedule data.

Novel support is acceptable when a user can add a novel, use every common manual category, configure a standard RSS/Atom source, refresh it repeatedly without duplicate items, see newly discovered chapters as unread updates, and retain both prior items and read state after restart or a failed refresh. Published chapter updates must not appear as upcoming calendar events.

## Multilang Wiki

Each multilingual wiki entry represents one named piece of content in a game or novel. An entry may contain:

| Field | Meaning |
| --- | --- |
| Official English name | The official English name as it appears in the game or an official publication/localization. |
| Official original name | The official name in the original game or publication language. It may contain mixed-language text, such as French and English together. |
| Official Vietnamese translation | The official Vietnamese translation, when one exists. |
| Automatic Vietnamese translation | A machine-generated Vietnamese translation, kept separately when an official translation is unavailable or useful for comparison. |
| English description | A description of the entry written in English. |
| Others | Additional names and notes, including Hán Việt forms of Chinese or Japanese names and other useful reference information. |

The category must support the following basic actions:

1. Add a multilingual wiki entry to a work.
2. Edit an existing entry.
3. View the list of entries for a work.
4. View the complete information for an entry.

Official and automatic Vietnamese translations are separate values. An automatic translation must never replace or overwrite an official translation.

### Baseline game entity types

Every game module, including the generic fallback, must expose `NPC` and `Region` as explicit wiki entity types rather than requiring the user to hide them under a generic character or place label.

| Entity type | Meaning |
| --- | --- |
| NPC | A named non-playable character or person. Keep it distinguishable from a playable-character type while using the same multilingual naming fields. |
| Region | A named geographic, administrative, world, or map area. It may later have game-specific hierarchy or relationships without changing the shared multilingual naming fields. |

NPC and Region entries support the same add, edit, list, detail, name search, and `#[` reference workflows as every other multilingual wiki entry. A game module may add structured subtypes, relationships, validation, and display metadata, but it must preserve the entry’s stable identity and shared names. If an NPC later becomes playable or a region is reorganized, existing notes and checklist references must not break.

## Linked folders and images

A user can link a local folder to a specific work to browse related image files. A folder link is an association to an existing folder; the app must not silently move, rename, copy, or delete the user’s files. The UI should handle missing or inaccessible folders clearly.

Images can be viewed from linked folders and attached to notes from either of these sources:

- A local file, including a file in a linked work folder.
- An Internet URL.

Attachments should show a thumbnail preview when the source is available. The original source reference should remain available so the user can open or inspect it. Remote previews may be unavailable when there is no network access or when the URL stops working.

## Notes

Notes belong to a work and use simple rich text. A note can contain text, local image attachments, remote image URLs, and YouTube URLs. A recognized YouTube URL should render an embedded video in the note while retaining the original URL.

The rich-text format is an implementation decision that must preserve the note’s content and be safe to render. Raw executable markup must not be allowed to run merely because it was pasted into a note.

Notes also support Logseq-inspired entity references. Typing `#[` opens an autocomplete menu for characters and other multilingual wiki entries belonging to the current work. Continuing to type filters the results by any known name, and choosing a result inserts a linked reference rendered in a human-readable form such as `#[[Entity Name]]`. Clicking the rendered reference opens that entity’s wiki entry.

The interaction follows Logseq’s use of `[[Page name]]` references and `#[[multi-word tag]]` syntax, but Perwiga links must resolve by the selected entry’s stable identifier rather than its display name alone. Renaming an entity must not break existing references. See Logseq’s official [tutorial](https://github.com/logseq/docs/blob/master/pages/tutorial.md) and [Markdown syntax](https://github.com/logseq/logseq/blob/master/docs/logseq-markdown-syntax.md) for the interaction being referenced.

## Checklist

Each work can have a checklist of things to do. A checklist item has a label and a completion state. Clicking an item toggles completion, and the state must be persisted so it remains correct after closing and reopening the app. Checklists may be used for quests, collection goals, reading goals, research tasks, or personal reminders.

Checklist item text supports the same `#[` entity autocomplete and linked references as notes. Clicking the checkbox changes completion; clicking an entity reference opens the referenced wiki entry. These actions must remain distinct even when the link appears next to the checkbox.

## Feeds and chapter updates

A work can have one or more update sources when its module declares feed support. Standards-compliant RSS and Atom parsing, transport, caching, retries, and refresh status belong to the shared core. A title module owns source URLs, authentication requirements, custom parsing, chapter interpretation, and source-specific deduplication rules.

All providers map updates into a common feed item containing, at minimum, the owning work, source identity, stable external identity when available, title, URL, publication time when supplied, discovery time, persisted unread/read state, and provenance. A feed item may additionally identify a chapter number or title, but the original source values must remain available. Providers should prefer a stable RSS/Atom GUID, then a canonical URL, and otherwise use a documented source-specific identity so repeated refreshes do not create duplicates.

For novels, a newly observed chapter is shown as an unread published update. The user can mark an update as read without changing the source record. Its publication timestamp records when the chapter appeared; it must not be converted into an upcoming calendar event. Feed refresh is opportunistic: show the last successful refresh, retain existing items and read state when a later refresh fails, and never delete prior items merely because they disappear from a shortened feed response.

## Events and universal calendar

The app should try to fetch event data for supported games and display those events in a universal calendar. “Supported” is intentionally game-specific: each game may have its own provider or hardcoded adapter for locating and interpreting event data.

Title-specific fetching should be isolated behind a common event model containing, at minimum, the owning work, title, start time, end time or all-day status, and source information. The universal calendar should render events from every work with event capability together, with a way to identify or filter the owner. A provider that is unavailable, unsupported, or temporarily failing must not prevent the calendar from displaying locally stored events from other works.

Automatic fetching is opportunistic rather than guaranteed. The app should show when event data was last fetched and should preserve the last successful result when a later fetch fails.

Feeds and schedules are separate capabilities. A published RSS item is not a calendar event. A novel module may expose scheduled events only when a confirmed source supplies actual future publication or community-event times.

The current timeline groups normalized events into one swimlane per title-defined event type. The shared renderer does not hard-code Endfield categories; it consumes the type, dates, source, patch metadata, timezone, and optional featured-entity timing supplied by a module. Open-ended events remain visibly open-ended instead of receiving a guessed closing date. Endfield 6★ Operator details show the live number of whole days since their latest recorded ended Headhunting appearance. A Headhunting preview shows each returning featured Operator's whole-day gap from the previous applicable banner end to the hovered banner start; first recorded appearances do not receive a fabricated gap.

Endfield recurring event names encode their patch occurrence. A single occurrence within patch 1.4 is rendered as `Event 1.4`; two separate occurrences fully inside that patch are `Event 1.4.1` and `Event 1.4.2`; an occurrence spanning two patches is `Event 1.4–1.5`. Stable provider/source identities make repeated snapshot imports additive and idempotent without deleting or replacing existing calendar rows.

## Common core and title modules

Perwiga uses a shared core plus statically linked Rust title modules grouped under `games/` and `novels/`. Common workflows should not contain conditionals based on a title, work kind, or module identifier. Instead, the core discovers a module through a family-aware registry and uses a small capability-based contract.

| Shared by every work | Owned by an individual title module |
| --- | --- |
| Work identity, work kind, and module registration | Title metadata and stable family-local module identifier |
| Multilingual base entity identity and names | Supported entity types and title-specific fields |
| Notes, checklists, attachments, folders, and entity links | Optional title-specific validation and display metadata |
| Normalized feed/activity and universal event/calendar models | RSS/API endpoints, authentication needs, and response parsers |
| HTTP transport, caching, retries, scheduling, and source provenance | Mapping source records into normalized feeds, events, and entities |
| SQLite connection, common migrations, and integrity rules | Namespaced module tables and migrations when extensions are needed |
| Consistent provider status and error presentation | Source-specific error classification, deduplication, and time-zone rules |

Every module declares its capabilities. A module can support manual entities, update feeds, scheduled events, custom entity fields, or any additive combination. Each work family has a built-in `generic` module for titles without dedicated support. The generic UI renders common capabilities; custom title UI should be introduced only when the shared schema and components cannot accurately represent the title.

The base entity record remains common so notes and checklist references can always point to a stable entity ID. A title module may add normalized, namespaced tables for fields unique to that title. It must not add many title-specific nullable columns to a global table or hide queryable structured data in an opaque blob.

See [ADR-002: Games and novels as modular library works](docs/decisions/ADR-002-games-and-novels-as-library-works.md) for the work model, module contract, feed/calendar boundary, dependency rules, and trade-offs. [ADR-001](docs/decisions/ADR-001-modular-game-support.md) retains the earlier game-only rationale.

## Initial domain model

The logical model should keep the library work separate from category-specific content:

```text
LibraryWork (game | novel) 1 ──── * MultilangWikiEntry
             ├──── * LinkedFolder
             ├──── * Note ──── * NoteAttachment
             ├──── * Checklist ──── * ChecklistItem
             ├──── * FeedSource ──── * FeedItem ────> Updates
             └──── * CalendarEvent ────> UniversalCalendar
```

At minimum, a work needs a stable identifier, work kind, family-local module identifier, display name, and created/updated timestamps. A multilingual wiki entry needs its owning work identifier, the fields listed above, and created/updated timestamps. Linked folders, notes, checklists, feed sources/items, and events must also retain an explicit owning work identifier. Additional categories should be able to attach to a work without changing the core `LibraryWork` record.

An attachment should identify its source type (local file or remote URL) and source reference. A checklist item needs a persisted completion state. Entity links in notes and checklist items need a stable reference to the target multilingual wiki entry in addition to their visible label. A feed item needs source identity, deduplication identity, publication/discovery timestamps, persisted unread/read state, and provenance. A calendar event needs enough normalized date/time data for the universal calendar, plus source/provider metadata for refreshes and troubleshooting.

The data model should preserve Unicode exactly enough to retain original scripts, accents, diacritics, and mixed-language names. Missing translations are valid data and should be represented as empty/optional fields rather than fabricated text.

## Storage and privacy

- SQLite is the primary database.
- The canonical personal database is [`perwiga.sqlite`](perwiga.sqlite) at the repository root. CLI and UAT `--database` paths may point elsewhere, but tests and UI experiments must continue to use temporary databases.
- The primary SQLite database is intentionally stored in Git because this is a private, personal repository. Do not add the primary database file to `.gitignore`.
- The repository containing the database must remain private because its Git history may retain deleted or changed personal data.
- Git treats SQLite as a binary file and cannot safely merge concurrent database edits. Close the app and ensure pending SQLite journal/WAL data has been checkpointed before committing the database.
- Transient SQLite sidecar files such as `-wal`, `-shm`, and `-journal` are runtime artifacts, not the canonical database. Their ignore/cleanup policy must be documented when the database path and journal mode are selected.
- Tests and development fixtures must use temporary databases and must never modify the tracked personal database.
- Schema changes should use repeatable migrations rather than ad-hoc database edits.
- Foreign-key enforcement should be enabled so category entries cannot outlive their owning work accidentally.
- Local folder and file references should be stored as references; the app must not perform destructive file operations as a side effect of linking or previewing.
- Remote image, feed, and event fetching should be treated as optional I/O with clear failure states and timeouts.
- The app should not require an account or remote database for the core workflow.

## Planned user flow

```text
Open app → View library → Add game or novel → Open work → Choose category
         → Add/edit wiki entry, note, checklist, folder link, or feed
         → View images, updates, checklist state, or the universal calendar
```

## Technology requirements

- Use Rust for the application core, including domain logic, SQLite access, filesystem coordination, and title-specific feed/event adapters.
- Build game and novel support as statically linked Rust modules behind a shared contract and family-aware central registry.
- Target macOS, Windows, and Linux from one shared codebase.
- Keep the desktop UI framework replaceable until it has been evaluated for rich text, local file access, image thumbnails, safe YouTube embeds, SQLite integration, and native packaging on all target platforms.
- A thin non-Rust presentation layer is acceptable if the selected cross-platform framework requires one, but business rules and persistence behavior should remain in Rust.
- Use platform APIs or well-supported cross-platform libraries for application data directories, folder selection, URL opening, and file handling.
- Avoid assumptions about path separators, case sensitivity, drive letters, Unicode paths, or the availability of a Unix shell.
- Keep platform-specific behavior behind small interfaces so it can be tested without forking the main application logic.
- Verify builds and core workflows on all three target operating systems before calling a release cross-platform.

## Development

Use the verified commands in the Quick start table above. Tests use in-memory or temporary SQLite databases. No desktop packaging command is documented yet because the lightweight desktop shell has not been selected and verified on all three target platforms.

## Contributing

Keep the app local-first and preserve the distinction between official, automatic, and user-provided text. When adding a category or integration, document its purpose, ownership relationship to `LibraryWork`, CRUD behavior, source/provenance rules, and failure behavior before implementing it.

Update this README when user-facing requirements or supported workflows change. Use `AGENTS.md` for repository-wide instructions that apply to coding agents and contributors.
