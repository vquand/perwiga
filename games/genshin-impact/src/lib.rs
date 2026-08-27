use std::{
    collections::{BTreeMap, HashSet},
    sync::OnceLock,
};

use perwiga_core::{
    model::{
        AliasInput, EntityAppearanceInput, EntityAppearanceLocationInput, EntityImportSummary,
        EntityInput, SourcedEntityInput, WikiEntity,
    },
    Capability, EntityFacetDefinition, EntityFacetOption, EntityPresentation, EntityTypeDefinition,
    LibraryModule, ModuleRegistry, PerwigaError, Store, ThemeDefinition, WorkKind,
};
use serde::Deserialize;

pub struct GenshinImpactModule;

static CAPABILITIES: &[Capability] = &[Capability::ManualContent, Capability::EntitySchema];
const SOURCE_PROVIDER: &str = "Genshin Impact official global character directory";
const SOURCE_URL: &str = "https://genshin.hoyoverse.com/en/character/mondstadt";
const HAN_VIET_DICTIONARY_URL: &str = "https://hvdic.thivien.net/transcript.php#trans";
const WEAPON_SOURCE_PROVIDER: &str = "GenshinData game-data extracts via genshin-db";
const WEAPON_SOURCE_URL: &str = "https://github.com/theBowja/genshin-db";
const SKIN_SOURCE_PROVIDER: &str = "Official Genshin HoYoWiki Outfit catalog";
const SKIN_SOURCE_URL: &str = "https://wiki.hoyolab.com/pc/genshin/aggregate/34";
const ARTIFACT_SOURCE_PROVIDER: &str =
    "GenshinData Artifact catalog verified against official HoYoWiki";
const ARTIFACT_DOMAIN_SOURCE_PROVIDER: &str = "GenshinData Artifact Domain extracts";
const REGION_SOURCE_PROVIDER: &str = "GenshinData Geography extracts via genshin-db";
const ARTIFACT_SOURCE_URL: &str = "https://wiki.hoyolab.com/pc/genshin/aggregate/5";
const ONE_STAR_COLOR: &str = "#8b9198";
const TWO_STAR_COLOR: &str = "#6fa66f";
const THREE_STAR_COLOR: &str = "#6287c5";
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
    EntityTypeDefinition {
        key: "weapon",
        display_name: "Weapon",
        description: "An obtainable equippable weapon from the Genshin Impact catalog.",
    },
    EntityTypeDefinition {
        key: "skin",
        display_name: "Skin",
        description: "An alternate appearance for a character, weapon, or weapon type.",
    },
    EntityTypeDefinition {
        key: "artifact-set",
        display_name: "Artifact Set",
        description: "A named collection of equippable Artifacts with shared set effects.",
    },
    EntityTypeDefinition {
        key: "artifact-piece",
        display_name: "Artifact Piece",
        description: "One Flower, Plume, Sands, Goblet, or Circlet belonging to an Artifact Set.",
    },
    EntityTypeDefinition {
        key: "domain",
        display_name: "Domain",
        description: "A repeatable challenge entrance that hosts Artifact rewards.",
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
static DOMAIN_REGION_OPTIONS: &[EntityFacetOption] = &[
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
        value: "Nod-Krai",
        display_name: "Nod-Krai",
    },
    EntityFacetOption {
        value: "Snezhnaya",
        display_name: "Snezhnaya",
    },
];
static REGION_TYPE_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "World",
        display_name: "World",
    },
    EntityFacetOption {
        value: "Nation",
        display_name: "Nation",
    },
    EntityFacetOption {
        value: "Autonomous region",
        display_name: "Autonomous region",
    },
    EntityFacetOption {
        value: "Special region",
        display_name: "Special region",
    },
    EntityFacetOption {
        value: "Subregion",
        display_name: "Subregion",
    },
];
static REGION_PARENT_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Teyvat",
        display_name: "Teyvat",
    },
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
        value: "Nod-Krai",
        display_name: "Nod-Krai",
    },
    EntityFacetOption {
        value: "Snezhnaya",
        display_name: "Snezhnaya",
    },
    EntityFacetOption {
        value: "Temple of Space",
        display_name: "Temple of Space",
    },
];
static ARTIFACT_SLOT_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Flower of Life",
        display_name: "Flower of Life",
    },
    EntityFacetOption {
        value: "Plume of Death",
        display_name: "Plume of Death",
    },
    EntityFacetOption {
        value: "Sands of Eon",
        display_name: "Sands of Eon",
    },
    EntityFacetOption {
        value: "Goblet of Eonothem",
        display_name: "Goblet of Eonothem",
    },
    EntityFacetOption {
        value: "Circlet of Logos",
        display_name: "Circlet of Logos",
    },
];
static WEAPON_TYPE_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Sword",
        display_name: "Sword",
    },
    EntityFacetOption {
        value: "Claymore",
        display_name: "Claymore",
    },
    EntityFacetOption {
        value: "Polearm",
        display_name: "Polearm",
    },
    EntityFacetOption {
        value: "Catalyst",
        display_name: "Catalyst",
    },
    EntityFacetOption {
        value: "Bow",
        display_name: "Bow",
    },
];
static WEAPON_MAIN_STAT_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "23",
        display_name: "23",
    },
    EntityFacetOption {
        value: "33",
        display_name: "33",
    },
    EntityFacetOption {
        value: "38",
        display_name: "38",
    },
    EntityFacetOption {
        value: "39",
        display_name: "39",
    },
    EntityFacetOption {
        value: "40",
        display_name: "40",
    },
    EntityFacetOption {
        value: "41",
        display_name: "41",
    },
    EntityFacetOption {
        value: "42",
        display_name: "42",
    },
    EntityFacetOption {
        value: "44",
        display_name: "44",
    },
    EntityFacetOption {
        value: "45",
        display_name: "45",
    },
    EntityFacetOption {
        value: "46",
        display_name: "46",
    },
    EntityFacetOption {
        value: "48",
        display_name: "48",
    },
    EntityFacetOption {
        value: "49",
        display_name: "49",
    },
];
static WEAPON_SUBSTAT_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "ATK",
        display_name: "ATK",
    },
    EntityFacetOption {
        value: "CRIT DMG",
        display_name: "CRIT DMG",
    },
    EntityFacetOption {
        value: "CRIT Rate",
        display_name: "CRIT Rate",
    },
    EntityFacetOption {
        value: "DEF",
        display_name: "DEF",
    },
    EntityFacetOption {
        value: "Elemental Mastery",
        display_name: "Elemental Mastery",
    },
    EntityFacetOption {
        value: "Energy Recharge",
        display_name: "Energy Recharge",
    },
    EntityFacetOption {
        value: "HP",
        display_name: "HP",
    },
    EntityFacetOption {
        value: "None",
        display_name: "None",
    },
    EntityFacetOption {
        value: "Physical DMG Bonus",
        display_name: "Physical DMG Bonus",
    },
];
static SKIN_APPLICATION_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Character",
        display_name: "Character",
    },
    EntityFacetOption {
        value: "Weapon",
        display_name: "Weapon",
    },
    EntityFacetOption {
        value: "Weapon type",
        display_name: "Weapon type",
    },
];
static SKIN_CHARACTER_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Aether",
        display_name: "Aether",
    },
    EntityFacetOption {
        value: "Amber",
        display_name: "Amber",
    },
    EntityFacetOption {
        value: "Barbara",
        display_name: "Barbara",
    },
    EntityFacetOption {
        value: "Bennett",
        display_name: "Bennett",
    },
    EntityFacetOption {
        value: "Charlotte",
        display_name: "Charlotte",
    },
    EntityFacetOption {
        value: "Citlali",
        display_name: "Citlali",
    },
    EntityFacetOption {
        value: "Diluc",
        display_name: "Diluc",
    },
    EntityFacetOption {
        value: "Durin",
        display_name: "Durin",
    },
    EntityFacetOption {
        value: "Fischl",
        display_name: "Fischl",
    },
    EntityFacetOption {
        value: "Ganyu",
        display_name: "Ganyu",
    },
    EntityFacetOption {
        value: "Hu Tao",
        display_name: "Hu Tao",
    },
    EntityFacetOption {
        value: "Jean",
        display_name: "Jean",
    },
    EntityFacetOption {
        value: "Kaeya",
        display_name: "Kaeya",
    },
    EntityFacetOption {
        value: "Kamisato Ayaka",
        display_name: "Kamisato Ayaka",
    },
    EntityFacetOption {
        value: "Keqing",
        display_name: "Keqing",
    },
    EntityFacetOption {
        value: "Kirara",
        display_name: "Kirara",
    },
    EntityFacetOption {
        value: "Klee",
        display_name: "Klee",
    },
    EntityFacetOption {
        value: "Lisa",
        display_name: "Lisa",
    },
    EntityFacetOption {
        value: "Lumine",
        display_name: "Lumine",
    },
    EntityFacetOption {
        value: "Mona",
        display_name: "Mona",
    },
    EntityFacetOption {
        value: "Neuvillette",
        display_name: "Neuvillette",
    },
    EntityFacetOption {
        value: "Nilou",
        display_name: "Nilou",
    },
    EntityFacetOption {
        value: "Ningguang",
        display_name: "Ningguang",
    },
    EntityFacetOption {
        value: "Rosaria",
        display_name: "Rosaria",
    },
    EntityFacetOption {
        value: "Shenhe",
        display_name: "Shenhe",
    },
    EntityFacetOption {
        value: "Xiangling",
        display_name: "Xiangling",
    },
    EntityFacetOption {
        value: "Xingqiu",
        display_name: "Xingqiu",
    },
    EntityFacetOption {
        value: "Yaoyao",
        display_name: "Yaoyao",
    },
    EntityFacetOption {
        value: "Yelan",
        display_name: "Yelan",
    },
];
static ENTITY_FACETS: &[EntityFacetDefinition] = &[
    EntityFacetDefinition {
        key: "region",
        display_name: "Region",
        entity_types: &["character"],
        options: DOMAIN_REGION_OPTIONS,
    },
    EntityFacetDefinition {
        key: "weapon_type",
        display_name: "Weapon type",
        entity_types: &["weapon"],
        options: WEAPON_TYPE_OPTIONS,
    },
    EntityFacetDefinition {
        key: "main_stat",
        display_name: "Base ATK (Lv. 1)",
        entity_types: &["weapon"],
        options: WEAPON_MAIN_STAT_OPTIONS,
    },
    EntityFacetDefinition {
        key: "substat",
        display_name: "Substat",
        entity_types: &["weapon"],
        options: WEAPON_SUBSTAT_OPTIONS,
    },
    EntityFacetDefinition {
        key: "ascension_region",
        display_name: "Ascension region",
        entity_types: &["weapon"],
        options: REGION_OPTIONS,
    },
    EntityFacetDefinition {
        key: "skin_application",
        display_name: "Target",
        entity_types: &["skin"],
        options: SKIN_APPLICATION_OPTIONS,
    },
    EntityFacetDefinition {
        key: "skin_character",
        display_name: "Character",
        entity_types: &["skin"],
        options: SKIN_CHARACTER_OPTIONS,
    },
    EntityFacetDefinition {
        key: "skin_weapon_type",
        display_name: "Weapon type",
        entity_types: &["skin"],
        options: WEAPON_TYPE_OPTIONS,
    },
    EntityFacetDefinition {
        key: "artifact_slot",
        display_name: "Artifact slot",
        entity_types: &["artifact-piece"],
        options: ARTIFACT_SLOT_OPTIONS,
    },
    EntityFacetDefinition {
        key: "domain_region",
        display_name: "Region",
        entity_types: &["domain"],
        options: DOMAIN_REGION_OPTIONS,
    },
    EntityFacetDefinition {
        key: "region_type",
        display_name: "Region type",
        entity_types: &["region"],
        options: REGION_TYPE_OPTIONS,
    },
    EntityFacetDefinition {
        key: "parent_region",
        display_name: "Parent region",
        entity_types: &["region"],
        options: REGION_PARENT_OPTIONS,
    },
];
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
struct CharacterHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: CharacterCatalogScope,
    confidence: CharacterHanVietConfidence,
    characters: Vec<CuratedCharacterHanViet>,
}

