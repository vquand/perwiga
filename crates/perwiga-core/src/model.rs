use serde::{Deserialize, Serialize};

use crate::module::WorkKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryWork {
    pub id: String,
    pub kind: WorkKind,
    pub module_id: String,
    pub display_name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiEntity {
    pub id: String,
    pub work_id: String,
    pub entity_type: String,
    pub official_english_name: String,
    pub official_original_name: String,
    pub official_vietnamese_name: Option<String>,
    pub automatic_vietnamese_translation: Option<String>,
    pub english_description: Option<String>,
    pub other_information: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityAlias {
    pub id: String,
    pub entity_id: String,
    pub value: String,
    pub language: Option<String>,
    pub kind: String,
    pub label: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiEntityDetail {
    pub entity: WikiEntity,
    pub aliases: Vec<EntityAlias>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub work_id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checklist {
    pub id: String,
    pub work_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: String,
    pub checklist_id: String,
    pub label: String,
    pub is_complete: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedFolder {
    pub id: String,
    pub work_id: String,
    pub path: String,
    pub label: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteAttachment {
    pub id: String,
    pub note_id: String,
    pub source_type: String,
    pub source_reference: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedSource {
    pub id: String,
    pub work_id: String,
    pub url: String,
    pub provenance: String,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedItem {
    pub id: String,
    pub source_id: String,
    pub external_identity: String,
    pub title: String,
    pub url: Option<String>,
    pub published_at: Option<String>,
    pub discovered_at: String,
    pub is_read: bool,
    pub item_kind: Option<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub work_id: String,
    pub title: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub is_all_day: bool,
    pub source_url: Option<String>,
    pub source_provider: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityInput {
    pub entity_type: String,
    pub official_english_name: String,
    pub official_original_name: String,
    pub official_vietnamese_name: Option<String>,
    pub automatic_vietnamese_translation: Option<String>,
    pub english_description: Option<String>,
    pub other_information: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntityPatch {
    pub official_english_name: Option<String>,
    pub official_original_name: Option<String>,
    pub official_vietnamese_name: Option<String>,
    pub automatic_vietnamese_translation: Option<String>,
    pub english_description: Option<String>,
    pub other_information: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasInput {
    pub value: String,
    pub language: Option<String>,
    pub kind: String,
    pub label: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityAliasBatchInput {
    pub entity_id: String,
    pub alias: AliasInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasBatchSummary {
    pub inserted: usize,
    pub unchanged: usize,
}
