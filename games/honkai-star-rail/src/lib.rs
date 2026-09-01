use std::{
    collections::{BTreeMap, HashSet},
    sync::OnceLock,
};

use perwiga_core::{
    model::{AliasInput, EntityImportSummary, EntityInput, SourcedEntityInput, WikiEntity},
    Capability, EntityFacetDefinition, EntityFacetOption, EntityPresentation, EntityTypeDefinition,
    LibraryModule, ModuleRegistry, PerwigaError, Store, ThemeDefinition, WorkKind,
};
use serde::{de::DeserializeOwned, Deserialize};

pub struct HonkaiStarRailModule;

static CAPABILITIES: &[Capability] = &[Capability::ManualContent, Capability::EntitySchema];
const SOURCE_PROVIDER: &str = "StarRailRes game-data extracts via Dimbreath/StarRailData";
const NPC_SOURCE_PROVIDER: &str = "HoYoWiki via HoYoLAB";
const SOURCE_URL: &str = "https://github.com/Mar-7th/StarRailRes";
const ASSET_BASE_URL: &str = "https://vizualabstract.github.io/StarRailStaticAPI/assets";
const THREE_STAR_COLOR: &str = "#6682a9";
const FOUR_STAR_COLOR: &str = "#9d79c7";
const FIVE_STAR_COLOR: &str = "#d8b66f";

static ENTITY_TYPES: &[EntityTypeDefinition] = &[
    EntityTypeDefinition {
        key: "character",
        display_name: "Character",
        description: "A playable character or playable path variant from the Star Rail catalog.",
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
        key: "light-cone",
        display_name: "Light Cone",
        description: "An equippable Light Cone from the Star Rail catalog.",
    },
    EntityTypeDefinition {
        key: "relic-set",
        display_name: "Relic Set",
        description: "A named collection of equippable Relics with set effects.",
    },
    EntityTypeDefinition {
        key: "relic",
        display_name: "Relic",
        description: "One named Relic piece belonging to a Relic Set.",
    },
    EntityTypeDefinition {
        key: "path",
        display_name: "Path",
        description: "A combat Path used by Characters and Light Cones.",
    },
    EntityTypeDefinition {
        key: "element",
        display_name: "Element",
        description: "A combat element used by Characters.",
    },
];

static PATH_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Destruction",
        display_name: "Destruction",
    },
    EntityFacetOption {
        value: "The Hunt",
        display_name: "The Hunt",
    },
    EntityFacetOption {
        value: "Erudition",
        display_name: "Erudition",
    },
    EntityFacetOption {
        value: "Harmony",
        display_name: "Harmony",
    },
    EntityFacetOption {
        value: "Nihility",
        display_name: "Nihility",
    },
    EntityFacetOption {
        value: "Preservation",
        display_name: "Preservation",
    },
    EntityFacetOption {
        value: "Abundance",
        display_name: "Abundance",
    },
    EntityFacetOption {
        value: "Remembrance",
        display_name: "Remembrance",
    },
    EntityFacetOption {
        value: "Elation",
        display_name: "Elation",
    },
];
static ELEMENT_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Physical",
        display_name: "Physical",
    },
    EntityFacetOption {
        value: "Fire",
        display_name: "Fire",
    },
    EntityFacetOption {
        value: "Ice",
        display_name: "Ice",
    },
    EntityFacetOption {
        value: "Lightning",
        display_name: "Lightning",
    },
    EntityFacetOption {
        value: "Wind",
        display_name: "Wind",
    },
    EntityFacetOption {
        value: "Quantum",
        display_name: "Quantum",
    },
    EntityFacetOption {
        value: "Imaginary",
        display_name: "Imaginary",
    },
];
static RELIC_SLOT_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Head",
        display_name: "Head",
    },
    EntityFacetOption {
        value: "Hands",
        display_name: "Hands",
    },
    EntityFacetOption {
        value: "Body",
        display_name: "Body",
    },
    EntityFacetOption {
        value: "Feet",
        display_name: "Feet",
    },
    EntityFacetOption {
        value: "Planar Sphere",
        display_name: "Planar Sphere",
    },
    EntityFacetOption {
        value: "Link Rope",
        display_name: "Link Rope",
    },
];
static ENTITY_FACETS: &[EntityFacetDefinition] = &[
    EntityFacetDefinition {
        key: "path",
        display_name: "Path",
        entity_types: &["character", "light-cone"],
        options: PATH_OPTIONS,
    },
    EntityFacetDefinition {
        key: "element",
        display_name: "Element",
        entity_types: &["character"],
        options: ELEMENT_OPTIONS,
    },
    EntityFacetDefinition {
        key: "relic_slot",
        display_name: "Relic slot",
        entity_types: &["relic"],
        options: RELIC_SLOT_OPTIONS,
    },
];
const THEME: ThemeDefinition = ThemeDefinition {
    background: "#0e1420",
    surface: "#151e2d",
    surface_raised: "#1d2a3e",
    surface_active: "#263751",
    border: "#3a4e6a",
    border_strong: "#607a9d",
    text: "#f2f5fb",
    text_muted: "#c3ccda",
    text_subtle: "#8f9db2",
    accent: "#9cc9ff",
    accent_ink: "#0c1725",
};

#[derive(Debug, Deserialize)]
struct SourceMetadata {
    commit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CatalogScope {
    included_records: usize,
}

#[derive(Debug, Deserialize)]
struct CharacterSnapshot {
    schema: String,
    checked_at: String,
    source: SourceMetadata,
    catalog_scope: CatalogScope,
    characters: Vec<CharacterRecord>,
}

#[derive(Debug, Deserialize)]
struct CharacterRecord {
    source_key: String,
    game_data_id: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    rarity: u8,
    path_english_name: String,
    path_vietnamese_name: String,
    element_key: String,
    element_english_name: String,
    element_vietnamese_name: String,
    max_sp: Option<u16>,
    thumbnail_path: String,
    thumbnail_url: String,
}

#[derive(Debug, Deserialize)]
struct CharacterHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: CatalogScope,
    characters: Vec<CharacterHanVietRecord>,
}

#[derive(Debug, Deserialize)]
struct CharacterHanVietRecord {
    source_key: String,
    han_viet_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LightConeSnapshot {
    schema: String,
    checked_at: String,
    source: SourceMetadata,
    catalog_scope: CatalogScope,
    light_cones: Vec<LightConeRecord>,
}

#[derive(Debug, Deserialize)]
struct LightConeRecord {
    source_key: String,
    game_data_id: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    official_english_description: Option<String>,
    rarity: u8,
    path_english_name: String,
    path_vietnamese_name: String,
    thumbnail_path: String,
}

#[derive(Debug, Deserialize)]
struct LightConeHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: LightConeHanVietScope,
    light_cones: Vec<LightConeHanVietRecord>,
}

