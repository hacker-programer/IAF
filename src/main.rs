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
use std::collections::{HashMap, HashSet};
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
    ChatSession, ChatMessage, CicleState, CiclePhase, CaptchaRequest, AuditStep,
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
pub const STUDY_SYSTEM_PROMPT: &str = include_str!("../prompts/study_system_prompt.txt");

// ============================================================================
// Helpers de Autenticación
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
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Token inválido o expirado.".into()))?;
    if !state.user_store.is_admin(&username) {
        return Err((StatusCode::FORBIDDEN, "Se requiere rol admin.".into()));
    }
    Ok(username)
}

/// Verifica que el usuario esté autenticado (normal o admin)
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
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Token inválido o expirado.".into()))
}

// ============================================================================
// Chat Helpers (nueva estructura de almacenamiento)
// ============================================================================

// FIX #46: Usar la versión unificada de utils.rs (70 chars + hash anti-colisión)
use iaf::utils::sanitize_filename;

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

/// Limpia archivos viejos con el mismo UUID. FIX #10: validación estricta del sufijo.
fn clean_old_chat_files(dir: &PathBuf, session_id: &str) {
    if !dir.exists() || !looks_like_uuid_stem(session_id) {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let expected_suffix = format!("-{}", session_id);
            if fname.ends_with(&expected_suffix) && fname.len() > expected_suffix.len() {
                let _ = fs::remove_file(&path);
                eprintln!("[IAF] Limpiado archivo duplicado: {}", path.display());
            }
        }
    }
}
/// Determina si un nombre de archivo (sin extensión) parece un UUID
fn looks_like_uuid_stem(stem: &str) -> bool {
    stem.len() >= 30
        && stem.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        && stem.matches('-').count() >= 3
}

/// Migración recursiva: renombra archivos <uuid>.json a <title>-<uuid>.json
/// dentro de un directorio dado. Retorna cantidad de archivos migrados.
fn migrate_chats_in_dir(dir: &PathBuf) -> usize {
    if !dir.exists() || !dir.is_dir() {
        return 0;
    }
    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(e) => e.filter_map(Result::ok).collect(),
        Err(_) => return 0,
    };
    let mut migrated = 0;
    for entry in &entries {
        let path = entry.path();
        if path.is_dir() {
            // Recurse into subdirectories (user folders)
            migrated += migrate_chats_in_dir(&path);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        // Si ya tiene el formato nuevo (<title>-<uuid>.json), saltar
        if fname.contains('-') && fname.matches('-').count() >= 1 && !looks_like_uuid_stem(fname) {
            continue;
        }
        // Es formato viejo: <uuid>.json
        if !looks_like_uuid_stem(fname) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str::<ChatSession>(&content) {
                let safe_title = sanitize_filename(&session.title);
                let new_name = format!("{}-{}.json", safe_title, session.id);
                let new_path = dir.join(&new_name);
                if !new_path.exists() {
                    let _ = fs::rename(&path, &new_path);
                    migrated += 1;
                }
            }
        }
    }
    migrated
}

/// Migra chats existentes del formato viejo (<uuid>.json) al nuevo (<title>-<uuid>.json).
/// También migra prompts.json y local_projects.json al formato por usuario.
fn migrate_chats(state: &AppState) {
    let chats_dir = state.base_workspace.join(".config").join("chats");
    if !chats_dir.exists() {
        return;
    }

    // 1. Migrar archivos de chat en el directorio raíz y subdirectorios
    let migrated = migrate_chats_in_dir(&chats_dir);
    if migrated > 0 {
        eprintln!("[IAF] Migrados {} chats al nuevo formato <titulo>-<UUID>.json", migrated);
    }

    // 2. Migrar prompts.json legacy → per-user globalPrompt.json
    let prompts_path = state.base_workspace.join(".config").join("prompts.json");
    if prompts_path.exists() {
        if let Ok(content) = fs::read_to_string(&prompts_path) {
            if let Ok(parsed) = serde_json::from_str::<PromptConfig>(&content) {
                // Si hay un global_current distinto del default, migrarlo al admin
                if parsed.global_current != parsed.global_default {
                    let admin_prompt_dir = state.base_workspace.join(".config").join("data").join("admin");
                    let _ = fs::create_dir_all(&admin_prompt_dir);
                    let admin_prompt_path = admin_prompt_dir.join("globalPrompt.json");
                    if !admin_prompt_path.exists() {
                        let _ = fs::write(&admin_prompt_path, &parsed.global_current);
                        eprintln!("[IAF] Migrado prompts.json → data/admin/globalPrompt.json");
                    }
                }
                // Migrar project prompts → per-user per-project localPrompt.json
                for (proj_name, proj_prompt) in &parsed.projects {
                    let proj_dir = state.base_workspace.join(".config").join("data")
                        .join("admin").join(proj_name);
                    let _ = fs::create_dir_all(&proj_dir);
                    let local_path = proj_dir.join("localPrompt.json");
                    if !local_path.exists() {
                        let _ = fs::write(&local_path, proj_prompt);
                        eprintln!("[IAF] Migrado prompt proyecto '{}' → data/admin/{}/localPrompt.json", proj_name, proj_name);
                    }
                }
            }
        }
        // Renombrar prompts.json a prompts.json.bak para no procesarlo dos veces
        let bak_path = state.base_workspace.join(".config").join("prompts.json.bak");
        if !bak_path.exists() {
            let _ = fs::rename(&prompts_path, &bak_path);
            eprintln!("[IAF] prompts.json renombrado a prompts.json.bak (migración completada)");
        }
    }

    // 3. Migrar local_projects.json legacy → per-user
    let local_proj_path = state.base_workspace.join(".config").join("local_projects.json");
    let bak_path = state.base_workspace.join(".config").join("local_projects.json.bak");
    if local_proj_path.exists() {
        // Ya se carga en main(), solo marcar como migrado
        if !bak_path.exists() {
            let _ = fs::copy(&local_proj_path, &bak_path);
            eprintln!("[IAF] local_projects.json respaldado como .bak (copia, no rename)");
        }
    } else if bak_path.exists() {
        let _ = fs::copy(&bak_path, &local_proj_path);
        eprintln!("[IAF] local_projects.json restaurado desde .bak (recovery)");
    } else {
        let projects = state.projects.lock().unwrap();
        if !projects.is_empty() {
            let json = serde_json::to_string_pretty(&*projects).unwrap_or_default();
            let _ = fs::write(&local_proj_path, &json);
            eprintln!("[IAF] Proyectos persistidos en local_projects.json (recovery)");
        }
    }
}

