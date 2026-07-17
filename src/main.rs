use axum::{
    extract::{State, Json, Path as AxumPath},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
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

use crate::desktop::DesktopController;
use crate::agent::{discover_projects, run_agent_loop};
use crate::state::{AppState, Project, PromptConfig, ActiveAgentStatus, ProcessRegistry, ToolResultStore, SubAgentManager};
use crate::auth::{UserStore, ChallengeStore, SessionStore, UserLimits};

use std::sync::OnceLock;

fn deepseek_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY no configurada"))
}

fn voyage_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("VOYAGE_API_KEY").expect("VOYAGE_API_KEY no configurada"))
}

fn openrouter_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY no configurada"))
}


const DEFAULT_GLOBAL_SYSTEM_PROMPT: &str = include_str!("../prompts/default_system_prompt.txt");


/// Detecta el directorio base de trabajo dinámicamente en tiempo de ejecución.
///
/// Estrategia de detección (en orden de prioridad):
/// 1. Variable de entorno `IAF_WORKSPACE` — permite override explícito
/// 2. Directorio del ejecutable — útil cuando se ejecuta desde build
/// 3. Directorio de trabajo actual (`std::env::current_dir()`) — fallback natural
///
/// Si ninguna opción es válida, se usa `current_dir()` sin validación adicional.
fn detect_base_workspace() -> PathBuf {
    // 1. Variable de entorno IAF_WORKSPACE (control explícito del usuario)
    if let Ok(env_ws) = std::env::var("IAF_WORKSPACE") {
        let p = PathBuf::from(&env_ws);
        if p.exists() && p.is_dir() {
            eprintln!("[IAF] base_workspace detectado vía IAF_WORKSPACE: {}", p.display());
            return p;
        }
        eprintln!("[IAF] IAF_WORKSPACE={} no existe o no es directorio, ignorando...", env_ws);
    }

    // 2. Directorio del ejecutable (busca .config/ o Cargo.toml hacia arriba)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Subir hasta encontrar un directorio con .config/ o Cargo.toml
            let mut candidate = exe_dir.to_path_buf();
            for _ in 0..5 {
                if candidate.join(".config").exists() || candidate.join("Cargo.toml").exists() {
                    eprintln!("[IAF] base_workspace detectado vía directorio del ejecutable: {}", candidate.display());
                    return candidate;
                }
                if let Some(parent) = candidate.parent() {
                    candidate = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }
    }

    // 3. Directorio de trabajo actual
    if let Ok(cwd) = std::env::current_dir() {
        eprintln!("[IAF] base_workspace detectado vía current_dir: {}", cwd.display());
        return cwd;
    }

    // 4. Fallback absoluto (no debería llegar aquí)
    let fallback = PathBuf::from(".");
    eprintln!("[IAF] ⚠️ No se pudo detectar base_workspace, usando fallback: {}", fallback.display());
    fallback
}



#[derive(Deserialize)]
pub struct LocalProjectRequest {
    pub name: String,
    pub path: String,
}

async fn add_local_project(State(state): State<AppState>, Json(payload): Json<LocalProjectRequest>) -> impl IntoResponse {
    let path = PathBuf::from(&payload.path);
    if !path.exists() || !path.is_dir() {
        return Json(json!({ "status": "error", "message": "El directorio especificado no existe o no es una carpeta válida." }));
    }

    let mut projs = state.projects.lock().unwrap();
    // Validar duplicados
    if projs.iter().any(|p| p.name == payload.name) {
        return Json(json!({ "status": "error", "message": "Ya existe un proyecto con ese nombre." }));
    }

    projs.push(Project {
        name: payload.name.clone(),
        path: payload.path.clone(),
        is_local: true,
    });

    // Guardar en la configuración local de proyectos si se desea, o persistirlo dinámicamente
    // Aquí actualizamos el archivo de prompts/config para guardar los proyectos locales
    let config_dir = state.base_workspace.join(".config");
    let local_config_path = config_dir.join("local_projects.json");
    let _ = fs::write(&local_config_path, serde_json::to_string_pretty(&*projs).unwrap());

    Json(json!({ "status": "ok" }))
}

// Historial de Chats endpoints
#[derive(Serialize, Deserialize, Clone)]
struct ChatSessionSummary {
    id: String,
    title: String,
    project_name: Option<String>,
}

async fn get_chats(State(state): State<AppState>) -> impl IntoResponse {
    let chats_dir = state.base_workspace.join(".config").join("chats");
    let mut summaries = Vec::new();
    if let Ok(entries) = fs::read_dir(chats_dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                        summaries.push(ChatSessionSummary {
                            id: session.id,
                            title: session.title,
                            project_name: session.project_name,
                        });
                    }
                }
            }
        }
    }
    Json(summaries)
}

async fn get_chat_session(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", id));
    if chat_file.exists() {
        if let Ok(content) = fs::read_to_string(chat_file) {
            if let Ok(session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                return Json(json!({ "status": "ok", "session": session }));
            }
        }
    }
    Json(json!({ "status": "error", "message": "No se encontró el chat." }))
}

