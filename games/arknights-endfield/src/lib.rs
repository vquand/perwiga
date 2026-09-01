use std::{
    collections::{BTreeMap, HashSet},
    sync::OnceLock,
};

use chrono::{DateTime, FixedOffset};
use perwiga_core::{
    lore::{
        LoreAssertionKind, LoreCandidateBatch, LoreCertainty, LoreClaimCandidate,
        LoreEventCandidate, LoreEvidenceCandidate, LoreEvidenceStance, LoreImportSummary,
        LoreInvolvementCandidate, LoreSourceCandidate, LoreSubjectCandidate, LoreSubjectReference,
        LoreTimeCandidate, LoreTimeKind, LoreTimePrecision,
    },
    model::{
        AliasInput, CalendarEvent, CalendarEventInput, EntityAliasBatchInput,
        EntityAppearanceInput, EntityImportSummary, EntityInput, SourcedEntityInput, WikiEntity,
    },
    CalendarEventPresentation, Capability, EntityEventRecencyPresentation, EntityFacetDefinition,
    EntityFacetOption, EntityPresentation, EntityTypeDefinition, EventFeaturedEntityPresentation,
    EventPreviousGapPresentation, LibraryModule, LoreRoleDefinition, LoreSchemaDefinition,
    LoreSubjectTypeDefinition, ModuleRegistry, PerwigaError, Store, ThemeDefinition, WorkKind,
};
use serde::{Deserialize, Serialize};

pub struct ArknightsEndfieldModule;