#[derive(Debug, Deserialize)]
struct LightConeHanVietScope {
    source_records: usize,
    alias_records: usize,
}

#[derive(Debug, Deserialize)]
struct LightConeHanVietRecord {
    source_key: String,
    game_data_id: String,
    han_viet_name: String,
    confidence: String,
}

#[derive(Debug, Deserialize)]
struct RelicSetSnapshot {
    schema: String,
    checked_at: String,
    source: SourceMetadata,
    catalog_scope: CatalogScope,
    relic_sets: Vec<RelicSetRecord>,
}

#[derive(Debug, Deserialize)]
struct RelicSetRecord {
    source_key: String,
    game_data_id: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    descriptions: LocalizedDescriptions,
    icon_path: String,
}

#[derive(Debug, Deserialize)]
struct LocalizedDescriptions {
    en: Vec<String>,
    vi: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RelicSnapshot {
    schema: String,
    checked_at: String,
    source: SourceMetadata,
    catalog_scope: CatalogScope,
    relics: Vec<RelicRecord>,
}

#[derive(Debug, Deserialize)]
struct RelicRecord {
    source_key: String,
    game_data_id: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    relic_set_source_key: String,
    slot: String,
    rarity: u8,
    max_level: u8,
    icon_path: String,
}

#[derive(Debug, Deserialize)]
struct RelicHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: RelicHanVietScope,
    relic_sets: Vec<RelicSetHanVietRecord>,
    relics: Vec<RelicHanVietRecord>,
}

#[derive(Debug, Deserialize)]
struct RelicHanVietScope {
    relic_sets: RelicHanVietSubscope,
    relic_pieces: RelicHanVietSubscope,
}

#[derive(Debug, Deserialize)]
struct RelicHanVietSubscope {
    included_records: usize,
    generated_aliases: usize,
}

#[derive(Debug, Deserialize)]
struct RelicSetHanVietRecord {
    source_key: String,
    generated_han_viet_aliases: Vec<RelicHanVietAlias>,
}

#[derive(Debug, Deserialize)]
struct RelicHanVietRecord {
    source_key: String,
    generated_han_viet_aliases: Vec<RelicHanVietAlias>,
}

#[derive(Debug, Deserialize)]
struct RelicHanVietAlias {
    value: String,
    language: String,
    status: String,
    confidence: String,
}

#[derive(Debug, Deserialize)]
struct PathSnapshot {
    schema: String,
    checked_at: String,
    source: SourceMetadata,
    catalog_scope: CatalogScope,
    paths: Vec<PathRecord>,
}

#[derive(Debug, Deserialize)]
struct PathRecord {
    source_key: String,
    game_data_id: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    official_english_description: Option<String>,
    icon_path: String,
}

#[derive(Debug, Deserialize)]
struct ElementSnapshot {
    schema: String,
    checked_at: String,
    source: SourceMetadata,
    catalog_scope: CatalogScope,
    elements: Vec<ElementRecord>,
}

#[derive(Debug, Deserialize)]
struct ElementRecord {
    source_key: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    official_english_description: Option<String>,
    source_key_name: String,
    color: Option<String>,
    icon_path: String,
}

#[derive(Debug, Deserialize)]
struct NpcSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: NpcCatalogScope,
    npcs: Vec<NpcRecord>,
}

#[derive(Debug, Deserialize)]
struct NpcCatalogScope {
    included_records: usize,
    english_joined_records: usize,
}

#[derive(Debug, Deserialize)]
struct NpcRecord {
    source_key: String,
    hoyowiki_entry_page_id: String,
    page_url: String,
    official_english_name: Option<String>,
    official_simplified_chinese_name: String,
}

#[derive(Debug, Deserialize)]
struct NpcHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: NpcHanVietCatalogScope,
    npcs: Vec<NpcHanVietRecord>,
}

#[derive(Debug, Deserialize)]
struct NpcHanVietCatalogScope {
    included_records: usize,
    output_records: usize,
}

#[derive(Debug, Deserialize)]
struct NpcHanVietRecord {
    source_key: String,
    official_simplified_chinese_name: String,
    han_viet_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TaxonomyHanVietSnapshot {
    schema: String,
    checked_at: String,
    paths: Vec<TaxonomyHanVietRecord>,
    elements: Vec<TaxonomyHanVietRecord>,
}

#[derive(Debug, Deserialize)]
struct TaxonomyHanVietRecord {
    source_key: String,
    generated_aliases: Vec<GeneratedHanVietAlias>,
}

#[derive(Debug, Deserialize)]
struct GeneratedHanVietAlias {
    value: String,
    language: String,
    kind: String,
    official: bool,
    confidence: String,
}

fn source_commit(source: &SourceMetadata) -> &str {
    source.commit.as_deref().unwrap_or("unversioned")
}

fn required_snapshot<T: DeserializeOwned>(
    cell: &'static OnceLock<Result<T, String>>,
    file_name: &str,
    schema: &str,
    validate: impl FnOnce(&T) -> Result<(), String>,
) -> Result<&'static T, PerwigaError> {
    match cell.get_or_init(|| {
        let value: T = serde_json::from_str(file_name)
            .map_err(|error| format!("invalid bundled Star Rail snapshot: {error}"))?;
        validate(&value)?;
        Ok(value)
    }) {
        Ok(value) => Ok(value),
        Err(message) => Err(PerwigaError::Validation(format!("{schema}: {message}"))),
    }
}

fn validate_name(value: &str, field: &str, source_key: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{source_key} has an empty {field}"))
    } else {
        Ok(())
    }
}

fn validate_generated_alias(alias: &GeneratedHanVietAlias, source_key: &str) -> Result<(), String> {
    if alias.language != "vi"
        || alias.kind != "han-viet"
        || alias.official
        || alias.confidence != "E"
    {
        return Err(format!("invalid Hán-Việt alias metadata for {source_key}"));
    }
    validate_name(&alias.value, "Hán-Việt alias", source_key)
}

