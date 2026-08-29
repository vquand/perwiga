use serde::{Deserialize, Serialize};

use crate::{error::Result, PerwigaError};

pub const LORE_CANDIDATE_SCHEMA: &str = "perwiga-lore-candidates/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityExternalIdentity {
    pub id: String,
    pub work_id: String,
    pub entity_id: String,
    pub source_provider: String,
    pub source_identity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreTimeKind {
    Moment,
    Interval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreTimePrecision {
    Exact,
    Bounded,
    Approximate,
    Relative,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreAssertionKind {
    DirectFact,
    InWorldReport,
    Hearsay,
    Inference,
    Interpretation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreCertainty {
    Confirmed,
    Probable,
    Possible,
    Disputed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreRelationKind {
    Before,
    After,
    Overlaps,
    Contains,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreEvidenceStance {
    Supports,
    Contradicts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreSourceCandidate {
    pub key: String,
    pub kind: String,
    pub provider: String,
    pub identity: String,
    pub title: String,
    pub language: String,
    pub url: Option<String>,
    pub content_sha256: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreEvidenceCandidate {
    pub source_key: String,
    pub locator: String,
    pub excerpt: String,
    #[serde(default)]
    pub stance: Option<LoreEvidenceStance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LorePeriodCandidate {
    pub key: String,
    pub name_en: String,
    #[serde(default)]
    pub description_en: Option<String>,
    pub order: i64,
    #[serde(default)]
    pub parent_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreSubjectReference {
    pub provider: String,
    pub identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreSubjectCandidate {
    pub key: String,
    pub attested_name: String,
    pub proposed_type: String,
    #[serde(default)]
    pub entity_ref: Option<LoreSubjectReference>,
    pub evidence: Vec<LoreEvidenceCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreTimeCandidate {
    pub kind: LoreTimeKind,
    pub precision: LoreTimePrecision,
    pub label: String,
    #[serde(default)]
    pub start_period_key: Option<String>,
    #[serde(default)]
    pub end_period_key: Option<String>,
    #[serde(default)]
    pub relations: Vec<LoreEventRelationCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreEventRelationCandidate {
    pub kind: LoreRelationKind,
    pub event_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreInvolvementCandidate {
    pub subject_key: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreClaimCandidate {
    pub key: String,
    pub text_en: String,
    pub assertion_kind: LoreAssertionKind,
    pub certainty: LoreCertainty,
    #[serde(default)]
    pub branch_group: Option<String>,
    #[serde(default)]
    pub involvements: Vec<LoreInvolvementCandidate>,
    pub evidence: Vec<LoreEvidenceCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreEventCandidate {
    pub key: String,
    pub title_en: String,
    #[serde(default)]
    pub summary_en: Option<String>,
    pub time: LoreTimeCandidate,
    #[serde(default)]
    pub involvements: Vec<LoreInvolvementCandidate>,
    #[serde(default)]
    pub claims: Vec<LoreClaimCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreCorpusMetadata {
    pub version: String,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreGeneratorMetadata {
    pub name: String,
    pub version: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreCandidateBatch {
    pub schema: String,
    pub module_id: String,
    pub corpus: LoreCorpusMetadata,
    pub generator: LoreGeneratorMetadata,
    pub sources: Vec<LoreSourceCandidate>,
    pub periods: Vec<LorePeriodCandidate>,
    pub subjects: Vec<LoreSubjectCandidate>,
    pub events: Vec<LoreEventCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LorePeriod {
    pub id: String,
    pub work_id: String,
    pub source_provider: String,
    pub source_identity: String,
    pub name_en: String,
    pub description_en: Option<String>,
    pub display_order: i64,
    pub parent_period_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreSource {
    pub id: String,
    pub work_id: String,
    pub source_provider: String,
    pub source_identity: String,
    pub source_kind: String,
    pub title: String,
    pub language: String,
    pub source_url: Option<String>,
    pub content_sha256: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreEvidence {
    pub id: String,
    pub source_id: String,
    pub locator: String,
    pub excerpt: String,
    pub excerpt_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreEvent {
    pub id: String,
    pub work_id: String,
    pub source_provider: String,
    pub source_identity: String,
    pub revision: i64,
    pub title_en: String,
    pub summary_en: Option<String>,
    pub time_kind: LoreTimeKind,
    pub time_precision: LoreTimePrecision,
    pub time_label: String,
    pub start_period_id: Option<String>,
    pub end_period_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreEventRelation {
    pub id: String,
    pub event_id: String,
    pub related_event_id: String,
    pub relation_kind: LoreRelationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreSubject {
    pub id: String,
    pub work_id: String,
    pub attested_name: String,
    pub proposed_type: String,
    pub wiki_entity_id: Option<String>,
    pub source_provider: String,
    pub source_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreClaim {
    pub id: String,
    pub event_id: String,
    pub claim_key: String,
    pub text_en: String,
    pub assertion_kind: LoreAssertionKind,
    pub certainty: LoreCertainty,
    pub branch_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreInvolvement {
    pub id: String,
    pub event_id: Option<String>,
    pub claim_id: Option<String>,
    pub subject_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreClaimEvidence {
    pub claim_id: String,
    pub evidence: LoreEvidence,
    pub stance: LoreEvidenceStance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreEventDetail {
    pub event: LoreEvent,
    pub claims: Vec<LoreClaim>,
    pub involvements: Vec<LoreInvolvement>,
    pub subjects: Vec<LoreSubject>,
    pub evidence: Vec<LoreClaimEvidence>,
    pub sources: Vec<LoreSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreGraph {
    pub periods: Vec<LorePeriod>,
    pub events: Vec<LoreEvent>,
    pub relations: Vec<LoreEventRelation>,
    pub subjects: Vec<LoreSubject>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoreCandidateRecord {
    pub id: String,
    pub batch_id: String,
    pub candidate_key: String,
    pub candidate_kind: String,
    pub payload_json: String,
    pub status: String,
    pub validation_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LoreImportSummary {
    pub batch_inserted: usize,
    pub candidates_inserted: usize,
    pub candidates_unchanged: usize,
    pub auto_accepted: usize,
    pub pending: usize,
}

impl LoreCandidateBatch {
    pub fn validate(&self) -> Result<()> {
        if self.schema != LORE_CANDIDATE_SCHEMA {
            return Err(PerwigaError::Validation(format!(
                "unsupported lore candidate schema {}",
                self.schema
            )));
        }
        required(&self.module_id, "lore module ID")?;
        required(&self.corpus.version, "lore corpus version")?;
        required(&self.corpus.manifest_sha256, "lore corpus manifest hash")?;
        required(&self.generator.name, "lore generator name")?;
        required(&self.generator.version, "lore generator version")?;
        required(&self.generator.generated_at, "lore generator timestamp")?;

        let source_keys = unique_keys(
            self.sources.iter().map(|source| source.key.as_str()),
            "lore source",
        )?;
        for source in &self.sources {
            required(&source.key, "lore source key")?;
            required(&source.kind, "lore source kind")?;
            required(&source.provider, "lore source provider")?;
            required(&source.identity, "lore source identity")?;
            required(&source.title, "lore source title")?;
            required(&source.language, "lore source language")?;
            required(&source.content_sha256, "lore source content hash")?;
            required(&source.checked_at, "lore source checked date")?;
            if let Some(url) = &source.url {
                if !url.starts_with("https://") {
                    return Err(PerwigaError::Validation(
                        "lore source URL must use HTTPS".into(),
                    ));
                }
            }
        }

        let period_keys = unique_keys(
            self.periods.iter().map(|period| period.key.as_str()),
            "lore period",
        )?;
        for period in &self.periods {
            required(&period.key, "lore period key")?;
            required(&period.name_en, "lore period name")?;
            if let Some(parent) = &period.parent_key {
                if !period_keys.contains(parent) {
                    return Err(PerwigaError::Validation(format!(
                        "lore period {} references unknown parent {}",
                        period.key, parent
                    )));
                }
            }
        }

        let subject_keys = unique_keys(
            self.subjects.iter().map(|subject| subject.key.as_str()),
            "lore subject",
        )?;
        for subject in &self.subjects {
            required(&subject.key, "lore subject key")?;
            required(&subject.attested_name, "lore subject name")?;
            required(&subject.proposed_type, "lore subject type")?;
            if let Some(reference) = &subject.entity_ref {
                required(&reference.provider, "lore subject entity provider")?;
                required(&reference.identity, "lore subject entity identity")?;
            }
            validate_evidence(&subject.evidence, &source_keys, "lore subject")?;
        }

        let event_keys = unique_keys(
            self.events.iter().map(|event| event.key.as_str()),
            "lore event",
        )?;
        for event in &self.events {
            required(&event.key, "lore event key")?;
            required(&event.title_en, "lore event title")?;
            required(&event.time.label, "lore event time label")?;
            if let Some(period) = &event.time.start_period_key {
                if !period_keys.contains(period) {
                    return Err(PerwigaError::Validation(format!(
                        "lore event {} references unknown start period {}",
                        event.key, period
                    )));
                }
            }
            if let Some(period) = &event.time.end_period_key {
                if !period_keys.contains(period) {
                    return Err(PerwigaError::Validation(format!(
                        "lore event {} references unknown end period {}",
                        event.key, period
                    )));
                }
            }
            for relation in &event.time.relations {
                if !event_keys.contains(&relation.event_key) {
                    return Err(PerwigaError::Validation(format!(
                        "lore event {} references unknown related event {}",
                        event.key, relation.event_key
                    )));
                }
                if relation.event_key == event.key
                    && matches!(
                        relation.kind,
                        LoreRelationKind::Before | LoreRelationKind::After
                    )
                {
                    return Err(PerwigaError::Validation(format!(
                        "lore event {} cannot precede itself",
                        event.key
                    )));
                }
            }
            validate_involvements(&event.involvements, &subject_keys, "lore event")?;
            for claim in &event.claims {
                required(&claim.key, "lore claim key")?;
                required(&claim.text_en, "lore claim text")?;
                validate_involvements(&claim.involvements, &subject_keys, "lore claim")?;
                validate_evidence(&claim.evidence, &source_keys, "lore claim")?;
            }
        }
        validate_temporal_cycles(&self.events)?;
        Ok(())
    }
}

fn validate_temporal_cycles(events: &[LoreEventCandidate]) -> Result<()> {
    let mut edges: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for event in events {
        for relation in &event.time.relations {
            match relation.kind {
                LoreRelationKind::Before => edges
                    .entry(event.key.as_str())
                    .or_default()
                    .push(relation.event_key.as_str()),
                LoreRelationKind::After => edges
                    .entry(relation.event_key.as_str())
                    .or_default()
                    .push(event.key.as_str()),
                LoreRelationKind::Overlaps | LoreRelationKind::Contains => {}
            }
        }
    }
    let mut visiting = std::collections::HashSet::new();
    let mut visited = std::collections::HashSet::new();
    for event in events {
        if has_cycle(event.key.as_str(), &edges, &mut visiting, &mut visited) {
            return Err(PerwigaError::Validation(
                "lore temporal precedence contains a cycle".into(),
            ));
        }
    }
    Ok(())
}

fn has_cycle<'a>(
    key: &'a str,
    edges: &std::collections::HashMap<&'a str, Vec<&'a str>>,
    visiting: &mut std::collections::HashSet<&'a str>,
    visited: &mut std::collections::HashSet<&'a str>,
) -> bool {
    if visiting.contains(key) {
        return true;
    }
    if visited.contains(key) {
        return false;
    }
    visiting.insert(key);
    if edges.get(key).is_some_and(|next| {
        next.iter()
            .any(|target| has_cycle(target, edges, visiting, visited))
    }) {
        return true;
    }
    visiting.remove(key);
    visited.insert(key);
    false
}

fn required(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(PerwigaError::Validation(format!("{field} cannot be empty")));
    }
    Ok(())
}

fn unique_keys<'a, I>(values: I, label: &str) -> Result<std::collections::HashSet<String>>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut keys = std::collections::HashSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(PerwigaError::Validation(format!(
                "{label} key cannot be empty"
            )));
        }
        if !keys.insert(value.to_string()) {
            return Err(PerwigaError::Validation(format!(
                "duplicate {label} key {value}"
            )));
        }
    }
    Ok(keys)
}

fn validate_evidence(
    evidence: &[LoreEvidenceCandidate],
    source_keys: &std::collections::HashSet<String>,
    label: &str,
) -> Result<()> {
    if evidence.is_empty() {
        return Err(PerwigaError::Validation(format!(
            "{label} must include at least one evidence excerpt"
        )));
    }
    for item in evidence {
        if !source_keys.contains(&item.source_key) {
            return Err(PerwigaError::Validation(format!(
                "{label} references unknown source {}",
                item.source_key
            )));
        }
        required(&item.locator, "lore evidence locator")?;
        required(&item.excerpt, "lore evidence excerpt")?;
    }
    Ok(())
}

fn validate_involvements(
    involvements: &[LoreInvolvementCandidate],
    subject_keys: &std::collections::HashSet<String>,
    label: &str,
) -> Result<()> {
    for involvement in involvements {
        if !subject_keys.contains(&involvement.subject_key) {
            return Err(PerwigaError::Validation(format!(
                "{label} references unknown subject {}",
                involvement.subject_key
            )));
        }
        required(&involvement.role, "lore involvement role")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch() -> LoreCandidateBatch {
        LoreCandidateBatch {
            schema: LORE_CANDIDATE_SCHEMA.into(),
            module_id: "arknights-endfield".into(),
            corpus: LoreCorpusMetadata {
                version: "fixture".into(),
                manifest_sha256: "abc".into(),
            },
            generator: LoreGeneratorMetadata {
                name: "fixture".into(),
                version: "1".into(),
                generated_at: "2026-08-29T00:00:00Z".into(),
            },
            sources: vec![LoreSourceCandidate {
                key: "source-1".into(),
                kind: "official-client".into(),
                provider: "fixture".into(),
                identity: "text:1".into(),
                title: "Fixture source".into(),
                language: "en".into(),
                url: None,
                content_sha256: "hash".into(),
                checked_at: "2026-08-29".into(),
            }],
            periods: vec![LorePeriodCandidate {
                key: "before".into(),
                name_en: "Before the story".into(),
                description_en: None,
                order: 0,
                parent_key: None,
            }],
            subjects: vec![LoreSubjectCandidate {
                key: "unknown-person".into(),
                attested_name: "Unknown Person".into(),
                proposed_type: "npc".into(),
                entity_ref: None,
                evidence: vec![LoreEvidenceCandidate {
                    source_key: "source-1".into(),
                    locator: "line:1".into(),
                    excerpt: "Unknown Person".into(),
                    stance: None,
                }],
            }],
            events: vec![LoreEventCandidate {
                key: "first-event".into(),
                title_en: "First event".into(),
                summary_en: None,
                time: LoreTimeCandidate {
                    kind: LoreTimeKind::Moment,
                    precision: LoreTimePrecision::Relative,
                    label: "Before the story".into(),
                    start_period_key: Some("before".into()),
                    end_period_key: None,
                    relations: vec![],
                },
                involvements: vec![LoreInvolvementCandidate {
                    subject_key: "unknown-person".into(),
                    role: "mentioned".into(),
                }],
                claims: vec![LoreClaimCandidate {
                    key: "first-claim".into(),
                    text_en: "The person is mentioned.".into(),
                    assertion_kind: LoreAssertionKind::DirectFact,
                    certainty: LoreCertainty::Confirmed,
                    branch_group: None,
                    involvements: vec![],
                    evidence: vec![LoreEvidenceCandidate {
                        source_key: "source-1".into(),
                        locator: "line:1".into(),
                        excerpt: "The person is mentioned.".into(),
                        stance: Some(LoreEvidenceStance::Supports),
                    }],
                }],
            }],
        }
    }

    #[test]
    fn validates_a_complete_candidate_batch() {
        assert!(batch().validate().is_ok());
    }

    #[test]
    fn rejects_a_claim_without_evidence() {
        let mut value = batch();
        value.events[0].claims[0].evidence.clear();
        let error = value.validate().expect_err("evidence is required");
        assert!(error.to_string().contains("evidence"));
    }

    #[test]
    fn rejects_unknown_subject_and_temporal_self_reference() {
        let mut value = batch();
        value.events[0].involvements[0].subject_key = "missing".into();
        let error = value.validate().expect_err("unknown subject");
        assert!(error.to_string().contains("unknown subject"));

        let mut value = batch();
        value.events[0]
            .time
            .relations
            .push(LoreEventRelationCandidate {
                kind: LoreRelationKind::Before,
                event_key: "first-event".into(),
            });
        let error = value.validate().expect_err("self-reference");
        assert!(error.to_string().contains("cannot precede itself"));
    }

    #[test]
    fn rejects_a_temporal_precedence_cycle() {
        let mut value = batch();
        value.events.push(LoreEventCandidate {
            key: "second-event".into(),
            title_en: "Second event".into(),
            summary_en: None,
            time: LoreTimeCandidate {
                kind: LoreTimeKind::Moment,
                precision: LoreTimePrecision::Relative,
                label: "After the first event".into(),
                start_period_key: Some("before".into()),
                end_period_key: None,
                relations: vec![LoreEventRelationCandidate {
                    kind: LoreRelationKind::Before,
                    event_key: "first-event".into(),
                }],
            },
            involvements: vec![],
            claims: vec![],
        });
        value.events[0]
            .time
            .relations
            .push(LoreEventRelationCandidate {
                kind: LoreRelationKind::Before,
                event_key: "second-event".into(),
            });
        let error = value.validate().expect_err("cycle");
        assert!(error.to_string().contains("cycle"));
    }
}