// ============================================================================
// Endpoint de Descarga de Scripts
// ============================================================================

/// Sirve scripts desde la carpeta scripts/ del proyecto
/// GET /api/scripts/generate_keys  -> scripts/generate_keys.ps1
/// GET /api/scripts/sign_nonce     -> scripts/sign_nonce.ps1
async fn serve_script(
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> impl IntoResponse {
    let safe_name: String = name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();

    if safe_name.is_empty() || safe_name.len() > 64 {
        return (StatusCode::BAD_REQUEST, Json(json!({ "status": "error", "message": "Nombre de script invalido." }))).into_response();
    }

    let script_path = state.base_workspace
        .join("scripts")
        .join(format!("{}.ps1", safe_name));

    if !script_path.exists() {
        return (StatusCode::NOT_FOUND, Json(json!({
            "status": "error",
            "message": format!("Script '{}' no encontrado. Disponibles: generate_keys, sign_nonce", safe_name),
            "available_scripts": ["generate_keys", "sign_nonce"]
        }))).into_response();
    }

    match fs::read_to_string(&script_path) {
        Ok(content) => {
            let filename = format!("{}.ps1", safe_name);
            let headers = [
                ("Content-Type", "application/x-powershell"),
                ("Content-Disposition", &format!("attachment; filename=\"{}\"", filename)),
            ];
            (StatusCode::OK, headers, content).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "status": "error",
                "message": format!("Error leyendo script: {}", e)
            }))).into_response()
        }
    }
}