// Auditoría e Interrupción endpoints
async fn get_agent_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.active_agent.lock().unwrap().clone();
    Json(json!({
        "running": status.running,
        "interrupted": status.interrupted,
        "esperando_respuesta_usuario": status.esperando_respuesta_usuario,
        "pregunta_usuario": status.pregunta_usuario,
        "esperando_aprobacion_plan": status.esperando_aprobacion_plan,
        "plan_propuesto": status.plan_propuesto,
        "thinking_content": status.thinking_content,
        "steps": status.steps,
        "current_session_id": status.current_session_id,
    }))
}

async fn interrupt_agent(State(state): State<AppState>) -> impl IntoResponse {
    let mut status = state.active_agent.lock().unwrap();
    if status.running {
        status.interrupted = true;
        status.esperando_respuesta_usuario = false;
        status.esperando_aprobacion_plan = false;
        status.steps.push(crate::state::AuditStep {
            step_type: "error".to_string(),
            title: "Interrumpido por el usuario".to_string(),
            detail: "Se envió una señal manual de interrupción.".to_string(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        });
        Json(json!({ "status": "ok", "message": "Agente marcado para interrupción." }))
    } else {
        Json(json!({ "status": "error", "message": "El agente no está corriendo." }))
    }
}

#[derive(Deserialize)]
struct RespondRequest {
    respuesta: String,
}

async fn respond_to_agent(State(state): State<AppState>, Json(payload): Json<RespondRequest>) -> impl IntoResponse {
    let mut status = state.active_agent.lock().unwrap();
    if status.esperando_respuesta_usuario {
        let respuesta = payload.respuesta.clone();
        status.respuesta_usuario = Some(payload.respuesta);
        status.esperando_respuesta_usuario = false;
        
        // Guardar la respuesta del usuario en el archivo JSON de la conversación
        if let Some(ref session_id) = status.current_session_id {
            let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", session_id));
            if chat_file.exists() {
                if let Ok(content) = fs::read_to_string(&chat_file) {
                    if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                        session.messages.push(crate::state::ChatMessage {
                            role: "user".to_string(),
                            content: respuesta,
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        });
                        let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                    }
                }
            }
        }
        
        Json(json!({ "status": "ok", "message": "Respuesta enviada al agente." }))
    } else {
        Json(json!({ "status": "error", "message": "El agente no está esperando respuesta." }))
    }
}

#[derive(Deserialize)]
struct ApprovePlanRequest {
    aprobado: bool,
}

