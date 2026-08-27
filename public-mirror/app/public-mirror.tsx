"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { EntityDetail } from "./components/entity-detail";
import { EntityList } from "./components/entity-list";
import { Timeline } from "./components/timeline";
import type { Catalog, Entity, Facet, Work } from "./types";

type PublicMirrorProps = { initialCatalog: Catalog };

function entityName(entity: Entity) { return entity.catalog_label || entity.official_english_name; }

function matchesQuery(entity: Entity, query: string) {
  if (!query.trim()) return true;
  const needle = query.trim().toLocaleLowerCase();
  return [entity.official_english_name, entity.official_original_name, entity.official_vietnamese_name, entity.automatic_vietnamese_translation, ...entity.aliases.map((alias) => alias.value)].some((value) => value?.toLocaleLowerCase().includes(needle));
}

function matchesFacet(entity: Entity, facet: Facet, value: string) {
  if (!value) return true;
  return entity.presentation?.facets?.[facet.key] === value || entity.presentation?.facet_values?.[facet.key]?.includes(value) === true;
}

function sortEntities(entities: Entity[], sort: string) {
  const byName = (left: Entity, right: Entity) => entityName(left).localeCompare(entityName(right), undefined, { sensitivity: "base" });
  if (sort === "name") return [...entities].sort(byName);
  const direction = sort === "rarity-asc" ? 1 : -1;
  return [...entities].sort((left, right) => {
    const leftRarity = left.presentation?.rarity;
    const rightRarity = right.presentation?.rarity;
    if (leftRarity === undefined && rightRarity !== undefined) return 1;
    if (leftRarity !== undefined && rightRarity === undefined) return -1;
    if (leftRarity !== rightRarity) return ((leftRarity || 0) - (rightRarity || 0)) * direction;
    return byName(left, right);
  });
}

function countForType(entities: Entity[], type: string) { return type ? entities.filter((entity) => entity.entity_type === type).length : entities.length; }

function themeStyle(work: Work | undefined): React.CSSProperties {
  if (!work) return {};
  return { "--background": work.theme.background, "--surface": work.theme.surface, "--surface-raised": work.theme.surface_raised, "--surface-active": work.theme.surface_active, "--border": work.theme.border, "--border-strong": work.theme.border_strong, "--text": work.theme.text, "--text-muted": work.theme.text_muted, "--text-subtle": work.theme.text_subtle, "--accent": work.theme.accent, "--accent-ink": work.theme.accent_ink } as React.CSSProperties;
}