// ============================================================================
// Endpoints de Autenticación
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
        Ok(None) => Json(json!({ "status": "error", "message": "Credenciales inválidas." })),
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
        return Json(json!({ "status": "error", "message": "Solo los administradores usan autenticación por nonce." }));
    }
    if user.public_key.is_none() {
        return Json(json!({ "status": "error", "message": "Este admin no tiene clave pública configurada." }));
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
        None => return Json(json!({ "status": "error", "message": "Este usuario no tiene clave pública." })),
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
        Ok(false) => Json(json!({ "status": "error", "message": "Firma inválida." })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

async fn keygen() -> impl IntoResponse {
    let (private_hex, public_hex) = generate_keypair();
    Json(json!({
        "status": "ok",
        "private_key": private_hex,
        "public_key": public_hex,
        "warning": "Guarda tu private_key en un lugar seguro. NUNCA la compartas. Esta es la ÚNICA vez que la verás."
    }))
}

#[derive(Deserialize)]
struct LogoutRequest {
    token: String,
}

async fn logout(State(state): State<AppState>, Json(payload): Json<LogoutRequest>) -> impl IntoResponse {
    state.session_store.revoke_token(&payload.token);
    Json(json!({ "status": "ok", "message": "Sesión cerrada." }))
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
        Err(e) => return Json(json!({ "status": "error", "message": format!("Nonce inválido: {}", e) })),
    };
    match crate::auth::sign_message(&payload.private_key, &nonce_bytes) {
        Ok(signature) => Json(json!({ "status": "ok", "signature": signature })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

async fn client_check() -> impl IntoResponse {
    // FIX #1/#39: v3.0 — cliente Rust eliminado. El frontend detecta
    // la plataforma (Electron/Capacitor/Navegador) automáticamente.
    Json(json!({
        "status": "ok",
        "client_installed": true,
        "message": "IAF v3.0 usa Electron (desktop) o Capacitor (Android) como cliente.",
        "instructions": "Electron: cd electron && npm install && npm start. Capacitor: cd capacitor && .\\setup_capacitor.ps1"
    }))
}
// ============================================================================
// Endpoints Admin (gestión de usuarios)
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
    // Agregar campos calculados que el frontend espera (has_study_access, has_programming_access)
    let users_json: Vec<serde_json::Value> = users.iter().map(|u| {
        let mut v = serde_json::to_value(u).unwrap_or(json!({}));
        v["has_study_access"] = json!(u.has_study_access());
        v["has_programming_access"] = json!(u.has_programming_access());
        v
    }).collect();
    Json(json!({ "status": "ok", "users": users_json })).into_response()
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
    } else if payload.is_admin && payload.public_key.is_none() {
        // FIX #13: Error claro cuando admin no tiene clave pública
        Err("Para crear un admin necesitás generar claves (botón 'Generar Claves') o subir un .pem.".into())
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
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateAccessRequest {
    modo_estudio: Option<bool>,
    modo_programador: Option<bool>,
    editar_system_prompt_global: Option<bool>,
    editar_system_prompt_local: Option<bool>,
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
        payload.modo_estudio.unwrap_or(false),
        payload.modo_programador.unwrap_or(false),
        payload.editar_system_prompt_global.unwrap_or(false),
        payload.editar_system_prompt_local.unwrap_or(false),
    ) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateScheduleRequest {
    horarios: HashMap<String, Vec<Vec<u32>>>,
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
    // Convertir Vec<Vec<u32>> a Vec<(u32, u32)>
    let converted: HashMap<String, Vec<(u32, u32)>> = payload.horarios.into_iter()
        .map(|(day, ranges)| {
            let tuples: Vec<(u32, u32)> = ranges.into_iter()
                .filter(|v| v.len() >= 2)
                .map(|v| (v[0], v[1]))
                .collect();
            (day, tuples)
        })
        .collect();
    let schedule = WeeklySchedule { horarios: converted };
    match state.user_store.update_schedule(&username, schedule) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": e }))).into_response(),
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
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

async fn admin_delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(username): AxumPath<String>,
) -> impl IntoResponse {
    let _admin = match require_admin(&state, &headers).await {
        Ok(a) => a, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    match state.user_store.delete_user(&username) {
        Ok(()) => Json(json!({ "status": "ok" })).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({ "status": "error", "message": e }))).into_response(),
    }
}

// ============================================================================
// Endpoints de System Prompts
// ============================================================================

async fn get_global_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let prompt_path = if username == "admin_local" || state.user_store.is_admin(&username) {
        state.base_workspace.join(".config").join("data").join("admin").join("globalPrompt.json")
    } else {
        state.base_workspace.join(".config").join("data").join(&username).join("globalPrompt.json")
    };

    if prompt_path.exists() {
        if let Ok(content) = fs::read_to_string(&prompt_path) {
            return Json(json!({ "status": "ok", "content": content })).into_response();
        }
    }

    // Fallback al default
    let prompts = state.prompts.lock().unwrap();
    Json(json!({ "status": "ok", "content": prompts.global_current })).into_response()
}

#[derive(Deserialize)]
struct SavePromptRequest {
    content: String,
}

async fn save_global_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SavePromptRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = match state.user_store.find_user(&username) {
        Some(u) => u,
        None => return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Usuario no encontrado." }))).into_response(),
    };

    if !user.is_admin && !user.editar_system_prompt_global {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenés permiso para editar el prompt global." }))).into_response();
    }

    let prompt_dir = state.base_workspace.join(".config").join("data").join(&username);
    let _ = fs::create_dir_all(&prompt_dir);
    let prompt_path = prompt_dir.join("globalPrompt.json");
    let _ = fs::write(&prompt_path, &payload.content);

    // También actualizar en memoria
    let mut prompts = state.prompts.lock().unwrap();
    prompts.global_current = payload.content.clone();
    let _ = fs::write(&state.config_path, serde_json::to_string_pretty(&*prompts).unwrap());

    Json(json!({ "status": "ok" })).into_response()
}

async fn reset_global_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = match state.user_store.find_user(&username) {
        Some(u) => u,
        None => return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Usuario no encontrado." }))).into_response(),
    };

    if !user.is_admin && !user.editar_system_prompt_global {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenés permiso para editar el prompt global." }))).into_response();
    }

    let mut prompts = state.prompts.lock().unwrap();
    prompts.global_current = prompts.global_default.clone();
    let _ = fs::write(&state.config_path, serde_json::to_string_pretty(&*prompts).unwrap());

    // También actualizar archivo por usuario
    let prompt_dir = state.base_workspace.join(".config").join("data").join(&username);
    let _ = fs::create_dir_all(&prompt_dir);
    let prompt_path = prompt_dir.join("globalPrompt.json");
    let _ = fs::write(&prompt_path, &prompts.global_current);

    Json(json!({ "status": "ok", "content": prompts.global_current })).into_response()
}

async fn get_local_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(project_name): AxumPath<String>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let prompt_path = state.base_workspace.join(".config").join("data").join(&username)
        .join(&project_name).join("localPrompt.json");

    if prompt_path.exists() {
        if let Ok(content) = fs::read_to_string(&prompt_path) {
            return Json(json!({ "status": "ok", "content": content })).into_response();
        }
    }
    Json(json!({ "status": "ok", "content": "" })).into_response()
}

#[derive(Deserialize)]
struct SaveLocalPromptRequest {
    project_name: String,
    content: String,
}

async fn save_local_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveLocalPromptRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = match state.user_store.find_user(&username) {
        Some(u) => u,
        None => return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Usuario no encontrado." }))).into_response(),
    };

    if !user.is_admin && !user.editar_system_prompt_local {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenés permiso para editar prompts locales." }))).into_response();
    }

    let prompt_dir = state.base_workspace.join(".config").join("data").join(&username)
        .join(&payload.project_name);
    let _ = fs::create_dir_all(&prompt_dir);
    let prompt_path = prompt_dir.join("localPrompt.json");
    let _ = fs::write(&prompt_path, &payload.content);

    Json(json!({ "status": "ok" })).into_response()
}

