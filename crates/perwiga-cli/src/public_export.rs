use std::{collections::HashMap, fs, path::PathBuf};

use clap::Args;
use perwiga_core::{
    model::{CalendarEvent, EntityAlias, EntityAppearance, LibraryWork, WikiEntity},
    CalendarEventPresentation, EntityEventRecencyPresentation, EntityFacetDefinition,
    EntityPresentation, LibraryModule, PerwigaError, Result, Store, ThemeDefinition, WorkKind,
};
use serde::Serialize;
use url::Url;

const PUBLIC_CATALOG_SCHEMA: &str = "perwiga.public-catalog.v1";

#[derive(Debug, Args)]
pub struct ExportPublicCommand {
    /// Output JSON path for the generated read-only public catalog.
    #[arg(long)]
    pub output: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct PublicCatalog {
    pub schema: &'static str,
    pub works: Vec<PublicWork>,
    pub entities: Vec<PublicEntity>,
    pub events: Vec<PublicEvent>,
}

#[derive(Debug, Serialize)]
pub struct PublicWork {
    pub id: String,
    pub kind: WorkKind,
    pub module_id: String,
    pub display_name: String,
    pub theme: ThemeDefinition,
    pub entity_types: Vec<PublicEntityType>,
    pub facets: Vec<EntityFacetDefinition>,
}

#[derive(Debug, Serialize)]
pub struct PublicEntityType {
    pub key: String,
    pub display_name: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct PublicEntity {
    pub id: String,
    pub work_id: String,
    pub entity_type: String,
    pub official_english_name: String,
    pub official_original_name: String,
    pub official_vietnamese_name: Option<String>,
    pub automatic_vietnamese_translation: Option<String>,
    pub english_description: Option<String>,
    pub aliases: Vec<PublicAlias>,
    pub appearances: Vec<PublicAppearance>,
    pub catalog_label: Option<String>,
    pub presentation: Option<EntityPresentation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_recency: Option<EntityEventRecencyPresentation>,
}

#[derive(Debug, Serialize)]
pub struct PublicAlias {
    pub value: String,
    pub language: Option<String>,
    pub kind: String,
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublicAppearance {
    pub relation_kind: String,
    pub related_title: String,
    pub related_source_url: Option<String>,
    pub source_provider: String,
    pub locations: Vec<PublicAppearanceLocation>,
}

#[derive(Debug, Serialize)]
pub struct PublicAppearanceLocation {
    pub location_name: String,
    pub region_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublicEvent {
    pub id: String,
    pub work_id: String,
    pub title: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub is_all_day: bool,
    pub source_url: Option<String>,
    pub source_provider: String,
    pub event_type: Option<String>,
    pub time_zone: Option<String>,
    pub patch_start: Option<String>,
    pub patch_end: Option<String>,
    pub presentation: Option<CalendarEventPresentation>,
}

/// Build a public projection from the canonical SQLite database. The
/// projection is intentionally not a second database: it is a disposable,
/// read-only JSON build artifact generated from the canonical source.
pub fn build_catalog(
    store: &Store,
    modules: impl Iterator<Item = &'static dyn LibraryModule>,
) -> Result<PublicCatalog> {
    let modules = modules
        .map(|module| ((module.kind(), module.id()), module))
        .collect::<HashMap<_, _>>();
    let all_works = store.list_works()?;
    let mut works = Vec::new();
    let mut entities = Vec::new();
    let mut public_work_ids = HashMap::new();

    for work in all_works {
        let Some(module) = modules.get(&(work.kind, work.module_id.as_str())) else {
            continue;
        };
        let public_entities = store
            .list_entities(&work.id, None)?
            .into_iter()
            .filter(is_public_entity)
            .collect::<Vec<_>>();
        if public_entities.is_empty() {
            continue;
        }

        public_work_ids.insert(work.id.clone(), (work.kind, work.module_id.clone()));
        entities.extend(
            public_entities
                .into_iter()
                .map(|entity| public_entity(store, *module, entity))
                .collect::<Result<Vec<_>>>()?,
        );
        works.push(public_work(&work, *module));
    }

    let events = store
        .list_calendar_events()?
        .into_iter()
        .filter(|event| is_public_event(event) && public_work_ids.contains_key(&event.work_id))
        .filter_map(|event| {
            let (_, module_id) = public_work_ids.get(&event.work_id)?;
            let module = modules
                .get(&(WorkKind::Game, module_id.as_str()))
                .or_else(|| modules.get(&(WorkKind::Novel, module_id.as_str())))?;
            Some(public_event(*module, event))
        })
        .collect::<Result<Vec<_>>>()?;

    for entity in &mut entities {
        entity.appearances = store
            .list_entity_appearances(&entity.id)?
            .into_iter()
            .filter_map(public_appearance)
            .collect();
    }

    works.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    entities.sort_by(|left, right| {
        left.work_id
            .cmp(&right.work_id)
            .then_with(|| left.entity_type.cmp(&right.entity_type))
            .then_with(|| left.official_english_name.cmp(&right.official_english_name))
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(PublicCatalog {
        schema: PUBLIC_CATALOG_SCHEMA,
        works,
        entities,
        events,
    })
}

pub fn write_catalog(
    store: &Store,
    modules: impl Iterator<Item = &'static dyn LibraryModule>,
    output: &PathBuf,
) -> Result<()> {
    let catalog = build_catalog(store, modules)?;
    let json = serde_json::to_vec_pretty(&catalog)
        .map_err(|error| PerwigaError::Validation(format!("serialize public catalog: {error}")))?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, json)?;
    Ok(())
}

fn public_work(work: &LibraryWork, module: &dyn LibraryModule) -> PublicWork {
    PublicWork {
        id: work.id.clone(),
        kind: work.kind,
        module_id: work.module_id.clone(),
        display_name: work.display_name.clone(),
        theme: module.theme(),
        entity_types: module
            .entity_types()
            .iter()
            .map(|definition| PublicEntityType {
                key: definition.key.to_string(),
                display_name: definition.display_name.to_string(),
                description: definition.description.to_string(),
            })
            .collect(),
        facets: module.entity_facets().to_vec(),
    }
}

fn public_entity(
    store: &Store,
    module: &dyn LibraryModule,
    entity: WikiEntity,
) -> Result<PublicEntity> {
    let entity_id = entity.id.clone();
    let presentation = module
        .entity_presentation(&entity)
        .map(rewrite_entity_presentation);
    let event_recency = module.entity_event_recency(&entity);
    let catalog_label = module.entity_catalog_label(&entity);
    let aliases = store
        .list_aliases(&entity_id)?
        .into_iter()
        .map(public_alias)
        .collect();
    Ok(PublicEntity {
        id: entity.id,
        work_id: entity.work_id,
        entity_type: entity.entity_type,
        official_english_name: entity.official_english_name,
        official_original_name: entity.official_original_name,
        official_vietnamese_name: entity.official_vietnamese_name,
        automatic_vietnamese_translation: entity.automatic_vietnamese_translation,
        english_description: entity.english_description,
        aliases,
        appearances: Vec::new(),
        catalog_label,
        presentation,
        event_recency,
    })
}

fn public_alias(alias: EntityAlias) -> PublicAlias {
    PublicAlias {
        value: alias.value,
        language: alias.language,
        kind: alias.kind,
        label: alias.label,
    }
}

fn public_event(module: &dyn LibraryModule, event: CalendarEvent) -> Result<PublicEvent> {
    let presentation = module
        .calendar_event_presentation(&event)
        .map(rewrite_event_presentation);
    Ok(PublicEvent {
        id: event.id,
        work_id: event.work_id,
        title: event.title,
        starts_at: event.starts_at,
        ends_at: event.ends_at,
        is_all_day: event.is_all_day,
        source_url: public_url(event.source_url.as_deref()),
        source_provider: event.source_provider,
        event_type: event.event_type,
        time_zone: event.time_zone,
        patch_start: event.patch_start,
        patch_end: event.patch_end,
        presentation,
    })
}

fn public_appearance(appearance: EntityAppearance) -> Option<PublicAppearance> {
    let source_url = public_url(appearance.related_source_url.as_deref());
    if source_url.is_none() {
        return None;
    }
    Some(PublicAppearance {
        relation_kind: appearance.relation_kind,
        related_title: appearance.related_title,
        related_source_url: source_url,
        source_provider: appearance.source_provider,
        locations: appearance
            .locations
            .into_iter()
            .map(|location| PublicAppearanceLocation {
                location_name: location.location_name,
                region_name: location.region_name,
            })
            .collect(),
    })
}

fn is_public_entity(entity: &WikiEntity) -> bool {
    let info = entity.other_information.as_deref().unwrap_or_default();
    info.contains("source key:") || info.contains("roster index:")
}

fn is_public_event(event: &CalendarEvent) -> bool {
    event
        .source_identity
        .as_deref()
        .is_some_and(|identity| !identity.is_empty())
        && public_url(event.source_url.as_deref()).is_some()
        && event.source_provider != "manual"
}

fn public_url(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    let parsed = Url::parse(value).ok()?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return None;
    }
    Some(value.to_string())
}

fn rewrite_entity_presentation(mut presentation: EntityPresentation) -> EntityPresentation {
    presentation.thumbnail_url = public_asset_url(&presentation.thumbnail_url);
    presentation.context_icon_url = presentation
        .context_icon_url
        .as_deref()
        .map(public_asset_url);
    presentation
}

fn rewrite_event_presentation(
    mut presentation: CalendarEventPresentation,
) -> CalendarEventPresentation {
    for entity in &mut presentation.featured_entities {
        entity.thumbnail_url = public_asset_url(&entity.thumbnail_url);
    }
    presentation
}

fn public_asset_url(value: &str) -> String {
    value
        .strip_prefix("/assets/modules/")
        .map(|path| format!("/assets/modules/{path}"))
        .unwrap_or_else(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use perwiga_core::{
        model::{AliasInput, EntityAppearanceInput, EntityInput, WikiEntity},
        Capability, EntityEventRecencyPresentation, EntityTypeDefinition, LibraryModule,
        ModuleRegistry, Store, WorkKind,
    };

    use super::build_catalog;

    struct RecencyModule;

    static RECENCY_MODULE: RecencyModule = RecencyModule;
    static RECENCY_ENTITY_TYPES: &[EntityTypeDefinition] = &[];

    impl LibraryModule for RecencyModule {
        fn id(&self) -> &'static str {
            "recency-test"
        }

        fn kind(&self) -> WorkKind {
            WorkKind::Game
        }

        fn display_name(&self) -> &'static str {
            "Recency Test"
        }

        fn capabilities(&self) -> &'static [Capability] {
            &[]
        }