#[derive(Debug, Deserialize)]
struct CharacterHanVietConfidence {
    han_viet_names: String,
}

#[derive(Debug, Deserialize)]
struct CuratedCharacterHanViet {
    source_key: String,
    official_english_name: String,
    official_traditional_chinese_name: String,
    han_viet_name: String,
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

#[derive(Debug, Deserialize)]
struct WeaponSnapshot {
    schema: String,
    checked_at: String,
    source: WeaponSource,
    catalog_scope: WeaponCatalogScope,
    weapons: Vec<CuratedWeapon>,
}

#[derive(Debug, Deserialize)]
struct WeaponSource {
    commit: String,
}

#[derive(Debug, Deserialize)]
struct WeaponCatalogScope {
    included_records: usize,
    excluded_non_obtainable_records: usize,
}

#[derive(Debug, Deserialize)]
struct CuratedWeapon {
    source_key: String,
    game_data_id: u64,
    genshin_db_key: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    official_english_description: String,
    weapon_type: String,
    rarity: u8,
    base_attack: u8,
    substat: String,
    base_substat_value: Option<String>,
    ascension_region: String,
    ascension_material: String,
    release_version: String,
    thumbnail_asset: String,
}

#[derive(Debug, Deserialize)]
struct WeaponThumbnailSnapshot {
    schema: String,
    catalog_scope: WeaponThumbnailCatalogScope,
    weapons: Vec<CuratedWeaponThumbnail>,
}

#[derive(Debug, Deserialize)]
struct WeaponThumbnailCatalogScope {
    catalog_records: usize,
    wiki_matches: usize,
    missing_from_hoyowiki: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CuratedWeaponThumbnail {
    source_key: String,
    official_english_name: String,
    thumbnail_asset: String,
}

#[derive(Debug, Deserialize)]
struct WeaponHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: CharacterCatalogScope,
    confidence: CharacterHanVietConfidence,
    weapons: Vec<CuratedWeaponHanViet>,
}

#[derive(Debug, Deserialize)]
struct CuratedWeaponHanViet {
    source_key: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    han_viet_name: String,
}

#[derive(Debug, Deserialize)]
struct SkinSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: SkinCatalogScope,
    skins: Vec<CuratedSkin>,
}

#[derive(Debug, Deserialize)]
struct SkinCatalogScope {
    included_records: usize,
    excluded_default_outfits: usize,
    confirmed_weapon_skins: usize,
}

#[derive(Debug, Deserialize)]
struct CuratedSkin {
    source_key: String,
    hoyowiki_entry_page_id: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    official_english_description: String,
    rarity: u8,
    application_kind: String,
    applicable_characters: Vec<String>,
    applicable_weapons: Vec<String>,
    applicable_weapon_types: Vec<String>,
    thumbnail_asset: String,
}

#[derive(Debug, Deserialize)]
struct SkinHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: CharacterCatalogScope,
    confidence: CharacterHanVietConfidence,
    skins: Vec<CuratedSkinHanViet>,
}

#[derive(Debug, Deserialize)]
struct CuratedSkinHanViet {
    source_key: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    han_viet_name: String,
}

#[derive(Debug, Deserialize)]
struct ArtifactSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: ArtifactCatalogScope,
    artifact_sets: Vec<CuratedArtifactSet>,
    artifact_pieces: Vec<CuratedArtifactPiece>,
}

#[derive(Debug, Deserialize)]
struct ArtifactCatalogScope {
    artifact_sets: usize,
    artifact_pieces: usize,
}

#[derive(Debug, Deserialize)]
struct CuratedArtifactSet {
    source_key: String,
    game_data_id: u64,
    hoyowiki_entry_page_id: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    minimum_rarity: u8,
    maximum_rarity: u8,
    effect_1_piece: Option<String>,
    effect_2_piece: Option<String>,
    effect_4_piece: Option<String>,
    effect_tags: Vec<String>,
    artifact_piece_source_keys: Vec<String>,
    thumbnail_asset: String,
}

#[derive(Debug, Deserialize)]
struct CuratedArtifactPiece {
    source_key: String,
    artifact_set_source_key: String,
    artifact_set_english_name: String,
    slot: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    official_english_description: String,
    official_english_story: String,
    minimum_rarity: u8,
    maximum_rarity: u8,
    thumbnail_asset: String,
}

#[derive(Debug, Deserialize)]
struct ArtifactHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: ArtifactCatalogScope,
    confidence: CharacterHanVietConfidence,
    artifact_sets: Vec<CuratedArtifactHanViet>,
    artifact_pieces: Vec<CuratedArtifactHanViet>,
}

#[derive(Debug, Deserialize)]
struct CuratedArtifactHanViet {
    source_key: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    han_viet_name: String,
}

#[derive(Debug, Deserialize)]
struct ArtifactDomainSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: ArtifactDomainCatalogScope,
    domains: Vec<CuratedArtifactDomain>,
}

#[derive(Debug, Deserialize)]
struct ArtifactDomainCatalogScope {
    domain_entrances: usize,
    challenge_stages: usize,
}

#[derive(Debug, Deserialize)]
struct CuratedArtifactDomain {
    source_key: String,
    game_data_entrance_id: u64,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    official_english_description: String,
    region: String,
    challenge_stage_ids: Vec<u64>,
    challenge_stage_names: Vec<String>,
    recommended_levels: Vec<u64>,
    minimum_unlock_rank: u64,
    recommended_elements: Vec<String>,
    hosted_artifact_set_names: Vec<String>,
    hosted_artifact_set_source_keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RegionSnapshot {
    schema: String,
    checked_at: String,
    source: WeaponSource,
    catalog_scope: RegionCatalogScope,
    regions: Vec<CuratedRegion>,
}

#[derive(Debug, Deserialize)]
struct RegionCatalogScope {
    regions: usize,
    subregions: usize,
    included_records: usize,
}

#[derive(Debug, Deserialize)]
struct CuratedRegion {
    source_key: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    subtype: String,
    parent_source_key: Option<String>,
    region_id: Option<u64>,
    area_id: Option<u64>,
    viewpoint_ids: Vec<u64>,
    official_english_description: String,
    source_table: String,
    catalog_index: usize,
}

#[derive(Debug, Deserialize)]
struct RegionHanVietSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: CharacterCatalogScope,
    confidence: CharacterHanVietConfidence,
    regions: Vec<CuratedRegionHanViet>,
}

#[derive(Debug, Deserialize)]
struct CuratedRegionHanViet {
    source_key: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    han_viet_name: String,
}

#[derive(Debug, Deserialize)]
struct NpcSnapshot {
    schema: String,
    checked_at: String,
    catalog_scope: NpcCatalogScope,
    npcs: Vec<CuratedNpc>,
}

#[derive(Debug, Deserialize)]
struct NpcCatalogScope {
    included_records: usize,
}

