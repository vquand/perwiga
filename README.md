# Perwiga

Perwiga is a personal, local-first game wiki. It stores information about the games a person plays in a SQLite database and organizes that information into categories that can grow over time.

The first planned category is a multilingual wiki for named game content such as places, characters, items, factions, and other entities. The app will also connect game-specific folders, notes, checklists, and event data to each game.

## Status

This repository currently contains the product requirements and agent guidance only. Rust is the preferred implementation language, but the desktop UI framework, database schema, and development commands have not been selected or implemented yet.

## Product goals

- Add a new game to the local library.
- Open a game and manage information belonging to that game.
- Organize game information by category rather than putting every field directly on the game record.
- Start with a multilingual wiki that supports adding, editing, and viewing a list of named entries.
- Link one or more local folders to a specific game and browse their images.
- Create simple rich-text notes with local images, remote image URLs, thumbnails, and YouTube embeds.
- Track game-specific checklists with click-to-complete items.
- Fetch supported game events when possible and show all events in one universal calendar.
- Provide the same core experience on macOS, Windows, and Linux as far as platform capabilities allow.
- Keep data available locally through SQLite so the app can work without a remote service.

## Planned game support

Each supported game is represented by a registered game module with a stable module identifier. The initial support list is:

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

Games without a dedicated module use a built-in `generic` manual module. They still support the common wiki, folders, notes, checklists, and manually entered events. Assigning a dedicated module later must preserve that existing common data.

## Multilang Wiki

Each multilingual wiki entry represents one named piece of content in a game. An entry may contain:

| Field | Meaning |
| --- | --- |
| Official English name | The official English name as it appears in the game. |
| Official original name | The official name in the original in-game language. It may contain mixed-language text, such as French and English together. |
| Official Vietnamese translation | The official Vietnamese translation, when one exists. |
| Automatic Vietnamese translation | A machine-generated Vietnamese translation, kept separately when an official translation is unavailable or useful for comparison. |
| English description | A description of the entry written in English. |
| Others | Additional names and notes, including Hán Việt forms of Chinese or Japanese names and other useful reference information. |

The category must support the following basic actions:

1. Add a multilingual wiki entry to a game.
2. Edit an existing entry.
3. View the list of entries for a game.
4. View the complete information for an entry.

Official and automatic Vietnamese translations are separate values. An automatic translation must never replace or overwrite an official translation.

## Game folders and images

A user can link a local folder to a specific game to browse image files related to that game. A folder link is an association to an existing folder; the app must not silently move, rename, copy, or delete the user’s files. The UI should handle missing or inaccessible folders clearly.

Images can be viewed from linked folders and attached to notes from either of these sources:

- A local file, including a file in a linked game folder.
- An Internet URL.

Attachments should show a thumbnail preview when the source is available. The original source reference should remain available so the user can open or inspect it. Remote previews may be unavailable when there is no network access or when the URL stops working.

## Notes

Notes belong to a game and use simple rich text. A note can contain text, local image attachments, remote image URLs, and YouTube URLs. A recognized YouTube URL should render an embedded video in the note while retaining the original URL.

The rich-text format is an implementation decision that must preserve the note’s content and be safe to render. Raw executable markup must not be allowed to run merely because it was pasted into a note.

Notes also support Logseq-inspired entity references. Typing `#[` opens an autocomplete menu for characters and other multilingual wiki entries belonging to the current game. Continuing to type filters the results by any known name, and choosing a result inserts a linked reference rendered in a human-readable form such as `#[[Entity Name]]`. Clicking the rendered reference opens that entity’s wiki entry.

