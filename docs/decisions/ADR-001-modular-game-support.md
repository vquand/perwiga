# ADR-001: Modular game support behind shared contracts

## Status

Superseded by [ADR-002](ADR-002-games-and-novels-as-library-works.md)

## Date

2026-08-22

## Context

Perwiga has common local-first workflows—multilingual entities, notes, checklists, images, links, and a universal calendar—but supported games do not share one external data shape or one entity taxonomy.

A game may expose event data through RSS, a public or private API, a static page, or no automatic source at all. Source URLs, authentication, rate limits, response formats, deduplication keys, and time-zone semantics can differ. Games also have different entity types, relations, and structured fields.

Putting these differences into shared code would create title-based conditionals, a large nullable schema, and changes that risk breaking unrelated games. Fully dynamic native plugins would provide isolation but add Rust ABI, packaging, security, and cross-platform distribution complexity that is not needed for a private application.

The initially planned games and series are:

- Genshin Impact
- Honkai: Star Rail
- Arknights: Endfield
- Heroes of Might and Magic
- StarCraft and StarCraft: Brood War
- StarCraft II
- Warcraft
- World of Warcraft

## Decision

Use statically linked Rust game modules behind a versioned shared contract. A central composition root registers concrete modules at application startup. Shared core code consumes only module descriptors and capability interfaces; it does not import concrete modules or switch on game names.

Provide a built-in `generic` module for arbitrary games without dedicated support. It exposes the common manual capabilities and no game-specific integration. A game can later migrate from `generic` to a dedicated module without replacing its common game or content records.

### Dependency direction

```text
Desktop UI ──> Application core ──> Shared game-module contract
                      │                         ▲
                      ├──> Shared storage       │
                      ├──> Shared integrations  │
                      │                         │
Composition root ─────┴────────────> Concrete game modules
```

The composition root is the only place that knows the full list of concrete game modules.

### Required module contract

The exact Rust trait signatures will be defined with the first executable architecture. The contract must expose these concepts without leaking provider-specific response types:

| Contract concept | Responsibility |
| --- | --- |
| Module identity | Stable, persisted module ID plus display name and optional franchise metadata. |
| Capability manifest | Declares supported entity schemas, event sources, RSS/API integrations, and optional custom views. |
| Entity type definitions | Namespaced type IDs, labels, relationships, validation, and display metadata. |
| Event source descriptors | Stable source IDs, refresh policy, configuration needs, and availability. |
| Fetch operation | Uses shared transport services and returns validated source records or structured provider errors. |
| Normalization | Maps source records into the common event/entity contracts with provenance and deduplication identity. |
| Module migrations | Applies deterministic, namespaced schema changes for game-specific structured data. |

Contract changes should be additive where possible. Persisted module, entity-type, and source identifiers must not be renamed or reused without migrations.

### Common ownership

The common core owns behavior that has the same meaning for every game:

- Game records and registration.
- Base entity IDs, game ownership, multilingual names, descriptions, and reference targets.
- Notes, rich content, entity links, checklists, attachments, and linked folders.
- Normalized event records and universal calendar presentation.
- SQLite connection management, common migrations, integrity checks, and Git snapshot safety.
- HTTP transport, timeouts, caching, retries, scheduling, and generic provider status.
- Cross-platform filesystem and application-path abstractions.

### Module ownership

Each game module owns facts and behavior that only its game can interpret correctly:

- Entity type taxonomy and game-specific relationships.
- Additional queryable fields and their validation/display metadata.
- RSS/API/source locations, credentials requirements, rate limits, and refresh rules.
- Response parsing, source-specific validation, and fixture data.
- Mapping to normalized events/entities, including source IDs, deduplication, and time zones.
- Module-specific tables and migrations.
- Rare custom UI that cannot be represented by shared capability-driven components.

Shared transport may retrieve bytes or feed entries, but it must not interpret game-specific fields. External responses remain untrusted until the owning module validates them.

### Entity storage

Use a hybrid schema:

- Common tables contain stable entity identity, game ownership, type key, multilingual names, descriptions, timestamps, and the target used by note/checklist references.
- Modules use namespaced normalized tables for additional structured fields that need validation, querying, or relations.
- Small non-queryable source snapshots may be retained for provenance, but an opaque JSON blob is not the primary model for editable structured content.
- Entity type keys are namespaced by module so identical labels in different games do not collide.

### Event storage

All modules map events into one common calendar contract. The normalized record includes the owning game, stable source identity, title, start/end or all-day semantics, time-zone information, provenance, and refresh state. Provider-specific payloads never become the calendar UI contract.

### Module granularity

A module boundary follows data and integration boundaries, not only franchise branding. Related games may share a private helper crate, but remain separate modules when their sources or schemas differ. In particular, StarCraft: Brood War and StarCraft II are separate modules, as are Warcraft and World of Warcraft.

The `generic` module is the fallback boundary for manually managed games. It does not acquire title-specific conditionals over time; when a game needs custom entities or integrations, add a dedicated module and migrate the game’s module assignment while preserving common data.

## Alternatives considered

### One generic schema and shared source layer

- Simpler initial file layout.
- Produces game-name conditionals and many nullable or opaque fields.
- Makes changes for one game risky for every other game.
- Rejected because game data and external sources differ materially.

### Fully separate applications or databases per game

- Maximum isolation.
- Duplicates notes, checklists, image handling, multilingual naming, migrations, and calendar behavior.
- Prevents a straightforward universal calendar and shared entity-reference experience.
- Rejected because most user workflows are common.

### Runtime native plugin system

- Allows modules to be installed without rebuilding the app.
- Introduces ABI/versioning, trust, discovery, signing, and cross-platform packaging problems.
- Rejected for the initial private application. The shared contract may be kept clean enough to revisit this later.

## Consequences

- Adding a game requires a module, registry entry, capability declaration, and contract tests, but should not require edits to unrelated modules or shared calendar logic.
- The common entity/event contracts must remain small and stable.
- Module-specific migrations require deterministic ordering and clear failure reporting.
- Shared franchise helpers are allowed but cannot become dependencies of unrelated modules.
- All supported modules ship with the application binary, so adding or updating one requires rebuilding the app.
- A game can still use all manual common features when it has no RSS/API event provider.
- Arbitrary games remain addable through the `generic` module, and dedicated support can be introduced later without migrating shared content to a different ownership model.