#[derive(Debug, Deserialize)]
struct CuratedNpc {
    source_key: String,
    page_id: u64,
    page_title: String,
    page_url: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    official_traditional_chinese_name: String,
    official_vietnamese_name: String,
    official_english_description: String,
    regions: Vec<String>,
    locations: Vec<String>,
    relationships: Vec<CuratedNpcRelationship>,
    categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CuratedNpcRelationship {
    kind: String,
    title: String,
    locations: Vec<String>,
    regions: Vec<String>,
    source: String,
}

#[derive(Debug, Deserialize)]
struct NpcHanVietSnapshot {
    schema: String,
    catalog_scope: NpcHanVietScope,
    confidence: CharacterHanVietConfidence,
    npcs: Vec<CuratedNpcHanViet>,
}

#[derive(Debug, Deserialize)]
struct NpcHanVietScope {
    aliases: usize,
}

#[derive(Debug, Deserialize)]
struct CuratedNpcHanViet {
    source_key: String,
    official_english_name: String,
    official_simplified_chinese_name: String,
    han_viet_name: String,
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

fn curated_han_viet_snapshot() -> Result<&'static CharacterHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<CharacterHanVietSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: CharacterHanVietSnapshot =
            serde_json::from_str(include_str!("../data/character-han-viet.json"))
                .map_err(|error| format!("invalid bundled Genshin Hán-Việt snapshot: {error}"))?;
        validate_han_viet_snapshot(
            &snapshot,
            curated_snapshot().map_err(|error| error.to_string())?,
        )?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn validate_han_viet_snapshot(
    snapshot: &CharacterHanVietSnapshot,
    catalog: &CharacterSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.character-han-viet.v1" {
        return Err("unexpected Genshin Hán-Việt snapshot schema".into());
    }
    if snapshot.confidence.han_viet_names != "E" {
        return Err("Genshin Hán-Việt aliases must retain generated confidence E".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.characters.len()
        || snapshot.characters.len() != catalog.characters.len()
    {
        return Err("Genshin Hán-Việt snapshot coverage is incomplete".into());
    }

    let catalog_by_key = catalog
        .characters
        .iter()
        .map(|character| (character.source_key.as_str(), character))
        .collect::<BTreeMap<_, _>>();
    let mut source_keys = HashSet::new();
    for alias in &snapshot.characters {
        if !source_keys.insert(alias.source_key.as_str()) {
            return Err(format!(
                "duplicate Genshin Hán-Việt alias: {}",
                alias.source_key
            ));
        }
        let character = catalog_by_key
            .get(alias.source_key.as_str())
            .ok_or_else(|| format!("unknown Genshin Hán-Việt alias: {}", alias.source_key))?;
        if alias.official_english_name != character.official_english_name
            || character.official_traditional_chinese_name.as_deref()
                != Some(alias.official_traditional_chinese_name.as_str())
        {
            return Err(format!(
                "Genshin Hán-Việt source-name mismatch: {}",
                alias.source_key
            ));
        }
        validate_source_text(
            &alias.han_viet_name,
            300,
            "generated Hán-Việt name",
            &alias.source_key,
        )?;
    }
    Ok(())
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

fn curated_weapon_snapshot() -> Result<&'static WeaponSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<WeaponSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: WeaponSnapshot =
            serde_json::from_str(include_str!("../data/weapons.json"))
                .map_err(|error| format!("invalid bundled Genshin weapon snapshot: {error}"))?;
        validate_weapon_snapshot(&snapshot)?;
        let thumbnails: WeaponThumbnailSnapshot = serde_json::from_str(include_str!(
            "../data/weapon-thumbnails-hoyowiki.json"
        ))
        .map_err(|error| format!("invalid bundled HoYoWiki weapon thumbnail snapshot: {error}"))?;
        validate_weapon_thumbnail_snapshot(&thumbnails, &snapshot)?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_weapon_han_viet_snapshot() -> Result<&'static WeaponHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<WeaponHanVietSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: WeaponHanVietSnapshot =
            serde_json::from_str(include_str!("../data/weapon-han-viet.json"))
                .map_err(|error| format!("invalid bundled weapon Hán-Việt snapshot: {error}"))?;
        validate_weapon_han_viet_snapshot(
            &snapshot,
            curated_weapon_snapshot().map_err(|error| error.to_string())?,
        )?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_skin_snapshot() -> Result<&'static SkinSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<SkinSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: SkinSnapshot = serde_json::from_str(include_str!("../data/skins.json"))
            .map_err(|error| format!("invalid bundled Genshin Skin snapshot: {error}"))?;
        validate_skin_snapshot(&snapshot)?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_skin_han_viet_snapshot() -> Result<&'static SkinHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<SkinHanVietSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: SkinHanVietSnapshot =
            serde_json::from_str(include_str!("../data/skin-han-viet.json"))
                .map_err(|error| format!("invalid bundled Skin Hán-Việt snapshot: {error}"))?;
        validate_skin_han_viet_snapshot(
            &snapshot,
            curated_skin_snapshot().map_err(|error| error.to_string())?,
        )?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_artifact_snapshot() -> Result<&'static ArtifactSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<ArtifactSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: ArtifactSnapshot =
            serde_json::from_str(include_str!("../data/artifacts.json"))
                .map_err(|error| format!("invalid bundled Artifact snapshot: {error}"))?;
        if snapshot.schema != "perwiga.genshin-impact.artifacts.v1"
            || snapshot.catalog_scope.artifact_sets != snapshot.artifact_sets.len()
            || snapshot.catalog_scope.artifact_pieces != snapshot.artifact_pieces.len()
        {
            return Err("Genshin Artifact snapshot scope is inconsistent".into());
        }
        let mut keys = HashSet::new();
        for (source_key, rarity, asset) in snapshot
            .artifact_sets
            .iter()
            .map(|item| (&item.source_key, item.maximum_rarity, &item.thumbnail_asset))
            .chain(
                snapshot
                    .artifact_pieces
                    .iter()
                    .map(|item| (&item.source_key, item.maximum_rarity, &item.thumbnail_asset)),
            )
        {
            if !keys.insert(source_key.as_str())
                || !matches!(rarity, 1..=5)
                || asset != &format!("{source_key}.png")
                || artifact_thumbnail(source_key).is_none()
            {
                return Err(format!(
                    "invalid Artifact record or thumbnail: {source_key}"
                ));
            }
        }
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_artifact_han_viet_snapshot() -> Result<&'static ArtifactHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<ArtifactHanVietSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: ArtifactHanVietSnapshot =
            serde_json::from_str(include_str!("../data/artifact-han-viet.json"))
                .map_err(|error| format!("invalid bundled Artifact Hán-Việt snapshot: {error}"))?;
        let catalog = curated_artifact_snapshot().map_err(|error| error.to_string())?;
        if snapshot.schema != "perwiga.genshin-impact.artifact-han-viet.v1"
            || snapshot.confidence.han_viet_names != "E"
            || snapshot.catalog_scope.artifact_sets != snapshot.artifact_sets.len()
            || snapshot.catalog_scope.artifact_pieces != snapshot.artifact_pieces.len()
            || snapshot.artifact_sets.len() != catalog.artifact_sets.len()
            || snapshot.artifact_pieces.len() != catalog.artifact_pieces.len()
        {
            return Err("Genshin Artifact Hán-Việt coverage is incomplete".into());
        }
        let catalog_names = catalog
            .artifact_sets
            .iter()
            .map(|item| {
                (
                    &item.source_key,
                    (
                        &item.official_english_name,
                        &item.official_simplified_chinese_name,
                    ),
                )
            })
            .chain(catalog.artifact_pieces.iter().map(|item| {
                (
                    &item.source_key,
                    (
                        &item.official_english_name,
                        &item.official_simplified_chinese_name,
                    ),
                )
            }))
            .collect::<BTreeMap<_, _>>();
        let mut keys = HashSet::new();
        for alias in snapshot
            .artifact_sets
            .iter()
            .chain(snapshot.artifact_pieces.iter())
        {
            if !keys.insert(alias.source_key.as_str())
                || catalog_names.get(&alias.source_key)
                    != Some(&(
                        &alias.official_english_name,
                        &alias.official_simplified_chinese_name,
                    ))
                || alias.han_viet_name.trim().is_empty()
            {
                return Err(format!(
                    "invalid Artifact Hán-Việt alias: {}",
                    alias.source_key
                ));
            }
        }
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_artifact_domain_snapshot() -> Result<&'static ArtifactDomainSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<ArtifactDomainSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: ArtifactDomainSnapshot =
            serde_json::from_str(include_str!("../data/artifact-domains.json"))
                .map_err(|error| format!("invalid bundled Artifact Domain snapshot: {error}"))?;
        if snapshot.schema != "perwiga.genshin-impact.artifact-domains.v1"
            || snapshot.catalog_scope.domain_entrances != snapshot.domains.len()
            || snapshot.catalog_scope.challenge_stages
                != snapshot
                    .domains
                    .iter()
                    .map(|domain| domain.challenge_stage_ids.len())
                    .sum::<usize>()
        {
            return Err("Genshin Artifact Domain snapshot scope is inconsistent".into());
        }
        let mut keys = HashSet::new();
        for domain in &snapshot.domains {
            if !keys.insert(domain.source_key.as_str())
                || !DOMAIN_REGION_OPTIONS
                    .iter()
                    .any(|option| option.value == domain.region)
            {
                return Err(format!("invalid Artifact Domain: {}", domain.source_key));
            }
        }
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_region_snapshot() -> Result<&'static RegionSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<RegionSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: RegionSnapshot =
            serde_json::from_str(include_str!("../data/regions.json"))
                .map_err(|error| format!("invalid bundled Genshin Region snapshot: {error}"))?;
        validate_region_snapshot(&snapshot)?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_region_han_viet_snapshot() -> Result<&'static RegionHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<RegionHanVietSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: RegionHanVietSnapshot =
            serde_json::from_str(include_str!("../data/region-han-viet.json"))
                .map_err(|error| format!("invalid bundled Region Hán-Việt snapshot: {error}"))?;
        validate_region_han_viet_snapshot(
            &snapshot,
            curated_region_snapshot().map_err(|error| error.to_string())?,
        )?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn region_type_display_name(subtype: &str) -> Option<&'static str> {
    match subtype {
        "world" => Some("World"),
        "nation" => Some("Nation"),
        "autonomous-region" => Some("Autonomous region"),
        "special-region" => Some("Special region"),
        "subregion" => Some("Subregion"),
        _ => None,
    }
}

fn validate_region_snapshot(snapshot: &RegionSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.regions.v1"
        || snapshot.catalog_scope.regions != 9
        || snapshot.catalog_scope.subregions != 207
        || snapshot.catalog_scope.included_records != snapshot.regions.len()
        || snapshot.regions.len() != 217
        || snapshot.source.commit.len() != 40
        || !snapshot
            .source
            .commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Genshin Region snapshot scope is inconsistent".into());
    }

    let mut keys = HashSet::new();
    let mut indexes = HashSet::new();
    for region in &snapshot.regions {
        if !keys.insert(region.source_key.as_str())
            || !indexes.insert(region.catalog_index)
            || region_type_display_name(&region.subtype).is_none()
        {
            return Err(format!(
                "invalid Genshin Region identity: {}",
                region.source_key
            ));
        }
        for (value, field) in [
            (&region.official_english_name, "English name"),
            (
                &region.official_simplified_chinese_name,
                "Simplified Chinese name",
            ),
            (
                &region.official_traditional_chinese_name,
                "Traditional Chinese name",
            ),
            (&region.official_vietnamese_name, "Vietnamese name"),
            (&region.official_english_description, "English description"),
            (&region.source_table, "source table"),
        ] {
            validate_source_text(value, 10_000, field, &region.source_key)?;
        }
        if region.subtype == "world" {
            if region.parent_source_key.is_some()
                || region.region_id.is_some()
                || region.area_id.is_some()
                || !region.viewpoint_ids.is_empty()
            {
                return Err("the Genshin world anchor has invalid hierarchy data".into());
            }
        } else if region.subtype == "subregion" {
            if region.parent_source_key.is_none()
                || region.region_id.is_none()
                || region.area_id.is_none()
                || region.viewpoint_ids.is_empty()
            {
                return Err(format!(
                    "incomplete Genshin subregion: {}",
                    region.source_key
                ));
            }
        } else if region.parent_source_key.is_none()
            || region.region_id.is_none()
            || region.area_id.is_some()
            || region.viewpoint_ids.is_empty()
        {
            return Err(format!(
                "incomplete Genshin top-level region: {}",
                region.source_key
            ));
        }
    }
    if indexes != (1..=snapshot.regions.len()).collect::<HashSet<_>>() {
        return Err("Genshin Region catalog indexes are not contiguous".into());
    }

    let by_key = snapshot
        .regions
        .iter()
        .map(|region| (region.source_key.as_str(), region))
        .collect::<BTreeMap<_, _>>();
    for region in &snapshot.regions {
        let mut cursor = region;
        let mut ancestry = HashSet::new();
        ancestry.insert(cursor.source_key.as_str());
        while let Some(parent_key) = cursor.parent_source_key.as_deref() {
            if !ancestry.insert(parent_key) {
                return Err(format!("cyclic Genshin Region hierarchy at {parent_key}"));
            }
            cursor = by_key
                .get(parent_key)
                .copied()
                .ok_or_else(|| format!("unknown Genshin parent Region: {parent_key}"))?;
        }
    }
    Ok(())
}

fn validate_region_han_viet_snapshot(
    snapshot: &RegionHanVietSnapshot,
    catalog: &RegionSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.region-han-viet.v1"
        || snapshot.confidence.han_viet_names != "E"
        || snapshot.catalog_scope.included_records != snapshot.regions.len()
        || snapshot.regions.len() != catalog.regions.len()
    {
        return Err("Genshin Region Hán-Việt coverage is incomplete".into());
    }
    let catalog_by_key = catalog
        .regions
        .iter()
        .map(|region| (region.source_key.as_str(), region))
        .collect::<BTreeMap<_, _>>();
    let mut keys = HashSet::new();
    for alias in &snapshot.regions {
        let region = catalog_by_key
            .get(alias.source_key.as_str())
            .ok_or_else(|| {
                format!(
                    "unknown Genshin Region Hán-Việt alias: {}",
                    alias.source_key
                )
            })?;
        if !keys.insert(alias.source_key.as_str())
            || alias.official_english_name != region.official_english_name
            || alias.official_simplified_chinese_name != region.official_simplified_chinese_name
        {
            return Err(format!(
                "invalid Genshin Region Hán-Việt alias: {}",
                alias.source_key
            ));
        }
        validate_source_text(
            &alias.han_viet_name,
            300,
            "generated Region Hán-Việt name",
            &alias.source_key,
        )?;
    }
    Ok(())
}

fn curated_npc_snapshot() -> Result<&'static NpcSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<NpcSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: NpcSnapshot = serde_json::from_str(include_str!("../data/npcs.json"))
            .map_err(|error| format!("invalid bundled Genshin NPC snapshot: {error}"))?;
        let mut keys = HashSet::new();
        if snapshot.schema != "perwiga.genshin-impact.npcs.v1"
            || snapshot.npcs.is_empty()
            || snapshot.npcs.len() != snapshot.catalog_scope.included_records
            || snapshot.npcs.iter().any(|npc| {
                npc.source_key != format!("genshin-fandom-npc-{}", npc.page_id)
                    || !keys.insert(npc.source_key.as_str())
                    || npc.official_english_name.trim().is_empty()
                    || npc.page_url
                        != format!(
                            "https://genshin-impact.fandom.com/wiki/{}",
                            npc.page_title.replace(' ', "_")
                        )
                    || npc.relationships.iter().any(|relation| {
                        !matches!(relation.kind.as_str(), "quest" | "event" | "action")
                            || relation.title.trim().is_empty()
                    })
            })
        {
            return Err("Genshin NPC snapshot scope or identity is inconsistent".into());
        }
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn curated_npc_han_viet_snapshot() -> Result<&'static NpcHanVietSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<NpcHanVietSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot: NpcHanVietSnapshot =
            serde_json::from_str(include_str!("../data/npc-han-viet.json")).map_err(|error| {
                format!("invalid bundled Genshin NPC Hán-Việt snapshot: {error}")
            })?;
        if snapshot.schema != "perwiga.genshin-impact.npc-han-viet.v1"
            || snapshot.confidence.han_viet_names != "E"
            || snapshot.catalog_scope.aliases != snapshot.npcs.len()
        {
            return Err("Genshin NPC Hán-Việt snapshot scope is inconsistent".into());
        }
        let catalog = curated_npc_snapshot().map_err(|error| error.to_string())?;
        let catalog_by_key = catalog
            .npcs
            .iter()
            .map(|npc| (npc.source_key.as_str(), npc))
            .collect::<BTreeMap<_, _>>();
        let mut keys = HashSet::new();
        for alias in &snapshot.npcs {
            let npc = catalog_by_key
                .get(alias.source_key.as_str())
                .ok_or_else(|| format!("unknown NPC Hán-Việt alias {}", alias.source_key))?;
            if !keys.insert(alias.source_key.as_str())
                || alias.official_english_name != npc.official_english_name
                || alias.official_simplified_chinese_name != npc.official_simplified_chinese_name
            {
                return Err(format!("invalid NPC Hán-Việt alias {}", alias.source_key));
            }
            validate_source_text(
                &alias.han_viet_name,
                300,
                "generated NPC Hán-Việt name",
                &alias.source_key,
            )?;
        }
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn validate_skin_snapshot(snapshot: &SkinSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.skins.v1"
        || snapshot.catalog_scope.included_records != snapshot.skins.len()
        || snapshot.catalog_scope.excluded_default_outfits != 120
        || snapshot.catalog_scope.confirmed_weapon_skins != 0
    {
        return Err("Genshin Skin snapshot scope is inconsistent".into());
    }
    let allowed_characters = SKIN_CHARACTER_OPTIONS
        .iter()
        .map(|option| option.value)
        .collect::<HashSet<_>>();
    let allowed_weapon_types = WEAPON_TYPE_OPTIONS
        .iter()
        .map(|option| option.value)
        .collect::<HashSet<_>>();
    let mut keys = HashSet::new();
    let mut ids = HashSet::new();
    for skin in &snapshot.skins {
        if skin.source_key != format!("hoyowiki-outfit-{}", skin.hoyowiki_entry_page_id)
            || !keys.insert(skin.source_key.as_str())
            || !ids.insert(skin.hoyowiki_entry_page_id.as_str())
        {
            return Err(format!(
                "invalid or duplicate Genshin Skin identity: {}",
                skin.source_key
            ));
        }
        for (value, field) in [
            (&skin.official_english_name, "English name"),
            (
                &skin.official_simplified_chinese_name,
                "Simplified Chinese name",
            ),
            (
                &skin.official_traditional_chinese_name,
                "Traditional Chinese name",
            ),
            (&skin.official_vietnamese_name, "Vietnamese name"),
            (&skin.official_english_description, "English description"),
        ] {
            validate_source_text(value, 10_000, field, &skin.source_key)?;
        }
        if !matches!(skin.rarity, 4 | 5)
            || skin.application_kind != "character"
            || skin.applicable_characters.is_empty()
            || skin
                .applicable_characters
                .iter()
                .any(|value| !allowed_characters.contains(value.as_str()))
            || skin
                .applicable_weapon_types
                .iter()
                .any(|value| !allowed_weapon_types.contains(value.as_str()))
            || !skin.applicable_weapons.is_empty()
            || !skin.applicable_weapon_types.is_empty()
            || skin_thumbnail(&skin.source_key).is_none()
        {
            return Err(format!(
                "invalid Genshin Skin filter or asset data: {}",
                skin.source_key
            ));
        }
        let extension = skin
            .thumbnail_asset
            .rsplit_once('.')
            .map(|(_, value)| value);
        if !matches!(extension, Some("png" | "gif"))
            || !skin
                .thumbnail_asset
                .starts_with(&format!("{}.", skin.source_key))
        {
            return Err(format!(
                "invalid Genshin Skin thumbnail: {}",
                skin.source_key
            ));
        }
    }
    Ok(())
}

fn validate_skin_han_viet_snapshot(
    snapshot: &SkinHanVietSnapshot,
    catalog: &SkinSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.skin-han-viet.v1"
        || snapshot.confidence.han_viet_names != "E"
        || snapshot.catalog_scope.included_records != snapshot.skins.len()
        || snapshot.skins.len() != catalog.skins.len()
    {
        return Err("Genshin Skin Hán-Việt coverage is incomplete".into());
    }
    let catalog_by_key = catalog
        .skins
        .iter()
        .map(|skin| (skin.source_key.as_str(), skin))
        .collect::<BTreeMap<_, _>>();
    let mut keys = HashSet::new();
    for alias in &snapshot.skins {
        let skin = catalog_by_key
            .get(alias.source_key.as_str())
            .ok_or_else(|| format!("unknown Genshin Skin Hán-Việt alias: {}", alias.source_key))?;
        if !keys.insert(alias.source_key.as_str())
            || alias.official_english_name != skin.official_english_name
            || alias.official_simplified_chinese_name != skin.official_simplified_chinese_name
        {
            return Err(format!(
                "invalid Genshin Skin Hán-Việt alias: {}",
                alias.source_key
            ));
        }
        validate_source_text(
            &alias.han_viet_name,
            300,
            "generated Skin Hán-Việt name",
            &alias.source_key,
        )?;
    }
    Ok(())
}

fn validate_weapon_han_viet_snapshot(
    snapshot: &WeaponHanVietSnapshot,
    catalog: &WeaponSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.weapon-han-viet.v1" {
        return Err("unexpected Genshin weapon Hán-Việt snapshot schema".into());
    }
    if snapshot.confidence.han_viet_names != "E"
        || snapshot.catalog_scope.included_records != snapshot.weapons.len()
        || snapshot.weapons.len() != catalog.weapons.len()
    {
        return Err("Genshin weapon Hán-Việt coverage is incomplete".into());
    }
    let catalog_by_key = catalog
        .weapons
        .iter()
        .map(|weapon| (weapon.source_key.as_str(), weapon))
        .collect::<BTreeMap<_, _>>();
    let mut source_keys = HashSet::new();
    for alias in &snapshot.weapons {
        if !source_keys.insert(alias.source_key.as_str()) {
            return Err(format!(
                "duplicate Genshin weapon Hán-Việt alias: {}",
                alias.source_key
            ));
        }
        let weapon = catalog_by_key
            .get(alias.source_key.as_str())
            .ok_or_else(|| {
                format!(
                    "unknown Genshin weapon Hán-Việt alias: {}",
                    alias.source_key
                )
            })?;
        if alias.official_english_name != weapon.official_english_name
            || alias.official_simplified_chinese_name != weapon.official_simplified_chinese_name
        {
            return Err(format!(
                "Genshin weapon Hán-Việt source-name mismatch: {}",
                alias.source_key
            ));
        }
        validate_source_text(
            &alias.han_viet_name,
            300,
            "generated weapon Hán-Việt name",
            &alias.source_key,
        )?;
    }
    Ok(())
}

fn validate_weapon_snapshot(snapshot: &WeaponSnapshot) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.weapons.v1" {
        return Err("unexpected Genshin weapon snapshot schema".into());
    }
    if snapshot.catalog_scope.included_records != snapshot.weapons.len()
        || snapshot.catalog_scope.excluded_non_obtainable_records != 3
    {
        return Err("Genshin weapon snapshot scope is inconsistent".into());
    }
    if snapshot.source.commit.len() != 40
        || !snapshot
            .source
            .commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Genshin weapon snapshot has an invalid source commit".into());
    }
    let allowed_types = WEAPON_TYPE_OPTIONS
        .iter()
        .map(|option| option.value)
        .collect::<HashSet<_>>();
    let allowed_base_attack = WEAPON_MAIN_STAT_OPTIONS
        .iter()
        .map(|option| option.value)
        .collect::<HashSet<_>>();
    let allowed_substats = WEAPON_SUBSTAT_OPTIONS
        .iter()
        .map(|option| option.value)
        .collect::<HashSet<_>>();
    let allowed_regions = REGION_OPTIONS
        .iter()
        .map(|option| option.value)
        .collect::<HashSet<_>>();
    let mut source_keys = HashSet::new();
    let mut game_data_ids = HashSet::new();
    for weapon in &snapshot.weapons {
        if weapon.source_key != format!("genshin-data-weapon-{}", weapon.game_data_id)
            || !source_keys.insert(weapon.source_key.as_str())
            || !game_data_ids.insert(weapon.game_data_id)
        {
            return Err(format!(
                "invalid or duplicate Genshin weapon identity: {}",
                weapon.source_key
            ));
        }
        for (value, field) in [
            (&weapon.genshin_db_key, "genshin-db key"),
            (&weapon.official_english_name, "English name"),
            (
                &weapon.official_simplified_chinese_name,
                "Simplified Chinese name",
            ),
            (
                &weapon.official_traditional_chinese_name,
                "Traditional Chinese name",
            ),
            (&weapon.official_vietnamese_name, "Vietnamese name"),
            (&weapon.official_english_description, "English description"),
            (&weapon.ascension_material, "ascension material"),
            (&weapon.release_version, "release version"),
        ] {
            validate_source_text(value, 10_000, field, &weapon.source_key)?;
        }
        if !matches!(weapon.rarity, 1..=5)
            || !allowed_types.contains(weapon.weapon_type.as_str())
            || !allowed_base_attack.contains(weapon.base_attack.to_string().as_str())
            || !allowed_substats.contains(weapon.substat.as_str())
            || !allowed_regions.contains(weapon.ascension_region.as_str())
        {
            return Err(format!(
                "invalid Genshin weapon filter data: {}",
                weapon.source_key
            ));
        }
        if let Some(value) = &weapon.base_substat_value {
            validate_source_text(value, 40, "base substat value", &weapon.source_key)?;
        }
        if weapon.thumbnail_asset != format!("{}.png", weapon.source_key)
            || weapon_thumbnail(&weapon.source_key).is_none()
        {
            return Err(format!(
                "missing trusted local weapon thumbnail for {}",
                weapon.source_key
            ));
        }
    }
    Ok(())
}