fn validate_character_snapshot(snapshot: &CharacterSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.characters.v1" {
        return Err("unexpected character snapshot schema".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.characters.len() {
        return Err("character snapshot count does not match metadata".into());
    }
    let mut keys = HashSet::new();
    for character in &snapshot.characters {
        if !keys.insert(character.source_key.as_str())
            || character.source_key != format!("starrailres-character-{}", character.game_data_id)
        {
            return Err(format!(
                "invalid or duplicate Character source key: {}",
                character.source_key
            ));
        }
        for (name, field) in [
            (&character.official_english_name, "English name"),
            (
                &character.official_simplified_chinese_name,
                "Simplified Chinese name",
            ),
            (
                &character.official_traditional_chinese_name,
                "Traditional Chinese name",
            ),
            (&character.official_vietnamese_name, "Vietnamese name"),
            (&character.path_english_name, "Path"),
            (&character.element_english_name, "Element"),
        ] {
            validate_name(name, field, &character.source_key)?;
        }
        if !matches!(character.rarity, 4 | 5) {
            return Err(format!(
                "unsupported Character rarity: {}",
                character.source_key
            ));
        }
    }
    Ok(())
}

fn validate_character_han_viet_snapshot(
    snapshot: &CharacterHanVietSnapshot,
    catalog: &CharacterSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.character-han-viet.v1"
        || snapshot.catalog_scope.included_records != snapshot.characters.len()
        || snapshot.characters.len() != catalog.characters.len()
    {
        return Err("Character Hán-Việt snapshot coverage is incomplete".into());
    }
    let catalog_keys = catalog
        .characters
        .iter()
        .map(|character| character.source_key.as_str())
        .collect::<HashSet<_>>();
    let mut keys = HashSet::new();
    for character in &snapshot.characters {
        if !keys.insert(character.source_key.as_str())
            || !catalog_keys.contains(character.source_key.as_str())
        {
            return Err(format!(
                "invalid or duplicate Character Hán-Việt source key: {}",
                character.source_key
            ));
        }
        if let Some(alias) = &character.han_viet_name {
            validate_name(alias, "Hán-Việt alias", &character.source_key)?;
        }
    }
    Ok(())
}

fn validate_light_cone_snapshot(snapshot: &LightConeSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.light-cones.v1" {
        return Err("unexpected Light Cone snapshot schema".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.light_cones.len() {
        return Err("Light Cone snapshot count does not match metadata".into());
    }
    let mut keys = HashSet::new();
    for light_cone in &snapshot.light_cones {
        if !keys.insert(light_cone.source_key.as_str())
            || light_cone.source_key
                != format!("starrailres-light-cone-{}", light_cone.game_data_id)
        {
            return Err(format!(
                "invalid or duplicate Light Cone source key: {}",
                light_cone.source_key
            ));
        }
        validate_name(
            &light_cone.official_english_name,
            "English name",
            &light_cone.source_key,
        )?;
        validate_name(
            &light_cone.official_simplified_chinese_name,
            "Simplified Chinese name",
            &light_cone.source_key,
        )?;
        validate_name(
            &light_cone.official_traditional_chinese_name,
            "Traditional Chinese name",
            &light_cone.source_key,
        )?;
        validate_name(
            &light_cone.official_vietnamese_name,
            "Vietnamese name",
            &light_cone.source_key,
        )?;
        if !matches!(light_cone.rarity, 3 | 4 | 5) {
            return Err(format!(
                "unsupported Light Cone rarity: {}",
                light_cone.source_key
            ));
        }
    }
    Ok(())
}

fn validate_light_cone_han_viet_snapshot(
    snapshot: &LightConeHanVietSnapshot,
    catalog: &LightConeSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.light-cone-han-viet.v1"
        || snapshot.catalog_scope.source_records != catalog.light_cones.len()
        || snapshot.catalog_scope.alias_records != snapshot.light_cones.len()
        || snapshot.light_cones.len() != catalog.light_cones.len()
    {
        return Err("Light Cone Hán-Việt snapshot coverage is incomplete".into());
    }
    let catalog_by_key = catalog
        .light_cones
        .iter()
        .map(|light_cone| {
            (
                light_cone.source_key.as_str(),
                light_cone.game_data_id.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut keys = HashSet::new();
    for light_cone in &snapshot.light_cones {
        if !keys.insert(light_cone.source_key.as_str())
            || catalog_by_key.get(light_cone.source_key.as_str())
                != Some(&light_cone.game_data_id.as_str())
            || light_cone.confidence != "E"
        {
            return Err(format!(
                "invalid Light Cone Hán-Việt alias: {}",
                light_cone.source_key
            ));
        }
        validate_name(
            &light_cone.han_viet_name,
            "Hán-Việt alias",
            &light_cone.source_key,
        )?;
    }
    Ok(())
}

fn validate_relic_set_snapshot(snapshot: &RelicSetSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.relic-sets.v1" {
        return Err("unexpected Relic Set snapshot schema".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.relic_sets.len() {
        return Err("Relic Set snapshot count does not match metadata".into());
    }
    let mut keys = HashSet::new();
    for relic_set in &snapshot.relic_sets {
        if !keys.insert(relic_set.source_key.as_str())
            || relic_set.source_key != format!("starrail-relic-set-{}", relic_set.game_data_id)
        {
            return Err(format!(
                "invalid or duplicate Relic Set source key: {}",
                relic_set.source_key
            ));
        }
        for name in [
            &relic_set.official_english_name,
            &relic_set.official_simplified_chinese_name,
            &relic_set.official_traditional_chinese_name,
            &relic_set.official_vietnamese_name,
        ] {
            validate_name(name, "name", &relic_set.source_key)?;
        }
    }
    Ok(())
}

fn validate_relic_snapshot(snapshot: &RelicSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.relics.v1" {
        return Err("unexpected Relic snapshot schema".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.relics.len() {
        return Err("Relic snapshot count does not match metadata".into());
    }
    let mut keys = HashSet::new();
    for relic in &snapshot.relics {
        if !keys.insert(relic.source_key.as_str())
            || relic.source_key != format!("starrail-relic-{}", relic.game_data_id)
        {
            return Err(format!(
                "invalid or duplicate Relic source key: {}",
                relic.source_key
            ));
        }
        validate_name(
            &relic.official_english_name,
            "English name",
            &relic.source_key,
        )?;
        if !matches!(relic.rarity, 2 | 3 | 4 | 5) {
            return Err(format!("unsupported Relic rarity: {}", relic.source_key));
        }
    }
    Ok(())
}

fn validate_relic_han_viet_snapshot(
    snapshot: &RelicHanVietSnapshot,
    relic_sets: &RelicSetSnapshot,
    relics: &RelicSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.relic-han-viet.v1"
        || snapshot.catalog_scope.relic_sets.included_records != relic_sets.relic_sets.len()
        || snapshot.catalog_scope.relic_sets.generated_aliases != snapshot.relic_sets.len()
        || snapshot.catalog_scope.relic_pieces.included_records != relics.relics.len()
        || snapshot.catalog_scope.relic_pieces.generated_aliases != snapshot.relics.len()
        || snapshot.relic_sets.len() != relic_sets.relic_sets.len()
        || snapshot.relics.len() != relics.relics.len()
    {
        return Err("Relic Hán-Việt snapshot coverage is incomplete".into());
    }
    for (records, expected, kind) in [
        (
            snapshot
                .relic_sets
                .iter()
                .map(|record| {
                    (
                        record.source_key.as_str(),
                        &record.generated_han_viet_aliases,
                    )
                })
                .collect::<Vec<_>>(),
            relic_sets
                .relic_sets
                .iter()
                .map(|record| record.source_key.as_str())
                .collect::<HashSet<_>>(),
            "Relic Set",
        ),
        (
            snapshot
                .relics
                .iter()
                .map(|record| {
                    (
                        record.source_key.as_str(),
                        &record.generated_han_viet_aliases,
                    )
                })
                .collect::<Vec<_>>(),
            relics
                .relics
                .iter()
                .map(|record| record.source_key.as_str())
                .collect::<HashSet<_>>(),
            "Relic",
        ),
    ] {
        let mut keys = HashSet::new();
        for (source_key, aliases) in records {
            if !keys.insert(source_key) || !expected.contains(source_key) || aliases.is_empty() {
                return Err(format!(
                    "invalid or missing {kind} Hán-Việt alias: {source_key}"
                ));
            }
            for alias in aliases {
                if alias.language != "vi-Han-Viet"
                    || alias.status != "generated"
                    || !matches!(alias.confidence.as_str(), "high" | "medium" | "low")
                {
                    return Err(format!("invalid {kind} alias metadata: {source_key}"));
                }
                validate_name(&alias.value, "Hán-Việt alias", source_key)?;
            }
        }
    }
    Ok(())
}

fn validate_path_snapshot(snapshot: &PathSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.paths.v1" {
        return Err("unexpected Path snapshot schema".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.paths.len() {
        return Err("Path snapshot count does not match metadata".into());
    }
    Ok(())
}

fn validate_element_snapshot(snapshot: &ElementSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.elements.v1" {
        return Err("unexpected Element snapshot schema".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.elements.len() {
        return Err("Element snapshot count does not match metadata".into());
    }
    Ok(())
}

fn validate_npc_snapshot(snapshot: &NpcSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.npcs.v1"
        || snapshot.catalog_scope.included_records != snapshot.npcs.len()
        || snapshot.catalog_scope.english_joined_records > snapshot.npcs.len()
    {
        return Err("NPC snapshot scope is inconsistent".into());
    }
    let mut keys = HashSet::new();
    for npc in &snapshot.npcs {
        if !keys.insert(npc.source_key.as_str())
            || npc.source_key != format!("hoyowiki-npc-{}", npc.hoyowiki_entry_page_id)
            || !npc
                .hoyowiki_entry_page_id
                .chars()
                .all(|value| value.is_ascii_digit())
        {
            return Err(format!(
                "invalid or duplicate NPC source key: {}",
                npc.source_key
            ));
        }
        validate_name(
            &npc.official_simplified_chinese_name,
            "Simplified Chinese name",
            &npc.source_key,
        )?;
        validate_name(&npc.page_url, "HoYoWiki page URL", &npc.source_key)?;
    }
    Ok(())
}

fn validate_npc_han_viet_snapshot(
    snapshot: &NpcHanVietSnapshot,
    catalog: &NpcSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.npc-han-viet.v1"
        || snapshot.catalog_scope.included_records != catalog.npcs.len()
        || snapshot.catalog_scope.output_records != snapshot.npcs.len()
        || snapshot.npcs.len() != catalog.npcs.len()
    {
        return Err("NPC Hán-Việt snapshot coverage is incomplete".into());
    }
    let catalog_by_key = catalog
        .npcs
        .iter()
        .map(|npc| {
            (
                npc.source_key.as_str(),
                npc.official_simplified_chinese_name.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut keys = HashSet::new();
    for npc in &snapshot.npcs {
        if !keys.insert(npc.source_key.as_str())
            || catalog_by_key.get(npc.source_key.as_str())
                != Some(&npc.official_simplified_chinese_name.as_str())
        {
            return Err(format!(
                "invalid NPC Hán-Việt source key: {}",
                npc.source_key
            ));
        }
        if let Some(alias) = &npc.han_viet_name {
            validate_name(alias, "Hán-Việt alias", &npc.source_key)?;
        }
    }
    Ok(())
}

fn validate_taxonomy_han_viet_snapshot(
    snapshot: &TaxonomyHanVietSnapshot,
    paths: &PathSnapshot,
    elements: &ElementSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.honkai-star-rail.taxonomy-han-viet.v1"
        || snapshot.paths.len() != paths.paths.len()
        || snapshot.elements.len() != elements.elements.len()
    {
        return Err("Path/Element Hán-Việt snapshot coverage is incomplete".into());
    }
    for (records, expected, kind) in [
        (
            &snapshot.paths,
            paths
                .paths
                .iter()
                .map(|path| path.source_key.as_str())
                .collect::<HashSet<_>>(),
            "Path",
        ),
        (
            &snapshot.elements,
            elements
                .elements
                .iter()
                .map(|element| element.source_key.as_str())
                .collect::<HashSet<_>>(),
            "Element",
        ),
    ] {
        let mut keys = HashSet::new();
        for record in records {
            if !keys.insert(record.source_key.as_str())
                || !expected.contains(record.source_key.as_str())
            {
                return Err(format!("invalid or duplicate {kind} Hán-Việt source key"));
            }
            if record.generated_aliases.is_empty() {
                return Err(format!(
                    "missing {kind} Hán-Việt alias: {}",
                    record.source_key
                ));
            }
            for alias in &record.generated_aliases {
                validate_generated_alias(alias, &record.source_key)?;
            }
        }
    }
    Ok(())
}

fn character_snapshot() -> Result<&'static CharacterSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<CharacterSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/characters.json"),
        "characters",
        validate_character_snapshot,
    )
}

fn character_han_viet_snapshot() -> Result<&'static CharacterHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<CharacterHanVietSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/character-han-viet.json"),
        "character-han-viet",
        |snapshot| {
            validate_character_han_viet_snapshot(
                snapshot,
                character_snapshot().map_err(|error| error.to_string())?,
            )
        },
    )
}

fn light_cone_snapshot() -> Result<&'static LightConeSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<LightConeSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/light-cones.json"),
        "light-cones",
        validate_light_cone_snapshot,
    )
}

fn light_cone_han_viet_snapshot() -> Result<&'static LightConeHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<LightConeHanVietSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/light-cone-han-viet.json"),
        "light-cone-han-viet",
        |snapshot| {
            validate_light_cone_han_viet_snapshot(
                snapshot,
                light_cone_snapshot().map_err(|error| error.to_string())?,
            )
        },
    )
}

fn relic_set_snapshot() -> Result<&'static RelicSetSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<RelicSetSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/relic-sets.json"),
        "relic-sets",
        validate_relic_set_snapshot,
    )
}

fn relic_snapshot() -> Result<&'static RelicSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<RelicSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/relics.json"),
        "relics",
        validate_relic_snapshot,
    )
}