// ============================================================================
// Legacy prompt endpoints (compatibilidad)
// ============================================================================

async fn legacy_prompts_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let prompts = state.prompts.lock().unwrap();
    Json(json!({
        "status": "ok",
        "global_current": prompts.global_current,
        "global_default": prompts.global_default,
        "projects": prompts.projects,
    })).into_response()
}

#[derive(Deserialize)]
struct LegacyPromptPost {
    global: Option<String>,
    local: Option<String>,
    project_name: Option<String>,
}

async fn legacy_prompts_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LegacyPromptPost>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let mut prompts = state.prompts.lock().unwrap();
    if let Some(global) = payload.global {
        prompts.global_current = global;
    }
    if let (Some(proj), Some(local)) = (payload.project_name, payload.local) {
        prompts.projects.insert(proj, local);
    }
    let _ = fs::write(&state.config_path, serde_json::to_string_pretty(&*prompts).unwrap());
    Json(json!({ "status": "ok" })).into_response()
}

async fn legacy_prompts_reset(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let mut prompts = state.prompts.lock().unwrap();
    prompts.global_current = prompts.global_default.clone();
    let _ = fs::write(&state.config_path, serde_json::to_string_pretty(&*prompts).unwrap());
    Json(json!({ "status": "ok", "content": prompts.global_current })).into_response()
}

#[derive(Deserialize)]
struct RefinePromptRequest {
    prompt: String,
    feedback: Option<String>,
    session_id: Option<String>,
    project_name: Option<String>,
}

async fn legacy_prompts_refine(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RefinePromptRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    // Refinar el prompt usando el agente (llamada rápida a DeepSeek)
    let feedback = payload.feedback.unwrap_or_default();
    let system_prompt = "Eres un refinador experto de prompts. Tu tarea es mejorar el prompt del usuario para que sea más claro, específico y efectivo. Mantén el significado original pero hazlo más estructurado. Responde ÚNICAMENTE con el prompt refinado, sin explicaciones ni markdown.";

    let refine_request = format!(
        "Prompt original: {}\n{}\nPor favor, refina este prompt para que sea más claro y efectivo.",
        payload.prompt,
        if feedback.is_empty() { String::new() } else { format!("Feedback adicional: {}", feedback) }
    );

    let api_key = deepseek_key();
    let client = reqwest::Client::new();

    match client.post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "deepseek-chat",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": refine_request}
            ],
            "max_tokens": 1024,
            "temperature": 0.3,
        }))
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(body) = resp.text().await {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(choices) = parsed["choices"].as_array() {
                        if let Some(first) = choices.first() {
                            if let Some(msg) = first["message"]["content"].as_str() {
                                return Json(json!({ "status": "ok", "refined": msg.trim() })).into_response();
                            }
                        }
                    }
                }
            }
            Json(json!({ "status": "ok", "refined": payload.prompt })).into_response()
        }
        Err(_) => Json(json!({ "status": "ok", "refined": payload.prompt })).into_response(),
    }
}

// ============================================================================
// Endpoints de Proyectos
// ============================================================================

async fn get_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(_) => {
            // Si no hay auth, devolver proyectos igual (port 80)
            let projects = state.projects.lock().unwrap();
            return Json(json!({ "status": "ok", "projects": *projects })).into_response();
        }
    };

    let projects = state.projects.lock().unwrap();
    Json(json!({ "status": "ok", "projects": *projects })).into_response()
}

#[derive(Deserialize)]
struct ForkRequest {
    repo_url: String,
}

async fn fork_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ForkRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let user = match state.user_store.find_user(&username) {
        Some(u) => u,
        None => return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Usuario no encontrado." }))).into_response(),
    };

    if !user.is_admin && !user.limits.can_fork_repos {
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenés permiso para forkear repos." }))).into_response();
    }

    // Extraer nombre del repo de la URL
    let repo_name = payload.repo_url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .split('/')
        .last()
        .unwrap_or("repo")
        .to_string();

    let project_dir = state.base_workspace.join(&repo_name);

    if project_dir.exists() {
        return Json(json!({ "status": "error", "message": format!("El directorio '{}' ya existe.", repo_name) })).into_response();
    }

    // Intentar clonar con gh
    let output = std::process::Command::new("gh")
        .args(["repo", "clone", &payload.repo_url, &project_dir.to_string_lossy().to_string()])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let mut projects = state.projects.lock().unwrap();
            projects.push(Project {
                name: repo_name.clone(),
                path: project_dir.to_string_lossy().to_string(),
                is_local: false,
            });

            let local_config = state.base_workspace.join(".config").join("local_projects.json");
            let _ = fs::write(&local_config, serde_json::to_string_pretty(&*projects).unwrap());

            Json(json!({ "status": "ok", "project": { "name": repo_name, "path": project_dir.to_string_lossy().to_string() } })).into_response()
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            Json(json!({ "status": "error", "message": format!("Error al clonar: {}", stderr) })).into_response()
        }
        Err(e) => Json(json!({ "status": "error", "message": format!("Error ejecutando gh: {}", e) })).into_response(),
    }
}

#[derive(Deserialize)]
struct AddLocalProjectRequest {
    name: String,
    path: String,
}