static CAPABILITIES: &[Capability] = &[
    Capability::ManualContent,
    Capability::EntitySchema,
    Capability::ScheduledEvents,
    Capability::LoreTimeline,
];
static LORE_SUBJECT_TYPES: &[LoreSubjectTypeDefinition] = &[
    LoreSubjectTypeDefinition {
        key: "operator",
        display_name: "Operator",
    },
    LoreSubjectTypeDefinition {
        key: "npc",
        display_name: "NPC",
    },
    LoreSubjectTypeDefinition {
        key: "region",
        display_name: "Region",
    },
    LoreSubjectTypeDefinition {
        key: "faction",
        display_name: "Faction",
    },
    LoreSubjectTypeDefinition {
        key: "concept",
        display_name: "Concept",
    },
    LoreSubjectTypeDefinition {
        key: "unknown",
        display_name: "Unknown / hear-say entity",
    },
];
static LORE_ROLES: &[LoreRoleDefinition] = &[
    LoreRoleDefinition {
        key: "actor",
        display_name: "Actor",
        description: "Performs or initiates the event.",
    },
    LoreRoleDefinition {
        key: "affected",
        display_name: "Affected",
        description: "Is changed, harmed, helped, or otherwise affected.",
    },
    LoreRoleDefinition {
        key: "witness",
        display_name: "Witness",
        description: "Reports, observes, or remembers the event.",
    },
    LoreRoleDefinition {
        key: "source",
        display_name: "Source",
        description: "Is the in-world or archival source of the claim.",
    },
    LoreRoleDefinition {
        key: "location",
        display_name: "Location",
        description: "Marks the region or place involved.",
    },
    LoreRoleDefinition {
        key: "mentioned",
        display_name: "Mentioned",
        description: "Is named without a stronger involvement claim.",
    },
];
static LORE_SCHEMA: LoreSchemaDefinition = LoreSchemaDefinition {
    subject_types: LORE_SUBJECT_TYPES,
    roles: LORE_ROLES,
};
static THEME: ThemeDefinition = ThemeDefinition {
    background: "#080d11",
    surface: "#0e151b",
    surface_raised: "#131c23",
    surface_active: "#19252d",
    border: "#2b3740",
    border_strong: "#43515b",
    text: "#f2f5f2",
    text_muted: "#a7b0b4",
    text_subtle: "#77838a",
    accent: "#d7f75b",
    accent_ink: "#111807",
};
static ENTITY_TYPES: &[EntityTypeDefinition] = &[
    EntityTypeDefinition {
        key: "operator",
        display_name: "Operator",
        description: "A named operator listed by the official Endfield operator directory.",
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
        description: "A named weapon or weapon type recorded in official update notes.",
    },
    EntityTypeDefinition {
        key: "enemy",
        display_name: "Enemy",
        description: "A named enemy recorded in official update notes.",
    },
    EntityTypeDefinition {
        key: "mission",
        display_name: "Mission",
        description: "A named main, exploration, side, or other mission.",
    },
    EntityTypeDefinition {
        key: "event",
        display_name: "Event",
        description: "A named in-game event or activity.",
    },
    EntityTypeDefinition {
        key: "item",
        display_name: "Item",
        description: "A named item, material, reward, or other database record.",
    },
    EntityTypeDefinition {
        key: "gear-set",
        display_name: "Gear Set",
        description: "A client-defined gear suit and its three-piece set effect.",
    },
    EntityTypeDefinition {
        key: "essence",
        display_name: "Essence",
        description: "A production weapon Essence preset with three official effect terms.",
    },
];
static WEAPON_TYPE_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Sword",
        display_name: "Sword",
    },
    EntityFacetOption {
        value: "Greatsword",
        display_name: "Greatsword",
    },
    EntityFacetOption {
        value: "Polearm",
        display_name: "Polearm",
    },
    EntityFacetOption {
        value: "Handcannon",
        display_name: "Handcannon",
    },
    EntityFacetOption {
        value: "Arts Unit",
        display_name: "Arts Unit",
    },
];
static OPERATOR_ROLE_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Guard",
        display_name: "Guard",
    },
    EntityFacetOption {
        value: "Caster",
        display_name: "Caster",
    },
    EntityFacetOption {
        value: "Supporter",
        display_name: "Supporter",
    },
    EntityFacetOption {
        value: "Defender",
        display_name: "Defender",
    },
    EntityFacetOption {
        value: "Vanguard",
        display_name: "Vanguard",
    },
    EntityFacetOption {
        value: "Striker",
        display_name: "Striker",
    },
];
static OPERATOR_ELEMENT_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Heat",
        display_name: "Heat",
    },
    EntityFacetOption {
        value: "Cryo",
        display_name: "Cryo",
    },
    EntityFacetOption {
        value: "Electric",
        display_name: "Electric",
    },
    EntityFacetOption {
        value: "Nature",
        display_name: "Nature",
    },
    EntityFacetOption {
        value: "Physical",
        display_name: "Physical",
    },
];
static OPERATOR_RACE_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Anasa",
        display_name: "Anasa",
    },
    EntityFacetOption {
        value: "Anaty",
        display_name: "Anaty",
    },
    EntityFacetOption {
        value: "Caprinae",
        display_name: "Caprinae",
    },
    EntityFacetOption {
        value: "Cautus",
        display_name: "Cautus",
    },
    EntityFacetOption {
        value: "Feline",
        display_name: "Feline",
    },
    EntityFacetOption {
        value: "Kylin",
        display_name: "Kylin",
    },
    EntityFacetOption {
        value: "Kuranta",
        display_name: "Kuranta",
    },
    EntityFacetOption {
        value: "Liberi",
        display_name: "Liberi",
    },
    EntityFacetOption {
        value: "Lung",
        display_name: "Lung",
    },
    EntityFacetOption {
        value: "Lupo",
        display_name: "Lupo",
    },
    EntityFacetOption {
        value: "Perro",
        display_name: "Perro",
    },
    EntityFacetOption {
        value: "Phidia",
        display_name: "Phidia",
    },
    EntityFacetOption {
        value: "Sankta",
        display_name: "Sankta",
    },
    EntityFacetOption {
        value: "Sarkaz",
        display_name: "Sarkaz",
    },
    EntityFacetOption {
        value: "Savra",
        display_name: "Savra",
    },
    EntityFacetOption {
        value: "Ursus",
        display_name: "Ursus",
    },
    EntityFacetOption {
        value: "Vouivre",
        display_name: "Vouivre",
    },
    EntityFacetOption {
        value: "Vulpo",
        display_name: "Vulpo",
    },
];
static OPERATOR_SUBRACE_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Djall",
        display_name: "Djall",
    },
    EntityFacetOption {
        value: "Nachzehrer",
        display_name: "Nachzehrer",
    },
    EntityFacetOption {
        value: "Vampire",
        display_name: "Vampire",
    },
];
static ITEM_TYPE_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "collectible",
        display_name: "Collectible",
    },
    EntityFacetOption {
        value: "consumable",
        display_name: "Consumable",
    },
    EntityFacetOption {
        value: "craftable",
        display_name: "Craftable",
    },
    EntityFacetOption {
        value: "crafting-ingredient",
        display_name: "Crafting ingredient",
    },
    EntityFacetOption {
        value: "equipment",
        display_name: "Equipment",
    },
    EntityFacetOption {
        value: "material",
        display_name: "Material",
    },
    EntityFacetOption {
        value: "progression-material",
        display_name: "Progression material",
    },
    EntityFacetOption {
        value: "gatherable",
        display_name: "Gatherable",
    },
    EntityFacetOption {
        value: "natural-resource",
        display_name: "Natural resource",
    },
    EntityFacetOption {
        value: "plant",
        display_name: "Plant",
    },
    EntityFacetOption {
        value: "mineral",
        display_name: "Mineral",
    },
    EntityFacetOption {
        value: "aic-product",
        display_name: "AIC product",
    },
    EntityFacetOption {
        value: "facility",
        display_name: "Facility",
    },
    EntityFacetOption {
        value: "logistics",
        display_name: "Logistics",
    },
    EntityFacetOption {
        value: "tactical-item",
        display_name: "Tactical item",
    },
    EntityFacetOption {
        value: "functional-item",
        display_name: "Functional item",
    },
    EntityFacetOption {
        value: "personal-device",
        display_name: "Personal device",
    },
    EntityFacetOption {
        value: "gift",
        display_name: "Gift",
    },
    EntityFacetOption {
        value: "ornamental",
        display_name: "Ornamental",
    },
    EntityFacetOption {
        value: "mission-item",
        display_name: "Mission item",
    },
    EntityFacetOption {
        value: "currency",
        display_name: "Currency",
    },
    EntityFacetOption {
        value: "permit",
        display_name: "Permit",
    },
    EntityFacetOption {
        value: "seed",
        display_name: "Seed",
    },
    EntityFacetOption {
        value: "essence-material",
        display_name: "Essence material",
    },
    EntityFacetOption {
        value: "valuable",
        display_name: "Valuable",
    },
];
static ITEM_REGION_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Valley IV",
        display_name: "Valley IV",
    },
    EntityFacetOption {
        value: "Wuling",
        display_name: "Wuling",
    },
];
static ESSENCE_PRIMARY_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Agility Boost",
        display_name: "Agility Boost",
    },
    EntityFacetOption {
        value: "Intellect Boost",
        display_name: "Intellect Boost",
    },
    EntityFacetOption {
        value: "Main Attribute Boost",
        display_name: "Main Attribute Boost",
    },
    EntityFacetOption {
        value: "Strength Boost",
        display_name: "Strength Boost",
    },
    EntityFacetOption {
        value: "Will Boost",
        display_name: "Will Boost",
    },
];
static ESSENCE_SECONDARY_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Arts DMG Boost",
        display_name: "Arts DMG Boost",
    },
    EntityFacetOption {
        value: "Arts Intensity Boost",
        display_name: "Arts Intensity Boost",
    },
    EntityFacetOption {
        value: "Attack Boost",
        display_name: "Attack Boost",
    },
    EntityFacetOption {
        value: "Critical Rate Boost",
        display_name: "Critical Rate Boost",
    },
    EntityFacetOption {
        value: "Cryo DMG Boost",
        display_name: "Cryo DMG Boost",
    },
    EntityFacetOption {
        value: "Electric DMG Boost",
        display_name: "Electric DMG Boost",
    },
    EntityFacetOption {
        value: "HP Boost",
        display_name: "HP Boost",
    },
    EntityFacetOption {
        value: "Heat DMG Boost",
        display_name: "Heat DMG Boost",
    },
    EntityFacetOption {
        value: "Nature DMG Boost",
        display_name: "Nature DMG Boost",
    },
    EntityFacetOption {
        value: "Physical DMG Boost",
        display_name: "Physical DMG Boost",
    },
    EntityFacetOption {
        value: "Treatment Efficiency Boost",
        display_name: "Treatment Efficiency Boost",
    },
    EntityFacetOption {
        value: "Ultimate Gain Efficiency Boost",
        display_name: "Ultimate Gain Efficiency Boost",
    },
];
static ESSENCE_SKILL_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Assault",
        display_name: "Assault",
    },
    EntityFacetOption {
        value: "Brutality",
        display_name: "Brutality",
    },
    EntityFacetOption {
        value: "Combative",
        display_name: "Combative",
    },
    EntityFacetOption {
        value: "Crusher",
        display_name: "Crusher",
    },
    EntityFacetOption {
        value: "Detonate",
        display_name: "Detonate",
    },
    EntityFacetOption {
        value: "Efficacy",
        display_name: "Efficacy",
    },
    EntityFacetOption {
        value: "Flow",
        display_name: "Flow",
    },
    EntityFacetOption {
        value: "Fracture",
        display_name: "Fracture",
    },
    EntityFacetOption {
        value: "Infliction",
        display_name: "Infliction",
    },
    EntityFacetOption {
        value: "Inspiring",
        display_name: "Inspiring",
    },
    EntityFacetOption {
        value: "Medicant",
        display_name: "Medicant",
    },
    EntityFacetOption {
        value: "Pursuit",
        display_name: "Pursuit",
    },
    EntityFacetOption {
        value: "Suppression",
        display_name: "Suppression",
    },
    EntityFacetOption {
        value: "Twilight",
        display_name: "Twilight",
    },
];
static REGION_TYPE_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "World",
        display_name: "World",
    },
    EntityFacetOption {
        value: "Macroregion",
        display_name: "Macroregion",
    },
    EntityFacetOption {
        value: "Mobile base",
        display_name: "Mobile base",
    },
    EntityFacetOption {
        value: "Playable region",
        display_name: "Playable region",
    },
    EntityFacetOption {
        value: "Subregion",
        display_name: "Subregion",
    },
    EntityFacetOption {
        value: "Outpost",
        display_name: "Outpost",
    },
    EntityFacetOption {
        value: "Facility",
        display_name: "Facility",
    },
    EntityFacetOption {
        value: "Interior",
        display_name: "Interior",
    },
    EntityFacetOption {
        value: "Anomalous zone",
        display_name: "Anomalous zone",
    },
    EntityFacetOption {
        value: "Megastructure",
        display_name: "Megastructure",
    },
    EntityFacetOption {
        value: "Infrastructure landmark",
        display_name: "Infrastructure landmark",
    },
    EntityFacetOption {
        value: "Iconic landmark",
        display_name: "Iconic landmark",
    },
    EntityFacetOption {
        value: "Natural landmark",
        display_name: "Natural landmark",
    },
    EntityFacetOption {
        value: "Hideout",
        display_name: "Hideout",
    },
    EntityFacetOption {
        value: "Fort",
        display_name: "Fort",
    },
    EntityFacetOption {
        value: "Underground complex",
        display_name: "Underground complex",
    },
    EntityFacetOption {
        value: "Street",
        display_name: "Street",
    },
    EntityFacetOption {
        value: "Transit facility",
        display_name: "Transit facility",
    },
    EntityFacetOption {
        value: "Route",
        display_name: "Route",
    },
    EntityFacetOption {
        value: "Shelter",
        display_name: "Shelter",
    },
];
static REGION_PARENT_OPTIONS: &[EntityFacetOption] = &[
    EntityFacetOption {
        value: "Talos-II",
        display_name: "Talos-II",
    },
    EntityFacetOption {
        value: "Valley IV",
        display_name: "Valley IV",
    },
    EntityFacetOption {
        value: "Wuling",
        display_name: "Wuling",
    },
    EntityFacetOption {
        value: "The Hub",
        display_name: "The Hub",
    },
    EntityFacetOption {
        value: "Valley Pass",
        display_name: "Valley Pass",
    },
    EntityFacetOption {
        value: "Originium Science Park",
        display_name: "Originium Science Park",
    },
    EntityFacetOption {
        value: "Origin Lodespring",
        display_name: "Origin Lodespring",
    },
    EntityFacetOption {
        value: "Power Plateau",
        display_name: "Power Plateau",
    },
    EntityFacetOption {
        value: "Jingyu Valley",
        display_name: "Jingyu Valley",
    },
    EntityFacetOption {
        value: "Marker Stone",
        display_name: "Marker Stone",
    },
    EntityFacetOption {
        value: "Yinglung Pass",
        display_name: "Yinglung Pass",
    },
];
static ENTITY_FACETS: &[EntityFacetDefinition] = &[
    EntityFacetDefinition {
        key: "role",
        display_name: "Role",
        entity_types: &["operator"],
        options: OPERATOR_ROLE_OPTIONS,
    },
    EntityFacetDefinition {
        key: "element",
        display_name: "Element",
        entity_types: &["operator"],
        options: OPERATOR_ELEMENT_OPTIONS,
    },
    EntityFacetDefinition {
        key: "weapon_type",
        display_name: "Weapon type",
        entity_types: &["operator", "weapon", "essence"],
        options: WEAPON_TYPE_OPTIONS,
    },
    EntityFacetDefinition {
        key: "race",
        display_name: "Race",
        entity_types: &["operator"],
        options: OPERATOR_RACE_OPTIONS,
    },
    EntityFacetDefinition {
        key: "subrace",
        display_name: "Subrace",
        entity_types: &["operator"],
        options: OPERATOR_SUBRACE_OPTIONS,
    },
    EntityFacetDefinition {
        key: "item_type",
        display_name: "Type",
        entity_types: &["item"],
        options: ITEM_TYPE_OPTIONS,
    },
    EntityFacetDefinition {
        key: "region",
        display_name: "Region",
        entity_types: &["item"],
        options: ITEM_REGION_OPTIONS,
    },
    EntityFacetDefinition {
        key: "essence_primary",
        display_name: "Primary effect",
        entity_types: &["essence"],
        options: ESSENCE_PRIMARY_OPTIONS,
    },
    EntityFacetDefinition {
        key: "essence_secondary",
        display_name: "Secondary effect",
        entity_types: &["essence"],
        options: ESSENCE_SECONDARY_OPTIONS,
    },
    EntityFacetDefinition {
        key: "essence_skill",
        display_name: "Skill effect",
        entity_types: &["essence"],
        options: ESSENCE_SKILL_OPTIONS,
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

#[derive(Debug, Deserialize)]
struct OperatorSnapshot {
    checked_at: String,
    operators: Vec<CuratedOperator>,
    #[serde(default)]
    official_localizations: Vec<CuratedLocaleDirectory>,
    #[serde(default)]
    additional_aliases: Vec<CuratedAdditionalAlias>,
}

#[derive(Debug, Deserialize)]
struct CuratedOperator {
    source_key: String,
    official_english_name: String,
    official_original_name: String,
    official_vietnamese_name: Option<String>,
    rarity: u8,
    class: String,
    element: String,
    weapon_type: String,
    race: Option<String>,
    subrace: Option<String>,
    han_viet_name: String,
}

#[derive(Debug, Deserialize)]
struct NpcSnapshot {
    checked_at: String,
    sources: BTreeMap<String, String>,
    npcs: Vec<CuratedNpc>,
}

#[derive(Debug, Deserialize)]
struct LoreAppearanceSnapshot {
    checked_at: String,
    client_data_commit: String,
    catalog_scope: LoreAppearanceScope,
    sources: BTreeMap<String, String>,
    evidence_note: String,
    missions: Vec<LoreAppearanceMission>,
    unplaced_characters: Vec<LoreAppearanceCharacter>,
}

#[derive(Debug, Deserialize)]
struct LoreAppearanceScope {
    operators: usize,
    npcs: usize,
    characters: usize,
    placed_characters: usize,
    unplaced_characters: usize,
    missions: usize,
}

#[derive(Debug, Deserialize)]
struct LoreAppearanceMission {
    mission_id: String,
    title_en: String,
    summary_en: Option<String>,
    period_key: Option<String>,
    source_locator: String,
    characters: Vec<LoreAppearanceCharacter>,
}

#[derive(Debug, Clone, Deserialize)]
struct LoreAppearanceCharacter {
    entity_type: String,
    source_key: String,
    name_en: String,
}

#[derive(Debug, Deserialize)]
struct CuratedNpc {
    source_key: String,
    catalog_index: u16,
    official_english_name: String,
    official_original_name: String,
    official_vietnamese_name: Option<String>,
    han_viet_name: Option<String>,
    name_evidence: String,
    name_confidence: String,
    source_url: String,
}

#[derive(Debug, Deserialize)]
struct WeaponSnapshot {
    weapons: Vec<CuratedWeapon>,
}

#[derive(Debug, Deserialize)]
struct CuratedWeapon {
    source_key: String,
    rarity: u8,
    weapon_type: String,
}

#[derive(Debug, Deserialize)]
struct ItemSnapshot {
    items: Vec<CuratedItem>,
}

#[derive(Debug, Deserialize)]
struct CuratedItem {
    source_key: String,
    client_icon_id: String,
    rarity: u8,
    classifications: Vec<String>,
    region: Option<CuratedItemRegion>,
}

#[derive(Debug, Deserialize)]
struct CuratedItemRegion {
    english: String,
}

#[derive(Debug, Deserialize)]
struct GearEssenceSnapshot {
    checked_at: String,
    client_data_commit: String,
    sources: BTreeMap<String, String>,
    gear_sets: Vec<CuratedGearSet>,
    essences: Vec<CuratedEssence>,
}

#[derive(Debug, Deserialize)]
struct CuratedGearSet {
    source_key: String,
    official_english_name: String,
    official_original_name: String,
    official_vietnamese_name: Option<String>,
    official_traditional_chinese_name: Option<String>,
    han_viet_name: Option<String>,
    client_logo_id: String,
    official_english_effect: String,
    member_item_ids: Vec<String>,
    rarities: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct CuratedEssence {
    source_key: String,
    official_english_name: String,
    official_original_name: String,
    official_vietnamese_name: Option<String>,
    official_traditional_chinese_name: Option<String>,
    han_viet_name: Option<String>,
    catalog_label: String,
    catalog_han_viet_label: Option<String>,
    rarity: u8,
    client_icon_id: String,
    weapon_type: String,
    compatible_weapon_ids: Vec<String>,
    terms: Vec<CuratedEssenceTerm>,
}

#[derive(Debug, Deserialize)]
struct CuratedEssenceTerm {
    kind: String,
    official_english_name: String,
}

#[derive(Debug, Deserialize)]
struct RegionSnapshot {
    checked_at: String,
    regions: Vec<CuratedRegion>,
}

#[derive(Debug, Deserialize)]
struct CuratedRegion {
    source_key: String,
    official_english_name: String,
    official_original_name: String,
    official_vietnamese_name: Option<String>,
    han_viet_name: String,
    subtype: String,
    parent_source_key: Option<String>,
    #[serde(default)]
    related_source_keys: Vec<String>,
    status: String,
    #[serde(default)]
    client_keys: Vec<String>,
    source_table: String,
    catalog_index: u16,
    evidence_url: String,
}

#[derive(Debug, Deserialize)]
struct CuratedLocaleDirectory {
    language: String,
    label: String,
    source_url: String,
    names: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct CuratedAdditionalAlias {
    source_key: String,
    value: String,
    language: Option<String>,
    kind: String,
    label: String,
    notes: String,
}

#[derive(Debug, Deserialize)]
struct EventSnapshot {
    metadata: EventSnapshotMetadata,
    events: Vec<CuratedEvent>,
}

#[derive(Debug, Deserialize)]
struct EventSnapshotMetadata {
    module_id: String,
    source_provider: String,
    time_zone: String,
    source_checked_at: String,
}

#[derive(Debug, Deserialize)]
struct CuratedEvent {
    source_key: String,
    title: String,
    event_type: String,
    starts_at: String,
    ends_at: Option<String>,
    #[serde(default)]
    is_all_day: bool,
    patch_start: String,
    patch_end: Option<String>,
    occurrence_index: Option<u16>,
    #[serde(default)]
    recurring: bool,
    source_url: String,
    schedule_note: Option<String>,
    #[serde(default)]
    featured_operator_keys: Vec<String>,
    featured_heading: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CuratedAliasImportSummary {
    pub inserted: usize,
    pub unchanged: usize,
    pub unresolved: Vec<String>,
}

fn curated_snapshot() -> &'static OperatorSnapshot {
    static SNAPSHOT: OnceLock<OperatorSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        serde_json::from_str::<OperatorSnapshot>(include_str!("../data/operators.json"))
            .expect("the bundled Endfield operator snapshot must be valid JSON")
    })
}

fn curated_operators() -> &'static [CuratedOperator] {
    curated_snapshot().operators.as_slice()
}

fn curated_operator(source_key: &str) -> Option<&'static CuratedOperator> {
    curated_operators()
        .iter()
        .find(|operator| operator.source_key == source_key)
}

fn curated_npc_snapshot() -> &'static NpcSnapshot {
    static SNAPSHOT: OnceLock<NpcSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        let snapshot = serde_json::from_str::<NpcSnapshot>(include_str!("../data/npcs.json"))
            .expect("the bundled Endfield NPC snapshot must be valid JSON");
        assert_eq!(snapshot.npcs.len(), 134, "verified NPC scope changed");
        snapshot
    })
}

