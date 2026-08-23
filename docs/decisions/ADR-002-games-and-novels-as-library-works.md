# ADR-002: Model games and novels as modular library works

## Status

Accepted

## Date

2026-08-22

## Context

Perwiga's multilingual wiki, notes, checklists, entity references, attachments, and linked folders are useful for both games and novels. Keeping those workflows game-only would duplicate the same local-first behavior when novel support is added.

Games and novels do not have identical integrations. Games may expose future events through APIs, pages, or feeds. Web novels more commonly expose RSS or Atom entries after a chapter is published and may have no reliable future schedule. Their entity taxonomies and title-specific chapter metadata can also differ.

ADR-001 established a shared core with statically linked game modules. That dependency direction remains useful, but the persisted owner and module contract need to represent more than games. Published feed activity must also remain semantically separate from scheduled calendar events.

## Decision

Model the item a user adds to the library as a `LibraryWork`. Each work has a stable ID, display name, work kind (`game` or `novel`), family-local module ID, and timestamps. Wiki entries, folders, notes, checklists, feeds, events, and future categories reference the owning work rather than a game-only record.

Use one shared capability-based module contract with concrete modules grouped by work family:

```text
games/<module-id>/
novels/<module-id>/
```

Module identity is the pair `(work_kind, module_id)`. This preserves existing game IDs while allowing each family to have a `generic` fallback without a global name collision. A module can only be assigned to a work of the matching kind.

### Dependency direction

```text
Desktop UI ──> Application core ──> Shared library-module contract
                      │                          ▲
                      ├──> Shared storage        │
                      ├──> Shared integrations   │
                      │                          │
Composition root ─────┴────────────> Game and novel modules
```

Only the composition root enumerates concrete modules. Shared code may branch on declared capabilities and work kind semantics, but not on a title or module ID.

### Shared capabilities

The contract may expose these additive capabilities:

| Capability | Meaning |
| --- | --- |
| Manual content | Common entities, notes, checklists, attachments, and linked folders. |
| Entity schema | Namespaced title-specific entity types, relations, fields, validation, and display metadata. |
| Update feed | Sources that normalize RSS, Atom, API, or page records into published feed items. |
| Scheduled events | Sources that normalize genuine scheduled events into calendar records. |
| Custom view | A rare title-specific view that cannot be represented by common components. |

All works receive the manual common capabilities. Feed and event capabilities are independent and optional.

Modules may also provide additive semantic theme metadata for shared presentation surfaces. The shared UI consumes background, surface, border, text, and accent tokens without branching on module IDs. Modules without an override receive a neutral fallback. Theme metadata changes presentation only; it must not alter work ownership, persistence behavior, entity schemas, or source interpretation.

### Feed model

The shared core owns standards-compliant RSS/Atom transport and parsing, caching, retries, refresh status, and the normalized `FeedItem` contract. A normalized feed item contains at least:

- Owning work and source identity
- Stable external identity when available
- Title and canonical or source URL
- Publication time when supplied by the source
- Discovery time recorded by Perwiga
- Persisted unread/read state owned by the user
- Source provenance and refresh metadata
- Optional normalized item kind, such as `chapter-release`, `news`, or `announcement`

Providers prefer a stable feed GUID, then a canonical URL, and otherwise use a documented source-specific identity. A novel module may add normalized chapter metadata in a namespaced extension when it is queryable or title-specific. The original source values remain available for provenance.

Newly discovered items begin unread. Refreshes are additive and idempotent, and they preserve user read state. A provider must not delete stored items merely because an upstream feed is shortened, reordered, unavailable, or temporarily malformed.

### Calendar boundary

`FeedItem` and `CalendarEvent` are separate records and capabilities:

- A feed publication time says when an update was published.
- A calendar start/end says when an event is scheduled to occur.
- A newly observed novel chapter is a feed item, not an upcoming event.
- A feed item is mapped to a calendar event only when a confirmed source explicitly describes a genuine schedule, or when the user creates an event manually.

The universal calendar can therefore remain useful for games without fabricating novel schedules. A separate updates view can combine feed items from games and novels.

### Ownership boundaries

The shared core owns behavior with the same meaning for both work kinds:

- Library work identity and module assignment
- Base entity identity, multilingual names, descriptions, and stable reference targets
- Notes, rich content, checklists, attachments, and linked folders
- Feed item and calendar event contracts
- SQLite connection management, common migrations, integrity rules, and Git snapshot safety
- HTTP transport, standards-compliant feed parsing, timeouts, caching, retries, and generic provider status

The owning title module handles behavior only that title or source can interpret:

- Entity taxonomy, relationships, and title-specific fields
- Semantic title theme tokens used by shared presentation components
- Source locations, authentication, rate limits, and refresh rules
- Custom HTML/API parsing and source-specific validation
- Mapping to feed items, events, entities, or chapter extensions
- Deduplication fallbacks, time-zone interpretation, fixtures, and module migrations

### Generic modules

The game `generic` module keeps the existing manual fallback behavior. The novel `generic` module provides the same manual core plus user-configured standards-compliant RSS/Atom feeds. It must not accumulate title-specific parsing or entity conditionals. When a title needs those behaviors, add a dedicated module and preserve all common work-owned data during reassignment.

## Migration and compatibility

No executable schema exists when this decision is accepted, so the first schema should use `LibraryWork` rather than introducing a game-only owner. If a game-only schema is implemented before this decision is applied, migrate it additively: preserve stable IDs, set existing records to work kind `game`, retain module assignments, and keep every foreign-key relationship and entity reference intact.

Persisted work kinds, module IDs, source IDs, entity type keys, and feed identities must not be renamed or reused without a data-preserving migration.

## Alternatives considered

### Treat novels as games

- Reuses the existing language with minimal documentation changes.
- Makes game scheduling assumptions leak into novels and obscures important domain differences.
- Rejected because a published chapter update is not a game event or future schedule.

### Separate applications or databases

- Gives games and novels complete isolation.
- Duplicates wiki, notes, links, files, checklists, SQLite safety, and feed infrastructure.
- Rejected because the local-first knowledge workflows are intentionally shared.

### One universal module with title conditionals

- Appears simpler while few titles are supported.
- Accumulates source-specific parsing, incompatible taxonomies, and fragile title checks in shared code.
- Rejected for the same modularity reasons recorded in ADR-001.

### Convert every feed entry into a calendar event

- Produces one combined chronological UI.
- Confuses publication history with future schedules and makes the calendar noisy and misleading.
- Rejected in favor of separate feed and event contracts.

## Consequences

- Shared records and user flows use work-oriented terminology instead of game-only ownership.
- Existing planned game modules remain valid under `games/`.
- Novel support can begin with a generic module and standard feeds without inventing title-specific APIs.
- The application needs an updates view in addition to the universal calendar.
- Module lookup and persisted assignment include work kind as well as module ID.
- Feed parser tests, deduplication tests, read-state persistence tests, and feed-versus-calendar contract tests become part of implementation acceptance.
- Adding another work kind later requires explicit semantics and module-family rules, but does not require duplicating the common wiki core.
