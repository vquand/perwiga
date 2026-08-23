use perwiga_core::{
    feed::{normalize_feed_items, parse_feed},
    model::{AliasInput, EntityInput, EntityPatch, FeedItem},
    service::Application,
    ModuleRegistry, Store, WorkKind,
};

fn app() -> Application {
    let mut registry = ModuleRegistry::default();
    perwiga_game_generic::register(&mut registry).expect("generic game registration");
    perwiga_game_arknights_endfield::register(&mut registry).expect("Endfield registration");
    perwiga_novel_generic::register(&mut registry).expect("generic novel registration");
    Application::new(
        Store::open_in_memory().expect("in-memory database"),
        registry,
    )
}

#[test]
fn multilingual_entities_round_trip_and_automatic_translation_stays_separate() {
    let app = app();
    let work = app
        .create_work(WorkKind::Game, "arknights-endfield", "Arknights: Endfield")
        .expect("work");
    let entity = app
        .create_entity(
            &work.id,
            &EntityInput {
                entity_type: "region".into(),
                official_english_name: "Fixture region".into(),
                official_original_name: "测试区域".into(),
                official_vietnamese_name: None,
                automatic_vietnamese_translation: Some("Wuling - Đèo Ứng Long".into()),
                english_description: Some("A named area on Talos-II.".into()),
                other_information: Some(
                    "Synthetic Unicode fixture: Yinglung Pass / 测试区域".into(),
                ),
            },
        )
        .expect("entity");
    assert_eq!(
        app.store()
            .get_entity(&entity.id)
            .expect("read")
            .expect("entity"),
        entity
    );
    let updated = app
        .update_entity(
            &entity.id,
            &EntityPatch {
                official_vietnamese_name: Some("Vũ Lăng - Ứng Long Đạo".into()),
                ..EntityPatch::default()
            },
        )
        .expect("official translation update");
    assert_eq!(
        updated.official_vietnamese_name.as_deref(),
        Some("Vũ Lăng - Ứng Long Đạo")
    );
    assert_eq!(
        updated.automatic_vietnamese_translation.as_deref(),
        Some("Wuling - Đèo Ứng Long")
    );
}

#[test]
fn aliases_search_and_stable_references_are_owned_by_one_work() {
    let app = app();
    let first = app
        .create_work(WorkKind::Game, "arknights-endfield", "Endfield")
        .expect("first work");
    let second = app
        .create_work(WorkKind::Game, "generic", "Other game")
        .expect("second work");
    let entity = app
        .create_entity(
            &first.id,
            &EntityInput {
                entity_type: "npc".into(),
                official_english_name: "Perlica".into(),
                official_original_name: "Perlica".into(),
                official_vietnamese_name: None,
                automatic_vietnamese_translation: None,
                english_description: None,
                other_information: None,
            },
        )
        .expect("entity");
    app.add_alias(
        &entity.id,
        &AliasInput {
            value: "Perlica-san".into(),
            language: Some("en".into()),
            kind: "nickname".into(),
            label: None,
            notes: None,
        },
    )
    .expect("alias");
    assert_eq!(
        app.store()
            .search_entities(&first.id, "san")
            .expect("search")
            .len(),
        1
    );
    assert!(app
        .store()
        .list_entities(&second.id, None)
        .expect("other work list")
        .is_empty());
    let note = app
        .store()
        .create_note(&first.id, "References", "#[[Perlica]]")
        .expect("note");
    app.store()
        .add_entity_reference("note", &note.id, &entity.id, "Perlica")
        .expect("same-work reference");
    let other_note = app
        .store()
        .create_note(&second.id, "Cross-work", "#[[Perlica]]")
        .expect("other note");
    let error = app
        .store()
        .add_entity_reference("note", &other_note.id, &entity.id, "Perlica")
        .expect_err("cross-work link denied");
    assert!(error.to_string().contains("ownership"));
}

#[test]
fn checklist_toggle_and_folder_link_preserve_local_file_references() {
    let app = app();
    let work = app
        .create_work(WorkKind::Game, "arknights-endfield", "Endfield")
        .expect("work");
    let checklist = app
        .store()
        .create_checklist(&work.id, "Explore Wuling")
        .expect("checklist");
    let item = app
        .store()
        .add_checklist_item(&checklist.id, "Visit Yinglung Pass")
        .expect("item");
    assert!(!item.is_complete);
    assert!(
        app.store()
            .toggle_checklist_item(&item.id)
            .expect("toggle")
            .is_complete
    );
    let temp_folder = tempfile::tempdir().expect("temporary image folder");
    std::fs::write(temp_folder.path().join("wuling.png"), b"not a real image")
        .expect("image fixture");
    std::fs::write(temp_folder.path().join("notes.txt"), b"not an image").expect("text fixture");
    let path = temp_folder.path().to_str().expect("UTF-8 temp path");
    let folder = app
        .store()
        .link_folder(&work.id, path, Some("Screenshots"))
        .expect("folder link");
    assert_eq!(folder.path, path);
    assert_eq!(
        app.store()
            .list_folder_images(&folder.id)
            .expect("browse images")
            .len(),
        1
    );
}