fn relic_han_viet_snapshot() -> Result<&'static RelicHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<RelicHanVietSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/relic-han-viet.json"),
        "relic-han-viet",
        |snapshot| {
            validate_relic_han_viet_snapshot(
                snapshot,
                relic_set_snapshot().map_err(|error| error.to_string())?,
                relic_snapshot().map_err(|error| error.to_string())?,
            )
        },
    )
}

fn path_snapshot() -> Result<&'static PathSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<PathSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/paths.json"),
        "paths",
        validate_path_snapshot,
    )
}

fn element_snapshot() -> Result<&'static ElementSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<ElementSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/elements.json"),
        "elements",
        validate_element_snapshot,
    )
}

fn npc_snapshot() -> Result<&'static NpcSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<NpcSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/npcs.json"),
        "npcs",
        validate_npc_snapshot,
    )
}

fn npc_han_viet_snapshot() -> Result<&'static NpcHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<NpcHanVietSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/npc-han-viet.json"),
        "npc-han-viet",
        |snapshot| {
            validate_npc_han_viet_snapshot(
                snapshot,
                npc_snapshot().map_err(|error| error.to_string())?,
            )
        },
    )
}

fn taxonomy_han_viet_snapshot() -> Result<&'static TaxonomyHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<TaxonomyHanVietSnapshot, String>> = OnceLock::new();
    required_snapshot(
        &SNAPSHOT,
        include_str!("../data/taxonomy-han-viet.json"),
        "taxonomy-han-viet",
        |snapshot| {
            validate_taxonomy_han_viet_snapshot(
                snapshot,
                path_snapshot().map_err(|error| error.to_string())?,
                element_snapshot().map_err(|error| error.to_string())?,
            )
        },
    )
}

