use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use perwiga_core::{lore::LoreCandidateBatch, Store, WorkKind};
use perwiga_web::{router_with_store, validate_bind_address};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn json_response(response: axum::response::Response) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("JSON response")
}

#[tokio::test]
async fn endfield_setup_is_idempotent() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");

    let first = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("first setup response");
    assert_eq!(first.status(), StatusCode::OK);
    let first_json = json_response(first).await;

    let second = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("second setup response");
    assert_eq!(second.status(), StatusCode::OK);
    let second_json = json_response(second).await;

    assert_eq!(first_json["work"]["id"], second_json["work"]["id"]);
    assert_eq!(first_json["work"]["module_id"], "arknights-endfield");
    assert_eq!(
        first_json["lore_schema"]["subject_types"][0]["key"],
        "operator"
    );
    assert!(first_json["lore_schema"]["roles"]
        .as_array()
        .expect("lore roles")
        .iter()
        .any(|role| role["key"] == "witness"));
    assert_eq!(first_json["entity_types"].as_array().unwrap().len(), 10);
    let facets = first_json["entity_facets"]
        .as_array()
        .expect("entity facet definitions");
    let weapon_type = facets
        .iter()
        .find(|facet| facet["key"] == "weapon_type")
        .expect("weapon type facet");
    assert_eq!(weapon_type["display_name"], "Weapon type");
    assert_eq!(
        weapon_type["entity_types"],
        json!(["operator", "weapon", "essence"])
    );
    assert_eq!(
        weapon_type["options"],
        json!([
            {"value": "Sword", "display_name": "Sword"},
            {"value": "Greatsword", "display_name": "Greatsword"},
            {"value": "Polearm", "display_name": "Polearm"},
            {"value": "Handcannon", "display_name": "Handcannon"},
            {"value": "Arts Unit", "display_name": "Arts Unit"}
        ])
    );
    for key in ["role", "element", "race", "subrace"] {
        let facet = facets
            .iter()
            .find(|facet| facet["key"] == key)
            .unwrap_or_else(|| panic!("missing {key} facet"));
        assert_eq!(facet["entity_types"], json!(["operator"]));
    }
    let item_type = facets
        .iter()
        .find(|facet| facet["key"] == "item_type")
        .expect("item type facet");
    assert_eq!(item_type["display_name"], "Type");
    assert_eq!(item_type["entity_types"], json!(["item"]));
    assert!(item_type["options"]
        .as_array()
        .expect("item type options")
        .iter()
        .any(|option| option["value"] == "crafting-ingredient"));
    let region = facets
        .iter()
        .find(|facet| facet["key"] == "region")
        .expect("item region facet");
    assert_eq!(region["entity_types"], json!(["item"]));
    assert_eq!(
        region["options"],
        json!([
            {"value": "Valley IV", "display_name": "Valley IV"},
            {"value": "Wuling", "display_name": "Wuling"}
        ])
    );

    let work_id = first_json["work"]["id"].as_str().expect("work id");
    let lore_graph = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/lore-graph?limit=500"))
                .body(Body::empty())
                .expect("lore graph request"),
        )
        .await
        .expect("lore graph response");
    assert_eq!(lore_graph.status(), StatusCode::OK);
    let lore_graph = json_response(lore_graph).await;
    assert!(lore_graph["periods"].as_array().unwrap().len() >= 4);
    assert_eq!(lore_graph["events"].as_array().unwrap().len(), 200);
    assert!(!lore_graph["relations"].as_array().unwrap().is_empty());
    assert!(!lore_graph["involvements"].as_array().unwrap().is_empty());
    assert!(lore_graph["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|subject| subject["attested_name"] == "Talos-II"
            && !subject["wiki_entity_id"].is_null()));
    let character_subjects = lore_graph["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|subject| matches!(subject["entity_type"].as_str(), Some("operator" | "npc")))
        .collect::<Vec<_>>();
    assert_eq!(character_subjects.len(), 167);
    assert!(character_subjects
        .iter()
        .all(|subject| !subject["wiki_entity_id"].is_null()));
    let avywenna = lore_graph["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|subject| subject["attested_name"] == "Avywenna")
        .expect("Avywenna lore subject");
    assert_eq!(avywenna["entity_type"], "operator");
    assert_eq!(
        avywenna["presentation"]["thumbnail_url"],
        "/assets/modules/arknights-endfield/operators/avywenna.webp"
    );
    let visitor = lore_graph["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["title_en"] == "Visitor from the Band I")
        .expect("Avywenna quest on lore timeline");
    let wuling = lore_graph["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["title_en"] == "Chapter II sends Endministrators to Wuling")
        .expect("Wuling timeline event");
    assert!(visitor["period_order"].as_i64() < wuling["period_order"].as_i64());

    for subject_type in ["operator", "npc", "region"] {
        let filtered = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/works/{work_id}/lore-graph?limit=500&subject_type={subject_type}"
                ))
                .body(Body::empty())
                .expect("filtered lore graph request"),
            )
            .await
            .expect("filtered lore graph response");
        assert_eq!(filtered.status(), StatusCode::OK);
        let filtered = json_response(filtered).await;
        let filtered_events = filtered["events"].as_array().expect("filtered events");
        assert!(!filtered_events.is_empty());
        assert!(filtered_events.len() < lore_graph["events"].as_array().unwrap().len());
        for event in filtered_events {
            let event_id = event["id"].as_str().expect("event id");
            assert!(filtered["involvements"]
                .as_array()
                .unwrap()
                .iter()
                .any(|involvement| {
                    involvement["event_id"] == event_id
                        && filtered["subjects"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|subject| {
                                subject["id"] == involvement["subject_id"]
                                    && subject["entity_type"] == subject_type
                            })
                }));
        }
    }
    let invalid_filter = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/lore-graph?subject_type=faction"
            ))
            .body(Body::empty())
            .expect("invalid lore filter request"),
        )
        .await
        .expect("invalid lore filter response");
    assert_eq!(invalid_filter.status(), StatusCode::BAD_REQUEST);

    let wiki_events = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=event"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Wiki event list response");
    assert_eq!(wiki_events.status(), StatusCode::OK);
    let wiki_events = json_response(wiki_events).await;
    let wiki_events = wiki_events.as_array().expect("Wiki event list");
    assert_eq!(wiki_events.len(), 74);
    let version = wiki_events
        .iter()
        .find(|event| event["official_english_name"] == "Zeroth Directive 1.0")
        .expect("Zeroth Directive Wiki event");
    assert_eq!(version["official_original_name"], "Zeroth Directive 1.0");
    assert_eq!(
        version["english_description"],
        "Version start is the official release; end is the next announced maintenance start."
    );
    assert!(version["other_information"]
        .as_str()
        .expect("event provenance")
        .contains("Curated event source key: version-1-0."));
    let mut wiki_event_titles = wiki_events
        .iter()
        .map(|event| {
            event["official_english_name"]
                .as_str()
                .expect("Wiki event title")
                .to_string()
        })
        .collect::<Vec<_>>();
    wiki_event_titles.sort();

    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/works/{work_id}/entities"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "entity_type": "operator",
                        "official_english_name": "Laevatain",
                        "official_original_name": "莱万汀",
                        "official_vietnamese_name": "Laevatain",
                        "other_information": "Official source key: laevatain. Rarity: 6★."
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create Laevatain response");
    assert_eq!(created.status(), StatusCode::CREATED);

    let operators = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?entity_type=operator"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("operator list response");
    assert_eq!(operators.status(), StatusCode::OK);
    let operators = json_response(operators).await;
    let laevatain = operators
        .as_array()
        .expect("operator list")
        .iter()
        .find(|operator| operator["official_english_name"] == "Laevatain")
        .expect("Laevatain");
    assert_eq!(
        laevatain["event_recency"],
        json!({
            "heading": "Last limited banner",
            "event_title": "Fest of Brilliance",
            "ended_at": "2026-06-05T06:00:00+08:00"
        })
    );
    let detail = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/entities/{}",
                laevatain["id"].as_str().expect("Laevatain ID")
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("Laevatain detail response");
    assert_eq!(detail.status(), StatusCode::OK);
    assert_eq!(
        json_response(detail).await["event_recency"]["event_title"],
        "Fest of Brilliance"
    );

    let timeline = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/calendar-events"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("timeline response");
    assert_eq!(timeline.status(), StatusCode::OK);
    let events = json_response(timeline).await;
    let events = events.as_array().expect("timeline events");
    assert!(events.len() >= 55);
    let mut calendar_event_titles = events
        .iter()
        .map(|event| {
            event["title"]
                .as_str()
                .expect("calendar event title")
                .to_string()
        })
        .collect::<Vec<_>>();
    calendar_event_titles.sort();
    assert_eq!(wiki_event_titles, calendar_event_titles);
    assert!(events.iter().any(|event| {
        event["title"] == "Sanity Supply 1.4.2"
            && event["event_type"] == "Supply"
            && event["starts_at"] == "2026-08-26T04:00:00+08:00"
            && event["ends_at"] == "2026-09-02T04:00:00+08:00"
    }));

    let scars = events
        .iter()
        .find(|event| event["source_identity"] == "scars-of-the-forge-1-0")
        .expect("Scars of the Forge");
    assert_eq!(scars["presentation"]["heading"], "Rate-UP Operator");
    assert_eq!(
        scars["presentation"]["featured_entities"],
        json!([{
            "display_name": "Laevatain",
            "thumbnail_url": "/assets/modules/arknights-endfield/operators/laevatain.webp",
            "accent_color": "#b51e3f",
            "label": "6★"
        }])
    );

    let fest = events
        .iter()
        .find(|event| event["source_identity"] == "fest-of-brilliance-1-2")
        .expect("Fest of Brilliance");
    assert_eq!(fest["presentation"]["heading"], "Featured Operators");
    let fest_operators = fest["presentation"]["featured_entities"]
        .as_array()
        .expect("featured Operators");
    assert_eq!(
        fest_operators
            .iter()
            .map(|operator| operator["display_name"].as_str().expect("display name"))
            .collect::<Vec<_>>(),
        ["Laevatain", "Gilberta", "Ardelia", "Pogranichnik"]
    );
    let laevatain_gap = fest_operators
        .iter()
        .find(|operator| operator["display_name"] == "Laevatain")
        .expect("Laevatain gap");
    assert_eq!(laevatain_gap["previous_event_gap"]["days"], 96);
    assert_eq!(
        laevatain_gap["previous_event_gap"]["event_title"],
        "Scars of the Forge"
    );
    let gilberta_gap = fest_operators
        .iter()
        .find(|operator| operator["display_name"] == "Gilberta")
        .expect("Gilberta gap");
    assert_eq!(gilberta_gap["previous_event_gap"]["days"], 79);
    assert!(fest_operators
        .iter()
        .find(|operator| operator["display_name"] == "Ardelia")
        .expect("Ardelia")
        .get("previous_event_gap")
        .is_none());

    for (source_identity, operator_name, source_key) in [
        ("red-knight-1-1", "Rossi", "rossi"),
        ("fistful-reflections-1-3", "Mi Fu", "mifu"),
        ("danse-macabre-1-3", "Camille", "camille"),
        ("star-streaking-boundaries-1-4", "Liino", "liino"),
    ] {
        let narrative = events
            .iter()
            .find(|event| event["source_identity"] == source_identity)
            .unwrap_or_else(|| panic!("missing narrative event {source_identity}"));
        assert_eq!(narrative["presentation"]["heading"], "Featured Operator");
        assert_eq!(
            narrative["presentation"]["featured_entities"],
            json!([{
                "display_name": operator_name,
                "thumbnail_url": format!(
                    "/assets/modules/arknights-endfield/operators/{source_key}.webp"
                ),
                "accent_color": "#b51e3f",
                "label": "6★"
            }])
        );
    }

    let supply = events
        .iter()
        .find(|event| event["source_identity"] == "sanity-supply-1-4-2")
        .expect("Sanity Supply");
    assert!(supply["presentation"].is_null());
}

