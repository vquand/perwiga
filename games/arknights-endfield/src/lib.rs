use std::{
    collections::{BTreeMap, HashSet},
    sync::OnceLock,
};

use perwiga_core::{
    model::{AliasInput, EntityAliasBatchInput, WikiEntity},
    Capability, EntityPresentation, EntityTypeDefinition, LibraryModule, ModuleRegistry,
    PerwigaError, Store, ThemeDefinition, WorkKind,
};
use serde::{Deserialize, Serialize};

pub struct ArknightsEndfieldModule;

static CAPABILITIES: &[Capability] = &[Capability::ManualContent, Capability::EntitySchema];
static THEME: ThemeDefinition = ThemeDefinition {
    background: "#080d11",
    surface: "#0e151b",
    surface_raised: "#131c23",
    surface_active: "#19252d",
    border: "#2b3740",
    border_strong: "#43515b",
    text: "#f2f5f2",
    text_muted: "#a7b0b4",
    text_subtle: "#77838a",
    accent: "#d7f75b",
    accent_ink: "#111807",
};
static ENTITY_TYPES: &[EntityTypeDefinition] = &[
    EntityTypeDefinition {
        key: "operator",
        display_name: "Operator",
        description: "A named operator listed by the official Endfield operator directory.",
    },
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
    EntityTypeDefinition {
        key: "weapon",
        display_name: "Weapon",
        description: "A named weapon or weapon type recorded in official update notes.",
    },
    EntityTypeDefinition {
        key: "enemy",
        display_name: "Enemy",
        description: "A named enemy recorded in official update notes.",
    },
    EntityTypeDefinition {
        key: "mission",
        display_name: "Mission",
        description: "A named main, exploration, side, or other mission.",
    },
    EntityTypeDefinition {
        key: "event",
        display_name: "Event",
        description: "A named in-game event or activity.",
    },
    EntityTypeDefinition {
        key: "item",
        display_name: "Item",
        description: "A named item, material, reward, or other database record.",
    },
];

#[derive(Debug, Deserialize)]
struct OperatorSnapshot {
    checked_at: String,
    operators: Vec<CuratedOperator>,
    #[serde(default)]
    official_localizations: Vec<CuratedLocaleDirectory>,
    #[serde(default)]
    additional_aliases: Vec<CuratedAdditionalAlias>,
}

#[derive(Debug, Deserialize)]
struct CuratedOperator {
    source_key: String,
    rarity: u8,
    han_viet_name: String,
}

#[derive(Debug, Deserialize)]
struct CuratedLocaleDirectory {
    language: String,
    label: String,
    source_url: String,
    names: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct CuratedAdditionalAlias {
    source_key: String,
    value: String,
    language: Option<String>,
    kind: String,
    label: String,
    notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CuratedAliasImportSummary {
    pub inserted: usize,
    pub unchanged: usize,
    pub unresolved: Vec<String>,
}

fn curated_snapshot() -> &'static OperatorSnapshot {
    static SNAPSHOT: OnceLock<OperatorSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        serde_json::from_str::<OperatorSnapshot>(include_str!("../data/operators.json"))
            .expect("the bundled Endfield operator snapshot must be valid JSON")
    })
}

fn curated_operators() -> &'static [CuratedOperator] {
    curated_snapshot().operators.as_slice()
}

fn curated_operator(source_key: &str) -> Option<&'static CuratedOperator> {
    curated_operators()
        .iter()
        .find(|operator| operator.source_key == source_key)
}

fn operator_source_key(entity: &WikiEntity) -> Option<&str> {
    if entity.entity_type != "operator" {
        return None;
    }
    let information = entity.other_information.as_deref()?;
    let (_, suffix) = information.split_once("Official source key: ")?;
    let source_key = suffix.split('.').next()?.trim();
    curated_operator(source_key).map(|operator| operator.source_key.as_str())
}

fn rarity_color(rarity: u8) -> Option<&'static str> {
    match rarity {
        4 => Some("#55a8ff"),
        5 => Some("#d9a441"),
        6 => Some("#b51e3f"),
        _ => None,
    }
}