#[test]
fn checklist_and_feed_read_state_survive_reopening_the_same_database() {
    let temp = tempfile::tempdir().expect("temporary database directory");
    let path = temp.path().join("persistent.sqlite");
    let mut registry = ModuleRegistry::default();
    perwiga_game_generic::register(&mut registry).expect("generic game registration");
    perwiga_game_arknights_endfield::register(&mut registry).expect("Endfield registration");
    perwiga_novel_generic::register(&mut registry).expect("generic novel registration");
    let mut app = Application::new(Store::open(&path).expect("open database"), registry);
    let work = app
        .create_work(WorkKind::Novel, "generic", "A feed-backed novel")
        .expect("work");
    let checklist = app
        .store()
        .create_checklist(&work.id, "Persistence")
        .expect("checklist");
    let checklist_item = app
        .store()
        .add_checklist_item(&checklist.id, "Keep this complete")
        .expect("checklist item");
    app.store()
        .toggle_checklist_item(&checklist_item.id)
        .expect("toggle");
    let source = app
        .create_feed_source(&work.id, "https://example.test/persist.xml", "fixture")
        .expect("feed source");
    let parsed = parse_feed(r#"<rss><channel><item><guid>persist</guid><title>Persisted update</title></item></channel></rss>"#).expect("RSS");
    let normalized =
        normalize_feed_items(&parsed, "2026-08-22T00:00:00Z", "fixture").expect("normalize");
    let item = app
        .store_mut()
        .upsert_feed_items(&source.id, &normalized)
        .expect("feed item")
        .remove(0);
    app.store()
        .set_feed_item_read(&item.id, true)
        .expect("mark read");
    drop(app);

    let mut registry = ModuleRegistry::default();
    perwiga_game_generic::register(&mut registry).expect("generic game registration");
    perwiga_game_arknights_endfield::register(&mut registry).expect("Endfield registration");
    perwiga_novel_generic::register(&mut registry).expect("generic novel registration");
    let reopened = Application::new(Store::open(&path).expect("reopen database"), registry);
    assert!(
        reopened
            .store()
            .list_checklist_items(&checklist.id)
            .expect("checklist items")
            .first()
            .expect("saved item")
            .is_complete
    );
    assert!(
        reopened
            .store()
            .list_feed_items(&source.id)
            .expect("feed items")
            .first()
            .expect("saved feed item")
            .is_read
    );
}

#[test]
fn feed_refresh_is_idempotent_preserves_read_state_and_stays_out_of_calendar() {
    let mut app = app();
    let work = app
        .create_work(WorkKind::Novel, "generic", "A feed-backed novel")
        .expect("work");
    let source = app
        .create_feed_source(&work.id, "https://example.test/endfield.xml", "fixture")
        .expect("source");
    let parsed = parse_feed(r#"<rss><channel><item><guid>one</guid><title>Update One</title><link>https://example.test/one</link></item></channel></rss>"#).expect("RSS");
    let first =
        normalize_feed_items(&parsed, "2026-08-22T00:00:00Z", "fixture").expect("normalize");
    let inserted = app
        .store_mut()
        .upsert_feed_items(&source.id, &first)
        .expect("insert");
    let read = app
        .store()
        .set_feed_item_read(&inserted[0].id, true)
        .expect("read state");
    assert!(read.is_read);
    let second =
        normalize_feed_items(&parsed, "2026-08-23T00:00:00Z", "fixture").expect("normalize repeat");
    let refreshed = app
        .store_mut()
        .upsert_feed_items(&source.id, &second)
        .expect("refresh");
    assert_eq!(refreshed.len(), 1);
    assert!(refreshed[0].is_read);
    app.store()
        .record_feed_refresh(&source.id, false, Some("temporary failure"))
        .expect("failure status");
    assert_eq!(
        app.store()
            .list_feed_items(&source.id)
            .expect("items")
            .len(),
        1
    );
    app.store()
        .create_calendar_event(
            &work.id,
            "Manual event",
            "2026-09-01",
            None,
            true,
            None,
            "manual",
        )
        .expect("event");
    assert_eq!(
        app.store().list_calendar_events().expect("calendar").len(),
        1
    );
    let items: Vec<FeedItem> = app.store().list_feed_items(&source.id).expect("feed items");
    assert_eq!(items[0].title, "Update One");
}

#[test]
fn boundary_validation_rejects_unsafe_remote_sources_and_invalid_database_state_is_detectable() {
    let app = app();
    let work = app
        .create_work(WorkKind::Game, "arknights-endfield", "Endfield")
        .expect("work");
    assert!(app
        .create_feed_source(&work.id, "https://example.test/endfield.xml", "unconfirmed")
        .is_err());
    assert!(app
        .store()
        .create_feed_source(&work.id, "file:///etc/passwd", "bad")
        .is_err());
    let note = app
        .store()
        .create_note(&work.id, "Images", "safe text")
        .expect("note");
    assert!(app
        .store()
        .attach_to_note(&note.id, "remote-url", "javascript:alert(1)")
        .is_err());
    assert!(app
        .store()
        .create_calendar_event(
            &work.id,
            "Invalid event",
            "not-a-timestamp",
            None,
            false,
            None,
            "manual",
        )
        .is_err());
    assert_eq!(app.store().integrity_check().expect("integrity"), "ok");
    assert_eq!(
        app.store().foreign_key_violations().expect("foreign keys"),
        0
    );
}