async fn approve_agent_plan(State(state): State<AppState>, Json(payload): Json<ApprovePlanRequest>) -> impl IntoResponse {
    let mut status = state.active_agent.lock().unwrap();
    if status.esperando_aprobacion_plan {
        status.esperando_aprobacion_plan = false;
        if payload.aprobado {
            status.steps.push(crate::state::AuditStep {
                step_type: "thinking".to_string(),
                title: "Plan Aprobado".to_string(),
                detail: "El usuario aprobó el plan propuesto. Continuando...".to_string(),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
        } else {
            status.interrupted = true;
            status.steps.push(crate::state::AuditStep {
                step_type: "error".to_string(),
                title: "Plan Rechazado".to_string(),
                detail: "El usuario rechazó el plan. Ejecución cancelada.".to_string(),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
        }
        Json(json!({ "status": "ok" }))
    } else {
        Json(json!({ "status": "error", "message": "El agente no está esperando aprobación de plan." }))
    }
}

#[derive(Deserialize)]
struct RefinePromptRequest {
    prompt: String,
    feedback: Option<String>,
    session_id: Option<String>,
    project_name: Option<String>,
}

async fn refine_prompt_endpoint(State(state): State<AppState>, Json(payload): Json<RefinePromptRequest>) -> impl IntoResponse {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build() {
            Ok(c) => c,
            Err(e) => return Json(json!({ "status": "error", "message": format!("Error creando cliente HTTP: {}", e) })),
        };
    
    // Obtener System Prompt Global Actual
    let global_prompt = {
        let prompts = state.prompts.lock().unwrap();
        prompts.global_current.clone()
    };

    // Obtener System Prompt Local si existe para este proyecto
    let local_prompt = payload.project_name.as_ref().and_then(|name| {
        let prompts = state.prompts.lock().unwrap();
        prompts.projects.get(name).cloned()
    });

    let system_prompt_context = if let Some(ref local) = local_prompt {
        format!("{}\n\n[PROMPT LOCAL DEL PROYECTO ACTIVO]:\n{}", global_prompt, local)
    } else {
        global_prompt
    };

    // Obtener Memorias locales del archivo MEMORIES.md del proyecto si existe
    let mut memories_content = "No hay archivo MEMORIES.md registrado en este proyecto aún.".to_string();
    if let Some(ref proj_name) = payload.project_name {
        // Buscar ruta física de la carpeta del proyecto
        let projs = state.projects.lock().unwrap();
        let proj_path_opt = projs.iter().find(|p| p.name == *proj_name).map(|p| p.path.clone());
        let final_proj_path = proj_path_opt.unwrap_or_else(|| state.base_workspace.join(proj_name).to_string_lossy().to_string());
        
        let memories_path = std::path::Path::new(&final_proj_path).join("MEMORIES.md");
        if memories_path.exists() {
            if let Ok(content) = fs::read_to_string(memories_path) {
                memories_content = content;
            }
        }
    } else {
        // Comprobar si existe en la raíz base_workspace por defecto
        let memories_path = state.base_workspace.join("MEMORIES.md");
        if memories_path.exists() {
            if let Ok(content) = fs::read_to_string(memories_path) {
                memories_content = content;
            }
        }
    }

    let refine_system_prompt = format!("Eres un refinador experto en prompts de IA. Tu objetivo es estructurar, mejorar y corregir prompts.
Debes mantener estrictamente el formato estructurado en 5 bloques en español:
1. Rol y Contexto (Rol de programador principal en Rust/JS/HTML).
2. Meta Técnica Rígida (Qué se quiere hacer exactamente).
3. Restricciones y Reglas (Prohibido asumir, prohibido crear APIs externas innecesarias, código optimizado obligatoriamente para correr en un Pentium de 4GB RAM y 2 cores).
4. Formato de Salida (Código limpio, comentarios inline).
5. Datos de Soporte (Mencionar archivos relevantes).

Se te provee el SYSTEM PROMPT global y local del proyecto que guiará al agente principal, junto a las MEMORIAS locales persistentes de limitaciones técnicas del proyecto:

---
**[SYSTEM PROMPT DEL AGENTE PRINCIPAL (GLOBAL + LOCAL)]**
{}
---

---
**[MEMORIAS DEL PROYECTO ACTUAL (MEMORIES.md)]**
{}
---

Si el usuario te provee un prompt base y una retroalimentación/instrucción adicional de ajuste, debes aplicarla sobre el prompt base y devolver el prompt final estructurado entero.
Adicionalmente, se te inyectará el historial reciente del chat para que entiendas de qué elementos o archivos (como 'el botón azul') se venía hablando en mensajes anteriores, de modo que el prompt refinado mantenga la coherencia total. No agregues introducciones ni explicaciones; empieza directamente con el prompt final estructurado.", system_prompt_context, memories_content);

    let mut api_messages = vec![
        json!({ "role": "system", "content": refine_system_prompt }),
    ];

    // Cargar historial de chat si session_id está presente para dar contexto al refinador
    if let Some(ref s_id) = payload.session_id {
        let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", s_id));
        if chat_file.exists() {
            if let Ok(content) = fs::read_to_string(&chat_file) {
                if let Ok(session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                    // Tomar los últimos 6 mensajes para no saturar el contexto de refinado
                    let start_idx = session.messages.len().saturating_sub(6);
                    for m in &session.messages[start_idx..] {
                        let role = if m.role == "agent" { "assistant" } else { "user" };
                        // Sanitizar para evitar meter el reporte de auditoría completo
                        let clean_content = if m.content.contains("**[Auditoría de Herramientas Ejecutadas]**") {
                            m.content.split("**[Auditoría de Herramientas Ejecutadas]**").next().unwrap_or("").trim().to_string()
                        } else {
                            m.content.clone()
                        };
                        api_messages.push(json!({ "role": role, "content": clean_content }));
                    }
                }
            }
        }
    }

    api_messages.push(json!({ "role": "user", "content": format!("Prompt base a refinar:\n```\n{}\n```", payload.prompt) }));

    if let Some(ref fb) = payload.feedback {
        if !fb.trim().is_empty() {
            api_messages.push(json!({ "role": "user", "content": format!("Instrucción adicional de modificación:\n```\n{}\n```", fb) }));
        }
    }

    let response = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", deepseek_key()))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "deepseek-v4-flash",
            "messages": api_messages
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if let Ok(res_val) = res.json::<serde_json::Value>().await {
                let refined = res_val["choices"][0]["message"]["content"].as_str().unwrap_or(&payload.prompt).to_string();
                Json(json!({ "status": "ok", "refined": refined }))
            } else {
                Json(json!({ "status": "error", "message": "Error decodificando respuesta de refinación." }))
            }
        }
        Err(e) => {
            Json(json!({ "status": "error", "message": format!("Error en llamada de refinamiento: {}", e) }))
        }
    }
}

#[derive(Deserialize)]
struct ChatInput {
    message: String,
    project_name: Option<String>,
    session_id: Option<String>, // Para continuar chat o iniciar uno nuevo
}

async fn chat_endpoint(State(state): State<AppState>, Json(payload): Json<ChatInput>) -> impl IntoResponse {
    // 1. Determinar el Session ID
    let session_id = payload.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let chats_dir = state.base_workspace.join(".config").join("chats");
    let _ = fs::create_dir_all(&chats_dir);
    let chat_file = chats_dir.join(format!("{}.json", session_id));

    // 2. Cargar sesión existente o crear una nueva
    let mut session = if chat_file.exists() {
        if let Ok(content) = fs::read_to_string(&chat_file) {
            serde_json::from_str::<crate::state::ChatSession>(&content).unwrap_or_else(|_| crate::state::ChatSession {
                id: session_id.clone(),
                title: "Nueva conversación".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            })
        } else {
            crate::state::ChatSession {
                id: session_id.clone(),
                title: "Nueva conversación".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            }
        }
    } else {
        // Generar título descriptivo conciso usando DeepSeek V4 Flash
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        let prompt_title = format!(
            "Analiza el siguiente mensaje de usuario y genera un título descriptivo muy corto (máximo 4 palabras) en español que resuma el tema. No agregues comillas ni explicaciones:\n\n\"{}\"",
            payload.message
        );
        
        let mut generated_title = payload.message.chars().take(28).collect::<String>();
        
        let response_title = client
            .post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", deepseek_key()))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": "deepseek-v4-flash",
                "messages": [
                    { "role": "user", "content": prompt_title }
                ]
            }))
            .send()
            .await;

        if let Ok(res) = response_title {
            if let Ok(res_val) = res.json::<serde_json::Value>().await {
                if let Some(content) = res_val["choices"][0]["message"]["content"].as_str() {
                    let clean_title = content.trim().replace("\"", "").replace("'", "");
                    if !clean_title.is_empty() {
                        generated_title = clean_title;
                    }
                }
            }
        }

        crate::state::ChatSession {
            id: session_id.clone(),
            title: generated_title,
            messages: Vec::new(),
            project_name: payload.project_name.clone(),
            steps: None,
        }
    };

    // 3. Guardar el nuevo mensaje del usuario
    let user_msg = crate::state::ChatMessage {
        role: "user".to_string(),
        content: payload.message.clone(),
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
    };
    session.messages.push(user_msg);
    // Guardar la sesión al disco antes de invocar al agente
    let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());

    // 4. Bloquear para evitar concurrencia
    {
        let mut status = state.active_agent.lock().unwrap();
        if status.running {
            return Json(json!({ "status": "error", "message": "El agente ya está corriendo. Interrúmpelo antes de continuar." }));
        }
        // Resetear estado
        *status = ActiveAgentStatus::default();
        status.running = true;
        status.current_session_id = Some(session_id.clone());
    }

    // 5. Lanzar el agente en un task separado
    let state_clone = state.clone();
    let project_name = payload.project_name.clone();
    let messages_clone = session.messages.clone();
    let session_clone = session.clone();
    let chat_file_clone = chat_file.clone();

    let handle = tokio::spawn(async move {
        let result = run_agent_loop(
            messages_clone,
            project_name,
            state_clone.clone(),
            deepseek_key(),
            voyage_key(),
            openrouter_key(),
            Some(session_id.clone()),
        ).await;

        // Guardar resultado final
        if let Ok(ref final_msg) = result {
            let response_msg = crate::state::ChatMessage {
                role: "agent".to_string(),
                content: final_msg.clone(),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            };
            if let Ok(content) = fs::read_to_string(&chat_file_clone) {
                if let Ok(mut s) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                    s.messages.push(response_msg);
                    // Guardar pasos de auditoría si existen
                    let steps = {
                        let status = state_clone.active_agent.lock().unwrap();
                        if !status.steps.is_empty() {
                            Some(status.steps.clone())
                        } else {
                            None
                        }
                    };
                    s.steps = steps;
                    let _ = fs::write(&chat_file_clone, serde_json::to_string_pretty(&s).unwrap());
                }
            }
        }

        // Limpiar estado del agente
        {
            let mut status = state_clone.active_agent.lock().unwrap();
            status.running = false;
        }
        // Matar procesos hijo
        state_clone.process_registry.kill_all();

        // Reap tool results antiguos
        state_clone.tool_results.reap_old(1800);
        // Reap sub-agentes completados
        state_clone.sub_agents.reap_old(1800);
        // Reap sesiones expiradas
        state_clone.session_store.reap();
        // Reap challenges expirados
        state_clone.challenge_store.reap();

        result
    });

    // Guardar el AbortHandle para interrupción
    {
        let mut ah = state.abort_handle.lock().unwrap();
        *ah = Some(handle.abort_handle());
    }

    Json(json!({ "status": "ok", "session_id": session.id, "title": session.title }))
}