async fn add_local_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AddLocalProjectRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let project_path = PathBuf::from(&payload.path);
    if !project_path.exists() || !project_path.is_dir() {
        return Json(json!({ "status": "error", "message": "La ruta no existe o no es un directorio." })).into_response();
    }

    let mut projects = state.projects.lock().unwrap();

    // Verificar que no haya duplicados
    if projects.iter().any(|p| p.name == payload.name || p.path == payload.path) {
        return Json(json!({ "status": "error", "message": "Ya existe un proyecto con ese nombre o ruta." })).into_response();
    }

    projects.push(Project {
        name: payload.name.clone(),
        path: payload.path.clone(),
        is_local: true,
    });

    let local_config = state.base_workspace.join(".config").join("local_projects.json");
    let _ = fs::write(&local_config, serde_json::to_string_pretty(&*projects).unwrap());

    Json(json!({ "status": "ok", "project": { "name": payload.name, "path": payload.path } })).into_response()
}

// ============================================================================
// Endpoints de Cicles
// ============================================================================

async fn get_cicle(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(project_name): AxumPath<String>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    match state.load_cicle(&username, &project_name) {
        Some(cicle) => Json(json!({ "status": "ok", "cicle": cicle })).into_response(),
        None => Json(json!({
            "status": "ok",
            "cicle": CicleState {
                project_name,
                current_phase: CiclePhase::Implementacion,
                iteration_count: 0,
                started_at: now,
                last_updated: now,
            }
        })).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateCicleRequest {
    cicle: CicleState,
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

    let mut cicle = payload.cicle;
    cicle.project_name = project_name;

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
        return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "No tenés acceso al modo estudio." }))).into_response();
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

async fn study_get_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    Json(json!({ "status": "ok", "projects": [] })).into_response()
}

#[derive(Deserialize)]
struct StudyCreateProjectRequest {
    name: String,
    topic: String,
}

async fn study_create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<StudyCreateProjectRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    Json(json!({ "status": "ok", "project_id": uuid::Uuid::new_v4().to_string() })).into_response()
}

#[derive(Deserialize)]
struct StudyAddMemberRequest {
    username: String,
    role: String,
}

async fn study_add_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(payload): Json<StudyAddMemberRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };
    let _ = (id, payload);
    Json(json!({ "status": "ok" })).into_response()
}

#[derive(Deserialize)]
struct StudyBuildPromptRequest {
    topic: String,
    project_name: Option<String>,
}

async fn study_build_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<StudyBuildPromptRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let prompt = state.study_engine.build_study_system_prompt(&username, &payload.topic);
    Json(json!({ "status": "ok", "prompt": prompt })).into_response()
}

// ============================================================================
// Endpoints del Agente
// ============================================================================

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    project_name: Option<String>,
    session_id: Option<String>,
    mode: Option<String>,
}