#[tokio::test]
async fn lore_graph_and_review_endpoints_keep_candidates_reviewable() {
    let mut store = Store::open_in_memory().expect("in-memory database");
    let work = store
        .insert_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
        .expect("work");
    let batch: LoreCandidateBatch = serde_json::from_value(json!({
        "schema": "perwiga-lore-candidates/v1",
        "module_id": "arknights-endfield",
        "corpus": {"version": "official-fixture-1", "manifest_sha256": "web-fixture"},
        "generator": {"name": "fixture", "version": "1", "generated_at": "2026-08-29T00:00:00Z"},
        "sources": [{
            "key": "source-1", "kind": "official_archive", "provider": "Endfield official",
            "identity": "archive-1", "title": "Archive", "language": "en",
            "url": "https://endfield.gryphline.com/en-us/archive/1",
            "content_sha256": "content-1", "checked_at": "2026-08-29"
        }],
        "periods": [{"key": "era-1", "name_en": "Era One", "description_en": null, "order": 1, "parent_key": null}],
        "subjects": [{
            "key": "witness", "attested_name": "Unknown Witness", "proposed_type": "unknown",
            "entity_ref": null,
            "evidence": [{"source_key": "source-1", "locator": "archive#1", "excerpt": "A witness spoke."}]
        }],
        "events": [{
            "key": "event-1", "title_en": "A witnessed arrival", "summary_en": "An arrival was recorded.",
            "time": {"kind": "moment", "precision": "relative", "label": "Era One", "start_period_key": "era-1", "end_period_key": null, "relations": []},
            "involvements": [{"subject_key": "witness", "role": "witness"}],
            "claims": [{
                "key": "claim-1", "text_en": "An arrival was recorded.", "assertion_kind": "direct_fact", "certainty": "confirmed",
                "branch_group": null, "involvements": [],
                "evidence": [{"source_key": "source-1", "locator": "archive#1", "excerpt": "An arrival was recorded."}]
            }]
        }]
    }))
    .expect("candidate batch");
    store
        .import_lore_candidate_batch(&work.id, &batch)
        .expect("candidate import");
    let app = router_with_store(store).expect("UAT router");

    let pending = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{}/lore-candidates?status=pending",
                work.id
            ))
            .body(Body::empty())
            .expect("pending request"),
        )
        .await
        .expect("pending response");
    assert_eq!(pending.status(), StatusCode::OK);
    let pending_json = json_response(pending).await;
    let event_candidate = pending_json
        .as_array()
        .expect("pending candidates")
        .iter()
        .find(|candidate| candidate["candidate_kind"] == "event")
        .and_then(|candidate| candidate["id"].as_str())
        .expect("event candidate ID")
        .to_string();

    let reviewed = app
        .clone()
        .oneshot(
            Request::post(format!("/api/lore-candidates/{event_candidate}/decisions"))
                .header("content-type", "application/json")
                .body(Body::from(json!({"decision": "approve"}).to_string()))
                .expect("review request"),
        )
        .await
        .expect("review response");
    assert_eq!(reviewed.status(), StatusCode::OK);
    assert_eq!(json_response(reviewed).await["status"], "approved");

    let remaining = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{}/lore-candidates?status=pending",
                work.id
            ))
            .body(Body::empty())
            .expect("remaining candidates request"),
        )
        .await
        .expect("remaining candidates response");
    let remaining_json = json_response(remaining).await;
    for candidate_id in remaining_json
        .as_array()
        .expect("remaining candidates")
        .iter()
        .filter_map(|candidate| candidate["id"].as_str())
    {
        let response = app
            .clone()
            .oneshot(
                Request::post(format!("/api/lore-candidates/{candidate_id}/decisions"))
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"decision": "approve"}).to_string()))
                    .expect("remaining review request"),
            )
            .await
            .expect("remaining review response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let graph = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{}/lore-graph?limit=10", work.id))
                .body(Body::empty())
                .expect("graph request"),
        )
        .await
        .expect("graph response");
    assert_eq!(graph.status(), StatusCode::OK);
    let graph_json = json_response(graph).await;
    let event_id = graph_json["events"][0]["id"]
        .as_str()
        .expect("materialized event ID");

    let detail = app
        .clone()
        .oneshot(
            Request::get(format!("/api/lore-events/{event_id}"))
                .body(Body::empty())
                .expect("detail request"),
        )
        .await
        .expect("detail response");
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_json = json_response(detail).await;
    assert_eq!(detail_json["claims"].as_array().unwrap().len(), 1);
    assert_eq!(detail_json["evidence"].as_array().unwrap().len(), 1);

    let subjects = app
        .oneshot(
            Request::get(format!("/api/works/{}/lore-subjects", work.id))
                .body(Body::empty())
                .expect("subjects request"),
        )
        .await
        .expect("subjects response");
    assert_eq!(subjects.status(), StatusCode::OK);
    assert_eq!(json_response(subjects).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn genshin_setup_is_idempotent_and_lists_a_switchable_game() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");

    let first = app
        .clone()
        .oneshot(
            Request::post("/api/uat/genshin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("first Genshin setup response");
    assert_eq!(first.status(), StatusCode::OK);
    let first_json = json_response(first).await;

    let second = app
        .clone()
        .oneshot(
            Request::post("/api/uat/genshin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("second Genshin setup response");
    assert_eq!(second.status(), StatusCode::OK);
    let second_json = json_response(second).await;

    assert_eq!(first_json["work"]["id"], second_json["work"]["id"]);
    assert_eq!(first_json["work"]["module_id"], "genshin-impact");
    assert_eq!(first_json["work"]["display_name"], "Genshin Impact");
    assert_eq!(
        first_json["entity_types"]
            .as_array()
            .expect("Genshin entity types")
            .iter()
            .map(|entity_type| entity_type["key"].as_str().expect("entity type key"))
            .collect::<Vec<_>>(),
        vec![
            "character",
            "npc",
            "region",
            "weapon",
            "skin",
            "artifact-set",
            "artifact-piece",
            "domain",
            "event",
            "book",
            "quest",
        ]
    );
    assert!(first_json["lore_schema"]["subject_types"]
        .as_array()
        .unwrap()
        .iter()
        .any(|subject_type| subject_type["key"] == "book"));
    assert!(first_json["lore_schema"]["roles"]
        .as_array()
        .unwrap()
        .iter()
        .any(|role| role["key"] == "document"));

    let facets = first_json["entity_facets"]
        .as_array()
        .expect("Genshin facets");
    for key in ["weapon_type", "main_stat", "substat", "ascension_region"] {
        let facet = facets
            .iter()
            .find(|facet| facet["key"] == key)
            .unwrap_or_else(|| panic!("missing Genshin {key} facet"));
        assert_eq!(facet["entity_types"], json!(["weapon"]));
    }
    for key in ["skin_application", "skin_character", "skin_weapon_type"] {
        let facet = facets
            .iter()
            .find(|facet| facet["key"] == key)
            .unwrap_or_else(|| panic!("missing Genshin {key} facet"));
        assert_eq!(facet["entity_types"], json!(["skin"]));
    }
    assert_eq!(
        facets
            .iter()
            .find(|facet| facet["key"] == "artifact_slot")
            .expect("Artifact slot facet")["entity_types"],
        json!(["artifact-piece"])
    );
    assert_eq!(
        facets
            .iter()
            .find(|facet| facet["key"] == "domain_region")
            .expect("Domain region facet")["entity_types"],
        json!(["domain"])
    );
    for key in ["region_type", "parent_region"] {
        let facet = facets
            .iter()
            .find(|facet| facet["key"] == key)
            .unwrap_or_else(|| panic!("missing Genshin {key} facet"));
        assert_eq!(facet["entity_types"], json!(["region"]));
    }

    let works = app
        .clone()
        .oneshot(
            Request::get("/api/works?kind=game")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("work list response");
    let works_json = json_response(works).await;
    assert!(works_json
        .as_array()
        .expect("work list")
        .iter()
        .any(|work| work["module_id"] == "genshin-impact"));

    let modules = app
        .clone()
        .oneshot(
            Request::get("/api/modules?kind=game")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("module list response");
    let modules_json = json_response(modules).await;
    assert!(modules_json
        .as_array()
        .expect("module list")
        .iter()
        .any(|module| {
            module["id"] == "genshin-impact" && module["display_name"] == "Genshin Impact"
        }));

    let work_id = first_json["work"]["id"].as_str().expect("Genshin work id");
    for (entity_type, expected_count) in [("book", 47), ("quest", 2545)] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/works/{work_id}/entities?entity_type={entity_type}"
                ))
                .body(Body::empty())
                .expect("Genshin lore entity list request"),
            )
            .await
            .expect("Genshin lore entity list response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json_response(response).await.as_array().unwrap().len(),
            expected_count
        );
    }
    let lore_graph = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/lore-graph?limit=10&subject_type=quest"
            ))
            .body(Body::empty())
            .expect("Genshin lore graph request"),
        )
        .await
        .expect("Genshin lore graph response");
    assert_eq!(lore_graph.status(), StatusCode::OK);
    assert_eq!(
        json_response(lore_graph).await["events"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    let first_lore_page = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/lore-graph?limit=500"))
                .body(Body::empty())
                .expect("first Genshin lore page request"),
        )
        .await
        .expect("first Genshin lore page response");
    assert_eq!(first_lore_page.status(), StatusCode::OK);
    let first_lore_page = json_response(first_lore_page).await;
    let first_lore_events = first_lore_page["events"]
        .as_array()
        .expect("first lore page events");
    assert_eq!(first_lore_events.len(), 500);
    let first_last_period = first_lore_events
        .last()
        .and_then(|event| event["period_order"].as_i64())
        .expect("first lore page period");
    let next_cursor = first_lore_page["next_cursor"]
        .as_str()
        .expect("next lore page cursor");
    let second_lore_page = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/lore-graph?limit=500&cursor={next_cursor}"
            ))
            .body(Body::empty())
            .expect("second Genshin lore page request"),
        )
        .await
        .expect("second Genshin lore page response");
    assert_eq!(second_lore_page.status(), StatusCode::OK);
    let second_lore_page = json_response(second_lore_page).await;
    let second_lore_events = second_lore_page["events"]
        .as_array()
        .expect("second lore page events");
    assert_eq!(second_lore_events.len(), 500);
    let second_first_period = second_lore_events
        .first()
        .and_then(|event| event["period_order"].as_i64())
        .expect("second lore page period");
    assert!(second_first_period >= first_last_period);
    assert!(first_lore_events.iter().all(|first_event| {
        second_lore_events
            .iter()
            .all(|second_event| first_event["id"] != second_event["id"])
    }));
    let wiki_events = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=event"))
                .body(Body::empty())
                .expect("Genshin Wiki event list request"),
        )
        .await
        .expect("Genshin Wiki event list response");
    assert_eq!(wiki_events.status(), StatusCode::OK);
    let wiki_events = json_response(wiki_events).await;
    assert_eq!(
        wiki_events.as_array().expect("Genshin Wiki events").len(),
        109
    );

    let timeline = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/calendar-events"))
                .body(Body::empty())
                .expect("Genshin timeline request"),
        )
        .await
        .expect("Genshin timeline response");
    assert_eq!(timeline.status(), StatusCode::OK);
    let timeline = json_response(timeline).await;
    let timeline = timeline.as_array().expect("Genshin timeline events");
    assert_eq!(timeline.len(), 109);
    let odette = timeline
        .iter()
        .find(|event| event["source_identity"] == "swan-shadow-silken-ice-7-0")
        .expect("Odette Character Wish");
    assert_eq!(odette["presentation"]["heading"], "Featured Character");
    assert_eq!(
        odette["presentation"]["featured_entities"][0]["display_name"],
        "Odette"
    );

    let characters = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?entity_type=character"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("Genshin character list response");
    assert_eq!(characters.status(), StatusCode::OK);
    let character_json = json_response(characters).await;
    assert_eq!(
        character_json.as_array().expect("character list").len(),
        118
    );
    let albedo = character_json
        .as_array()
        .expect("character list")
        .iter()
        .find(|character| character["official_english_name"] == "Albedo")
        .expect("Albedo");
    assert_eq!(albedo["presentation"]["label"], "5★");
    assert_eq!(albedo["presentation"]["rarity"], 5);
    assert_eq!(albedo["presentation"]["accent_color"], "#d8b66f");
    assert_eq!(albedo["presentation"]["context_label"], "Mondstadt");
    assert_eq!(
        albedo["presentation"]["context_icon_url"],
        "/assets/modules/genshin-impact/regions/mondstadt.webp"
    );
    assert_eq!(
        albedo["presentation"]["thumbnail_url"],
        "/assets/modules/genshin-impact/characters/hoyoverse-content-104816.png"
    );
    let han_viet_search = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?query=A%20B%E1%BB%91i%20%C4%90a"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("Genshin Hán-Việt search response");
    assert_eq!(han_viet_search.status(), StatusCode::OK);
    let han_viet_search_json = json_response(han_viet_search).await;
    assert_eq!(
        han_viet_search_json
            .as_array()
            .expect("Hán-Việt search result")
            .iter()
            .map(|character| character["official_english_name"]
                .as_str()
                .expect("character name"))
            .collect::<Vec<_>>(),
        vec!["Albedo"]
    );

    let npcs = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=npc"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Genshin NPC list response");
    assert_eq!(npcs.status(), StatusCode::OK);
    let npc_json = json_response(npcs).await;
    assert_eq!(npc_json.as_array().expect("NPC list").len(), 4_967);
    let liben = npc_json
        .as_array()
        .expect("NPC list")
        .iter()
        .find(|npc| npc["official_english_name"] == "Liben")
        .expect("Liben");
    assert_eq!(liben["official_original_name"], "立本");
    let liben_id = liben["id"].as_str().expect("Liben ID");
    let liben_detail = app
        .clone()
        .oneshot(
            Request::get(format!("/api/entities/{liben_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Liben detail response");
    assert_eq!(liben_detail.status(), StatusCode::OK);
    let liben_detail = json_response(liben_detail).await;
    assert_eq!(liben_detail["appearances"][0]["relation_kind"], "event");
    assert_eq!(
        liben_detail["appearances"][0]["related_title"],
        "Marvelous Merchandise"
    );
    assert_eq!(
        liben_detail["appearances"][0]["locations"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let aloy = character_json
        .as_array()
        .expect("character list")
        .iter()
        .find(|character| character["official_english_name"] == "Aloy")
        .expect("Aloy");
    assert_eq!(aloy["presentation"]["label"], "5★");
    assert_eq!(aloy["presentation"]["rarity"], 5);
    assert_eq!(aloy["presentation"]["accent_color"], "#d94b4b");

    let regions = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=region"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Genshin Region list response");
    assert_eq!(regions.status(), StatusCode::OK);
    let region_json = json_response(regions).await;
    assert_eq!(region_json.as_array().expect("Region list").len(), 217);
    let liyue_harbor = region_json
        .as_array()
        .expect("Region list")
        .iter()
        .find(|region| region["official_english_name"] == "Liyue Harbor")
        .expect("Liyue Harbor");
    assert_eq!(liyue_harbor["official_original_name"], "璃月港");
    assert_eq!(liyue_harbor["presentation"]["label"], "SUBREGION");
    assert_eq!(liyue_harbor["presentation"]["context_label"], "Liyue");
    assert_eq!(
        liyue_harbor["presentation"]["facets"]["region_type"],
        "Subregion"
    );
    assert_eq!(
        liyue_harbor["presentation"]["facets"]["parent_region"],
        "Liyue"
    );
    let region_han_viet_search = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?query=Ly%20Nguy%E1%BB%87t%20C%E1%BA%A3ng"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("Region Hán-Việt search response");
    assert!(json_response(region_han_viet_search)
        .await
        .as_array()
        .expect("Region Hán-Việt search result")
        .iter()
        .any(|entity| entity["official_english_name"] == "Liyue Harbor"));

    let thumbnail = app
        .clone()
        .oneshot(
            Request::get("/assets/modules/genshin-impact/characters/hoyoverse-content-104816.png")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Genshin thumbnail response");
    assert_eq!(thumbnail.status(), StatusCode::OK);
    assert_eq!(thumbnail.headers()["content-type"], "image/png");

    let region_icon = app
        .clone()
        .oneshot(
            Request::get("/assets/modules/genshin-impact/regions/mondstadt.webp")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Genshin region icon response");
    assert_eq!(region_icon.status(), StatusCode::OK);
    assert_eq!(region_icon.headers()["content-type"], "image/webp");
    let region_icon_bytes = region_icon
        .into_body()
        .collect()
        .await
        .expect("Genshin region icon body")
        .to_bytes();
    assert!(region_icon_bytes.starts_with(b"RIFF"));
    assert_eq!(&region_icon_bytes[8..12], b"WEBP");

    let weapons = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=weapon"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Genshin weapon list response");
    assert_eq!(weapons.status(), StatusCode::OK);
    let weapon_json = json_response(weapons).await;
    assert_eq!(weapon_json.as_array().expect("weapon list").len(), 246);
    let gravestone = weapon_json
        .as_array()
        .expect("weapon list")
        .iter()
        .find(|weapon| weapon["official_english_name"] == "Wolf's Gravestone")
        .expect("Wolf's Gravestone");
    assert_eq!(gravestone["presentation"]["rarity"], 5);
    assert_eq!(gravestone["presentation"]["accent_color"], "#d8b66f");
    assert_eq!(
        gravestone["presentation"]["facets"]["weapon_type"],
        "Claymore"
    );
    assert_eq!(gravestone["presentation"]["facets"]["main_stat"], "46");
    assert_eq!(gravestone["presentation"]["facets"]["substat"], "ATK");
    assert_eq!(
        gravestone["presentation"]["facets"]["ascension_region"],
        "Mondstadt"
    );
    assert_eq!(
        gravestone["presentation"]["thumbnail_url"],
        "/assets/modules/genshin-impact/weapons/genshin-data-weapon-12502.png"
    );
    let weapon_thumbnail = app
        .clone()
        .oneshot(
            Request::get("/assets/modules/genshin-impact/weapons/genshin-data-weapon-12502.png")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Genshin weapon thumbnail response");
    assert_eq!(weapon_thumbnail.status(), StatusCode::OK);
    assert_eq!(weapon_thumbnail.headers()["content-type"], "image/png");
    let weapon_han_viet_search = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?query=Lang%20%C4%90%C3%ADch%20M%E1%BA%A1t%20L%E1%BB%99"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("weapon Hán-Việt search response");
    assert_eq!(weapon_han_viet_search.status(), StatusCode::OK);
    let weapon_han_viet_json = json_response(weapon_han_viet_search).await;
    assert!(weapon_han_viet_json
        .as_array()
        .expect("weapon Hán-Việt search result")
        .iter()
        .any(|entity| entity["official_english_name"] == "Wolf's Gravestone"));

    let skins = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=skin"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Genshin Skin list response");
    assert_eq!(skins.status(), StatusCode::OK);
    let skin_json = json_response(skins).await;
    assert_eq!(skin_json.as_array().expect("Skin list").len(), 29);
    let diluc = skin_json
        .as_array()
        .expect("Skin list")
        .iter()
        .find(|skin| skin["official_english_name"] == "Red Dead of Night")
        .expect("Diluc Skin");
    assert_eq!(diluc["presentation"]["rarity"], 5);
    assert_eq!(diluc["presentation"]["accent_color"], "#d8b66f");
    assert_eq!(diluc["presentation"]["context_label"], "Diluc");
    assert_eq!(
        diluc["presentation"]["facet_values"]["skin_character"],
        json!(["Diluc"])
    );
    let traveler = skin_json
        .as_array()
        .expect("Skin list")
        .iter()
        .find(|skin| skin["official_english_name"] == "As Heaven and Earth Are Made Anew")
        .expect("Traveler Skin");
    assert_eq!(
        traveler["presentation"]["facet_values"]["skin_character"],
        json!(["Aether", "Lumine"])
    );
    assert!(traveler["presentation"]["thumbnail_url"]
        .as_str()
        .expect("Traveler thumbnail")
        .ends_with(".gif"));

    let skin_thumbnail = app
        .clone()
        .oneshot(
            Request::get("/assets/modules/genshin-impact/skins/hoyowiki-outfit-5777.png")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Skin thumbnail response");
    assert_eq!(skin_thumbnail.status(), StatusCode::OK);
    assert_eq!(skin_thumbnail.headers()["content-type"], "image/png");
    let traveler_thumbnail = app
        .clone()
        .oneshot(
            Request::get("/assets/modules/genshin-impact/skins/hoyowiki-outfit-9251.gif")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Traveler Skin thumbnail response");
    assert_eq!(traveler_thumbnail.status(), StatusCode::OK);
    assert_eq!(traveler_thumbnail.headers()["content-type"], "image/gif");

    let skin_han_viet_search = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?query=%C3%82n%20H%E1%BB%93ng%20Chung%20D%E1%BA%A1"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("Skin Hán-Việt search response");
    let skin_han_viet_json = json_response(skin_han_viet_search).await;
    assert!(skin_han_viet_json
        .as_array()
        .expect("Skin Hán-Việt search result")
        .iter()
        .any(|entity| entity["official_english_name"] == "Red Dead of Night"));

    let artifact_sets = json_response(
        app.clone()
            .oneshot(
                Request::get(format!(
                    "/api/works/{work_id}/entities?entity_type=artifact-set"
                ))
                .body(Body::empty())
                .expect("request"),
            )
            .await
            .expect("Artifact Set response"),
    )
    .await;
    assert_eq!(artifact_sets.as_array().expect("Artifact Sets").len(), 63);
    let gladiator = artifact_sets
        .as_array()
        .expect("Artifact Sets")
        .iter()
        .find(|entity| entity["official_english_name"] == "Gladiator's Finale")
        .expect("Gladiator set");
    assert_eq!(gladiator["presentation"]["rarity"], 5);
    assert_eq!(gladiator["presentation"]["accent_color"], "#d8b66f");

    let artifact_pieces = json_response(
        app.clone()
            .oneshot(
                Request::get(format!(
                    "/api/works/{work_id}/entities?entity_type=artifact-piece"
                ))
                .body(Body::empty())
                .expect("request"),
            )
            .await
            .expect("Artifact Piece response"),
    )
    .await;
    assert_eq!(
        artifact_pieces.as_array().expect("Artifact Pieces").len(),
        299
    );
    let flower = artifact_pieces
        .as_array()
        .expect("Artifact Pieces")
        .iter()
        .find(|entity| entity["official_english_name"] == "Gladiator's Nostalgia")
        .expect("Gladiator flower");
    assert_eq!(
        flower["presentation"]["context_label"],
        "Gladiator's Finale"
    );
    assert_eq!(
        flower["presentation"]["facets"]["artifact_slot"],
        "Flower of Life"
    );
    let artifact_thumbnail = app.clone().oneshot(Request::get("/assets/modules/genshin-impact/artifacts/genshin-data-artifact-piece-15001-flower.png").body(Body::empty()).expect("request")).await.expect("Artifact thumbnail response");
    assert_eq!(artifact_thumbnail.status(), StatusCode::OK);
    assert_eq!(artifact_thumbnail.headers()["content-type"], "image/png");

    let domains = json_response(
        app.clone()
            .oneshot(
                Request::get(format!("/api/works/{work_id}/entities?entity_type=domain"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("Domain response"),
    )
    .await;
    assert_eq!(domains.as_array().expect("Domains").len(), 22);
    let valley = domains
        .as_array()
        .expect("Domains")
        .iter()
        .find(|entity| entity["official_english_name"] == "Valley of Remembrance")
        .expect("Valley of Remembrance");
    assert_eq!(
        valley["presentation"]["facets"]["domain_region"],
        "Mondstadt"
    );

    let script = app
        .oneshot(
            Request::get("/assets/app.js")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("app script response");
    let javascript = String::from_utf8(
        script
            .into_body()
            .collect()
            .await
            .expect("app script body")
            .to_bytes()
            .to_vec(),
    )
    .expect("UTF-8 JavaScript");
    assert!(javascript.contains("api.setupGenshin()"));
    assert!(javascript.contains("api.setupStarRail()"));
    assert!(javascript.contains("for (const work of state.works)"));
    assert!(javascript.contains("chooseGame(work.id)"));
    assert!(javascript.contains("maybeLoadMoreLore"));
    assert!(javascript.contains("addEventListener(\"scroll\""));
    assert!(javascript.contains("dom.loreMap.setAttribute(\"aria-busy\", \"true\")"));
}

#[tokio::test]
async fn star_rail_setup_is_idempotent_and_imports_crawled_catalogs() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");

    let first = app
        .clone()
        .oneshot(
            Request::post("/api/uat/star-rail")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("first Star Rail setup response");
    assert_eq!(first.status(), StatusCode::OK);
    let first_json = json_response(first).await;

    let second = app
        .clone()
        .oneshot(
            Request::post("/api/uat/star-rail")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("second Star Rail setup response");
    assert_eq!(second.status(), StatusCode::OK);
    let second_json = json_response(second).await;

    assert_eq!(first_json["work"]["id"], second_json["work"]["id"]);
    assert_eq!(first_json["work"]["module_id"], "honkai-star-rail");
    assert_eq!(first_json["work"]["display_name"], "Honkai: Star Rail");
    assert_eq!(
        first_json["entity_types"]
            .as_array()
            .expect("Star Rail entity types")
            .iter()
            .map(|entity_type| entity_type["key"].as_str().expect("entity type key"))
            .collect::<Vec<_>>(),
        vec![
            "character",
            "npc",
            "region",
            "light-cone",
            "relic-set",
            "relic",
            "path",
            "element",
        ]
    );

    let facets = first_json["entity_facets"]
        .as_array()
        .expect("Star Rail facets");
    assert_eq!(
        facets
            .iter()
            .find(|facet| facet["key"] == "path")
            .expect("Path facet")["entity_types"],
        json!(["character", "light-cone"])
    );
    assert_eq!(
        facets
            .iter()
            .find(|facet| facet["key"] == "element")
            .expect("Element facet")["entity_types"],
        json!(["character"])
    );
    assert_eq!(
        facets
            .iter()
            .find(|facet| facet["key"] == "relic_slot")
            .expect("Relic slot facet")["entity_types"],
        json!(["relic"])
    );

    let work_id = first_json["work"]["id"]
        .as_str()
        .expect("Star Rail work id");
    for (entity_type, expected_count) in [
        ("character", 97),
        ("npc", 433),
        ("light-cone", 169),
        ("relic-set", 60),
        ("relic", 742),
        ("path", 9),
        ("element", 7),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/api/works/{work_id}/entities?entity_type={entity_type}"
                ))
                .body(Body::empty())
                .expect("entity list request"),
            )
            .await
            .expect("entity list response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json_response(response)
                .await
                .as_array()
                .expect("entity list")
                .len(),
            expected_count,
            "unexpected {entity_type} count"
        );
    }

    let characters = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?entity_type=character"
            ))
            .body(Body::empty())
            .expect("character list request"),
        )
        .await
        .expect("character list response");
    let characters = json_response(characters).await;
    let march = characters
        .as_array()
        .expect("character list")
        .iter()
        .find(|character| {
            character["official_english_name"] == "March 7th"
                && character["other_information"]
                    .as_str()
                    .is_some_and(|value| value.contains("Path: Preservation"))
        })
        .expect("March 7th Preservation variant");
    assert_eq!(march["official_original_name"], "三月七");
    assert_eq!(march["presentation"]["label"], "4★");
    assert_eq!(march["presentation"]["rarity"], 4);
    assert_eq!(march["presentation"]["facets"]["path"], "Preservation");
    assert_eq!(march["presentation"]["facets"]["element"], "Ice");
    assert_eq!(
        march["presentation"]["thumbnail_url"],
        "/assets/modules/honkai-star-rail/characters/starrailres-character-1001.png"
    );
    assert_eq!(
        march["presentation"]["context_icon_url"],
        "https://vizualabstract.github.io/StarRailStaticAPI/assets/icon/element/Ice.png"
    );

    let thumbnail = app
        .clone()
        .oneshot(
            Request::get(
                "/assets/modules/honkai-star-rail/characters/starrailres-character-1001.png",
            )
            .body(Body::empty())
            .expect("Star Rail character thumbnail request"),
        )
        .await
        .expect("Star Rail character thumbnail response");
    assert_eq!(thumbnail.status(), StatusCode::OK);
    assert_eq!(thumbnail.headers()[header::CONTENT_TYPE], "image/png");
    assert!(thumbnail
        .into_body()
        .collect()
        .await
        .expect("Star Rail character thumbnail body")
        .to_bytes()
        .starts_with(b"\x89PNG\r\n\x1a\n"));

    let trailblazer = characters
        .as_array()
        .expect("character list")
        .iter()
        .find(|character| {
            character["official_original_name"] == "{NICKNAME}"
                && character["catalog_label"] == "Trailblazer (Male) · Destruction · Physical"
        })
        .expect("male Trailblazer");
    assert_eq!(
        trailblazer["catalog_label"],
        "Trailblazer (Male) · Destruction · Physical"
    );
    let trailblazer_id = trailblazer["id"].as_str().expect("Trailblazer id");
    let detail = app
        .clone()
        .oneshot(
            Request::get(format!("/api/entities/{trailblazer_id}"))
                .body(Body::empty())
                .expect("Trailblazer detail request"),
        )
        .await
        .expect("Trailblazer detail response");
    assert_eq!(detail.status(), StatusCode::OK);
    assert_eq!(
        json_response(detail).await["catalog_label"],
        "Trailblazer (Male) · Destruction · Physical"
    );

    let han_viet_search = app
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?query=Tam%20Nguy%E1%BB%87t%20Th%E1%BA%A5t"
            ))
            .body(Body::empty())
            .expect("Hán-Việt search request"),
        )
        .await
        .expect("Hán-Việt search response");
    assert_eq!(han_viet_search.status(), StatusCode::OK);
    let han_viet_search = json_response(han_viet_search).await;
    let han_viet_results = han_viet_search.as_array().expect("Hán-Việt results");
    assert_eq!(han_viet_results.len(), 2);
    assert!(han_viet_results
        .iter()
        .all(|entity| entity["official_english_name"] == "March 7th"));
}

#[tokio::test]
async fn entity_round_trip_preserves_translation_provenance() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let setup = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("setup response");
    let setup_json = json_response(setup).await;
    let work_id = setup_json["work"]["id"].as_str().expect("work id");

    let payload = json!({
        "entity_type": "operator",
        "official_english_name": "Verified English",
        "official_original_name": "已验证原名",
        "official_vietnamese_name": "Tên chính thức",
        "automatic_vietnamese_translation": "Tên máy dịch",
        "english_description": "A UAT entity.",
        "other_information": "Created in a temporary database."
    });
    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/works/{work_id}/entities"))
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request"),
        )
        .await
        .expect("create response");
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_json = json_response(created).await;
    let entity_id = created_json["id"].as_str().expect("entity id");
    assert_eq!(created_json["official_vietnamese_name"], "Tên chính thức");
    assert_eq!(
        created_json["automatic_vietnamese_translation"],
        "Tên máy dịch"
    );

    let detail = app
        .clone()
        .oneshot(
            Request::get(format!("/api/entities/{entity_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("detail response");
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_json = json_response(detail).await;
    assert_eq!(
        detail_json["entity"]["official_original_name"],
        "已验证原名"
    );

    let listed = app
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?query=m%C3%A1y%20d%E1%BB%8Bch"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("list response");
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(json_response(listed).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn endfield_operator_list_exposes_rarity_presentation_and_local_thumbnail() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let setup = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("setup response");
    let setup_json = json_response(setup).await;
    let work_id = setup_json["work"]["id"].as_str().expect("work id");

    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/works/{work_id}/entities"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "entity_type": "operator",
                        "official_english_name": "Akekuri",
                        "official_original_name": "秋栗",
                        "official_vietnamese_name": "Akekuri",
                        "other_information": "Official source key: akekuri. Rarity: 4★."
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create response");
    assert_eq!(created.status(), StatusCode::CREATED);

    let listed = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?entity_type=operator"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("list response");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_json = json_response(listed).await;
    assert_eq!(listed_json[0]["presentation"]["label"], "4★");
    assert_eq!(listed_json[0]["presentation"]["rarity"], 4);
    assert_eq!(listed_json[0]["presentation"]["accent_color"], "#55a8ff");
    assert_eq!(
        listed_json[0]["presentation"]["facets"],
        json!({
            "role": "Vanguard",
            "element": "Heat",
            "weapon_type": "Sword",
            "race": "Perro"
        })
    );
    assert_eq!(
        listed_json[0]["presentation"]["thumbnail_url"],
        "/assets/modules/arknights-endfield/operators/akekuri.webp"
    );

    let thumbnail = app
        .oneshot(
            Request::get("/assets/modules/arknights-endfield/operators/akekuri.webp")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("thumbnail response");
    assert_eq!(thumbnail.status(), StatusCode::OK);
    assert_eq!(thumbnail.headers()["content-type"], "image/webp");
    let bytes = thumbnail
        .into_body()
        .collect()
        .await
        .expect("thumbnail body")
        .to_bytes();
    assert!(bytes.len() > 1_000);
}

#[tokio::test]
async fn endfield_weapon_list_exposes_rarity_border_and_local_thumbnail() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let setup = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("setup response");
    let setup_json = json_response(setup).await;
    let work_id = setup_json["work"]["id"].as_str().expect("work id");

    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/works/{work_id}/entities"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "entity_type": "weapon",
                        "official_english_name": "Aggeloslayer",
                        "official_original_name": "天使杀手",
                        "official_vietnamese_name": "Sát Thủ Diệt Aggeloi",
                        "other_information": "Curated source key: aggeloslayer. Catalog index: 007/77. Rarity: 4-star."
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create response");
    assert_eq!(created.status(), StatusCode::CREATED);

    let listed = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=weapon"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list response");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_json = json_response(listed).await;
    assert_eq!(listed_json[0]["presentation"]["label"], "4★");
    assert_eq!(listed_json[0]["presentation"]["rarity"], 4);
    assert_eq!(listed_json[0]["presentation"]["accent_color"], "#55a8ff");
    assert_eq!(
        listed_json[0]["presentation"]["facets"]["weapon_type"],
        "Polearm"
    );
    assert_eq!(
        listed_json[0]["presentation"]["thumbnail_url"],
        "/assets/modules/arknights-endfield/weapons/aggeloslayer.png"
    );

    let thumbnail = app
        .oneshot(
            Request::get("/assets/modules/arknights-endfield/weapons/aggeloslayer.png")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("thumbnail response");
    assert_eq!(thumbnail.status(), StatusCode::OK);
    assert_eq!(thumbnail.headers()["content-type"], "image/png");
    let bytes = thumbnail
        .into_body()
        .collect()
        .await
        .expect("thumbnail body")
        .to_bytes();
    assert!(bytes.len() > 1_000);
}

#[tokio::test]
async fn endfield_item_list_exposes_many_types_region_rarity_and_local_thumbnail() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let setup = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("setup response");
    let setup_json = json_response(setup).await;
    let work_id = setup_json["work"]["id"].as_str().expect("work id");

    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/works/{work_id}/entities"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "entity_type": "item",
                        "official_english_name": "Jincao",
                        "official_original_name": "锦草",
                        "official_vietnamese_name": "Cỏ Sao Lụa",
                        "other_information": "Curated source key: item_plant_grass_1. Catalog index: 037/731. Rarity: 1-star."
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create response");
    assert_eq!(created.status(), StatusCode::CREATED);

    let listed = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=item"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list response");
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_json = json_response(listed).await;
    assert_eq!(listed_json[0]["presentation"]["rarity"], 1);
    assert_eq!(listed_json[0]["presentation"]["facets"]["region"], "Wuling");
    assert_eq!(
        listed_json[0]["presentation"]["facet_values"]["item_type"],
        json!([
            "craftable",
            "crafting-ingredient",
            "material",
            "gatherable",
            "natural-resource",
            "plant"
        ])
    );
    assert_eq!(
        listed_json[0]["presentation"]["thumbnail_url"],
        "/assets/modules/arknights-endfield/items/item_plant_grass_1.webp"
    );

    let thumbnail = app
        .oneshot(
            Request::get("/assets/modules/arknights-endfield/items/item_plant_grass_1.webp")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("thumbnail response");
    assert_eq!(thumbnail.status(), StatusCode::OK);
    assert_eq!(thumbnail.headers()["content-type"], "image/webp");
    let bytes = thumbnail
        .into_body()
        .collect()
        .await
        .expect("thumbnail body")
        .to_bytes();
    assert!(bytes.len() > 1_000);
}

#[tokio::test]
async fn endfield_gear_sets_and_essences_are_seeded_with_readable_labels_and_facets() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let setup = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("setup response");
    let setup_json = json_response(setup).await;
    let work_id = setup_json["work"]["id"].as_str().expect("work id");

    let gear_response = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?entity_type=gear-set"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("Gear Set list response");
    let gear = json_response(gear_response).await;
    assert_eq!(gear.as_array().unwrap().len(), 23);
    let roving = gear
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["official_english_name"] == "Roving MSGR")
        .expect("Roving MSGR");
    assert_eq!(roving["presentation"]["label"], "3–4★");
    assert!(roving["presentation"]["thumbnail_url"]
        .as_str()
        .unwrap()
        .contains("item_equip_t2_suit_agi01_body_01"));

    let essence_response = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=essence"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Essence list response");
    let essences = json_response(essence_response).await;
    assert_eq!(essences.as_array().unwrap().len(), 154);
    let suppression = essences
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["catalog_label"]
                == "Flawless Essence — Industry 0.1 — Strength Boost Lv.4 · Attack Boost Lv.4 · Suppression Lv.2"
        })
        .expect("readable Essence catalog label");
    assert_eq!(suppression["official_english_name"], "Flawless Essence");
    assert_eq!(suppression["presentation"]["rarity"], 5);
    assert_eq!(
        suppression["presentation"]["facets"]["weapon_type"],
        "Greatsword"
    );
    assert_eq!(
        suppression["presentation"]["facets"]["essence_skill"],
        "Suppression"
    );

    let thumbnail = app
        .oneshot(
            Request::get("/assets/modules/arknights-endfield/items/item_gem_rarity_5.webp")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Essence thumbnail response");
    assert_eq!(thumbnail.status(), StatusCode::OK);
    assert_eq!(thumbnail.headers()["content-type"], "image/webp");
}