fn curated_weapon_snapshot() -> &'static WeaponSnapshot {
    static SNAPSHOT: OnceLock<WeaponSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        serde_json::from_str::<WeaponSnapshot>(include_str!("../data/weapons.json"))
            .expect("the bundled Endfield weapon snapshot must be valid JSON")
    })
}

fn curated_weapon(source_key: &str) -> Option<&'static CuratedWeapon> {
    curated_weapon_snapshot()
        .weapons
        .iter()
        .find(|weapon| weapon.source_key == source_key)
}

fn curated_item_snapshot() -> &'static ItemSnapshot {
    static SNAPSHOT: OnceLock<ItemSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        serde_json::from_str::<ItemSnapshot>(include_str!("../data/items.json"))
            .expect("the bundled Endfield item snapshot must be valid JSON")
    })
}

fn curated_item(source_key: &str) -> Option<&'static CuratedItem> {
    curated_item_snapshot()
        .items
        .iter()
        .find(|item| item.source_key == source_key)
}

fn curated_gear_essence_snapshot() -> &'static GearEssenceSnapshot {
    static SNAPSHOT: OnceLock<GearEssenceSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        let snapshot =
            serde_json::from_str::<GearEssenceSnapshot>(include_str!("../data/gear-essences.json"))
                .expect("the bundled Endfield Gear Set and Essence snapshot must be valid JSON");
        assert_eq!(
            snapshot.gear_sets.len(),
            23,
            "verified Gear Set scope changed"
        );
        assert_eq!(
            snapshot.essences.len(),
            154,
            "verified Essence scope changed"
        );
        snapshot
    })
}

fn curated_gear_set(source_key: &str) -> Option<&'static CuratedGearSet> {
    curated_gear_essence_snapshot()
        .gear_sets
        .iter()
        .find(|record| record.source_key == source_key)
}

fn curated_essence(source_key: &str) -> Option<&'static CuratedEssence> {
    curated_gear_essence_snapshot()
        .essences
        .iter()
        .find(|record| record.source_key == source_key)
}

fn curated_region_snapshot() -> &'static RegionSnapshot {
    static SNAPSHOT: OnceLock<RegionSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        let snapshot = serde_json::from_str::<RegionSnapshot>(include_str!("../data/regions.json"))
            .expect("the bundled Endfield Region snapshot must be valid JSON");
        assert_eq!(snapshot.regions.len(), 43, "verified Region scope changed");
        snapshot
    })
}

fn curated_region(source_key: &str) -> Option<&'static CuratedRegion> {
    curated_region_snapshot()
        .regions
        .iter()
        .find(|record| record.source_key == source_key)
}

fn region_type_label(value: &str) -> String {
    value
        .split('-')
        .enumerate()
        .map(|(index, word)| {
            if index == 0 {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const OPERATOR_SOURCE_PROVIDER: &str = "Arknights: Endfield official operator directory";
const NPC_SOURCE_PROVIDER: &str = "beyondGameData@90bc55768143f1998ad02d03ed0042919135979f";

fn validate_endfield_work(store: &Store, work_id: &str) -> perwiga_core::Result<()> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "arknights-endfield" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Arknights: Endfield module"
        )));
    }
    Ok(())
}

pub fn import_curated_operators(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    validate_endfield_work(store, work_id)?;
    let inputs = curated_operators()
        .iter()
        .map(|record| SourcedEntityInput {
            source_provider: OPERATOR_SOURCE_PROVIDER.to_string(),
            source_identity: format!("operator:{}", record.source_key),
            entity: EntityInput {
                entity_type: "operator".to_string(),
                official_english_name: record.official_english_name.clone(),
                official_original_name: record.official_original_name.clone(),
                official_vietnamese_name: record.official_vietnamese_name.clone(),
                automatic_vietnamese_translation: None,
                english_description: Some(format!(
                    "A {}-star {} Operator who uses {} and deals {} damage.",
                    record.rarity, record.class, record.weapon_type, record.element
                )),
                other_information: Some(format!(
                    "Official source key: {}. Official operator directory checked {}.",
                    record.source_key,
                    curated_snapshot().checked_at
                )),
            },
            aliases: vec![AliasInput {
                value: record.han_viet_name.clone(),
                language: Some("vi".to_string()),
                kind: "han-viet".to_string(),
                label: Some("Hán-Việt (generated)".to_string()),
                notes: Some(format!(
                    "Generated from the official Simplified Chinese name; not an official Vietnamese localization. Checked {}. Confidence E (generated).",
                    curated_snapshot().checked_at
                )),
            }],
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

pub fn import_curated_npcs(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    validate_endfield_work(store, work_id)?;
    let snapshot = curated_npc_snapshot();
    let client_source = snapshot
        .sources
        .get("localized_client_data")
        .map(String::as_str)
        .unwrap_or("unrecorded client localization source");
    let inputs = snapshot
        .npcs
        .iter()
        .map(|record| SourcedEntityInput {
            source_provider: NPC_SOURCE_PROVIDER.to_string(),
            source_identity: format!("npc:{}", record.source_key),
            entity: EntityInput {
                entity_type: "npc".to_string(),
                official_english_name: record.official_english_name.clone(),
                official_original_name: record.official_original_name.clone(),
                official_vietnamese_name: record.official_vietnamese_name.clone(),
                automatic_vietnamese_translation: None,
                english_description: Some("A named non-playable character in Arknights: Endfield.".to_string()),
                other_information: Some(format!(
                    "Curated NPC source key: {}. Catalog index: {}/134. Name evidence: {}. Confidence {}. Catalog source: {}. Client localization source: {}. Checked {}.",
                    record.source_key,
                    record.catalog_index,
                    record.name_evidence,
                    record.name_confidence,
                    record.source_url,
                    client_source,
                    snapshot.checked_at
                )),
            },
            aliases: record
                .han_viet_name
                .as_ref()
                .map(|value| AliasInput {
                    value: value.clone(),
                    language: Some("vi".to_string()),
                    kind: "han-viet".to_string(),
                    label: Some("Hán-Việt (generated)".to_string()),
                    notes: Some(format!(
                        "Generated from aligned Simplified Chinese client text; not an official Vietnamese localization. Checked {}. Confidence E (generated).",
                        snapshot.checked_at
                    )),
                })
                .into_iter()
                .collect(),
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

fn region_alias(record: &CuratedRegion) -> AliasInput {
    AliasInput {
        value: record.han_viet_name.clone(),
        language: Some("vi".to_string()),
        kind: "han-viet".to_string(),
        label: Some("Hán-Việt (generated)".to_string()),
        notes: Some(format!(
            "Generated from verified Simplified Chinese client text with Thi Viện dictionary-attested readings; not an official Vietnamese localization. Checked {}. Confidence E (generated).",
            curated_region_snapshot().checked_at
        )),
    }
}

fn region_input(record: &CuratedRegion) -> SourcedEntityInput {
    let parent = record
        .parent_source_key
        .as_deref()
        .and_then(curated_region)
        .map(|parent| parent.official_english_name.as_str());
    SourcedEntityInput {
        source_provider: "beyondGameData@90bc55768143f1998ad02d03ed0042919135979f".to_string(),
        source_identity: format!("region:{}", record.source_key),
        entity: EntityInput {
            entity_type: "region".to_string(),
            official_english_name: record.official_english_name.clone(),
            official_original_name: record.official_original_name.clone(),
            official_vietnamese_name: record.official_vietnamese_name.clone(),
            automatic_vietnamese_translation: None,
            english_description: Some(match parent {
                Some(parent) => format!(
                    "A {} in {}.",
                    region_type_label(&record.subtype).to_lowercase(),
                    parent
                ),
                None => format!("A {} in Arknights: Endfield.", region_type_label(&record.subtype).to_lowercase()),
            }),
            other_information: Some(format!(
                "Curated Region source key: {}. Catalog index: {}/43. Subtype: {}. Parent source key: {}. Related source keys: {}. Status: {}. Client keys: {}. Source table: {}. Evidence: {}. Checked {}.",
                record.source_key,
                record.catalog_index,
                record.subtype,
                record.parent_source_key.as_deref().unwrap_or("none"),
                if record.related_source_keys.is_empty() { "none".to_string() } else { record.related_source_keys.join(", ") },
                record.status,
                if record.client_keys.is_empty() { "none".to_string() } else { record.client_keys.join(", ") },
                record.source_table,
                record.evidence_url,
                curated_region_snapshot().checked_at
            )),
        },
        aliases: vec![region_alias(record)],
    }
}

pub fn import_curated_regions(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "arknights-endfield" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Arknights: Endfield module"
        )));
    }

    let existing = store.list_entities(work_id, Some("region"))?;
    let mut existing_by_key = BTreeMap::new();
    for entity in &existing {
        if let Some(key) = region_source_key(entity) {
            if existing_by_key.insert(key.to_string(), entity).is_some() {
                return Err(PerwigaError::Conflict(format!(
                    "multiple Region rows claim curated source key {key}"
                )));
            }
        }
    }

    let mut pending = Vec::new();
    let mut existing_aliases = Vec::new();
    let mut summary = EntityImportSummary::default();
    for record in &curated_region_snapshot().regions {
        if let Some(entity) = existing_by_key.get(&record.source_key) {
            summary.unchanged += 1;
            existing_aliases.push(EntityAliasBatchInput {
                entity_id: entity.id.clone(),
                alias: region_alias(record),
            });
        } else {
            pending.push(region_input(record));
        }
    }
    let inserted = store.import_sourced_entities(work_id, &pending)?;
    let aliases = store.add_aliases_batch(&existing_aliases)?;
    summary.inserted += inserted.inserted;
    summary.unchanged += inserted.unchanged;
    summary.aliases_inserted += inserted.aliases_inserted + aliases.inserted;
    summary.aliases_unchanged += inserted.aliases_unchanged + aliases.unchanged;
    Ok(summary)
}

fn generated_alias(value: &str, label: &str) -> AliasInput {
    AliasInput {
        value: value.to_string(),
        language: Some("vi".to_string()),
        kind: "han-viet".to_string(),
        label: Some(label.to_string()),
        notes: Some(format!(
            "Generated from verified Simplified Chinese client text with Thi Viện dictionary-attested readings; not an official Vietnamese localization. Checked {}. Confidence E (generated).",
            curated_gear_essence_snapshot().checked_at
        )),
    }
}

fn traditional_chinese_alias(value: &str) -> AliasInput {
    AliasInput {
        value: value.to_string(),
        language: Some("zh-Hant".to_string()),
        kind: "official-localization".to_string(),
        label: Some("Official Traditional Chinese".to_string()),
        notes: Some(format!(
            "Official client localization aligned independently by localization key. Checked {}. Confidence B.",
            curated_gear_essence_snapshot().checked_at
        )),
    }
}

pub fn import_curated_gear_sets_and_essences(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "arknights-endfield" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Arknights: Endfield module"
        )));
    }
    let snapshot = curated_gear_essence_snapshot();
    let provider = format!("beyondGameData@{}", snapshot.client_data_commit);
    let source_url = snapshot
        .sources
        .get("client_data")
        .cloned()
        .unwrap_or_else(|| "https://github.com/555me/beyondGameData".to_string());
    let mut inputs = Vec::with_capacity(snapshot.gear_sets.len() + snapshot.essences.len());

    for gear in &snapshot.gear_sets {
        let mut aliases = Vec::new();
        if let Some(value) = &gear.official_traditional_chinese_name {
            aliases.push(traditional_chinese_alias(value));
        }
        if let Some(value) = &gear.han_viet_name {
            aliases.push(generated_alias(value, "Hán-Việt (generated)"));
        }
        inputs.push(SourcedEntityInput {
            source_provider: provider.clone(),
            source_identity: format!("gear-set:{}", gear.source_key),
            entity: EntityInput {
                entity_type: "gear-set".to_string(),
                official_english_name: gear.official_english_name.clone(),
                official_original_name: gear.official_original_name.clone(),
                official_vietnamese_name: gear.official_vietnamese_name.clone(),
                automatic_vietnamese_translation: None,
                english_description: Some(gear.official_english_effect.clone()),
                other_information: Some(format!(
                    "Curated Gear Set source key: {}. Client logo ID: {}. Set requires 3 pieces and has {} cataloged gear-piece variants. Available rarities: {}. Source: {}. Checked {}.",
                    gear.source_key,
                    gear.client_logo_id,
                    gear.member_item_ids.len(),
                    gear.rarities.iter().map(u8::to_string).collect::<Vec<_>>().join(", "),
                    source_url,
                    snapshot.checked_at
                )),
            },
            aliases,
        });
    }

    for essence in &snapshot.essences {
        let mut aliases = vec![AliasInput {
            value: essence.catalog_label.clone(),
            language: Some("en".to_string()),
            kind: "catalog-label".to_string(),
            label: Some("Source-composed catalog label".to_string()),
            notes: Some("UI/search label composed from the official Essence rarity name and its three official effect-term names; not an official standalone item name.".to_string()),
        }];
        if let Some(value) = &essence.official_traditional_chinese_name {
            aliases.push(traditional_chinese_alias(value));
        }
        if let Some(value) = &essence.han_viet_name {
            aliases.push(generated_alias(value, "Hán-Việt (generated)"));
        }
        if let Some(value) = &essence.catalog_han_viet_label {
            aliases.push(generated_alias(value, "Hán-Việt catalog label (generated)"));
        }
        let terms = essence
            .terms
            .iter()
            .map(|term| format!("{}: {}", term.kind, term.official_english_name))
            .collect::<Vec<_>>()
            .join("; ");
        inputs.push(SourcedEntityInput {
            source_provider: provider.clone(),
            source_identity: format!("essence:{}", essence.source_key),
            entity: EntityInput {
                entity_type: "essence".to_string(),
                official_english_name: essence.official_english_name.clone(),
                official_original_name: essence.official_original_name.clone(),
                official_vietnamese_name: essence.official_vietnamese_name.clone(),
                automatic_vietnamese_translation: None,
                english_description: Some(format!(
                    "A {}★ {} Essence preset for {}. {}.",
                    essence.rarity, essence.official_english_name, essence.weapon_type, terms
                )),
                other_information: Some(format!(
                    "Curated Essence source key: {}. Catalog label: {}. Client icon ID: {}. Compatible weapon IDs: {}. Source: {}. Checked {}. The catalog label is source-composed and is not represented as an official name.",
                    essence.source_key,
                    essence.catalog_label,
                    essence.client_icon_id,
                    essence.compatible_weapon_ids.join(", "),
                    source_url,
                    snapshot.checked_at
                )),
            },
            aliases,
        });
    }
    store.import_sourced_entities(work_id, &inputs)
}

