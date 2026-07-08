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

use crate::desktop::DesktopController;
use crate::agent::{discover_projects, run_agent_loop};
use crate::state::{AppState, Project, PromptConfig, ActiveAgentStatus, ProcessRegistry, ToolResultStore, SubAgentManager};

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
        // Cancelar todos los sub-agentes
        state.sub_agents.cancel_all();
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
            state.sub_agents.cancel_all();
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
    let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());

    // 4. Cancelar la tarea del agente anterior si ya estaba activa
    {
        let mut handle_opt = state.abort_handle.lock().unwrap();
        if let Some(ref handle) = *handle_opt {
            println!("Abortando agente activo anterior debido a nuevo mensaje...");
            handle.abort();
        }
        *handle_opt = None;
    }

    // Cancelar sub-agentes de sesión anterior
    state.sub_agents.cancel_all();

    // Resetear interrupción para nuevo agente
    {
        let mut status = state.active_agent.lock().unwrap();
        status.interrupted = false;
        status.esperando_respuesta_usuario = false;
        status.esperando_aprobacion_plan = false;
        status.running = false;
        status.steps = Vec::new();
        status.pregunta_usuario = None;
        status.respuesta_usuario = None;
        status.plan_propuesto = None;
        status.thinking_content = Vec::new();
        status.current_session_id = Some(session_id.clone());
    }

    let state_clone = state.clone();
    let session_id_clone = session_id.clone();

    let handle = tokio::spawn(async move {
        let result = run_agent_loop(
            Vec::new(), // Vacío, el prompt se construye internamente
            payload.project_name.clone(),
            state_clone,
            deepseek_key(),
            voyage_key(),
            openrouter_key(),
            Some(session_id_clone.clone()),
        ).await;

        match result {
            Ok(msg) => {
                println!("Bucle del agente finalizado exitosamente.");
                let chat_file = tokio::task::block_in_place(|| std::path::PathBuf::from("c:\\Users\\Fa\\Desktop\\IAF").join(".config").join("chats").join(format!("{}.json", session_id_clone)));
                if let Ok(content) = tokio::task::block_in_place(|| std::fs::read_to_string(&chat_file)) {
                    if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                        session.messages.push(crate::state::ChatMessage {
                            role: "agent".to_string(),
                            content: msg,
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        });
                        let _ = tokio::task::block_in_place(|| std::fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap()));
                    }
                }
            }
            Err(e) => {
                eprintln!("Error en bucle del agente: {}", e);
            }
        }
    });

    {
        let mut handle_opt = state.abort_handle.lock().unwrap();
        *handle_opt = Some(handle.abort_handle());
    }

    Json(json!({ "status": "ok", "session_id": session_id }))
}

// ============================================================================
// CAPTCHA Endpoints
// ============================================================================

async fn captcha_status(State(state): State<AppState>) -> impl IntoResponse {
    let captcha = state.pending_captcha.lock().unwrap().clone();
    match captcha {
        Some(req) if req.solved_content.is_none() => {
            Json(json!({ "status": "pending", "id": req.id, "url": req.url }))
        }
        _ => {
            Json(json!({ "status": "none" }))
        }
    }
}

#[derive(Deserialize)]
struct CaptchaSolveRequest {
    id: String,
    content: String,
}

async fn captcha_solve(State(state): State<AppState>, Json(payload): Json<CaptchaSolveRequest>) -> impl IntoResponse {
    let mut captcha = state.pending_captcha.lock().unwrap();
    if let Some(ref mut req) = *captcha {
        if req.id == payload.id {
            req.solved_content = Some(payload.content);
            return Json(json!({ "status": "ok" }));
        }
    }
    Json(json!({ "status": "error", "message": "Captcha no encontrado o ya resuelto." }))
}

// ============================================================================
// Desktop control handlers
// ============================================================================

#[derive(Deserialize)]
struct MoveMouseRequest { x: f64, y: f64 }

async fn move_mouse_handler(State(state): State<AppState>, Json(payload): Json<MoveMouseRequest>) -> impl IntoResponse {
    let mut desktop = state.desktop.lock().unwrap();
    desktop.move_to(payload.x, payload.y);
    Json(json!({ "status": "ok" }))
}

#[derive(Deserialize)]
struct ClickRequest { button: Option<String> }

async fn click_handler(State(state): State<AppState>, Json(payload): Json<ClickRequest>) -> impl IntoResponse {
    let mut desktop = state.desktop.lock().unwrap();
    desktop.click(&payload.button.unwrap_or_else(|| "left".to_string()));
    Json(json!({ "status": "ok" }))
}

#[derive(Deserialize)]
struct TypeTextRequest { text: String }

async fn type_text_handler(State(state): State<AppState>, Json(payload): Json<TypeTextRequest>) -> impl IntoResponse {
    let mut desktop = state.desktop.lock().unwrap();
    desktop.type_text(&payload.text);
    Json(json!({ "status": "ok" }))
}

#[derive(Deserialize)]
struct LaunchRequest { executable: String, args: Option<Vec<String>>, work_dir: Option<String> }

async fn launch_handler(State(state): State<AppState>, Json(payload): Json<LaunchRequest>) -> impl IntoResponse {
    let mut desktop = state.desktop.lock().unwrap();
    match desktop.launch(&payload.executable, payload.args.as_deref().unwrap_or(&[]), payload.work_dir.as_deref()) {
        Ok(pid) => Json(json!({ "status": "ok", "pid": pid })),
        Err(e) => Json(json!({ "status": "error", "message": e })),
    }
}