fn validate_weapon_thumbnail_snapshot(
    snapshot: &WeaponThumbnailSnapshot,
    catalog: &WeaponSnapshot,
) -> Result<(), String> {
    if snapshot.schema != "perwiga.genshin-impact.weapon-thumbnails-hoyowiki.v1"
        || snapshot.catalog_scope.catalog_records != catalog.weapons.len()
        || snapshot.catalog_scope.wiki_matches != snapshot.weapons.len()
        || snapshot.weapons.len() + snapshot.catalog_scope.missing_from_hoyowiki.len()
            != catalog.weapons.len()
    {
        return Err("Genshin HoYoWiki weapon-thumbnail coverage is inconsistent".into());
    }
    let catalog_by_key = catalog
        .weapons
        .iter()
        .map(|weapon| (weapon.source_key.as_str(), weapon))
        .collect::<BTreeMap<_, _>>();
    let mut covered = HashSet::new();
    for thumbnail in &snapshot.weapons {
        let weapon = catalog_by_key
            .get(thumbnail.source_key.as_str())
            .ok_or_else(|| {
                format!(
                    "unknown HoYoWiki weapon thumbnail: {}",
                    thumbnail.source_key
                )
            })?;
        if !covered.insert(thumbnail.source_key.as_str())
            || thumbnail.official_english_name != weapon.official_english_name
            || thumbnail.thumbnail_asset != weapon.thumbnail_asset
            || weapon_hoyowiki_thumbnail(&thumbnail.source_key).is_none()
        {
            return Err(format!(
                "invalid HoYoWiki weapon thumbnail: {}",
                thumbnail.source_key
            ));
        }
    }
    for source_key in &snapshot.catalog_scope.missing_from_hoyowiki {
        if !catalog_by_key.contains_key(source_key.as_str())
            || !covered.insert(source_key)
            || weapon_hoyowiki_thumbnail(source_key).is_some()
            || weapon_fallback_thumbnail(source_key).is_none()
        {
            return Err(format!(
                "invalid HoYoWiki weapon-thumbnail gap: {source_key}"
            ));
        }
    }
    if covered.len() != catalog.weapons.len() {
        return Err("Genshin HoYoWiki weapon-thumbnail coverage is incomplete".into());
    }
    Ok(())
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

fn weapon_accent(rarity: u8) -> Option<&'static str> {
    match rarity {
        1 => Some(ONE_STAR_COLOR),
        2 => Some(TWO_STAR_COLOR),
        3 => Some(THREE_STAR_COLOR),
        4 => Some(FOUR_STAR_COLOR),
        5 => Some(FIVE_STAR_COLOR),
        _ => None,
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

fn curated_weapon(source_key: &str) -> Option<&'static CuratedWeapon> {
    curated_weapon_snapshot()
        .ok()?
        .weapons
        .iter()
        .find(|weapon| weapon.source_key == source_key)
}

fn weapon_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "weapon" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("GenshinData weapon source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_weapon(source_key).map(|weapon| weapon.source_key.as_str())
}

fn curated_skin(source_key: &str) -> Option<&'static CuratedSkin> {
    curated_skin_snapshot()
        .ok()?
        .skins
        .iter()
        .find(|skin| skin.source_key == source_key)
}