#[tokio::test]
async fn endfield_regions_are_seeded_with_han_viet_hierarchy_and_filters() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let setup = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("setup response");
    let setup_json = json_response(setup).await;
    let work_id = setup_json["work"]["id"].as_str().expect("work id");
    for (key, display_name) in [
        ("region_type", "Region type"),
        ("parent_region", "Parent region"),
    ] {
        let facet = setup_json["entity_facets"]
            .as_array()
            .expect("facet definitions")
            .iter()
            .find(|facet| facet["key"] == key)
            .unwrap_or_else(|| panic!("missing {key} facet"));
        assert_eq!(facet["display_name"], display_name);
        assert_eq!(facet["entity_types"], json!(["region"]));
    }

    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?entity_type=region"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Region list response");
    assert_eq!(response.status(), StatusCode::OK);
    let regions = json_response(response).await;
    assert_eq!(regions.as_array().expect("Region list").len(), 43);
    let jingyu = regions
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["official_english_name"] == "Jingyu Valley")
        .expect("Jingyu Valley");
    assert_eq!(jingyu["presentation"]["label"], "Subregion");
    assert_eq!(jingyu["presentation"]["facets"]["parent_region"], "Wuling");

    let search = app
        .oneshot(
            Request::get(format!(
                "/api/works/{work_id}/entities?query=C%E1%BA%A3nh%20Ng%E1%BB%8Dc%20C%E1%BB%91c"
            ))
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("Hán-Việt search response");
    let search = json_response(search).await;
    assert!(search
        .as_array()
        .expect("search results")
        .iter()
        .any(|entry| entry["official_english_name"] == "Jingyu Valley"));
}