fn curated_event_snapshot() -> Result<&'static EventSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<EventSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        let snapshot = serde_json::from_str(include_str!("../data/events.json"))
            .map_err(|error| format!("invalid bundled Endfield event snapshot: {error}"))?;
        validate_event_snapshot(&snapshot)?;
        Ok(snapshot)
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

pub fn curated_lore_candidate_batch() -> Result<&'static LoreCandidateBatch, PerwigaError> {
    static BATCH: OnceLock<Result<LoreCandidateBatch, String>> = OnceLock::new();
    match BATCH.get_or_init(|| {
        let mut batch =
            serde_json::from_str::<LoreCandidateBatch>(include_str!("../data/lore.json"))
                .map_err(|error| format!("invalid bundled Endfield lore seed: {error}"))?;
        let appearances = serde_json::from_str::<LoreAppearanceSnapshot>(include_str!(
            "../data/lore-appearances.json"
        ))
        .map_err(|error| format!("invalid bundled Endfield lore appearance snapshot: {error}"))?;
        extend_lore_with_character_appearances(&mut batch, &appearances)?;
        batch
            .validate()
            .map_err(|error| format!("invalid bundled Endfield lore seed: {error}"))?;
        Ok(batch)
    }) {
        Ok(batch) => Ok(batch),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
}

fn lore_character_subject_key(character: &LoreAppearanceCharacter) -> String {
    if character.entity_type == "operator" && character.source_key == "avywenna" {
        "avywenna".to_string()
    } else {
        format!("catalog-{}-{}", character.entity_type, character.source_key)
    }
}

fn lore_character_reference(character: &LoreAppearanceCharacter) -> LoreSubjectReference {
    let provider = if character.entity_type == "operator" {
        OPERATOR_SOURCE_PROVIDER
    } else {
        NPC_SOURCE_PROVIDER
    };
    LoreSubjectReference {
        provider: provider.to_string(),
        identity: format!("{}:{}", character.entity_type, character.source_key),
    }
}

fn extend_lore_with_character_appearances(
    batch: &mut LoreCandidateBatch,
    snapshot: &LoreAppearanceSnapshot,
) -> Result<(), String> {
    if snapshot.client_data_commit != "90bc55768143f1998ad02d03ed0042919135979f" {
        return Err("Endfield lore appearances use an unexpected client-data commit".into());
    }
    if snapshot.catalog_scope.operators != curated_operators().len()
        || snapshot.catalog_scope.npcs != curated_npc_snapshot().npcs.len()
        || snapshot.catalog_scope.characters
            != snapshot.catalog_scope.operators + snapshot.catalog_scope.npcs
        || snapshot.catalog_scope.missions != snapshot.missions.len()
        || snapshot.catalog_scope.unplaced_characters != snapshot.unplaced_characters.len()
        || snapshot.catalog_scope.placed_characters + snapshot.catalog_scope.unplaced_characters
            != snapshot.catalog_scope.characters
    {
        return Err("Endfield lore appearance coverage does not match the curated catalogs".into());
    }

    let dialogue_source_key = "client-mission-dialogue";
    let operator_catalog_source_key = "official-operator-directory-catalog";
    let npc_catalog_source_key = "client-npc-catalog";
    batch.sources.extend([
        LoreSourceCandidate {
            key: dialogue_source_key.into(),
            kind: "structured-client-dialogue".into(),
            provider: format!("beyondGameData@{}", snapshot.client_data_commit),
            identity: "dialog/json/mission".into(),
            title: "Arknights: Endfield mission dialogue actor data".into(),
            language: "en".into(),
            url: snapshot.sources.get("dialogue").cloned(),
            content_sha256: format!("git-{}-dialogue", snapshot.client_data_commit),
            checked_at: snapshot.checked_at.clone(),
        },
        LoreSourceCandidate {
            key: operator_catalog_source_key.into(),
            kind: "official-operator-directory".into(),
            provider: OPERATOR_SOURCE_PROVIDER.into(),
            identity: "operator-directory".into(),
            title: "Arknights: Endfield official Operator directory".into(),
            language: "en".into(),
            url: Some("https://endfield.gryphline.com/en-us/operator".into()),
            content_sha256: "official-operator-directory-checked-2026-08-24".into(),
            checked_at: snapshot.checked_at.clone(),
        },
        LoreSourceCandidate {
            key: npc_catalog_source_key.into(),
            kind: "structured-client-localization".into(),
            provider: NPC_SOURCE_PROVIDER.into(),
            identity: "tableCfg/NpcTable.json".into(),
            title: "Arknights: Endfield NPC client table".into(),
            language: "en".into(),
            url: snapshot.sources.get("npc_templates").cloned(),
            content_sha256: format!("git-{}-npc-table", snapshot.client_data_commit),
            checked_at: snapshot.checked_at.clone(),
        },
    ]);

    let mut appearances_by_character: BTreeMap<(String, String), Vec<&LoreAppearanceMission>> =
        BTreeMap::new();
    for mission in &snapshot.missions {
        for character in &mission.characters {
            appearances_by_character
                .entry((character.entity_type.clone(), character.source_key.clone()))
                .or_default()
                .push(mission);
        }
    }
    let all_characters = snapshot
        .missions
        .iter()
        .flat_map(|mission| mission.characters.iter().cloned())
        .chain(snapshot.unplaced_characters.iter().cloned())
        .collect::<Vec<_>>();
    let mut seen_characters = HashSet::new();
    for character in all_characters {
        let identity = (character.entity_type.clone(), character.source_key.clone());
        if !seen_characters.insert(identity.clone()) {
            continue;
        }
        let source_key = if let Some(mission) = appearances_by_character
            .get(&identity)
            .and_then(|missions| missions.first())
        {
            let subject_key = lore_character_subject_key(&character);
            if subject_key == "avywenna" {
                continue;
            }
            batch.subjects.push(LoreSubjectCandidate {
                key: subject_key,
                attested_name: character.name_en.clone(),
                proposed_type: character.entity_type.clone(),
                entity_ref: Some(lore_character_reference(&character)),
                evidence: vec![LoreEvidenceCandidate {
                    source_key: dialogue_source_key.into(),
                    locator: mission.source_locator.clone(),
                    excerpt: format!(
                        "{} is staged as an actor in the structured dialogue for {}.",
                        character.name_en, mission.title_en
                    ),
                    stance: Some(LoreEvidenceStance::Supports),
                }],
            });
            continue;
        } else if character.entity_type == "operator" {
            operator_catalog_source_key
        } else {
            npc_catalog_source_key
        };
        batch.subjects.push(LoreSubjectCandidate {
            key: lore_character_subject_key(&character),
            attested_name: character.name_en.clone(),
            proposed_type: character.entity_type.clone(),
            entity_ref: Some(lore_character_reference(&character)),
            evidence: vec![LoreEvidenceCandidate {
                source_key: source_key.into(),
                locator: format!("catalog:{}", character.source_key),
                excerpt: format!(
                    "{} is in the curated {} catalog; no mission actor placement is attested in this snapshot.",
                    character.name_en, character.entity_type
                ),
                stance: Some(LoreEvidenceStance::Supports),
            }],
        });
    }

    for mission in &snapshot.missions {
        let involvements = mission
            .characters
            .iter()
            .map(|character| LoreInvolvementCandidate {
                subject_key: lore_character_subject_key(character),
                role: "actor".into(),
            })
            .collect::<Vec<_>>();
        let evidence = LoreEvidenceCandidate {
            source_key: dialogue_source_key.into(),
            locator: mission.source_locator.clone(),
            excerpt: format!(
                "The structured mission dialogue stages {} catalogued character(s). {}",
                mission.characters.len(),
                snapshot.evidence_note
            ),
            stance: Some(LoreEvidenceStance::Supports),
        };
        if mission.mission_id == "sm1l5m1" {
            let event = batch
                .events
                .iter_mut()
                .find(|event| event.key == "visitor-from-the-band-i")
                .ok_or("the curated Visitor from the Band I event is missing")?;
            let mut existing_subjects = event
                .involvements
                .iter()
                .map(|involvement| involvement.subject_key.clone())
                .collect::<HashSet<_>>();
            event.involvements.extend(
                involvements
                    .iter()
                    .filter(|involvement| existing_subjects.insert(involvement.subject_key.clone()))
                    .cloned(),
            );
            event.claims.push(LoreClaimCandidate {
                key: "sm1l5m1-character-staging".into(),
                text_en: format!(
                    "The client dialogue data associates {} catalogued character(s) with {}.",
                    mission.characters.len(),
                    mission.title_en
                ),
                assertion_kind: LoreAssertionKind::DirectFact,
                certainty: LoreCertainty::Confirmed,
                branch_group: None,
                involvements,
                evidence: vec![evidence],
            });
            continue;
        }
        batch.events.push(LoreEventCandidate {
            key: format!("client-mission-{}", mission.mission_id),
            title_en: mission.title_en.clone(),
            summary_en: mission.summary_en.clone(),
            time: LoreTimeCandidate {
                kind: LoreTimeKind::Moment,
                precision: if mission.period_key.is_some() {
                    LoreTimePrecision::Relative
                } else {
                    LoreTimePrecision::Unknown
                },
                label: format!(
                    "Mission {} · {}",
                    mission.mission_id,
                    if mission.period_key.is_some() {
                        "broad story placement"
                    } else {
                        "exact placement unresolved"
                    }
                ),
                start_period_key: mission.period_key.clone(),
                end_period_key: None,
                relations: Vec::new(),
            },
            involvements: involvements.clone(),
            claims: vec![LoreClaimCandidate {
                key: format!("{}-character-staging", mission.mission_id),
                text_en: format!(
                    "The client dialogue data associates {} catalogued character(s) with {}.",
                    mission.characters.len(),
                    mission.title_en
                ),
                assertion_kind: LoreAssertionKind::DirectFact,
                certainty: LoreCertainty::Confirmed,
                branch_group: None,
                involvements,
                evidence: vec![evidence],
            }],
        });
    }
    batch.corpus.version = "reviewed-web-and-client-dialogue-2026-09-v6".into();
    batch.corpus.manifest_sha256 =
        "0f56eb0443b10bdb3955f76db1dc1cdba3e7671612009c3d4b046aae2f52a262".into();
    batch.generator.name = "perwiga-curated-seed-and-client-dialogue".into();
    batch.generator.version = "6".into();
    Ok(())
}

pub fn import_curated_lore(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<LoreImportSummary> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "arknights-endfield" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Arknights: Endfield module"
        )));
    }
    import_curated_lore_appearances(store, work_id)?;
    store.import_reviewed_lore_candidate_batch(work_id, curated_lore_candidate_batch()?)
}