fn skin_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "skin" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("Official HoYoWiki Skin source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_skin(source_key).map(|skin| skin.source_key.as_str())
}

fn artifact_source_key(entity: &WikiEntity) -> Option<&str> {
    if !matches!(
        entity.entity_type.as_str(),
        "artifact-set" | "artifact-piece"
    ) {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("GenshinData Artifact source key: ")?;
    Some(suffix.split('.').next()?.trim())
}

fn artifact_domain_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "domain" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("GenshinData Artifact Domain source key: ")?;
    Some(suffix.split('.').next()?.trim())
}

fn curated_region(source_key: &str) -> Option<&'static CuratedRegion> {
    curated_region_snapshot()
        .ok()?
        .regions
        .iter()
        .find(|region| region.source_key == source_key)
}

fn region_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "region" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("GenshinData Region source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_region(source_key).map(|region| region.source_key.as_str())
}

fn region_parent(region: &CuratedRegion) -> Option<&'static CuratedRegion> {
    curated_region(region.parent_source_key.as_deref()?)
}

fn npc_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "npc" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("Genshin Fandom NPC source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_npc_snapshot()
        .ok()?
        .npcs
        .iter()
        .find(|npc| npc.source_key == source_key)
        .map(|npc| npc.source_key.as_str())
}