        fn entity_types(&self) -> &'static [EntityTypeDefinition] {
            RECENCY_ENTITY_TYPES
        }

        fn entity_event_recency(
            &self,
            _entity: &WikiEntity,
        ) -> Option<EntityEventRecencyPresentation> {
            Some(EntityEventRecencyPresentation {
                heading: "Last limited banner".into(),
                event_title: "Previous Banner".into(),
                ended_at: "2026-06-05T06:00:00+08:00".into(),
            })
        }
    }

    #[test]
    fn public_export_contains_curated_records_but_excludes_unmarked_records() {
        let mut store = Store::open_in_memory().expect("in-memory store");
        let work = store
            .insert_work(WorkKind::Game, "generic", "Public Game")
            .expect("work");
        let curated = store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "character".to_string(),
                    official_english_name: "Curated Character".to_string(),
                    official_original_name: "原名".to_string(),
                    official_vietnamese_name: Some("Nhân vật".to_string()),
                    automatic_vietnamese_translation: None,
                    english_description: Some("A public description.".to_string()),
                    other_information: Some(
                        "Curated source key: character-1. Source: https://example.com/source"
                            .to_string(),
                    ),
                },
            )
            .expect("curated entity");
        store
            .add_alias(
                &curated.id,
                &AliasInput {
                    value: "Hán Việt".to_string(),
                    language: Some("vi-Han".to_string()),
                    kind: "han-viet".to_string(),
                    label: Some("Hán Việt".to_string()),
                    notes: Some("Internal provenance must not be published".to_string()),
                },
            )
            .expect("alias");
        let private = store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "character".to_string(),
                    official_english_name: "Private Character".to_string(),
                    official_original_name: "秘密".to_string(),
                    official_vietnamese_name: None,
                    automatic_vietnamese_translation: None,
                    english_description: Some("This must stay local.".to_string()),
                    other_information: None,
                },
            )
            .expect("private entity");
        store
            .import_entity_appearances(&[EntityAppearanceInput {
                entity_id: curated.id.clone(),
                relation_kind: "quest".to_string(),
                related_title: "Public Quest".to_string(),
                related_source_url: Some("https://example.com/quest".to_string()),
                source_provider: "Public source".to_string(),
                source_identity: "quest-1".to_string(),
                source_notes: Some("Private note must not be published".to_string()),
                locations: Vec::new(),
            }])
            .expect("appearance");

        let mut registry = ModuleRegistry::default();
        perwiga_game_generic::register(&mut registry).expect("generic module");
        let catalog = build_catalog(&store, registry.all()).expect("public catalog");

        assert_eq!(catalog.entities.len(), 1);
        assert_eq!(catalog.entities[0].id, curated.id);
        assert_eq!(catalog.entities[0].aliases[0].value, "Hán Việt");
        assert_eq!(catalog.entities[0].appearances.len(), 1);
        assert!(!catalog
            .entities
            .iter()
            .any(|entity| entity.id == private.id));
    }

    #[test]
    fn public_export_preserves_module_owned_event_recency() {
        let store = Store::open_in_memory().expect("in-memory store");
        let work = store
            .insert_work(WorkKind::Game, "recency-test", "Recency Test")
            .expect("work");
        store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "character".to_string(),
                    official_english_name: "Featured Character".to_string(),
                    official_original_name: "原名".to_string(),
                    official_vietnamese_name: None,
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some("Curated source key: character-1".to_string()),
                },
            )
            .expect("entity");

        let catalog = build_catalog(
            &store,
            std::iter::once(&RECENCY_MODULE as &'static dyn LibraryModule),
        )
        .expect("public catalog");

        assert_eq!(
            catalog.entities[0].event_recency,
            Some(EntityEventRecencyPresentation {
                heading: "Last limited banner".into(),
                event_title: "Previous Banner".into(),
                ended_at: "2026-06-05T06:00:00+08:00".into(),
            })
        );
    }
}
