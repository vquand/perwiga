use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};

use chrono::{SecondsFormat, Utc};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use url::Url;
use uuid::Uuid;

use crate::{
    error::{PerwigaError, Result},
    lore::{
        EntityExternalIdentity, LoreAssertionKind, LoreCandidateBatch, LoreCandidateRecord,
        LoreCertainty, LoreClaim, LoreClaimCandidate, LoreClaimEvidence, LoreEvent,
        LoreEventCandidate, LoreEventDetail, LoreEventRelation, LoreEventRelationCandidate,
        LoreEvidence, LoreEvidenceCandidate, LoreEvidenceStance, LoreImportSummary,
        LoreInvolvement, LorePeriod, LorePeriodCandidate, LoreRelationKind, LoreSource,
        LoreSourceCandidate, LoreSubject, LoreSubjectCandidate, LoreTimeKind, LoreTimePrecision,
    },
    model::{
        AliasBatchSummary, AliasInput, CalendarEvent, CalendarEventImportSummary,
        CalendarEventInput, Checklist, ChecklistItem, EntityAlias, EntityAliasBatchInput,
        EntityAppearance, EntityAppearanceInput, EntityImportSummary, EntityInput, EntityPatch,
        FeedItem, FeedSource, LibraryWork, LinkedFolder, Note, NoteAttachment, SourcedEntityInput,
        WikiEntity, WikiEntityDetail,
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
        if current.unwrap_or(0) < 4 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(SCHEMA_V4)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![4_i64, now()],
            )?;
            transaction.commit()?;
        }
        if current.unwrap_or(0) < 5 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(SCHEMA_V5)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![5_i64, now()],
            )?;
            transaction.commit()?;
        }
        if current.unwrap_or(0) < 6 {
            let transaction = self.connection.transaction()?;
            transaction.execute_batch(SCHEMA_V6)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![6_i64, now()],
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
                               AND source_provider IS NULL
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

    /// Adds a stable module/source identity without changing the multilingual
    /// wiki row. This lets title-owned snapshots resolve entities whose common
    /// names are not unique (for example, two entries with the same display
    /// name) while preserving the existing entity contract.
    pub fn add_entity_external_identity(
        &self,
        work_id: &str,
        entity_id: &str,
        source_provider: &str,
        source_identity: &str,
    ) -> Result<EntityExternalIdentity> {
        ensure_work(&self.connection, work_id)?;
        let owner: Option<String> = self
            .connection
            .query_row(
                "SELECT work_id FROM wiki_entities WHERE id = ?1",
                [entity_id],
                |row| row.get(0),
            )
            .optional()?;
        match owner {
            Some(owner) if owner == work_id => {}
            Some(_) => {
                return Err(PerwigaError::Conflict(
                    "entity external identity crosses work ownership boundaries".into(),
                ))
            }
            None => return Err(PerwigaError::NotFound(format!("entity {entity_id}"))),
        }
        let source_provider = required_text(source_provider, "entity identity provider")?;
        let source_identity = required_text(source_identity, "entity identity")?;
        self.connection.execute(
            "INSERT INTO entity_external_identities
             (id, work_id, entity_id, source_provider, source_identity, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (work_id, source_provider, source_identity) DO NOTHING",
            params![
                new_id(),
                work_id,
                entity_id,
                source_provider,
                source_identity,
                now()
            ],
        )?;
        self.connection
            .query_row(
                "SELECT id, work_id, entity_id, source_provider, source_identity
                 FROM entity_external_identities
                 WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
                params![work_id, source_provider, source_identity],
                |row| {
                    Ok(EntityExternalIdentity {
                        id: row.get(0)?,
                        work_id: row.get(1)?,
                        entity_id: row.get(2)?,
                        source_provider: row.get(3)?,
                        source_identity: row.get(4)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn resolve_entity_external_identity(
        &self,
        work_id: &str,
        source_provider: &str,
        source_identity: &str,
    ) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT entity_id FROM entity_external_identities
                 WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
                params![work_id, source_provider, source_identity],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Stores a validated candidate batch without materializing unreviewed
    /// lore into the canonical graph. Sources and exact external identity
    /// references receive an automatic validation status; narrative events,
    /// claims, ordering, and provisional subjects remain reviewable records.
    pub fn import_lore_candidate_batch(
        &mut self,
        work_id: &str,
        batch: &LoreCandidateBatch,
    ) -> Result<LoreImportSummary> {
        batch.validate()?;
        let transaction = self.connection.transaction()?;
        let module_id: String = transaction
            .query_row(
                "SELECT module_id FROM library_works WHERE id = ?1",
                [work_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
        if module_id != batch.module_id {
            return Err(PerwigaError::Conflict(format!(
                "lore candidate module {} does not match work module {}",
                batch.module_id, module_id
            )));
        }
        if let Some(batch_id) = transaction
            .query_row(
                "SELECT id FROM lore_candidate_batches
                 WHERE work_id = ?1 AND corpus_manifest_sha256 = ?2
                   AND generator_name = ?3 AND generator_version = ?4",
                params![
                    work_id,
                    batch.corpus.manifest_sha256,
                    batch.generator.name,
                    batch.generator.version
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            transaction.commit()?;
            return Ok(LoreImportSummary {
                batch_inserted: 0,
                candidates_inserted: 0,
                candidates_unchanged: self.count_lore_candidates(&batch_id)?,
                auto_accepted: 0,
                pending: 0,
            });
        }

        let batch_id = new_id();
        let timestamp = now();
        transaction.execute(
            "INSERT INTO lore_candidate_batches
             (id, work_id, schema, corpus_version, corpus_manifest_sha256,
              generator_name, generator_version, generated_at, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                batch_id,
                work_id,
                batch.schema,
                batch.corpus.version,
                batch.corpus.manifest_sha256,
                batch.generator.name,
                batch.generator.version,
                batch.generator.generated_at,
                timestamp,
            ],
        )?;

        let mut summary = LoreImportSummary {
            batch_inserted: 1,
            ..LoreImportSummary::default()
        };
        for source in &batch.sources {
            let status = "auto_accepted";
            insert_lore_candidate(
                &transaction,
                &batch_id,
                &format!("source:{}", source.key),
                "source",
                serde_json::to_string(source)
                    .map_err(|error| PerwigaError::Validation(error.to_string()))?,
                status,
            )?;
            summary.candidates_inserted += 1;
            summary.auto_accepted += 1;
        }
        for period in &batch.periods {
            insert_lore_candidate(
                &transaction,
                &batch_id,
                &format!("period:{}", period.key),
                "period",
                serde_json::to_string(period)
                    .map_err(|error| PerwigaError::Validation(error.to_string()))?,
                "pending",
            )?;
            summary.candidates_inserted += 1;
            summary.pending += 1;
        }
        for subject in &batch.subjects {
            let status = if subject.entity_ref.is_some() {
                "auto_accepted"
            } else {
                "pending"
            };
            insert_lore_candidate(
                &transaction,
                &batch_id,
                &format!("subject:{}", subject.key),
                "subject",
                serde_json::to_string(subject)
                    .map_err(|error| PerwigaError::Validation(error.to_string()))?,
                status,
            )?;
            summary.candidates_inserted += 1;
            if status == "auto_accepted" {
                summary.auto_accepted += 1;
            } else {
                summary.pending += 1;
            }
        }
        for event in &batch.events {
            insert_lore_candidate(
                &transaction,
                &batch_id,
                &format!("event:{}", event.key),
                "event",
                serde_json::to_string(event)
                    .map_err(|error| PerwigaError::Validation(error.to_string()))?,
                "pending",
            )?;
            summary.candidates_inserted += 1;
            summary.pending += 1;
            for claim in &event.claims {
                insert_lore_candidate(
                    &transaction,
                    &batch_id,
                    &format!("claim:{}:{}", event.key, claim.key),
                    "claim",
                    serde_json::to_string(&(event.key.as_str(), claim))
                        .map_err(|error| PerwigaError::Validation(error.to_string()))?,
                    "pending",
                )?;
                summary.candidates_inserted += 1;
                summary.pending += 1;
            }
            for (index, relation) in event.time.relations.iter().enumerate() {
                insert_lore_candidate(
                    &transaction,
                    &batch_id,
                    &format!("relation:{}:{index}", event.key),
                    "relation",
                    serde_json::to_string(&(event.key.as_str(), relation))
                        .map_err(|error| PerwigaError::Validation(error.to_string()))?,
                    "pending",
                )?;
                summary.candidates_inserted += 1;
                summary.pending += 1;
            }
        }
        transaction.commit()?;
        Ok(summary)
    }

    pub fn list_lore_candidates(
        &self,
        work_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<LoreCandidateRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT c.id, c.batch_id, c.candidate_key, c.candidate_kind,
                    c.payload_json, c.status, c.validation_note
             FROM lore_candidates c
             JOIN lore_candidate_batches b ON b.id = c.batch_id
             WHERE b.work_id = ?1 AND (?2 IS NULL OR c.status = ?2)
             ORDER BY c.created_at, c.id",
        )?;
        let rows = statement.query_map(params![work_id, status], |row| {
            Ok(LoreCandidateRecord {
                id: row.get(0)?,
                batch_id: row.get(1)?,
                candidate_key: row.get(2)?,
                candidate_kind: row.get(3)?,
                payload_json: row.get(4)?,
                status: row.get(5)?,
                validation_note: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn count_lore_candidates(&self, batch_id: &str) -> Result<usize> {
        Ok(self.connection.query_row(
            "SELECT COUNT(*) FROM lore_candidates WHERE batch_id = ?1",
            [batch_id],
            |row| row.get::<_, i64>(0),
        )? as usize)
    }

    pub fn review_lore_candidate(
        &mut self,
        candidate_id: &str,
        decision: &str,
        normalized_payload_json: Option<&str>,
        merge_target_id: Option<&str>,
        notes: Option<&str>,
    ) -> Result<LoreCandidateRecord> {
        let transaction = self.connection.transaction()?;
        let candidate = transaction
            .query_row(
                "SELECT c.id, c.batch_id, c.candidate_key, c.candidate_kind,
                        c.payload_json, c.status, c.validation_note
                 FROM lore_candidates c WHERE c.id = ?1",
                [candidate_id],
                row_to_lore_candidate,
            )
            .optional()?
            .ok_or_else(|| PerwigaError::NotFound(format!("lore candidate {candidate_id}")))?;
        if matches!(
            candidate.status.as_str(),
            "approved" | "rejected" | "merged"
        ) {
            return Err(PerwigaError::Conflict(format!(
                "lore candidate {} already has status {}",
                candidate.id, candidate.status
            )));
        }
        if !matches!(decision, "approve" | "reject" | "merge") {
            return Err(PerwigaError::Validation(format!(
                "unsupported lore review decision {decision}"
            )));
        }
        if decision == "merge" && merge_target_id.is_none() {
            return Err(PerwigaError::Validation(
                "merge decisions require a target ID".into(),
            ));
        }
        let payload = normalized_payload_json.unwrap_or(candidate.payload_json.as_str());
        if normalized_payload_json.is_some() {
            serde_json::from_str::<serde_json::Value>(payload).map_err(|error| {
                PerwigaError::Validation(format!("invalid edited lore payload: {error}"))
            })?;
        }
        let status = match decision {
            "approve" => "approved",
            "reject" => "rejected",
            "merge" => "merged",
            _ => unreachable!(),
        };
        transaction.execute(
            "INSERT INTO lore_review_decisions
             (id, candidate_id, decision, decision_source, normalized_payload_json,
              merge_target_id, notes, created_at)
             VALUES (?1, ?2, ?3, 'human', ?4, ?5, ?6, ?7)",
            params![
                new_id(),
                candidate.id,
                decision,
                normalized_payload_json,
                merge_target_id,
                non_empty_option(notes, "lore review notes")?,
                now()
            ],
        )?;
        transaction.execute(
            "UPDATE lore_candidates SET payload_json = ?1, status = ?2, updated_at = ?3
             WHERE id = ?4",
            params![payload, status, now(), candidate.id],
        )?;
        if decision == "approve" {
            let work_id: String = transaction.query_row(
                "SELECT work_id FROM lore_candidate_batches WHERE id = ?1",
                [&candidate.batch_id],
                |row| row.get(0),
            )?;
            materialize_lore_batch(&transaction, &work_id, &candidate.batch_id)?;
        }
        transaction.commit()?;
        self.get_lore_candidate(candidate_id)?
            .ok_or_else(|| PerwigaError::NotFound(format!("lore candidate {candidate_id}")))
    }

    pub fn get_lore_candidate(&self, candidate_id: &str) -> Result<Option<LoreCandidateRecord>> {
        self.connection
            .query_row(
                "SELECT id, batch_id, candidate_key, candidate_kind, payload_json,
                        status, validation_note
                 FROM lore_candidates WHERE id = ?1",
                [candidate_id],
                row_to_lore_candidate,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_lore_candidate_work_id(&self, candidate_id: &str) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT b.work_id FROM lore_candidates c
                 JOIN lore_candidate_batches b ON b.id = c.batch_id
                 WHERE c.id = ?1",
                [candidate_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_lore_subjects(&self, work_id: &str) -> Result<Vec<LoreSubject>> {
        let mut statement = self.connection.prepare(
            "SELECT id, work_id, attested_name, proposed_type, wiki_entity_id,
                    source_provider, source_identity
             FROM lore_subjects WHERE work_id = ?1
             ORDER BY attested_name COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([work_id], |row| {
            Ok(LoreSubject {
                id: row.get(0)?,
                work_id: row.get(1)?,
                attested_name: row.get(2)?,
                proposed_type: row.get(3)?,
                wiki_entity_id: row.get(4)?,
                source_provider: row.get(5)?,
                source_identity: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn list_lore_graph(
        &self,
        work_id: &str,
        subject_id: Option<&str>,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<crate::lore::LoreGraph> {
        let limit = limit.clamp(1, 200) as i64;
        let mut periods_statement = self.connection.prepare(
            "SELECT id, work_id, source_provider, source_identity, name_en,
                    description_en, display_order, parent_period_id
             FROM lore_periods WHERE work_id = ?1
             ORDER BY display_order, id",
        )?;
        let period_rows = periods_statement.query_map([work_id], |row| {
            Ok(LorePeriod {
                id: row.get(0)?,
                work_id: row.get(1)?,
                source_provider: row.get(2)?,
                source_identity: row.get(3)?,
                name_en: row.get(4)?,
                description_en: row.get(5)?,
                display_order: row.get(6)?,
                parent_period_id: row.get(7)?,
            })
        })?;
        let periods = period_rows.collect::<std::result::Result<Vec<_>, _>>()?;

        let mut events_statement = self.connection.prepare(
            "SELECT e.id, e.work_id, e.source_provider, e.source_identity,
                    r.revision, r.title_en, r.summary_en, r.time_kind,
                    r.time_precision, r.time_label, r.start_period_id, r.end_period_id
             FROM lore_events e
             JOIN lore_event_revisions r
               ON r.event_id = e.id AND r.revision = e.active_revision
             WHERE e.work_id = ?1
               AND (?2 IS NULL OR e.id > ?2)
               AND (?3 IS NULL OR EXISTS (
                 SELECT 1 FROM lore_involvements i
                 LEFT JOIN lore_claims c ON c.id = i.claim_id
                 WHERE i.subject_id = ?3 AND (i.event_id = e.id OR c.event_id = e.id)
               ))
             ORDER BY e.id LIMIT ?4",
        )?;
        let event_rows =
            events_statement.query_map(params![work_id, cursor, subject_id, limit + 1], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            })?;
        let mut events = Vec::new();
        for row in event_rows.take((limit + 1) as usize) {
            let (
                id,
                event_work_id,
                source_provider,
                source_identity,
                revision,
                title_en,
                summary_en,
                time_kind_value,
                time_precision_value,
                time_label,
                start_period_id,
                end_period_id,
            ) = row?;
            events.push(LoreEvent {
                id,
                work_id: event_work_id,
                source_provider,
                source_identity,
                revision,
                title_en,
                summary_en,
                time_kind: parse_time_kind(&time_kind_value)?,
                time_precision: parse_time_precision(&time_precision_value)?,
                time_label,
                start_period_id,
                end_period_id,
            });
        }
        let next_cursor = if events.len() > limit as usize {
            events.pop();
            events.last().map(|event| event.id.clone())
        } else {
            None
        };
        let mut relations_statement = self.connection.prepare(
            "SELECT r.id, r.event_id, r.related_event_id, r.relation_kind
             FROM lore_event_relations r
             JOIN lore_events e ON e.id = r.event_id
             WHERE e.work_id = ?1 ORDER BY r.id",
        )?;
        let relation_rows = relations_statement.query_map([work_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut relations = Vec::new();
        for row in relation_rows {
            let (id, event_id, related_event_id, kind) = row?;
            relations.push(LoreEventRelation {
                id,
                event_id,
                related_event_id,
                relation_kind: parse_relation_kind(&kind)?,
            });
        }
        Ok(crate::lore::LoreGraph {
            periods,
            events,
            relations,
            subjects: self.list_lore_subjects(work_id)?,
            next_cursor,
        })
    }

    pub fn get_lore_event_detail(&self, event_id: &str) -> Result<Option<LoreEventDetail>> {
        let event = self
            .connection
            .query_row(
                "SELECT e.id, e.work_id, e.source_provider, e.source_identity,
                        r.revision, r.title_en, r.summary_en, r.time_kind,
                        r.time_precision, r.time_label, r.start_period_id, r.end_period_id
                 FROM lore_events e
                 JOIN lore_event_revisions r
                   ON r.event_id = e.id AND r.revision = e.active_revision
                 WHERE e.id = ?1",
                [event_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            id,
            work_id,
            source_provider,
            source_identity,
            revision,
            title_en,
            summary_en,
            time_kind_value,
            time_precision_value,
            time_label,
            start_period_id,
            end_period_id,
        )) = event
        else {
            return Ok(None);
        };
        let event = LoreEvent {
            id,
            work_id,
            source_provider,
            source_identity,
            revision,
            title_en,
            summary_en,
            time_kind: parse_time_kind(&time_kind_value)?,
            time_precision: parse_time_precision(&time_precision_value)?,
            time_label,
            start_period_id,
            end_period_id,
        };
        let mut claims_statement = self.connection.prepare(
            "SELECT id, event_id, claim_key, text_en, assertion_kind,
                    certainty, branch_group
             FROM lore_claims WHERE event_id = ?1 ORDER BY claim_key, id",
        )?;
        let claim_rows = claims_statement.query_map([event_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })?;
        let mut claims = Vec::new();
        for row in claim_rows {
            let (id, event_id, claim_key, text_en, assertion, certainty_value, branch_group) = row?;
            claims.push(LoreClaim {
                id,
                event_id,
                claim_key,
                text_en,
                assertion_kind: parse_assertion_kind(&assertion)?,
                certainty: parse_certainty(&certainty_value)?,
                branch_group,
            });
        }
        let mut involvements_statement = self.connection.prepare(
            "SELECT id, event_id, claim_id, subject_id, role
             FROM lore_involvements WHERE event_id = ?1
                OR claim_id IN (SELECT id FROM lore_claims WHERE event_id = ?1)
             ORDER BY id",
        )?;
        let involvement_rows = involvements_statement.query_map([event_id], |row| {
            Ok(LoreInvolvement {
                id: row.get(0)?,
                event_id: row.get(1)?,
                claim_id: row.get(2)?,
                subject_id: row.get(3)?,
                role: row.get(4)?,
            })
        })?;
        let involvements = involvement_rows.collect::<std::result::Result<Vec<_>, _>>()?;
        let mut subjects = Vec::new();
        let mut seen_subjects = HashSet::new();
        for involvement in &involvements {
            if !seen_subjects.insert(involvement.subject_id.clone()) {
                continue;
            }
            if let Some(subject) = self
                .connection
                .query_row(
                    "SELECT id, work_id, attested_name, proposed_type, wiki_entity_id,
                            source_provider, source_identity
                     FROM lore_subjects WHERE id = ?1",
                    [&involvement.subject_id],
                    |row| {
                        Ok(LoreSubject {
                            id: row.get(0)?,
                            work_id: row.get(1)?,
                            attested_name: row.get(2)?,
                            proposed_type: row.get(3)?,
                            wiki_entity_id: row.get(4)?,
                            source_provider: row.get(5)?,
                            source_identity: row.get(6)?,
                        })
                    },
                )
                .optional()?
            {
                subjects.push(subject);
            }
        }
        let mut evidence_statement = self.connection.prepare(
            "SELECT ce.claim_id, e.id, e.source_id, e.locator, e.excerpt,
                    e.excerpt_sha256, ce.stance
             FROM lore_claim_evidence ce
             JOIN lore_evidence e ON e.id = ce.evidence_id
             WHERE ce.claim_id IN (SELECT id FROM lore_claims WHERE event_id = ?1)
             ORDER BY e.id",
        )?;
        let evidence_rows = evidence_statement.query_map([event_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                LoreEvidence {
                    id: row.get(1)?,
                    source_id: row.get(2)?,
                    locator: row.get(3)?,
                    excerpt: row.get(4)?,
                    excerpt_fingerprint: row.get(5)?,
                },
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut evidence = Vec::new();
        for row in evidence_rows {
            let (claim_id, item, stance) = row?;
            evidence.push(LoreClaimEvidence {
                claim_id,
                evidence: item,
                stance: parse_evidence_stance(&stance)?,
            });
        }
        let mut sources_statement = self.connection.prepare(
            "SELECT DISTINCT s.id, s.work_id, s.source_provider, s.source_identity,
                    s.source_kind, s.title, s.language, s.source_url,
                    s.content_sha256, s.checked_at
             FROM lore_sources s JOIN lore_evidence e ON e.source_id = s.id
             WHERE e.id IN (SELECT evidence_id FROM lore_claim_evidence
                            WHERE claim_id IN (SELECT id FROM lore_claims WHERE event_id = ?1))
             ORDER BY s.id",
        )?;
        let source_rows = sources_statement.query_map([event_id], |row| {
            Ok(LoreSource {
                id: row.get(0)?,
                work_id: row.get(1)?,
                source_provider: row.get(2)?,
                source_identity: row.get(3)?,
                source_kind: row.get(4)?,
                title: row.get(5)?,
                language: row.get(6)?,
                source_url: row.get(7)?,
                content_sha256: row.get(8)?,
                checked_at: row.get(9)?,
            })
        })?;
        let sources = source_rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Some(LoreEventDetail {
            event,
            claims,
            involvements,
            subjects,
            evidence,
            sources,
        }))
    }

    pub fn get_entity_detail(&self, id: &str) -> Result<Option<WikiEntityDetail>> {
        let Some(entity) = self.get_entity(id)? else {
            return Ok(None);
        };
        Ok(Some(WikiEntityDetail {
            aliases: self.list_aliases(id)?,
            appearances: self.list_entity_appearances(id)?,
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

    /// Imports module-owned relationship rows without replacing existing data.
    /// The generic table is namespaced by the source provider/identity, while
    /// the owning module controls the relation kind and meaning.
    pub fn import_entity_appearances(&mut self, inputs: &[EntityAppearanceInput]) -> Result<usize> {
        let transaction = self.connection.transaction()?;
        let mut inserted = 0;
        for input in inputs {
            let entity_id = required_text(&input.entity_id, "appearance entity ID")?;
            let relation_kind = required_text(&input.relation_kind, "appearance relation kind")?;
            let related_title = required_text(&input.related_title, "appearance title")?;
            let source_provider =
                required_text(&input.source_provider, "appearance source provider")?;
            let source_identity =
                required_text(&input.source_identity, "appearance source identity")?;
            let entity_exists: Option<String> = transaction
                .query_row(
                    "SELECT work_id FROM wiki_entities WHERE id = ?1",
                    [&entity_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if entity_exists.is_none() {
                return Err(PerwigaError::NotFound(format!("entity {entity_id}")));
            }
            let related_source_url = input
                .related_source_url
                .as_deref()
                .map(validate_remote_url)
                .transpose()?;
            let appearance_id = transaction
                .query_row(
                    "SELECT id FROM entity_appearances
                     WHERE entity_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
                    params![entity_id, source_provider, source_identity],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            let appearance_id = match appearance_id {
                Some(id) => id,
                None => {
                    let id = new_id();
                    transaction.execute(
                        "INSERT INTO entity_appearances
                         (id, entity_id, relation_kind, related_title, related_source_url,
                          source_provider, source_identity, source_notes, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                        params![
                            id,
                            entity_id,
                            relation_kind,
                            related_title,
                            related_source_url,
                            source_provider,
                            source_identity,
                            non_empty_option(input.source_notes.as_deref(), "appearance notes")?,
                            now(),
                        ],
                    )?;
                    inserted += 1;
                    id
                }
            };
            for location in &input.locations {
                let location_name = required_text(&location.location_name, "appearance location")?;
                transaction.execute(
                    "INSERT INTO entity_appearance_locations
                     (id, appearance_id, location_name, region_name)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (appearance_id, location_name, region_name) DO NOTHING",
                    params![
                        new_id(),
                        appearance_id,
                        location_name,
                        non_empty_option(location.region_name.as_deref(), "location region")?,
                    ],
                )?;
            }
        }
        transaction.commit()?;
        Ok(inserted)
    }

    pub fn list_entity_appearances(&self, entity_id: &str) -> Result<Vec<EntityAppearance>> {
        let mut statement = self.connection.prepare(
            "SELECT id, entity_id, relation_kind, related_title, related_source_url,
                    source_provider, source_identity, source_notes
             FROM entity_appearances WHERE entity_id = ?1
             ORDER BY relation_kind, related_title COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([entity_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })?;
        let mut appearances = Vec::new();
        for row in rows {
            let (
                id,
                entity_id,
                relation_kind,
                related_title,
                related_source_url,
                source_provider,
                source_identity,
                source_notes,
            ) = row?;
            let mut locations = self.connection.prepare(
                "SELECT id, appearance_id, location_name, region_name
                 FROM entity_appearance_locations WHERE appearance_id = ?1
                 ORDER BY location_name COLLATE NOCASE, id",
            )?;
            let location_rows = locations.query_map([&id], |row| {
                Ok(crate::model::EntityAppearanceLocation {
                    id: row.get(0)?,
                    appearance_id: row.get(1)?,
                    location_name: row.get(2)?,
                    region_name: row.get(3)?,
                })
            })?;
            let locations = location_rows.collect::<std::result::Result<Vec<_>, _>>()?;
            appearances.push(EntityAppearance {
                id,
                entity_id,
                relation_kind,
                related_title,
                related_source_url,
                source_provider,
                source_identity,
                source_notes,
                locations,
            });
        }
        Ok(appearances)
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

fn insert_lore_candidate(
    transaction: &rusqlite::Transaction<'_>,
    batch_id: &str,
    candidate_key: &str,
    candidate_kind: &str,
    payload_json: String,
    status: &str,
) -> Result<()> {
    let fingerprint = stable_fingerprint(&payload_json);
    transaction.execute(
        "INSERT INTO lore_candidates
         (id, batch_id, candidate_key, candidate_kind, payload_json, fingerprint,
          status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            new_id(),
            batch_id,
            candidate_key,
            candidate_kind,
            payload_json,
            fingerprint,
            status,
            now()
        ],
    )?;
    Ok(())
}

fn row_to_lore_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<LoreCandidateRecord> {
    Ok(LoreCandidateRecord {
        id: row.get(0)?,
        batch_id: row.get(1)?,
        candidate_key: row.get(2)?,
        candidate_kind: row.get(3)?,
        payload_json: row.get(4)?,
        status: row.get(5)?,
        validation_note: row.get(6)?,
    })
}

fn candidate_payloads<T: DeserializeOwned>(
    transaction: &rusqlite::Transaction<'_>,
    batch_id: &str,
    candidate_kind: &str,
) -> Result<Vec<T>> {
    let mut statement = transaction.prepare(
        "SELECT payload_json FROM lore_candidates
         WHERE batch_id = ?1 AND candidate_kind = ?2
           AND status IN ('approved', 'auto_accepted')
         ORDER BY candidate_key",
    )?;
    let rows = statement.query_map(params![batch_id, candidate_kind], |row| {
        row.get::<_, String>(0)
    })?;
    rows.map(|row| {
        let payload = row?;
        serde_json::from_str(&payload).map_err(|error| {
            PerwigaError::Validation(format!("invalid lore candidate payload: {error}"))
        })
    })
    .collect()
}

fn materialize_lore_batch(
    transaction: &rusqlite::Transaction<'_>,
    work_id: &str,
    batch_id: &str,
) -> Result<()> {
    let work_module: String = transaction.query_row(
        "SELECT module_id FROM library_works WHERE id = ?1",
        [work_id],
        |row| row.get(0),
    )?;
    let lore_provider = format!("lore:{work_module}");

    let source_candidates: Vec<LoreSourceCandidate> =
        candidate_payloads(transaction, batch_id, "source")?;
    let mut sources: HashMap<String, String> = HashMap::new();
    for source in source_candidates {
        let source_id = ensure_lore_source(transaction, work_id, &source)?;
        sources.insert(source.key, source_id);
    }

    let period_candidates: Vec<LorePeriodCandidate> =
        candidate_payloads(transaction, batch_id, "period")?;
    let mut periods: HashMap<String, String> = HashMap::new();
    let mut remaining = period_candidates;
    while !remaining.is_empty() {
        let before = remaining.len();
        let mut next = Vec::new();
        for period in remaining {
            let parent_id = match period.parent_key.as_deref() {
                None => None,
                Some(parent_key) => match periods.get(parent_key) {
                    Some(id) => Some(id.clone()),
                    None => {
                        next.push(period);
                        continue;
                    }
                },
            };
            let identity = format!("period:{}", period.key);
            transaction.execute(
                "INSERT INTO lore_periods
                 (id, work_id, source_provider, source_identity, name_en,
                  description_en, display_order, parent_period_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
                 ON CONFLICT (work_id, source_provider, source_identity) DO NOTHING",
                params![
                    new_id(),
                    work_id,
                    lore_provider,
                    identity,
                    period.name_en,
                    period.description_en,
                    period.order,
                    parent_id,
                    now()
                ],
            )?;
            let id: String = transaction.query_row(
                "SELECT id FROM lore_periods WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
                params![work_id, lore_provider, identity],
                |row| row.get(0),
            )?;
            periods.insert(period.key, id);
        }
        if next.len() == before {
            // A reviewed child may arrive before its parent is reviewed. Leave
            // it pending in the materialized graph; the next approval reruns
            // this pass and can attach it once the parent exists.
            break;
        }
        remaining = next;
    }

    let subject_candidates: Vec<LoreSubjectCandidate> =
        candidate_payloads(transaction, batch_id, "subject")?;
    let mut subjects: HashMap<String, String> = HashMap::new();
    for subject in subject_candidates {
        let identity = format!("subject:{}", subject.key);
        let resolved_entity = match subject.entity_ref.as_ref() {
            Some(reference) => transaction
                .query_row(
                    "SELECT entity_id FROM entity_external_identities
                     WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
                    params![work_id, reference.provider, reference.identity],
                    |row| row.get::<_, String>(0),
                )
                .optional()?,
            None => None,
        };
        transaction.execute(
            "INSERT INTO lore_subjects
             (id, work_id, attested_name, proposed_type, wiki_entity_id,
              source_provider, source_identity, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT (work_id, source_provider, source_identity) DO NOTHING",
            params![
                new_id(),
                work_id,
                subject.attested_name,
                subject.proposed_type,
                resolved_entity,
                lore_provider,
                identity,
                now()
            ],
        )?;
        let subject_id: String = transaction.query_row(
            "SELECT id FROM lore_subjects WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
            params![work_id, lore_provider, identity],
            |row| row.get(0),
        )?;
        if let Some(entity_id) = resolved_entity {
            transaction.execute(
                "UPDATE lore_subjects SET wiki_entity_id = COALESCE(wiki_entity_id, ?1), updated_at = ?2 WHERE id = ?3",
                params![entity_id, now(), subject_id],
            )?;
        }
        for evidence in subject.evidence {
            let source_id = sources.get(&evidence.source_key).ok_or_else(|| {
                PerwigaError::Validation(format!(
                    "unknown lore evidence source {}",
                    evidence.source_key
                ))
            })?;
            let evidence_id = ensure_lore_evidence(transaction, source_id, &evidence)?;
            transaction.execute(
                "INSERT INTO lore_subject_evidence (id, subject_id, evidence_id, created_at)
                 VALUES (?1, ?2, ?3, ?4) ON CONFLICT (subject_id, evidence_id) DO NOTHING",
                params![new_id(), subject_id, evidence_id, now()],
            )?;
        }
        subjects.insert(subject.key, subject_id);
    }

    let event_candidates: Vec<LoreEventCandidate> =
        candidate_payloads(transaction, batch_id, "event")?;
    let mut events: HashMap<String, String> = HashMap::new();
    for event in &event_candidates {
        let identity = format!("event:{}", event.key);
        transaction.execute(
            "INSERT INTO lore_events
             (id, work_id, source_provider, source_identity, active_revision, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)
             ON CONFLICT (work_id, source_provider, source_identity) DO NOTHING",
            params![new_id(), work_id, lore_provider, identity, now()],
        )?;
        let event_id: String = transaction.query_row(
            "SELECT id FROM lore_events WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
            params![work_id, lore_provider, identity],
            |row| row.get(0),
        )?;
        let start_period = event
            .time
            .start_period_key
            .as_ref()
            .and_then(|key| periods.get(key))
            .cloned();
        let end_period = event
            .time
            .end_period_key
            .as_ref()
            .and_then(|key| periods.get(key))
            .cloned();
        ensure_event_revision(transaction, &event_id, event, start_period, end_period)?;
        events.insert(event.key.clone(), event_id);
    }

    for event in &event_candidates {
        let event_id = events.get(&event.key).expect("event map populated");
        for involvement in &event.involvements {
            if let Some(subject_id) = subjects.get(&involvement.subject_key) {
                ensure_lore_involvement(
                    transaction,
                    Some(event_id),
                    None,
                    subject_id,
                    &involvement.role,
                )?;
            }
        }
    }

    type LoreClaimEnvelope = (String, LoreClaimCandidate);
    let claim_candidates: Vec<LoreClaimEnvelope> =
        candidate_payloads(transaction, batch_id, "claim")?;
    for (event_key, claim) in claim_candidates {
        let Some(event_id) = events.get(&event_key) else {
            continue;
        };
        let claim_id = ensure_lore_claim(transaction, event_id, &claim)?;
        for involvement in &claim.involvements {
            if let Some(subject_id) = subjects.get(&involvement.subject_key) {
                ensure_lore_involvement(
                    transaction,
                    None,
                    Some(&claim_id),
                    subject_id,
                    &involvement.role,
                )?;
            }
        }
        for evidence in &claim.evidence {
            let Some(source_id) = sources.get(&evidence.source_key) else {
                continue;
            };
            let evidence_id = ensure_lore_evidence(transaction, source_id, evidence)?;
            transaction.execute(
                "INSERT INTO lore_claim_evidence (id, claim_id, evidence_id, stance, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (claim_id, evidence_id, stance) DO NOTHING",
                params![
                    new_id(),
                    claim_id,
                    evidence_id,
                    evidence_stance(evidence.stance.unwrap_or(LoreEvidenceStance::Supports)),
                    now()
                ],
            )?;
        }
    }

    type LoreRelationEnvelope = (String, LoreEventRelationCandidate);
    let relation_candidates: Vec<LoreRelationEnvelope> =
        candidate_payloads(transaction, batch_id, "relation")?;
    for (event_key, relation) in relation_candidates {
        let (Some(event_id), Some(related_id)) =
            (events.get(&event_key), events.get(&relation.event_key))
        else {
            continue;
        };
        transaction.execute(
            "INSERT INTO lore_event_relations
             (id, event_id, related_event_id, relation_kind, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (event_id, related_event_id, relation_kind) DO NOTHING",
            params![
                new_id(),
                event_id,
                related_id,
                relation_kind(relation.kind),
                now()
            ],
        )?;
    }
    Ok(())
}

fn ensure_lore_source(
    transaction: &rusqlite::Transaction<'_>,
    work_id: &str,
    source: &LoreSourceCandidate,
) -> Result<String> {
    if let Some(url) = source.url.as_deref() {
        validate_remote_url(url)?;
    }
    transaction.execute(
        "INSERT INTO lore_sources
         (id, work_id, source_provider, source_identity, source_kind, title,
          language, source_url, content_sha256, checked_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
         ON CONFLICT (work_id, source_provider, source_identity) DO NOTHING",
        params![
            new_id(),
            work_id,
            source.provider,
            source.identity,
            source.kind,
            source.title,
            source.language,
            source.url,
            source.content_sha256,
            source.checked_at,
        ],
    )?;
    transaction
        .query_row(
            "SELECT id FROM lore_sources WHERE work_id = ?1 AND source_provider = ?2 AND source_identity = ?3",
            params![work_id, source.provider, source.identity],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn ensure_lore_evidence(
    transaction: &rusqlite::Transaction<'_>,
    source_id: &str,
    evidence: &LoreEvidenceCandidate,
) -> Result<String> {
    let fingerprint = stable_fingerprint(&evidence.excerpt);
    transaction.execute(
        "INSERT INTO lore_evidence
         (id, source_id, locator, excerpt, excerpt_sha256, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (source_id, locator, excerpt_sha256) DO NOTHING",
        params![
            new_id(),
            source_id,
            evidence.locator,
            evidence.excerpt,
            fingerprint,
            now()
        ],
    )?;
    transaction
        .query_row(
            "SELECT id FROM lore_evidence WHERE source_id = ?1 AND locator = ?2 AND excerpt_sha256 = ?3",
            params![source_id, evidence.locator, fingerprint],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn ensure_event_revision(
    transaction: &rusqlite::Transaction<'_>,
    event_id: &str,
    event: &LoreEventCandidate,
    start_period_id: Option<String>,
    end_period_id: Option<String>,
) -> Result<()> {
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT revision FROM lore_event_revisions
             WHERE event_id = ?1 AND title_en = ?2
               AND COALESCE(summary_en, '') = COALESCE(?3, '')
               AND time_kind = ?4 AND time_precision = ?5 AND time_label = ?6
               AND COALESCE(start_period_id, '') = COALESCE(?7, '')
               AND COALESCE(end_period_id, '') = COALESCE(?8, '')",
            params![
                event_id,
                event.title_en,
                event.summary_en,
                time_kind(event.time.kind),
                time_precision(event.time.precision),
                event.time.label,
                start_period_id,
                end_period_id
            ],
            |row| row.get(0),
        )
        .optional()?;
    if existing.is_some() {
        return Ok(());
    }
    let next_revision: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(revision), 0) + 1 FROM lore_event_revisions WHERE event_id = ?1",
        [event_id],
        |row| row.get(0),
    )?;
    transaction.execute(
        "INSERT INTO lore_event_revisions
         (id, event_id, revision, title_en, summary_en, time_kind,
          time_precision, time_label, start_period_id, end_period_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            new_id(),
            event_id,
            next_revision,
            event.title_en,
            event.summary_en,
            time_kind(event.time.kind),
            time_precision(event.time.precision),
            event.time.label,
            start_period_id,
            end_period_id,
            now()
        ],
    )?;
    transaction.execute(
        "UPDATE lore_events SET active_revision = ?1, updated_at = ?2 WHERE id = ?3",
        params![next_revision, now(), event_id],
    )?;
    Ok(())
}

fn ensure_lore_claim(
    transaction: &rusqlite::Transaction<'_>,
    event_id: &str,
    claim: &LoreClaimCandidate,
) -> Result<String> {
    transaction.execute(
        "INSERT INTO lore_claims
         (id, event_id, claim_key, text_en, assertion_kind, certainty, branch_group, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (event_id, claim_key) DO NOTHING",
        params![
            new_id(),
            event_id,
            claim.key,
            claim.text_en,
            assertion_kind(claim.assertion_kind),
            certainty(claim.certainty),
            claim.branch_group,
            now()
        ],
    )?;
    transaction
        .query_row(
            "SELECT id FROM lore_claims WHERE event_id = ?1 AND claim_key = ?2",
            params![event_id, claim.key],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn ensure_lore_involvement(
    transaction: &rusqlite::Transaction<'_>,
    event_id: Option<&str>,
    claim_id: Option<&str>,
    subject_id: &str,
    role: &str,
) -> Result<()> {
    let existing: Option<String> = if let Some(event_id) = event_id {
        transaction
            .query_row(
                "SELECT id FROM lore_involvements
                 WHERE event_id = ?1 AND subject_id = ?2 AND role = ?3",
                params![event_id, subject_id, role],
                |row| row.get(0),
            )
            .optional()?
    } else {
        transaction
            .query_row(
                "SELECT id FROM lore_involvements
                 WHERE claim_id = ?1 AND subject_id = ?2 AND role = ?3",
                params![claim_id, subject_id, role],
                |row| row.get(0),
            )
            .optional()?
    };
    if existing.is_none() {
        transaction.execute(
            "INSERT INTO lore_involvements
             (id, event_id, claim_id, subject_id, role, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![new_id(), event_id, claim_id, subject_id, role, now()],
        )?;
    }
    Ok(())
}

fn time_kind(value: LoreTimeKind) -> &'static str {
    match value {
        LoreTimeKind::Moment => "moment",
        LoreTimeKind::Interval => "interval",
    }
}

fn time_precision(value: LoreTimePrecision) -> &'static str {
    match value {
        LoreTimePrecision::Exact => "exact",
        LoreTimePrecision::Bounded => "bounded",
        LoreTimePrecision::Approximate => "approximate",
        LoreTimePrecision::Relative => "relative",
        LoreTimePrecision::Unknown => "unknown",
    }
}

fn assertion_kind(value: LoreAssertionKind) -> &'static str {
    match value {
        LoreAssertionKind::DirectFact => "direct_fact",
        LoreAssertionKind::InWorldReport => "in_world_report",
        LoreAssertionKind::Hearsay => "hearsay",
        LoreAssertionKind::Inference => "inference",
        LoreAssertionKind::Interpretation => "interpretation",
    }
}

fn certainty(value: LoreCertainty) -> &'static str {
    match value {
        LoreCertainty::Confirmed => "confirmed",
        LoreCertainty::Probable => "probable",
        LoreCertainty::Possible => "possible",
        LoreCertainty::Disputed => "disputed",
        LoreCertainty::Unknown => "unknown",
    }
}

fn relation_kind(value: LoreRelationKind) -> &'static str {
    match value {
        LoreRelationKind::Before => "before",
        LoreRelationKind::After => "after",
        LoreRelationKind::Overlaps => "overlaps",
        LoreRelationKind::Contains => "contains",
    }
}

fn evidence_stance(value: LoreEvidenceStance) -> &'static str {
    match value {
        LoreEvidenceStance::Supports => "supports",
        LoreEvidenceStance::Contradicts => "contradicts",
    }
}

fn parse_time_kind(value: &str) -> Result<LoreTimeKind> {
    match value {
        "moment" => Ok(LoreTimeKind::Moment),
        "interval" => Ok(LoreTimeKind::Interval),
        _ => Err(PerwigaError::Validation(format!(
            "unknown lore time kind: {value}"
        ))),
    }
}

fn parse_time_precision(value: &str) -> Result<LoreTimePrecision> {
    match value {
        "exact" => Ok(LoreTimePrecision::Exact),
        "bounded" => Ok(LoreTimePrecision::Bounded),
        "approximate" => Ok(LoreTimePrecision::Approximate),
        "relative" => Ok(LoreTimePrecision::Relative),
        "unknown" => Ok(LoreTimePrecision::Unknown),
        _ => Err(PerwigaError::Validation(format!(
            "unknown lore time precision: {value}"
        ))),
    }
}

fn parse_assertion_kind(value: &str) -> Result<LoreAssertionKind> {
    match value {
        "direct_fact" => Ok(LoreAssertionKind::DirectFact),
        "in_world_report" => Ok(LoreAssertionKind::InWorldReport),
        "hearsay" => Ok(LoreAssertionKind::Hearsay),
        "inference" => Ok(LoreAssertionKind::Inference),
        "interpretation" => Ok(LoreAssertionKind::Interpretation),
        _ => Err(PerwigaError::Validation(format!(
            "unknown lore assertion kind: {value}"
        ))),
    }
}

fn parse_certainty(value: &str) -> Result<LoreCertainty> {
    match value {
        "confirmed" => Ok(LoreCertainty::Confirmed),
        "probable" => Ok(LoreCertainty::Probable),
        "possible" => Ok(LoreCertainty::Possible),
        "disputed" => Ok(LoreCertainty::Disputed),
        "unknown" => Ok(LoreCertainty::Unknown),
        _ => Err(PerwigaError::Validation(format!(
            "unknown lore certainty: {value}"
        ))),
    }
}

fn parse_relation_kind(value: &str) -> Result<LoreRelationKind> {
    match value {
        "before" => Ok(LoreRelationKind::Before),
        "after" => Ok(LoreRelationKind::After),
        "overlaps" => Ok(LoreRelationKind::Overlaps),
        "contains" => Ok(LoreRelationKind::Contains),
        _ => Err(PerwigaError::Validation(format!(
            "unknown lore relation kind: {value}"
        ))),
    }
}

fn parse_evidence_stance(value: &str) -> Result<LoreEvidenceStance> {
    match value {
        "supports" => Ok(LoreEvidenceStance::Supports),
        "contradicts" => Ok(LoreEvidenceStance::Contradicts),
        _ => Err(PerwigaError::Validation(format!(
            "unknown lore evidence stance: {value}"
        ))),
    }
}

fn stable_fingerprint(value: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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

const SCHEMA_V4: &str = r#"
CREATE TABLE entity_appearances (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL REFERENCES wiki_entities (id),
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('quest', 'event', 'action')),
    related_title TEXT NOT NULL CHECK (length(trim(related_title)) > 0),
    related_source_url TEXT,
    source_provider TEXT NOT NULL CHECK (length(trim(source_provider)) > 0),
    source_identity TEXT NOT NULL CHECK (length(trim(source_identity)) > 0),
    source_notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (entity_id, source_provider, source_identity)
);
CREATE INDEX entity_appearances_entity_idx ON entity_appearances (entity_id, relation_kind);

CREATE TABLE entity_appearance_locations (
    id TEXT PRIMARY KEY,
    appearance_id TEXT NOT NULL REFERENCES entity_appearances (id),
    location_name TEXT NOT NULL CHECK (length(trim(location_name)) > 0),
    region_name TEXT,
    UNIQUE (appearance_id, location_name, region_name)
);
CREATE INDEX entity_appearance_locations_appearance_idx
ON entity_appearance_locations (appearance_id, location_name);
"#;

const SCHEMA_V5: &str = r#"
CREATE TABLE entity_external_identities (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    entity_id TEXT NOT NULL REFERENCES wiki_entities (id),
    source_provider TEXT NOT NULL CHECK (length(trim(source_provider)) > 0),
    source_identity TEXT NOT NULL CHECK (length(trim(source_identity)) > 0),
    created_at TEXT NOT NULL,
    UNIQUE (work_id, source_provider, source_identity),
    UNIQUE (entity_id, source_provider, source_identity)
);
CREATE INDEX entity_external_identities_entity_idx
ON entity_external_identities (entity_id, source_provider);

CREATE TABLE lore_periods (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    source_provider TEXT NOT NULL CHECK (length(trim(source_provider)) > 0),
    source_identity TEXT NOT NULL CHECK (length(trim(source_identity)) > 0),
    name_en TEXT NOT NULL CHECK (length(trim(name_en)) > 0),
    description_en TEXT,
    display_order INTEGER NOT NULL,
    parent_period_id TEXT REFERENCES lore_periods (id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (work_id, source_provider, source_identity)
);
CREATE INDEX lore_periods_order_idx ON lore_periods (work_id, display_order, id);

CREATE TABLE lore_sources (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    source_provider TEXT NOT NULL CHECK (length(trim(source_provider)) > 0),
    source_identity TEXT NOT NULL CHECK (length(trim(source_identity)) > 0),
    source_kind TEXT NOT NULL CHECK (length(trim(source_kind)) > 0),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    language TEXT NOT NULL CHECK (length(trim(language)) > 0),
    source_url TEXT,
    content_sha256 TEXT NOT NULL CHECK (length(trim(content_sha256)) > 0),
    checked_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (work_id, source_provider, source_identity)
);
CREATE INDEX lore_sources_work_idx ON lore_sources (work_id, source_provider);

CREATE TABLE lore_evidence (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES lore_sources (id),
    locator TEXT NOT NULL CHECK (length(trim(locator)) > 0),
    excerpt TEXT NOT NULL CHECK (length(trim(excerpt)) > 0),
    excerpt_sha256 TEXT NOT NULL CHECK (length(trim(excerpt_sha256)) > 0),
    created_at TEXT NOT NULL,
    UNIQUE (source_id, locator, excerpt_sha256)
);

CREATE TABLE lore_events (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    source_provider TEXT NOT NULL CHECK (length(trim(source_provider)) > 0),
    source_identity TEXT NOT NULL CHECK (length(trim(source_identity)) > 0),
    active_revision INTEGER NOT NULL CHECK (active_revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (work_id, source_provider, source_identity)
);
CREATE INDEX lore_events_work_idx ON lore_events (work_id, id);

CREATE TABLE lore_event_revisions (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES lore_events (id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    title_en TEXT NOT NULL CHECK (length(trim(title_en)) > 0),
    summary_en TEXT,
    time_kind TEXT NOT NULL CHECK (time_kind IN ('moment', 'interval')),
    time_precision TEXT NOT NULL CHECK (time_precision IN ('exact', 'bounded', 'approximate', 'relative', 'unknown')),
    time_label TEXT NOT NULL CHECK (length(trim(time_label)) > 0),
    start_period_id TEXT REFERENCES lore_periods (id),
    end_period_id TEXT REFERENCES lore_periods (id),
    decision_id TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (event_id, revision)
);

CREATE TABLE lore_event_relations (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES lore_events (id),
    related_event_id TEXT NOT NULL REFERENCES lore_events (id),
    relation_kind TEXT NOT NULL CHECK (relation_kind IN ('before', 'after', 'overlaps', 'contains')),
    created_at TEXT NOT NULL,
    UNIQUE (event_id, related_event_id, relation_kind)
);
CREATE INDEX lore_event_relations_event_idx ON lore_event_relations (event_id, relation_kind);

CREATE TABLE lore_subjects (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    attested_name TEXT NOT NULL CHECK (length(trim(attested_name)) > 0),
    proposed_type TEXT NOT NULL CHECK (length(trim(proposed_type)) > 0),
    wiki_entity_id TEXT REFERENCES wiki_entities (id),
    source_provider TEXT NOT NULL CHECK (length(trim(source_provider)) > 0),
    source_identity TEXT NOT NULL CHECK (length(trim(source_identity)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (work_id, source_provider, source_identity)
);
CREATE INDEX lore_subjects_work_idx ON lore_subjects (work_id, proposed_type, attested_name);

CREATE TABLE lore_claims (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES lore_events (id),
    claim_key TEXT NOT NULL CHECK (length(trim(claim_key)) > 0),
    text_en TEXT NOT NULL CHECK (length(trim(text_en)) > 0),
    assertion_kind TEXT NOT NULL CHECK (assertion_kind IN ('direct_fact', 'in_world_report', 'hearsay', 'inference', 'interpretation')),
    certainty TEXT NOT NULL CHECK (certainty IN ('confirmed', 'probable', 'possible', 'disputed', 'unknown')),
    branch_group TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (event_id, claim_key)
);
CREATE INDEX lore_claims_event_idx ON lore_claims (event_id, claim_key);

CREATE TABLE lore_involvements (
    id TEXT PRIMARY KEY,
    event_id TEXT REFERENCES lore_events (id),
    claim_id TEXT REFERENCES lore_claims (id),
    subject_id TEXT NOT NULL REFERENCES lore_subjects (id),
    role TEXT NOT NULL CHECK (length(trim(role)) > 0),
    created_at TEXT NOT NULL,
    CHECK ((event_id IS NOT NULL AND claim_id IS NULL) OR (event_id IS NULL AND claim_id IS NOT NULL)),
    UNIQUE (event_id, claim_id, subject_id, role)
);
CREATE INDEX lore_involvements_subject_idx ON lore_involvements (subject_id, event_id, claim_id);

CREATE TABLE lore_claim_evidence (
    id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL REFERENCES lore_claims (id),
    evidence_id TEXT NOT NULL REFERENCES lore_evidence (id),
    stance TEXT NOT NULL CHECK (stance IN ('supports', 'contradicts')),
    created_at TEXT NOT NULL,
    UNIQUE (claim_id, evidence_id, stance)
);

CREATE TABLE lore_candidate_batches (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES library_works (id),
    schema TEXT NOT NULL CHECK (length(trim(schema)) > 0),
    corpus_version TEXT NOT NULL CHECK (length(trim(corpus_version)) > 0),
    corpus_manifest_sha256 TEXT NOT NULL CHECK (length(trim(corpus_manifest_sha256)) > 0),
    generator_name TEXT NOT NULL CHECK (length(trim(generator_name)) > 0),
    generator_version TEXT NOT NULL CHECK (length(trim(generator_version)) > 0),
    generated_at TEXT NOT NULL,
    imported_at TEXT NOT NULL,
    UNIQUE (work_id, corpus_manifest_sha256, generator_name, generator_version)
);

CREATE TABLE lore_candidates (
    id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL REFERENCES lore_candidate_batches (id),
    candidate_key TEXT NOT NULL CHECK (length(trim(candidate_key)) > 0),
    candidate_kind TEXT NOT NULL CHECK (candidate_kind IN ('period', 'source', 'subject', 'event', 'claim', 'relation')),
    payload_json TEXT NOT NULL CHECK (length(trim(payload_json)) > 0),
    fingerprint TEXT NOT NULL CHECK (length(trim(fingerprint)) > 0),
    status TEXT NOT NULL CHECK (status IN ('pending', 'auto_accepted', 'approved', 'rejected', 'merged', 'invalid')),
    validation_note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (batch_id, candidate_key),
    UNIQUE (batch_id, fingerprint)
);
CREATE INDEX lore_candidates_status_idx ON lore_candidates (batch_id, status, created_at);

CREATE TABLE lore_review_decisions (
    id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES lore_candidates (id),
    decision TEXT NOT NULL CHECK (decision IN ('approve', 'reject', 'merge')),
    decision_source TEXT NOT NULL CHECK (decision_source IN ('rule', 'human')),
    normalized_payload_json TEXT,
    merge_target_id TEXT,
    notes TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX lore_review_decisions_candidate_idx ON lore_review_decisions (candidate_id, created_at);
"#;

const SCHEMA_V6: &str = r#"
CREATE TABLE lore_subject_evidence (
    id TEXT PRIMARY KEY,
    subject_id TEXT NOT NULL REFERENCES lore_subjects (id),
    evidence_id TEXT NOT NULL REFERENCES lore_evidence (id),
    created_at TEXT NOT NULL,
    UNIQUE (subject_id, evidence_id)
);
CREATE INDEX lore_subject_evidence_subject_idx ON lore_subject_evidence (subject_id, evidence_id);
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
        lore::{
            LoreAssertionKind, LoreCandidateBatch, LoreCertainty, LoreClaimCandidate,
            LoreCorpusMetadata, LoreEventCandidate, LoreEvidenceCandidate, LoreGeneratorMetadata,
            LoreInvolvementCandidate, LorePeriodCandidate, LoreSourceCandidate,
            LoreSubjectCandidate, LoreTimeCandidate, LoreTimeKind, LoreTimePrecision,
            LORE_CANDIDATE_SCHEMA,
        },
        model::{
            AliasInput, CalendarEventInput, EntityAppearanceInput, EntityAppearanceLocationInput,
            EntityInput, EntityPatch, SourcedEntityInput,
        },
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

    fn lore_batch_fixture() -> LoreCandidateBatch {
        let source = LoreEvidenceCandidate {
            source_key: "archive-1".into(),
            locator: "chapter-1#opening".into(),
            excerpt: "The expedition reached the frontier.".into(),
            stance: None,
        };
        LoreCandidateBatch {
            schema: LORE_CANDIDATE_SCHEMA.into(),
            module_id: "arknights-endfield".into(),
            corpus: LoreCorpusMetadata {
                version: "official-fixture-1".into(),
                manifest_sha256: "fixture-manifest".into(),
            },
            generator: LoreGeneratorMetadata {
                name: "fixture-generator".into(),
                version: "1".into(),
                generated_at: "2026-08-29T00:00:00Z".into(),
            },
            sources: vec![LoreSourceCandidate {
                key: "archive-1".into(),
                kind: "official_archive".into(),
                provider: "Arknights: Endfield official archive".into(),
                identity: "archive-1".into(),
                title: "Frontier archive".into(),
                language: "en".into(),
                url: Some("https://endfield.gryphline.com/en-us/archive/1".into()),
                content_sha256: "fixture-content".into(),
                checked_at: "2026-08-29".into(),
            }],
            periods: vec![LorePeriodCandidate {
                key: "frontier-era".into(),
                name_en: "Frontier Era".into(),
                description_en: Some("A synthetic period for persistence tests.".into()),
                order: 10,
                parent_key: None,
            }],
            subjects: vec![LoreSubjectCandidate {
                key: "unknown-witness".into(),
                attested_name: "Unknown Witness".into(),
                proposed_type: "unknown".into(),
                entity_ref: None,
                evidence: vec![source.clone()],
            }],
            events: vec![LoreEventCandidate {
                key: "frontier-arrival".into(),
                title_en: "Arrival at the frontier".into(),
                summary_en: Some("The expedition reached the frontier.".into()),
                time: LoreTimeCandidate {
                    kind: LoreTimeKind::Moment,
                    precision: LoreTimePrecision::Relative,
                    label: "During the Frontier Era".into(),
                    start_period_key: Some("frontier-era".into()),
                    end_period_key: None,
                    relations: Vec::new(),
                },
                involvements: vec![LoreInvolvementCandidate {
                    subject_key: "unknown-witness".into(),
                    role: "witness".into(),
                }],
                claims: vec![LoreClaimCandidate {
                    key: "arrival-confirmed".into(),
                    text_en: "The expedition reached the frontier.".into(),
                    assertion_kind: LoreAssertionKind::DirectFact,
                    certainty: LoreCertainty::Confirmed,
                    branch_group: None,
                    involvements: Vec::new(),
                    evidence: vec![source],
                }],
            }],
        }
    }

    #[test]
    fn lore_candidate_import_is_idempotent_and_review_materializes_graph() {
        let mut store = Store::open_in_memory().expect("in-memory store");
        let work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("work");
        let batch = lore_batch_fixture();

        let imported = store
            .import_lore_candidate_batch(&work.id, &batch)
            .expect("candidate batch import");
        assert_eq!(imported.batch_inserted, 1);
        assert_eq!(imported.candidates_inserted, 5);
        assert_eq!(imported.auto_accepted, 1);
        assert_eq!(imported.pending, 4);

        let duplicate = store
            .import_lore_candidate_batch(&work.id, &batch)
            .expect("duplicate batch import");
        assert_eq!(duplicate.batch_inserted, 0);
        assert_eq!(duplicate.candidates_inserted, 0);
        assert_eq!(duplicate.candidates_unchanged, 5);

        let pending_candidates = store
            .list_lore_candidates(&work.id, Some("pending"))
            .expect("pending candidates")
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(pending_candidates.len(), 4);
        for candidate in pending_candidates {
            let reviewed = store
                .review_lore_candidate(&candidate.id, "approve", None, None, None)
                .expect("approve lore candidate");
            assert_eq!(reviewed.status, "approved");
        }

        let graph = store
            .list_lore_graph(&work.id, None, None, 50)
            .expect("lore graph");
        assert_eq!(graph.periods.len(), 1);
        assert_eq!(graph.events.len(), 1);
        assert_eq!(graph.subjects.len(), 1);
        assert!(graph.subjects[0].wiki_entity_id.is_none());
        assert_eq!(graph.events[0].time_precision, LoreTimePrecision::Relative);

        let detail = store
            .get_lore_event_detail(&graph.events[0].id)
            .expect("event detail")
            .expect("event exists");
        assert_eq!(detail.claims.len(), 1);
        assert_eq!(detail.evidence.len(), 1);
        assert_eq!(detail.sources.len(), 1);
        assert_eq!(detail.involvements.len(), 1);
    }

    #[test]
    fn entity_external_identity_is_work_scoped_and_idempotent() {
        let store = Store::open_in_memory().expect("in-memory store");
        let first_work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("first work");
        let second_work = store
            .insert_work(WorkKind::Game, "generic", "Other game")
            .expect("second work");
        let entity = store
            .insert_entity(
                &first_work.id,
                &EntityInput {
                    entity_type: "npc".into(),
                    official_english_name: "Witness".into(),
                    official_original_name: "Witness".into(),
                    official_vietnamese_name: None,
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: None,
                },
            )
            .expect("entity");

        let identity = store
            .add_entity_external_identity(
                &first_work.id,
                &entity.id,
                "endfield-official",
                "npc:witness",
            )
            .expect("identity");
        let duplicate = store
            .add_entity_external_identity(
                &first_work.id,
                &entity.id,
                "endfield-official",
                "npc:witness",
            )
            .expect("duplicate identity");
        assert_eq!(identity.id, duplicate.id);
        assert_eq!(
            store
                .resolve_entity_external_identity(
                    &first_work.id,
                    "endfield-official",
                    "npc:witness"
                )
                .expect("resolve identity"),
            Some(entity.id.clone())
        );
        assert!(matches!(
            store.add_entity_external_identity(
                &second_work.id,
                &entity.id,
                "endfield-official",
                "npc:witness"
            ),
            Err(PerwigaError::Conflict(_))
        ));
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
        assert_eq!(schema_version, 6);
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
    fn entity_appearances_are_normalized_idempotent_and_keep_many_locations() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "genshin-impact", "Genshin Impact")
            .expect("work");
        let npc = store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "npc".into(),
                    official_english_name: "Liben".into(),
                    official_original_name: "立本".into(),
                    official_vietnamese_name: Some("Liben".into()),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: None,
                },
            )
            .expect("NPC");
        let input = EntityAppearanceInput {
            entity_id: npc.id.clone(),
            relation_kind: "event".into(),
            related_title: "Marvelous Merchandise".into(),
            related_source_url: Some(
                "https://genshin-impact.fandom.com/wiki/Marvelous_Merchandise".into(),
            ),
            source_provider: "Genshin Impact Wiki".into(),
            source_identity: "npc-14969:event:marvelous-merchandise".into(),
            source_notes: Some("Source-backed event relationship".into()),
            locations: vec![
                EntityAppearanceLocationInput {
                    location_name: "Mondstadt".into(),
                    region_name: Some("Mondstadt".into()),
                },
                EntityAppearanceLocationInput {
                    location_name: "Liyue Harbor".into(),
                    region_name: Some("Liyue".into()),
                },
            ],
        };
        assert_eq!(
            store
                .import_entity_appearances(&[input.clone()])
                .expect("first import"),
            1
        );
        assert_eq!(
            store
                .import_entity_appearances(&[input])
                .expect("second import"),
            0
        );
        let appearances = store.list_entity_appearances(&npc.id).expect("appearances");
        assert_eq!(appearances.len(), 1);
        assert_eq!(appearances[0].relation_kind, "event");
        assert_eq!(appearances[0].locations.len(), 2);
        let detail = store
            .get_entity_detail(&npc.id)
            .expect("entity detail")
            .expect("NPC detail");
        assert_eq!(detail.appearances, appearances);
        assert_eq!(store.integrity_check().expect("integrity"), "ok");
        assert_eq!(store.foreign_key_violations().expect("foreign keys"), 0);
    }

    #[test]
    fn entity_appearance_source_urls_are_validated_at_the_storage_boundary() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Novel, "generic", "A novel")
            .expect("work");
        let character = store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "character".into(),
                    official_english_name: "Mara".into(),
                    official_original_name: "Mara".into(),
                    official_vietnamese_name: None,
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: None,
                },
            )
            .expect("character");
        let error = store
            .import_entity_appearances(&[EntityAppearanceInput {
                entity_id: character.id,
                relation_kind: "chapter".into(),
                related_title: "Chapter 1".into(),
                related_source_url: Some("javascript:alert(1)".into()),
                source_provider: "Novel manuscript".into(),
                source_identity: "chapter-1".into(),
                source_notes: None,
                locations: vec![],
            }])
            .expect_err("unsafe URL must be rejected");
        assert!(error.to_string().contains("URL must use http or https"));
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
    fn sourced_entities_may_share_an_official_name_when_identities_differ() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("Endfield work");
        let inputs = [
            sourced_character("essence-1", "Flawless Essence", "无瑕基质"),
            sourced_character("essence-2", "Flawless Essence", "无瑕基质"),
        ];

        let summary = store
            .import_sourced_entities(&work.id, &inputs)
            .expect("stable source identities disambiguate duplicate official names");
        assert_eq!(summary.inserted, 2);
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
