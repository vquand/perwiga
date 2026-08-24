use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use perwiga_core::Store;
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
    assert_eq!(first_json["entity_types"].as_array().unwrap().len(), 8);
    let facets = first_json["entity_facets"]
        .as_array()
        .expect("entity facet definitions");
    let weapon_type = facets
        .iter()
        .find(|facet| facet["key"] == "weapon_type")
        .expect("weapon type facet");
    assert_eq!(weapon_type["display_name"], "Weapon type");
    assert_eq!(weapon_type["entity_types"], json!(["operator", "weapon"]));
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
        vec!["character", "npc", "region"]
    );

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
    assert_eq!(albedo["presentation"]["label"], "Mondstadt");
    assert_eq!(
        albedo["presentation"]["thumbnail_url"],
        "/assets/modules/genshin-impact/characters/hoyoverse-content-104816.png"
    );

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
    assert!(javascript.contains("for (const work of state.works)"));
    assert!(javascript.contains("chooseGame(work.id)"));
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
            Request::get(format!("/api/works/{work_id}/entities?query=m%C3%A1y"))
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
    assert!(html.contains("data-event-status=\"upcoming\""));
    assert!(!html.contains("<select id=\"game-switcher\""));
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
    assert_eq!(json_response(modules).await.as_array().unwrap().len(), 3);

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