// ============================================================================
// AUTH ENDPOINTS — Autenticación Ed25519 Challenge-Response
// ============================================================================

/// Paso 1: Solicitar un challenge (nonce) para firmar.
#[derive(Deserialize)]
struct ChallengeRequest {
    username: String,
}

async fn auth_challenge(
    State(state): State<AppState>,
    Json(payload): Json<ChallengeRequest>,
) -> impl IntoResponse {
    let user = state.user_store.find_user(&payload.username);
    if user.is_none() {
        return Json(json!({
            "status": "error",
            "message": "Usuario no encontrado."
        }));
    }

    let nonce = state.challenge_store.generate_challenge(&payload.username);
    Json(json!({
        "status": "ok",
        "nonce": nonce,
        "message": "Firma este nonce con tu clave privada Ed25519 y envíalo a /api/auth/verify."
    }))
}

/// Paso 2: Verificar la firma del challenge.
#[derive(Deserialize)]
struct VerifyRequest {
    username: String,
    nonce: String,
    signature: String,
}

async fn auth_verify(
    State(state): State<AppState>,
    Json(payload): Json<VerifyRequest>,
) -> impl IntoResponse {
    let user = match state.user_store.find_user(&payload.username) {
        Some(u) => u,
        None => {
            return Json(json!({
                "status": "error",
                "message": "Usuario no encontrado."
            }));
        }
    };

    match state.challenge_store.verify_challenge(
        &payload.username,
        &payload.nonce,
        &payload.signature,
        &user.public_key,
    ) {
        Ok(true) => {
            let token = state.session_store.create_session(&payload.username);
            Json(json!({
                "status": "ok",
                "token": token,
                "username": payload.username,
                "is_admin": user.is_admin,
                "permissions": user.permissions,
                "limits": user.limits,
                "message": "Autenticación exitosa."
            }))
        }
        Ok(false) => {
            Json(json!({
                "status": "error",
                "message": "Firma inválida. El nonce no coincide o la firma es incorrecta."
            }))
        }
        Err(e) => {
            Json(json!({
                "status": "error",
                "message": format!("Error de verificación: {}", e)
            }))
        }
    }
}

