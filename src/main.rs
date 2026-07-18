#![allow(dead_code, unused_imports, unused_variables, unused_mut, unused_assignments, unused_must_use)]
use axum::{
    extract::{State, Json, Path as AxumPath},
    response::IntoResponse,
    routing::{get, post, put, delete},
    Router,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod agent;
mod scraper;
mod validator;
mod desktop;
mod state;
mod sub_agent;
mod auth;
mod study;
mod sync;
mod client_protocol;

use crate::state::{
    AppState, Project, PromptConfig, ActiveAgentStatus, ProcessRegistry, ToolResultStore, SubAgentManager,
    ChatSession, ChatMessage, CicleState, CiclePhase,
};
use crate::desktop::DesktopController;
use crate::auth::{UserStore, ChallengeStore, SessionStore, UserLimits, WeeklySchedule, generate_keypair};
use crate::study::StudyEngine;
use crate::sync::SyncStore;
use crate::client_protocol::{
    ClientRequest, ConnectRequest, HeartbeatRequest,
    ClientResponseWrapper, PollRequest, ConnectedClient,
};
use std::sync::OnceLock;

fn deepseek_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY no configurada"))
}

const DEFAULT_GLOBAL_SYSTEM_PROMPT: &str = include_str!("../prompts/default_system_prompt.txt");
const STUDY_SYSTEM_PROMPT: &str = include_str!("../prompts/study_system_prompt.txt");

// ============================================================================
// Helpers de AutenticaciÃ³n
// ============================================================================

/// Extrae el token Bearer del header Authorization
fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers.get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// Verifica que el usuario sea admin (por token o por ser puerto 80)
async fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, String)> {
    if state.port_80 {
        return Ok("admin_local".to_string());
    }
    let token = extract_bearer_token(headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Token Bearer requerido.".into()))?;
    let username = state.session_store.validate_token(&token)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Token invÃ¡lido o expirado.".into()))?;
    if !state.user_store.is_admin(&username) {
        return Err((StatusCode::FORBIDDEN, "Se requiere rol admin.".into()));
    }
    Ok(username)
}

/// Verifica que el usuario estÃ© autenticado (normal o admin)
async fn require_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, String)> {
    if state.port_80 {
        return Ok("admin_local".to_string());
    }
    let token = extract_bearer_token(headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Token Bearer requerido.".into()))?;
    state.session_store.validate_token(&token)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Token invÃ¡lido o expirado.".into()))
}

// ============================================================================
// Chat Helpers (nueva estructura de almacenamiento)
// ============================================================================

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect::<String>()
        .trim()
        .replace(" ", "_")
        .chars()
        .take(40)
        .collect()
}

fn get_chat_dir(state: &AppState, username: &str, is_admin_or_port80: bool) -> PathBuf {
    if is_admin_or_port80 || username == "admin_local" {
        state.base_workspace.join(".config").join("chats")
    } else {
        state.base_workspace.join(".config").join("chats").join(username)
    }
}

fn get_chat_path(state: &AppState, username: &str, is_admin_or_port80: bool, title: &str, id: &str) -> PathBuf {
    let dir = get_chat_dir(state, username, is_admin_or_port80);
    let safe_title = sanitize_filename(title);
    dir.join(format!("{}-{}.json", safe_title, id))
}

/// Migra chats existentes del formato viejo (<uuid>.json) al nuevo (<title>-<uuid>.json)
fn migrate_chats(state: &AppState) {
    let old_dir = state.base_workspace.join(".config").join("chats");
    if !old_dir.exists() { return; }

    let entries: Vec<_> = match fs::read_dir(&old_dir) {
        Ok(e) => e.filter_map(Result::ok).collect(),
        Err(_) => return,
    };

    let mut migrated = 0;
    for entry in &entries {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        // Si ya tiene el formato nuevo (contiene un guion con titulo antes), saltar
        if fname.contains('-') && fname.matches('-').count() >= 1 { continue; }
        // Es formato viejo: <uuid>.json
        if fname.len() < 30 { continue; } // probablemente no es UUID

        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str::<ChatSession>(&content) {
                let safe_title = sanitize_filename(&session.title);
                let new_name = format!("{}-{}.json", safe_title, session.id);
                let new_path = old_dir.join(&new_name);
                if !new_path.exists() {
                    let _ = fs::rename(&path, &new_path);
                    migrated += 1;
                }
            }
        }
    }
    if migrated > 0 {
        eprintln!("[IAF] Migrados {} chats al nuevo formato <titulo>-<UUID>.json", migrated);
    }
}