fn import_curated_lore_appearances(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<usize> {
    let snapshot = serde_json::from_str::<LoreAppearanceSnapshot>(include_str!(
        "../data/lore-appearances.json"
    ))
    .map_err(|error| {
        PerwigaError::Validation(format!(
            "invalid bundled Endfield lore appearance snapshot: {error}"
        ))
    })?;
    let dialogue_url = snapshot.sources.get("dialogue").cloned();
    let mut inputs = Vec::new();
    for mission in &snapshot.missions {
        for character in &mission.characters {
            let reference = lore_character_reference(character);
            let entity_id = store
                .resolve_entity_external_identity(
                    work_id,
                    &reference.provider,
                    &reference.identity,
                )?
                .or(store.resolve_sourced_entity_id(
                    work_id,
                    &reference.provider,
                    &reference.identity,
                )?);
            let Some(entity_id) = entity_id else {
                return Err(PerwigaError::NotFound(format!(
                    "Endfield lore appearance character {}:{} is not imported",
                    character.entity_type, character.source_key
                )));
            };
            inputs.push(EntityAppearanceInput {
                entity_id,
                relation_kind: "quest".into(),
                related_title: mission.title_en.clone(),
                source_provider: format!("beyondGameData@{}", snapshot.client_data_commit),
                source_identity: format!("mission:{}", mission.mission_id),
                related_source_url: dialogue_url
                    .as_ref()
                    .map(|base| format!("{base}/{}", mission.mission_id)),
                source_notes: Some(format!(
                    "{} Source locator: {}",
                    snapshot.evidence_note, mission.source_locator
                )),
                locations: Vec::new(),
            });
        }
    }
    store.import_entity_appearances(&inputs)
}

fn validate_event_snapshot(snapshot: &EventSnapshot) -> Result<(), String> {
    if snapshot.metadata.module_id != "arknights-endfield" {
        return Err("Endfield event snapshot has the wrong module ID".into());
    }
    let mut seen = HashSet::new();
    for event in &snapshot.events {
        if !seen.insert(event.source_key.as_str()) {
            return Err(format!(
                "duplicate Endfield event source key {}",
                event.source_key
            ));
        }
    }
    Ok(())
}

pub fn recurring_event_title(
    title: &str,
    patch_start: &str,
    patch_end: &str,
    occurrence_index: Option<u16>,
) -> String {
    let patch = if patch_start == patch_end {
        patch_start.to_string()
    } else {
        format!("{patch_start}–{patch_end}")
    };
    match occurrence_index {
        Some(index) => format!("{title} {patch}.{index}"),
        None => format!("{title} {patch}"),
    }
}

fn curated_event_display_title(event: &CuratedEvent) -> String {
    if !event.recurring {
        return event.title.clone();
    }
    recurring_event_title(
        &event.title,
        &event.patch_start,
        event.patch_end.as_deref().unwrap_or(&event.patch_start),
        event.occurrence_index,
    )
}

pub fn curated_calendar_events() -> Result<Vec<CalendarEventInput>, PerwigaError> {
    let snapshot = curated_event_snapshot()?;
    snapshot
        .events
        .iter()
        .map(|event| {
            let patch_end = event
                .patch_end
                .clone()
                .unwrap_or_else(|| event.patch_start.clone());
            Ok(CalendarEventInput {
                title: curated_event_display_title(event),
                starts_at: event.starts_at.clone(),
                ends_at: event.ends_at.clone(),
                is_all_day: event.is_all_day,
                source_url: Some(event.source_url.clone()),
                source_provider: snapshot.metadata.source_provider.clone(),
                source_identity: Some(event.source_key.clone()),
                event_type: Some(event.event_type.clone()),
                time_zone: Some(snapshot.metadata.time_zone.clone()),
                patch_start: Some(event.patch_start.clone()),
                patch_end: Some(patch_end),
                occurrence_index: event.occurrence_index,
                schedule_note: event.schedule_note.clone(),
                source_checked_at: Some(snapshot.metadata.source_checked_at.clone()),
            })
        })
        .collect()
}

pub fn import_curated_event_entities(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<EntityImportSummary> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "arknights-endfield" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Arknights: Endfield module"
        )));
    }

    let snapshot = curated_event_snapshot()?;
    let inputs = snapshot
        .events
        .iter()
        .map(|event| {
            let title = curated_event_display_title(event);
            let patch_end = event.patch_end.as_deref().unwrap_or(&event.patch_start);
            let published_end = event.ends_at.as_deref().unwrap_or("No published end time");
            SourcedEntityInput {
                source_provider: snapshot.metadata.source_provider.clone(),
                source_identity: event.source_key.clone(),
                entity: EntityInput {
                    entity_type: "event".to_string(),
                    official_english_name: title.clone(),
                    // The curated schedule currently has no verified original-language title.
                    // Preserve the English catalog anchor instead of inventing a localization.
                    official_original_name: title,
                    official_vietnamese_name: None,
                    automatic_vietnamese_translation: None,
                    english_description: event.schedule_note.clone(),
                    other_information: Some(format!(
                        "Curated event source key: {}. Type: {}. Start: {}. End: {}. Time zone: {}. Patch: {}–{}. Source: {}. Checked {}. The confirmed schedule snapshot does not provide verified original-language or Vietnamese titles; the English schedule title is repeated in the required original-name field.",
                        event.source_key,
                        event.event_type,
                        event.starts_at,
                        published_end,
                        snapshot.metadata.time_zone,
                        event.patch_start,
                        patch_end,
                        event.source_url,
                        snapshot.metadata.source_checked_at
                    )),
                },
                aliases: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    store.import_sourced_entities(work_id, &inputs)
}

fn operator_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "operator" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("Official source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_operator(source_key).map(|operator| operator.source_key.as_str())
}

fn weapon_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "weapon" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("Curated source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_weapon(source_key).map(|weapon| weapon.source_key.as_str())
}

fn item_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "item" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("Curated source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_item(source_key).map(|item| item.source_key.as_str())
}

fn gear_set_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "gear-set" {
        return None;
    }
    let (_, suffix) = entity
        .other_information
        .as_deref()?
        .split_once("Curated Gear Set source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_gear_set(source_key).map(|record| record.source_key.as_str())
}

fn essence_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "essence" {
        return None;
    }
    let (_, suffix) = entity
        .other_information
        .as_deref()?
        .split_once("Curated Essence source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_essence(source_key).map(|record| record.source_key.as_str())
}

fn region_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "region" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let suffix = information
        .split_once("Curated Region source key: ")
        .or_else(|| information.split_once("Curated source key: "))?
        .1;
    let source_key = suffix.split('.').next()?.trim();
    curated_region(source_key).map(|record| record.source_key.as_str())
}

fn rarity_color(rarity: u8) -> Option<&'static str> {
    match rarity {
        1 => Some("#6f7b84"),
        2 => Some("#5aa273"),
        3 => Some("#8a9aa8"),
        4 => Some("#55a8ff"),
        5 => Some("#d9a441"),
        6 => Some("#b51e3f"),
        _ => None,
    }
}

fn event_instant(value: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}

fn last_ended_headhunting_event(source_key: &str) -> Option<&'static CuratedEvent> {
    curated_event_snapshot()
        .ok()?
        .events
        .iter()
        .filter(|event| {
            event.event_type == "Headhunting"
                && event.ends_at.is_some()
                && event
                    .featured_operator_keys
                    .iter()
                    .any(|key| key == source_key)
        })
        .max_by_key(|event| event.ends_at.as_deref().and_then(event_instant))
}

fn previous_headhunting_gap(
    current_event: &CuratedEvent,
    source_key: &str,
) -> Option<EventPreviousGapPresentation> {
    if current_event.event_type != "Headhunting" {
        return None;
    }
    let current_start = event_instant(&current_event.starts_at)?;
    let previous = curated_event_snapshot()
        .ok()?
        .events
        .iter()
        .filter(|event| {
            event.source_key != current_event.source_key
                && event.event_type == "Headhunting"
                && event
                    .featured_operator_keys
                    .iter()
                    .any(|key| key == source_key)
        })
        .filter_map(|event| {
            let ended_at = event.ends_at.as_deref()?;
            let end = event_instant(ended_at)?;
            (end <= current_start).then_some((event, end))
        })
        .max_by_key(|(_, end)| *end)?;
    Some(EventPreviousGapPresentation {
        days: current_start.signed_duration_since(previous.1).num_days(),
        event_title: previous.0.title.clone(),
        ended_at: previous.0.ends_at.clone()?,
    })
}

impl LibraryModule for ArknightsEndfieldModule {
    fn id(&self) -> &'static str {
        "arknights-endfield"
    }
    fn kind(&self) -> WorkKind {
        WorkKind::Game
    }
    fn display_name(&self) -> &'static str {
        "Arknights: Endfield"
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
    fn lore_schema(&self) -> Option<&'static LoreSchemaDefinition> {
        Some(&LORE_SCHEMA)
    }
    fn theme(&self) -> ThemeDefinition {
        THEME
    }
    fn entity_presentation(&self, entity: &WikiEntity) -> Option<EntityPresentation> {
        match entity.entity_type.as_str() {
            "operator" => {
                let source_key = operator_source_key(entity)?;
                let operator = curated_operator(source_key)?;
                let mut facets = BTreeMap::from([
                    ("role".to_string(), operator.class.clone()),
                    ("element".to_string(), operator.element.clone()),
                    ("weapon_type".to_string(), operator.weapon_type.clone()),
                ]);
                if let Some(race) = &operator.race {
                    facets.insert("race".to_string(), race.clone());
                }
                if let Some(subrace) = &operator.subrace {
                    facets.insert("subrace".to_string(), subrace.clone());
                }
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/arknights-endfield/operators/{source_key}.webp"
                    ),
                    accent_color: rarity_color(operator.rarity)?.to_string(),
                    context_label: None,
                    context_icon_url: None,
                    label: format!("{}★", operator.rarity),
                    rarity: Some(operator.rarity),
                    facets,
                    facet_values: BTreeMap::new(),
                })
            }
            "npc" => Some(EntityPresentation {
                thumbnail_url: "/assets/placeholders/character.svg".to_string(),
                accent_color: THEME.accent.to_string(),
                context_label: None,
                context_icon_url: None,
                label: "NPC".to_string(),
                rarity: None,
                facets: BTreeMap::new(),
                facet_values: BTreeMap::new(),
            }),
            "weapon" => {
                let source_key = weapon_source_key(entity)?;
                let weapon = curated_weapon(source_key)?;
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/arknights-endfield/weapons/{source_key}.png"
                    ),
                    accent_color: rarity_color(weapon.rarity)?.to_string(),
                    context_label: None,
                    context_icon_url: None,
                    label: format!("{}★", weapon.rarity),
                    rarity: Some(weapon.rarity),
                    facets: BTreeMap::from([(
                        "weapon_type".to_string(),
                        weapon.weapon_type.clone(),
                    )]),
                    facet_values: BTreeMap::new(),
                })
            }
            "item" => {
                let source_key = item_source_key(entity)?;
                let item = curated_item(source_key)?;
                let mut facets = BTreeMap::new();
                if let Some(region) = &item.region {
                    facets.insert("region".to_string(), region.english.clone());
                }
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/arknights-endfield/items/{}.webp",
                        item.client_icon_id
                    ),
                    accent_color: rarity_color(item.rarity)?.to_string(),
                    context_label: None,
                    context_icon_url: None,
                    label: format!("{}★", item.rarity),
                    rarity: Some(item.rarity),
                    facets,
                    facet_values: BTreeMap::from([(
                        "item_type".to_string(),
                        item.classifications.clone(),
                    )]),
                })
            }
            "gear-set" => {
                let source_key = gear_set_source_key(entity)?;
                let gear = curated_gear_set(source_key)?;
                let rarity = *gear.rarities.last()?;
                let first_member = gear.member_item_ids.first()?;
                let thumbnail = curated_item(first_member)
                    .map(|item| item.client_icon_id.as_str())
                    .unwrap_or(first_member);
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/arknights-endfield/items/{thumbnail}.webp"
                    ),
                    accent_color: rarity_color(rarity)?.to_string(),
                    context_label: None,
                    context_icon_url: None,
                    label: format!(
                        "{}★",
                        gear.rarities
                            .iter()
                            .map(u8::to_string)
                            .collect::<Vec<_>>()
                            .join("–")
                    ),
                    rarity: Some(rarity),
                    facets: BTreeMap::new(),
                    facet_values: BTreeMap::new(),
                })
            }
            "essence" => {
                let source_key = essence_source_key(entity)?;
                let essence = curated_essence(source_key)?;
                let term = |kind: &str| {
                    essence
                        .terms
                        .iter()
                        .find(|term| term.kind == kind)
                        .map(|term| term.official_english_name.clone())
                };
                let mut facets =
                    BTreeMap::from([("weapon_type".to_string(), essence.weapon_type.clone())]);
                for (key, kind) in [
                    ("essence_primary", "Primary attribute"),
                    ("essence_secondary", "Secondary attribute"),
                    ("essence_skill", "Skill effect"),
                ] {
                    if let Some(value) = term(kind) {
                        facets.insert(key.to_string(), value);
                    }
                }
                Some(EntityPresentation {
                    thumbnail_url: format!(
                        "/assets/modules/arknights-endfield/items/{}.webp",
                        essence.client_icon_id
                    ),
                    accent_color: rarity_color(essence.rarity)?.to_string(),
                    context_label: None,
                    context_icon_url: None,
                    label: format!("{}★", essence.rarity),
                    rarity: Some(essence.rarity),
                    facets,
                    facet_values: BTreeMap::new(),
                })
            }
            "region" => {
                let source_key = region_source_key(entity)?;
                let region = curated_region(source_key)?;
                let region_type = region_type_label(&region.subtype);
                let parent = region
                    .parent_source_key
                    .as_deref()
                    .and_then(curated_region)
                    .map(|record| record.official_english_name.clone());
                let mut facets = BTreeMap::from([("region_type".to_string(), region_type.clone())]);
                if let Some(parent) = &parent {
                    facets.insert("parent_region".to_string(), parent.clone());
                }
                Some(EntityPresentation {
                    thumbnail_url: String::new(),
                    accent_color: THEME.accent.to_string(),
                    context_label: parent,
                    context_icon_url: None,
                    label: region_type,
                    rarity: None,
                    facets,
                    facet_values: BTreeMap::new(),
                })
            }
            _ => None,
        }
    }

    fn entity_catalog_label(&self, entity: &WikiEntity) -> Option<String> {
        let source_key = essence_source_key(entity)?;
        Some(curated_essence(source_key)?.catalog_label.clone())
    }

    fn entity_event_recency(&self, entity: &WikiEntity) -> Option<EntityEventRecencyPresentation> {
        let source_key = operator_source_key(entity)?;
        let operator = curated_operator(source_key)?;
        if operator.rarity != 6 {
            return None;
        }
        let event = last_ended_headhunting_event(source_key)?;
        Some(EntityEventRecencyPresentation {
            heading: "Last limited banner".into(),
            event_title: event.title.clone(),
            ended_at: event.ends_at.clone()?,
        })
    }

    fn calendar_event_presentation(
        &self,
        event: &CalendarEvent,
    ) -> Option<CalendarEventPresentation> {
        let source_identity = event.source_identity.as_deref()?;
        let curated_event = curated_event_snapshot()
            .ok()?
            .events
            .iter()
            .find(|curated| curated.source_key == source_identity)?;
        if curated_event.featured_operator_keys.is_empty() {
            return None;
        }

        let featured_entities = curated_event
            .featured_operator_keys
            .iter()
            .map(|source_key| {
                let operator = curated_operator(source_key)?;
                Some(EventFeaturedEntityPresentation {
                    display_name: operator.official_english_name.clone(),
                    thumbnail_url: format!(
                        "/assets/modules/arknights-endfield/operators/{source_key}.webp"
                    ),
                    accent_color: rarity_color(operator.rarity)?.to_string(),
                    label: format!("{}★", operator.rarity),
                    previous_event_gap: previous_headhunting_gap(curated_event, source_key),
                })
            })
            .collect::<Option<Vec<_>>>()?;

        Some(CalendarEventPresentation {
            heading: curated_event.featured_heading.clone().unwrap_or_else(|| {
                match (curated_event.event_type.as_str(), featured_entities.len()) {
                    ("Headhunting", 1) => "Rate-UP Operator".to_string(),
                    (_, 1) => "Featured Operator".to_string(),
                    _ => "Featured Operators".to_string(),
                }
            }),
            featured_entities,
        })
    }
}