/// Logout: invalida el token de sesión.
#[derive(Deserialize)]
struct LogoutRequest {
    token: String,
}

async fn auth_logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> impl IntoResponse {
    let revoked = state.session_store.revoke_token(&payload.token);
    if revoked {
        Json(json!({ "status": "ok", "message": "Sesión cerrada." }))
    } else {
        Json(json!({ "status": "error", "message": "Token no encontrado o ya expirado." }))
    }
}

// ============================================================================
// ADMIN USER MANAGEMENT ENDPOINTS
// ============================================================================

/// Helper: extrae y valida el token del header Authorization: Bearer <token>
/// Retorna Some(username) si el token es válido, o None si no.
fn extract_auth_user(state: &AppState, auth_header: &Option<String>) -> Option<String> {
    let header = auth_header.as_ref()?;
    let token = header.strip_prefix("Bearer ")?;
    state.session_store.validate_token(token)
}

/// Helper: verifica que el usuario autenticado es admin.
/// Retorna Ok(()) si es admin, Err(response) si no.
fn require_admin(state: &AppState, auth_header: &Option<String>) -> Result<String, axum::Json<serde_json::Value>> {
    let username = extract_auth_user(state, auth_header)
        .ok_or_else(|| Json(json!({ "status": "error", "message": "No autenticado. Usa /api/auth/verify primero." })))?;

    if !state.user_store.is_admin(&username) {
        return Err(Json(json!({ "status": "error", "message": "Acceso denegado. Se requieren permisos de administrador." })));
    }

    Ok(username)
}

/// GET /api/admin/users — Listar todos los usuarios (solo admin).
async fn admin_list_users(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    if let Err(e) = require_admin(&state, &auth_header) {
        return e;
    }

    let users = state.user_store.list_users();
    Json(json!({ "status": "ok", "users": users }))
}

/// POST /api/admin/users — Crear un nuevo usuario (solo admin).
#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    public_key: String,
    #[serde(default)]
    is_admin: bool,
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(default)]
    limits: Option<UserLimits>,
}

