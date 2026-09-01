//! Localhost UAT composition root for Perwiga.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, MutexGuard},
};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use perwiga_core::{
    lore::{
        LoreCandidateRecord, LoreClaim, LoreClaimEvidence, LoreEvent, LoreEventDetail,
        LoreEventRelation, LoreGraph, LoreInvolvement, LorePeriod, LoreSource, LoreSubject,
    },
    model::{
        AliasInput, CalendarEvent, EntityAlias, EntityAppearance, EntityInput, EntityPatch,
        LibraryWork, WikiEntity,
    },
    service::Application,
    CalendarEventPresentation, EntityEventRecencyPresentation, EntityFacetDefinition,
    EntityPresentation, EntityTypeDefinition, LibraryModule, LoreSchemaDefinition, ModuleRegistry,
    PerwigaError, Store, ThemeDefinition, WorkKind,
};
use serde::{Deserialize, Serialize};

pub const ENDFIELD_MODULE_ID: &str = "arknights-endfield";
pub const GENSHIN_MODULE_ID: &str = "genshin-impact";

pub fn validate_bind_address(address: SocketAddr) -> Result<(), String> {
    if address.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!(
            "UAT server must bind to a loopback address, not {}",
            address.ip()
        ))
    }
}

#[derive(Clone)]
struct WebState {
    application: Arc<Mutex<Application>>,
}

impl WebState {
    fn lock(&self) -> Result<MutexGuard<'_, Application>, WebError> {
        self.application.lock().map_err(|_| WebError::Internal)
    }
}

#[derive(Serialize)]
struct SetupResponse {
    work: LibraryWork,
    entity_types: Vec<EntityTypeResponse>,
    entity_facets: &'static [EntityFacetDefinition],
    theme: ThemeDefinition,
    lore_schema: Option<&'static LoreSchemaDefinition>,
}

#[derive(Serialize)]
struct ModuleResponse {
    kind: WorkKind,
    id: &'static str,
    display_name: &'static str,
    theme: ThemeDefinition,
}