async fn chat_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let is_admin = username == "admin_local" || state.user_store.is_admin(&username);

    // Verificar límites y permisos
    if !is_admin {
        let user = match state.user_store.find_user(&username) {
            Some(u) => u,
            None => return (StatusCode::FORBIDDEN, Json(json!({ "status": "error", "message": "Usuario no encontrado." }))).into_response(),
        };

        if !user.limits.activacion {
            return (StatusCode::FORBIDDEN, Json(json!({
                "status": "error",
                "message": "Tu cuenta está desactivada. Contacta al administrador."
            }))).into_response();
        }

        match payload.mode.as_deref() {
            Some("study") => {
                if !user.has_study_access() {
                    return (StatusCode::FORBIDDEN, Json(json!({
                        "status": "error",
                        "message": "No tienes permiso para usar el modo estudio. Contacta al administrador."
                    }))).into_response();
                }
            }
            Some("programming") | None => {
                if !user.has_programming_access() {
                    return (StatusCode::FORBIDDEN, Json(json!({
                        "status": "error",
                        "message": "No tienes permiso para usar el modo programador. Contacta al administrador."
                    }))).into_response();
                }
            }
            _ => {}
        }
    }
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
                title: "Nueva conversación".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            })
        } else {
            ChatSession {
                id: session_id.clone(),
                title: "Nueva conversación".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            }
        }
    } else {
        let title = "Nueva conversación".to_string();
        ChatSession {
            id: session_id.clone(),
            title,
            messages: Vec::new(),
            project_name: payload.project_name.clone(),
            steps: None,
        }
    };

    // Actualizar título si el primer mensaje del usuario cambia la conversación
    if session.messages.is_empty() {
        session.title = payload.message.chars().take(60).collect();
        if payload.message.len() > 60 {
            session.title.push_str("...");
        }
    }

    // Limpiar archivos viejos con el mismo UUID para evitar duplicados
    clean_old_chat_files(&chat_dir, &session_id);

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

    // Iniciar agente en background (BUG #4 fix)
    {
        let mut agent = state.active_agent.lock().unwrap();
        agent.current_chat_path = Some(save_path.to_string_lossy().to_string());
        if !agent.running {
            agent.running = true;
            agent.interrupted = false;
            agent.finished = false;
            agent.final_message = None;
            // BUG-002 FIX: Limpiar info_messages al iniciar nuevo agente
            agent.info_messages.clear();
            // BUG FIX: Solo limpiar steps si es conversacion NUEVA. Si es existente, cargar desde sesion.
            if chat_file.is_some() {
                if let Some(ref steps) = session.steps { agent.steps = steps.clone(); }
            } else {
                agent.steps.clear();
            }
            agent.thinking_content.clear();
            agent.esperando_respuesta_usuario = false;
            agent.respuesta_usuario = None;
            agent.esperando_aprobacion_plan = false;
            agent.plan_propuesto = None;
            agent.pregunta_usuario = None;
            agent.current_session_id = Some(session_id.clone());

            let state_bg = state.clone();
            let session_bg = session.clone();
            let sid_bg = session_id.clone();
            let uname_bg = username.clone();
            let is_admin_bg = is_admin;
            let mode_bg = payload.mode.clone().unwrap_or_else(|| "programming".to_string());
            let dk = deepseek_key().to_string();
            let vk = std::env::var("VOYAGE_API_KEY").unwrap_or_default();
            let ok = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();

            tokio::spawn(async move {
                let result = crate::agent::run_agent_loop(
                    session_bg.messages.clone(),
                    session_bg.project_name.clone(),
                    state_bg.clone(),
                    &dk, &vk, &ok,
                    Some(sid_bg.clone()),
                    &uname_bg,
                    &mode_bg,
                ).await;
                let save_p_bg = get_chat_path(&state_bg, &uname_bg, is_admin_bg, &session_bg.title, &sid_bg);
                let mut updated = if let Ok(c) = fs::read_to_string(&save_p_bg) {
                    serde_json::from_str::<ChatSession>(&c).unwrap_or_else(|_| session_bg.clone())
                } else { session_bg.clone() };

                // Guardar el mensaje final antes de que result sea movido
                let final_msg = match &result {
                    Ok(resp) => Some(resp.clone()),
                    Err(e) => Some(format!("Error: {}", e)),
                };

                match result {
                    Ok(resp) => {
                        updated.messages.push(ChatMessage {
                            role: "agent".to_string(), content: resp,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        });
                    }
                    Err(e) => {
                        updated.messages.push(ChatMessage {
                            role: "agent_error".to_string(),
                            content: format!("Error: {}", e),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        });
                    }
                }

                let steps = { let ag = state_bg.active_agent.lock().unwrap(); ag.steps.clone() };
                updated.steps = Some(steps);
                let _ = fs::write(&save_p_bg, serde_json::to_string_pretty(&updated).unwrap());

                let mut ag = state_bg.active_agent.lock().unwrap();
                ag.running = false;
                if !ag.finished { ag.finished = true; ag.final_message = final_msg; }
            });
        }
    }

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
    let mut seen_ids: HashSet<String> = HashSet::new();

    if is_admin {
        // Para admins: listar TODO (directamente en chats/ y en subdirectorios de usuarios)
        let base_chats = state.base_workspace.join(".config").join("chats");
        if let Ok(entries) = fs::read_dir(&base_chats) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    // Subdirectorio de usuario
                    let user_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    if let Ok(sub_entries) = fs::read_dir(&path) {
                        for sub in sub_entries.filter_map(Result::ok) {
                            if sub.path().extension().and_then(|e| e.to_str()) != Some("json") {
                                continue;
                            }
                            // Saltar archivos .bak
                            let fname = sub.file_name().to_string_lossy().to_string();
                            if fname.ends_with(".bak") {
                                continue;
                            }
                            if let Ok(content) = fs::read_to_string(sub.path()) {
                                if let Ok(s) = serde_json::from_str::<ChatSession>(&content) {
                                    // DEDUP: solo primera ocurrencia de cada ID
                                    if seen_ids.contains(&s.id) {
                                        continue;
                                    }
                                    seen_ids.insert(s.id.clone());
                                    summaries.push(json!({
                                        "id": s.id,
                                        "title": s.title,
                                        "project_name": s.project_name,
                                        "username": user_name,
                                        "path": sub.path().to_string_lossy(),
                                    }));
                                }
                            }
                        }
                    }
                } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    // Saltar archivos .bak
                    if fname.ends_with(".bak") {
                        continue;
                    }
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(s) = serde_json::from_str::<ChatSession>(&content) {
                            if seen_ids.contains(&s.id) {
                                continue;
                            }
                            seen_ids.insert(s.id.clone());
                            summaries.push(json!({
                                "id": s.id,
                                "title": s.title,
                                "project_name": s.project_name,
                                "username": "admin",
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
                if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.ends_with(".bak") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(s) = serde_json::from_str::<ChatSession>(&content) {
                        if seen_ids.contains(&s.id) {
                            continue;
                        }
                        seen_ids.insert(s.id.clone());
                        summaries.push(json!({
                            "id": s.id,
                            "title": s.title,
                            "project_name": s.project_name,
                            "username": username,
                            "path": entry.path().to_string_lossy(),
                        }));
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

    // Buscar primero en el directorio del usuario
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

    // Si es admin, buscar también en subdirectorios de usuarios
    if is_admin {
        let base_chats = state.base_workspace.join(".config").join("chats");
        if let Ok(entries) = fs::read_dir(&base_chats) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(sub_entries) = fs::read_dir(&path) {
                        for sub in sub_entries.filter_map(Result::ok) {
                            let fname = sub.file_name().to_string_lossy().to_string();
                            if fname.ends_with(&format!("-{}.json", id)) || fname == format!("{}.json", id) {
                                if let Ok(content) = fs::read_to_string(sub.path()) {
                                    if let Ok(session) = serde_json::from_str::<ChatSession>(&content) {
                                        return Json(json!({ "status": "ok", "session": session })).into_response();
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // También buscar en archivos directamente en root
                    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if fname.ends_with(&format!("-{}.json", id)) || fname == format!("{}.json", id) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(session) = serde_json::from_str::<ChatSession>(&content) {
                                return Json(json!({ "status": "ok", "session": session })).into_response();
                            }
                        }
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

    let versions = state.sync_store.get_file_history(&project_id, &path);
    Json(json!({ "status": "ok", "versions": versions })).into_response()
}

// ============================================================================
// Endpoints de Agente Status / Auditoría
// ============================================================================

async fn get_agent_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(_) => {
            let agent = state.active_agent.lock().unwrap();
            return Json(json!({
                "status": "ok",
                "running": agent.running,
                "active": agent.running,
                "finished": agent.finished,
                "interrupted": agent.interrupted,
                "steps": agent.steps,
                "thinking_content": agent.thinking_content,
                "info_messages": agent.info_messages,
                "final_message": agent.final_message,
                "esperando_respuesta_usuario": agent.esperando_respuesta_usuario,
                "pregunta_usuario": agent.pregunta_usuario,
                "esperando_aprobacion_plan": agent.esperando_aprobacion_plan,
                "plan_propuesto": agent.plan_propuesto,
            })).into_response();
        }
    };

    let agent = state.active_agent.lock().unwrap();
    Json(json!({
        "status": "ok",
        "running": agent.running,
        "active": agent.running,
        "finished": agent.finished,
        "interrupted": agent.interrupted,
        "steps": agent.steps,
        "thinking_content": agent.thinking_content,
        "info_messages": agent.info_messages,
        "final_message": agent.final_message,
        "esperando_respuesta_usuario": agent.esperando_respuesta_usuario,
        "pregunta_usuario": agent.pregunta_usuario,
        "esperando_aprobacion_plan": agent.esperando_aprobacion_plan,
        "plan_propuesto": agent.plan_propuesto,
    })).into_response()
}

async fn agent_steps(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(_) => {
            let agent = state.active_agent.lock().unwrap();
            return Json(json!({ "status": "ok", "steps": agent.steps })).into_response();
        }
    };
    let agent = state.active_agent.lock().unwrap();
    Json(json!({ "status": "ok", "steps": agent.steps })).into_response()
}

async fn agent_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(_) => {
            let agent = state.active_agent.lock().unwrap();
            let summary: Vec<String> = agent.steps.iter()
                .map(|s| format!("{}: {}", s.title, s.detail))
                .collect();
            return Json(json!({ "status": "ok", "summary": summary })).into_response();
        }
    };
    let agent = state.active_agent.lock().unwrap();
    let summary: Vec<String> = agent.steps.iter()
        .map(|s| format!("{}: {}", s.title, s.detail))
        .collect();
    Json(json!({ "status": "ok", "summary": summary })).into_response()
}

#[derive(Deserialize)]
struct AgentResponderRequest {
    respuesta: String,
}

async fn agent_responder(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AgentResponderRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let mut agent = state.active_agent.lock().unwrap();
    agent.respuesta_usuario = Some(payload.respuesta.clone());
    agent.esperando_respuesta_usuario = false;

    Json(json!({ "status": "ok" })).into_response()
}

#[derive(Deserialize)]
struct AgentApprovePlanRequest {
    aprobado: bool,
    feedback: Option<String>,
}

async fn agent_approve_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AgentApprovePlanRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let mut agent = state.active_agent.lock().unwrap();
    // Marcar que la aprobación fue procesada — el agente lee el estado
    agent.esperando_aprobacion_plan = false;

    // Si el usuario aprobó, desbloquear al agente
    if payload.aprobado {
        agent.respuesta_usuario = Some("PLAN_APROBADO".to_string());
    } else {
        agent.respuesta_usuario = Some(format!("PLAN_RECHAZADO: {}", payload.feedback.unwrap_or_default()));
    }

    Json(json!({ "status": "ok" })).into_response()
}

async fn agent_interrupt(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let mut agent = state.active_agent.lock().unwrap();
    agent.interrupted = true;
    agent.running = false;

    if let Some(ref path) = agent.current_chat_path {
        // Append interruption message to chat
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(mut session) = serde_json::from_str::<ChatSession>(&content) {
                session.messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: "⏹️ Agente interrumpido por el usuario.".to_string(),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
                let _ = fs::write(path, serde_json::to_string_pretty(&session).unwrap());
            }
        }
    }

    Json(json!({ "status": "ok" })).into_response()
}

// ============================================================================
// Endpoints de CAPTCHA
// ============================================================================

#[derive(Deserialize)]
struct CaptchaSolveRequest {
    id: String,
    solved_content: String,
}

async fn captcha_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(_) => {
            // Si no hay auth en puerto 80, igual devolvemos status ok pero sin captcha
            if state.port_80 {
                return Json(json!({ "status": "ok", "url": null })).into_response();
            }
            return Json(json!({ "status": "ok", "url": null })).into_response();
        }
    };

    let captcha = state.pending_captcha.lock().unwrap();
    match captcha.as_ref() {
        Some(c) => Json(json!({
            "status": "ok",
            "id": c.id,
            "url": c.url,
            "sitekey": c.sitekey,
        })).into_response(),
        None => Json(json!({ "status": "ok", "url": null })).into_response(),
    }
}

