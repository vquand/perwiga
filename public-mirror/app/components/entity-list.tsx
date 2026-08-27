import Image from "next/image";
import type { Entity, EntityType } from "../types";

type EntityListProps = {
  entities: Entity[];
  types: EntityType[];
  selectedId: string | null;
  onSelect: (id: string) => void;
};

function initials(entity: Entity) {
  const label = entity.catalog_label || entity.official_english_name;
  const words = label.replace(/["']/g, "").split(/\s+/).filter(Boolean);
  return words.slice(0, 2).map((word) => word[0]).join("").toUpperCase() || "?";
}

function typeLabel(types: EntityType[], key: string) {
  return types.find((type) => type.key === key)?.display_name || key;
}

export function EntityList({ entities, types, selectedId, onSelect }: EntityListProps) {
  if (!entities.length) {
    return (
      <div className="empty-state" role="status">
        <span className="empty-glyph" aria-hidden="true">⌁</span>
        <h3>No matching records</h3>
        <p>Try another name, category, or filter.</p>
      </div>
    );
  }

  return (
    <ul className="entity-list" aria-label="Public wiki records">
      {entities.map((entity) => {
        const presentation = entity.presentation;
        const label = entity.catalog_label || entity.official_english_name;
        return (
          <li key={entity.id}>
            <button
              type="button"
              className={`entity-row${entity.id === selectedId ? " is-selected" : ""}`}
              onClick={() => onSelect(entity.id)}
              aria-current={entity.id === selectedId ? "true" : undefined}
              style={{ borderLeftColor: presentation?.accent_color || "var(--border)" }}
            >
              <span
                className="entity-avatar"
                style={{ borderColor: presentation?.accent_color || "var(--border-strong)" }}
              >
                {presentation?.thumbnail_url ? (
                  <Image src={presentation.thumbnail_url} alt="" width={48} height={48} loading="lazy" unoptimized />
                ) : (
                  <span aria-hidden="true">{initials(entity)}</span>
                )}
                {presentation?.label && <small>{presentation.label}</small>}
              </span>
              <span className="entity-copy">
                <strong>{label}</strong>
                <span>{entity.official_original_name}</span>
              </span>
              <span className="entity-meta">
                {presentation?.context_label && <small>{presentation.context_label}</small>}
                <small>{typeLabel(types, entity.entity_type)}</small>
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