// ============================================================================
// Project management
// ============================================================================

#[derive(Deserialize)]
struct ForkRequest {
    repo_url: String,
    name: Option<String>,
}

async fn fork_project(State(state): State<AppState>, Json(payload): Json<ForkRequest>) -> impl IntoResponse {
    let repo_url = payload.repo_url.trim();
    let project_name = payload.name.unwrap_or_else(|| {
        repo_url
            .split('/')
            .last()
            .unwrap_or("repo")
            .replace(".git", "")
            .to_string()
    });

    let project_dir = state.base_workspace.join(&project_name);
    
    if project_dir.exists() {
        return Json(json!({ "status": "error", "message": format!("El proyecto '{}' ya existe localmente.", project_name) }));
    }

    // Forkear usando gh cli
    let fork_output = std::process::Command::new("gh")
        .args(&["repo", "fork", repo_url, "--clone", "--default-branch-only"])
        .current_dir(&state.base_workspace)
        .output();

    match fork_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if output.status.success() {
                let mut projs = state.projects.lock().unwrap();
                if !projs.iter().any(|p| p.name == project_name) {
                    projs.push(Project {
                        name: project_name.clone(),
                        path: project_dir.to_string_lossy().to_string(),
                        is_local: false,
                    });
                    let config_dir = state.base_workspace.join(".config");
                    let local_config_path = config_dir.join("local_projects.json");
                    let _ = fs::write(&local_config_path, serde_json::to_string_pretty(&*projs).unwrap());
                }
                Json(json!({ "status": "ok", "message": format!("Fork y clon exitoso.\nstdout: {}\nstderr: {}", stdout, stderr) }))
            } else {
                Json(json!({ "status": "error", "message": format!("Error en fork: {}\n{}", stdout, stderr) }))
            }
        }
        Err(e) => {
            Json(json!({ "status": "error", "message": format!("Error ejecutando gh: {}", e) }))
        }
    }
}

async fn get_projects(State(state): State<AppState>) -> impl IntoResponse {
    let projs = state.projects.lock().unwrap().clone();
    Json(projs)
}

#[derive(Deserialize)]
struct SavePromptsRequest {
    global: Option<String>,
    project_prompts: Option<HashMap<String, String>>,
}

async fn get_prompts(State(state): State<AppState>) -> impl IntoResponse {
    let prompts = state.prompts.lock().unwrap().clone();
    Json(json!({
        "global_default": prompts.global_default,
        "global_current": prompts.global_current,
        "projects": prompts.projects,
    }))
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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_default();

    let response = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", deepseek_key()))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "deepseek-v4-flash",
            "messages": [
                {
                    "role": "system",
                    "content": "Eres un resumidor técnico de auditorías de agentes de IA. Resume los pasos de ejecución de forma concisa pero técnicamente precisa. Agrupa acciones similares. Enumera los archivos modificados y las decisiones clave tomadas. Responde en español."
                },
                {
                    "role": "user",
                    "content": format!("Resume estos pasos de auditoría:\n\n{}", steps_text)
                }
            ]
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if let Ok(res_val) = res.json::<serde_json::Value>().await {
                let summary = res_val["choices"][0]["message"]["content"].as_str().unwrap_or("No se pudo generar resumen.").to_string();
                Json(json!({ "status": "ok", "resumen": summary }))
            } else {
                Json(json!({ "status": "error", "message": "Error decodificando respuesta." }))
            }
        }
        Err(e) => {
            Json(json!({ "status": "error", "message": format!("Error: {}", e) }))
        }
    }
}

// ============================================================================
// MAIN — Inicialización del servidor
// ============================================================================

#[tokio::main]
async fn main() {
    let base_workspace = PathBuf::from("c:\\Users\\Fa\\Desktop\\IAF");
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
    };
    // Auto-descubrir proyectos locales por defecto
    discover_projects(&state);

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .nest_service("/", ServeDir::new("public"))
        .route("/api/projects", get(get_projects))
        .route("/api/projects/fork", post(fork_project))
        .route("/api/projects/local", post(add_local_project))
        .route("/api/prompts", get(get_prompts).post(save_prompts))
        .route("/api/prompts/reset", post(reset_global_prompt))
        .route("/api/chat", post(chat_endpoint))
        .route("/api/chats", get(get_chats))
        .route("/api/chats/:id", get(get_chat_session))
        .route("/api/chats/:id/summarize_steps", post(summarize_chat_steps))
        .route("/api/agent/status", get(get_agent_status))
        .route("/api/agent/interrupt", post(interrupt_agent))
        .route("/api/agent/responder", post(respond_to_agent))
        .route("/api/agent/aprobar_plan", post(approve_agent_plan))
        .route("/api/prompts/refine", post(refine_prompt_endpoint))
        .route("/api/captcha/status", get(captcha_status))
        .route("/api/captcha/solve", post(captcha_solve))
        .nest_service("/assets/images", ServeDir::new("src/assets/images"))
        .route("/api/desktop/move", post(move_mouse_handler))
        .route("/api/desktop/click", post(click_handler))
        .route("/api/desktop/type", post(type_text_handler))
        .route("/api/desktop/launch", post(launch_handler))
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
            eprintln!("Error en el servidor de Axum: {}", e);
        }
    }
}
