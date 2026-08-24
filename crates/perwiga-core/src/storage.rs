use std::{path::Path, time::Duration};

use chrono::{SecondsFormat, Utc};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use url::Url;
use uuid::Uuid;

use crate::{
    error::{PerwigaError, Result},
    model::{
        AliasBatchSummary, AliasInput, CalendarEvent, CalendarEventImportSummary,
        CalendarEventInput, Checklist, ChecklistItem, EntityAlias, EntityAliasBatchInput,
        EntityImportSummary, EntityInput, EntityPatch, FeedItem, FeedSource, LibraryWork,
        LinkedFolder, Note, NoteAttachment, SourcedEntityInput, WikiEntity, WikiEntityDetail,
    },
    module::WorkKind,
};

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(Duration::from_secs(5))?;
        let mut store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> Result<()> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );",
        )?;
        let current: Option<i64> =
            self.connection
                .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                    row.get(0)
                })?;
        if current.unwrap_or(0) < 1 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(SCHEMA_V1)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![1_i64, now()],
            )?;
            transaction.commit()?;
        }
        if current.unwrap_or(0) < 2 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(SCHEMA_V2)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![2_i64, now()],
            )?;
            transaction.commit()?;
        }
        if current.unwrap_or(0) < 3 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(SCHEMA_V3)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![3_i64, now()],
            )?;
            transaction.commit()?;
        }
        Ok(())
    }

    pub fn integrity_check(&self) -> Result<String> {
        Ok(self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?)
    }

    pub fn foreign_key_violations(&self) -> Result<i64> {
        Ok(self.connection.query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_check",
            [],
            |row| row.get(0),
        )?)
    }

    pub fn insert_work(
        &self,
        kind: WorkKind,
        module_id: &str,
        display_name: &str,
    ) -> Result<LibraryWork> {
        let display_name = required_text(display_name, "work display name")?;
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO library_works
             (id, work_kind, module_id, display_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, kind.as_str(), module_id, display_name, timestamp],
        )?;
        self.get_work(&id)?
            .ok_or_else(|| PerwigaError::NotFound("new work".into()))
    }

    pub fn list_works(&self) -> Result<Vec<LibraryWork>> {
        let mut statement = self.connection.prepare(
            "SELECT id, work_kind, module_id, display_name, created_at, updated_at
             FROM library_works ORDER BY display_name COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([], row_to_work)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn get_work(&self, id: &str) -> Result<Option<LibraryWork>> {
        self.connection
            .query_row(
                "SELECT id, work_kind, module_id, display_name, created_at, updated_at
                 FROM library_works WHERE id = ?1",
                [id],
                row_to_work,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn insert_entity(&self, work_id: &str, input: &EntityInput) -> Result<WikiEntity> {
        let work_exists = self.get_work(work_id)?.is_some();
        if !work_exists {
            return Err(PerwigaError::NotFound(format!("work {work_id}")));
        }
        let english = required_text(&input.official_english_name, "official English name")?;
        let original = required_text(&input.official_original_name, "official original name")?;
        let entity_type = required_text(&input.entity_type, "entity type")?;
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO wiki_entities
             (id, work_id, entity_type, official_english_name, official_original_name,
              official_vietnamese_name, automatic_vietnamese_translation, english_description,
              other_information, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                id,
                work_id,
                entity_type,
                english,
                original,
                non_empty_option(
                    input.official_vietnamese_name.as_deref(),
                    "official Vietnamese name"
                )?,
                non_empty_option(
                    input.automatic_vietnamese_translation.as_deref(),
                    "automatic Vietnamese translation"
                )?,
                non_empty_option(input.english_description.as_deref(), "English description")?,
                non_empty_option(input.other_information.as_deref(), "other information")?,
                timestamp,
            ],
        )?;
        self.get_entity(&id)?
            .ok_or_else(|| PerwigaError::NotFound("new entity".into()))
    }

    /// Imports source-owned entities atomically without replacing existing rows.
    /// Stable source identities make repeat imports idempotent while preserving
    /// any personal edits made after the first import.
    pub fn import_sourced_entities(
        &mut self,
        work_id: &str,
        inputs: &[SourcedEntityInput],
    ) -> Result<EntityImportSummary> {
        let transaction = self.connection.transaction()?;
        let work_exists = transaction
            .query_row(
                "SELECT 1 FROM library_works WHERE id = ?1",
                [work_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !work_exists {
            return Err(PerwigaError::NotFound(format!("work {work_id}")));
        }

        let mut summary = EntityImportSummary::default();
        let timestamp = now();
        for input in inputs {
            let source_provider = required_text(&input.source_provider, "entity source provider")?;
            let source_identity = required_text(&input.source_identity, "entity source identity")?;
            let entity_type = required_text(&input.entity.entity_type, "entity type")?;
            let english =
                required_text(&input.entity.official_english_name, "official English name")?;
            let original = required_text(
                &input.entity.official_original_name,
                "official original name",
            )?;
            let vietnamese = non_empty_option(
                input.entity.official_vietnamese_name.as_deref(),
                "official Vietnamese name",
            )?;
            let automatic_vietnamese = non_empty_option(
                input.entity.automatic_vietnamese_translation.as_deref(),
                "automatic Vietnamese translation",
            )?;
            let description = non_empty_option(
                input.entity.english_description.as_deref(),
                "English description",
            )?;
            let information = non_empty_option(
                input.entity.other_information.as_deref(),
                "other information",
            )?;

            let existing_id = transaction
                .query_row(
                    "SELECT id FROM wiki_entities
                     WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
                    params![work_id, source_provider, source_identity],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            let entity_id = match existing_id {
                Some(id) => {
                    summary.unchanged += 1;
                    id
                }
                None => {
                    let name_collision = transaction
                        .query_row(
                            "SELECT id FROM wiki_entities
                             WHERE work_id = ?1 AND entity_type = ?2
                               AND official_english_name = ?3 COLLATE NOCASE
                             LIMIT 1",
                            params![work_id, entity_type, english],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()?;
                    if name_collision.is_some() {
                        return Err(PerwigaError::Conflict(format!(
                            "an unlinked {entity_type} named {english} already exists in work {work_id}"
                        )));
                    }

                    let id = new_id();
                    transaction.execute(
                        "INSERT INTO wiki_entities
                         (id, work_id, entity_type, official_english_name,
                          official_original_name, official_vietnamese_name,
                          automatic_vietnamese_translation, english_description,
                          other_information, created_at, updated_at, source_provider,
                          source_identity)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, ?12)",
                        params![
                            id,
                            work_id,
                            entity_type,
                            english,
                            original,
                            vietnamese,
                            automatic_vietnamese,
                            description,
                            information,
                            timestamp,
                            source_provider,
                            source_identity,
                        ],
                    )?;
                    summary.inserted += 1;
                    id
                }
            };

            for alias in &input.aliases {
                let value = required_text(&alias.value, "alias")?;
                let kind = required_text(&alias.kind, "alias kind")?;
                let changed = transaction.execute(
                    "INSERT INTO entity_aliases
                     (id, entity_id, value, language, kind, label, notes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT (entity_id, value, kind) DO NOTHING",
                    params![
                        new_id(),
                        entity_id,
                        value,
                        non_empty_option(alias.language.as_deref(), "alias language")?,
                        kind,
                        non_empty_option(alias.label.as_deref(), "alias label")?,
                        non_empty_option(alias.notes.as_deref(), "alias notes")?,
                    ],
                )?;
                if changed == 1 {
                    summary.aliases_inserted += 1;
                } else {
                    summary.aliases_unchanged += 1;
                }
            }
        }

        transaction.commit()?;
        Ok(summary)
    }

    pub fn update_entity(&self, id: &str, patch: &EntityPatch) -> Result<WikiEntity> {
        let timestamp = now();
        let values = [
            non_empty_option(
                patch.official_english_name.as_deref(),
                "official English name",
            )?,
            non_empty_option(
                patch.official_original_name.as_deref(),
                "official original name",
            )?,
            non_empty_option(
                patch.official_vietnamese_name.as_deref(),
                "official Vietnamese name",
            )?,
            non_empty_option(
                patch.automatic_vietnamese_translation.as_deref(),
                "automatic Vietnamese translation",
            )?,
            non_empty_option(patch.english_description.as_deref(), "English description")?,
            non_empty_option(patch.other_information.as_deref(), "other information")?,
        ];
        if values.iter().all(Option::is_none) {
            return self
                .get_entity(id)?
                .ok_or_else(|| PerwigaError::NotFound(format!("entity {id}")));
        }
        self.connection.execute(
            "UPDATE wiki_entities SET
               official_english_name = COALESCE(?1, official_english_name),
               official_original_name = COALESCE(?2, official_original_name),
               official_vietnamese_name = COALESCE(?3, official_vietnamese_name),
               automatic_vietnamese_translation = COALESCE(?4, automatic_vietnamese_translation),
               english_description = COALESCE(?5, english_description),
               other_information = COALESCE(?6, other_information),
               updated_at = ?7
             WHERE id = ?8",
            params![
                values[0], values[1], values[2], values[3], values[4], values[5], timestamp, id
            ],
        )?;
        if self.connection.changes() == 0 {
            return Err(PerwigaError::NotFound(format!("entity {id}")));
        }
        self.get_entity(id)?
            .ok_or_else(|| PerwigaError::NotFound(format!("entity {id}")))
    }

    pub fn get_entity(&self, id: &str) -> Result<Option<WikiEntity>> {
        self.connection
            .query_row(
                "SELECT id, work_id, entity_type, official_english_name, official_original_name,
                        official_vietnamese_name, automatic_vietnamese_translation, english_description,
                        other_information, created_at, updated_at
                 FROM wiki_entities WHERE id = ?1",
                [id],
                row_to_entity,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_entity_detail(&self, id: &str) -> Result<Option<WikiEntityDetail>> {
        let Some(entity) = self.get_entity(id)? else {
            return Ok(None);
        };
        Ok(Some(WikiEntityDetail {
            aliases: self.list_aliases(id)?,
            entity,
        }))
    }

    pub fn list_entities(
        &self,
        work_id: &str,
        entity_type: Option<&str>,
    ) -> Result<Vec<WikiEntity>> {
        let mut statement = self.connection.prepare(
            "SELECT id, work_id, entity_type, official_english_name, official_original_name,
                    official_vietnamese_name, automatic_vietnamese_translation, english_description,
                    other_information, created_at, updated_at
             FROM wiki_entities
             WHERE work_id = ?1 AND (?2 IS NULL OR entity_type = ?2)
             ORDER BY official_english_name COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map(params![work_id, entity_type], row_to_entity)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn search_entities(&self, work_id: &str, query: &str) -> Result<Vec<WikiEntity>> {
        let query = required_text(query, "entity search query")?;
        let pattern = format!("%{query}%");
        let mut statement = self.connection.prepare(
            "SELECT e.id, e.work_id, e.entity_type, e.official_english_name, e.official_original_name,
                    e.official_vietnamese_name, e.automatic_vietnamese_translation, e.english_description,
                    e.other_information, e.created_at, e.updated_at
             FROM wiki_entities e
             WHERE e.work_id = ?1 AND (
                 e.official_english_name LIKE ?2 COLLATE NOCASE OR
                 e.official_original_name LIKE ?2 OR
                 COALESCE(e.official_vietnamese_name, '') LIKE ?2 OR
                 COALESCE(e.automatic_vietnamese_translation, '') LIKE ?2 OR
                 EXISTS (SELECT 1 FROM entity_aliases a WHERE a.entity_id = e.id AND a.value LIKE ?2)
             )
             ORDER BY e.official_english_name COLLATE NOCASE, e.id",
        )?;
        let rows = statement.query_map(params![work_id, pattern], row_to_entity)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn list_aliases(&self, entity_id: &str) -> Result<Vec<EntityAlias>> {
        let mut statement = self.connection.prepare(
            "SELECT id, entity_id, value, language, kind, label, notes
             FROM entity_aliases WHERE entity_id = ?1 ORDER BY value COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([entity_id], row_to_alias)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn add_alias(&self, entity_id: &str, input: &AliasInput) -> Result<EntityAlias> {
        if self.get_entity(entity_id)?.is_none() {
            return Err(PerwigaError::NotFound(format!("entity {entity_id}")));
        }
        let value = required_text(&input.value, "alias")?;
        let kind = required_text(&input.kind, "alias kind")?;
        let id = new_id();
        self.connection.execute(
            "INSERT INTO entity_aliases (id, entity_id, value, language, kind, label, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (entity_id, value, kind) DO NOTHING",
            params![
                id,
                entity_id,
                value,
                non_empty_option(input.language.as_deref(), "alias language")?,
                kind,
                non_empty_option(input.label.as_deref(), "alias label")?,
                non_empty_option(input.notes.as_deref(), "alias notes")?,
            ],
        )?;
        self.connection
            .query_row(
                "SELECT id, entity_id, value, language, kind, label, notes
                 FROM entity_aliases WHERE entity_id = ?1 AND value = ?2 AND kind = ?3",
                params![entity_id, value, kind],
                row_to_alias,
            )
            .map_err(Into::into)
    }

    /// Adds aliases for multiple entities atomically. Existing aliases are
    /// retained and counted as unchanged; no alias is ever replaced.
    pub fn add_aliases_batch(
        &mut self,
        inputs: &[EntityAliasBatchInput],
    ) -> Result<AliasBatchSummary> {
        let transaction = self.connection.transaction()?;
        let mut summary = AliasBatchSummary {
            inserted: 0,
            unchanged: 0,
        };

        for input in inputs {
            let entity_exists = transaction
                .query_row(
                    "SELECT 1 FROM wiki_entities WHERE id = ?1",
                    [&input.entity_id],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !entity_exists {
                return Err(PerwigaError::NotFound(format!(
                    "entity {}",
                    input.entity_id
                )));
            }

            let value = required_text(&input.alias.value, "alias")?;
            let kind = required_text(&input.alias.kind, "alias kind")?;
            let changed = transaction.execute(
                "INSERT INTO entity_aliases (id, entity_id, value, language, kind, label, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (entity_id, value, kind) DO NOTHING",
                params![
                    new_id(),
                    input.entity_id,
                    value,
                    non_empty_option(input.alias.language.as_deref(), "alias language")?,
                    kind,
                    non_empty_option(input.alias.label.as_deref(), "alias label")?,
                    non_empty_option(input.alias.notes.as_deref(), "alias notes")?,
                ],
            )?;
            if changed == 1 {
                summary.inserted += 1;
            } else {
                summary.unchanged += 1;
            }
        }

        transaction.commit()?;
        Ok(summary)
    }

    pub fn create_note(&self, work_id: &str, title: &str, content: &str) -> Result<Note> {
        ensure_work(&self.connection, work_id)?;
        let title = required_text(title, "note title")?;
        let content = required_text(content, "note content")?;
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO notes (id, work_id, title, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, work_id, title, content, timestamp],
        )?;
        self.get_note(&id)?
            .ok_or_else(|| PerwigaError::NotFound("new note".into()))
    }

    pub fn get_note(&self, id: &str) -> Result<Option<Note>> {
        self.connection
            .query_row(
                "SELECT id, work_id, title, content, created_at, updated_at FROM notes WHERE id = ?1",
                [id],
                row_to_note,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_notes(&self, work_id: &str) -> Result<Vec<Note>> {
        let mut statement = self.connection.prepare(
            "SELECT id, work_id, title, content, created_at, updated_at
             FROM notes WHERE work_id = ?1 ORDER BY updated_at DESC, id",
        )?;
        let rows = statement.query_map([work_id], row_to_note)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn update_note(
        &self,
        id: &str,
        title: Option<&str>,
        content: Option<&str>,
    ) -> Result<Note> {
        let title = non_empty_option(title, "note title")?;
        let content = non_empty_option(content, "note content")?;
        if title.is_none() && content.is_none() {
            return self
                .get_note(id)?
                .ok_or_else(|| PerwigaError::NotFound(format!("note {id}")));
        }
        self.connection.execute(
            "UPDATE notes SET title = COALESCE(?1, title), content = COALESCE(?2, content), updated_at = ?3
             WHERE id = ?4",
            params![title, content, now(), id],
        )?;
        self.get_note(id)?
            .ok_or_else(|| PerwigaError::NotFound(format!("note {id}")))
    }

    pub fn list_note_attachments(&self, note_id: &str) -> Result<Vec<NoteAttachment>> {
        let mut statement = self.connection.prepare(
            "SELECT id, note_id, source_type, source_reference, created_at
             FROM note_attachments WHERE note_id = ?1 ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([note_id], row_to_attachment)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn attach_to_note(
        &self,
        note_id: &str,
        source_type: &str,
        source_reference: &str,
    ) -> Result<NoteAttachment> {
        if self.get_note(note_id)?.is_none() {
            return Err(PerwigaError::NotFound(format!("note {note_id}")));
        }
        let source_type = required_text(source_type, "attachment source type")?;
        let source_reference = required_text(source_reference, "attachment source reference")?;
        match source_type.as_str() {
            "local-file" => {}
            "remote-url" => {
                validate_remote_url(&source_reference)?;
            }
            other => {
                return Err(PerwigaError::Validation(format!(
                    "unsupported attachment source type {other}"
                )))
            }
        }
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO note_attachments (id, note_id, source_type, source_reference, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, note_id, source_type, source_reference, timestamp],
        )?;
        self.connection
            .query_row(
                "SELECT id, note_id, source_type, source_reference, created_at
                 FROM note_attachments WHERE id = ?1",
                [id],
                row_to_attachment,
            )
            .map_err(Into::into)
    }

    pub fn link_folder(
        &self,
        work_id: &str,
        path: &str,
        label: Option<&str>,
    ) -> Result<LinkedFolder> {
        ensure_work(&self.connection, work_id)?;
        let path = required_text(path, "linked folder path")?;
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO linked_folders (id, work_id, path, label, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                work_id,
                path,
                non_empty_option(label, "folder label")?,
                timestamp
            ],
        )?;
        self.connection
            .query_row(
                "SELECT id, work_id, path, label, created_at FROM linked_folders WHERE id = ?1",
                [id],
                row_to_folder,
            )
            .map_err(Into::into)
    }

    pub fn list_linked_folders(&self, work_id: &str) -> Result<Vec<LinkedFolder>> {
        let mut statement = self.connection.prepare(
            "SELECT id, work_id, path, label, created_at
             FROM linked_folders WHERE work_id = ?1 ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([work_id], row_to_folder)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Browse image references without changing the linked folder or its files.
    pub fn list_folder_images(&self, folder_id: &str) -> Result<Vec<String>> {
        let path: String = self
            .connection
            .query_row(
                "SELECT path FROM linked_folders WHERE id = ?1",
                [folder_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| PerwigaError::NotFound(format!("linked folder {folder_id}")))?;
        let mut images = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let is_image = entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "avif"
                    )
                });
            if is_image {
                images.push(entry.path().to_string_lossy().into_owned());
            }
        }
        images.sort();
        Ok(images)
    }

    pub fn create_checklist(&self, work_id: &str, title: &str) -> Result<Checklist> {
        ensure_work(&self.connection, work_id)?;
        let title = required_text(title, "checklist title")?;
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO checklists (id, work_id, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, work_id, title, timestamp],
        )?;
        self.connection
            .query_row(
                "SELECT id, work_id, title, created_at, updated_at FROM checklists WHERE id = ?1",
                [id],
                row_to_checklist,
            )
            .map_err(Into::into)
    }

    pub fn list_checklists(&self, work_id: &str) -> Result<Vec<Checklist>> {
        let mut statement = self.connection.prepare(
            "SELECT id, work_id, title, created_at, updated_at
             FROM checklists WHERE work_id = ?1 ORDER BY updated_at DESC, id",
        )?;
        let rows = statement.query_map([work_id], row_to_checklist)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn add_checklist_item(&self, checklist_id: &str, label: &str) -> Result<ChecklistItem> {
        let label = required_text(label, "checklist item")?;
        if !exists(
            &self.connection,
            "SELECT 1 FROM checklists WHERE id = ?1",
            checklist_id,
        )? {
            return Err(PerwigaError::NotFound(format!("checklist {checklist_id}")));
        }
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO checklist_items (id, checklist_id, label, is_complete, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, ?4, ?4)",
            params![id, checklist_id, label, timestamp],
        )?;
        self.get_checklist_item(&id)?
            .ok_or_else(|| PerwigaError::NotFound("new checklist item".into()))
    }

    pub fn toggle_checklist_item(&self, item_id: &str) -> Result<ChecklistItem> {
        let timestamp = now();
        self.connection.execute(
            "UPDATE checklist_items SET is_complete = CASE is_complete WHEN 0 THEN 1 ELSE 0 END,
             updated_at = ?1 WHERE id = ?2",
            params![timestamp, item_id],
        )?;
        self.get_checklist_item(item_id)?
            .ok_or_else(|| PerwigaError::NotFound(format!("checklist item {item_id}")))
    }

    pub fn get_checklist_item(&self, id: &str) -> Result<Option<ChecklistItem>> {
        self.connection
            .query_row(
                "SELECT id, checklist_id, label, is_complete, created_at, updated_at
                 FROM checklist_items WHERE id = ?1",
                [id],
                row_to_checklist_item,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_checklist_items(&self, checklist_id: &str) -> Result<Vec<ChecklistItem>> {
        let mut statement = self.connection.prepare(
            "SELECT id, checklist_id, label, is_complete, created_at, updated_at
             FROM checklist_items WHERE checklist_id = ?1 ORDER BY created_at, id",
        )?;
        let rows = statement.query_map([checklist_id], row_to_checklist_item)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_feed_source(
        &self,
        work_id: &str,
        url: &str,
        provenance: &str,
    ) -> Result<FeedSource> {
        ensure_work(&self.connection, work_id)?;
        let url = validate_remote_url(url)?;
        let provenance = required_text(provenance, "feed provenance")?;
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO feed_sources (id, work_id, url, provenance, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, work_id, url, provenance, timestamp],
        )?;
        self.get_feed_source(&id)?
            .ok_or_else(|| PerwigaError::NotFound("new feed source".into()))
    }

    pub fn get_feed_source(&self, id: &str) -> Result<Option<FeedSource>> {
        self.connection
            .query_row(
                "SELECT id, work_id, url, provenance, last_attempt_at, last_success_at, last_error, created_at
                 FROM feed_sources WHERE id = ?1",
                [id],
                row_to_feed_source,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn upsert_feed_items(
        &mut self,
        source_id: &str,
        items: &[FeedItem],
    ) -> Result<Vec<FeedItem>> {
        if self.get_feed_source(source_id)?.is_none() {
            return Err(PerwigaError::NotFound(format!("feed source {source_id}")));
        }
        let transaction = self.connection.transaction()?;
        for item in items {
            required_text(&item.external_identity, "feed item identity")?;
            required_text(&item.title, "feed item title")?;
            required_text(&item.discovered_at, "feed discovery time")?;
            let existing_read: Option<i64> = transaction
                .query_row(
                    "SELECT is_read FROM feed_items WHERE source_id = ?1 AND external_identity = ?2",
                    params![source_id, item.external_identity],
                    |row| row.get(0),
                )
                .optional()?;
            transaction.execute(
                "INSERT INTO feed_items
                 (id, source_id, external_identity, title, url, published_at, discovered_at,
                  is_read, item_kind, provenance)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (source_id, external_identity) DO UPDATE SET
                   title = excluded.title,
                   url = COALESCE(excluded.url, feed_items.url),
                   published_at = COALESCE(excluded.published_at, feed_items.published_at),
                   item_kind = COALESCE(excluded.item_kind, feed_items.item_kind),
                   provenance = excluded.provenance",
                params![
                    item.id,
                    source_id,
                    item.external_identity,
                    item.title,
                    item.url,
                    item.published_at,
                    item.discovered_at,
                    existing_read.unwrap_or(i64::from(item.is_read)),
                    item.item_kind,
                    item.provenance,
                ],
            )?;
        }
        transaction.commit()?;
        items
            .iter()
            .map(|item| {
                self.connection
                    .query_row(
                        "SELECT id, source_id, external_identity, title, url, published_at, discovered_at,
                                is_read, item_kind, provenance
                         FROM feed_items WHERE source_id = ?1 AND external_identity = ?2",
                        params![source_id, item.external_identity],
                        row_to_feed_item,
                    )
                    .map_err(Into::into)
            })
            .collect()
    }

    pub fn set_feed_item_read(&self, item_id: &str, is_read: bool) -> Result<FeedItem> {
        self.connection.execute(
            "UPDATE feed_items SET is_read = ?1 WHERE id = ?2",
            params![i64::from(is_read), item_id],
        )?;
        self.connection
            .query_row(
                "SELECT id, source_id, external_identity, title, url, published_at, discovered_at,
                        is_read, item_kind, provenance
                 FROM feed_items WHERE id = ?1",
                [item_id],
                row_to_feed_item,
            )
            .optional()?
            .ok_or_else(|| PerwigaError::NotFound(format!("feed item {item_id}")))
    }

    pub fn list_feed_items(&self, source_id: &str) -> Result<Vec<FeedItem>> {
        let mut statement = self.connection.prepare(
            "SELECT id, source_id, external_identity, title, url, published_at, discovered_at,
                    is_read, item_kind, provenance
             FROM feed_items WHERE source_id = ?1 ORDER BY COALESCE(published_at, discovered_at) DESC, id",
        )?;
        let rows = statement.query_map([source_id], row_to_feed_item)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn record_feed_refresh(
        &self,
        source_id: &str,
        success: bool,
        error: Option<&str>,
    ) -> Result<FeedSource> {
        if self.get_feed_source(source_id)?.is_none() {
            return Err(PerwigaError::NotFound(format!("feed source {source_id}")));
        }
        let timestamp = now();
        self.connection.execute(
            "UPDATE feed_sources SET last_attempt_at = ?1,
             last_success_at = CASE WHEN ?2 = 1 THEN ?1 ELSE last_success_at END,
             last_error = CASE WHEN ?2 = 1 THEN NULL ELSE ?3 END
             WHERE id = ?4",
            params![timestamp, i64::from(success), error, source_id],
        )?;
        self.get_feed_source(source_id)?
            .ok_or_else(|| PerwigaError::NotFound(format!("feed source {source_id}")))
    }

    pub fn create_calendar_event(
        &self,
        work_id: &str,
        title: &str,
        starts_at: &str,
        ends_at: Option<&str>,
        is_all_day: bool,
        source_url: Option<&str>,
        source_provider: &str,
    ) -> Result<CalendarEvent> {
        ensure_work(&self.connection, work_id)?;
        let title = required_text(title, "calendar event title")?;
        let starts_at = required_text(starts_at, "calendar event start")?;
        let source_provider = required_text(source_provider, "calendar event provider")?;
        validate_event_timestamp(&starts_at, is_all_day, "calendar event start")?;
        if let Some(end) = ends_at {
            required_text(end, "calendar event end")?;
            validate_event_timestamp(end, is_all_day, "calendar event end")?;
        }
        if let Some(url) = source_url {
            validate_remote_url(url)?;
        }
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO calendar_events
             (id, work_id, title, starts_at, ends_at, is_all_day, source_url, source_provider,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                work_id,
                title,
                starts_at,
                ends_at,
                i64::from(is_all_day),
                source_url,
                source_provider,
                timestamp
            ],
        )?;
        self.connection
            .query_row(
                "SELECT id, work_id, title, starts_at, ends_at, is_all_day, source_url,
                        source_provider, source_identity, event_type, time_zone, patch_start,
                        patch_end, occurrence_index, schedule_note, source_checked_at,
                        created_at, updated_at
                 FROM calendar_events WHERE id = ?1",
                [id],
                row_to_calendar_event,
            )
            .map_err(Into::into)
    }

    pub fn list_calendar_events(&self) -> Result<Vec<CalendarEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT id, work_id, title, starts_at, ends_at, is_all_day, source_url,
                    source_provider, source_identity, event_type, time_zone, patch_start,
                    patch_end, occurrence_index, schedule_note, source_checked_at,
                    created_at, updated_at
             FROM calendar_events ORDER BY starts_at, id",
        )?;
        let rows = statement.query_map([], row_to_calendar_event)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn list_calendar_events_for_work(&self, work_id: &str) -> Result<Vec<CalendarEvent>> {
        ensure_work(&self.connection, work_id)?;
        let mut statement = self.connection.prepare(
            "SELECT id, work_id, title, starts_at, ends_at, is_all_day, source_url,
                    source_provider, source_identity, event_type, time_zone, patch_start,
                    patch_end, occurrence_index, schedule_note, source_checked_at,
                    created_at, updated_at
             FROM calendar_events WHERE work_id = ?1 ORDER BY starts_at, id",
        )?;
        let rows = statement.query_map([work_id], row_to_calendar_event)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn import_calendar_events(
        &mut self,
        work_id: &str,
        inputs: &[CalendarEventInput],
    ) -> Result<CalendarEventImportSummary> {
        ensure_work(&self.connection, work_id)?;
        let inputs = inputs
            .iter()
            .map(validate_calendar_event_input)
            .collect::<Result<Vec<_>>>()?;
        let transaction = self.connection.transaction()?;
        let mut summary = CalendarEventImportSummary::default();
        for input in inputs {
            let source_identity = input.source_identity.as_deref().ok_or_else(|| {
                PerwigaError::Validation(
                    "imported calendar events require a stable source identity".into(),
                )
            })?;
            let exists = transaction
                .query_row(
                    "SELECT 1 FROM calendar_events
                     WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
                    params![work_id, input.source_provider, source_identity],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if exists {
                summary.unchanged += 1;
                continue;
            }
            let id = new_id();
            let timestamp = now();
            transaction.execute(
                "INSERT INTO calendar_events
                 (id, work_id, title, starts_at, ends_at, is_all_day, source_url,
                  source_provider, source_identity, event_type, time_zone, patch_start,
                  patch_end, occurrence_index, schedule_note, source_checked_at,
                  created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         ?14, ?15, ?16, ?17, ?17)",
                params![
                    id,
                    work_id,
                    input.title,
                    input.starts_at,
                    input.ends_at,
                    i64::from(input.is_all_day),
                    input.source_url,
                    input.source_provider,
                    input.source_identity,
                    input.event_type,
                    input.time_zone,
                    input.patch_start,
                    input.patch_end,
                    input.occurrence_index.map(i64::from),
                    input.schedule_note,
                    input.source_checked_at,
                    timestamp,
                ],
            )?;
            summary.inserted += 1;
        }
        transaction.commit()?;
        Ok(summary)
    }

    pub fn add_entity_reference(
        &self,
        owner_type: &str,
        owner_id: &str,
        entity_id: &str,
        visible_label: &str,
    ) -> Result<()> {
        let visible_label = required_text(visible_label, "entity reference label")?;
        if self.get_entity(entity_id)?.is_none() {
            return Err(PerwigaError::NotFound(format!("entity {entity_id}")));
        }
        let entity_work_id: String = self.connection.query_row(
            "SELECT work_id FROM wiki_entities WHERE id = ?1",
            [entity_id],
            |row| row.get(0),
        )?;
        let (note_id, checklist_item_id, owner_work_id) = match owner_type {
            "note" => {
                let owner_work_id: String = self
                    .connection
                    .query_row(
                        "SELECT work_id FROM notes WHERE id = ?1",
                        [owner_id],
                        |row| row.get(0),
                    )
                    .optional()?
                    .ok_or_else(|| PerwigaError::NotFound(format!("note {owner_id}")))?;
                (Some(owner_id), None, owner_work_id)
            }
            "checklist-item" => {
                let owner_work_id: String = self
                    .connection
                    .query_row(
                        "SELECT c.work_id FROM checklist_items i
                     JOIN checklists c ON c.id = i.checklist_id WHERE i.id = ?1",
                        [owner_id],
                        |row| row.get(0),
                    )
                    .optional()?
                    .ok_or_else(|| PerwigaError::NotFound(format!("checklist item {owner_id}")))?;
                (None, Some(owner_id), owner_work_id)
            }
            other => {
                return Err(PerwigaError::Validation(format!(
                    "unsupported reference owner type {other}"
                )))
            }
        };
        if owner_work_id != entity_work_id {
            return Err(PerwigaError::Conflict(
                "entity references cannot cross work ownership boundaries".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO entity_references (id, note_id, checklist_item_id, entity_id, visible_label)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![new_id(), note_id, checklist_item_id, entity_id, visible_label],
        )?;
        Ok(())
    }
}

const SCHEMA_V1: &str = r#"
CREATE TABLE library_works (
    id TEXT PRIMARY KEY,
    work_kind TEXT NOT NULL CHECK (work_kind IN ('game', 'novel')),
    module_id TEXT NOT NULL,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX library_works_kind_module_idx ON library_works (work_kind, module_id);

CREATE TABLE wiki_entities (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    entity_type TEXT NOT NULL CHECK (length(trim(entity_type)) > 0),
    official_english_name TEXT NOT NULL CHECK (length(trim(official_english_name)) > 0),
    official_original_name TEXT NOT NULL CHECK (length(trim(official_original_name)) > 0),
    official_vietnamese_name TEXT,
    automatic_vietnamese_translation TEXT,
    english_description TEXT,
    other_information TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX wiki_entities_work_type_idx ON wiki_entities (work_id, entity_type);
CREATE INDEX wiki_entities_names_idx ON wiki_entities (official_english_name, official_original_name);

CREATE TABLE entity_aliases (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL REFERENCES wiki_entities (id),
    value TEXT NOT NULL CHECK (length(trim(value)) > 0),
    language TEXT,
    kind TEXT NOT NULL CHECK (length(trim(kind)) > 0),
    label TEXT,
    notes TEXT,
    UNIQUE (entity_id, value, kind)
);
CREATE INDEX entity_aliases_value_idx ON entity_aliases (value);

CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    content TEXT NOT NULL CHECK (length(trim(content)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX notes_work_idx ON notes (work_id, updated_at);

CREATE TABLE note_attachments (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL REFERENCES notes (id),
    source_type TEXT NOT NULL CHECK (source_type IN ('local-file', 'remote-url')),
    source_reference TEXT NOT NULL CHECK (length(trim(source_reference)) > 0),
    created_at TEXT NOT NULL
);

CREATE TABLE linked_folders (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    path TEXT NOT NULL CHECK (length(trim(path)) > 0),
    label TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE checklists (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE checklist_items (
    id TEXT PRIMARY KEY,
    checklist_id TEXT NOT NULL REFERENCES checklists (id),
    label TEXT NOT NULL CHECK (length(trim(label)) > 0),
    is_complete INTEGER NOT NULL CHECK (is_complete IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE entity_references (
    id TEXT PRIMARY KEY,
    note_id TEXT REFERENCES notes (id),
    checklist_item_id TEXT REFERENCES checklist_items (id),
    entity_id TEXT NOT NULL REFERENCES wiki_entities (id),
    visible_label TEXT NOT NULL CHECK (length(trim(visible_label)) > 0),
    CHECK ((note_id IS NOT NULL AND checklist_item_id IS NULL)
        OR (note_id IS NULL AND checklist_item_id IS NOT NULL))
);
CREATE INDEX entity_references_entity_idx ON entity_references (entity_id);

CREATE TABLE feed_sources (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    url TEXT NOT NULL CHECK (length(trim(url)) > 0),
    provenance TEXT NOT NULL CHECK (length(trim(provenance)) > 0),
    last_attempt_at TEXT,
    last_success_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (work_id, url)
);
CREATE TABLE feed_items (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES feed_sources (id),
    external_identity TEXT NOT NULL CHECK (length(trim(external_identity)) > 0),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    url TEXT,
    published_at TEXT,
    discovered_at TEXT NOT NULL,
    is_read INTEGER NOT NULL CHECK (is_read IN (0, 1)),
    item_kind TEXT,
    provenance TEXT NOT NULL CHECK (length(trim(provenance)) > 0),
    UNIQUE (source_id, external_identity)
);
CREATE INDEX feed_items_unread_idx ON feed_items (source_id, is_read, discovered_at);

CREATE TABLE calendar_events (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    starts_at TEXT NOT NULL,
    ends_at TEXT,
    is_all_day INTEGER NOT NULL CHECK (is_all_day IN (0, 1)),
    source_url TEXT,
    source_provider TEXT NOT NULL CHECK (length(trim(source_provider)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX calendar_events_start_idx ON calendar_events (starts_at, id);
"#;

const SCHEMA_V2: &str = r#"
ALTER TABLE calendar_events ADD COLUMN source_identity TEXT;
ALTER TABLE calendar_events ADD COLUMN event_type TEXT;
ALTER TABLE calendar_events ADD COLUMN time_zone TEXT;
ALTER TABLE calendar_events ADD COLUMN patch_start TEXT;
ALTER TABLE calendar_events ADD COLUMN patch_end TEXT;
ALTER TABLE calendar_events ADD COLUMN occurrence_index INTEGER;
ALTER TABLE calendar_events ADD COLUMN schedule_note TEXT;
ALTER TABLE calendar_events ADD COLUMN source_checked_at TEXT;
CREATE UNIQUE INDEX calendar_events_source_identity_idx
ON calendar_events (work_id, source_provider, source_identity)
WHERE source_identity IS NOT NULL;
CREATE INDEX calendar_events_work_start_idx
ON calendar_events (work_id, starts_at, id);
"#;

const SCHEMA_V3: &str = r#"
ALTER TABLE wiki_entities ADD COLUMN source_provider TEXT;
ALTER TABLE wiki_entities ADD COLUMN source_identity TEXT;
CREATE UNIQUE INDEX wiki_entities_source_identity_idx
ON wiki_entities (work_id, source_provider, source_identity)
WHERE source_identity IS NOT NULL;
"#;

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn new_id() -> String {
    Uuid::new_v4().simple().to_string()
}

fn required_text(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(PerwigaError::Validation(format!("{field} cannot be empty")));
    }
    Ok(value.to_string())
}

fn non_empty_option(value: Option<&str>, field: &str) -> Result<Option<String>> {
    value.map(|value| required_text(value, field)).transpose()
}

fn validate_remote_url(value: &str) -> Result<String> {
    let value = required_text(value, "URL")?;
    let parsed = Url::parse(&value)
        .map_err(|error| PerwigaError::Validation(format!("invalid URL: {error}")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(PerwigaError::Validation(
            "URL must use http or https and include a host".into(),
        ));
    }
    Ok(value)
}

fn validate_event_timestamp(value: &str, is_all_day: bool, field: &str) -> Result<()> {
    if is_all_day {
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(|_| ())
            .map_err(|error| {
                PerwigaError::Validation(format!(
                    "{field} must be YYYY-MM-DD for an all-day event: {error}"
                ))
            })
    } else {
        chrono::DateTime::parse_from_rfc3339(value)
            .map(|_| ())
            .map_err(|error| PerwigaError::Validation(format!("{field} must be RFC3339: {error}")))
    }
}

fn validate_calendar_event_input(input: &CalendarEventInput) -> Result<CalendarEventInput> {
    let mut input = input.clone();
    input.title = required_text(&input.title, "calendar event title")?;
    input.starts_at = required_text(&input.starts_at, "calendar event start")?;
    input.source_provider = required_text(&input.source_provider, "calendar event provider")?;
    validate_event_timestamp(&input.starts_at, input.is_all_day, "calendar event start")?;
    input.ends_at = non_empty_option(input.ends_at.as_deref(), "calendar event end")?;
    if let Some(end) = input.ends_at.as_deref() {
        validate_event_timestamp(end, input.is_all_day, "calendar event end")?;
        let valid_order = if input.is_all_day {
            chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d")
                .expect("validated all-day event end must parse")
                >= chrono::NaiveDate::parse_from_str(&input.starts_at, "%Y-%m-%d")
                    .expect("validated all-day event start must parse")
        } else {
            chrono::DateTime::parse_from_rfc3339(end).expect("validated event end must parse")
                >= chrono::DateTime::parse_from_rfc3339(&input.starts_at)
                    .expect("validated event start must parse")
        };
        if !valid_order {
            return Err(PerwigaError::Validation(
                "calendar event end cannot be before its start".into(),
            ));
        }
    }
    input.source_url = input
        .source_url
        .as_deref()
        .map(validate_remote_url)
        .transpose()?;
    input.source_identity = non_empty_option(
        input.source_identity.as_deref(),
        "calendar event source identity",
    )?;
    input.event_type = non_empty_option(input.event_type.as_deref(), "calendar event type")?;
    input.time_zone = non_empty_option(input.time_zone.as_deref(), "calendar event time zone")?;
    input.patch_start = non_empty_option(input.patch_start.as_deref(), "calendar event patch")?;
    input.patch_end = non_empty_option(input.patch_end.as_deref(), "calendar event patch")?;
    input.schedule_note = non_empty_option(
        input.schedule_note.as_deref(),
        "calendar event schedule note",
    )?;
    input.source_checked_at = non_empty_option(
        input.source_checked_at.as_deref(),
        "calendar event source checked date",
    )?;
    Ok(input)
}

fn ensure_work(connection: &Connection, work_id: &str) -> Result<()> {
    if exists(
        connection,
        "SELECT 1 FROM library_works WHERE id = ?1",
        work_id,
    )? {
        Ok(())
    } else {
        Err(PerwigaError::NotFound(format!("work {work_id}")))
    }
}

fn exists(connection: &Connection, query: &str, id: &str) -> Result<bool> {
    Ok(connection
        .query_row(query, [id], |_| Ok(()))
        .optional()?
        .is_some())
}

fn parse_kind(value: String) -> rusqlite::Result<WorkKind> {
    match value.as_str() {
        "game" => Ok(WorkKind::Game),
        "novel" => Ok(WorkKind::Novel),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown work kind {other}"),
            )),
        )),
    }
}

fn row_to_work(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryWork> {
    Ok(LibraryWork {
        id: row.get(0)?,
        kind: parse_kind(row.get(1)?)?,
        module_id: row.get(2)?,
        display_name: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn row_to_entity(row: &rusqlite::Row<'_>) -> rusqlite::Result<WikiEntity> {
    Ok(WikiEntity {
        id: row.get(0)?,
        work_id: row.get(1)?,
        entity_type: row.get(2)?,
        official_english_name: row.get(3)?,
        official_original_name: row.get(4)?,
        official_vietnamese_name: row.get(5)?,
        automatic_vietnamese_translation: row.get(6)?,
        english_description: row.get(7)?,
        other_information: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_alias(row: &rusqlite::Row<'_>) -> rusqlite::Result<EntityAlias> {
    Ok(EntityAlias {
        id: row.get(0)?,
        entity_id: row.get(1)?,
        value: row.get(2)?,
        language: row.get(3)?,
        kind: row.get(4)?,
        label: row.get(5)?,
        notes: row.get(6)?,
    })
}

fn row_to_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        work_id: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn row_to_attachment(row: &rusqlite::Row<'_>) -> rusqlite::Result<NoteAttachment> {
    Ok(NoteAttachment {
        id: row.get(0)?,
        note_id: row.get(1)?,
        source_type: row.get(2)?,
        source_reference: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn row_to_folder(row: &rusqlite::Row<'_>) -> rusqlite::Result<LinkedFolder> {
    Ok(LinkedFolder {
        id: row.get(0)?,
        work_id: row.get(1)?,
        path: row.get(2)?,
        label: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn row_to_checklist(row: &rusqlite::Row<'_>) -> rusqlite::Result<Checklist> {
    Ok(Checklist {
        id: row.get(0)?,
        work_id: row.get(1)?,
        title: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn row_to_checklist_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChecklistItem> {
    Ok(ChecklistItem {
        id: row.get(0)?,
        checklist_id: row.get(1)?,
        label: row.get(2)?,
        is_complete: row.get::<_, i64>(3)? != 0,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn row_to_feed_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<FeedSource> {
    Ok(FeedSource {
        id: row.get(0)?,
        work_id: row.get(1)?,
        url: row.get(2)?,
        provenance: row.get(3)?,
        last_attempt_at: row.get(4)?,
        last_success_at: row.get(5)?,
        last_error: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn row_to_feed_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<FeedItem> {
    Ok(FeedItem {
        id: row.get(0)?,
        source_id: row.get(1)?,
        external_identity: row.get(2)?,
        title: row.get(3)?,
        url: row.get(4)?,
        published_at: row.get(5)?,
        discovered_at: row.get(6)?,
        is_read: row.get::<_, i64>(7)? != 0,
        item_kind: row.get(8)?,
        provenance: row.get(9)?,
    })
}

fn row_to_calendar_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<CalendarEvent> {
    Ok(CalendarEvent {
        id: row.get(0)?,
        work_id: row.get(1)?,
        title: row.get(2)?,
        starts_at: row.get(3)?,
        ends_at: row.get(4)?,
        is_all_day: row.get::<_, i64>(5)? != 0,
        source_url: row.get(6)?,
        source_provider: row.get(7)?,
        source_identity: row.get(8)?,
        event_type: row.get(9)?,
        time_zone: row.get(10)?,
        patch_start: row.get(11)?,
        patch_end: row.get(12)?,
        occurrence_index: row.get::<_, Option<i64>>(13)?.map(|value| value as u16),
        schedule_note: row.get(14)?,
        source_checked_at: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        model::{AliasInput, CalendarEventInput, EntityInput, EntityPatch, SourcedEntityInput},
        module::WorkKind,
    };

    use super::*;

    fn sourced_event() -> CalendarEventInput {
        CalendarEventInput {
            title: "Sanity Supply 1.4.2".into(),
            starts_at: "2026-08-26T04:00:00+08:00".into(),
            ends_at: Some("2026-09-02T04:00:00+08:00".into()),
            is_all_day: false,
            source_url: Some("https://endfield.gryphline.com/en-us/news/5200".into()),
            source_provider: "Arknights: Endfield official notices".into(),
            source_identity: Some("sanity-supply-1-4-2".into()),
            event_type: Some("Supply".into()),
            time_zone: Some("Asia server (UTC+8)".into()),
            patch_start: Some("1.4".into()),
            patch_end: Some("1.4".into()),
            occurrence_index: Some(2),
            schedule_note: None,
            source_checked_at: Some("2026-08-24".into()),
        }
    }

    fn sourced_character(
        source_identity: &str,
        english: &str,
        chinese: &str,
    ) -> SourcedEntityInput {
        SourcedEntityInput {
            source_provider: "Genshin Impact official character directory".into(),
            source_identity: source_identity.into(),
            entity: EntityInput {
                entity_type: "character".into(),
                official_english_name: english.into(),
                official_original_name: english.into(),
                official_vietnamese_name: Some(english.into()),
                automatic_vietnamese_translation: None,
                english_description: Some(format!("Official profile for {english}.")),
                other_information: Some(format!("Official source key: {source_identity}.")),
            },
            aliases: vec![AliasInput {
                value: chinese.into(),
                language: Some("zh-Hant".into()),
                kind: "official-localization".into(),
                label: Some("Official Traditional Chinese".into()),
                notes: Some("Confirmed first-party localization.".into()),
            }],
        }
    }

    #[test]
    fn sourced_entity_migration_preserves_existing_rows() {
        let connection = Connection::open_in_memory().expect("legacy in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL
                 );",
            )
            .expect("migration ledger");
        connection
            .execute_batch(SCHEMA_V1)
            .expect("version 1 schema");
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1)",
                ["2026-01-01T00:00:00.000Z"],
            )
            .expect("version 1 marker");
        connection
            .execute(
                "INSERT INTO library_works
                 (id, work_kind, module_id, display_name, created_at, updated_at)
                 VALUES (?1, 'game', 'arknights-endfield', 'Arknights: Endfield', ?2, ?2)",
                params!["legacy-work", "2026-01-01T00:00:00.000Z"],
            )
            .expect("legacy work");
        connection
            .execute(
                "INSERT INTO calendar_events
                 (id, work_id, title, starts_at, ends_at, is_all_day, source_url,
                  source_provider, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?8)",
                params![
                    "legacy-event",
                    "legacy-work",
                    "Existing personal event",
                    "2026-01-22T11:00:00+08:00",
                    "2026-01-23T11:00:00+08:00",
                    "https://example.com/event",
                    "Manual",
                    "2026-01-01T00:00:00.000Z",
                ],
            )
            .expect("legacy calendar row");

        let mut store = Store { connection };
        store.migrate().expect("additive migrations");

        let events = store
            .list_calendar_events_for_work("legacy-work")
            .expect("migrated calendar rows");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "legacy-event");
        assert_eq!(events[0].title, "Existing personal event");
        assert_eq!(events[0].source_provider, "Manual");
        assert_eq!(events[0].source_identity, None);
        let schema_version: i64 = store
            .connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("schema version");
        assert_eq!(schema_version, 3);
        let entity_source_columns: i64 = store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('wiki_entities')
                 WHERE name IN ('source_provider', 'source_identity')",
                [],
                |row| row.get(0),
            )
            .expect("entity source columns");
        assert_eq!(entity_source_columns, 2);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
        assert_eq!(store.foreign_key_violations().expect("foreign keys"), 0);
    }

    #[test]
    fn sourced_calendar_import_is_additive_idempotent_and_work_scoped() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let endfield = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("Endfield work");
        let other = store
            .insert_work(WorkKind::Game, "generic", "Other game")
            .expect("other work");

        let first = store
            .import_calendar_events(&endfield.id, &[sourced_event()])
            .expect("first import");
        assert_eq!(first.inserted, 1);
        assert_eq!(first.unchanged, 0);

        let second = store
            .import_calendar_events(&endfield.id, &[sourced_event()])
            .expect("idempotent import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 1);

        let events = store
            .list_calendar_events_for_work(&endfield.id)
            .expect("Endfield events");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].source_identity.as_deref(),
            Some("sanity-supply-1-4-2")
        );
        assert_eq!(events[0].event_type.as_deref(), Some("Supply"));
        assert_eq!(events[0].occurrence_index, Some(2));
        assert!(store
            .list_calendar_events_for_work(&other.id)
            .expect("other events")
            .is_empty());
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
        assert_eq!(store.foreign_key_violations().expect("foreign keys"), 0);
    }

    #[test]
    fn sourced_entity_import_is_transactional_idempotent_and_preserves_user_edits() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let genshin = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("Genshin work");
        let other = store
            .insert_work(WorkKind::Game, "generic", "Other game")
            .expect("other work");
        let inputs = [
            sourced_character("hoyoverse-content-104816", "Albedo", "阿貝多"),
            sourced_character("hoyoverse-content-104802", "Amber", "安柏"),
        ];

        let first = store
            .import_sourced_entities(&genshin.id, &inputs)
            .expect("first import");
        assert_eq!(first.inserted, 2);
        assert_eq!(first.unchanged, 0);
        assert_eq!(first.aliases_inserted, 2);
        assert_eq!(first.aliases_unchanged, 0);

        let albedo = store
            .search_entities(&genshin.id, "阿貝多")
            .expect("alias search")
            .pop()
            .expect("Albedo");
        store
            .update_entity(
                &albedo.id,
                &EntityPatch {
                    english_description: Some("Personal edited description".into()),
                    ..EntityPatch::default()
                },
            )
            .expect("personal edit");

        let second = store
            .import_sourced_entities(&genshin.id, &inputs)
            .expect("idempotent import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 2);
        assert_eq!(second.aliases_inserted, 0);
        assert_eq!(second.aliases_unchanged, 2);
        assert_eq!(
            store
                .get_entity(&albedo.id)
                .expect("edited entity")
                .expect("Albedo")
                .english_description
                .as_deref(),
            Some("Personal edited description")
        );
        assert!(store
            .list_entities(&other.id, None)
            .expect("other entities")
            .is_empty());
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
        assert_eq!(store.foreign_key_violations().expect("foreign keys"), 0);
    }

    #[test]
    fn invalid_sourced_entity_rolls_back_the_whole_batch() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("Genshin work");
        let valid = sourced_character("hoyoverse-content-104816", "Albedo", "阿貝多");
        let mut invalid = sourced_character("hoyoverse-content-104802", "Amber", "安柏");
        invalid.aliases[0].value = " ".into();

        store
            .import_sourced_entities(&work.id, &[valid, invalid])
            .expect_err("invalid batch must fail");
        assert!(store
            .list_entities(&work.id, Some("character"))
            .expect("characters")
            .is_empty());
    }
}