static MODULE: ArknightsEndfieldModule = ArknightsEndfieldModule;

pub fn module() -> &'static dyn LibraryModule {
    &MODULE
}

pub fn register(registry: &mut ModuleRegistry) -> perwiga_core::Result<()> {
    registry.register(module())
}

pub fn operator_thumbnail(source_key: &str) -> Option<&'static [u8]> {
    match source_key {
        "akekuri" => Some(include_bytes!("../assets/operators/akekuri.webp")),
        "alesh" => Some(include_bytes!("../assets/operators/alesh.webp")),
        "antal" => Some(include_bytes!("../assets/operators/antal.webp")),
        "arclight" => Some(include_bytes!("../assets/operators/arclight.webp")),
        "ardelia" => Some(include_bytes!("../assets/operators/ardelia.webp")),
        "avywenna" => Some(include_bytes!("../assets/operators/avywenna.webp")),
        "camille" => Some(include_bytes!("../assets/operators/camille.webp")),
        "catcher" => Some(include_bytes!("../assets/operators/catcher.webp")),
        "chen" => Some(include_bytes!("../assets/operators/chen.webp")),
        "dapan" => Some(include_bytes!("../assets/operators/dapan.webp")),
        "ember" => Some(include_bytes!("../assets/operators/ember.webp")),
        "endministrator1" => Some(include_bytes!("../assets/operators/endministrator1.webp")),
        "endministrator2" => Some(include_bytes!("../assets/operators/endministrator2.webp")),
        "estella" => Some(include_bytes!("../assets/operators/estella.webp")),
        "fluorite" => Some(include_bytes!("../assets/operators/fluorite.webp")),
        "gilberta" => Some(include_bytes!("../assets/operators/gilberta.webp")),
        "laevatain" => Some(include_bytes!("../assets/operators/laevatain.webp")),
        "lastrite" => Some(include_bytes!("../assets/operators/lastrite.webp")),
        "lifeng" => Some(include_bytes!("../assets/operators/lifeng.webp")),
        "liino" => Some(include_bytes!("../assets/operators/liino.webp")),
        "lizhiyan" => Some(include_bytes!("../assets/operators/lizhiyan.webp")),
        "mifu" => Some(include_bytes!("../assets/operators/mifu.webp")),
        "perlica" => Some(include_bytes!("../assets/operators/perlica.webp")),
        "pogranichnik" => Some(include_bytes!("../assets/operators/pogranichnik.webp")),
        "purrche" => Some(include_bytes!("../assets/operators/purrche.webp")),
        "rossi" => Some(include_bytes!("../assets/operators/rossi.webp")),
        "snowshine" => Some(include_bytes!("../assets/operators/snowshine.webp")),
        "tangtang" => Some(include_bytes!("../assets/operators/tangtang.webp")),
        "typhoea" => Some(include_bytes!("../assets/operators/typhoea.webp")),
        "wulfgard" => Some(include_bytes!("../assets/operators/wulfgard.webp")),
        "xaihi" => Some(include_bytes!("../assets/operators/xaihi.webp")),
        "yvonne" => Some(include_bytes!("../assets/operators/yvonne.webp")),
        "zhuangfy" => Some(include_bytes!("../assets/operators/zhuangfy.webp")),
        _ => None,
    }
}

include!(concat!(env!("OUT_DIR"), "/weapon_thumbnails.rs"));
include!(concat!(env!("OUT_DIR"), "/item_thumbnails.rs"));

pub fn import_curated_aliases(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<CuratedAliasImportSummary> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "arknights-endfield" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Arknights: Endfield module"
        )));
    }

    let mut resolved = HashSet::new();
    let mut inputs = Vec::new();
    for entity in store.list_entities(work_id, Some("operator"))? {
        let Some(source_key) = operator_source_key(&entity) else {
            continue;
        };
        let Some(operator) = curated_operator(source_key) else {
            continue;
        };
        resolved.insert(source_key.to_string());
        inputs.push(EntityAliasBatchInput {
            entity_id: entity.id.clone(),
            alias: AliasInput {
                value: operator.han_viet_name.clone(),
                language: Some("vi".to_string()),
                kind: "han-viet".to_string(),
                label: Some("Hán-Việt (generated)".to_string()),
                notes: Some(format!(
                    "Generated search alias from the verified official Simplified Chinese name; not an official Vietnamese localization. Sources: https://endfield.hypergryph.com/operator and https://hvdic.thivien.net/transcript.php#trans. Checked {}. Confidence E (generated).",
                    curated_snapshot().checked_at
                )),
            },
        });

        let primary_names = [
            Some(entity.official_english_name.as_str()),
            Some(entity.official_original_name.as_str()),
            entity.official_vietnamese_name.as_deref(),
            entity.automatic_vietnamese_translation.as_deref(),
        ];
        for localization in &curated_snapshot().official_localizations {
            let Some(value) = localization.names.get(source_key) else {
                continue;
            };
            if primary_names.iter().flatten().any(|name| *name == value) {
                continue;
            }
            inputs.push(EntityAliasBatchInput {
                entity_id: entity.id.clone(),
                alias: AliasInput {
                    value: value.clone(),
                    language: Some(localization.language.clone()),
                    kind: "official-localization".to_string(),
                    label: Some(localization.label.clone()),
                    notes: Some(format!(
                        "Official localized operator name from {}. Checked {}. Confidence A.",
                        localization.source_url,
                        curated_snapshot().checked_at
                    )),
                },
            });
        }

        for alias in curated_snapshot()
            .additional_aliases
            .iter()
            .filter(|alias| alias.source_key == source_key)
        {
            inputs.push(EntityAliasBatchInput {
                entity_id: entity.id.clone(),
                alias: AliasInput {
                    value: alias.value.clone(),
                    language: alias.language.clone(),
                    kind: alias.kind.clone(),
                    label: Some(alias.label.clone()),
                    notes: Some(alias.notes.clone()),
                },
            });
        }
    }

    let batch = store.add_aliases_batch(&inputs)?;
    let unresolved = curated_operators()
        .iter()
        .filter(|operator| !resolved.contains(&operator.source_key))
        .map(|operator| operator.source_key.clone())
        .collect();
    Ok(CuratedAliasImportSummary {
        inserted: batch.inserted,
        unchanged: batch.unchanged,
        unresolved,
    })
}

#[cfg(test)]
mod tests {
    use perwiga_core::{
        model::{CalendarEvent, EntityInput, WikiEntity},
        Store, WorkKind,
    };

    use super::*;

    fn operator(source_key: &str) -> WikiEntity {
        WikiEntity {
            id: "operator-1".into(),
            work_id: "work-1".into(),
            entity_type: "operator".into(),
            official_english_name: "Akekuri".into(),
            official_original_name: "秋栗".into(),
            official_vietnamese_name: Some("Akekuri".into()),
            automatic_vietnamese_translation: None,
            english_description: None,
            other_information: Some(format!("Official source key: {source_key}. Rarity: 4★.")),
            created_at: "2026-08-23T00:00:00Z".into(),
            updated_at: "2026-08-23T00:00:00Z".into(),
        }
    }

    fn weapon(source_key: &str) -> WikiEntity {
        WikiEntity {
            id: "weapon-1".into(),
            work_id: "work-1".into(),
            entity_type: "weapon".into(),
            official_english_name: "Aggeloslayer".into(),
            official_original_name: "天使杀手".into(),
            official_vietnamese_name: Some("Sát Thủ Diệt Aggeloi".into()),
            automatic_vietnamese_translation: None,
            english_description: None,
            other_information: Some(format!(
                "Curated source key: {source_key}. Catalog index: 007/77. Rarity: 4-star."
            )),
            created_at: "2026-08-24T00:00:00Z".into(),
            updated_at: "2026-08-24T00:00:00Z".into(),
        }
    }

    fn item(source_key: &str) -> WikiEntity {
        WikiEntity {
            id: "item-1".into(),
            work_id: "work-1".into(),
            entity_type: "item".into(),
            official_english_name: "Jincao".into(),
            official_original_name: "锦草".into(),
            official_vietnamese_name: Some("Cỏ Sao Lụa".into()),
            automatic_vietnamese_translation: None,
            english_description: None,
            other_information: Some(format!(
                "Curated source key: {source_key}. Catalog index: 037/731. Rarity: 1-star."
            )),
            created_at: "2026-08-24T00:00:00Z".into(),
            updated_at: "2026-08-24T00:00:00Z".into(),
        }
    }

    fn gear_set(source_key: &str) -> WikiEntity {
        WikiEntity {
            id: "gear-set-1".into(),
            work_id: "work-1".into(),
            entity_type: "gear-set".into(),
            official_english_name: "Roving MSGR".into(),
            official_original_name: "巡行信使".into(),
            official_vietnamese_name: Some("LLV Cơ Động".into()),
            automatic_vietnamese_translation: None,
            english_description: None,
            other_information: Some(format!("Curated Gear Set source key: {source_key}.")),
            created_at: "2026-08-26T00:00:00Z".into(),
            updated_at: "2026-08-26T00:00:00Z".into(),
        }
    }

    fn essence(source_key: &str) -> WikiEntity {
        WikiEntity {
            id: "essence-1".into(),
            work_id: "work-1".into(),
            entity_type: "essence".into(),
            official_english_name: "Flawless Essence".into(),
            official_original_name: "无瑕基质".into(),
            official_vietnamese_name: Some("Tinh Chất Hoàn Hảo".into()),
            automatic_vietnamese_translation: None,
            english_description: None,
            other_information: Some(format!("Curated Essence source key: {source_key}.")),
            created_at: "2026-08-26T00:00:00Z".into(),
            updated_at: "2026-08-26T00:00:00Z".into(),
        }
    }