impl LibraryModule for ArknightsEndfieldModule {
    fn id(&self) -> &'static str {
        "arknights-endfield"
    }
    fn kind(&self) -> WorkKind {
        WorkKind::Game
    }
    fn display_name(&self) -> &'static str {
        "Arknights: Endfield"
    }
    fn capabilities(&self) -> &'static [Capability] {
        CAPABILITIES
    }
    fn entity_types(&self) -> &'static [EntityTypeDefinition] {
        ENTITY_TYPES
    }
    fn theme(&self) -> ThemeDefinition {
        THEME
    }
    fn entity_presentation(&self, entity: &WikiEntity) -> Option<EntityPresentation> {
        let source_key = operator_source_key(entity)?;
        let operator = curated_operator(source_key)?;
        Some(EntityPresentation {
            thumbnail_url: format!(
                "/assets/modules/arknights-endfield/operators/{source_key}.webp"
            ),
            accent_color: rarity_color(operator.rarity)?.to_string(),
            label: format!("{}★", operator.rarity),
        })
    }
}

static MODULE: ArknightsEndfieldModule = ArknightsEndfieldModule;

pub fn module() -> &'static dyn LibraryModule {
    &MODULE
}

pub fn register(registry: &mut ModuleRegistry) -> perwiga_core::Result<()> {
    registry.register(module())
}

pub fn operator_thumbnail(source_key: &str) -> Option<&'static [u8]> {
    match source_key {
        "akekuri" => Some(include_bytes!("../assets/operators/akekuri.webp")),
        "alesh" => Some(include_bytes!("../assets/operators/alesh.webp")),
        "antal" => Some(include_bytes!("../assets/operators/antal.webp")),
        "arclight" => Some(include_bytes!("../assets/operators/arclight.webp")),
        "ardelia" => Some(include_bytes!("../assets/operators/ardelia.webp")),
        "avywenna" => Some(include_bytes!("../assets/operators/avywenna.webp")),
        "camille" => Some(include_bytes!("../assets/operators/camille.webp")),
        "catcher" => Some(include_bytes!("../assets/operators/catcher.webp")),
        "chen" => Some(include_bytes!("../assets/operators/chen.webp")),
        "dapan" => Some(include_bytes!("../assets/operators/dapan.webp")),
        "ember" => Some(include_bytes!("../assets/operators/ember.webp")),
        "endministrator1" => Some(include_bytes!("../assets/operators/endministrator1.webp")),
        "endministrator2" => Some(include_bytes!("../assets/operators/endministrator2.webp")),
        "estella" => Some(include_bytes!("../assets/operators/estella.webp")),
        "fluorite" => Some(include_bytes!("../assets/operators/fluorite.webp")),
        "gilberta" => Some(include_bytes!("../assets/operators/gilberta.webp")),
        "laevatain" => Some(include_bytes!("../assets/operators/laevatain.webp")),
        "lastrite" => Some(include_bytes!("../assets/operators/lastrite.webp")),
        "lifeng" => Some(include_bytes!("../assets/operators/lifeng.webp")),
        "liino" => Some(include_bytes!("../assets/operators/liino.webp")),
        "lizhiyan" => Some(include_bytes!("../assets/operators/lizhiyan.webp")),
        "mifu" => Some(include_bytes!("../assets/operators/mifu.webp")),
        "perlica" => Some(include_bytes!("../assets/operators/perlica.webp")),
        "pogranichnik" => Some(include_bytes!("../assets/operators/pogranichnik.webp")),
        "purrche" => Some(include_bytes!("../assets/operators/purrche.webp")),
        "rossi" => Some(include_bytes!("../assets/operators/rossi.webp")),
        "snowshine" => Some(include_bytes!("../assets/operators/snowshine.webp")),
        "tangtang" => Some(include_bytes!("../assets/operators/tangtang.webp")),
        "typhoea" => Some(include_bytes!("../assets/operators/typhoea.webp")),
        "wulfgard" => Some(include_bytes!("../assets/operators/wulfgard.webp")),
        "xaihi" => Some(include_bytes!("../assets/operators/xaihi.webp")),
        "yvonne" => Some(include_bytes!("../assets/operators/yvonne.webp")),
        "zhuangfy" => Some(include_bytes!("../assets/operators/zhuangfy.webp")),
        _ => None,
    }
}

