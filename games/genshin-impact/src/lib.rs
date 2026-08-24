use std::{
    collections::{BTreeMap, HashSet},
    sync::OnceLock,
};

use perwiga_core::{
    model::{AliasInput, EntityImportSummary, EntityInput, SourcedEntityInput, WikiEntity},
    Capability, EntityFacetDefinition, EntityFacetOption, EntityPresentation, EntityTypeDefinition,
    LibraryModule, ModuleRegistry, PerwigaError, Store, ThemeDefinition, WorkKind,
};
use serde::Deserialize;

pub struct GenshinImpactModule;

static CAPABILITIES: &[Capability] = &[Capability::ManualContent, Capability::EntitySchema];
const SOURCE_PROVIDER: &str = "Genshin Impact official global character directory";
const SOURCE_URL: &str = "https://genshin.hoyoverse.com/en/character/mondstadt";
static ENTITY_TYPES: &[EntityTypeDefinition] = &[
    EntityTypeDefinition {
        key: "character",
        display_name: "Character",
        description:
            "A playable character listed by the official Genshin Impact character directory.",
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
];
static REGION_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Mondstadt",
        display_name: "Mondstadt",
    },
    EntityFacetOption {
        value: "Liyue",
        display_name: "Liyue",
    },
    EntityFacetOption {
        value: "Inazuma",
        display_name: "Inazuma",
    },
    EntityFacetOption {
        value: "Sumeru",
        display_name: "Sumeru",
    },
    EntityFacetOption {
        value: "Fontaine",
        display_name: "Fontaine",
    },
    EntityFacetOption {
        value: "Natlan",
        display_name: "Natlan",
    },
    EntityFacetOption {
        value: "Snezhnaya",
        display_name: "Snezhnaya",
    },
];
static ENTITY_FACETS: &[EntityFacetDefinition] = &[EntityFacetDefinition {
    key: "region",
    display_name: "Region",
    entity_types: &["character"],
    options: REGION_OPTIONS,
}];
const THEME: ThemeDefinition = ThemeDefinition {
    background: "#101722",
    surface: "#182332",
    surface_raised: "#202d3e",
    surface_active: "#29394d",
    border: "#3a4c62",
    border_strong: "#667b94",
    text: "#f8f5eb",
    text_muted: "#c9c4b7",
    text_subtle: "#969188",
    accent: "#d8b66f",
    accent_ink: "#251b0b",
};

#[derive(Debug, Deserialize)]
struct CharacterSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: CharacterCatalogScope,
    characters: Vec<CuratedCharacter>,
}

#[derive(Debug, Deserialize)]
struct CharacterCatalogScope {
    included_records: usize,
}

#[derive(Debug, Deserialize)]
struct CuratedCharacter {
    source_key: String,
    official_content_id: u64,
    region: String,
    official_english_name: String,
    official_traditional_chinese_name: Option<String>,
    official_vietnamese_name: Option<String>,
    official_english_description: Option<String>,
    thumbnail_asset: String,
}