    fn region(source_key: &str) -> WikiEntity {
        let record = curated_region(source_key).expect("curated region");
        WikiEntity {
            id: "region-1".into(),
            work_id: "work-1".into(),
            entity_type: "region".into(),
            official_english_name: record.official_english_name.clone(),
            official_original_name: record.official_original_name.clone(),
            official_vietnamese_name: record.official_vietnamese_name.clone(),
            automatic_vietnamese_translation: None,
            english_description: None,
            other_information: Some(format!(
                "Curated Region source key: {source_key}. Subtype: {}.",
                record.subtype
            )),
            created_at: "2026-08-26T00:00:00Z".into(),
            updated_at: "2026-08-26T00:00:00Z".into(),
        }
    }

    fn calendar_event(source_key: &str) -> CalendarEvent {
        let input = curated_calendar_events()
            .expect("curated events")
            .into_iter()
            .find(|event| event.source_identity.as_deref() == Some(source_key))
            .unwrap_or_else(|| panic!("missing event {source_key}"));
        CalendarEvent {
            id: format!("event-{source_key}"),
            work_id: "work-1".into(),
            title: input.title,
            starts_at: input.starts_at,
            ends_at: input.ends_at,
            is_all_day: input.is_all_day,
            source_url: input.source_url,
            source_provider: input.source_provider,
            source_identity: input.source_identity,
            event_type: input.event_type,
            time_zone: input.time_zone,
            patch_start: input.patch_start,
            patch_end: input.patch_end,
            occurrence_index: input.occurrence_index,
            schedule_note: input.schedule_note,
            source_checked_at: input.source_checked_at,
            created_at: "2026-08-24T00:00:00Z".into(),
            updated_at: "2026-08-24T00:00:00Z".into(),
        }
    }

    #[test]
    fn operator_presentation_uses_module_owned_rarity_and_thumbnail_metadata() {
        let presentation = module()
            .entity_presentation(&operator("akekuri"))
            .expect("curated operator presentation");

        assert_eq!(presentation.label, "4★");
        assert_eq!(presentation.rarity, Some(4));
        assert_eq!(
            presentation.facets,
            BTreeMap::from([
                ("element".to_string(), "Heat".to_string()),
                ("race".to_string(), "Perro".to_string()),
                ("role".to_string(), "Vanguard".to_string()),
                ("weapon_type".to_string(), "Sword".to_string()),
            ])
        );
        assert_eq!(presentation.accent_color, "#55a8ff");
        assert_eq!(
            presentation.thumbnail_url,
            "/assets/modules/arknights-endfield/operators/akekuri.webp"
        );
        assert!(operator_thumbnail("akekuri").is_some());
        assert!(operator_thumbnail("../akekuri").is_none());

        let five_star = module()
            .entity_presentation(&operator("purrche"))
            .expect("five-star presentation");
        assert_eq!(five_star.label, "5★");
        assert_eq!(five_star.accent_color, "#d9a441");

        let six_star = module()
            .entity_presentation(&operator("typhoea"))
            .expect("six-star presentation");
        assert_eq!(six_star.label, "6★");
        assert_eq!(six_star.accent_color, "#b51e3f");

        let liino = module()
            .entity_presentation(&operator("liino"))
            .expect("Liino presentation");
        assert_eq!(
            liino.facets.get("subrace").map(String::as_str),
            Some("Djall")
        );

        for key in ["role", "element", "race", "subrace", "weapon_type"] {
            assert!(
                module()
                    .entity_facets()
                    .iter()
                    .any(|facet| facet.key == key),
                "missing {key} facet"
            );
        }

        for operator in &curated_snapshot().operators {
            let values = [
                ("role", Some(operator.class.as_str())),
                ("element", Some(operator.element.as_str())),
                ("weapon_type", Some(operator.weapon_type.as_str())),
                ("race", operator.race.as_deref()),
                ("subrace", operator.subrace.as_deref()),
            ];
            for (key, value) in values {
                let Some(value) = value else { continue };
                let definition = module()
                    .entity_facets()
                    .iter()
                    .find(|facet| facet.key == key)
                    .expect("facet definition");
                assert!(
                    definition
                        .options
                        .iter()
                        .any(|option| option.value == value),
                    "{value} from {} is missing from {key} options",
                    operator.source_key
                );
            }
        }
    }

    #[test]
    fn gear_set_and_essence_catalogs_import_additively_with_module_facets() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("Endfield work");
        let first = import_curated_gear_sets_and_essences(&mut store, &work.id)
            .expect("first catalog import");
        assert_eq!(first.inserted, 177);
        let second = import_curated_gear_sets_and_essences(&mut store, &work.id)
            .expect("idempotent catalog import");
        assert_eq!(second.unchanged, 177);
        assert_eq!(
            store
                .list_entities(&work.id, Some("gear-set"))
                .unwrap()
                .len(),
            23
        );
        assert_eq!(
            store
                .list_entities(&work.id, Some("essence"))
                .unwrap()
                .len(),
            154
        );

        let gear = module()
            .entity_presentation(&gear_set("suit_agi01"))
            .expect("Gear Set presentation");
        assert_eq!(gear.label, "3–4★");
        assert_eq!(gear.rarity, Some(4));
        assert!(gear
            .thumbnail_url
            .contains("item_equip_t2_suit_agi01_body_01"));

