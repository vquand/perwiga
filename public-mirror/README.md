# Perwiga Public Atlas

This is the public, read-only presentation of selected Perwiga catalog data.
It is deliberately not a second application database and it has no write
routes, notes, checklist state, local-folder references, feed read-state, or
private provenance notes.

## Source of truth

The canonical source remains [`../perwiga.sqlite`](../perwiga.sqlite). Run
`npm run prepare:public` from this directory whenever the canonical database or
module presentation code changes. The command creates a disposable JSON
projection and stages only the local thumbnails required by that projection.

The exporter includes records with curated/source provenance and verified
scheduled events. It intentionally omits free-form internal metadata and
private workspace tables. The generated files under `public/data/` and
`public/assets/` are deployment inputs, not an independently edited database.

## Local validation

```text
npm ci --ignore-scripts
npm run prepare:public
npm run build
npm test
```

The hosted site is configured for public URL access, so visitors do not need an
account. Editing remains available only through the local Perwiga application.