pub fn import_curated_aliases(
    store: &mut Store,
    work_id: &str,
) -> perwiga_core::Result<CuratedAliasImportSummary> {
    let work = store
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if work.kind != WorkKind::Game || work.module_id != "arknights-endfield" {
        return Err(PerwigaError::Validation(format!(
            "work {work_id} is not owned by the Arknights: Endfield module"
        )));
    }

    let mut resolved = HashSet::new();
    let mut inputs = Vec::new();
    for entity in store.list_entities(work_id, Some("operator"))? {
        let Some(source_key) = operator_source_key(&entity) else {
            continue;
        };
        let Some(operator) = curated_operator(source_key) else {
            continue;
        };
        resolved.insert(source_key.to_string());
        inputs.push(EntityAliasBatchInput {
            entity_id: entity.id.clone(),
            alias: AliasInput {
                value: operator.han_viet_name.clone(),
                language: Some("vi".to_string()),
                kind: "han-viet".to_string(),
                label: Some("Hán-Việt (generated)".to_string()),
                notes: Some(format!(
                    "Generated search alias from the verified official Simplified Chinese name; not an official Vietnamese localization. Sources: https://endfield.hypergryph.com/operator and https://hvdic.thivien.net/transcript.php#trans. Checked {}. Confidence E (generated).",
                    curated_snapshot().checked_at
                )),
            },
        });

        let primary_names = [
            Some(entity.official_english_name.as_str()),
            Some(entity.official_original_name.as_str()),
            entity.official_vietnamese_name.as_deref(),
            entity.automatic_vietnamese_translation.as_deref(),
        ];
        for localization in &curated_snapshot().official_localizations {
            let Some(value) = localization.names.get(source_key) else {
                continue;
            };
            if primary_names.iter().flatten().any(|name| *name == value) {
                continue;
            }
            inputs.push(EntityAliasBatchInput {
                entity_id: entity.id.clone(),
                alias: AliasInput {
                    value: value.clone(),
                    language: Some(localization.language.clone()),
                    kind: "official-localization".to_string(),
                    label: Some(localization.label.clone()),
                    notes: Some(format!(
                        "Official localized operator name from {}. Checked {}. Confidence A.",
                        localization.source_url,
                        curated_snapshot().checked_at
                    )),
                },
            });
        }

        for alias in curated_snapshot()
            .additional_aliases
            .iter()
            .filter(|alias| alias.source_key == source_key)
        {
            inputs.push(EntityAliasBatchInput {
                entity_id: entity.id.clone(),
                alias: AliasInput {
                    value: alias.value.clone(),
                    language: alias.language.clone(),
                    kind: alias.kind.clone(),
                    label: Some(alias.label.clone()),
                    notes: Some(alias.notes.clone()),
                },
            });
        }
    }

    let batch = store.add_aliases_batch(&inputs)?;
    let unresolved = curated_operators()
        .iter()
        .filter(|operator| !resolved.contains(&operator.source_key))
        .map(|operator| operator.source_key.clone())
        .collect();
    Ok(CuratedAliasImportSummary {
        inserted: batch.inserted,
        unchanged: batch.unchanged,
        unresolved,
    })
}

#[cfg(test)]
mod tests {
    use perwiga_core::{
        model::{EntityInput, WikiEntity},
        Store, WorkKind,
    };

    use super::*;