fn ensure_work(store: &Store, work_id: &str) -> perwiga_core::Result<()> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "honkai-star-rail" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Honkai: Star Rail module"
        )));
    }
    Ok(())
}

fn official_alias(value: &str, checked_at: &str) -> AliasInput {
    AliasInput {
        value: value.to_string(),
        language: Some("zh-Hant".to_string()),
        kind: "official-localization".to_string(),
        label: Some("Official Traditional Chinese".to_string()),
        notes: Some(format!(
            "Traditional Chinese in-game localization from StarRailRes. Checked {checked_at}."
        )),
    }
}

fn generated_han_viet_alias(
    alias: &GeneratedHanVietAlias,
    source_kind: &str,
    checked_at: &str,
) -> AliasInput {
    generated_han_viet_value(&alias.value, &alias.confidence, source_kind, checked_at)
}

fn generated_han_viet_value(
    value: &str,
    confidence: &str,
    source_kind: &str,
    checked_at: &str,
) -> AliasInput {
    AliasInput {
        value: value.to_string(),
        language: Some("vi".to_string()),
        kind: "han-viet".to_string(),
        label: Some("Hán-Việt (generated)".to_string()),
        notes: Some(format!(
            "Generated Hán-Việt search alias for the {source_kind} from Chinese character readings; not an official Vietnamese localization. Confidence {}. Checked {checked_at}. Source: https://hvdic.thivien.net/transcript.php#trans.",
            confidence
        )),
    }
}

fn marker(source_key: &str, source_kind: &str, commit: &str, checked_at: &str) -> String {
    format!(
        "StarRailRes source key: {source_key}. Catalog: {source_kind}. Source: {SOURCE_URL} at commit {commit}. Checked {checked_at}."
    )
}

fn join_description(values: &[String]) -> Option<String> {
    let value = values.join("\n\n");
    (!value.trim().is_empty()).then_some(value)
}

