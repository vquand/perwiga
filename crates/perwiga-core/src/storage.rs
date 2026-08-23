use std::{path::Path, time::Duration};

use chrono::{SecondsFormat, Utc};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use url::Url;
use uuid::Uuid;

use crate::{
    error::{PerwigaError, Result},
    model::{
        AliasBatchSummary, AliasInput, CalendarEvent, Checklist, ChecklistItem, EntityAlias,
        EntityAliasBatchInput, EntityInput, EntityPatch, FeedItem, FeedSource, LibraryWork,
        LinkedFolder, Note, NoteAttachment, WikiEntity, WikiEntityDetail,
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
                "SELECT id, work_id, title, starts_at, ends_at, is_all_day, source_url, source_provider,
                        created_at, updated_at
                 FROM calendar_events WHERE id = ?1",
                [id],
                row_to_calendar_event,
            )
            .map_err(Into::into)
    }

    pub fn list_calendar_events(&self) -> Result<Vec<CalendarEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT id, work_id, title, starts_at, ends_at, is_all_day, source_url, source_provider,
                    created_at, updated_at
             FROM calendar_events ORDER BY starts_at, id",
        )?;
        let rows = statement.query_map([], row_to_calendar_event)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
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
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}