fn artifact_aliases(
    record: &CuratedArtifactHanViet,
    traditional: &str,
    checked_at: &str,
) -> Vec<AliasInput> {
    vec![
        AliasInput {
            value: traditional.to_string(),
            language: Some("zh-Hant".to_string()),
            kind: "official-localization".to_string(),
            label: Some("In-game Traditional Chinese".to_string()),
            notes: Some(format!("Traditional Chinese in-game Artifact localization from the pinned genshin-db snapshot; checked {checked_at}.")),
        },
        AliasInput {
            value: record.han_viet_name.clone(),
            language: Some("vi".to_string()),
            kind: "han-viet".to_string(),
            label: Some("Hán-Việt (generated)".to_string()),
            notes: Some(format!("Generated search alias from the extracted Simplified Chinese Artifact name; not an official Vietnamese localization. Sources: {WEAPON_SOURCE_URL} and {HAN_VIET_DICTIONARY_URL}. Checked {checked_at}. Confidence E (generated).")),
        },
    ]
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
    let han_viet_snapshot = curated_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .characters
        .iter()
        .map(|alias| (alias.source_key.as_str(), alias))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .characters
        .iter()
        .map(|character| {
            let mut aliases = character
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
            let han_viet = han_viet_by_key
                .get(character.source_key.as_str())
                .expect("validated Hán-Việt coverage");
            aliases.push(AliasInput {
                value: han_viet.han_viet_name.clone(),
                language: Some("vi".to_string()),
                kind: "han-viet".to_string(),
                label: Some("Hán-Việt (generated)".to_string()),
                notes: Some(format!(
                    "Generated search alias from the verified official Traditional Chinese localization; not an official Vietnamese localization and not evidence of the original mainland Simplified Chinese name. Sources: {SOURCE_URL} and {HAN_VIET_DICTIONARY_URL}. Checked {}. Confidence E (generated).",
                    han_viet_snapshot.checked_at
                )),
            });
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

pub fn import_curated_regions(
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

    let snapshot = curated_region_snapshot()?;
    let han_viet_snapshot = curated_region_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .regions
        .iter()
        .map(|alias| (alias.source_key.as_str(), alias))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .regions
        .iter()
        .map(|region| {
            let han_viet = han_viet_by_key
                .get(region.source_key.as_str())
                .expect("validated Region Hán-Việt coverage");
            let parent = region_parent(region)
                .map(|parent| parent.official_english_name.as_str())
                .unwrap_or("None");
            SourcedEntityInput {
                source_provider: format!(
                    "{REGION_SOURCE_PROVIDER}@{}",
                    snapshot.source.commit
                ),
                source_identity: region.source_key.clone(),
                entity: EntityInput {
                    entity_type: "region".to_string(),
                    official_english_name: region.official_english_name.clone(),
                    official_original_name: region.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: Some(region.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: Some(region.official_english_description.clone()),
                    other_information: Some(format!(
                        "GenshinData Region source key: {}. Region type: {}. Parent Region: {}. Region ID: {}. Area ID: {}. Geography viewpoint IDs: {}. Source table: {}. Source: {} at commit {}. Checked {}.",
                        region.source_key,
                        region_type_display_name(&region.subtype)
                            .expect("validated Region subtype"),
                        parent,
                        region
                            .region_id
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "None".to_string()),
                        region
                            .area_id
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "None".to_string()),
                        region
                            .viewpoint_ids
                            .iter()
                            .map(u64::to_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                        region.source_table,
                        WEAPON_SOURCE_URL,
                        snapshot.source.commit,
                        snapshot.checked_at,
                    )),
                },
                aliases: vec![
                    AliasInput {
                        value: region.official_traditional_chinese_name.clone(),
                        language: Some("zh-Hant".to_string()),
                        kind: "official-localization".to_string(),
                        label: Some("In-game Traditional Chinese".to_string()),
                        notes: Some(format!(
                            "Traditional Chinese in-game Geography localization from the pinned genshin-db snapshot; checked {}.",
                            snapshot.checked_at
                        )),
                    },
                    AliasInput {
                        value: han_viet.han_viet_name.clone(),
                        language: Some("vi".to_string()),
                        kind: "han-viet".to_string(),
                        label: Some("Hán-Việt (generated)".to_string()),
                        notes: Some(format!(
                            "Generated search alias from the extracted Simplified Chinese Region name; not an official Vietnamese localization. Sources: {WEAPON_SOURCE_URL} and {HAN_VIET_DICTIONARY_URL}. Checked {}. Confidence E (generated).",
                            han_viet_snapshot.checked_at
                        )),
                    },
                ],
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_npcs(
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
    let snapshot = curated_npc_snapshot()?;
    let han_viet = curated_npc_han_viet_snapshot()?;
    let aliases = han_viet
        .npcs
        .iter()
        .map(|alias| (alias.source_key.as_str(), alias))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .npcs
        .iter()
        .map(|npc| {
            let mut entity_aliases = Vec::new();
            if !npc.official_traditional_chinese_name.is_empty() {
                entity_aliases.push(AliasInput {
                    value: npc.official_traditional_chinese_name.clone(),
                    language: Some("zh-Hant".into()),
                    kind: "official-localization".into(),
                    label: Some("In-game Traditional Chinese".into()),
                    notes: Some(format!("NPC page localization from {}. Checked {}.", npc.page_url, snapshot.checked_at)),
                });
            }
            if let Some(alias) = aliases.get(npc.source_key.as_str()) {
                entity_aliases.push(AliasInput {
                    value: alias.han_viet_name.clone(),
                    language: Some("vi".into()),
                    kind: "han-viet".into(),
                    label: Some("Hán-Việt (generated)".into()),
                    notes: Some("Generated confidence-E search alias; not official Vietnamese localization.".into()),
                });
            }
            SourcedEntityInput {
                source_provider: "Genshin Impact Wiki NPC catalog".into(),
                source_identity: npc.source_key.clone(),
                entity: EntityInput {
                    entity_type: "npc".into(),
                    official_english_name: npc.official_english_name.clone(),
                    official_original_name: if npc.official_simplified_chinese_name.is_empty() { npc.official_english_name.clone() } else { npc.official_simplified_chinese_name.clone() },
                    official_vietnamese_name: (!npc.official_vietnamese_name.is_empty()).then(|| npc.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: (!npc.official_english_description.is_empty()).then(|| npc.official_english_description.clone()),
                    other_information: Some(format!("Genshin Fandom NPC source key: {}. Page ID: {}. Regions: {}. Locations: {}. Categories: {}. Source: {}. Checked {}.", npc.source_key, npc.page_id, npc.regions.join(", "), npc.locations.join(", "), npc.categories.join(", "), npc.page_url, snapshot.checked_at)),
                },
                aliases: entity_aliases,
            }
        })
        .collect::<Vec<_>>();
    let summary = store.import_sourced_entities(work_id, &inputs)?;
    let entities = store.list_entities(work_id, Some("npc"))?;
    let by_source = entities
        .iter()
        .filter_map(|entity| npc_source_key(entity).map(|key| (key, entity.id.clone())))
        .collect::<BTreeMap<_, _>>();
    let appearances = snapshot
        .npcs
        .iter()
        .flat_map(|npc| {
            let Some(entity_id) = by_source.get(npc.source_key.as_str()) else {
                return Vec::new();
            };
            npc.relationships
                .iter()
                .map(|relation| EntityAppearanceInput {
                    entity_id: entity_id.clone(),
                    relation_kind: relation.kind.clone(),
                    related_title: relation.title.clone(),
                    related_source_url: Some(format!(
                        "https://genshin-impact.fandom.com/wiki/{}",
                        relation.title.replace(' ', "_")
                    )),
                    source_provider: "Genshin Impact Wiki NPC catalog".into(),
                    source_identity: format!(
                        "{}:{}:{}",
                        npc.source_key, relation.kind, relation.title
                    ),
                    source_notes: Some(format!(
                        "{} relationship from {}.",
                        relation.source, npc.page_url
                    )),
                    locations: relation
                        .locations
                        .iter()
                        .map(|location| EntityAppearanceLocationInput {
                            location_name: location.clone(),
                            region_name: relation.regions.first().cloned(),
                        })
                        .collect(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    store.import_entity_appearances(&appearances)?;
    Ok(summary)
}

pub fn import_curated_weapons(
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

    let snapshot = curated_weapon_snapshot()?;
    let han_viet_snapshot = curated_weapon_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .weapons
        .iter()
        .map(|alias| (alias.source_key.as_str(), alias))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .weapons
        .iter()
        .map(|weapon| {
            let han_viet = han_viet_by_key
                .get(weapon.source_key.as_str())
                .expect("validated weapon Hán-Việt coverage");
            SourcedEntityInput {
            source_provider: WEAPON_SOURCE_PROVIDER.to_string(),
            source_identity: weapon.source_key.clone(),
            entity: EntityInput {
                entity_type: "weapon".to_string(),
                official_english_name: weapon.official_english_name.clone(),
                official_original_name: weapon.official_simplified_chinese_name.clone(),
                official_vietnamese_name: Some(weapon.official_vietnamese_name.clone()),
                automatic_vietnamese_translation: None,
                english_description: Some(weapon.official_english_description.clone()),
                other_information: Some(format!(
                    "GenshinData weapon source key: {}. Game-data ID: {}. Weapon type: {}. Rarity: {}★. Base ATK at level 1: {}. Substat: {}{}. Ascension region: {} (derived from {}). Release version: {}. Source: {} at commit {}. Checked {}. The ascension region is a material-domain classification, not a claim about the weapon's lore origin.",
                    weapon.source_key,
                    weapon.game_data_id,
                    weapon.weapon_type,
                    weapon.rarity,
                    weapon.base_attack,
                    weapon.substat,
                    weapon
                        .base_substat_value
                        .as_ref()
                        .map(|value| format!(" ({value} at level 1)"))
                        .unwrap_or_default(),
                    weapon.ascension_region,
                    weapon.ascension_material,
                    weapon.release_version,
                    WEAPON_SOURCE_URL,
                    snapshot.source.commit,
                    snapshot.checked_at,
                )),
            },
                aliases: vec![
                    AliasInput {
                        value: weapon.official_traditional_chinese_name.clone(),
                        language: Some("zh-Hant".to_string()),
                        kind: "official-localization".to_string(),
                        label: Some("In-game Traditional Chinese".to_string()),
                        notes: Some(format!(
                            "Traditional Chinese in-game localization from GenshinData via genshin-db. Source commit {}; checked {}.",
                            snapshot.source.commit,
                            snapshot.checked_at,
                        )),
                    },
                    AliasInput {
                        value: han_viet.han_viet_name.clone(),
                        language: Some("vi".to_string()),
                        kind: "han-viet".to_string(),
                        label: Some("Hán-Việt (generated)".to_string()),
                        notes: Some(format!(
                            "Generated search alias from the extracted Simplified Chinese original name; not an official Vietnamese localization. Sources: {} and {}. Checked {}. Confidence E (generated).",
                            WEAPON_SOURCE_URL,
                            HAN_VIET_DICTIONARY_URL,
                            han_viet_snapshot.checked_at,
                        )),
                    },
                ],
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_skins(
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
    let snapshot = curated_skin_snapshot()?;
    let han_viet_snapshot = curated_skin_han_viet_snapshot()?;
    let han_viet_by_key = han_viet_snapshot
        .skins
        .iter()
        .map(|alias| (alias.source_key.as_str(), alias))
        .collect::<BTreeMap<_, _>>();
    let inputs = snapshot
        .skins
        .iter()
        .map(|skin| {
            let han_viet = han_viet_by_key
                .get(skin.source_key.as_str())
                .expect("validated Skin Hán-Việt coverage");
            SourcedEntityInput {
                source_provider: SKIN_SOURCE_PROVIDER.to_string(),
                source_identity: skin.source_key.clone(),
                entity: EntityInput {
                    entity_type: "skin".to_string(),
                    official_english_name: skin.official_english_name.clone(),
                    official_original_name: skin.official_simplified_chinese_name.clone(),
                    official_vietnamese_name: Some(skin.official_vietnamese_name.clone()),
                    automatic_vietnamese_translation: None,
                    english_description: Some(skin.official_english_description.clone()),
                    other_information: Some(format!(
                        "Official HoYoWiki Skin source key: {}. HoYoWiki entry page ID: {}. Applies to {}: {}. Rarity: {}★. Source: {}. Checked {}. The current official Outfit catalog contains no confirmed weapon or weapon-type skins.",
                        skin.source_key,
                        skin.hoyowiki_entry_page_id,
                        skin.application_kind,
                        skin.applicable_characters.join(", "),
                        skin.rarity,
                        SKIN_SOURCE_URL,
                        snapshot.checked_at,
                    )),
                },
                aliases: vec![
                    AliasInput {
                        value: skin.official_traditional_chinese_name.clone(),
                        language: Some("zh-Hant".to_string()),
                        kind: "official-localization".to_string(),
                        label: Some("In-game Traditional Chinese".to_string()),
                        notes: Some(format!(
                            "Traditional Chinese in-game Outfit localization from the pinned genshin-db snapshot; checked {}.",
                            snapshot.checked_at,
                        )),
                    },
                    AliasInput {
                        value: han_viet.han_viet_name.clone(),
                        language: Some("vi".to_string()),
                        kind: "han-viet".to_string(),
                        label: Some("Hán-Việt (generated)".to_string()),
                        notes: Some(format!(
                            "Generated search alias from the extracted Simplified Chinese Skin name; not an official Vietnamese localization. Sources: {} and {}. Checked {}. Confidence E (generated).",
                            WEAPON_SOURCE_URL,
                            HAN_VIET_DICTIONARY_URL,
                            han_viet_snapshot.checked_at,
                        )),
                    },
                ],
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_artifacts(
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
    let snapshot = curated_artifact_snapshot()?;
    let han_viet = curated_artifact_han_viet_snapshot()?;
    let aliases = han_viet
        .artifact_sets
        .iter()
        .chain(han_viet.artifact_pieces.iter())
        .map(|alias| (alias.source_key.as_str(), alias))
        .collect::<BTreeMap<_, _>>();
    let mut inputs =
        Vec::with_capacity(snapshot.artifact_sets.len() + snapshot.artifact_pieces.len());
    for set in &snapshot.artifact_sets {
        let effects = [
            set.effect_1_piece
                .as_ref()
                .map(|value| format!("1-piece: {value}")),
            set.effect_2_piece
                .as_ref()
                .map(|value| format!("2-piece: {value}")),
            set.effect_4_piece
                .as_ref()
                .map(|value| format!("4-piece: {value}")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
        let alias = aliases
            .get(set.source_key.as_str())
            .expect("validated Artifact Hán-Việt coverage");
        inputs.push(SourcedEntityInput {
            source_provider: ARTIFACT_SOURCE_PROVIDER.to_string(),
            source_identity: set.source_key.clone(),
            entity: EntityInput {
                entity_type: "artifact-set".to_string(),
                official_english_name: set.official_english_name.clone(),
                official_original_name: set.official_simplified_chinese_name.clone(),
                official_vietnamese_name: Some(set.official_vietnamese_name.clone()),
                automatic_vietnamese_translation: None,
                english_description: (!effects.is_empty()).then_some(effects),
                other_information: Some(format!("GenshinData Artifact source key: {}. Game-data set ID: {}. HoYoWiki entry page ID: {}. Rarity: {}–{}★. Pieces: {}. Effect tags: {}. Sources: {} and {}. Checked {}.", set.source_key, set.game_data_id, set.hoyowiki_entry_page_id, set.minimum_rarity, set.maximum_rarity, set.artifact_piece_source_keys.len(), set.effect_tags.join(", "), WEAPON_SOURCE_URL, ARTIFACT_SOURCE_URL, snapshot.checked_at)),
            },
            aliases: artifact_aliases(alias, &set.official_traditional_chinese_name, &han_viet.checked_at),
        });
    }
    for piece in &snapshot.artifact_pieces {
        let alias = aliases
            .get(piece.source_key.as_str())
            .expect("validated Artifact Hán-Việt coverage");
        inputs.push(SourcedEntityInput {
            source_provider: ARTIFACT_SOURCE_PROVIDER.to_string(),
            source_identity: piece.source_key.clone(),
            entity: EntityInput {
                entity_type: "artifact-piece".to_string(),
                official_english_name: piece.official_english_name.clone(),
                official_original_name: piece.official_simplified_chinese_name.clone(),
                official_vietnamese_name: Some(piece.official_vietnamese_name.clone()),
                automatic_vietnamese_translation: None,
                english_description: Some(piece.official_english_description.clone()),
                other_information: Some(format!("GenshinData Artifact source key: {}. Artifact Set: {} ({}). Slot: {}. Rarity: {}–{}★. Sources: {} and {}. Checked {}.\n\nLore:\n{}", piece.source_key, piece.artifact_set_english_name, piece.artifact_set_source_key, piece.slot, piece.minimum_rarity, piece.maximum_rarity, WEAPON_SOURCE_URL, ARTIFACT_SOURCE_URL, snapshot.checked_at, piece.official_english_story)),
            },
            aliases: artifact_aliases(alias, &piece.official_traditional_chinese_name, &han_viet.checked_at),
        });
    }
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_artifact_domains(
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
    let snapshot = curated_artifact_domain_snapshot()?;
    let inputs = snapshot.domains.iter().map(|domain| SourcedEntityInput {
        source_provider: ARTIFACT_DOMAIN_SOURCE_PROVIDER.to_string(),
        source_identity: domain.source_key.clone(),
        entity: EntityInput {
            entity_type: "domain".to_string(),
            official_english_name: domain.official_english_name.clone(),
            official_original_name: domain.official_simplified_chinese_name.clone(),
            official_vietnamese_name: Some(domain.official_vietnamese_name.clone()),
            automatic_vietnamese_translation: None,
            english_description: Some(domain.official_english_description.clone()),
            other_information: Some(format!("GenshinData Artifact Domain source key: {}. Entrance ID: {}. Region: {}. Minimum Adventure Rank: {}. Challenge stage IDs: {}. Challenge stage names: {}. Recommended levels: {}. Recommended elements: {}. Hosted Artifact Sets: {}. Hosted Artifact source keys: {}. Source: {}. Checked {}.", domain.source_key, domain.game_data_entrance_id, domain.region, domain.minimum_unlock_rank, domain.challenge_stage_ids.iter().map(u64::to_string).collect::<Vec<_>>().join(", "), domain.challenge_stage_names.join(", "), domain.recommended_levels.iter().map(u64::to_string).collect::<Vec<_>>().join(", "), domain.recommended_elements.join(", "), domain.hosted_artifact_set_names.join(", "), domain.hosted_artifact_set_source_keys.join(", "), WEAPON_SOURCE_URL, snapshot.checked_at)),
        },
        aliases: vec![AliasInput { value: domain.official_traditional_chinese_name.clone(), language: Some("zh-Hant".to_string()), kind: "official-localization".to_string(), label: Some("In-game Traditional Chinese".to_string()), notes: Some(format!("Traditional Chinese in-game Domain localization from the pinned genshin-db snapshot; checked {}.", snapshot.checked_at)) }],
    }).collect::<Vec<_>>();
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
        match entity.entity_type.as_str() {
            "character" => {
                let source_key = character_source_key(entity)?;
                let character = curated_character(source_key)?;
                let presentation = curated_character_presentation(source_key)?;
                let region_asset_key = region_asset_key(&character.region)?;
                region_icon(region_asset_key)?;
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/genshin-impact/characters/{source_key}.png"
                    ),
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
            "weapon" => {
                let source_key = weapon_source_key(entity)?;
                let weapon = curated_weapon(source_key)?;
                weapon_thumbnail(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/genshin-impact/weapons/{source_key}.png"
                    ),
                    accent_color: weapon_accent(weapon.rarity)?.to_string(),
                    context_label: Some(weapon.weapon_type.clone()),
                    context_icon_url: None,
                    label: format!("{}★", weapon.rarity),
                    rarity: Some(weapon.rarity),
                    facets: BTreeMap::from([
                        ("weapon_type".to_string(), weapon.weapon_type.clone()),
                        ("main_stat".to_string(), weapon.base_attack.to_string()),
                        ("substat".to_string(), weapon.substat.clone()),
                        (
                            "ascension_region".to_string(),
                            weapon.ascension_region.clone(),
                        ),
                    ]),
                    facet_values: BTreeMap::new(),
                })
            }
            "skin" => {
                let source_key = skin_source_key(entity)?;
                let skin = curated_skin(source_key)?;
                skin_thumbnail(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/genshin-impact/skins/{}",
                        skin.thumbnail_asset
                    ),
                    accent_color: weapon_accent(skin.rarity)?.to_string(),
                    context_label: Some(skin.applicable_characters.join(" · ")),
                    context_icon_url: None,
                    label: format!("{}★", skin.rarity),
                    rarity: Some(skin.rarity),
                    facets: BTreeMap::from([(
                        "skin_application".to_string(),
                        "Character".to_string(),
                    )]),
                    facet_values: BTreeMap::from([(
                        "skin_character".to_string(),
                        skin.applicable_characters.clone(),
                    )]),
                })
            }
            "artifact-set" => {
                let source_key = artifact_source_key(entity)?;
                let artifact = curated_artifact_snapshot()
                    .ok()?
                    .artifact_sets
                    .iter()
                    .find(|item| item.source_key == source_key)?;
                artifact_thumbnail(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/genshin-impact/artifacts/{}",
                        artifact.thumbnail_asset
                    ),
                    accent_color: weapon_accent(artifact.maximum_rarity)?.to_string(),
                    context_label: Some(format!(
                        "{} pieces",
                        artifact.artifact_piece_source_keys.len()
                    )),
                    context_icon_url: None,
                    label: format!("{}–{}★", artifact.minimum_rarity, artifact.maximum_rarity),
                    rarity: Some(artifact.maximum_rarity),
                    facets: BTreeMap::new(),
                    facet_values: BTreeMap::new(),
                })
            }
            "artifact-piece" => {
                let source_key = artifact_source_key(entity)?;
                let artifact = curated_artifact_snapshot()
                    .ok()?
                    .artifact_pieces
                    .iter()
                    .find(|item| item.source_key == source_key)?;
                artifact_thumbnail(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/genshin-impact/artifacts/{}",
                        artifact.thumbnail_asset
                    ),
                    accent_color: weapon_accent(artifact.maximum_rarity)?.to_string(),
                    context_label: Some(artifact.artifact_set_english_name.clone()),
                    context_icon_url: None,
                    label: format!("{}–{}★", artifact.minimum_rarity, artifact.maximum_rarity),
                    rarity: Some(artifact.maximum_rarity),
                    facets: BTreeMap::from([("artifact_slot".to_string(), artifact.slot.clone())]),
                    facet_values: BTreeMap::new(),
                })
            }
            "domain" => {
                let source_key = artifact_domain_source_key(entity)?;
                let domain = curated_artifact_domain_snapshot()
                    .ok()?
                    .domains
                    .iter()
                    .find(|item| item.source_key == source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: "/assets/placeholders/place.svg".to_string(),
                    accent_color: THEME.accent.to_string(),
                    context_label: Some(domain.region.clone()),
                    context_icon_url: None,
                    label: "DOMAIN".to_string(),
                    rarity: None,
                    facets: BTreeMap::from([("domain_region".to_string(), domain.region.clone())]),
                    facet_values: BTreeMap::new(),
                })
            }
            "region" => {
                let source_key = region_source_key(entity)?;
                let region = curated_region(source_key)?;
                let region_type = region_type_display_name(&region.subtype)?;
                let parent = region_parent(region);
                let mut facets =
                    BTreeMap::from([("region_type".to_string(), region_type.to_string())]);
                if let Some(parent) = parent {
                    facets.insert(
                        "parent_region".to_string(),
                        parent.official_english_name.clone(),
                    );
                }
                Some(EntityPresentation {
                    thumbnail_url: "/assets/placeholders/place.svg".to_string(),
                    accent_color: THEME.accent.to_string(),
                    context_label: parent.map(|value| value.official_english_name.clone()),
                    context_icon_url: None,
                    label: region_type.to_uppercase(),
                    rarity: None,
                    facets,
                    facet_values: BTreeMap::new(),
                })
            }
            _ => None,
        }
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
include!(concat!(env!("OUT_DIR"), "/weapon_hoyowiki_thumbnails.rs"));
include!(concat!(env!("OUT_DIR"), "/weapon_fallback_thumbnails.rs"));
include!(concat!(env!("OUT_DIR"), "/skin_png_thumbnails.rs"));
include!(concat!(env!("OUT_DIR"), "/skin_gif_thumbnails.rs"));
include!(concat!(env!("OUT_DIR"), "/artifact_thumbnails.rs"));

pub fn weapon_thumbnail(source_key: &str) -> Option<&'static [u8]> {
    weapon_hoyowiki_thumbnail(source_key).or_else(|| weapon_fallback_thumbnail(source_key))
}

pub fn skin_thumbnail(source_key: &str) -> Option<&'static [u8]> {
    skin_png_thumbnail(source_key).or_else(|| skin_gif_thumbnail(source_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use perwiga_core::Store;

    #[test]
    fn curated_region_import_is_complete_idempotent_and_preserves_hierarchy() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("Genshin work");

        let first = import_curated_regions(&mut store, &work.id).expect("first Region import");
        assert_eq!(first.inserted, 217);
        assert_eq!(first.aliases_inserted, 434);

        let regions = store
            .list_entities(&work.id, Some("region"))
            .expect("Region list");
        assert_eq!(regions.len(), 217);
        let liyue_harbor = regions
            .iter()
            .find(|region| region.official_english_name == "Liyue Harbor")
            .expect("Liyue Harbor");
        assert_eq!(liyue_harbor.official_original_name, "璃月港");
        assert!(store
            .list_aliases(&liyue_harbor.id)
            .expect("Region aliases")
            .iter()
            .any(|alias| alias.kind == "han-viet" && alias.value == "Ly Nguyệt Cảng"));

        let presentation = module()
            .entity_presentation(liyue_harbor)
            .expect("Region presentation");
        assert_eq!(presentation.label, "SUBREGION");
        assert_eq!(presentation.context_label.as_deref(), Some("Liyue"));
        assert_eq!(presentation.facets["region_type"], "Subregion");
        assert_eq!(presentation.facets["parent_region"], "Liyue");
        assert_eq!(presentation.rarity, None);

        let second = import_curated_regions(&mut store, &work.id).expect("second Region import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 217);
        assert_eq!(second.aliases_inserted, 0);
        assert_eq!(second.aliases_unchanged, 434);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
    }

    #[test]
    fn curated_npc_import_separates_appearances_and_preserves_many_locations() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("Genshin work");
        let summary = import_curated_npcs(&mut store, &work.id).expect("NPC import");
        assert_eq!(summary.inserted, 4_967);
        assert_eq!(summary.aliases_inserted, 6_771);
        let npcs = store
            .list_entities(&work.id, Some("npc"))
            .expect("NPC list");
        assert_eq!(npcs.len(), 4_967);
        let liben = npcs
            .iter()
            .find(|npc| npc.official_english_name == "Liben")
            .expect("Liben");
        assert_eq!(liben.official_original_name, "立本");
        let liben_appearances = store
            .list_entity_appearances(&liben.id)
            .expect("Liben appearances");
        assert_eq!(liben_appearances.len(), 1);
        assert_eq!(liben_appearances[0].relation_kind, "event");
        assert_eq!(liben_appearances[0].related_title, "Marvelous Merchandise");
        assert_eq!(liben_appearances[0].locations.len(), 2);
        let aarav = npcs
            .iter()
            .find(|npc| npc.official_english_name == "Aarav")
            .expect("Aarav");
        let appearances = store
            .list_entity_appearances(&aarav.id)
            .expect("Aarav appearances");
        assert_eq!(appearances.len(), 1);
        assert_eq!(appearances[0].relation_kind, "quest");
        assert_eq!(appearances[0].related_title, "The Illusions of the Mob");
        assert_eq!(appearances[0].locations[0].location_name, "Sumeru");
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
    }

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
            vec![
                "character",
                "npc",
                "region",
                "weapon",
                "skin",
                "artifact-set",
                "artifact-piece",
                "domain",
            ]
        );
    }

    #[test]
    fn curated_skin_import_is_complete_idempotent_and_filterable() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("Genshin work");
        let first = import_curated_skins(&mut store, &work.id).expect("first Skin import");
        assert_eq!(first.inserted, 29);
        assert_eq!(first.aliases_inserted, 58);
        let skins = store
            .list_entities(&work.id, Some("skin"))
            .expect("Skin list");
        assert_eq!(skins.len(), 29);

        let diluc = skins
            .iter()
            .find(|skin| skin.official_english_name == "Red Dead of Night")
            .expect("Diluc Skin");
        let aliases = store.list_aliases(&diluc.id).expect("Skin aliases");
        assert!(aliases.iter().any(|alias| {
            alias.value == "Ân Hồng Chung Dạ"
                && alias.kind == "han-viet"
                && alias.language.as_deref() == Some("vi")
        }));
        let presentation = module()
            .entity_presentation(diluc)
            .expect("Skin presentation");
        assert_eq!(presentation.rarity, Some(5));
        assert_eq!(presentation.accent_color, FIVE_STAR_COLOR);
        assert_eq!(presentation.context_label.as_deref(), Some("Diluc"));
        assert_eq!(presentation.facets["skin_application"], "Character");
        assert_eq!(presentation.facet_values["skin_character"], ["Diluc"]);
        assert!(skin_thumbnail("hoyowiki-outfit-5777").is_some());

        let traveler = skins
            .iter()
            .find(|skin| skin.official_english_name == "As Heaven and Earth Are Made Anew")
            .expect("Traveler Skin");
        let traveler_presentation = module()
            .entity_presentation(traveler)
            .expect("Traveler Skin presentation");
        assert_eq!(traveler_presentation.rarity, Some(4));
        assert_eq!(traveler_presentation.accent_color, FOUR_STAR_COLOR);
        assert_eq!(
            traveler_presentation.facet_values["skin_character"],
            ["Aether", "Lumine"]
        );
        assert!(traveler_presentation.thumbnail_url.ends_with(".gif"));

        let second = import_curated_skins(&mut store, &work.id).expect("second Skin import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 29);
        assert_eq!(second.aliases_inserted, 0);
        assert_eq!(second.aliases_unchanged, 58);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
    }

    #[test]
    fn skin_facets_are_module_owned() {
        let facets = module().entity_facets();
        for (key, label, option_count) in [
            ("skin_application", "Target", 3),
            ("skin_character", "Character", 29),
            ("skin_weapon_type", "Weapon type", 5),
        ] {
            let facet = facets
                .iter()
                .find(|facet| facet.key == key)
                .unwrap_or_else(|| panic!("missing {key} facet"));
            assert_eq!(facet.display_name, label);
            assert_eq!(facet.entity_types, &["skin"]);
            assert_eq!(facet.options.len(), option_count);
        }
        assert!(skin_thumbnail("../hoyowiki-outfit-5777").is_none());
    }

    #[test]
    fn curated_artifact_and_domain_imports_preserve_hierarchy_and_facets() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("Genshin work");
        let artifacts = import_curated_artifacts(&mut store, &work.id).expect("Artifact import");
        assert_eq!(artifacts.inserted, 362);
        assert_eq!(artifacts.aliases_inserted, 724);
        let domains = import_curated_artifact_domains(&mut store, &work.id).expect("Domain import");
        assert_eq!(domains.inserted, 22);
        assert_eq!(domains.aliases_inserted, 22);

        let sets = store
            .list_entities(&work.id, Some("artifact-set"))
            .expect("Artifact Sets");
        let pieces = store
            .list_entities(&work.id, Some("artifact-piece"))
            .expect("Artifact Pieces");
        assert_eq!(sets.len(), 63);
        assert_eq!(pieces.len(), 299);
        let gladiator = sets
            .iter()
            .find(|item| item.official_english_name == "Gladiator's Finale")
            .expect("Gladiator set");
        assert!(store
            .list_aliases(&gladiator.id)
            .expect("Artifact aliases")
            .iter()
            .any(
                |alias| alias.kind == "han-viet" && alias.value == "Giác Đẩu Sĩ Đích Chung Mán Lễ"
            ));
        let set_presentation = module()
            .entity_presentation(gladiator)
            .expect("set presentation");
        assert_eq!(set_presentation.rarity, Some(5));
        assert_eq!(set_presentation.accent_color, FIVE_STAR_COLOR);

        let flower = pieces
            .iter()
            .find(|item| item.official_english_name == "Gladiator's Nostalgia")
            .expect("Gladiator flower");
        let piece_presentation = module()
            .entity_presentation(flower)
            .expect("piece presentation");
        assert_eq!(
            piece_presentation.context_label.as_deref(),
            Some("Gladiator's Finale")
        );
        assert_eq!(piece_presentation.facets["artifact_slot"], "Flower of Life");

        let domain = store
            .list_entities(&work.id, Some("domain"))
            .expect("Domains")
            .into_iter()
            .find(|item| item.official_english_name == "Valley of Remembrance")
            .expect("Artifact Domain");
        assert_eq!(
            module()
                .entity_presentation(&domain)
                .expect("Domain presentation")
                .facets["domain_region"],
            "Mondstadt"
        );
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
    }

    #[test]
    fn curated_weapon_import_is_complete_idempotent_and_filterable() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("Genshin work");

        let first = import_curated_weapons(&mut store, &work.id).expect("first weapon import");
        assert_eq!(first.inserted, 246);
        assert_eq!(first.unchanged, 0);
        assert_eq!(first.aliases_inserted, 492);

        let weapons = store
            .list_entities(&work.id, Some("weapon"))
            .expect("weapon list");
        assert_eq!(weapons.len(), 246);
        let gravestone = weapons
            .iter()
            .find(|weapon| weapon.official_english_name == "Wolf's Gravestone")
            .expect("Wolf's Gravestone");
        assert_eq!(gravestone.official_original_name, "狼的末路");
        assert_eq!(
            gravestone.official_vietnamese_name.as_deref(),
            Some("Đường Cùng Của Sói")
        );
        assert!(gravestone
            .other_information
            .as_deref()
            .expect("weapon provenance")
            .contains("Ascension region: Mondstadt"));
        let aliases = store.list_aliases(&gravestone.id).expect("weapon aliases");
        assert!(aliases.iter().any(|alias| {
            alias.value == "Lang Đích Mạt Lộ"
                && alias.language.as_deref() == Some("vi")
                && alias.kind == "han-viet"
                && alias.label.as_deref() == Some("Hán-Việt (generated)")
        }));

        let presentation = module()
            .entity_presentation(gravestone)
            .expect("weapon presentation");
        assert_eq!(presentation.label, "5★");
        assert_eq!(presentation.rarity, Some(5));
        assert_eq!(presentation.accent_color, "#d8b66f");
        assert_eq!(presentation.facets["weapon_type"], "Claymore");
        assert_eq!(presentation.facets["main_stat"], "46");
        assert_eq!(presentation.facets["substat"], "ATK");
        assert_eq!(presentation.facets["ascension_region"], "Mondstadt");
        assert_eq!(
            presentation.thumbnail_url,
            "/assets/modules/genshin-impact/weapons/genshin-data-weapon-12502.png"
        );
        assert!(weapon_thumbnail("genshin-data-weapon-12502")
            .expect("bundled weapon thumbnail")
            .starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(weapon_hoyowiki_thumbnail("genshin-data-weapon-12502").is_some());
        assert!(weapon_hoyowiki_thumbnail("genshin-data-weapon-11521").is_none());
        assert!(weapon_fallback_thumbnail("genshin-data-weapon-11521").is_some());

        let four_star = weapons
            .iter()
            .find(|weapon| weapon.official_english_name == "Favonius Sword")
            .expect("Favonius Sword");
        assert_eq!(
            module()
                .entity_presentation(four_star)
                .expect("4-star presentation")
                .accent_color,
            "#9a77c7"
        );
        let three_star = weapons
            .iter()
            .find(|weapon| weapon.official_english_name == "Cool Steel")
            .expect("Cool Steel");
        assert_eq!(
            module()
                .entity_presentation(three_star)
                .expect("3-star presentation")
                .accent_color,
            "#6287c5"
        );

        let second = import_curated_weapons(&mut store, &work.id).expect("second weapon import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 246);
        assert_eq!(second.aliases_inserted, 0);
        assert_eq!(second.aliases_unchanged, 492);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
    }

    #[test]
    fn weapon_facets_are_module_owned_and_complete() {
        let facets = module().entity_facets();
        for (key, label, option_count) in [
            ("weapon_type", "Weapon type", 5),
            ("main_stat", "Base ATK (Lv. 1)", 12),
            ("substat", "Substat", 9),
            ("ascension_region", "Ascension region", 7),
        ] {
            let facet = facets
                .iter()
                .find(|facet| facet.key == key)
                .unwrap_or_else(|| panic!("missing {key} facet"));
            assert_eq!(facet.display_name, label);
            assert_eq!(facet.entity_types, &["weapon"]);
            assert_eq!(facet.options.len(), option_count);
        }
        assert!(weapon_thumbnail("../genshin-data-weapon-12502").is_none());
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
        assert_eq!(first.aliases_inserted, 236);

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
        assert!(aliases.iter().any(|alias| {
            alias.value == "A Bối Đa"
                && alias.language.as_deref() == Some("vi")
                && alias.kind == "han-viet"
                && alias.label.as_deref() == Some("Hán-Việt (generated)")
                && alias
                    .notes
                    .as_deref()
                    .is_some_and(|notes| notes.contains("not an official Vietnamese localization"))
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
        assert_eq!(second.aliases_unchanged, 236);
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
