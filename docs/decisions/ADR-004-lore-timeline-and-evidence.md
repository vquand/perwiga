# ADR-004: Source-backed lore timeline with reviewable candidate batches

## Status

Accepted

## Date

2026-08-29

## Context

The Endfield module has a useful catalog of Operators, NPCs, regions, and
scheduled events, but catalog identity is not the same as narrative evidence.
Lore needs a timeline that can represent exact and fuzzy time, competing claims,
hear-say, unresolved entities, and relationships across events without inventing
Gregorian dates or presenting generated prose as canon. The map should read from
past on the left to future on the right, while remaining useful when the source
only gives an era or relative ordering.

An offline LLM curation session is a convenient way to turn an official corpus
into structured candidates, but its output is untrusted input. The personal
SQLite database is tracked source data, so imports must be additive,
idempotent, reviewable, and safe to reject.

## Decision

### Canonical model

Add a shared lore capability and normalized tables for:

- periods/eras with an explicit display order and optional parent period;
- source records and short exact evidence excerpts with locators and content
  hashes;
- events with revisioned English titles/summaries and a typed time precision;
- event-to-event ordering/overlap relationships;
- lore subjects and their involvement roles;
- claims with assertion kind, certainty, branch group, and claim evidence;
- candidate batches and human review decisions.

Lore subjects are separate from `wiki_entities` because an unknown or
hear-say entity cannot satisfy the wiki row's required official names. A
subject may later link to a canonical wiki entity through an external identity
or a human merge, while retaining its attested name and provenance in the
meantime. This keeps the visible lore subject list useful without fabricating
official fields.

### Time and branching

The model stores `moment` or `interval`, a precision (`exact`, `bounded`,
`approximate`, `relative`, or `unknown`), and a human-readable time label. It
does not coerce lore into a calendar date. Branch groups describe competing
claims or interpretations of the same event; they do not represent alternate
universes. Precedence edges are validated for cycles before persistence.

### LLM boundary and source policy

The first version has no provider integration. An offline curation session
produces a JSON batch conforming to
`docs/schemas/lore-candidate-batch-v1.schema.json`. The batch records the
official-corpus manifest hash and generator identity. The Endfield module owns
the source vocabulary and subject-role vocabulary; the shared core validates,
stores, reviews, and materializes candidates.

Only official in-game dialogue/archives, item/operator text, and official web
publications are in scope for the initial corpus. Community sources may be
added later as a distinct provider class. The database keeps hashes, locators,
and short exact excerpts rather than copying an entire corpus.

Sources and candidates are additive and idempotent. Sources receive an
automatic validation status; events, claims, ordering, and unresolved subjects
remain pending until a human approves them. Each approval materializes the
approved portion whose dependencies are available; later approvals safely
extend the same graph. Rejection leaves the source record and decision trail
intact.

### Interfaces

The CLI accepts an explicit JSON file:

```text
cargo run -p perwiga -- --database /tmp/lore.sqlite \
  lore candidates import --work <work-id> --file <batch.json>
```

The localhost UAT exposes a horizontal period-spine map, event detail with
claims/evidence, a provisional-subject filter, and an in-app review queue. The
existing scheduled-event calendar is unchanged and remains a separate
capability.

## Consequences

- Lore can be curated offline and audited before it becomes canonical.
- The map preserves uncertainty instead of implying unsupported dates.
- Provisional subjects can be used immediately and linked later without data
  loss.
- The shared core owns persistence and review mechanics; Endfield owns only its
  vocabulary and source extraction.
- A future LLM/provider integration must write the same candidate contract and
  cannot bypass validation or human review.

## Alternatives considered

### Store lore as notes or calendar events

This would lose normalized evidence, claim certainty, and many-to-many subject
relationships, and would confuse narrative chronology with scheduled events.
Rejected.

### Put every unknown entity directly in `wiki_entities`

This would require fake official names/descriptions or weakening the shared
entity contract. Rejected in favor of a provisional lore-subject table with a
later canonical link.

### Let the LLM write directly to canonical tables

This would make generated claims indistinguishable from reviewed canon and
would make retries difficult to audit. Rejected in favor of candidate batches
and explicit review decisions.