export function PublicMirror({ initialCatalog }: PublicMirrorProps) {
  const [workId, setWorkId] = useState(initialCatalog.works[0]?.id || "");
  const [view, setView] = useState<"wiki" | "timeline">("wiki");
  const [type, setType] = useState("");
  const [query, setQuery] = useState("");
  const [rarity, setRarity] = useState("");
  const [sort, setSort] = useState("name");
  const [facetValues, setFacetValues] = useState<Record<string, string>>({});
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const work = initialCatalog.works.find((candidate) => candidate.id === workId) || initialCatalog.works[0];
  const workEntities = useMemo(() => initialCatalog.entities.filter((entity) => entity.work_id === work?.id), [initialCatalog.entities, work?.id]);
  const workEvents = useMemo(() => initialCatalog.events.filter((event) => event.work_id === work?.id), [initialCatalog.events, work?.id]);
  const facets = useMemo(() => (work?.facets || []).filter((facet) => !type || facet.entity_types.includes(type)), [work?.facets, type]);
  const rarityOptions = useMemo(() => Array.from(new Set(workEntities.filter((entity) => !type || entity.entity_type === type).map((entity) => entity.presentation?.rarity).filter((value): value is number => value !== undefined))).sort((left, right) => right - left), [workEntities, type]);
  const filteredEntities = useMemo(() => {
    const filtered = workEntities.filter((entity) => {
      if (type && entity.entity_type !== type) return false;
      if (rarity && entity.presentation?.rarity !== Number(rarity)) return false;
      if (!matchesQuery(entity, query)) return false;
      return facets.every((facet) => matchesFacet(entity, facet, facetValues[facet.key] || ""));
    });
    return sortEntities(filtered, sort);
  }, [facetValues, facets, query, rarity, sort, type, workEntities]);
  const activeEntity = filteredEntities.find((entity) => entity.id === selectedId) || filteredEntities[0];
  const visibleEntities = filteredEntities.slice(0, 350);

  function switchWork(nextWorkId: string) { setWorkId(nextWorkId); setType(""); setQuery(""); setRarity(""); setSort("name"); setFacetValues({}); setSelectedId(null); }
  function switchType(nextType: string) { setType(nextType); setRarity(""); setFacetValues({}); setSelectedId(null); }

  return (
    <div style={themeStyle(work)}>
      <a className="skip-link" href="#public-content">Skip to public catalog</a>
      <header className="topbar"><Link className="brand" href="/" aria-label="Perwiga Public Atlas home"><span className="brand-mark" aria-hidden="true">P</span><span>PERWIGA</span></Link><span className="public-badge">Public read-only mirror</span></header>
      <main id="public-content">
        <section className="hero" aria-labelledby="atlas-title"><div><p className="eyebrow">Public catalog / generated from Perwiga</p><h1 id="atlas-title">Perwiga<br />Atlas</h1><p>Explore selected multilingual wiki records and verified event schedules from the Perwiga library.</p></div><div className="hero-note"><strong>No account required</strong><span>This view is read-only. Private notes, checklists, folder links, feed state, and internal metadata are never exported.</span></div></section>
        <nav className="work-tabs" aria-label="Public library works">{initialCatalog.works.map((candidate) => <button key={candidate.id} type="button" className={`work-tab${candidate.id === work?.id ? " is-active" : ""}`} onClick={() => switchWork(candidate.id)}>{candidate.display_name}<small>{candidate.kind} · {initialCatalog.entities.filter((entity) => entity.work_id === candidate.id).length} records</small></button>)}</nav>
        <nav className="view-tabs" aria-label="Public catalog view"><button type="button" className={`view-tab${view === "wiki" ? " is-active" : ""}`} onClick={() => setView("wiki")}>Wiki</button><button type="button" className={`view-tab${view === "timeline" ? " is-active" : ""}`} onClick={() => setView("timeline")}>Timeline</button></nav>
        {view === "wiki" && work ? <section className="wiki-layout" aria-label={`${work.display_name} public wiki`}><aside className="catalog-nav" aria-label="Record types"><p className="panel-kicker">Browse records</p><div className="type-list"><button type="button" className={`type-button${type === "" ? " is-active" : ""}`} onClick={() => switchType("")}>All records <small>{workEntities.length}</small></button>{work.entity_types.map((entityType) => <button type="button" key={entityType.key} className={`type-button${type === entityType.key ? " is-active" : ""}`} onClick={() => switchType(entityType.key)}>{entityType.display_name} <small>{countForType(workEntities, entityType.key)}</small></button>)}</div></aside><section className="collection" aria-labelledby="collection-title"><div className="collection-head"><div><p className="eyebrow">{work.module_id}</p><h2 id="collection-title">{type ? work.entity_types.find((candidate) => candidate.key === type)?.display_name : "All records"}</h2></div><p className="record-count">{filteredEntities.length.toLocaleString()} found</p></div><label className="search-label" htmlFor="public-search">Search names and aliases<input id="public-search" type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="English, original, Vietnamese, alias…" autoComplete="off" /></label>{(rarityOptions.length > 0 || facets.length > 0) && <div className="filters">{rarityOptions.length > 0 && <label className="filter-field">Rarity<select className="filter-select" value={rarity} onChange={(event) => setRarity(event.target.value)}><option value="">All rarities</option>{rarityOptions.map((value) => <option value={value} key={value}>{value}★</option>)}</select></label>}{facets.map((facet) => <label className="filter-field" key={facet.key}>{facet.display_name}<select className="filter-select" value={facetValues[facet.key] || ""} onChange={(event) => setFacetValues((previous) => ({ ...previous, [facet.key]: event.target.value }))}><option value="">All</option>{facet.options.map((option) => <option value={option.value} key={option.value}>{option.display_name}</option>)}</select></label>)}<label className="filter-field">Sort<select className="filter-select" value={sort} onChange={(event) => setSort(event.target.value)}><option value="name">Name A–Z</option><option value="rarity-desc">Rarity high–low</option><option value="rarity-asc">Rarity low–high</option></select></label></div>}{filteredEntities.length > visibleEntities.length && <p className="filter-note">Showing the first {visibleEntities.length} results. Search or filter to narrow the catalog.</p>}<EntityList entities={visibleEntities} types={work.entity_types} selectedId={activeEntity?.id || null} onSelect={setSelectedId} /></section><aside className="inspector" aria-label="Record detail"><EntityDetail entity={activeEntity} /></aside></section> : work ? <section className="timeline-panel" aria-labelledby="timeline-title"><p className="eyebrow">{work.display_name} / universal schedule</p><h2 id="timeline-title">Event timeline</h2><p className="timeline-intro">Publicly sourced events are grouped into swimlanes by their title-defined type. Dates remain exactly as recorded by the source.</p><Timeline events={workEvents} /></section> : <div className="empty-state"><h3>No public catalog available</h3><p>The mirror has not received a public data snapshot yet.</p></div>}
        <footer className="mirror-footer"><span>Generated from the canonical Perwiga SQLite source.</span><span>Read-only presentation · no login · no editing endpoints</span></footer>
      </main>
    </div>
  );
}
