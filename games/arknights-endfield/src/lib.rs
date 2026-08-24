use std::{
    collections::{BTreeMap, HashSet},
    sync::OnceLock,
};

use chrono::{DateTime, FixedOffset};
use perwiga_core::{
    model::{AliasInput, CalendarEvent, CalendarEventInput, EntityAliasBatchInput, WikiEntity},
    CalendarEventPresentation, Capability, EntityEventRecencyPresentation, EntityFacetDefinition,
    EntityFacetOption, EntityPresentation, EntityTypeDefinition, EventFeaturedEntityPresentation,
    EventPreviousGapPresentation, LibraryModule, ModuleRegistry, PerwigaError, Store,
    ThemeDefinition, WorkKind,
};
use serde::{Deserialize, Serialize};

pub struct ArknightsEndfieldModule;

static CAPABILITIES: &[Capability] = &[
    Capability::ManualContent,
    Capability::EntitySchema,
    Capability::ScheduledEvents,
];
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
        entity_types: &["operator", "weapon"],
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
    rarity: u8,
    class: String,
    element: String,
    weapon_type: String,
    race: Option<String>,
    subrace: Option<String>,
    han_viet_name: String,
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

fn curated_event_snapshot() -> Result<&'static EventSnapshot, PerwigaError> {
    static SNAPSHOT: OnceLock<Result<EventSnapshot, String>> = OnceLock::new();
    match SNAPSHOT.get_or_init(|| {
        serde_json::from_str(include_str!("../data/events.json"))
            .map_err(|error| format!("invalid bundled Endfield event snapshot: {error}"))
    }) {
        Ok(snapshot) => Ok(snapshot),
        Err(message) => Err(PerwigaError::Validation(message.clone())),
    }
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

pub fn curated_calendar_events() -> Result<Vec<CalendarEventInput>, PerwigaError> {
    let snapshot = curated_event_snapshot()?;
    if snapshot.metadata.module_id != "arknights-endfield" {
        return Err(PerwigaError::Validation(
            "Endfield event snapshot has the wrong module ID".into(),
        ));
    }
    let mut seen = HashSet::new();
    snapshot
        .events
        .iter()
        .map(|event| {
            if !seen.insert(event.source_key.clone()) {
                return Err(PerwigaError::Conflict(format!(
                    "duplicate Endfield event source key {}",
                    event.source_key
                )));
            }
            let patch_end = event
                .patch_end
                .clone()
                .unwrap_or_else(|| event.patch_start.clone());
            let title = if event.recurring {
                recurring_event_title(
                    &event.title,
                    &event.patch_start,
                    &patch_end,
                    event.occurrence_index,
                )
            } else {
                event.title.clone()
            };
            Ok(CalendarEventInput {
                title,
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
            _ => None,
        }
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
        assert_eq!(weapon_type.entity_types, &["operator", "weapon"]);
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