async fn admin_create_user(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    if let Err(e) = require_admin(&state, &auth_header) {
        return e;
    }

    let limits = payload.limits.unwrap_or_else(|| {
        if payload.is_admin {
            UserLimits::admin()
        } else {
            UserLimits::default()
        }
    });

    match state.user_store.create_user(
        &payload.username,
        &payload.public_key,
        payload.is_admin,
        payload.permissions,
        limits,
    ) {
        Ok(account) => Json(json!({ "status": "ok", "user": account })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

/// PUT /api/admin/users/:username/limits — Actualizar límites de un usuario (solo admin).
#[derive(Deserialize)]
struct UpdateLimitsRequest {
    limits: UserLimits,
}

async fn admin_update_limits(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    AxumPath(username): AxumPath<String>,
    Json(payload): Json<UpdateLimitsRequest>,
) -> impl IntoResponse {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    if let Err(e) = require_admin(&state, &auth_header) {
        return e;
    }

    match state.user_store.update_limits(&username, payload.limits) {
        Ok(()) => Json(json!({ "status": "ok", "message": format!("Límites de '{}' actualizados.", username) })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

/// PUT /api/admin/users/:username/permissions — Actualizar permisos (solo admin).
#[derive(Deserialize)]
struct UpdatePermissionsRequest {
    permissions: Vec<String>,
}

async fn admin_update_permissions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    AxumPath(username): AxumPath<String>,
    Json(payload): Json<UpdatePermissionsRequest>,
) -> impl IntoResponse {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    if let Err(e) = require_admin(&state, &auth_header) {
        return e;
    }

    match state.user_store.update_permissions(&username, payload.permissions) {
        Ok(()) => Json(json!({ "status": "ok", "message": format!("Permisos de '{}' actualizados.", username) })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

/// PUT /api/admin/users/:username/key — Cambiar clave pública de un usuario (solo admin).
#[derive(Deserialize)]
struct UpdateKeyRequest {
    public_key: String,
}

async fn admin_update_key(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    AxumPath(username): AxumPath<String>,
    Json(payload): Json<UpdateKeyRequest>,
) -> impl IntoResponse {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    if let Err(e) = require_admin(&state, &auth_header) {
        return e;
    }

    match state.user_store.update_public_key(&username, &payload.public_key) {
        Ok(()) => Json(json!({ "status": "ok", "message": format!("Clave pública de '{}' actualizada.", username) })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

/// DELETE /api/admin/users/:username — Eliminar un usuario (solo admin).
async fn admin_delete_user(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    AxumPath(username): AxumPath<String>,
) -> impl IntoResponse {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    if let Err(e) = require_admin(&state, &auth_header) {
        return e;
    }

    // No permitir auto-eliminarse
    if let Some(auth_user) = extract_auth_user(&state, &auth_header) {
        if auth_user == username {
            return Json(json!({ "status": "error", "message": "No puedes eliminar tu propia cuenta de administrador." }));
        }
    }

    match state.user_store.delete_user(&username) {
        Ok(()) => Json(json!({ "status": "ok", "message": format!("Usuario '{}' eliminado.", username) })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

/// POST /api/auth/keygen — Generar un nuevo par de claves Ed25519.
/// (Endpoint auxiliar para setup inicial. El cliente NUNCA debe compartir la clave privada.)
async fn auth_keygen() -> impl IntoResponse {
    let (private_hex, public_hex) = crate::auth::generate_keypair();
    Json(json!({
        "status": "ok",
        "private_key": private_hex,
        "public_key": public_hex,
        "warning": "⚠️ GUARDA TU CLAVE PRIVADA DE FORMA SEGURA. NUNCA LA COMPARTAS. La clave pública es la que se registra en el sistema."
    }))
}


// ============================================================================
// MAIN
// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() {
    let config_dir = base_workspace.join(".config");
    fs::create_dir_all(&config_dir).unwrap_or_default();
    let _ = fs::create_dir_all(config_dir.join("chats"));
    
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

    // Inicializar lista de proyectos descubiertos
    let mut initial_projects = Vec::new();
    let local_config_path = config_dir.join("local_projects.json");
    if local_config_path.exists() {
        if let Ok(content) = fs::read_to_string(&local_config_path) {
            if let Ok(parsed) = serde_json::from_str::<Vec<Project>>(&content) {
                initial_projects = parsed;
            }
        }
    }

    // Inicializar stores de autenticación
    let user_store = UserStore::load(&config_dir);
    let challenge_store = ChallengeStore::new(300); // 5 minutos TTL
    let session_store = SessionStore::new();

    // Verificar si hay usuarios. Si no, mostrar advertencia.
    {
        let users = user_store.list_users();
        if users.is_empty() {
            eprintln!("[IAF] ⚠️  ADVERTENCIA: No hay usuarios configurados.");
            eprintln!("[IAF] ⚠️  Usa POST /api/auth/keygen para generar un par de claves.");
            eprintln!("[IAF] ⚠️  Luego edita manualmente .config/users.json para agregar tu cuenta admin,");
            eprintln!("[IAF] ⚠️  o usa el endpoint POST /api/admin/users (requiere auth bootstrap).");
        } else {
            eprintln!("[IAF] ✅ {} usuario(s) cargado(s) desde users.json", users.len());
        }
    }

    let state = AppState {
        config_path,
        prompts: Arc::new(Mutex::new(prompts)),
        projects: Arc::new(Mutex::new(initial_projects)),
        base_workspace,
        pending_captcha: Arc::new(Mutex::new(None)),
        active_agent: Arc::new(Mutex::new(ActiveAgentStatus::default())),
        abort_handle: Arc::new(Mutex::new(None)),
        desktop: Arc::new(Mutex::new(DesktopController::new())),
        image_store: Arc::new(Mutex::new(HashMap::new())),
        context_store: Arc::new(Mutex::new(HashMap::new())),
        process_registry: ProcessRegistry::new(),
        tool_results: ToolResultStore::new(),
        sub_agents: SubAgentManager::new(),
        user_store,
        challenge_store,
        session_store,
    };
    // Auto-descubrir proyectos locales por defecto
    discover_projects(&state);

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .nest_service("/", ServeDir::new("public"))
        // Proyectos
        .route("/api/projects", get(get_projects))
        .route("/api/projects/fork", post(fork_project))
        .route("/api/projects/local", post(add_local_project))
        // Prompts
        .route("/api/prompts", get(get_prompts).post(save_prompts))
        .route("/api/prompts/reset", post(reset_global_prompt))
        .route("/api/prompts/refine", post(refine_prompt_endpoint))
        // Chat
        .route("/api/chat", post(chat_endpoint))
        .route("/api/chats", get(get_chats))
        .route("/api/chats/:id", get(get_chat_session))
        .route("/api/chats/:id/summarize_steps", post(summarize_chat_steps))
        // Agente
        .route("/api/agent/status", get(get_agent_status))
        .route("/api/agent/interrupt", post(interrupt_agent))
        .route("/api/agent/responder", post(respond_to_agent))
        .route("/api/agent/aprobar_plan", post(approve_agent_plan))
        // CAPTCHA
        .route("/api/captcha/status", get(captcha_status))
        .route("/api/captcha/solve", post(captcha_solve))
        // Desktop
        .route("/api/desktop/move", post(move_mouse_handler))
        .route("/api/desktop/click", post(click_handler))
        .route("/api/desktop/type", post(type_text_handler))
        .route("/api/desktop/launch", post(launch_handler))
        // Auth — Challenge-Response
        .route("/api/auth/challenge", post(auth_challenge))
        .route("/api/auth/verify", post(auth_verify))
        .route("/api/auth/logout", post(auth_logout))
        .route("/api/auth/keygen", get(auth_keygen))
        // Admin — Gestión de usuarios
        .route("/api/admin/users", get(admin_list_users).post(admin_create_user))
        .route("/api/admin/users/:username/limits", axum::routing::put(admin_update_limits))
        .route("/api/admin/users/:username/permissions", axum::routing::put(admin_update_permissions))
        .route("/api/admin/users/:username/key", axum::routing::put(admin_update_key))
        .route("/api/admin/users/:username", axum::routing::delete(admin_delete_user))
        // Assets
        .nest_service("/assets/images", ServeDir::new("src/assets/images"))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error fatal: No se pudo enlazar (bind) al puerto {}: {}", addr, e);
            std::process::exit(1);
        }
    };
    println!("Servidor Agent-First iniciado en http://{}", addr);
    
    match axum::serve(listener, app).await {
        Ok(_) => {
            println!("El servidor de Axum se detuvo de forma limpia (Ok).");
        }
        Err(e) => {
            eprintln!("El servidor de Axum terminó con un error: {}", e);
        }
    }
}

// ============================================================================
// Handlers auxiliares (sin cambios)
// ============================================================================

async fn get_projects(State(state): State<AppState>) -> impl IntoResponse {
    let projs = state.projects.lock().unwrap().clone();
    Json(projs)
}

#[derive(Deserialize)]
struct ForkRequest {
    repo_url: String,
}

async fn fork_project(State(state): State<AppState>, Json(payload): Json<ForkRequest>) -> impl IntoResponse {
    let output = std::process::Command::new("gh")
        .args(&["repo", "fork", &payload.repo_url, "--clone"])
        .current_dir(&state.base_workspace)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            discover_projects(&state);
            Json(json!({ "status": "ok", "stdout": stdout, "stderr": stderr }))
        }
        Err(e) => {
            Json(json!({ "status": "error", "message": format!("Error corriendo gh CLI: {}", e) }))
        }
    }
}

async fn get_prompts(State(state): State<AppState>) -> impl IntoResponse {
    let prompts = state.prompts.lock().unwrap().clone();
    Json(json!({
        "global_default": prompts.global_default,
        "global_current": prompts.global_current,
        "projects": prompts.projects,
    }))
}

#[derive(Deserialize)]
struct SavePromptsRequest {
    global: Option<String>,
    project_prompts: Option<HashMap<String, String>>,
}

async fn save_prompts(State(state): State<AppState>, Json(payload): Json<SavePromptsRequest>) -> impl IntoResponse {
    let mut prompts = state.prompts.lock().unwrap();
    if let Some(ref new_global) = payload.global {
        prompts.global_current = new_global.clone();
    }
    if let Some(project_map) = &payload.project_prompts {
        for (name, prompt) in project_map {
            prompts.projects.insert(name.clone(), prompt.clone());
        }
    }

    let config_path = state.config_path.clone();
    let _ = fs::write(&config_path, serde_json::to_string_pretty(&*prompts).unwrap());
    Json(json!({ "status": "ok" }))
}

async fn reset_global_prompt(State(state): State<AppState>) -> impl IntoResponse {
    let mut prompts = state.prompts.lock().unwrap();
    prompts.global_current = prompts.global_default.clone();
    let config_path = state.config_path.clone();
    let _ = fs::write(&config_path, serde_json::to_string_pretty(&*prompts).unwrap());
    Json(json!({ "status": "ok", "message": "Prompt global restaurado al valor por defecto." }))
}

async fn summarize_chat_steps(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", id));
    if !chat_file.exists() {
        return Json(json!({ "status": "error", "message": "Chat no encontrado." }));
    }

    let session = match std::fs::read_to_string(&chat_file)
        .ok()
        .and_then(|c| serde_json::from_str::<crate::state::ChatSession>(&c).ok())
    {
        Some(s) => s,
        None => return Json(json!({ "status": "error", "message": "No se pudo leer la sesión." })),
    };

    let steps = session.steps.unwrap_or_default();
    if steps.is_empty() {
        return Json(json!({ "status": "ok", "resumen": "No hay pasos registrados para esta conversación." }));
    }

    let steps_text: String = steps.iter()
        .map(|s| format!("[{}] {}: {}", s.step_type, s.title, s.detail))
        .collect::<Vec<_>>()
        .join("\n");

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Json(json!({ "status": "error", "message": format!("Error creando cliente HTTP: {}", e) })),
    };

    let response = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", deepseek_key()))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "deepseek-v4-flash",
            "messages": [
                {
                    "role": "system",
                    "content": "Eres un auditor técnico experto. Resume el siguiente registro de pasos de ejecución de un agente de desarrollo en español, en formato markdown conciso. Incluye: 1) Qué hizo el agente, 2) Por qué, 3) Qué falta por hacer."
                },
                {
                    "role": "user",
                    "content": format!("Registro de pasos a resumir:\n\n{}", steps_text)
                }
            ]
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if let Ok(res_val) = res.json::<serde_json::Value>().await {
                let summary = res_val["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("No se pudo generar resumen.")
                    .to_string();
                Json(json!({ "status": "ok", "resumen": summary }))
            } else {
                Json(json!({ "status": "error", "message": "Error decodificando respuesta." }))
            }
        }
        Err(e) => Json(json!({ "status": "error", "message": format!("Error de conexión: {}", e) })),
    }
}

// ============================================================================
// CAPTCHA Handlers
// ============================================================================

async fn captcha_status(State(state): State<AppState>) -> impl IntoResponse {
    let captcha = state.pending_captcha.lock().unwrap();
    if let Some(ref c) = *captcha {
        Json(json!({
            "pending": true,
            "id": c.id,
            "sitekey": c.sitekey,
            "url": c.url,
        }))
    } else {
        Json(json!({ "pending": false }))
    }
}

#[derive(Deserialize)]
struct CaptchaSolveRequest {
    id: String,
    solved_content: String,
}

async fn captcha_solve(State(state): State<AppState>, Json(payload): Json<CaptchaSolveRequest>) -> impl IntoResponse {
    let mut captcha = state.pending_captcha.lock().unwrap();
    if let Some(ref mut c) = *captcha {
        if c.id == payload.id {
            c.solved_content = Some(payload.solved_content);
            Json(json!({ "status": "ok", "message": "CAPTCHA resuelto." }))
        } else {
            Json(json!({ "status": "error", "message": "ID de CAPTCHA no coincide." }))
        }
    } else {
        Json(json!({ "status": "error", "message": "No hay CAPTCHA pendiente." }))
    }
}

// ============================================================================
// Desktop Handlers
// ============================================================================

#[derive(Deserialize)]
struct MoveMouseRequest {
    x: f64,
    y: f64,
}

async fn move_mouse_handler(State(state): State<AppState>, Json(payload): Json<MoveMouseRequest>) -> impl IntoResponse {
    let desktop = state.desktop.lock().unwrap();
    match desktop.move_mouse(payload.x, payload.y) {
        Ok(()) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

#[derive(Deserialize)]
struct ClickRequest {
    button: Option<String>,
}

async fn click_handler(State(state): State<AppState>, Json(payload): Json<ClickRequest>) -> impl IntoResponse {
    let desktop = state.desktop.lock().unwrap();
    let button = payload.button.unwrap_or_else(|| "left".to_string());
    match desktop.click(&button) {
        Ok(()) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

#[derive(Deserialize)]
struct TypeTextRequest {
    text: String,
}

async fn type_text_handler(State(state): State<AppState>, Json(payload): Json<TypeTextRequest>) -> impl IntoResponse {
    let desktop = state.desktop.lock().unwrap();
    match desktop.type_text(&payload.text) {
        Ok(()) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

#[derive(Deserialize)]
struct LaunchRequest {
    executable: String,
    args: Option<Vec<String>>,
}

async fn launch_handler(State(state): State<AppState>, Json(payload): Json<LaunchRequest>) -> impl IntoResponse {
    let desktop = state.desktop.lock().unwrap();
    match desktop.launch(&payload.executable, payload.args.unwrap_or_default()) {
        Ok(()) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}
