use perwiga_core::{Capability, EntityTypeDefinition, LibraryModule, ModuleRegistry, WorkKind};

pub struct GenericNovelModule;

static CAPABILITIES: &[Capability] = &[
    Capability::ManualContent,
    Capability::EntitySchema,
    Capability::UpdateFeed,
];
static ENTITY_TYPES: &[EntityTypeDefinition] = &[
    EntityTypeDefinition {
        key: "character",
        display_name: "Character",
        description: "A named character in the novel.",
    },
    EntityTypeDefinition {
        key: "place",
        display_name: "Place",
        description: "A named place or setting in the novel.",
    },
];

impl LibraryModule for GenericNovelModule {
    fn id(&self) -> &'static str {
        "generic"
    }
    fn kind(&self) -> WorkKind {
        WorkKind::Novel
    }
    fn display_name(&self) -> &'static str {
        "Generic novel"
    }
    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }
    fn entity_types(&self) -> &'static [EntityTypeDefinition] {
        ENTITY_TYPES
    }
}

static MODULE: GenericNovelModule = GenericNovelModule;

pub fn module() -> &'static dyn LibraryModule {
    &MODULE
}

pub fn register(registry: &mut ModuleRegistry) -> perwiga_core::Result<()> {
    registry.register(module())
}
