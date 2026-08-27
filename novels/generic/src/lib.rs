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
        description: "A named main, supporting, or side character in the novel.",
    },
    EntityTypeDefinition {
        key: "place",
        display_name: "Place",
        description: "A named place or setting in the novel.",
    },
    EntityTypeDefinition {
        key: "region",
        display_name: "Region",
        description: "A named geographic, political, or world region in the novel.",
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

#[cfg(test)]
mod tests {
    use super::module;

    #[test]
    fn generic_novels_keep_side_characters_and_regions_as_common_types() {
        let types = module()
            .entity_types()
            .iter()
            .map(|definition| definition.key)
            .collect::<Vec<_>>();
        assert_eq!(types, vec!["character", "place", "region"]);
        assert!(module().entity_types()[0]
            .description
            .contains("side character"));
    }
}