async fn captcha_solve(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CaptchaSolveRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    let mut captcha = state.pending_captcha.lock().unwrap();
    if let Some(ref mut c) = *captcha {
        if c.id == payload.id {
            c.solved_content = Some(payload.solved_content);
            return Json(json!({ "status": "ok" })).into_response();
        }
    }

    Json(json!({ "status": "error", "message": "CAPTCHA no encontrado." })).into_response()
}

// ============================================================================
// Endpoint de Reporte de Fallos
// ============================================================================

#[derive(Deserialize)]
struct ReportarFalloRequest {
    mensaje: String,
    project_name: Option<String>,
    session_id: Option<String>,
    error_details: Option<String>,
}

async fn reportar_fallo_usuario(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReportarFalloRequest>,
) -> impl IntoResponse {
    let username = match require_auth(&state, &headers).await {
        Ok(u) => u, Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    // Guardar el reporte en un archivo de logs
    let reports_dir = state.base_workspace.join(".config").join("bug_reports");
    let _ = fs::create_dir_all(&reports_dir);

    let report_id = uuid::Uuid::new_v4().to_string();
    let report_path = reports_dir.join(format!("{}.json", report_id));

    let report = json!({
        "id": report_id,
        "username": username,
        "message": payload.mensaje,
        "project_name": payload.project_name,
        "session_id": payload.session_id,
        "error_details": payload.error_details,
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
    });

    let _ = fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap());

    eprintln!("[IAF] Reporte de fallo #{} de {}: {}", report_id, username, payload.mensaje);

    Json(json!({ "status": "ok", "report_id": report_id })).into_response()
}

