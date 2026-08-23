use crate::{
    error::{PerwigaError, Result},
    model::{AliasInput, EntityInput, EntityPatch, FeedSource, LibraryWork, WikiEntity},
    module::{Capability, LibraryModule, ModuleRegistry, WorkKind},
    storage::Store,
};

/// Application boundary used by a UI or CLI. It enforces module/work ownership
/// before delegating durable writes to the shared SQLite store.
pub struct Application {
    store: Store,
    registry: ModuleRegistry,
}

impl Application {
    pub fn new(store: Store, registry: ModuleRegistry) -> Self {
        Self { store, registry }
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    pub fn modules(&self) -> impl Iterator<Item = &'static dyn LibraryModule> + '_ {
        self.registry.all()
    }

    pub fn create_work(
        &self,
        kind: WorkKind,
        module_id: &str,
        display_name: &str,
    ) -> Result<LibraryWork> {
        if self.registry.get(kind, module_id).is_none() {
            return Err(PerwigaError::Unsupported(format!(
                "module {kind}:{module_id} is not registered"
            )));
        }
        self.store.insert_work(kind, module_id, display_name)
    }

    pub fn create_entity(&self, work_id: &str, input: &EntityInput) -> Result<WikiEntity> {
        let work = self
            .store
            .get_work(work_id)?
            .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
        if !self
            .registry
            .supports_entity_type(work.kind, &work.module_id, &input.entity_type)
        {
            return Err(PerwigaError::Unsupported(format!(
                "entity type {} is not supported by {}:{}",
                input.entity_type, work.kind, work.module_id
            )));
        }
        self.store.insert_entity(work_id, input)
    }

    pub fn update_entity(&self, id: &str, patch: &EntityPatch) -> Result<WikiEntity> {
        self.store.update_entity(id, patch)
    }

    pub fn add_alias(
        &self,
        entity_id: &str,
        input: &AliasInput,
    ) -> Result<crate::model::EntityAlias> {
        self.store.add_alias(entity_id, input)
    }

    pub fn create_feed_source(
        &self,
        work_id: &str,
        url: &str,
        provenance: &str,
    ) -> Result<FeedSource> {
        let work = self
            .store
            .get_work(work_id)?
            .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
        let module = self
            .registry
            .get(work.kind, &work.module_id)
            .ok_or_else(|| {
                PerwigaError::Unsupported(format!(
                    "module {}:{} is not registered",
                    work.kind, work.module_id
                ))
            })?;
        if !module.capabilities().contains(&Capability::UpdateFeed) {
            return Err(PerwigaError::Unsupported(format!(
                "module {}:{} does not declare update-feed capability",
                work.kind, work.module_id
            )));
        }
        self.store.create_feed_source(work_id, url, provenance)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::{model::EntityInput, Capability, EntityTypeDefinition, ModuleRegistry};

    struct TestModule {
        id: &'static str,
        kind: WorkKind,
        types: &'static [EntityTypeDefinition],
    }

    static GAME_TYPES: &[EntityTypeDefinition] = &[EntityTypeDefinition {
        key: "npc",
        display_name: "NPC",
        description: "test",
    }];
    static NOVEL_TYPES: &[EntityTypeDefinition] = &[EntityTypeDefinition {
        key: "character",
        display_name: "Character",
        description: "test",
    }];
    static GAME_GENERIC: TestModule = TestModule {
        id: "generic",
        kind: WorkKind::Game,
        types: GAME_TYPES,
    };
    static NOVEL_GENERIC: TestModule = TestModule {
        id: "generic",
        kind: WorkKind::Novel,
        types: NOVEL_TYPES,
    };
    static ENDFIELD: TestModule = TestModule {
        id: "arknights-endfield",
        kind: WorkKind::Game,
        types: GAME_TYPES,
    };

    impl LibraryModule for TestModule {
        fn id(&self) -> &'static str {
            self.id
        }
        fn kind(&self) -> WorkKind {
            self.kind
        }
        fn display_name(&self) -> &'static str {
            self.id
        }
        fn capabilities(&self) -> &'static [Capability] {
            &[]
        }
        fn entity_types(&self) -> &'static [EntityTypeDefinition] {
            self.types
        }
    }

    fn app() -> (Application, TempDir) {
        let mut registry = ModuleRegistry::default();
        registry
            .register(&GAME_GENERIC)
            .expect("generic game registers");
        registry.register(&ENDFIELD).expect("Endfield registers");
        registry
            .register(&NOVEL_GENERIC)
            .expect("generic novel registers");
        let temp = tempfile::tempdir().expect("temporary database directory");
        let store = Store::open(temp.path().join("test.sqlite")).expect("database opens");
        (Application::new(store, registry), temp)
    }

    #[test]
    fn family_aware_registry_keeps_generic_game_and_novel_distinct() {
        let (app, _temp) = app();
        let modules: Vec<_> = app
            .modules()
            .map(|module| (module.kind(), module.id()))
            .collect();
        assert!(modules.contains(&(WorkKind::Game, "generic")));
        assert!(modules.contains(&(WorkKind::Novel, "generic")));
        assert!(modules.contains(&(WorkKind::Game, "arknights-endfield")));
    }

    #[test]
    fn endfield_work_accepts_source_confirmed_entity_types_and_rejects_unknown_types() {
        let (app, _temp) = app();
        let work = app
            .create_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("Endfield work");
        let entity = app
            .create_entity(
                &work.id,
                &EntityInput {
                    entity_type: "npc".into(),
                    official_english_name: "Perlica".into(),
                    official_original_name: "Perlica".into(),
                    official_vietnamese_name: Some("Perlica".into()),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some("Official English site operator directory".into()),
                },
            )
            .expect("NPC entity");
        assert_eq!(entity.entity_type, "npc");
        let error = app
            .create_entity(
                &work.id,
                &EntityInput {
                    entity_type: "unconfirmed-type".into(),
                    official_english_name: "Unknown".into(),
                    official_original_name: "Unknown".into(),
                    official_vietnamese_name: None,
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: None,
                },
            )
            .expect_err("unknown entity type must be rejected");
        assert!(error.to_string().contains("not supported"));
    }
}