#[tokio::test]
async fn entity_type_placeholders_are_served_and_replace_name_initials() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");

    for filename in [
        "character.svg",
        "place.svg",
        "concept.svg",
        "weapon.svg",
        "enemy.svg",
        "mission.svg",
        "event.svg",
        "item.svg",
        "faction.svg",
        "generic.svg",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/assets/placeholders/{filename}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("placeholder response");
        assert_eq!(response.status(), StatusCode::OK, "{filename}");
        assert_eq!(response.headers()["content-type"], "image/svg+xml");
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("placeholder body")
            .to_bytes();
        assert!(bytes.starts_with(b"<svg"), "{filename}");
    }

    let script = app
        .oneshot(
            Request::get("/assets/ui.js")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("UI script response");
    let bytes = script
        .into_body()
        .collect()
        .await
        .expect("UI script body")
        .to_bytes();
    let javascript = String::from_utf8(bytes.to_vec()).expect("UTF-8 JavaScript");
    assert!(javascript.contains("ENTITY_PLACEHOLDER_BY_TYPE"));
    assert!(javascript.contains("/assets/placeholders/"));
    assert!(javascript.contains("facet_values"));
    assert!(javascript.contains("presentation?.context_label"));
    assert!(javascript.contains("presentation?.context_icon_url"));
    assert!(javascript.contains("entity-context-icon"));
    assert!(javascript.contains("visually-hidden"));
    assert!(!javascript.contains("contextIcon.title"));
    assert!(!javascript.contains("function initials"));
    assert!(!javascript.contains("type-meta"));
    assert!(!javascript.contains("module index"));
}

