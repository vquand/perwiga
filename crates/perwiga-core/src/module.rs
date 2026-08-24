use std::{collections::BTreeMap, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    error::{PerwigaError, Result},
    model::{CalendarEvent, WikiEntity},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkKind {
    Game,
    Novel,
}

impl WorkKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Novel => "novel",
        }
    }
}

impl fmt::Display for WorkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkKind {
    type Err = PerwigaError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "game" => Ok(Self::Game),
            "novel" => Ok(Self::Novel),
            _ => Err(PerwigaError::Validation(format!(
                "unknown work kind {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    ManualContent,
    EntitySchema,
    UpdateFeed,
    ScheduledEvents,
    CustomView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityTypeDefinition {
    pub key: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
}

/// A value offered by a module-owned entity facet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EntityFacetOption {
    pub value: &'static str,
    pub display_name: &'static str,
}

/// Describes a title-specific filter while keeping the shared UI unaware of
/// title IDs and title-specific classification vocabularies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EntityFacetDefinition {
    pub key: &'static str,
    pub display_name: &'static str,
    pub entity_types: &'static [&'static str],
    pub options: &'static [EntityFacetOption],
}

/// Semantic presentation tokens supplied by a title module. Shared UIs consume
/// these values without branching on module IDs; modules that do not override
/// the contract receive the neutral default palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ThemeDefinition {
    pub background: &'static str,
    pub surface: &'static str,
    pub surface_raised: &'static str,
    pub surface_active: &'static str,
    pub border: &'static str,
    pub border_strong: &'static str,
    pub text: &'static str,
    pub text_muted: &'static str,
    pub text_subtle: &'static str,
    pub accent: &'static str,
    pub accent_ink: &'static str,
}

/// Optional, module-owned presentation metadata for a single entity. The
/// shared UI renders these semantic values without knowing which title
/// supplied them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntityPresentation {
    pub thumbnail_url: String,
    pub accent_color: String,
    pub label: String,
    pub rarity: Option<u8>,
    /// Single-valued facets such as an Operator's role or an Item's region.
    pub facets: BTreeMap<String, String>,
    /// Multi-valued facets such as an Item's non-exclusive classifications.
    pub facet_values: BTreeMap<String, Vec<String>>,
}

/// Optional, module-owned timing metadata for an entity's most recent
/// relevant scheduled event. The shared UI computes the live day count from
/// the supplied end timestamp without understanding title-specific rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntityEventRecencyPresentation {
    pub heading: String,
    pub event_title: String,
    pub ended_at: String,
}

/// The elapsed whole-day gap between a featured entity's previous event and
/// the event being presented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventPreviousGapPresentation {
    pub days: i64,
    pub event_title: String,
    pub ended_at: String,
}

/// A compact, module-owned entity reference shown in an event preview. The
/// shared calendar renders these values without understanding banner rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventFeaturedEntityPresentation {
    pub display_name: String,
    pub thumbnail_url: String,
    pub accent_color: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_event_gap: Option<EventPreviousGapPresentation>,
}

/// Optional, module-owned presentation metadata for a calendar event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CalendarEventPresentation {
    pub heading: String,
    pub featured_entities: Vec<EventFeaturedEntityPresentation>,
}

impl Default for ThemeDefinition {
    fn default() -> Self {
        Self {
            background: "#101216",
            surface: "#171a20",
            surface_raised: "#1d2128",
            surface_active: "#252b33",
            border: "#343b45",
            border_strong: "#4b5663",
            text: "#f4f5f6",
            text_muted: "#b2b8c0",
            text_subtle: "#7f8994",
            accent: "#8fc7ff",
            accent_ink: "#07131f",
        }
    }
}

pub trait LibraryModule: Sync {
    fn id(&self) -> &'static str;
    fn kind(&self) -> WorkKind;
    fn display_name(&self) -> &'static str;
    fn capabilities(&self) -> &'static [Capability];
    fn entity_types(&self) -> &'static [EntityTypeDefinition];
    fn entity_facets(&self) -> &'static [EntityFacetDefinition] {
        &[]
    }
    /// Returns trusted, compile-time theme tokens for generic shared views.
    fn theme(&self) -> ThemeDefinition {
        ThemeDefinition::default()
    }
    fn entity_presentation(&self, _entity: &WikiEntity) -> Option<EntityPresentation> {
        None
    }
    fn entity_event_recency(&self, _entity: &WikiEntity) -> Option<EntityEventRecencyPresentation> {
        None
    }
    fn calendar_event_presentation(
        &self,
        _event: &CalendarEvent,
    ) -> Option<CalendarEventPresentation> {
        None
    }
}

#[derive(Default)]
pub struct ModuleRegistry {
    modules: Vec<&'static dyn LibraryModule>,
}

impl ModuleRegistry {
    pub fn register(&mut self, module: &'static dyn LibraryModule) -> Result<()> {
        if self
            .modules
            .iter()
            .any(|registered| registered.kind() == module.kind() && registered.id() == module.id())
        {
            return Err(PerwigaError::Conflict(format!(
                "module {}:{} is already registered",
                module.kind(),
                module.id()
            )));
        }
        self.modules.push(module);
        Ok(())
    }

    pub fn get(&self, kind: WorkKind, id: &str) -> Option<&'static dyn LibraryModule> {
        self.modules
            .iter()
            .copied()
            .find(|module| module.kind() == kind && module.id() == id)
    }

    pub fn all(&self) -> impl Iterator<Item = &'static dyn LibraryModule> + '_ {
        self.modules.iter().copied()
    }

    pub fn supports_entity_type(&self, kind: WorkKind, module_id: &str, entity_type: &str) -> bool {
        self.get(kind, module_id).is_some_and(|module| {
            module
                .entity_types()
                .iter()
                .any(|definition| definition.key == entity_type)
        })
    }
}