fn curated_snapshot() -> Result<&'static CharacterSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<CharacterSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: CharacterSnapshot =
            serde_json::from_str(include_str!("../data/characters.json"))
                .map_err(|error| format!("invalid bundled Genshin character snapshot: {error}"))?;
        validate_snapshot(&snapshot)?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn validate_snapshot(snapshot: &CharacterSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.characters.v1" {
        return Err("unexpected Genshin character snapshot schema".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.characters.len() {
        return Err("Genshin character snapshot record count does not match its metadata".into());
    }

    let allowed_regions: HashSet<&str> = REGION_OPTIONS.iter().map(|option| option.value).collect();
    let mut source_keys = HashSet::new();
    let mut content_ids = HashSet::new();
    for character in &snapshot.characters {
        let expected_key = format!("hoyoverse-content-{}", character.official_content_id);
        if character.source_key != expected_key
            || !source_keys.insert(character.source_key.as_str())
            || !content_ids.insert(character.official_content_id)
        {
            return Err(format!(
                "invalid or duplicate Genshin source identity: {}",
                character.source_key
            ));
        }
        if !allowed_regions.contains(character.region.as_str()) {
            return Err(format!("unknown Genshin region: {}", character.region));
        }
        validate_source_text(
            &character.official_english_name,
            200,
            "English name",
            &character.source_key,
        )?;
        if let Some(name) = &character.official_traditional_chinese_name {
            validate_source_text(name, 200, "Traditional Chinese name", &character.source_key)?;
        }
        if let Some(name) = &character.official_vietnamese_name {
            validate_source_text(name, 200, "Vietnamese name", &character.source_key)?;
        }
        if let Some(description) = &character.official_english_description {
            validate_source_text(
                description,
                10_000,
                "English description",
                &character.source_key,
            )?;
        }
        if character.thumbnail_asset != format!("{}.png", character.source_key)
            || character_thumbnail(&character.source_key).is_none()
        {
            return Err(format!(
                "missing trusted local thumbnail for {}",
                character.source_key
            ));
        }
    }
    Ok(())
}

fn validate_source_text(
    value: &str,
    max_characters: usize,
    field: &str,
    source_key: &str,
) -> Result<(), String> {
    if value.trim().is_empty()
        || value.chars().count() > max_characters
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(format!("invalid {field} for {source_key}"));
    }
    Ok(())
}

fn curated_character(source_key: &str) -> Option<&'static CuratedCharacter> {
    curated_snapshot()
        .ok()?
        .characters
        .iter()
        .find(|character| character.source_key == source_key)
}

fn character_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "character" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("Official HoYoverse source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_character(source_key).map(|character| character.source_key.as_str())
}

pub fn import_curated_characters(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "genshin-impact" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Genshin Impact module"
        )));
    }

    let snapshot = curated_snapshot()?;
    let inputs = snapshot
        .characters
        .iter()
        .map(|character| {
            let aliases = character
                .official_traditional_chinese_name
                .as_ref()
                .map(|name| {
                    vec![AliasInput {
                        value: name.clone(),
                        language: Some("zh-Hant".to_string()),
                        kind: "official-localization".to_string(),
                        label: Some("Official Traditional Chinese".to_string()),
                        notes: Some(format!(
                            "Official Traditional Chinese localization from {SOURCE_URL}. Checked {}. It is not labeled as the original mainland Simplified Chinese name.",
                            snapshot.checked_at
                        )),
                    }]
                })
                .unwrap_or_default();
            SourcedEntityInput {
                source_provider: SOURCE_PROVIDER.to_string(),
                source_identity: character.source_key.clone(),
                entity: EntityInput {
                    entity_type: "character".to_string(),
                    official_english_name: character.official_english_name.clone(),
                    // The confirmed global source has no Simplified Chinese locale. Repeat the
                    // English catalog anchor here instead of mislabeling Traditional Chinese.
                    official_original_name: character.official_english_name.clone(),
                    official_vietnamese_name: character.official_vietnamese_name.clone(),
                    automatic_vietnamese_translation: None,
                    english_description: character.official_english_description.clone(),
                    other_information: Some(format!(
                        "Official HoYoverse source key: {}. Official content ID: {}. Region: {}. Source: {}. Checked {}. The confirmed source does not provide the original mainland Simplified Chinese name; official Traditional Chinese is retained as a provenance-labeled alias.",
                        character.source_key,
                        character.official_content_id,
                        character.region,
                        SOURCE_URL,
                        snapshot.checked_at
                    )),
                },
                aliases,
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

impl LibraryModule for GenshinImpactModule {
    fn id(&self) -> &'static str {
        "genshin-impact"
    }

    fn kind(&self) -> WorkKind {
        WorkKind::Game
    }

    fn display_name(&self) -> &'static str {
        "Genshin Impact"
    }

    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }

    fn entity_types(&self) -> &'static [EntityTypeDefinition] {
        ENTITY_TYPES
    }

    fn entity_facets(&self) -> &'static [EntityFacetDefinition] {
        ENTITY_FACETS
    }

    fn theme(&self) -> ThemeDefinition {
        THEME
    }

    fn entity_presentation(&self, entity: &WikiEntity) -> Option<EntityPresentation> {
        let source_key = character_source_key(entity)?;
        let character = curated_character(source_key)?;
        Some(EntityPresentation {
            thumbnail_url: format!("/assets/modules/genshin-impact/characters/{source_key}.png"),
            accent_color: THEME.accent.to_string(),
            label: character.region.clone(),
            rarity: None,
            facets: BTreeMap::from([("region".to_string(), character.region.clone())]),
            facet_values: BTreeMap::new(),
        })
    }
}

