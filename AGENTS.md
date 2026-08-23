# Agent instructions for Perwiga

## Scope and sources

These rules apply to the whole repository unless a deeper `AGENTS.md` adds more specific instructions. Read the relevant parts of [README.md](README.md) before changing behavior or data contracts. Follow [ADR-002](docs/decisions/ADR-002-games-and-novels-as-library-works.md) for work ownership and title-module boundaries; [ADR-001](docs/decisions/ADR-001-modular-game-support.md) is historical context.

Perwiga is a private, local-first desktop wiki for games and novels. The planned core is Rust with SQLite, targeting macOS, Windows, and Linux. The repository is currently documentation-only; do not invent framework, build, migration, or test commands before they exist and are verified.

## Non-negotiable SQLite data safety

- The primary SQLite database is personal source data and is intentionally tracked by Git. Never add it to `.gitignore`, publish it, or expose its history.
- Never delete or clear data in the primary SQLite database unless the user explicitly asks for that exact destructive change.
- Data deletion includes deleting one or more rows and clearing stored cells without deleting their rows. Treat `DELETE`, `DROP TABLE`, destructive schema rebuilds, `REPLACE`/`INSERT OR REPLACE`, cascading deletes, and updates that set existing values to `NULL`, empty text, zero, or defaults as destructive when they discard information.
- Cleanup, deduplication, reset, re-import, migration, test setup, and “fix inconsistent data” requests do not implicitly authorize data loss. Preserve the original values unless deletion is explicit.
- Never run mutating SQL against a database whose path or role is uncertain. Tests, fixtures, and experiments must use temporary databases and must never open the tracked personal database.

If the user explicitly requests deletion or clearing:

1. Resolve the exact database file, tables, rows, columns, and intended values with read-only inspection.
2. Close/checkpoint active connections or use SQLite’s online backup mechanism so pending WAL/journal data is included. Copying only the main file while uncheckpointed WAL data exists is not a valid backup.
3. Create a new timestamped backup without overwriting an older backup. Verify the backup with `PRAGMA integrity_check` and record counts for affected tables.
4. Report the verified backup path and recovery purpose before mutating the original database.
5. Apply only the requested change in a scoped transaction, then run integrity and affected-row checks again.

Migrations must be additive and data-preserving by default. Do not introduce `ON DELETE CASCADE`, destructive conflict handling, or a lossy migration without explicit user approval and the verified-backup procedure above.

## Architecture guardrails

- Keep shared workflows in the core and title-specific entity schemas, integrations, parsers, and migrations in `games/<module-id>/` or `novels/<module-id>/`.
- Every title module must contain its own `AGENTS.md`. Before editing a module, read the family-level and module-local instructions. Create the local file before adding a new module.
- Use each work family’s `generic` module for titles without dedicated support. Shared code must not branch on titles or module IDs outside the composition/registration boundary.
- Keep domain logic, SQLite access, filesystem coordination, and title integrations in Rust. A thin non-Rust UI layer is allowed if the selected desktop framework requires it.
- Preserve stable IDs, Unicode text, translation provenance, entity links, and work ownership. Automatic translations must never overwrite official text.
- Keep published feed activity separate from scheduled calendar events. Do not infer a future novel schedule from an RSS/Atom publication timestamp.
- Treat local files, URLs, RSS, APIs, and embedded content as untrusted boundary input. Validate and sanitize them before persistence or rendering.
- Linking or previewing a local folder must never move, rename, overwrite, or delete the user’s files.

## Working rules

- Inspect the repository, Git status, relevant specs, and scoped `AGENTS.md` files before editing.
- Preserve unrelated user changes and keep documentation synchronized with behavior and data contracts.
- Use migrations for schema changes and transactions for multi-record writes.
- Record expensive-to-reverse architectural choices in `docs/decisions/`.
- Add tests for persistence, ownership isolation, migrations, multilingual round-tripping, entity links, feed deduplication, feed read-state persistence, feed/calendar separation, external-source validation, and module contracts once implementation exists.
- Never commit secrets, generated build output, temporary test databases, or SQLite runtime sidecars unless a later accepted design explicitly requires them.

## Current repository state

There are no verified application commands yet. Do not report a build, test, lint, migration, or packaging command until it exists in the repository and has been run successfully.
