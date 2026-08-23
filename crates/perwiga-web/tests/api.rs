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
    assert_eq!(listed_json[0]["presentation"]["accent_color"], "#55a8ff");
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
    assert_eq!(json_response(modules).await.as_array().unwrap().len(), 2);

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