// ============================================================================
// Endpoints de AutenticaciÃ³n
// ============================================================================

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> impl IntoResponse {
    match state.user_store.verify_password(&payload.username, &payload.password) {
        Ok(Some(user)) => {
            let token = state.session_store.create_session(&user.username);
            Json(json!({
                "status": "ok",
                "token": token,
                "username": user.username,
                "is_admin": user.is_admin,
                "has_study_access": user.has_study_access(),
                "has_programming_access": user.has_programming_access(),
            }))
        }
        Ok(None) => Json(json!({ "status": "error", "message": "Credenciales invÃ¡lidas." })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

#[derive(Deserialize)]
struct ChallengeRequest {
    username: String,
}

async fn challenge(State(state): State<AppState>, Json(payload): Json<ChallengeRequest>) -> impl IntoResponse {
    let user = match state.user_store.find_user(&payload.username) {
        Some(u) => u,
        None => return Json(json!({ "status": "error", "message": "Usuario no encontrado." })),
    };
    if !user.is_admin {
        return Json(json!({ "status": "error", "message": "Solo los administradores usan autenticaciÃ³n por nonce." }));
    }
    if user.public_key.is_none() {
        return Json(json!({ "status": "error", "message": "Este admin no tiene clave pÃºblica configurada." }));
    }
    let nonce = state.challenge_store.generate_challenge(&payload.username);
    Json(json!({ "status": "ok", "nonce": nonce }))
}

#[derive(Deserialize)]
struct VerifyRequest {
    username: String,
    nonce: String,
    signature: String,
}

async fn verify(State(state): State<AppState>, Json(payload): Json<VerifyRequest>) -> impl IntoResponse {
    let user = match state.user_store.find_user(&payload.username) {
        Some(u) => u,
        None => return Json(json!({ "status": "error", "message": "Usuario no encontrado." })),
    };
    let pk = match &user.public_key {
        Some(k) => k.clone(),
        None => return Json(json!({ "status": "error", "message": "Este usuario no tiene clave pÃºblica." })),
    };
    match state.challenge_store.verify_challenge(&payload.username, &payload.nonce, &payload.signature, &pk) {
        Ok(true) => {
            let token = state.session_store.create_session(&user.username);
            Json(json!({
                "status": "ok", "token": token, "username": user.username,
                "is_admin": user.is_admin,
                "has_study_access": user.has_study_access(),
                "has_programming_access": user.has_programming_access(),
            }))
        }
        Ok(false) => Json(json!({ "status": "error", "message": "Firma invÃ¡lida." })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

async fn keygen() -> impl IntoResponse {
    let (private_hex, public_hex) = generate_keypair();
    Json(json!({
        "status": "ok",
        "private_key": private_hex,
        "public_key": public_hex,
        "warning": "Guarda tu private_key en un lugar seguro. NUNCA la compartas. Esta es la ÃšNICA vez que la verÃ¡s."
    }))
}

#[derive(Deserialize)]
struct LogoutRequest {
    token: String,
}

async fn logout(State(state): State<AppState>, Json(payload): Json<LogoutRequest>) -> impl IntoResponse {
    state.session_store.revoke_token(&payload.token);
    Json(json!({ "status": "ok", "message": "SesiÃ³n cerrada." }))
}

/// Helper para que los scripts .ps1 firmen nonces localmente.
#[derive(Deserialize)]
struct SignRequest {
    private_key: String,
    nonce: String,
}

async fn sign_nonce(Json(payload): Json<SignRequest>) -> impl IntoResponse {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    let nonce_bytes = match BASE64.decode(&payload.nonce) {
        Ok(b) => b,
        Err(e) => return Json(json!({ "status": "error", "message": format!("Nonce invÃ¡lido: {}", e) })),
    };
    match crate::auth::sign_message(&payload.private_key, &nonce_bytes) {
        Ok(signature) => Json(json!({ "status": "ok", "signature": signature })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

async fn client_check() -> impl IntoResponse {
    let possible_paths = vec![
        "client/target/release/iaf-client.exe",
        "client/target/debug/iaf-client.exe",
        "iaf-client.exe",
    ];
    let mut found = Vec::new();
    for path in &possible_paths {
        if std::path::Path::new(path).exists() {
            found.push(path.to_string());
        }
    }
    Json(json!({
        "status": "ok",
        "client_installed": !found.is_empty(),
        "found_at": found,
        "expected_paths": possible_paths,
        "instructions": if found.is_empty() {
            "Para instalar el cliente: cd client && cargo build --release. Luego: .\\client\\target\\release\\iaf-client.exe <url> <user> <token>"
        } else {
            "Cliente encontrado. Ejecutalo con: iaf-client.exe http://127.0.0.1:8080 <username> <token>"
        }
    }))
}

// ============================================================================
// Endpoints Admin (gestiÃ³n de usuarios)
// ============================================================================

async fn admin_list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let admin = match require_admin(&state, &headers).await {
        Ok(a) => a, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    let _ = admin;
    let users = state.user_store.list_users();
    Json(json!({ "status": "ok", "users": users })).into_response()
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    password: Option<String>,
    public_key: Option<String>,
    is_admin: bool,
    permissions: Option<Vec<String>>,
    modo_estudio: Option<bool>,
    modo_programador: Option<bool>,
    editar_system_prompt_global: Option<bool>,
    editar_system_prompt_local: Option<bool>,
}

async fn admin_create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let _admin = match require_admin(&state, &headers).await {
        Ok(a) => a, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let perms = payload.permissions.unwrap_or_else(|| vec!["read_file".into(), "search_code".into()]);
    let limits = if payload.is_admin { UserLimits::admin() } else { UserLimits::default() };
    let result = if payload.is_admin && payload.public_key.is_some() {
        state.user_store.create_admin(&payload.username, &payload.public_key.unwrap(), perms, limits)
    } else if let Some(ref pw) = payload.password {
        state.user_store.create_user_with_password(
            &payload.username, pw, payload.is_admin, perms, limits,
            payload.modo_estudio.unwrap_or(false),
            payload.modo_programador.unwrap_or(false),
            payload.editar_system_prompt_global.unwrap_or(false),
            payload.editar_system_prompt_local.unwrap_or(false),
        )
    } else {
        Err("Se requiere password (usuarios normales) o public_key (admins).".into())
    };

    match result {
        Ok(user) => Json(json!({ "status": "ok", "user": user })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateLimitsRequest {
    limits: UserLimits,
}

async fn admin_update_limits(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(username): AxumPath<String>,
    Json(payload): Json<UpdateLimitsRequest>,
) -> impl IntoResponse {
    let _admin = match require_admin(&state, &headers).await {
        Ok(a) => a, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    match state.user_store.update_limits(&username, payload.limits) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateAccessRequest {
    modo_estudio: bool,
    modo_programador: bool,
    editar_system_prompt_global: bool,
    editar_system_prompt_local: bool,
}

async fn admin_update_access(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(username): AxumPath<String>,
    Json(payload): Json<UpdateAccessRequest>,
) -> impl IntoResponse {
    let _admin = match require_admin(&state, &headers).await {
        Ok(a) => a, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    match state.user_store.update_access(
        &username,
        payload.modo_estudio,
        payload.modo_programador,
        payload.editar_system_prompt_global,
        payload.editar_system_prompt_local,
    ) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateScheduleRequest {
    horarios: HashMap<String, Vec<(u32, u32)>>,
}

async fn admin_update_schedule(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(username): AxumPath<String>,
    Json(payload): Json<UpdateScheduleRequest>,
) -> impl IntoResponse {
    let _admin = match require_admin(&state, &headers).await {
        Ok(a) => a, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    let schedule = WeeklySchedule { horarios: payload.horarios };
    match state.user_store.update_schedule(&username, schedule) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    new_password: String,
}

async fn admin_change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(username): AxumPath<String>,
    Json(payload): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    let _admin = match require_admin(&state, &headers).await {
        Ok(a) => a, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    match state.user_store.change_password(&username, &payload.new_password) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

async fn admin_delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(username): AxumPath<String>,
) -> impl IntoResponse {
    let admin_name = match require_admin(&state, &headers).await {
        Ok(a) => a, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    if username == admin_name {
        return (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": "No podÃ©s eliminarte a vos mismo." }))).into_response();
    }
    match state.user_store.delete_user(&username) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

// ============================================================================
// Endpoints de System Prompts (Global y Local)
// ============================================================================

#[derive(Deserialize)]
struct SaveGlobalPromptRequest {
    content: String,
}

async fn get_global_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = state.user_store.find_user(&username);
    let can_edit = user.as_ref().map(|u| u.can_edit_global_prompt()).unwrap_or(false);
    let content = state.load_global_prompt(&username);
    let default_content = {
        let prompts = state.prompts.lock().unwrap();
        prompts.global_default.clone()
    };

    Json(json!({
        "status": "ok",
        "content": content,
        "default_content": default_content,
        "can_edit": can_edit,
    })).into_response()
}

async fn save_global_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveGlobalPromptRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = state.user_store.find_user(&username);
    if !user.as_ref().map(|u| u.can_edit_global_prompt()).unwrap_or(false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenÃ©s permiso para editar el system prompt global." }))).into_response();
    }

    match state.save_global_prompt(&username, &payload.content) {
        Ok(()) => {
            // TambiÃ©n actualizar en memoria
            let mut prompts = state.prompts.lock().unwrap();
            prompts.global_current = payload.content.clone();
            Json(json!({ "status": "ok" })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

async fn reset_global_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = state.user_store.find_user(&username);
    if !user.as_ref().map(|u| u.can_edit_global_prompt()).unwrap_or(false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenÃ©s permiso para editar el system prompt global." }))).into_response();
    }

    let default_content = {
        let prompts = state.prompts.lock().unwrap();
        prompts.global_default.clone()
    };

    match state.save_global_prompt(&username, &default_content) {
        Ok(()) => {
            let mut prompts = state.prompts.lock().unwrap();
            prompts.global_current = default_content.clone();
            Json(json!({ "status": "ok", "content": default_content })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct SaveLocalPromptRequest {
    project_name: String,
    content: String,
}

async fn get_local_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(project_name): AxumPath<String>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = state.user_store.find_user(&username);
    let can_edit = user.as_ref().map(|u| u.can_edit_local_prompt()).unwrap_or(false);
    let content = state.load_local_prompt(&username, &project_name);

    Json(json!({
        "status": "ok",
        "content": content,
        "can_edit": can_edit,
    })).into_response()
}

async fn save_local_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveLocalPromptRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = state.user_store.find_user(&username);
    if !user.as_ref().map(|u| u.can_edit_local_prompt()).unwrap_or(false) {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenÃ©s permiso para editar system prompts locales." }))).into_response();
    }

    match state.save_local_prompt(&username, &payload.project_name, &payload.content) {
        Ok(()) => {
            // TambiÃ©n actualizar en memoria
            let mut prompts = state.prompts.lock().unwrap();
            prompts.projects.insert(payload.project_name.clone(), payload.content);
            Json(json!({ "status": "ok" })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

// ============================================================================
// Endpoints de Ciclos (Cicle)
// ============================================================================

async fn get_cicle(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(project_name): AxumPath<String>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let cicle = state.load_cicle(&username, &project_name)
        .unwrap_or_else(|| CicleState::new(&project_name));

    Json(json!({
        "status": "ok",
        "cicle": cicle,
    })).into_response()
}

#[derive(Deserialize)]
struct UpdateCicleRequest {
    phase: String,
}

async fn update_cicle(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(project_name): AxumPath<String>,
    Json(payload): Json<UpdateCicleRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let phase = match payload.phase.as_str() {
        "ciclo1_implementacion" => CiclePhase::Implementacion,
        "ciclo2_optimizacion" => CiclePhase::Optimizacion,
        "ciclo3_busqueda_bugs" => CiclePhase::BusquedaBugs,
        "ciclo4_reduccion" => CiclePhase::Reduccion,
        "ciclo5_segunda_busqueda_bugs" => CiclePhase::SegundaBusquedaBugs,
        "ciclo6_terminar" => CiclePhase::Terminar,
        _ => return (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": "Fase invÃ¡lida. Usar: ciclo1_implementacion, ciclo2_optimizacion, etc." }))).into_response(),
    };

    let mut cicle = state.load_cicle(&username, &project_name)
        .unwrap_or_else(|| CicleState::new(&project_name));
    cicle.current_phase = phase;
    cicle.last_updated = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    cicle.iteration_count += 1;

    match state.save_cicle(&username, &cicle) {
        Ok(()) => Json(json!({ "status": "ok", "cicle": cicle })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

// ============================================================================
// Endpoints de Estudio
// ============================================================================

#[derive(Deserialize)]
struct SaveProfileRequest {
    age: Option<u8>,
    high_capabilities: Option<String>,
    neurological_conditions: Option<Vec<String>>,
    favorite_games: Option<Vec<String>>,
    favorite_youtubers: Option<Vec<String>>,
    hobbies: Option<Vec<String>>,
}

async fn study_save_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveProfileRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = match state.user_store.find_user(&username) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": "Usuario no encontrado." }))).into_response(),
    };
    if !user.has_study_access() && !user.is_admin {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenÃ©s acceso al modo estudio." }))).into_response();
    }

    let mut profile = state.study_engine.get_or_create_profile(&username);
    if let Some(age) = payload.age { profile.age = Some(age); }
    if let Some(hc) = payload.high_capabilities { profile.high_capabilities = Some(hc); }
    if let Some(nc) = payload.neurological_conditions { profile.neurological_conditions = nc; }
    if let Some(fg) = payload.favorite_games { profile.favorite_games = fg; }
    if let Some(fy) = payload.favorite_youtubers { profile.favorite_youtubers = fy; }
    if let Some(h) = payload.hobbies { profile.hobbies = h; }

    match state.study_engine.save_profile(&profile) {
        Ok(()) => Json(json!({ "status": "ok", "profile": profile })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

async fn study_get_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    let profile = state.study_engine.get_or_create_profile(&username);
    let kb = state.study_engine.get_or_create_knowledge(&username);
    let engagement = state.study_engine.calculate_engagement(&username);

    Json(json!({
        "status": "ok",
        "profile": profile,
        "knowledge": kb,
        "engagement": engagement,
        "phase": profile.phase,
    })).into_response()
}

async fn study_get_knowledge(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    let kb = state.study_engine.get_or_create_knowledge(&username);
    Json(json!({ "status": "ok", "knowledge": kb })).into_response()
}

#[derive(Deserialize)]
struct CreateStudyProjectRequest {
    name: String,
    description: String,
}

async fn study_create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateStudyProjectRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    match state.study_engine.create_study_project(&payload.name, &payload.description, &username) {
        Ok(proj) => Json(json!({ "status": "ok", "project": proj })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct AddMemberRequest {
    username: String,
}

async fn study_add_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
    Json(payload): Json<AddMemberRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    match state.study_engine.add_member_to_project(&project_id, &payload.username) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

async fn study_get_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    let projects = state.study_engine.get_user_projects(&username);
    Json(json!({ "status": "ok", "projects": projects })).into_response()
}

#[derive(Deserialize)]
struct BuildStudyPromptRequest {
    project_id: Option<String>,
}

async fn study_build_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<BuildStudyPromptRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let base = if let Some(ref pid) = payload.project_id {
        let projects = state.study_engine.projects.lock().unwrap();
        projects.get(pid)
            .and_then(|p| p.study_prompt.clone())
            .unwrap_or_else(|| STUDY_SYSTEM_PROMPT.to_string())
    } else {
        STUDY_SYSTEM_PROMPT.to_string()
    };

    let prompt = state.study_engine.build_study_system_prompt(&username, &base);
    Json(json!({ "status": "ok", "system_prompt": prompt })).into_response()
}

// ============================================================================
// Endpoints de Chat (con nueva estructura de directorios)
// ============================================================================

#[derive(Deserialize)]
struct ChatInput {
    message: String,
    project_name: Option<String>,
    session_id: Option<String>,
    mode: Option<String>, // "programming" o "study"
}

async fn chat_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ChatInput>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let is_admin = username == "admin_local" || state.user_store.is_admin(&username);
    let has_session = payload.session_id.is_some();
    let session_id = payload.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Determinar directorio de chats
    let chat_dir = get_chat_dir(&state, &username, is_admin);
    let _ = fs::create_dir_all(&chat_dir);

    // Buscar chat existente o crear nuevo
    let chat_file = if has_session {
        let mut found = None;
        if let Ok(entries) = fs::read_dir(&chat_dir) {
            for entry in entries.filter_map(Result::ok) {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.ends_with(&format!("-{}.json", session_id)) {
                    found = Some(entry.path());
                    break;
                }
            }
        }
        found
    } else {
        None
    };

    let mut session = if let Some(ref path) = chat_file {
        if let Ok(content) = fs::read_to_string(path) {
            serde_json::from_str::<ChatSession>(&content).unwrap_or_else(|_| ChatSession {
                id: session_id.clone(),
                title: "Nueva conversaciÃ³n".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            })
        } else {
            ChatSession {
                id: session_id.clone(),
                title: "Nueva conversaciÃ³n".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            }
        }
    } else {
        let title = payload.message.chars().take(30).collect::<String>();
        ChatSession {
            id: session_id.clone(),
            title,
            messages: Vec::new(),
            project_name: payload.project_name.clone(),
            steps: None,
        }
    };

    // Agregar mensaje del usuario
    session.messages.push(ChatMessage {
        role: "user".to_string(),
        content: payload.message.clone(),
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
    });

    // Guardar
    let save_path = get_chat_path(&state, &username, is_admin, &session.title, &session.id);
    let _ = fs::create_dir_all(save_path.parent().unwrap());
    let _ = fs::write(&save_path, serde_json::to_string_pretty(&session).unwrap());

    Json(json!({
        "status": "ok",
        "session_id": session.id,
        "title": session.title,
        "chat_path": save_path.to_string_lossy(),
    })).into_response()
}

async fn get_chats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(_) => {
            return Json(json!({ "status": "ok", "chats": [] })).into_response();
        }
    };

    let is_admin = username == "admin_local" || state.user_store.is_admin(&username);
    let chat_dir = get_chat_dir(&state, &username, is_admin);

    let mut summaries = Vec::new();
    if is_admin {
        if let Ok(entries) = fs::read_dir(state.base_workspace.join(".config").join("chats")) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(sub_entries) = fs::read_dir(&path) {
                        for sub in sub_entries.filter_map(Result::ok) {
                            if sub.path().extension().and_then(|e| e.to_str()) == Some("json") {
                                if let Ok(content) = fs::read_to_string(sub.path()) {
                                    if let Ok(s) = serde_json::from_str::<ChatSession>(&content) {
                                        summaries.push(json!({
                                            "id": s.id, "title": s.title, "project_name": s.project_name,
                                            "path": sub.path().to_string_lossy(),
                                        }));
                                    }
                                }
                            }
                        }
                    }
                } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(s) = serde_json::from_str::<ChatSession>(&content) {
                            summaries.push(json!({
                                "id": s.id, "title": s.title, "project_name": s.project_name,
                                "path": path.to_string_lossy(),
                            }));
                        }
                    }
                }
            }
        }
    } else if chat_dir.exists() {
        if let Ok(entries) = fs::read_dir(&chat_dir) {
            for entry in entries.filter_map(Result::ok) {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(s) = serde_json::from_str::<ChatSession>(&content) {
                            summaries.push(json!({
                                "id": s.id, "title": s.title, "project_name": s.project_name,
                                "path": entry.path().to_string_lossy(),
                            }));
                        }
                    }
                }
            }
        }
    }

    Json(json!({ "status": "ok", "chats": summaries })).into_response()
}

async fn get_chat_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let is_admin = username == "admin_local" || state.user_store.is_admin(&username);

    let chat_dir = get_chat_dir(&state, &username, is_admin);
    if let Ok(entries) = fs::read_dir(&chat_dir) {
        for entry in entries.filter_map(Result::ok) {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.ends_with(&format!("-{}.json", id)) || fname == format!("{}.json", id) {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(session) = serde_json::from_str::<ChatSession>(&content) {
                        return Json(json!({ "status": "ok", "session": session })).into_response();
                    }
                }
            }
        }
    }

    (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": "Chat no encontrado." }))).into_response()
}

// ============================================================================
// Endpoints de Sync
// ============================================================================

#[derive(Deserialize)]
struct SyncManifestInput {
    project_id: String,
    client_files: HashMap<String, String>,
    last_sync: u64,
}

async fn sync_process(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SyncManifestInput>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let manifest = crate::sync::SyncManifest {
        client_files: payload.client_files,
        last_sync: payload.last_sync,
    };

    match state.sync_store.process_sync(&payload.project_id, &manifest) {
        Ok(response) => Json(json!({ "status": "ok", "response": response })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct PushVersionRequest {
    project_id: String,
    path: String,
    content_base64: String,
    message: String,
}

async fn sync_push_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PushVersionRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    match state.sync_store.push_version(&payload.project_id, &payload.path, &payload.content_base64, &username, &payload.message) {
        Ok(version) => Json(json!({ "status": "ok", "version": version })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

async fn sync_get_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((project_id, path)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    let decoded_path = urlencoding::decode(&path).unwrap_or_else(|_| std::borrow::Cow::Borrowed(&path));
    let history = state.sync_store.get_file_history(&project_id, &decoded_path);
    Json(json!({ "status": "ok", "history": history })).into_response()
}

// ============================================================================
// Endpoints del Cliente (protocolo de ejecuciÃ³n remota)
// ============================================================================

async fn client_connect(
    State(state): State<AppState>,
    Json(payload): Json<ConnectRequest>,
) -> impl IntoResponse {
    let username = match state.session_store.validate_token(&payload.token) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({ "status": "error", "message": "Token invÃ¡lido." }))).into_response(),
    };

    if username != payload.username {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Token no coincide con username." }))).into_response();
    }

    let client_id = format!("client_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    let client = ConnectedClient {
        client_id: client_id.clone(),
        username: username.clone(),
        connected_at: now,
        last_heartbeat: now,
        host_info: payload.host_info.clone(),
    };

    state.connected_clients.lock().unwrap().insert(client_id.clone(), client.clone());

    state.client_pending_requests.lock().unwrap().entry(client_id.clone()).or_insert_with(Vec::new);

    Json(json!({
        "status": "ok",
        "client_id": client_id,
        "pending_requests": Vec::<ClientRequest>::new(),
    })).into_response()
}

async fn client_heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<HeartbeatRequest>,
) -> impl IntoResponse {
    let mut clients = state.connected_clients.lock().unwrap();
    if let Some(client) = clients.get_mut(&payload.client_id) {
        let username = match state.session_store.validate_token(&payload.token) {
            Some(u) => u,
            None => return (StatusCode::UNAUTHORIZED, Json(json!({ "status": "error", "message": "Token invÃ¡lido." }))).into_response(),
        };
        if username != client.username {
            return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Token no coincide." }))).into_response();
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        client.last_heartbeat = now;
        Json(json!({ "status": "ok" })).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": "Cliente no encontrado." }))).into_response()
    }
}

async fn client_poll(
    State(state): State<AppState>,
    Json(payload): Json<PollRequest>,
) -> impl IntoResponse {
    let clients = state.connected_clients.lock().unwrap();
    let client = match clients.get(&payload.client_id) {
        Some(c) => c.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": "Cliente no encontrado." }))).into_response(),
    };
    drop(clients);

    let username = match state.session_store.validate_token(&payload.token) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({ "status": "error", "message": "Token invÃ¡lido." }))).into_response(),
    };
    if username != client.username {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Token no coincide." }))).into_response();
    }

    let mut pending = state.client_pending_requests.lock().unwrap();
    let requests = pending.remove(&payload.client_id).unwrap_or_default();

    Json(json!({ "status": "ok", "pending_requests": requests })).into_response()
}

async fn client_response(
    State(state): State<AppState>,
    Json(payload): Json<ClientResponseWrapper>,
) -> impl IntoResponse {
    let clients = state.connected_clients.lock().unwrap();
    let client = match clients.get(&payload.client_id) {
        Some(c) => c.clone(),
        None => return (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": "Cliente no encontrado." }))).into_response(),
    };
    drop(clients);

    let username = match state.session_store.validate_token(&payload.token) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({ "status": "error", "message": "Token invÃ¡lido." }))).into_response(),
    };
    if username != client.username {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Token no coincide." }))).into_response();
    }

    state.client_responses.lock().unwrap().insert(
        payload.response.request_id.clone(),
        payload.response.clone(),
    );

    Json(json!({ "status": "ok" })).into_response()
}

// ============================================================================
// Legacy Endpoints (delegate to agent or client)
// ============================================================================

async fn get_projects(State(state): State<AppState>) -> impl IntoResponse {
    let projs = state.projects.lock().unwrap().clone();
    Json(projs)
}

async fn get_agent_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.active_agent.lock().unwrap().clone();
    Json(json!({
        "running": status.running,
        "interrupted": status.interrupted,
        "esperando_respuesta_usuario": status.esperando_respuesta_usuario,
        "current_session_id": status.current_session_id,
    }))
}

// ============================================================================
// MAIN â€” Doble Puerto
// ============================================================================

fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::permissive();

    Router::new()
        // Auth
        .route("/api/auth/login", post(login))
        .route("/api/auth/challenge", post(challenge))
        .route("/api/auth/verify", post(verify))
        .route("/api/auth/keygen", get(keygen))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/sign", post(sign_nonce))
        // Projects & Agent
        .route("/api/projects", get(get_projects))
        .route("/api/agent/status", get(get_agent_status))
        // Chat
        .route("/api/chat", post(chat_endpoint))
        .route("/api/chats", get(get_chats))
        .route("/api/chats/:id", get(get_chat_session))
        // Admin
        .route("/api/admin/users", get(admin_list_users).post(admin_create_user))
        .route("/api/admin/users/:username/limits", put(admin_update_limits))
        .route("/api/admin/users/:username/access", put(admin_update_access))
        .route("/api/admin/users/:username/schedule", put(admin_update_schedule))
        .route("/api/admin/users/:username/password", put(admin_change_password))
        .route("/api/admin/users/:username", delete(admin_delete_user))
        // System Prompts
        .route("/api/prompts/global", get(get_global_prompt).post(save_global_prompt))
        .route("/api/prompts/global/reset", post(reset_global_prompt))
        .route("/api/prompts/local/:project_name", get(get_local_prompt))
        .route("/api/prompts/local", post(save_local_prompt))
        // Cicles
        .route("/api/cicles/:project_name", get(get_cicle).put(update_cicle))
        // Study
        .route("/api/study/profile", get(study_get_profile).post(study_save_profile))
        .route("/api/study/knowledge", get(study_get_knowledge))
        .route("/api/study/projects", get(study_get_projects).post(study_create_project))
        .route("/api/study/projects/:id/members", post(study_add_member))
        .route("/api/study/build-prompt", post(study_build_prompt))
        // Sync
        .route("/api/sync/process", post(sync_process))
        .route("/api/sync/push", post(sync_push_version))
        .route("/api/sync/history/:project_id/*path", get(sync_get_history))
        // Client
        .route("/api/client/connect", post(client_connect))
        .route("/api/client/check", get(client_check))
        .route("/api/client/heartbeat", post(client_heartbeat))
        .route("/api/client/poll", post(client_poll))
        .route("/api/client/response", post(client_response))
        .layer(cors)
        .nest_service("/", ServeDir::new("public"))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let base_workspace = detect_base_workspace();
    let config_dir = base_workspace.join(".config");
    let _ = fs::create_dir_all(&config_dir);
    let _ = fs::create_dir_all(config_dir.join("chats"));
    let _ = fs::create_dir_all(config_dir.join("data"));

    let config_path = config_dir.join("prompts.json");
    let mut prompts = PromptConfig {
        global_default: DEFAULT_GLOBAL_SYSTEM_PROMPT.to_string(),
        global_current: DEFAULT_GLOBAL_SYSTEM_PROMPT.to_string(),
        projects: HashMap::new(),
    };

    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(parsed) = serde_json::from_str::<PromptConfig>(&content) {
                prompts = parsed;
            }
        }
    } else {
        let _ = fs::write(&config_path, serde_json::to_string_pretty(&prompts).unwrap());
    }

    let mut initial_projects = Vec::new();
    let local_config_path = config_dir.join("local_projects.json");
    if local_config_path.exists() {
        if let Ok(content) = fs::read_to_string(&local_config_path) {
            if let Ok(parsed) = serde_json::from_str::<Vec<Project>>(&content) {
                initial_projects = parsed;
            }
        }
    }

    let state = AppState {
        config_path: config_path.clone(),
        prompts: Arc::new(Mutex::new(prompts)),
        projects: Arc::new(Mutex::new(initial_projects)),
        base_workspace: base_workspace.clone(),
        pending_captcha: Arc::new(Mutex::new(None)),
        active_agent: Arc::new(Mutex::new(ActiveAgentStatus::default())),
        abort_handle: Arc::new(Mutex::new(None)),
        desktop: Arc::new(Mutex::new(DesktopController::new())),
        image_store: Arc::new(Mutex::new(HashMap::new())),
        context_store: Arc::new(Mutex::new(HashMap::new())),
        process_registry: ProcessRegistry::new(),
        tool_results: ToolResultStore::new(),
        sub_agents: SubAgentManager::new(),
        user_store: UserStore::load(&config_dir),
        challenge_store: ChallengeStore::new(300),
        session_store: SessionStore::new(),
        study_engine: StudyEngine::new(config_dir.join("study")),
        sync_store: SyncStore::new(&config_dir),
        connected_clients: Arc::new(Mutex::new(HashMap::new())),
        client_pending_requests: Arc::new(Mutex::new(HashMap::new())),
        client_responses: Arc::new(Mutex::new(HashMap::new())),
        port_80: false,
    };

    // Migrar chats existentes al nuevo formato
    migrate_chats(&state);

    // Crear directorios para usuarios existentes
    for user in state.user_store.list_users() {
        if !user.is_admin {
            let _ = fs::create_dir_all(base_workspace.join(".config").join("chats").join(&user.username));
        }
    }

    let mut state_80 = state.clone();
    state_80.port_80 = true;
    let state_8080 = state;

    let app_80 = build_app(state_80);
    let app_8080 = build_app(state_8080);

    let addr_80 = SocketAddr::from(([0, 0, 0, 0], 80));
    let addr_8080 = SocketAddr::from(([127, 0, 0, 1], 8080));

    println!("ðŸš€ IAF Server iniciado:");
    println!("   â€¢ Puerto 80   â€” Admin local (sin auth): http://{}", addr_80);
    println!("   â€¢ Puerto 8080 â€” Usuarios (requiere login): http://{}", addr_8080);

    let srv_80 = tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr_80).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("âš ï¸  No se pudo bindear puerto 80 (requiere admin): {}", e);
                return;
            }
        };
        eprintln!("[IAF] Puerto 80 escuchando...");
        if let Err(e) = axum::serve(listener, app_80).await {
            eprintln!("[IAF] Error en servidor puerto 80: {}", e);
        }
    });

    let srv_8080 = tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr_8080).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("âŒ Error fatal bindeando puerto 8080: {}", e);
                std::process::exit(1);
            }
        };
        eprintln!("[IAF] Puerto 8080 escuchando...");
        if let Err(e) = axum::serve(listener, app_8080).await {
            eprintln!("[IAF] Error en servidor puerto 8080: {}", e);
        }
    });

    let _ = tokio::join!(srv_80, srv_8080);
}

fn detect_base_workspace() -> PathBuf {
    if let Ok(env_ws) = std::env::var("IAF_WORKSPACE") {
        let p = PathBuf::from(&env_ws);
        if p.exists() && p.is_dir() {
            eprintln!("[IAF] base_workspace vÃ­a IAF_WORKSPACE: {}", p.display());
            return p;
        }
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut candidate = exe_dir.to_path_buf();
            for _ in 0..5 {
                if candidate.join(".config").exists() || candidate.join("Cargo.toml").exists() {
                    eprintln!("[IAF] base_workspace vÃ­a exe: {}", candidate.display());
                    return candidate;
                }
                if let Some(parent) = candidate.parent() { candidate = parent.to_path_buf(); }
                else { break; }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        eprintln!("[IAF] base_workspace vÃ­a current_dir: {}", cwd.display());
        return cwd;
    }
    PathBuf::from(".")
}
