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
const FOUR_STAR_COLOR: &str = "#9a77c7";
const FIVE_STAR_COLOR: &str = "#d8b66f";
const COLLABORATION_COLOR: &str = "#d94b4b";
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

#[derive(Debug, Deserialize)]
struct CharacterPresentationSnapshot {
    schema: String,
    characters: Vec<CuratedCharacterPresentation>,
}

#[derive(Debug, Deserialize)]
struct CuratedCharacterPresentation {
    source_key: String,
    official_english_name: String,
    rarity: u8,
    collaboration: bool,
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

fn curated_presentation_snapshot() -> Result<&'static CharacterPresentationSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<CharacterPresentationSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: CharacterPresentationSnapshot = serde_json::from_str(include_str!(
            "../data/character-presentation.json"
        ))
        .map_err(|error| format!("invalid bundled Genshin presentation snapshot: {error}"))?;
        validate_presentation_snapshot(
            &snapshot,
            curated_snapshot().map_err(|error| error.to_string())?,
        )?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn validate_presentation_snapshot(
    snapshot: &CharacterPresentationSnapshot,
    catalog: &CharacterSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.character-presentation.v1" {
        return Err("unexpected Genshin character presentation snapshot schema".into());
    }
    if snapshot.characters.len() != catalog.characters.len() {
        return Err("Genshin character presentation coverage is incomplete".into());
    }

    let catalog_by_key = catalog
        .characters
        .iter()
        .map(|character| (character.source_key.as_str(), character))
        .collect::<BTreeMap<_, _>>();
    let mut source_keys = HashSet::new();
    for presentation in &snapshot.characters {
        if !source_keys.insert(presentation.source_key.as_str()) {
            return Err(format!(
                "duplicate Genshin character presentation: {}",
                presentation.source_key
            ));
        }
        let character = catalog_by_key
            .get(presentation.source_key.as_str())
            .ok_or_else(|| {
                format!(
                    "unknown Genshin character presentation: {}",
                    presentation.source_key
                )
            })?;
        if presentation.official_english_name != character.official_english_name {
            return Err(format!(
                "Genshin character presentation name mismatch: {}",
                presentation.source_key
            ));
        }
        if !matches!(presentation.rarity, 4 | 5) {
            return Err(format!(
                "invalid Genshin character rarity for {}",
                presentation.source_key
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

fn curated_character_presentation(
    source_key: &str,
) -> Option<&'static CuratedCharacterPresentation> {
    curated_presentation_snapshot()
        .ok()?
        .characters
        .iter()
        .find(|character| character.source_key == source_key)
}

fn character_accent(presentation: &CuratedCharacterPresentation) -> &'static str {
    if presentation.collaboration {
        return COLLABORATION_COLOR;
    }
    match presentation.rarity {
        4 => FOUR_STAR_COLOR,
        5 => FIVE_STAR_COLOR,
        _ => THEME.border_strong,
    }
}

fn region_asset_key(region: &str) -> Option<&'static str> {
    match region {
        "Mondstadt" => Some("mondstadt"),
        "Liyue" => Some("liyue"),
        "Inazuma" => Some("inazuma"),
        "Sumeru" => Some("sumeru"),
        "Fontaine" => Some("fontaine"),
        "Natlan" => Some("natlan"),
        "Snezhnaya" => Some("snezhnaya"),
        _ => None,
    }
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
        let presentation = curated_character_presentation(source_key)?;
        let region_asset_key = region_asset_key(&character.region)?;
        region_icon(region_asset_key)?;
        Some(EntityPresentation {
            thumbnail_url: format!("/assets/modules/genshin-impact/characters/{source_key}.png"),
            accent_color: character_accent(presentation).to_string(),
            context_label: Some(character.region.clone()),
            context_icon_url: Some(format!(
                "/assets/modules/genshin-impact/regions/{region_asset_key}.webp"
            )),
            label: format!("{}★", presentation.rarity),
            rarity: Some(presentation.rarity),
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
include!(concat!(env!("OUT_DIR"), "/region_icons.rs"));

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
        assert_eq!(presentation.label, "5★");
        assert_eq!(presentation.rarity, Some(5));
        assert_eq!(presentation.accent_color, "#d8b66f");
        assert_eq!(presentation.context_label.as_deref(), Some("Mondstadt"));
        assert_eq!(
            presentation.context_icon_url.as_deref(),
            Some("/assets/modules/genshin-impact/regions/mondstadt.webp")
        );
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
        let region_icon_bytes = region_icon("mondstadt").expect("Mondstadt region icon");
        assert!(region_icon_bytes.starts_with(b"RIFF"));
        assert_eq!(&region_icon_bytes[8..12], b"WEBP");

        let aloy = characters
            .iter()
            .find(|character| character.official_english_name == "Aloy")
            .expect("Aloy");
        let aloy_presentation = module()
            .entity_presentation(aloy)
            .expect("Aloy presentation");
        assert_eq!(aloy_presentation.label, "5★");
        assert_eq!(aloy_presentation.rarity, Some(5));
        assert_eq!(aloy_presentation.accent_color, "#d94b4b");

        let amber = characters
            .iter()
            .find(|character| character.official_english_name == "Amber")
            .expect("Amber");
        let amber_presentation = module()
            .entity_presentation(amber)
            .expect("Amber presentation");
        assert_eq!(amber_presentation.label, "4★");
        assert_eq!(amber_presentation.rarity, Some(4));
        assert_eq!(amber_presentation.accent_color, "#9a77c7");

        let second = import_curated_characters(&mut store, &work.id).expect("second import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 118);
        assert_eq!(second.aliases_inserted, 0);
        assert_eq!(second.aliases_unchanged, 118);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
        assert_eq!(store.foreign_key_violations().expect("foreign keys"), 0);
    }

    #[test]
    fn every_known_region_has_a_local_webp_emblem() {
        for region in REGION_OPTIONS {
            let asset_key = region_asset_key(region.value).expect("known region asset key");
            let bytes = region_icon(asset_key).expect("bundled region emblem");
            assert!(bytes.starts_with(b"RIFF"), "{}", region.value);
            assert_eq!(&bytes[8..12], b"WEBP", "{}", region.value);
        }
        assert!(region_asset_key("Khaenri'ah").is_none());
        assert!(region_icon("../mondstadt").is_none());
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

    #[test]
    fn presentation_snapshot_requires_complete_four_or_five_star_classification() {
        let catalog: CharacterSnapshot =
            serde_json::from_str(include_str!("../data/characters.json")).expect("catalog JSON");
        let mut presentation: CharacterPresentationSnapshot =
            serde_json::from_str(include_str!("../data/character-presentation.json"))
                .expect("presentation JSON");

        presentation.characters[0].rarity = 3;
        assert!(validate_presentation_snapshot(&presentation, &catalog)
            .expect_err("unsupported rarity must be rejected")
            .contains("invalid Genshin character rarity"));

        presentation.characters[0].rarity = 5;
        presentation.characters.pop();
        assert!(validate_presentation_snapshot(&presentation, &catalog)
            .expect_err("incomplete coverage must be rejected")
            .contains("coverage is incomplete"));
    }
}
