# ADR-003: Implement the first vertical slice as a Rust core with a CLI composition root

## Status

Accepted

## Date

2026-08-22

## Context

The product requirements require Rust for domain logic, SQLite persistence, and title integrations, while leaving the desktop UI framework replaceable until rich text, local files, image thumbnails, safe embeds, and packaging have been evaluated. The repository had no executable implementation or verified development commands.

The first implementation needs to prove the data contracts and the first dedicated game module without coupling persistence to a presentation framework or inventing an unconfirmed Endfield provider.

## Decision

Use a Cargo workspace with:

- `crates/perwiga-core` for the family-aware module contract, additive SQLite schema, validation, common workflows, bounded HTTP feed transport, and RSS/Atom normalization.
- `crates/perwiga-cli` as the composition root and first usable local interface. It registers the generic game, generic novel, and Arknights: Endfield modules and delegates behavior to the core.
- `games/<module-id>/` and `novels/<module-id>/` as statically linked Rust module crates.

The CLI uses an explicit SQLite path and all self-tests use in-memory or temporary databases. The canonical personal database remains a normal SQLite file and is not ignored or created by tests.

## Alternatives considered

### Select a desktop framework immediately

- Would provide a visible desktop shell sooner.
- Would make rich text, safe embeds, local image browsing, and cross-platform packaging decisions before the requirements have been evaluated.
- Rejected for the first slice; the core boundary allows a later desktop UI to consume the same application contract.

### Put Endfield data and title conditionals in the core

- Would make the first game appear faster.
- Would violate the module ownership rules and make future title support harder to isolate.
- Rejected; Endfield metadata and entity declarations live in its module, while common persistence remains shared.

### Use a dynamic plugin ABI

- Would isolate modules at runtime.
- Adds ABI, packaging, versioning, and security complexity that is unnecessary for a private local-first application.
- Rejected in favor of statically linked modules.

## Consequences

- The core can be tested without a UI or network service.
- The CLI is a usable self-test and data-management surface, not a final desktop experience.
- Future desktop work must preserve the typed core contracts and keep title-specific behavior behind module registration.
- Endfield automatic feeds/events remain opt-in work backed by confirmed sources rather than guessed integrations.