// ============================================================================
// Endpoints de Client Protocol
// ============================================================================

async fn client_connect(
    State(state): State<AppState>,
    Json(payload): Json<ConnectRequest>,
) -> impl IntoResponse {
    let username = match state.session_store.validate_token(&payload.token) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({ "status": "error", "message": "Token inválido." }))).into_response(),
    };

    let client_id = uuid::Uuid::new_v4().to_string();
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
            None => return (StatusCode::UNAUTHORIZED, Json(json!({ "status": "error", "message": "Token inválido." }))).into_response(),
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
        None => return (StatusCode::UNAUTHORIZED, Json(json!({ "status": "error", "message": "Token inválido." }))).into_response(),
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
        None => return (StatusCode::UNAUTHORIZED, Json(json!({ "status": "error", "message": "Token inválido." }))).into_response(),
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
// Build App Router
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
        .route("/api/scripts/:name", get(serve_script))
        // Projects & Agent
        .route("/api/projects", get(get_projects))
        .route("/api/projects/fork", post(fork_project))
        .route("/api/projects/local", post(add_local_project))
        .route("/api/agent/status", get(get_agent_status))
        .route("/api/agent/steps", get(agent_steps))
        .route("/api/agent/summary", get(agent_summary))
        .route("/api/agent/responder", post(agent_responder))
        .route("/api/agent/aprobar_plan", post(agent_approve_plan))
        .route("/api/agent/interrupt", post(agent_interrupt))
        // CAPTCHA
        .route("/api/captcha/status", get(captcha_status))
        .route("/api/captcha/solve", post(captcha_solve))
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
        // System Prompts (nuevos endpoints)
        .route("/api/prompts/global", get(get_global_prompt).post(save_global_prompt))
        .route("/api/prompts/global/reset", post(reset_global_prompt))
        .route("/api/prompts/local/:project_name", get(get_local_prompt))
        .route("/api/prompts/local", post(save_local_prompt))
        // System Prompts (legacy endpoints para compatibilidad)
        .route("/api/prompts", get(legacy_prompts_get).post(legacy_prompts_post))
        .route("/api/prompts/reset", post(legacy_prompts_reset))
        .route("/api/prompts/refine", post(legacy_prompts_refine))
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
        // Reporte de fallos por usuarios
        .route("/api/reportar-fallo", post(reportar_fallo_usuario))
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
        study_engine: StudyEngine::new(base_workspace.clone()),
        sync_store: SyncStore::new(&config_dir),
        connected_clients: Arc::new(Mutex::new(HashMap::new())),
        client_pending_requests: Arc::new(Mutex::new(HashMap::new())),
        client_responses: Arc::new(Mutex::new(HashMap::new())),
        port_80: false,
    };

    // Migrar chats existentes al nuevo formato (incluye prompts.json y local_projects.json)
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

    println!("🚀 IAF Server iniciado:");
    println!("   • Puerto 80   — Admin local (sin auth): http://{}", addr_80);
    println!("   • Puerto 8080 — Usuarios (requiere login): http://{}", addr_8080);

    let srv_80 = tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr_80).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("⚠️  No se pudo bindear puerto 80 (requiere admin): {}", e);
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
                eprintln!("❌ Error fatal bindeando puerto 8080: {}", e);
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
            eprintln!("[IAF] base_workspace vía IAF_WORKSPACE: {}", p.display());
            return p;
        }
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut candidate = exe_dir.to_path_buf();
            for _ in 0..5 {
                if candidate.join(".config").exists() || candidate.join("Cargo.toml").exists() {
                    eprintln!("[IAF] base_workspace vía exe: {}", candidate.display());
                    return candidate;
                }
                if let Some(parent) = candidate.parent() { candidate = parent.to_path_buf(); }
                else { break; }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        eprintln!("[IAF] base_workspace vía current_dir: {}", cwd.display());
        return cwd;
    }
    PathBuf::from(".")
}
