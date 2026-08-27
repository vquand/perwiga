export type Theme = {
  background: string;
  surface: string;
  surface_raised: string;
  surface_active: string;
  border: string;
  border_strong: string;
  text: string;
  text_muted: string;
  text_subtle: string;
  accent: string;
  accent_ink: string;
};

export type Facet = {
  key: string;
  display_name: string;
  entity_types: string[];
  options: Array<{ value: string; display_name: string }>;
};

export type EntityType = {
  key: string;
  display_name: string;
  description: string;
};

export type Presentation = {
  thumbnail_url: string;
  accent_color: string;
  context_label?: string;
  context_icon_url?: string;
  label: string;
  rarity?: number;
  facets: Record<string, string>;
  facet_values: Record<string, string[]>;
};

export type Alias = {
  value: string;
  language?: string;
  kind: string;
  label?: string;
};

export type Appearance = {
  relation_kind: string;
  related_title: string;
  related_source_url?: string;
  source_provider: string;
  locations: Array<{ location_name: string; region_name?: string }>;
};

export type EntityEventRecency = {
  heading: string;
  event_title: string;
  ended_at: string;
};

export type Entity = {
  id: string;
  work_id: string;
  entity_type: string;
  official_english_name: string;
  official_original_name: string;
  official_vietnamese_name?: string;
  automatic_vietnamese_translation?: string;
  english_description?: string;
  aliases: Alias[];
  appearances: Appearance[];
  catalog_label?: string;
  presentation?: Presentation;
  event_recency?: EntityEventRecency;
};

export type EventFeaturedEntity = {
  display_name: string;
  thumbnail_url: string;
  accent_color: string;
  label: string;
  previous_event_gap?: { days: number; event_title: string; ended_at: string };
};

export type Event = {
  id: string;
  work_id: string;
  title: string;
  starts_at: string;
  ends_at?: string;
  is_all_day: boolean;
  source_url?: string;
  source_provider: string;
  event_type?: string;
  time_zone?: string;
  patch_start?: string;
  patch_end?: string;
  presentation?: { heading: string; featured_entities: EventFeaturedEntity[] };
};

export type Work = {
  id: string;
  kind: "game" | "novel";
  module_id: string;
  display_name: string;
  theme: Theme;
  entity_types: EntityType[];
  facets: Facet[];
};

export type Catalog = {
  schema: string;
  works: Work[];
  entities: Entity[];
  events: Event[];
};