The interaction follows Logseq’s use of `[[Page name]]` references and `#[[multi-word tag]]` syntax, but Perwiga links must resolve by the selected entry’s stable identifier rather than its display name alone. Renaming an entity must not break existing references. See Logseq’s official [tutorial](https://github.com/logseq/docs/blob/master/pages/tutorial.md) and [Markdown syntax](https://github.com/logseq/logseq/blob/master/docs/logseq-markdown-syntax.md) for the interaction being referenced.

## Checklist

Each game can have a checklist of things to do. A checklist item has a label and a completion state. Clicking an item toggles completion, and the state must be persisted so it remains correct after closing and reopening the app. Checklists may be used for quests, collection goals, research tasks, or personal reminders.

Checklist item text supports the same `#[` entity autocomplete and linked references as notes. Clicking the checkbox changes completion; clicking an entity reference opens the referenced wiki entry. These actions must remain distinct even when the link appears next to the checkbox.

## Events and universal calendar

The app should try to fetch event data for supported games and display those events in a universal calendar. “Supported” is intentionally game-specific: each game may have its own provider or hardcoded adapter for locating and interpreting event data.

Game-specific fetching should be isolated behind a common event model containing, at minimum, the owning game, title, start time, end time or all-day status, and source information. The universal calendar should render events from every supported game together, with a way to identify or filter the owning game. A provider that is unavailable, unsupported, or temporarily failing must not prevent the calendar from displaying locally stored events from other games.

Automatic fetching is opportunistic rather than guaranteed. The app should show when event data was last fetched and should preserve the last successful result when a later fetch fails.

## Common core and game modules

Perwiga uses a shared core plus statically linked Rust game modules. Common workflows should not contain conditionals based on a game’s title or module identifier. Instead, the core discovers a module through a registry and uses a small capability-based contract.

| Shared by every game | Owned by an individual game module |
| --- | --- |
| Game identity and module registration | Game metadata and stable module identifier |
| Multilingual base entity identity and names | Supported entity types and game-specific fields |
| Notes, checklists, attachments, folders, and entity links | Optional game-specific validation and display metadata |
| Universal event and calendar model | RSS/API endpoints, authentication needs, and response parsers |
| HTTP transport, caching, retries, scheduling, and source provenance | Mapping source records into normalized events and entities |
| SQLite connection, common migrations, and integrity rules | Namespaced module tables and migrations when extensions are needed |
| Consistent provider status and error presentation | Source-specific error classification, deduplication, and time-zone rules |

Every module declares its capabilities. A module can support manual entities only, events from RSS, events from an API, custom entity fields, or any additive combination. The built-in `generic` module provides all manual common features for games without dedicated support. The generic UI renders common capabilities; custom game UI should be introduced only when the shared schema and components cannot accurately represent the game.

The base entity record remains common so notes and checklist references can always point to a stable entity ID. A game module may add normalized, namespaced tables for fields unique to that game. It must not add many game-specific nullable columns to a global table or hide queryable structured data in an opaque blob.

See [ADR-001: Modular game support](docs/decisions/ADR-001-modular-game-support.md) for the module contract, dependency rules, and trade-offs.

## Initial domain model

The logical model should keep the game library separate from category-specific content:

```text
Game 1 ──── * MultilangWikiEntry
     ├──── * LinkedFolder
     ├──── * Note ──── * NoteAttachment
     ├──── * Checklist ──── * ChecklistItem
     └──── * GameEvent ────> UniversalCalendar
```

At minimum, a game needs a stable identifier, a display name, and created/updated timestamps. A multilingual wiki entry needs its owning game identifier, the fields listed above, and created/updated timestamps. Linked folders, notes, checklists, and events must also retain an explicit owning game identifier. Additional categories should be able to attach to a game without changing the core `Game` record.

An attachment should identify its source type (local file or remote URL) and source reference. A checklist item needs a persisted completion state. Entity links in notes and checklist items need a stable reference to the target multilingual wiki entry in addition to their visible label. A game event needs enough normalized date/time data for the universal calendar, plus source/provider metadata for refreshes and troubleshooting.

The data model should preserve Unicode exactly enough to retain original scripts, accents, diacritics, and mixed-language names. Missing translations are valid data and should be represented as empty/optional fields rather than fabricated text.

## Storage and privacy

- SQLite is the primary database.
- The primary SQLite database is intentionally stored in Git because this is a private, personal repository. Do not add the primary database file to `.gitignore`.
- The repository containing the database must remain private because its Git history may retain deleted or changed personal data.
- Git treats SQLite as a binary file and cannot safely merge concurrent database edits. Close the app and ensure pending SQLite journal/WAL data has been checkpointed before committing the database.
- Transient SQLite sidecar files such as `-wal`, `-shm`, and `-journal` are runtime artifacts, not the canonical database. Their ignore/cleanup policy must be documented when the database path and journal mode are selected.
- Tests and development fixtures must use temporary databases and must never modify the tracked personal database.
- Schema changes should use repeatable migrations rather than ad-hoc database edits.
- Foreign-key enforcement should be enabled so category entries cannot outlive their game accidentally.
- Local folder and file references should be stored as references; the app must not perform destructive file operations as a side effect of linking or previewing.
- Remote image and event fetching should be treated as optional I/O with clear failure states and timeouts.
- The app should not require an account or remote database for the core workflow.

## Planned user flow

```text
Open app → View games → Add game → Open game → Choose category
         → Add/edit wiki entry, note, checklist, or folder link
         → View images, toggle checklist items, or view the universal calendar
```

## Technology requirements

- Use Rust for the application core, including domain logic, SQLite access, filesystem coordination, and game-specific event adapters.
- Build game support as statically linked Rust modules behind a shared contract and central registry.
- Target macOS, Windows, and Linux from one shared codebase.
- Keep the desktop UI framework replaceable until it has been evaluated for rich text, local file access, image thumbnails, safe YouTube embeds, SQLite integration, and native packaging on all target platforms.
- A thin non-Rust presentation layer is acceptable if the selected cross-platform framework requires one, but business rules and persistence behavior should remain in Rust.
- Use platform APIs or well-supported cross-platform libraries for application data directories, folder selection, URL opening, and file handling.
- Avoid assumptions about path separators, case sensitivity, drive letters, Unicode paths, or the availability of a Unix shell.
- Keep platform-specific behavior behind small interfaces so it can be tested without forking the main application logic.
- Verify builds and core workflows on all three target operating systems before calling a release cross-platform.

## Development

There are no implementation or test commands yet. When the Rust workspace and desktop framework are selected, add the exact setup, development, database migration, formatting, linting, test, packaging, and build commands here instead of documenting placeholders.

## Contributing

Keep the app local-first and preserve the distinction between official, automatic, and user-provided text. When adding a category or integration, document its purpose, ownership relationship to `Game`, CRUD behavior, source/provenance rules, and failure behavior before implementing it.

Update this README when user-facing requirements or supported workflows change. Use `AGENTS.md` for repository-wide instructions that apply to coding agents and contributors.