    fn operator(source_key: &str) -> WikiEntity {
        WikiEntity {
            id: "operator-1".into(),
            work_id: "work-1".into(),
            entity_type: "operator".into(),
            official_english_name: "Akekuri".into(),
            official_original_name: "秋栗".into(),
            official_vietnamese_name: Some("Akekuri".into()),
            automatic_vietnamese_translation: None,
            english_description: None,
            other_information: Some(format!("Official source key: {source_key}. Rarity: 4★.")),
            created_at: "2026-08-23T00:00:00Z".into(),
            updated_at: "2026-08-23T00:00:00Z".into(),
        }
    }

    #[test]
    fn operator_presentation_uses_module_owned_rarity_and_thumbnail_metadata() {
        let presentation = module()
            .entity_presentation(&operator("akekuri"))
            .expect("curated operator presentation");

        assert_eq!(presentation.label, "4★");
        assert_eq!(presentation.accent_color, "#55a8ff");
        assert_eq!(
            presentation.thumbnail_url,
            "/assets/modules/arknights-endfield/operators/akekuri.webp"
        );
        assert!(operator_thumbnail("akekuri").is_some());
        assert!(operator_thumbnail("../akekuri").is_none());

        let five_star = module()
            .entity_presentation(&operator("purrche"))
            .expect("five-star presentation");
        assert_eq!(five_star.label, "5★");
        assert_eq!(five_star.accent_color, "#d9a441");

        let six_star = module()
            .entity_presentation(&operator("typhoea"))
            .expect("six-star presentation");
        assert_eq!(six_star.label, "6★");
        assert_eq!(six_star.accent_color, "#b51e3f");
    }

    #[test]
    fn curated_multilingual_alias_import_is_transactional_and_idempotent() {
        let mut store = Store::open_in_memory().expect("temporary database");
        let work = store
            .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
            .expect("Endfield work");
        let akekuri = store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "operator".into(),
                    official_english_name: "Akekuri".into(),
                    official_original_name: "秋栗".into(),
                    official_vietnamese_name: Some("Akekuri".into()),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some("Official source key: akekuri. Rarity: 4★.".into()),
                },
            )
            .expect("operator");
        let chen = store
            .insert_entity(
                &work.id,
                &EntityInput {
                    entity_type: "operator".into(),
                    official_english_name: "Chen Qianyu".into(),
                    official_original_name: "陈千语".into(),
                    official_vietnamese_name: Some("Chen Qianyu".into()),
                    automatic_vietnamese_translation: None,
                    english_description: None,
                    other_information: Some("Official source key: chen. Rarity: 5★.".into()),
                },
            )
            .expect("operator");

        let first = import_curated_aliases(&mut store, &work.id).expect("first import");
        assert_eq!(first.inserted, 12);
        assert_eq!(first.unchanged, 0);
        assert_eq!(first.unresolved.len(), 31);

        let akekuri_aliases = store.list_aliases(&akekuri.id).expect("Akekuri aliases");
        assert_eq!(akekuri_aliases.len(), 5);
        assert!(akekuri_aliases.iter().any(|alias| {
            alias.value == "Thu Lật"
                && alias.language.as_deref() == Some("vi")
                && alias.kind == "han-viet"
                && alias.label.as_deref() == Some("Hán-Việt (generated)")
        }));
        assert!(akekuri_aliases.iter().any(|alias| {
            alias.value == "アケクリ"
                && alias.language.as_deref() == Some("ja")
                && alias.kind == "official-localization"
        }));

        let chen_aliases = store.list_aliases(&chen.id).expect("Chen aliases");
        assert_eq!(chen_aliases.len(), 7);
        assert!(chen_aliases.iter().any(|alias| {
            alias.value == "小陈"
                && alias.language.as_deref() == Some("zh-CN")
                && alias.kind == "fanfic"
                && alias.label.as_deref() == Some("[fanfic] Web-novel shorthand")
        }));

        let second = import_curated_aliases(&mut store, &work.id).expect("second import");
        assert_eq!(second.inserted, 0);
        assert_eq!(second.unchanged, 12);
        assert_eq!(store.list_aliases(&akekuri.id).unwrap().len(), 5);
        assert_eq!(store.list_aliases(&chen.id).unwrap().len(), 7);
    }
}