pub fn import_curated_characters(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    ensure_work(store, work_id)?;
    let snapshot = character_snapshot()?;
    let han_viet_snapshot = character_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .characters
        .iter()
        .map(|character| (character.source_key.as_str(), character))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .characters
        .iter()
        .map(|character| {
            let han_viet = han_viet_by_key
                .get(character.source_key.as_str())
                .expect("validated Character Hán-Việt coverage");
            let mut aliases = vec![official_alias(
                &character.official_traditional_chinese_name,
                &snapshot.checked_at,
            )];
            if let Some(value) = &han_viet.han_viet_name {
                aliases.push(generated_han_viet_value(
                    value,
                    "E",
                    "Character",
                    &han_viet_snapshot.checked_at,
                ));
            }
            SourcedEntityInput {
                source_provider: SOURCE_PROVIDER.to_string(),
                source_identity: character.source_key.clone(),
                entity: EntityInput {
                    entity_type: "character".to_string(),
                    official_english_name: character.official_english_name.clone(),
                    official_original_name: character.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: Some(character.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some(format!(
                        "{}. Thumbnail source URL: {}. StarRailRes portrait path: {}. Path: {} ({}). Element: {} ({}). Rarity: {}★. Game data ID: {}. Max Energy: {}. The basic StarRailRes character endpoint does not supply a biography; no description was inferred.",
                        marker(&character.source_key, "character", source_commit(&snapshot.source), &snapshot.checked_at),
                        character.thumbnail_url,
                        character.thumbnail_path,
                        character.path_english_name,
                        character.path_vietnamese_name,
                        character.element_english_name,
                        character.element_vietnamese_name,
                        character.rarity,
                        character.game_data_id,
                        character.max_sp.map_or_else(|| "not supplied".to_string(), |value| value.to_string()),
                    )),
                },
                aliases,
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_npcs(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    ensure_work(store, work_id)?;
    let snapshot = npc_snapshot()?;
    let han_viet_snapshot = npc_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .npcs
        .iter()
        .map(|npc| (npc.source_key.as_str(), npc))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .npcs
        .iter()
        .filter_map(|npc| {
            let official_english_name = npc.official_english_name.as_ref()?;
            let han_viet = han_viet_by_key
                .get(npc.source_key.as_str())
                .expect("validated NPC Hán-Việt coverage");
            let mut aliases = Vec::new();
            if let Some(value) = &han_viet.han_viet_name {
                aliases.push(generated_han_viet_value(
                    value,
                    "E",
                    "NPC",
                    &han_viet_snapshot.checked_at,
                ));
            }
            Some(SourcedEntityInput {
                source_provider: NPC_SOURCE_PROVIDER.to_string(),
                source_identity: npc.source_key.clone(),
                entity: EntityInput {
                    entity_type: "npc".to_string(),
                    official_english_name: official_english_name.clone(),
                    official_original_name: npc.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: None,
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some(format!(
                        "source key: {}. HoYoWiki NPC entry page ID: {}. Source: {}. Page: {}. Checked {}. The bundled English join supplies the display name; the Simplified Chinese source name is preserved for localized lookup.",
                        npc.source_key,
                        npc.hoyowiki_entry_page_id,
                        NPC_SOURCE_PROVIDER,
                        npc.page_url,
                        snapshot.checked_at,
                    )),
                },
                aliases,
            })
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_light_cones(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    ensure_work(store, work_id)?;
    let snapshot = light_cone_snapshot()?;
    let han_viet_snapshot = light_cone_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .light_cones
        .iter()
        .map(|light_cone| (light_cone.source_key.as_str(), light_cone))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .light_cones
        .iter()
        .map(|light_cone| {
            let han_viet = han_viet_by_key
                .get(light_cone.source_key.as_str())
                .expect("validated Light Cone Hán-Việt coverage");
            let mut aliases = vec![official_alias(
                &light_cone.official_traditional_chinese_name,
                &snapshot.checked_at,
            )];
            aliases.push(generated_han_viet_value(
                &han_viet.han_viet_name,
                &han_viet.confidence,
                "Light Cone",
                &han_viet_snapshot.checked_at,
            ));
            SourcedEntityInput {
                source_provider: SOURCE_PROVIDER.to_string(),
                source_identity: light_cone.source_key.clone(),
                entity: EntityInput {
                    entity_type: "light-cone".to_string(),
                    official_english_name: light_cone.official_english_name.clone(),
                    official_original_name: light_cone.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: Some(light_cone.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: light_cone.official_english_description.clone(),
                    other_information: Some(format!(
                        "{}. Path: {} ({}). Rarity: {}★. Game data ID: {}.",
                        marker(
                            &light_cone.source_key,
                            "light-cone",
                            source_commit(&snapshot.source),
                            &snapshot.checked_at
                        ),
                        light_cone.path_english_name,
                        light_cone.path_vietnamese_name,
                        light_cone.rarity,
                        light_cone.game_data_id,
                    )),
                },
                aliases,
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_relic_sets(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    ensure_work(store, work_id)?;
    let snapshot = relic_set_snapshot()?;
    let han_viet_snapshot = relic_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .relic_sets
        .iter()
        .map(|relic_set| (relic_set.source_key.as_str(), relic_set))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .relic_sets
        .iter()
        .map(|relic_set| {
            let han_viet = han_viet_by_key
                .get(relic_set.source_key.as_str())
                .expect("validated Relic Set Hán-Việt coverage");
            let mut aliases = vec![official_alias(
                &relic_set.official_traditional_chinese_name,
                &snapshot.checked_at,
            )];
            aliases.extend(han_viet.generated_han_viet_aliases.iter().map(|alias| {
                generated_han_viet_value(
                    &alias.value,
                    &alias.confidence,
                    "Relic Set",
                    &han_viet_snapshot.checked_at,
                )
            }));
            SourcedEntityInput {
                source_provider: SOURCE_PROVIDER.to_string(),
                source_identity: relic_set.source_key.clone(),
                entity: EntityInput {
                    entity_type: "relic-set".to_string(),
                    official_english_name: relic_set.official_english_name.clone(),
                    official_original_name: relic_set.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: Some(relic_set.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: join_description(&relic_set.descriptions.en),
                    other_information: Some(format!(
                        "{}. Vietnamese set effects are preserved in the bundled relic-set snapshot: {}. Game data ID: {}.",
                        marker(&relic_set.source_key, "relic-set", source_commit(&snapshot.source), &snapshot.checked_at),
                        join_description(&relic_set.descriptions.vi)
                            .unwrap_or_else(|| "not supplied".to_string()),
                        relic_set.game_data_id,
                    )),
                },
                aliases,
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_relics(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    ensure_work(store, work_id)?;
    let snapshot = relic_snapshot()?;
    let han_viet_snapshot = relic_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .relics
        .iter()
        .map(|relic| (relic.source_key.as_str(), relic))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .relics
        .iter()
        .map(|relic| {
            let han_viet = han_viet_by_key
                .get(relic.source_key.as_str())
                .expect("validated Relic Hán-Việt coverage");
            let mut aliases = vec![official_alias(
                &relic.official_traditional_chinese_name,
                &snapshot.checked_at,
            )];
            aliases.extend(han_viet.generated_han_viet_aliases.iter().map(|alias| {
                generated_han_viet_value(
                    &alias.value,
                    &alias.confidence,
                    "Relic",
                    &han_viet_snapshot.checked_at,
                )
            }));
            SourcedEntityInput {
                source_provider: SOURCE_PROVIDER.to_string(),
                source_identity: relic.source_key.clone(),
                entity: EntityInput {
                    entity_type: "relic".to_string(),
                    official_english_name: relic.official_english_name.clone(),
                    official_original_name: relic.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: Some(relic.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some(format!(
                        "{}. Parent Relic Set source key: {}. Slot: {}. Rarity: {}★. Max level: {}. Game data ID: {}.",
                        marker(&relic.source_key, "relic", source_commit(&snapshot.source), &snapshot.checked_at),
                        relic.relic_set_source_key,
                        relic.slot,
                        relic.rarity,
                        relic.max_level,
                        relic.game_data_id,
                    )),
                },
                aliases,
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_paths(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    ensure_work(store, work_id)?;
    let snapshot = path_snapshot()?;
    let han_viet_snapshot = taxonomy_han_viet_snapshot()?;
    let inputs = snapshot
        .paths
        .iter()
        .map(|path| {
            let han_viet = han_viet_snapshot
                .paths
                .iter()
                .find(|record| record.source_key == path.source_key)
                .expect("validated Path Hán-Việt coverage");
            let mut aliases = vec![official_alias(
                &path.official_traditional_chinese_name,
                &snapshot.checked_at,
            )];
            aliases.extend(han_viet.generated_aliases.iter().map(|alias| {
                generated_han_viet_alias(alias, "Path", &han_viet_snapshot.checked_at)
            }));
            SourcedEntityInput {
                source_provider: SOURCE_PROVIDER.to_string(),
                source_identity: path.source_key.clone(),
                entity: EntityInput {
                    entity_type: "path".to_string(),
                    official_english_name: path.official_english_name.clone(),
                    official_original_name: path.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: Some(path.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: path.official_english_description.clone(),
                    other_information: Some(format!(
                        "{}. Game data key: {}. Game data ID: {}.",
                        marker(
                            &path.source_key,
                            "path",
                            source_commit(&snapshot.source),
                            &snapshot.checked_at
                        ),
                        path.game_data_id,
                        path.game_data_id,
                    )),
                },
                aliases,
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_elements(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    ensure_work(store, work_id)?;
    let snapshot = element_snapshot()?;
    let han_viet_snapshot = taxonomy_han_viet_snapshot()?;
    let inputs = snapshot
        .elements
        .iter()
        .map(|element| {
            let han_viet = han_viet_snapshot
                .elements
                .iter()
                .find(|record| record.source_key == element.source_key)
                .expect("validated Element Hán-Việt coverage");
            let mut aliases = vec![official_alias(
                &element.official_traditional_chinese_name,
                &snapshot.checked_at,
            )];
            aliases.extend(han_viet.generated_aliases.iter().map(|alias| {
                generated_han_viet_alias(alias, "Element", &han_viet_snapshot.checked_at)
            }));
            SourcedEntityInput {
                source_provider: SOURCE_PROVIDER.to_string(),
                source_identity: element.source_key.clone(),
                entity: EntityInput {
                    entity_type: "element".to_string(),
                    official_english_name: element.official_english_name.clone(),
                    official_original_name: element.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: Some(element.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: element.official_english_description.clone(),
                    other_information: Some(format!(
                        "{}. Game data key: {}. Presentation color: {}.",
                        marker(
                            &element.source_key,
                            "element",
                            source_commit(&snapshot.source),
                            &snapshot.checked_at
                        ),
                        element.source_key_name,
                        element.color.as_deref().unwrap_or("not supplied"),
                    )),
                },
                aliases,
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

fn entity_source_key(entity: &WikiEntity) -> Option<&str> {
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("StarRailRes source key: ")?;
    suffix.split('.').next().map(str::trim)
}

fn rarity_color(rarity: u8) -> &'static str {
    match rarity {
        3 => THREE_STAR_COLOR,
        4 => FOUR_STAR_COLOR,
        5 => FIVE_STAR_COLOR,
        _ => THEME.border_strong,
    }
}

fn asset_url(path: &str) -> String {
    format!("{ASSET_BASE_URL}/{path}")
}

fn character_display_name(character: &CharacterRecord) -> String {
    if character.official_english_name != "{NICKNAME}" {
        return character.official_english_name.clone();
    }

    match character.game_data_id.as_str() {
        "8001" | "8003" | "8005" | "8007" | "8009" => "Trailblazer (Male)".to_string(),
        "8002" | "8004" | "8006" | "8008" | "8010" => "Trailblazer (Female)".to_string(),
        _ => "Trailblazer".to_string(),
    }
}

fn character_record(source_key: &str) -> Option<&'static CharacterRecord> {
    character_snapshot()
        .ok()?
        .characters
        .iter()
        .find(|record| record.source_key == source_key)
}

fn light_cone_record(source_key: &str) -> Option<&'static LightConeRecord> {
    light_cone_snapshot()
        .ok()?
        .light_cones
        .iter()
        .find(|record| record.source_key == source_key)
}

fn relic_set_record(source_key: &str) -> Option<&'static RelicSetRecord> {
    relic_set_snapshot()
        .ok()?
        .relic_sets
        .iter()
        .find(|record| record.source_key == source_key)
}

fn relic_record(source_key: &str) -> Option<&'static RelicRecord> {
    relic_snapshot()
        .ok()?
        .relics
        .iter()
        .find(|record| record.source_key == source_key)
}

fn path_record(source_key: &str) -> Option<&'static PathRecord> {
    path_snapshot()
        .ok()?
        .paths
        .iter()
        .find(|record| record.source_key == source_key)
}

fn element_record(source_key: &str) -> Option<&'static ElementRecord> {
    element_snapshot()
        .ok()?
        .elements
        .iter()
        .find(|record| record.source_key == source_key)
}

impl LibraryModule for HonkaiStarRailModule {
    fn id(&self) -> &'static str {
        "honkai-star-rail"
    }

    fn kind(&self) -> WorkKind {
        WorkKind::Game
    }

    fn display_name(&self) -> &'static str {
        "Honkai: Star Rail"
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

    fn entity_catalog_label(&self, entity: &WikiEntity) -> Option<String> {
        let source_key = entity_source_key(entity)?;
        if entity.entity_type != "character" {
            return None;
        }
        let character = character_record(source_key)?;
        let display_name = character_display_name(character);
        let needs_context = character.official_english_name == "{NICKNAME}"
            || character_snapshot().ok()?.characters.iter().any(|other| {
                other.source_key != character.source_key
                    && other.official_english_name == character.official_english_name
            });
        needs_context.then(|| {
            format!(
                "{} · {} · {}",
                display_name, character.path_english_name, character.element_english_name
            )
        })
    }

    fn entity_presentation(&self, entity: &WikiEntity) -> Option<EntityPresentation> {
        let source_key = entity_source_key(entity)?;
        match entity.entity_type.as_str() {
            "character" => {
                let character = character_record(source_key)?;
                character_thumbnail(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/honkai-star-rail/characters/{source_key}.png"
                    ),
                    accent_color: rarity_color(character.rarity).to_string(),
                    context_label: Some(format!(
                        "{} · {}",
                        character.path_english_name, character.element_english_name
                    )),
                    context_icon_url: element_record(&format!(
                        "starrail-element-{}",
                        character.element_key
                    ))
                    .map(|element| asset_url(&element.icon_path)),
                    label: format!("{}★", character.rarity),
                    rarity: Some(character.rarity),
                    facets: BTreeMap::from([
                        ("path".to_string(), character.path_english_name.clone()),
                        (
                            "element".to_string(),
                            character.element_english_name.clone(),
                        ),
                    ]),
                    facet_values: BTreeMap::new(),
                })
            }
            "light-cone" => {
                let light_cone = light_cone_record(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: asset_url(&light_cone.thumbnail_path),
                    accent_color: rarity_color(light_cone.rarity).to_string(),
                    context_label: Some(light_cone.path_english_name.clone()),
                    context_icon_url: None,
                    label: format!("{}★", light_cone.rarity),
                    rarity: Some(light_cone.rarity),
                    facets: BTreeMap::from([(
                        "path".to_string(),
                        light_cone.path_english_name.clone(),
                    )]),
                    facet_values: BTreeMap::new(),
                })
            }
            "relic-set" => {
                let relic_set = relic_set_record(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: asset_url(&relic_set.icon_path),
                    accent_color: THEME.accent.to_string(),
                    context_label: Some("Relic Set".to_string()),
                    context_icon_url: None,
                    label: "SET".to_string(),
                    rarity: None,
                    facets: BTreeMap::new(),
                    facet_values: BTreeMap::new(),
                })
            }
            "relic" => {
                let relic = relic_record(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: asset_url(&relic.icon_path),
                    accent_color: rarity_color(relic.rarity).to_string(),
                    context_label: Some(relic.slot.clone()),
                    context_icon_url: None,
                    label: format!("{}★", relic.rarity),
                    rarity: Some(relic.rarity),
                    facets: BTreeMap::from([("relic_slot".to_string(), relic.slot.clone())]),
                    facet_values: BTreeMap::new(),
                })
            }
            "path" => {
                let path = path_record(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: asset_url(&path.icon_path),
                    accent_color: THEME.accent.to_string(),
                    context_label: Some("Combat Path".to_string()),
                    context_icon_url: None,
                    label: "PATH".to_string(),
                    rarity: None,
                    facets: BTreeMap::new(),
                    facet_values: BTreeMap::new(),
                })
            }
            "element" => {
                let element = element_record(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: asset_url(&element.icon_path),
                    accent_color: element
                        .color
                        .clone()
                        .unwrap_or_else(|| THEME.accent.to_string()),
                    context_label: Some("Combat Element".to_string()),
                    context_icon_url: None,
                    label: "ELEMENT".to_string(),
                    rarity: None,
                    facets: BTreeMap::new(),
                    facet_values: BTreeMap::new(),
                })
            }
            _ => None,
        }
    }
}

static MODULE: HonkaiStarRailModule = HonkaiStarRailModule;

pub fn module() -> &'static dyn LibraryModule {
    &MODULE
}

pub fn register(registry: &mut ModuleRegistry) -> perwiga_core::Result<()> {
    registry.register(module())
}

include!(concat!(env!("OUT_DIR"), "/character_thumbnails.rs"));

pub fn character_thumbnail(source_key: &str) -> Option<&'static [u8]> {
    character_thumbnail_asset(source_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_exposes_stable_identity_baseline_types_and_facets() {
        assert_eq!(module().id(), "honkai-star-rail");
        assert_eq!(module().kind(), WorkKind::Game);
        assert_eq!(module().display_name(), "Honkai: Star Rail");
        assert_eq!(module().entity_types().len(), 8);
        assert!(module()
            .entity_types()
            .iter()
            .any(|definition| definition.key == "npc"));
        assert!(module()
            .entity_types()
            .iter()
            .any(|definition| definition.key == "region"));
        assert_eq!(module().entity_facets().len(), 3);
    }

    #[test]
    fn curated_catalog_imports_are_additive_idempotent_and_multilingual() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "honkai-star-rail", "Honkai: Star Rail")
            .expect("Star Rail work");

        let characters = import_curated_characters(&mut store, &work.id).expect("characters");
        assert_eq!(characters.inserted, 97);
        assert_eq!(characters.aliases_inserted, 182);
        let npcs = import_curated_npcs(&mut store, &work.id).expect("NPCs");
        assert_eq!(npcs.inserted, 433);
        assert_eq!(npcs.aliases_inserted, 430);
        let light_cones = import_curated_light_cones(&mut store, &work.id).expect("Light Cones");
        assert_eq!(light_cones.inserted, 169);
        assert_eq!(light_cones.aliases_inserted, 338);
        let relic_sets = import_curated_relic_sets(&mut store, &work.id).expect("Relic Sets");
        assert_eq!(relic_sets.inserted, 60);
        assert_eq!(relic_sets.aliases_inserted, 120);
        let relics = import_curated_relics(&mut store, &work.id).expect("Relics");
        assert_eq!(relics.inserted, 742);
        assert_eq!(relics.aliases_inserted, 1484);
        let paths = import_curated_paths(&mut store, &work.id).expect("Paths");
        assert_eq!(paths.inserted, 9);
        assert_eq!(paths.aliases_inserted, 18);
        let elements = import_curated_elements(&mut store, &work.id).expect("Elements");
        assert_eq!(elements.inserted, 7);
        assert_eq!(elements.aliases_inserted, 14);

        let march = store
            .list_entities(&work.id, Some("character"))
            .expect("Characters")
            .into_iter()
            .find(|entity| {
                entity.official_english_name == "March 7th"
                    && entity
                        .other_information
                        .as_deref()
                        .is_some_and(|value| value.contains("Path: Preservation"))
            })
            .expect("March 7th");
        assert_eq!(march.official_vietnamese_name.as_deref(), Some("March 7th"));
        assert!(store
            .list_aliases(&march.id)
            .expect("March aliases")
            .iter()
            .any(|alias| alias.value == "三月七" && alias.language.as_deref() == Some("zh-Hant")));
        assert!(store
            .list_aliases(&march.id)
            .expect("March aliases")
            .iter()
            .any(|alias| alias.value == "Tam Nguyệt Thất"
                && alias.language.as_deref() == Some("vi")
                && alias.kind == "han-viet"));
        let presentation = module().entity_presentation(&march).expect("presentation");
        assert_eq!(presentation.rarity, Some(4));
        assert_eq!(
            presentation.thumbnail_url,
            "/assets/modules/honkai-star-rail/characters/starrailres-character-1001.png"
        );
        assert_eq!(presentation.facets["path"], "Preservation");
        assert_eq!(presentation.facets["element"], "Ice");
        assert!(march.other_information.as_deref().is_some_and(|value| {
            value.contains(
                "Thumbnail source URL: https://raw.githubusercontent.com/Mar-7th/StarRailRes/",
            )
        }));

        let trailblazer = store
            .list_entities(&work.id, Some("character"))
            .expect("Characters")
            .into_iter()
            .find(|entity| {
                entity
                    .other_information
                    .as_deref()
                    .is_some_and(|value| value.contains("Game data ID: 8001."))
            })
            .expect("male Trailblazer");
        assert_eq!(
            module().entity_catalog_label(&trailblazer).as_deref(),
            Some("Trailblazer (Male) · Destruction · Physical")
        );

        let demetria = store
            .list_entities(&work.id, Some("npc"))
            .expect("NPCs")
            .into_iter()
            .find(|entity| entity.official_english_name == "Demetria")
            .expect("Demetria NPC");
        assert!(demetria
            .other_information
            .as_deref()
            .is_some_and(|value| value.contains("source key: hoyowiki-npc-")));
        assert!(store
            .list_aliases(&demetria.id)
            .expect("NPC aliases")
            .iter()
            .any(|alias| alias.value == "Đức Mặc Thắc Nhĩ" && alias.kind == "han-viet"));

        let second = import_curated_characters(&mut store, &work.id).expect("second characters");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 97);
        assert_eq!(second.aliases_inserted, 0);
        assert_eq!(second.aliases_unchanged, 182);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
    }
}