#[tokio::test]
async fn event_timeline_assets_support_featured_operator_hover_and_focus_previews() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");

    let script = app
        .clone()
        .oneshot(
            Request::get("/assets/ui.js")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("UI script response");
    let javascript = String::from_utf8(
        script
            .into_body()
            .collect()
            .await
            .expect("UI script body")
            .to_bytes()
            .to_vec(),
    )
    .expect("UTF-8 JavaScript");
    assert!(javascript.contains("featured_entities"));
    assert!(javascript.contains("event-feature-preview"));
    assert!(javascript.contains("previous_event_gap"));
    assert!(javascript.contains("event_recency"));
    assert!(javascript.contains("wholeDaysSince"));
    assert!(javascript.contains("entityAppearanceSection"));
    assert!(javascript.contains("appearance.related_title"));
    assert!(!javascript.contains("bar.title = description"));

    let styles = app
        .oneshot(
            Request::get("/assets/styles.css")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("stylesheet response");
    let css = String::from_utf8(
        styles
            .into_body()
            .collect()
            .await
            .expect("stylesheet body")
            .to_bytes()
            .to_vec(),
    )
    .expect("UTF-8 CSS");
    assert!(css.contains(".timeline-lane:has(.timeline-event:hover)"));
    assert!(css.contains(".timeline-lane:focus-within"));
    assert!(css.contains(".timeline-event:hover .event-feature-preview"));
    assert!(css.contains(".timeline-event:focus .event-feature-preview"));
    assert!(css.contains("background-color: var(--surface-raised)"));
    assert!(css.contains(".event-feature-gap"));
    assert!(css.contains(".entity-event-recency"));
    assert!(css.contains(".appearance-list"));
    assert!(css.contains(".lore-period-loading"));
    assert!(css.contains("@keyframes lore-skeleton-shimmer"));
    assert!(css.contains(".lore-skeleton-line"));
}

#[tokio::test]
async fn entity_can_be_edited_and_given_a_searchable_alias() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let setup = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("setup response");
    let setup_json = json_response(setup).await;
    let work_id = setup_json["work"]["id"].as_str().expect("work id");
    let created = app
        .clone()
        .oneshot(
            Request::post(format!("/api/works/{work_id}/entities"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "entity_type": "region",
                        "official_english_name": "Test Region",
                        "official_original_name": "测试区域"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create response");
    let created_json = json_response(created).await;
    let entity_id = created_json["id"].as_str().expect("entity id");

    let edited = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/entities/{entity_id}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "official_english_name": "Edited Region",
                        "english_description": "Updated during UAT."
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("edit response");
    assert_eq!(edited.status(), StatusCode::OK);
    assert_eq!(
        json_response(edited).await["official_english_name"],
        "Edited Region"
    );

    let alias = app
        .clone()
        .oneshot(
            Request::post(format!("/api/entities/{entity_id}/aliases"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "value": "UAT Area",
                        "language": "en",
                        "kind": "alternative",
                        "label": "Testing alias"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("alias response");
    assert_eq!(alias.status(), StatusCode::CREATED);

    let searched = app
        .oneshot(
            Request::get(format!("/api/works/{work_id}/entities?query=UAT%20Area"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("search response");
    assert_eq!(searched.status(), StatusCode::OK);
    assert_eq!(json_response(searched).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn browser_shell_and_health_endpoint_are_served_safely() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");

    let health = app
        .clone()
        .oneshot(
            Request::get("/api/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("health response");
    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(json_response(health).await["status"], "ok");

    let page = app
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("page response");
    assert_eq!(page.status(), StatusCode::OK);
    assert_eq!(page.headers()["content-type"], "text/html; charset=utf-8");
    assert!(page.headers().contains_key("content-security-policy"));
    let bytes = page
        .into_body()
        .collect()
        .await
        .expect("HTML body")
        .to_bytes();
    let html = String::from_utf8(bytes.to_vec()).expect("UTF-8 HTML");
    assert!(html.contains("<h1"));
    assert!(html.contains("Arknights: Endfield"));
    assert!(html.contains("id=\"game-switcher\""));
    assert!(html.contains("aria-haspopup=\"listbox\""));
    assert!(html.contains("id=\"game-switcher-menu\""));
    assert!(html.contains("role=\"listbox\""));
    assert!(html.contains("id=\"open-search\""));
    assert!(html.contains("Search current game records"));
    assert!(html.contains("id=\"rarity-filter\""));
    assert!(html.contains("id=\"entity-facet-filters\""));
    assert!(html.contains("id=\"entity-sort\""));
    assert!(html.contains("Rarity: high to low"));
    assert!(html.contains("id=\"timeline-view\""));
    assert!(html.contains("Event timeline"));
    assert!(html.contains("data-view=\"timeline\""));
    assert!(html.contains("id=\"lore-view\""));
    assert!(html.contains("data-view=\"lore\""));
    assert!(html.contains("id=\"lore-subject-type-filter\""));
    assert!(html.contains("Show events involving"));
    assert!(html.contains("id=\"lore-subject-search\""));
    assert!(html.contains("Search subjects"));
    assert!(html.contains("placeholder=\"Type a name or type\""));
    assert!(html.contains("id=\"lore-review\""));
    assert!(html.contains("data-event-status=\"upcoming\""));
    assert!(!html.contains("<select id=\"game-switcher\""));
}

#[tokio::test]
async fn lore_client_asset_is_served_as_a_module() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let response = app
        .oneshot(
            Request::get("/assets/lore.js")
                .body(Body::empty())
                .expect("lore asset request"),
        )
        .await
        .expect("lore asset response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/javascript; charset=utf-8"
    );
    let body = response
        .into_body()
        .collect()
        .await
        .expect("lore asset body")
        .to_bytes();
    let script = String::from_utf8(body.to_vec()).expect("UTF-8 lore asset");
    assert!(script.contains("renderLoreMap"));
    assert!(script.contains("renderLoreReview"));
    assert!(script.contains("lore-character-avatar"));
    assert!(script.contains("presentation?.thumbnail_url"));
    assert!(script.contains("characterGroupItems"));
    assert!(script.contains("lore-character-overflow"));
    assert!(script.contains("lore-character-count"));
    assert!(script.contains("onLoadMore"));
    assert!(script.contains("Load more lore events"));
    assert!(script.contains("renderPeriodLoading"));
    assert!(script.contains("lore-period-loading"));
    assert!(script.contains("Loading release data"));
}

#[tokio::test]
async fn known_name_editor_is_delivered_as_an_accessible_collapsed_disclosure() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");

    let script = app
        .oneshot(
            Request::get("/assets/ui.js")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("UI script response");
    assert_eq!(script.status(), StatusCode::OK);
    let bytes = script
        .into_body()
        .collect()
        .await
        .expect("UI script body")
        .to_bytes();
    let javascript = String::from_utf8(bytes.to_vec()).expect("UTF-8 JavaScript");

    assert!(javascript.contains("alias-editor-panel"));
    assert!(javascript.contains("toggle.setAttribute(\"aria-expanded\", \"false\")"));
    assert!(javascript.contains("panel.hidden = true"));
    assert!(javascript.contains("requestAnimationFrame(() => value.focus())"));
    assert!(javascript.contains("event.key === \"Escape\""));
}

#[tokio::test]
async fn library_lists_switchable_games_with_isolated_entities_and_module_themes() {
    let app = router_with_store(Store::open_in_memory().expect("in-memory database"))
        .expect("UAT router");
    let endfield_setup = app
        .clone()
        .oneshot(
            Request::post("/api/uat/endfield")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("Endfield setup response");
    let endfield_json = json_response(endfield_setup).await;
    let endfield_id = endfield_json["work"]["id"].as_str().expect("Endfield id");

    let generic = app
        .clone()
        .oneshot(
            Request::post("/api/works")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "kind": "game",
                        "module_id": "generic",
                        "display_name": "Second Game"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create generic game response");
    assert_eq!(generic.status(), StatusCode::CREATED);
    let generic_json = json_response(generic).await;
    let generic_id = generic_json["id"].as_str().expect("generic game id");

    let works = app
        .clone()
        .oneshot(
            Request::get("/api/works?kind=game")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("work list response");
    assert_eq!(works.status(), StatusCode::OK);
    let works_json = json_response(works).await;
    assert_eq!(works_json.as_array().expect("work list").len(), 2);

    let modules = app
        .clone()
        .oneshot(
            Request::get("/api/modules?kind=game")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("module list response");
    assert_eq!(modules.status(), StatusCode::OK);
    let modules_json = json_response(modules).await;
    let modules = modules_json.as_array().expect("module list");
    assert_eq!(modules.len(), 5);
    assert!(modules.iter().any(|module| {
        module["id"] == "heroes-of-might-and-magic"
            && module["display_name"] == "Heroes of Might and Magic"
    }));

    let generic_workspace = app
        .clone()
        .oneshot(
            Request::get(format!("/api/works/{generic_id}/workspace"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("generic workspace response");
    assert_eq!(generic_workspace.status(), StatusCode::OK);
    let generic_workspace_json = json_response(generic_workspace).await;
    assert_eq!(
        generic_workspace_json["entity_types"]
            .as_array()
            .expect("generic entity types")
            .len(),
        2
    );
    assert_ne!(
        endfield_json["theme"]["accent"],
        generic_workspace_json["theme"]["accent"]
    );

    for (work_id, entity_type, english_name) in [
        (endfield_id, "operator", "Endfield Only"),
        (generic_id, "npc", "Second Game Only"),
    ] {
        let created = app
            .clone()
            .oneshot(
                Request::post(format!("/api/works/{work_id}/entities"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "entity_type": entity_type,
                            "official_english_name": english_name,
                            "official_original_name": english_name
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("entity response");
        assert_eq!(created.status(), StatusCode::CREATED);
    }

    let generic_entities = app
        .oneshot(
            Request::get(format!("/api/works/{generic_id}/entities"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("generic entity list response");
    let generic_entities_json = json_response(generic_entities).await;
    assert_eq!(generic_entities_json.as_array().unwrap().len(), 1);
    assert_eq!(
        generic_entities_json[0]["official_english_name"],
        "Second Game Only"
    );
}

#[test]
fn uat_server_only_accepts_loopback_bind_addresses() {
    assert!(validate_bind_address("127.0.0.1:5178".parse().unwrap()).is_ok());
    assert!(validate_bind_address("[::1]:5178".parse().unwrap()).is_ok());
    assert!(validate_bind_address("0.0.0.0:5178".parse().unwrap()).is_err());
}
