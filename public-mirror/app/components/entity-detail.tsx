import Image from "next/image";
import type { Entity } from "../types";

type EntityDetailProps = { entity: Entity | undefined };

function Detail({ label, value }: { label: string; value?: string }) {
  if (!value) return null;
  return (
    <div className="detail-field">
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

export function EntityDetail({ entity }: EntityDetailProps) {
  if (!entity) {
    return (
      <div className="detail-empty">
        <span className="empty-glyph" aria-hidden="true">↗</span>
        <h2>Choose a record</h2>
        <p>Select a record to see its names, translations, aliases, and appearances.</p>
      </div>
    );
  }

  const label = entity.catalog_label || entity.official_english_name;
  const hanViet = entity.aliases.filter((alias) => alias.kind === "han-viet");
  const otherAliases = entity.aliases.filter((alias) => alias.kind !== "han-viet");

  return (
    <article className="detail-card">
      <header className="detail-heading">
        <div
          className="detail-avatar"
          style={{ borderColor: entity.presentation?.accent_color || "var(--accent)" }}
        >
          {entity.presentation?.thumbnail_url ? (
            <Image src={entity.presentation.thumbnail_url} alt="" width={104} height={104} unoptimized />
          ) : (
            <span aria-hidden="true">{label.slice(0, 2).toUpperCase()}</span>
          )}
        </div>
        <div>
          <p className="eyebrow">{entity.entity_type.replaceAll("-", " ")}</p>
          <h2>{label}</h2>
          <p className="detail-original">{entity.official_original_name}</p>
          {entity.presentation?.label && (
            <span className="rarity-chip" style={{ borderColor: entity.presentation.accent_color }}>
              {entity.presentation.label}
            </span>
          )}
        </div>
      </header>

      <dl className="detail-fields">
        <Detail label="Official English" value={entity.official_english_name} />
        <Detail label="Official original" value={entity.official_original_name} />
        <Detail label="Official Vietnamese" value={entity.official_vietnamese_name} />
        <Detail label="Automatic Vietnamese" value={entity.automatic_vietnamese_translation} />
      </dl>

      {entity.english_description && (
        <section className="detail-section">
          <h3>Description</h3>
          <p>{entity.english_description}</p>
        </section>
      )}

      {hanViet.length > 0 && (
        <section className="detail-section">
          <h3>Hán Việt</h3>
          <div className="alias-cloud">
            {hanViet.map((alias) => <span key={`${alias.kind}-${alias.value}`} className="alias-pill">{alias.value}</span>)}
          </div>
        </section>
      )}

      {otherAliases.length > 0 && (
        <section className="detail-section">
          <h3>Known names</h3>
          <div className="alias-cloud">
            {otherAliases.map((alias) => (
              <span key={`${alias.kind}-${alias.value}`} className="alias-pill">
                {alias.value}{alias.language ? ` · ${alias.language}` : ""}
              </span>
            ))}
          </div>
        </section>
      )}

      {entity.appearances.length > 0 && (
        <section className="detail-section">
          <h3>Appears in</h3>
          <ul className="appearance-list">
            {entity.appearances.map((appearance) => (
              <li key={`${appearance.relation_kind}-${appearance.related_title}`}>
                <span className="appearance-kind">{appearance.relation_kind}</span>
                {appearance.related_source_url ? (
                  <a href={appearance.related_source_url} target="_blank" rel="noreferrer">{appearance.related_title}</a>
                ) : (
                  <span>{appearance.related_title}</span>
                )}
                {appearance.locations.length > 0 && (
                  <small>{appearance.locations.map((location) => location.location_name).join(" · ")}</small>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}
    </article>
  );
}
