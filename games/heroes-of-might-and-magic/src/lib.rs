use std::collections::{BTreeMap, HashSet};

use perwiga_core::{
    model::{EntityImportSummary, EntityInput, SourcedEntityInput},
    Capability, EntityTypeDefinition, LibraryModule, ModuleRegistry, PerwigaError, Store, WorkKind,
};
use serde::Deserialize;

const SNAPSHOT_SCHEMA: &str = "perwiga.heroes-of-might-and-magic.heroes-iii-complete.v1";
const MODULE_ID: &str = "heroes-of-might-and-magic";
const EDITION: &str = "Heroes of Might and Magic III Complete";
const VCMI_URL: &str = "https://github.com/vcmi/vcmi";
const VCMI_COMMIT: &str = "93dd4eeac1a9f41ff8c7f090c1d323633a32d69e";
const WIKI_API_URL: &str = "https://heroes.thelazy.net/api.php";
const SOURCE_PROVIDER: &str = "VCMI and Heroes 3 Wiki";

#[derive(Debug, Deserialize)]
struct CuratedSnapshot {
    schema: String,
    checked_at: String,
    module_id: String,
    edition: String,
    sources: SnapshotSources,
    catalog_scope: CatalogScope,
    records: Vec<CuratedRecord>,
}

#[derive(Debug, Deserialize)]
struct SnapshotSources {
    structure: StructuralSource,
    labels_and_facts: WikiCatalogSource,
}

#[derive(Debug, Deserialize)]
struct StructuralSource {
    url: String,
    commit: String,
}

#[derive(Debug, Deserialize)]
struct WikiCatalogSource {
    url: String,
}