        let essence_entity = essence("gem_claym_0003_442");
        let essence = module()
            .entity_presentation(&essence_entity)
            .expect("Essence presentation");
        assert_eq!(essence.rarity, Some(5));
        assert_eq!(essence.facets["weapon_type"], "Greatsword");
        assert_eq!(essence.facets["essence_primary"], "Strength Boost");
        assert_eq!(essence.facets["essence_secondary"], "Attack Boost");
        assert_eq!(essence.facets["essence_skill"], "Suppression");
        assert_eq!(
            module().entity_catalog_label(&essence_entity).as_deref(),
            Some(
                "Flawless Essence — Industry 0.1 — Strength Boost Lv.4 · Attack Boost Lv.4 · Suppression Lv.2"
            )
        );
        assert_eq!(store.integrity_check().unwrap(), "ok");
    }

    #[test]
    fn region_catalog_imports_additively_with_hierarchy_and_facets() {
        let snapshot = curated_region_snapshot();
        assert_eq!(snapshot.regions.len(), 43);
        assert!(snapshot
            .regions
            .iter()
            .all(|record| !record.han_viet_name.trim().is_empty()));
        assert_eq!(
            snapshot
                .regions
                .iter()
                .filter(|record| record.subtype == "playable-region")
                .count(),
            2
        );
        assert_eq!(
            snapshot
                .regions
                .iter()
                .filter(|record| record.subtype == "subregion")
                .count(),
            14
        );
        let source_keys = snapshot
            .regions
            .iter()
            .map(|record| record.source_key.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(source_keys.len(), snapshot.regions.len());
        for record in &snapshot.regions {
            if let Some(parent) = &record.parent_source_key {
                assert!(
                    source_keys.contains(parent.as_str()),
                    "{} has an unknown parent {parent}",
                    record.source_key
                );
            }
            assert!(!record.related_source_keys.contains(&record.source_key));
        }

        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("Endfield work");
        let first = import_curated_regions(&mut store, &work.id).expect("first region import");
        assert_eq!(first.inserted, 43);
        assert_eq!(first.aliases_inserted, 43);
        let second = import_curated_regions(&mut store, &work.id).expect("idempotent import");
        assert_eq!(second.unchanged, 43);
        assert_eq!(second.aliases_unchanged, 43);

        let wuling = module()
            .entity_presentation(&region("wuling"))
            .expect("Wuling presentation");
        assert_eq!(wuling.facets["region_type"], "Playable region");
        assert_eq!(wuling.facets["parent_region"], "Talos-II");
        assert_eq!(wuling.context_label.as_deref(), Some("Talos-II"));

        let jingyu = module()
            .entity_presentation(&region("jingyu-valley"))
            .expect("Jingyu Valley presentation");
        assert_eq!(jingyu.facets["region_type"], "Subregion");
        assert_eq!(jingyu.facets["parent_region"], "Wuling");
        assert_eq!(jingyu.context_label.as_deref(), Some("Wuling"));

        for key in ["region_type", "parent_region"] {
            let facet = module()
                .entity_facets()
                .iter()
                .find(|facet| facet.key == key)
                .unwrap_or_else(|| panic!("missing {key} facet"));
            assert_eq!(facet.entity_types, &["region"]);
        }
        assert_eq!(store.integrity_check().unwrap(), "ok");
    }

    #[test]
    fn weapon_presentation_uses_curated_icon_and_rarity_metadata() {
        let presentation = module()
            .entity_presentation(&weapon("aggeloslayer"))
            .expect("curated weapon presentation");

        assert_eq!(presentation.label, "4★");
        assert_eq!(presentation.rarity, Some(4));
        assert_eq!(
            presentation.facets.get("weapon_type").map(String::as_str),
            Some("Polearm")
        );
        assert_eq!(presentation.accent_color, "#55a8ff");
        assert_eq!(
            presentation.thumbnail_url,
            "/assets/modules/arknights-endfield/weapons/aggeloslayer.png"
        );
        assert!(weapon_thumbnail("aggeloslayer").is_some());
        assert!(weapon_thumbnail("../aggeloslayer").is_none());

        let three_star = module()
            .entity_presentation(&weapon("opero-77"))
            .expect("three-star presentation");
        assert_eq!(three_star.label, "3★");
        assert_eq!(three_star.accent_color, "#8a9aa8");

        let six_star = module()
            .entity_presentation(&weapon("amaranthine-tassel"))
            .expect("six-star presentation");
        assert_eq!(six_star.label, "6★");
        assert_eq!(six_star.accent_color, "#b51e3f");

        assert_eq!(curated_weapon_snapshot().weapons.len(), 77);
        for weapon in &curated_weapon_snapshot().weapons {
            assert!(
                weapon_thumbnail(&weapon.source_key).is_some(),
                "missing thumbnail for {}",
                weapon.source_key
            );
        }

        let weapon_type = module()
            .entity_facets()
            .iter()
            .find(|facet| facet.key == "weapon_type")
            .expect("weapon type facet");
        assert_eq!(weapon_type.key, "weapon_type");
        assert_eq!(weapon_type.entity_types, &["operator", "weapon", "essence"]);
        assert_eq!(weapon_type.options.len(), 5);
    }

    #[test]
    fn item_presentation_preserves_many_types_and_explicit_region_metadata() {
        let presentation = module()
            .entity_presentation(&item("item_plant_grass_1"))
            .expect("curated item presentation");

        assert_eq!(presentation.label, "1★");
        assert_eq!(presentation.rarity, Some(1));
        assert_eq!(presentation.accent_color, "#6f7b84");
        assert_eq!(
            presentation.thumbnail_url,
            "/assets/modules/arknights-endfield/items/item_plant_grass_1.webp"
        );
        assert_eq!(
            presentation.facets.get("region").map(String::as_str),
            Some("Wuling")
        );
        assert_eq!(
            presentation.facet_values.get("item_type"),
            Some(&vec![
                "craftable".to_string(),
                "crafting-ingredient".to_string(),
                "material".to_string(),
                "gatherable".to_string(),
                "natural-resource".to_string(),
                "plant".to_string(),
            ])
        );
        assert!(item_thumbnail("item_plant_grass_1").is_some());
        assert!(item_thumbnail("../item_plant_grass_1").is_none());

        let item_type = module()
            .entity_facets()
            .iter()
            .find(|facet| facet.key == "item_type")
            .expect("item type facet");
        assert_eq!(item_type.entity_types, &["item"]);
        for expected in ["craftable", "crafting-ingredient", "material", "plant"] {
            assert!(
                item_type
                    .options
                    .iter()
                    .any(|option| option.value == expected),
                "missing {expected} item type"
            );
        }

        let region = module()
            .entity_facets()
            .iter()
            .find(|facet| facet.key == "region")
            .expect("item region facet");
        assert_eq!(region.entity_types, &["item"]);
        assert_eq!(
            region
                .options
                .iter()
                .map(|option| option.value)
                .collect::<Vec<_>>(),
            vec!["Valley IV", "Wuling"]
        );

        assert_eq!(curated_item_snapshot().items.len(), 731);
        assert_eq!(
            curated_item_snapshot()
                .items
                .iter()
                .filter(|item| item.region.is_some())
                .count(),
            45
        );
        for item in &curated_item_snapshot().items {
            assert!(
                item_thumbnail(&item.client_icon_id).is_some(),
                "missing thumbnail for {} ({})",
                item.source_key,
                item.client_icon_id
            );
            for classification in &item.classifications {
                assert!(
                    item_type
                        .options
                        .iter()
                        .any(|option| option.value == classification),
                    "missing {classification} option for {}",
                    item.source_key
                );
            }
        }
    }

    #[test]
    fn curated_multilingual_alias_import_is_transactional_and_idempotent() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("Endfield work");
        let akekuri = store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "operator".into(),
                    official_english_name: "Akekuri".into(),
                    official_original_name: "秋栗".into(),
                    official_vietnamese_name: Some("Akekuri".into()),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some("Official source key: akekuri. Rarity: 4★.".into()),
                },
            )
            .expect("operator");
        let chen = store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "operator".into(),
                    official_english_name: "Chen Qianyu".into(),
                    official_original_name: "陈千语".into(),
                    official_vietnamese_name: Some("Chen Qianyu".into()),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some("Official source key: chen. Rarity: 5★.".into()),
                },
            )
            .expect("operator");

        let first = import_curated_aliases(&mut store, &work.id).expect("first import");
        assert_eq!(first.inserted, 12);
        assert_eq!(first.unchanged, 0);
        assert_eq!(first.unresolved.len(), 31);

        let akekuri_aliases = store.list_aliases(&akekuri.id).expect("Akekuri aliases");
        assert_eq!(akekuri_aliases.len(), 5);
        assert!(akekuri_aliases.iter().any(|alias| {
            alias.value == "Thu Lật"
                && alias.language.as_deref() == Some("vi")
                && alias.kind == "han-viet"
                && alias.label.as_deref() == Some("Hán-Việt (generated)")
        }));
        assert!(akekuri_aliases.iter().any(|alias| {
            alias.value == "アケクリ"
                && alias.language.as_deref() == Some("ja")
                && alias.kind == "official-localization"
        }));

        let chen_aliases = store.list_aliases(&chen.id).expect("Chen aliases");
        assert_eq!(chen_aliases.len(), 7);
        assert!(chen_aliases.iter().any(|alias| {
            alias.value == "小陈"
                && alias.language.as_deref() == Some("zh-CN")
                && alias.kind == "fanfic"
                && alias.label.as_deref() == Some("[fanfic] Web-novel shorthand")
        }));

        let second = import_curated_aliases(&mut store, &work.id).expect("second import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 12);
        assert_eq!(store.list_aliases(&akekuri.id).unwrap().len(), 5);
        assert_eq!(store.list_aliases(&chen.id).unwrap().len(), 7);
    }

    #[test]
    fn recurring_event_titles_encode_patch_span_and_same_patch_occurrence() {
        assert_eq!(
            recurring_event_title("Sanity Supply", "1.4", "1.4", Some(2)),
            "Sanity Supply 1.4.2"
        );
        assert_eq!(
            recurring_event_title("Echoes of War", "1.4", "1.5", None),
            "Echoes of War 1.4–1.5"
        );
        assert_eq!(
            recurring_event_title("Monumental Etching: Beastly Howl", "1.4", "1.4", None),
            "Monumental Etching: Beastly Howl 1.4"
        );
    }

    #[test]
    fn curated_event_snapshot_has_patch_aware_asia_server_windows() {
        let events = curated_calendar_events().expect("curated event snapshot");
        assert!(events.len() >= 55, "expected a broad event history");
        assert!(events
            .iter()
            .all(|event| event.time_zone.as_deref() == Some("Asia server (UTC+8)")));

        let foreseeable = events
            .iter()
            .find(|event| event.source_identity.as_deref() == Some("sanity-supply-1-4-2"))
            .expect("foreseeable Sanity Supply occurrence");
        assert_eq!(foreseeable.title, "Sanity Supply 1.4.2");
        assert_eq!(foreseeable.event_type.as_deref(), Some("Supply"));
        assert_eq!(foreseeable.starts_at, "2026-08-26T04:00:00+08:00");
        assert_eq!(
            foreseeable.ends_at.as_deref(),
            Some("2026-09-02T04:00:00+08:00")
        );

        let cross_patch = events
            .iter()
            .find(|event| event.source_identity.as_deref() == Some("cleanse-and-rinse-1-1-1-2"))
            .expect("cross-patch event");
        assert_eq!(cross_patch.patch_start.as_deref(), Some("1.1"));
        assert_eq!(cross_patch.patch_end.as_deref(), Some("1.2"));
    }

    #[test]
    fn curated_lore_seed_is_valid_ordered_and_links_region_subjects() {
        let batch = curated_lore_candidate_batch().expect("curated lore batch");
        assert_eq!(batch.module_id, "arknights-endfield");
        assert!(batch.periods.len() >= 4);
        assert_eq!(batch.events.len(), 200);
        assert_eq!(
            batch
                .subjects
                .iter()
                .filter(|subject| matches!(subject.proposed_type.as_str(), "operator" | "npc"))
                .count(),
            167
        );

        let mut store = Store::open_in_memory().expect("in-memory store");
        let work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("work");
        import_curated_operators(&mut store, &work.id).expect("curated operators");
        import_curated_npcs(&mut store, &work.id).expect("curated NPCs");
        import_curated_regions(&mut store, &work.id).expect("curated regions");
        import_curated_lore(&mut store, &work.id).expect("curated lore import");

        let graph = store
            .list_lore_graph(&work.id, None, None, 500, None)
            .expect("lore graph");
        assert_eq!(graph.events.len(), batch.events.len());
        assert!(graph
            .events
            .windows(2)
            .all(|events| events[0].period_order.unwrap_or(i64::MAX)
                <= events[1].period_order.unwrap_or(i64::MAX)));
        let talos = graph
            .subjects
            .iter()
            .find(|subject| subject.attested_name == "Talos-II")
            .expect("Talos-II subject");
        assert!(talos.wiki_entity_id.is_some());
        let avywenna = graph
            .subjects
            .iter()
            .find(|subject| subject.attested_name == "Avywenna")
            .expect("Avywenna subject");
        assert!(avywenna.wiki_entity_id.is_some());
        let avywenna_appearances = store
            .list_entity_appearances(avywenna.wiki_entity_id.as_deref().expect("Avywenna entity"))
            .expect("Avywenna appearances");
        assert!(avywenna_appearances.iter().any(|appearance| {
            appearance.relation_kind == "quest"
                && appearance.related_title == "Visitor from the Band I"
        }));
        let catalog_characters = graph
            .subjects
            .iter()
            .filter(|subject| matches!(subject.proposed_type.as_str(), "operator" | "npc"))
            .collect::<Vec<_>>();
        assert_eq!(catalog_characters.len(), 167);
        assert!(catalog_characters
            .iter()
            .all(|subject| subject.wiki_entity_id.is_some()));
        let involved_character_ids = graph
            .involvements
            .iter()
            .map(|involvement| involvement.subject_id.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(
            catalog_characters
                .iter()
                .filter(|subject| involved_character_ids.contains(subject.id.as_str()))
                .count(),
            71
        );
        let visitor = graph
            .events
            .iter()
            .find(|event| event.title_en == "Visitor from the Band I")
            .expect("Avywenna quest");
        let character_subject_ids = catalog_characters
            .iter()
            .map(|subject| subject.id.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(
            graph
                .involvements
                .iter()
                .filter(|involvement| {
                    involvement.event_id.as_deref() == Some(visitor.id.as_str())
                        && character_subject_ids.contains(involvement.subject_id.as_str())
                })
                .count(),
            5
        );
        let wuling = graph
            .events
            .iter()
            .find(|event| event.title_en == "Chapter II sends Endministrators to Wuling")
            .expect("Wuling quest");
        assert!(visitor.period_order < wuling.period_order);
        assert!(graph.relations.len() >= 2);
    }

    #[test]
    fn curated_character_catalogs_import_additively_with_stable_source_identities() {
        let mut store = Store::open_in_memory().expect("in-memory store");
        let work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("work");

        let operators = import_curated_operators(&mut store, &work.id).expect("operators");
        let npcs = import_curated_npcs(&mut store, &work.id).expect("NPCs");
        assert_eq!(operators.inserted, 33);
        assert_eq!(npcs.inserted, 134);

        let avywenna = store
            .list_entities(&work.id, Some("operator"))
            .expect("operator list")
            .into_iter()
            .find(|entity| entity.official_english_name == "Avywenna")
            .expect("Avywenna");
        assert!(avywenna
            .other_information
            .as_deref()
            .is_some_and(|value| value.contains("Official source key: avywenna.")));
        let presentation = module()
            .entity_presentation(&avywenna)
            .expect("Avywenna presentation");
        assert_eq!(
            presentation.thumbnail_url,
            "/assets/modules/arknights-endfield/operators/avywenna.webp"
        );

        let second_operators =
            import_curated_operators(&mut store, &work.id).expect("operators again");
        let second_npcs = import_curated_npcs(&mut store, &work.id).expect("NPCs again");
        assert_eq!(second_operators.inserted, 0);
        assert_eq!(second_operators.unchanged, 33);
        assert_eq!(second_npcs.inserted, 0);
        assert_eq!(second_npcs.unchanged, 134);
    }

    #[test]
    fn headhunting_events_resolve_bundled_promoted_operator_portraits() {
        let snapshot = curated_event_snapshot().expect("curated event snapshot");
        let headhunting = snapshot
            .events
            .iter()
            .filter(|event| event.event_type == "Headhunting")
            .collect::<Vec<_>>();

        assert_eq!(headhunting.len(), 11);
        for event in &headhunting {
            assert!(
                !event.featured_operator_keys.is_empty(),
                "{} must identify at least one featured Operator",
                event.source_key
            );
            for source_key in &event.featured_operator_keys {
                let operator = curated_operator(source_key).unwrap_or_else(|| {
                    panic!(
                        "{} references unknown Operator {source_key}",
                        event.source_key
                    )
                });
                assert!(
                    operator_thumbnail(operator.source_key.as_str()).is_some(),
                    "{} has no bundled portrait",
                    operator.source_key
                );
            }
        }

        let fest = headhunting
            .iter()
            .find(|event| event.source_key == "fest-of-brilliance-1-2")
            .expect("Fest of Brilliance");
        assert_eq!(fest.featured_heading.as_deref(), Some("Featured Operators"));
        assert_eq!(
            fest.featured_operator_keys,
            ["laevatain", "gilberta", "ardelia", "pogranichnik"]
        );
    }

    #[test]
    fn six_star_operator_recency_uses_the_latest_ended_headhunting_event() {
        let recency = module()
            .entity_event_recency(&operator("laevatain"))
            .expect("Laevatain banner recency");
        assert_eq!(recency.heading, "Last limited banner");
        assert_eq!(recency.event_title, "Fest of Brilliance");
        assert_eq!(recency.ended_at, "2026-06-05T06:00:00+08:00");

        assert!(module()
            .entity_event_recency(&operator("akekuri"))
            .is_none());
        assert!(module()
            .entity_event_recency(&operator("typhoea"))
            .is_none());
    }

    #[test]
    fn group_banner_preview_has_one_gap_per_returning_operator() {
        let presentation = module()
            .calendar_event_presentation(&calendar_event("fest-of-brilliance-1-2"))
            .expect("Fest of Brilliance presentation");
        let laevatain = presentation
            .featured_entities
            .iter()
            .find(|operator| operator.display_name == "Laevatain")
            .expect("Laevatain");
        let gilberta = presentation
            .featured_entities
            .iter()
            .find(|operator| operator.display_name == "Gilberta")
            .expect("Gilberta");
        let ardelia = presentation
            .featured_entities
            .iter()
            .find(|operator| operator.display_name == "Ardelia")
            .expect("Ardelia");

        assert_eq!(laevatain.previous_event_gap.as_ref().unwrap().days, 96);
        assert_eq!(
            laevatain.previous_event_gap.as_ref().unwrap().event_title,
            "Scars of the Forge"
        );
        assert_eq!(gilberta.previous_event_gap.as_ref().unwrap().days, 79);
        assert_eq!(
            gilberta.previous_event_gap.as_ref().unwrap().event_title,
            "The Floaty Messenger"
        );
        assert!(ardelia.previous_event_gap.is_none());
    }

    #[test]
    fn narrative_events_resolve_source_backed_featured_operator_portraits() {
        let snapshot = curated_event_snapshot().expect("curated event snapshot");
        for (source_identity, operator_key) in [
            ("red-knight-1-1", "rossi"),
            ("fistful-reflections-1-3", "mifu"),
            ("danse-macabre-1-3", "camille"),
            ("star-streaking-boundaries-1-4", "liino"),
        ] {
            let event = snapshot
                .events
                .iter()
                .find(|event| event.source_key == source_identity)
                .unwrap_or_else(|| panic!("missing narrative event {source_identity}"));
            assert_eq!(event.event_type, "Narrative");
            assert_eq!(event.featured_operator_keys, [operator_key]);
            assert_eq!(event.featured_heading.as_deref(), Some("Featured Operator"));
            assert!(operator_thumbnail(operator_key).is_some());
        }
    }
}