#[derive(Serialize)]
struct EntityTypeResponse {
    key: &'static str,
    display_name: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
struct EntityListItemResponse {
    #[serde(flatten)]
    entity: WikiEntity,
    #[serde(skip_serializing_if = "Option::is_none")]
    catalog_label: Option<String>,
    presentation: Option<EntityPresentation>,
    event_recency: Option<EntityEventRecencyPresentation>,
}

#[derive(Serialize)]
struct EntityDetailResponse {
    entity: WikiEntity,
    aliases: Vec<EntityAlias>,
    appearances: Vec<EntityAppearance>,
    event_recency: Option<EntityEventRecencyPresentation>,
}

#[derive(Serialize)]
struct CalendarEventListItemResponse {
    #[serde(flatten)]
    event: CalendarEvent,
    presentation: Option<CalendarEventPresentation>,
}

#[derive(Serialize)]
struct LoreSubjectResponse {
    #[serde(flatten)]
    subject: LoreSubject,
    entity_type: Option<String>,
    presentation: Option<EntityPresentation>,
}

#[derive(Serialize)]
struct LoreGraphResponse {
    periods: Vec<LorePeriod>,
    events: Vec<LoreEvent>,
    relations: Vec<LoreEventRelation>,
    subjects: Vec<LoreSubjectResponse>,
    involvements: Vec<LoreInvolvement>,
    next_cursor: Option<String>,
}

#[derive(Serialize)]
struct LoreEventDetailResponse {
    event: LoreEvent,
    claims: Vec<LoreClaim>,
    involvements: Vec<LoreInvolvement>,
    subjects: Vec<LoreSubjectResponse>,
    evidence: Vec<LoreClaimEvidence>,
    sources: Vec<LoreSource>,
}

#[derive(Default, Deserialize)]
struct LoreGraphQuery {
    subject_id: Option<String>,
    cursor: Option<String>,
    limit: Option<usize>,
}

#[derive(Default, Deserialize)]
struct LoreCandidateQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
struct LoreDecisionRequest {
    decision: String,
    normalized_payload_json: Option<String>,
    merge_target_id: Option<String>,
    notes: Option<String>,
}

impl From<&EntityTypeDefinition> for EntityTypeResponse {
    fn from(definition: &EntityTypeDefinition) -> Self {
        Self {
            key: definition.key,
            display_name: definition.display_name,
            description: definition.description,
        }
    }
}

#[derive(Deserialize)]
struct EntityRequest {
    entity_type: String,
    official_english_name: String,
    official_original_name: String,
    official_vietnamese_name: Option<String>,
    automatic_vietnamese_translation: Option<String>,
    english_description: Option<String>,
    other_information: Option<String>,
}

impl From<EntityRequest> for EntityInput {
    fn from(request: EntityRequest) -> Self {
        Self {
            entity_type: request.entity_type,
            official_english_name: request.official_english_name,
            official_original_name: request.official_original_name,
            official_vietnamese_name: request.official_vietnamese_name,
            automatic_vietnamese_translation: request.automatic_vietnamese_translation,
            english_description: request.english_description,
            other_information: request.other_information,
        }
    }
}

#[derive(Deserialize)]
struct EntityPatchRequest {
    official_english_name: Option<String>,
    official_original_name: Option<String>,
    official_vietnamese_name: Option<String>,
    automatic_vietnamese_translation: Option<String>,
    english_description: Option<String>,
    other_information: Option<String>,
}

impl From<EntityPatchRequest> for EntityPatch {
    fn from(request: EntityPatchRequest) -> Self {
        Self {
            official_english_name: request.official_english_name,
            official_original_name: request.official_original_name,
            official_vietnamese_name: request.official_vietnamese_name,
            automatic_vietnamese_translation: request.automatic_vietnamese_translation,
            english_description: request.english_description,
            other_information: request.other_information,
        }
    }
}

#[derive(Deserialize)]
struct AliasRequest {
    value: String,
    language: Option<String>,
    kind: String,
    label: Option<String>,
    notes: Option<String>,
}

impl From<AliasRequest> for AliasInput {
    fn from(request: AliasRequest) -> Self {
        Self {
            value: request.value,
            language: request.language,
            kind: request.kind,
            label: request.label,
            notes: request.notes,
        }
    }
}

#[derive(Default, Deserialize)]
struct EntityListQuery {
    query: Option<String>,
    entity_type: Option<String>,
}

#[derive(Default, Deserialize)]
struct KindQuery {
    kind: Option<WorkKind>,
}

#[derive(Deserialize)]
struct WorkRequest {
    kind: WorkKind,
    module_id: String,
    display_name: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

enum WebError {
    Core(PerwigaError),
    Internal,
}

impl From<PerwigaError> for WebError {
    fn from(error: PerwigaError) -> Self {
        Self::Core(error)
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Core(PerwigaError::Validation(message)) => (StatusCode::BAD_REQUEST, message),
            Self::Core(PerwigaError::NotFound(message)) => {
                (StatusCode::NOT_FOUND, format!("{message} was not found"))
            }
            Self::Core(PerwigaError::Conflict(message)) => (StatusCode::CONFLICT, message),
            Self::Core(PerwigaError::Unsupported(message)) => {
                (StatusCode::UNPROCESSABLE_ENTITY, message)
            }
            Self::Core(_) | Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred".to_string(),
            ),
        };
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

pub fn router_with_store(store: Store) -> perwiga_core::Result<Router> {
    let mut registry = ModuleRegistry::default();
    arknights_endfield::register(&mut registry)?;
    generic_game::register(&mut registry)?;
    genshin_impact::register(&mut registry)?;
    heroes_of_might_and_magic::register(&mut registry)?;
    let state = WebState {
        application: Arc::new(Mutex::new(Application::new(store, registry))),
    };

    Ok(Router::new()
        .route("/", get(index))
        .route("/assets/styles.css", get(styles))
        .route("/assets/app.js", get(script))
        .route("/assets/api.js", get(api_script))
        .route("/assets/ui.js", get(ui_script))
        .route("/assets/lore.js", get(lore_script))
        .route(
            "/assets/placeholders/{filename}",
            get(entity_type_placeholder),
        )
        .route(
            "/assets/modules/arknights-endfield/operators/{filename}",
            get(endfield_operator_thumbnail),
        )
        .route(
            "/assets/modules/arknights-endfield/weapons/{filename}",
            get(endfield_weapon_thumbnail),
        )
        .route(
            "/assets/modules/arknights-endfield/items/{filename}",
            get(endfield_item_thumbnail),
        )
        .route(
            "/assets/modules/genshin-impact/characters/{filename}",
            get(genshin_character_thumbnail),
        )
        .route(
            "/assets/modules/genshin-impact/weapons/{filename}",
            get(genshin_weapon_thumbnail),
        )
        .route(
            "/assets/modules/genshin-impact/skins/{filename}",
            get(genshin_skin_thumbnail),
        )
        .route(
            "/assets/modules/genshin-impact/artifacts/{filename}",
            get(genshin_artifact_thumbnail),
        )
        .route(
            "/assets/modules/genshin-impact/regions/{filename}",
            get(genshin_region_icon),
        )
        .route("/api/health", get(health))
        .route("/api/uat/endfield", post(setup_endfield))
        .route("/api/uat/genshin", post(setup_genshin))
        .route("/api/modules", get(list_modules))
        .route("/api/works", get(list_works).post(create_work))
        .route("/api/works/{work_id}/workspace", get(get_workspace))
        .route(
            "/api/works/{work_id}/calendar-events",
            get(list_calendar_events),
        )
        .route("/api/works/{work_id}/lore-graph", get(list_lore_graph))
        .route(
            "/api/works/{work_id}/lore-subjects",
            get(list_lore_subjects),
        )
        .route(
            "/api/works/{work_id}/lore-candidates",
            get(list_lore_candidates),
        )
        .route("/api/lore-events/{event_id}", get(get_lore_event_detail))
        .route(
            "/api/lore-candidates/{candidate_id}/decisions",
            post(review_lore_candidate),
        )
        .route(
            "/api/works/{work_id}/entities",
            get(list_entities).post(create_entity),
        )
        .route(
            "/api/entities/{entity_id}",
            get(get_entity).patch(update_entity),
        )
        .route("/api/entities/{entity_id}/aliases", post(add_entity_alias))
        .with_state(state))
}

async fn index() -> impl IntoResponse {
    static_asset(
        "text/html; charset=utf-8",
        include_str!("../static/index.html"),
    )
}

async fn styles() -> impl IntoResponse {
    static_asset(
        "text/css; charset=utf-8",
        include_str!("../static/styles.css"),
    )
}

async fn script() -> impl IntoResponse {
    static_asset(
        "text/javascript; charset=utf-8",
        include_str!("../static/app.js"),
    )
}

async fn api_script() -> impl IntoResponse {
    static_asset(
        "text/javascript; charset=utf-8",
        include_str!("../static/api.js"),
    )
}

async fn ui_script() -> impl IntoResponse {
    static_asset(
        "text/javascript; charset=utf-8",
        include_str!("../static/ui.js"),
    )
}

async fn lore_script() -> impl IntoResponse {
    static_asset(
        "text/javascript; charset=utf-8",
        include_str!("../static/lore.js"),
    )
}

async fn entity_type_placeholder(Path(filename): Path<String>) -> Result<Response, WebError> {
    let body = match filename.as_str() {
        "character.svg" => include_str!("../static/placeholders/character.svg"),
        "place.svg" => include_str!("../static/placeholders/place.svg"),
        "concept.svg" => include_str!("../static/placeholders/concept.svg"),
        "weapon.svg" => include_str!("../static/placeholders/weapon.svg"),
        "enemy.svg" => include_str!("../static/placeholders/enemy.svg"),
        "mission.svg" => include_str!("../static/placeholders/mission.svg"),
        "event.svg" => include_str!("../static/placeholders/event.svg"),
        "item.svg" => include_str!("../static/placeholders/item.svg"),
        "faction.svg" => include_str!("../static/placeholders/faction.svg"),
        "generic.svg" => include_str!("../static/placeholders/generic.svg"),
        _ => return Err(PerwigaError::NotFound(format!("entity placeholder {filename}")).into()),
    };
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/svg+xml"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn endfield_operator_thumbnail(Path(filename): Path<String>) -> Result<Response, WebError> {
    let source_key = filename
        .strip_suffix(".webp")
        .ok_or_else(|| PerwigaError::NotFound(format!("operator thumbnail {filename}")))?;
    let bytes = arknights_endfield::operator_thumbnail(source_key)
        .ok_or_else(|| PerwigaError::NotFound(format!("operator thumbnail {filename}")))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn endfield_weapon_thumbnail(Path(filename): Path<String>) -> Result<Response, WebError> {
    let source_key = filename
        .strip_suffix(".png")
        .ok_or_else(|| PerwigaError::NotFound(format!("weapon thumbnail {filename}")))?;
    let bytes = arknights_endfield::weapon_thumbnail(source_key)
        .ok_or_else(|| PerwigaError::NotFound(format!("weapon thumbnail {filename}")))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn endfield_item_thumbnail(Path(filename): Path<String>) -> Result<Response, WebError> {
    let icon_id = filename
        .strip_suffix(".webp")
        .ok_or_else(|| PerwigaError::NotFound(format!("item thumbnail {filename}")))?;
    let bytes = arknights_endfield::item_thumbnail(icon_id)
        .ok_or_else(|| PerwigaError::NotFound(format!("item thumbnail {filename}")))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn genshin_character_thumbnail(Path(filename): Path<String>) -> Result<Response, WebError> {
    let source_key = filename
        .strip_suffix(".png")
        .ok_or_else(|| PerwigaError::NotFound(format!("character thumbnail {filename}")))?;
    let bytes = genshin_impact::character_thumbnail(source_key)
        .ok_or_else(|| PerwigaError::NotFound(format!("character thumbnail {filename}")))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn genshin_weapon_thumbnail(Path(filename): Path<String>) -> Result<Response, WebError> {
    let source_key = filename
        .strip_suffix(".png")
        .ok_or_else(|| PerwigaError::NotFound(format!("weapon thumbnail {filename}")))?;
    let bytes = genshin_impact::weapon_thumbnail(source_key)
        .ok_or_else(|| PerwigaError::NotFound(format!("weapon thumbnail {filename}")))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn genshin_skin_thumbnail(Path(filename): Path<String>) -> Result<Response, WebError> {
    let (source_key, content_type) = if let Some(source_key) = filename.strip_suffix(".png") {
        (source_key, "image/png")
    } else if let Some(source_key) = filename.strip_suffix(".gif") {
        (source_key, "image/gif")
    } else {
        return Err(PerwigaError::NotFound(format!("Skin thumbnail {filename}")).into());
    };
    let bytes = genshin_impact::skin_thumbnail(source_key)
        .ok_or_else(|| PerwigaError::NotFound(format!("Skin thumbnail {filename}")))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn genshin_artifact_thumbnail(Path(filename): Path<String>) -> Result<Response, WebError> {
    let source_key = filename
        .strip_suffix(".png")
        .ok_or_else(|| PerwigaError::NotFound(format!("Artifact thumbnail {filename}")))?;
    let bytes = genshin_impact::artifact_thumbnail(source_key)
        .ok_or_else(|| PerwigaError::NotFound(format!("Artifact thumbnail {filename}")))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

async fn genshin_region_icon(Path(filename): Path<String>) -> Result<Response, WebError> {
    let source_key = filename
        .strip_suffix(".webp")
        .ok_or_else(|| PerwigaError::NotFound(format!("region icon {filename}")))?;
    let bytes = genshin_impact::region_icon(source_key)
        .ok_or_else(|| PerwigaError::NotFound(format!("region icon {filename}")))?;
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/webp"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

fn static_asset(content_type: &'static str, body: &'static str) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
        ),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    (headers, body)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn setup_endfield(State(state): State<WebState>) -> Result<Json<SetupResponse>, WebError> {
    let mut application = state.lock()?;
    let module_name = application
        .modules()
        .find(|module| module.kind() == WorkKind::Game && module.id() == ENDFIELD_MODULE_ID)
        .map(|module| module.display_name())
        .ok_or_else(|| {
            PerwigaError::Unsupported("Endfield module is not registered".to_string())
        })?;
    let work = match application
        .store()
        .list_works()?
        .into_iter()
        .find(|work| work.kind == WorkKind::Game && work.module_id == ENDFIELD_MODULE_ID)
    {
        Some(work) => work,
        None => application.create_work(WorkKind::Game, ENDFIELD_MODULE_ID, module_name)?,
    };
    arknights_endfield::import_curated_operators(application.store_mut(), &work.id)?;
    arknights_endfield::import_curated_npcs(application.store_mut(), &work.id)?;
    arknights_endfield::import_curated_regions(application.store_mut(), &work.id)?;
    arknights_endfield::import_curated_lore(application.store_mut(), &work.id)?;
    arknights_endfield::import_curated_gear_sets_and_essences(application.store_mut(), &work.id)?;
    arknights_endfield::import_curated_event_entities(application.store_mut(), &work.id)?;
    let events = arknights_endfield::curated_calendar_events()?;
    application
        .store_mut()
        .import_calendar_events(&work.id, &events)?;
    Ok(Json(workspace_response(&application, work)?))
}

async fn setup_genshin(State(state): State<WebState>) -> Result<Json<SetupResponse>, WebError> {
    let mut application = state.lock()?;
    let module_name = application
        .modules()
        .find(|module| module.kind() == WorkKind::Game && module.id() == GENSHIN_MODULE_ID)
        .map(|module| module.display_name())
        .ok_or_else(|| {
            PerwigaError::Unsupported("Genshin Impact module is not registered".to_string())
        })?;
    let work = match application
        .store()
        .list_works()?
        .into_iter()
        .find(|work| work.kind == WorkKind::Game && work.module_id == GENSHIN_MODULE_ID)
    {
        Some(work) => work,
        None => application.create_work(WorkKind::Game, GENSHIN_MODULE_ID, module_name)?,
    };
    genshin_impact::import_curated_characters(application.store_mut(), &work.id)?;
    genshin_impact::import_curated_regions(application.store_mut(), &work.id)?;
    genshin_impact::import_curated_npcs(application.store_mut(), &work.id)?;
    genshin_impact::import_curated_weapons(application.store_mut(), &work.id)?;
    genshin_impact::import_curated_skins(application.store_mut(), &work.id)?;
    genshin_impact::import_curated_artifacts(application.store_mut(), &work.id)?;
    genshin_impact::import_curated_artifact_domains(application.store_mut(), &work.id)?;
    genshin_impact::import_curated_event_entities(application.store_mut(), &work.id)?;
    let events = genshin_impact::curated_calendar_events()?;
    application
        .store_mut()
        .import_calendar_events(&work.id, &events)?;
    Ok(Json(workspace_response(&application, work)?))
}

fn workspace_response(
    application: &Application,
    work: LibraryWork,
) -> Result<SetupResponse, WebError> {
    let module = application
        .modules()
        .find(|module| module.kind() == work.kind && module.id() == work.module_id)
        .ok_or_else(|| {
            PerwigaError::Unsupported(format!(
                "module {}:{} is not registered",
                work.kind, work.module_id
            ))
        })?;
    Ok(SetupResponse {
        work,
        entity_types: module
            .entity_types()
            .iter()
            .map(EntityTypeResponse::from)
            .collect(),
        entity_facets: module.entity_facets(),
        theme: module.theme(),
        lore_schema: module.lore_schema(),
    })
}

async fn list_modules(
    State(state): State<WebState>,
    Query(filters): Query<KindQuery>,
) -> Result<Json<Vec<ModuleResponse>>, WebError> {
    let application = state.lock()?;
    let modules = application
        .modules()
        .filter(|module| filters.kind.is_none_or(|kind| module.kind() == kind))
        .map(|module| ModuleResponse {
            kind: module.kind(),
            id: module.id(),
            display_name: module.display_name(),
            theme: module.theme(),
        })
        .collect();
    Ok(Json(modules))
}

async fn list_works(
    State(state): State<WebState>,
    Query(filters): Query<KindQuery>,
) -> Result<Json<Vec<LibraryWork>>, WebError> {
    let application = state.lock()?;
    let works = application
        .store()
        .list_works()?
        .into_iter()
        .filter(|work| filters.kind.is_none_or(|kind| work.kind == kind))
        .collect();
    Ok(Json(works))
}

async fn create_work(
    State(state): State<WebState>,
    Json(request): Json<WorkRequest>,
) -> Result<(StatusCode, Json<LibraryWork>), WebError> {
    let application = state.lock()?;
    let work = application.create_work(request.kind, &request.module_id, &request.display_name)?;
    Ok((StatusCode::CREATED, Json(work)))
}

async fn get_workspace(
    State(state): State<WebState>,
    Path(work_id): Path<String>,
) -> Result<Json<SetupResponse>, WebError> {
    let application = state.lock()?;
    let work = application
        .store()
        .get_work(&work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    Ok(Json(workspace_response(&application, work)?))
}

async fn list_entities(
    State(state): State<WebState>,
    Path(work_id): Path<String>,
    Query(filters): Query<EntityListQuery>,
) -> Result<Json<Vec<EntityListItemResponse>>, WebError> {
    let application = state.lock()?;
    let work = application
        .store()
        .get_work(&work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    let module = application
        .modules()
        .find(|module| module.kind() == work.kind && module.id() == work.module_id)
        .ok_or_else(|| {
            PerwigaError::Unsupported(format!(
                "module {}:{} is not registered",
                work.kind, work.module_id
            ))
        })?;
    let mut entities = match filters.query.as_deref().map(str::trim) {
        Some(query) if !query.is_empty() => application.store().search_entities(&work_id, query)?,
        _ => application
            .store()
            .list_entities(&work_id, filters.entity_type.as_deref())?,
    };
    if let Some(entity_type) = filters.entity_type {
        entities.retain(|entity| entity.entity_type == entity_type);
    }
    Ok(Json(
        entities
            .into_iter()
            .map(|entity| EntityListItemResponse {
                catalog_label: module.entity_catalog_label(&entity),
                presentation: module.entity_presentation(&entity),
                event_recency: module.entity_event_recency(&entity),
                entity,
            })
            .collect(),
    ))
}

async fn list_calendar_events(
    State(state): State<WebState>,
    Path(work_id): Path<String>,
) -> Result<Json<Vec<CalendarEventListItemResponse>>, WebError> {
    let application = state.lock()?;
    let work = application
        .store()
        .get_work(&work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    let module = application
        .modules()
        .find(|module| module.kind() == work.kind && module.id() == work.module_id)
        .ok_or_else(|| {
            PerwigaError::Unsupported(format!(
                "module {}:{} is not registered",
                work.kind, work.module_id
            ))
        })?;
    Ok(Json(
        application
            .store()
            .list_calendar_events_for_work(&work_id)?
            .into_iter()
            .map(|event| CalendarEventListItemResponse {
                presentation: module.calendar_event_presentation(&event),
                event,
            })
            .collect(),
    ))
}

fn lore_work(application: &Application, work_id: &str) -> Result<LibraryWork, WebError> {
    let work = application
        .store()
        .get_work(work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {work_id}")))?;
    if !application.supports_capability(work_id, perwiga_core::Capability::LoreTimeline)? {
        return Err(PerwigaError::Unsupported(format!(
            "module {}:{} does not support lore timelines",
            work.kind, work.module_id
        ))
        .into());
    }
    Ok(work)
}

fn lore_subject_response(
    application: &Application,
    module: &dyn LibraryModule,
    subject: LoreSubject,
) -> Result<LoreSubjectResponse, WebError> {
    let entity = subject
        .wiki_entity_id
        .as_deref()
        .map(|entity_id| application.store().get_entity(entity_id))
        .transpose()?
        .flatten();
    Ok(LoreSubjectResponse {
        entity_type: entity.as_ref().map(|entity| entity.entity_type.clone()),
        presentation: entity
            .as_ref()
            .and_then(|entity| module.entity_presentation(entity)),
        subject,
    })
}

async fn list_lore_graph(
    State(state): State<WebState>,
    Path(work_id): Path<String>,
    Query(filters): Query<LoreGraphQuery>,
) -> Result<Json<LoreGraphResponse>, WebError> {
    let application = state.lock()?;
    let work = lore_work(&application, &work_id)?;
    let module = application
        .modules()
        .find(|module| module.kind() == work.kind && module.id() == work.module_id)
        .ok_or_else(|| {
            PerwigaError::Unsupported(format!(
                "module {}:{} is not registered",
                work.kind, work.module_id
            ))
        })?;
    let graph = application.store().list_lore_graph(
        &work_id,
        filters.subject_id.as_deref(),
        filters.cursor.as_deref(),
        filters.limit.unwrap_or(100),
    )?;
    let LoreGraph {
        periods,
        events,
        relations,
        subjects,
        involvements,
        next_cursor,
    } = graph;
    Ok(Json(LoreGraphResponse {
        periods,
        events,
        relations,
        subjects: subjects
            .into_iter()
            .map(|subject| lore_subject_response(&application, module, subject))
            .collect::<Result<Vec<_>, _>>()?,
        involvements,
        next_cursor,
    }))
}

async fn list_lore_subjects(
    State(state): State<WebState>,
    Path(work_id): Path<String>,
) -> Result<Json<Vec<LoreSubjectResponse>>, WebError> {
    let application = state.lock()?;
    let work = lore_work(&application, &work_id)?;
    let module = application
        .modules()
        .find(|module| module.kind() == work.kind && module.id() == work.module_id)
        .ok_or_else(|| {
            PerwigaError::Unsupported(format!(
                "module {}:{} is not registered",
                work.kind, work.module_id
            ))
        })?;
    Ok(Json(
        application
            .store()
            .list_lore_subjects(&work_id)?
            .into_iter()
            .map(|subject| lore_subject_response(&application, module, subject))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

async fn list_lore_candidates(
    State(state): State<WebState>,
    Path(work_id): Path<String>,
    Query(filters): Query<LoreCandidateQuery>,
) -> Result<Json<Vec<LoreCandidateRecord>>, WebError> {
    let application = state.lock()?;
    lore_work(&application, &work_id)?;
    Ok(Json(application.store().list_lore_candidates(
        &work_id,
        filters.status.as_deref(),
    )?))
}

async fn get_lore_event_detail(
    State(state): State<WebState>,
    Path(event_id): Path<String>,
) -> Result<Json<LoreEventDetailResponse>, WebError> {
    let application = state.lock()?;
    let detail = application
        .store()
        .get_lore_event_detail(&event_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("lore event {event_id}")))?;
    let work = lore_work(&application, &detail.event.work_id)?;
    let module = application
        .modules()
        .find(|module| module.kind() == work.kind && module.id() == work.module_id)
        .ok_or_else(|| {
            PerwigaError::Unsupported(format!(
                "module {}:{} is not registered",
                work.kind, work.module_id
            ))
        })?;
    let LoreEventDetail {
        event,
        claims,
        involvements,
        subjects,
        evidence,
        sources,
    } = detail;
    Ok(Json(LoreEventDetailResponse {
        event,
        claims,
        involvements,
        subjects: subjects
            .into_iter()
            .map(|subject| lore_subject_response(&application, module, subject))
            .collect::<Result<Vec<_>, _>>()?,
        evidence,
        sources,
    }))
}

async fn review_lore_candidate(
    State(state): State<WebState>,
    Path(candidate_id): Path<String>,
    Json(request): Json<LoreDecisionRequest>,
) -> Result<Json<LoreCandidateRecord>, WebError> {
    let mut application = state.lock()?;
    let candidate = application
        .store()
        .get_lore_candidate(&candidate_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("lore candidate {candidate_id}")))?;
    let batch_work_id = application
        .store()
        .get_lore_candidate_work_id(&candidate.id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("lore candidate {candidate_id}")))?;
    lore_work(&application, &batch_work_id)?;
    Ok(Json(application.store_mut().review_lore_candidate(
        &candidate_id,
        &request.decision,
        request.normalized_payload_json.as_deref(),
        request.merge_target_id.as_deref(),
        request.notes.as_deref(),
    )?))
}

async fn create_entity(
    State(state): State<WebState>,
    Path(work_id): Path<String>,
    Json(request): Json<EntityRequest>,
) -> Result<(StatusCode, Json<WikiEntity>), WebError> {
    let application = state.lock()?;
    let entity = application.create_entity(&work_id, &request.into())?;
    Ok((StatusCode::CREATED, Json(entity)))
}

async fn get_entity(
    State(state): State<WebState>,
    Path(entity_id): Path<String>,
) -> Result<Json<EntityDetailResponse>, WebError> {
    let application = state.lock()?;
    let detail = application
        .store()
        .get_entity_detail(&entity_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("entity {entity_id}")))?;
    let work = application
        .store()
        .get_work(&detail.entity.work_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("work {}", detail.entity.work_id)))?;
    let module = application
        .modules()
        .find(|module| module.kind() == work.kind && module.id() == work.module_id)
        .ok_or_else(|| {
            PerwigaError::Unsupported(format!(
                "module {}:{} is not registered",
                work.kind, work.module_id
            ))
        })?;
    let event_recency = module.entity_event_recency(&detail.entity);
    Ok(Json(EntityDetailResponse {
        entity: detail.entity,
        aliases: detail.aliases,
        appearances: detail.appearances,
        event_recency,
    }))
}

async fn update_entity(
    State(state): State<WebState>,
    Path(entity_id): Path<String>,
    Json(request): Json<EntityPatchRequest>,
) -> Result<Json<WikiEntity>, WebError> {
    let application = state.lock()?;
    Ok(Json(
        application.update_entity(&entity_id, &request.into())?,
    ))
}

async fn add_entity_alias(
    State(state): State<WebState>,
    Path(entity_id): Path<String>,
    Json(request): Json<AliasRequest>,
) -> Result<(StatusCode, Json<EntityAlias>), WebError> {
    let application = state.lock()?;
    let alias = application.add_alias(&entity_id, &request.into())?;
    Ok((StatusCode::CREATED, Json(alias)))
}