#[derive(Debug, Deserialize)]
struct CatalogScope {
    included_records: usize,
    counts: BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
struct CuratedRecord {
    source_key: String,
    entity_type: String,
    vcmi_key: String,
    vcmi_index: u32,
    official_english_name: String,
    official_original_name: String,
    official_vietnamese_name: Option<String>,
    vcmi_source_file: String,
    wiki_source: WikiSource,
    facts: Vec<CatalogFact>,
}

#[derive(Debug, Deserialize)]
struct WikiSource {
    title: String,
    url: String,
    page_id: u64,
    revision_id: u64,
    revision_timestamp: String,
}

#[derive(Debug, Deserialize)]
struct CatalogFact {
    label: String,
    value: String,
}

fn curated_snapshot() -> perwiga_core::Result<CuratedSnapshot> {
    let snapshot: CuratedSnapshot = serde_json::from_str(include_str!("../data/heroes-iii.json"))
        .map_err(|error| {
        PerwigaError::Validation(format!("invalid bundled Heroes III snapshot: {error}"))
    })?;
    validate_snapshot(&snapshot)?;
    Ok(snapshot)
}

fn validate_snapshot(snapshot: &CuratedSnapshot) -> perwiga_core::Result<()> {
    if snapshot.schema != SNAPSHOT_SCHEMA
        || snapshot.module_id != MODULE_ID
        || snapshot.edition != EDITION
    {
        return Err(PerwigaError::Validation(
            "Heroes III snapshot identity does not match this module".to_string(),
        ));
    }
    if snapshot.sources.structure.url != VCMI_URL
        || snapshot.sources.structure.commit != VCMI_COMMIT
        || snapshot.sources.labels_and_facts.url != WIKI_API_URL
    {
        return Err(PerwigaError::Validation(
            "Heroes III snapshot source provenance changed without review".to_string(),
        ));
    }
    let expected = BTreeMap::from([
        ("artifact".to_string(), 141),
        ("creature".to_string(), 141),
        ("hero".to_string(), 156),
        ("spell".to_string(), 70),
        ("town".to_string(), 9),
    ]);
    if snapshot.catalog_scope.included_records != 517
        || snapshot.records.len() != 517
        || snapshot.catalog_scope.counts != expected
    {
        return Err(PerwigaError::Validation(
            "Heroes III snapshot does not have the reviewed Complete-edition counts".to_string(),
        ));
    }

    let mut counts = BTreeMap::<String, usize>::new();
    let mut source_keys = HashSet::new();
    let mut structural_ids = HashSet::new();
    for record in &snapshot.records {
        if !expected.contains_key(&record.entity_type) {
            return Err(PerwigaError::Validation(format!(
                "unsupported Heroes III entity type {}",
                record.entity_type
            )));
        }
        *counts.entry(record.entity_type.clone()).or_default() += 1;
        if !source_keys.insert(record.source_key.as_str()) {
            return Err(PerwigaError::Validation(format!(
                "duplicate Heroes III source key {}",
                record.source_key
            )));
        }
        if !structural_ids.insert((record.entity_type.as_str(), record.vcmi_index)) {
            return Err(PerwigaError::Validation(format!(
                "duplicate Heroes III structural ID {}:{}",
                record.entity_type, record.vcmi_index
            )));
        }
        let expected_source_key = format!(
            "homm3-complete-{}-{}",
            record.entity_type, record.vcmi_index
        );
        if record.source_key != expected_source_key
            || record.vcmi_key.trim().is_empty()
            || record.official_english_name.trim().is_empty()
            || record.official_original_name != record.official_english_name
            || !record.vcmi_source_file.starts_with("config/")
            || record.wiki_source.title.trim().is_empty()
            || !record
                .wiki_source
                .url
                .starts_with("https://heroes.thelazy.net/")
            || record.wiki_source.page_id == 0
            || record.wiki_source.revision_id == 0
            || record.wiki_source.revision_timestamp.trim().is_empty()
            || record.facts.is_empty()
            || record
                .facts
                .iter()
                .any(|fact| fact.label.trim().is_empty() || fact.value.trim().is_empty())
        {
            return Err(PerwigaError::Validation(format!(
                "invalid Heroes III catalog record {}",
                record.source_key
            )));
        }
    }
    if counts != expected {
        return Err(PerwigaError::Validation(
            "Heroes III record types do not match the reviewed counts".to_string(),
        ));
    }
    Ok(())
}

pub fn import_curated_heroes_iii(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != MODULE_ID {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Heroes of Might and Magic module"
        )));
    }

    let snapshot = curated_snapshot()?;
    let inputs = snapshot
        .records
        .iter()
        .map(|record| {
            let facts = record
                .facts
                .iter()
                .map(|fact| format!("{}: {}", fact.label, fact.value))
                .collect::<Vec<_>>()
                .join("; ");
            SourcedEntityInput {
                source_provider: SOURCE_PROVIDER.to_string(),
                source_identity: record.source_key.clone(),
                entity: EntityInput {
                    entity_type: record.entity_type.clone(),
                    official_english_name: record.official_english_name.clone(),
                    official_original_name: record.official_original_name.clone(),
                    official_vietnamese_name: record.official_vietnamese_name.clone(),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some(format!(
                        "{facts}. VCMI key: {} (index {}, {}). VCMI commit: {}. Wiki page: {} (page {}, revision {}, {}). Sources: {} and {}. Checked {}.",
                        record.vcmi_key,
                        record.vcmi_index,
                        record.vcmi_source_file,
                        VCMI_COMMIT,
                        record.wiki_source.title,
                        record.wiki_source.page_id,
                        record.wiki_source.revision_id,
                        record.wiki_source.revision_timestamp,
                        VCMI_URL,
                        record.wiki_source.url,
                        snapshot.checked_at,
                    )),
                },
                aliases: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub struct HeroesOfMightAndMagicModule;

static CAPABILITIES: &[Capability] = &[Capability::ManualContent, Capability::EntitySchema];
static ENTITY_TYPES: &[EntityTypeDefinition] = &[
    EntityTypeDefinition {
        key: "hero",
        display_name: "Hero",
        description: "A playable or campaign hero from Heroes of Might and Magic III Complete.",
    },
    EntityTypeDefinition {
        key: "npc",
        display_name: "NPC",
        description: "A named non-playable character or person.",
    },
    EntityTypeDefinition {
        key: "region",
        display_name: "Region",
        description: "A named geographic, administrative, world, or map area.",
    },
    EntityTypeDefinition {
        key: "town",
        display_name: "Town",
        description: "A playable town faction from Heroes III Complete.",
    },
    EntityTypeDefinition {
        key: "creature",
        display_name: "Creature",
        description: "A recruitable or neutral creature from Heroes III Complete.",
    },
    EntityTypeDefinition {
        key: "artifact",
        display_name: "Artifact",
        description: "An artifact or war machine represented in the Heroes III Complete catalog.",
    },
    EntityTypeDefinition {
        key: "spell",
        display_name: "Spell",
        description: "An adventure or combat spell from Heroes III Complete.",
    },
];

impl LibraryModule for HeroesOfMightAndMagicModule {
    fn id(&self) -> &'static str {
        "heroes-of-might-and-magic"
    }

    fn kind(&self) -> WorkKind {
        WorkKind::Game
    }

    fn display_name(&self) -> &'static str {
        "Heroes of Might and Magic"
    }

    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }

    fn entity_types(&self) -> &'static [EntityTypeDefinition] {
        ENTITY_TYPES
    }
}