static MODULE: GenshinImpactModule = GenshinImpactModule;

pub fn module() -> &'static dyn LibraryModule {
    &MODULE
}

pub fn register(registry: &mut ModuleRegistry) -> perwiga_core::Result<()> {
    registry.register(module())
}

include!(concat!(env!("OUT_DIR"), "/character_thumbnails.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use perwiga_core::Store;

    #[test]
    fn module_exposes_stable_identity_and_baseline_game_types() {
        assert_eq!(module().id(), "genshin-impact");
        assert_eq!(module().kind(), WorkKind::Game);
        assert_eq!(module().display_name(), "Genshin Impact");
        assert_eq!(
            module()
                .entity_types()
                .iter()
                .map(|entity_type| entity_type.key)
                .collect::<Vec<_>>(),
            vec!["character", "npc", "region"]
        );
    }

    #[test]
    fn curated_character_import_is_complete_idempotent_and_preserves_locale_provenance() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("Genshin work");

        let first = import_curated_characters(&mut store, &work.id).expect("first import");
        assert_eq!(first.inserted, 118);
        assert_eq!(first.unchanged, 0);
        assert_eq!(first.aliases_inserted, 118);

        let characters = store
            .list_entities(&work.id, Some("character"))
            .expect("character list");
        assert_eq!(characters.len(), 118);
        let albedo = characters
            .iter()
            .find(|character| character.official_english_name == "Albedo")
            .expect("Albedo");
        assert_eq!(albedo.official_original_name, "Albedo");
        assert_ne!(albedo.official_original_name, "阿貝多");
        assert_eq!(albedo.official_vietnamese_name.as_deref(), Some("Albedo"));
        assert!(albedo
            .other_information
            .as_deref()
            .expect("source provenance")
            .contains("Official HoYoverse source key: hoyoverse-content-104816."));
        let aliases = store.list_aliases(&albedo.id).expect("Albedo aliases");
        assert!(aliases.iter().any(|alias| {
            alias.value == "阿貝多"
                && alias.language.as_deref() == Some("zh-Hant")
                && alias.kind == "official-localization"
        }));

        let presentation = module()
            .entity_presentation(albedo)
            .expect("character presentation");
        assert_eq!(presentation.label, "Mondstadt");
        assert_eq!(presentation.rarity, None);
        assert_eq!(
            presentation.facets.get("region").map(String::as_str),
            Some("Mondstadt")
        );
        assert_eq!(
            presentation.thumbnail_url,
            "/assets/modules/genshin-impact/characters/hoyoverse-content-104816.png"
        );
        let thumbnail = character_thumbnail("hoyoverse-content-104816").expect("Albedo thumbnail");
        assert!(thumbnail.starts_with(b"\x89PNG\r\n\x1a\n"));

        let second = import_curated_characters(&mut store, &work.id).expect("second import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 118);
        assert_eq!(second.aliases_inserted, 0);
        assert_eq!(second.aliases_unchanged, 118);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
        assert_eq!(store.foreign_key_violations().expect("foreign keys"), 0);
    }

    #[test]
    fn snapshot_validation_rejects_control_characters_before_persistence() {
        let mut snapshot: CharacterSnapshot =
            serde_json::from_str(include_str!("../data/characters.json")).expect("snapshot JSON");
        snapshot.characters[0].official_vietnamese_name = Some("unsafe\0name".into());

        assert!(validate_snapshot(&snapshot)
            .expect_err("control character must be rejected")
            .contains("Vietnamese name"));
    }
}
