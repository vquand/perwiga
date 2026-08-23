use perwiga_core::{Capability, EntityTypeDefinition, LibraryModule, ModuleRegistry, WorkKind};

pub struct GenericGameModule;

static CAPABILITIES: &[Capability] = &[
    Capability::ManualContent,
    Capability::EntitySchema,
    Capability::UpdateFeed,
];
static ENTITY_TYPES: &[EntityTypeDefinition] = &[
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

impl LibraryModule for GenericGameModule {
    fn id(&self) -> &'static str {
        "generic"
    }
    fn kind(&self) -> WorkKind {
        WorkKind::Game
    }
    fn display_name(&self) -> &'static str {
        "Generic game"
    }
    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }
    fn entity_types(&self) -> &'static [EntityTypeDefinition] {
        ENTITY_TYPES
    }
}

static MODULE: GenericGameModule = GenericGameModule;

pub fn module() -> &'static dyn LibraryModule {
    &MODULE
}

pub fn register(registry: &mut ModuleRegistry) -> perwiga_core::Result<()> {
    registry.register(module())
}