static MODULE: HeroesOfMightAndMagicModule = HeroesOfMightAndMagicModule;

pub fn module() -> &'static dyn LibraryModule {
    &MODULE
}

pub fn register(registry: &mut ModuleRegistry) -> perwiga_core::Result<()> {
    registry.register(module())
}

#[cfg(test)]
mod tests {
    use super::*;
    use perwiga_core::Store;

    #[test]
    fn module_exposes_the_title_owned_catalog_types() {
        assert_eq!(module().id(), "heroes-of-might-and-magic");
        let keys = module()
            .entity_types()
            .iter()
            .map(|entity_type| entity_type.key)
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            ["hero", "npc", "region", "town", "creature", "artifact", "spell"]
        );
    }

    #[test]
    fn curated_snapshot_is_complete_and_contains_known_entries() {
        let snapshot = curated_snapshot().expect("valid bundled snapshot");
        assert_eq!(snapshot.records.len(), 517);
        for (entity_type, expected) in [
            ("town", 9),
            ("hero", 156),
            ("creature", 141),
            ("artifact", 141),
            ("spell", 70),
        ] {
            assert_eq!(
                snapshot
                    .records
                    .iter()
                    .filter(|record| record.entity_type == entity_type)
                    .count(),
                expected
            );
        }
        for name in [
            "Orrin",
            "Castle",
            "Pikeman",
            "Armageddon's Blade",
            "Magic Arrow",
        ] {
            assert!(
                snapshot
                    .records
                    .iter()
                    .any(|record| record.official_english_name == name),
                "missing {name}"
            );
        }
    }

    #[test]
    fn curated_import_is_additive_idempotent_and_work_owned() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(
                WorkKind::Game,
                "heroes-of-might-and-magic",
                "Heroes III Complete",
            )
            .expect("Heroes work");

        let first = import_curated_heroes_iii(&mut store, &work.id).expect("first import");
        assert_eq!(first.inserted, 517);
        assert_eq!(first.unchanged, 0);
        let second = import_curated_heroes_iii(&mut store, &work.id).expect("second import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 517);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
        assert_eq!(store.foreign_key_violations().expect("foreign keys"), 0);

        let other = store
            .insert_work(WorkKind::Game, "generic", "Other game")
            .expect("other work");
        let error = import_curated_heroes_iii(&mut store, &other.id).expect_err("ownership gate");
        assert!(error.to_string().contains("not owned"));
    }
}
