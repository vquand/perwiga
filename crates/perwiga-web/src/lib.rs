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
    model::{
        AliasInput, EntityAlias, EntityInput, EntityPatch, LibraryWork, WikiEntity,
        WikiEntityDetail,
    },
    service::Application,
    EntityPresentation, EntityTypeDefinition, ModuleRegistry, PerwigaError, Store, ThemeDefinition,
    WorkKind,
};
use serde::{Deserialize, Serialize};

pub const ENDFIELD_MODULE_ID: &str = "arknights-endfield";

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
    theme: ThemeDefinition,
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
    presentation: Option<EntityPresentation>,
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
    let state = WebState {
        application: Arc::new(Mutex::new(Application::new(store, registry))),
    };

    Ok(Router::new()
        .route("/", get(index))
        .route("/assets/styles.css", get(styles))
        .route("/assets/app.js", get(script))
        .route("/assets/api.js", get(api_script))
        .route("/assets/ui.js", get(ui_script))
        .route(
            "/assets/modules/arknights-endfield/operators/{filename}",
            get(endfield_operator_thumbnail),
        )
        .route("/api/health", get(health))
        .route("/api/uat/endfield", post(setup_endfield))
        .route("/api/modules", get(list_modules))
        .route("/api/works", get(list_works).post(create_work))
        .route("/api/works/{work_id}/workspace", get(get_workspace))
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
    let application = state.lock()?;
    let module = application
        .modules()
        .find(|module| module.kind() == WorkKind::Game && module.id() == ENDFIELD_MODULE_ID)
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
        None => {
            application.create_work(WorkKind::Game, ENDFIELD_MODULE_ID, module.display_name())?
        }
    };
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
        theme: module.theme(),
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
                presentation: module.entity_presentation(&entity),
                entity,
            })
            .collect(),
    ))
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
) -> Result<Json<WikiEntityDetail>, WebError> {
    let application = state.lock()?;
    let detail = application
        .store()
        .get_entity_detail(&entity_id)?
        .ok_or_else(|| PerwigaError::NotFound(format!("entity {entity_id}")))?;
    Ok(Json(detail))
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
