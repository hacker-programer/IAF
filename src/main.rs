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

use crate::desktop::DesktopController;
use crate::agent::{discover_projects, run_agent_loop};
use crate::state::{AppState, Project, PromptConfig, ActiveAgentStatus, ProcessRegistry};

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
        return Json(json!({ "status": "error", "message": "El directorio especificado no existe o no es una carpeta vÃ¡lida." }));
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

    // Guardar en la configuraciÃ³n local de proyectos si se desea, o persistirlo dinÃ¡micamente
    // AquÃ­ actualizamos el archivo de prompts/config para guardar los proyectos locales
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
    Json(json!({ "status": "error", "message": "No se encontrÃ³ el chat." }))
}

// AuditorÃ­a e InterrupciÃ³n endpoints
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
            detail: "Se enviÃ³ una seÃ±al manual de interrupciÃ³n.".to_string(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        });
        Json(json!({ "status": "ok", "message": "Agente marcado para interrupciÃ³n." }))
    } else {
        Json(json!({ "status": "error", "message": "El agente no estÃ¡ corriendo." }))
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
        
        // Guardar la respuesta del usuario en el archivo JSON de la conversaciÃ³n
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
        Json(json!({ "status": "error", "message": "El agente no estÃ¡ esperando respuesta." }))
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
                detail: "El usuario aprobÃ³ el plan propuesto. Continuando...".to_string(),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
        } else {
            status.interrupted = true;
            status.steps.push(crate::state::AuditStep {
                step_type: "error".to_string(),
                title: "Plan Rechazado".to_string(),
                detail: "El usuario rechazÃ³ el plan. EjecuciÃ³n cancelada.".to_string(),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
        }
        Json(json!({ "status": "ok" }))
    } else {
        Json(json!({ "status": "error", "message": "El agente no estÃ¡ esperando aprobaciÃ³n de plan." }))
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
    let mut memories_content = "No hay archivo MEMORIES.md registrado en este proyecto aÃºn.".to_string();
    if let Some(ref proj_name) = payload.project_name {
        // Buscar ruta fÃ­sica de la carpeta del proyecto
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
        // Comprobar si existe en la raÃ­z base_workspace por defecto
        let memories_path = state.base_workspace.join("MEMORIES.md");
        if memories_path.exists() {
            if let Ok(content) = fs::read_to_string(memories_path) {
                memories_content = content;
            }
        }
    }

    let refine_system_prompt = format!("Eres un refinador experto en prompts de IA. Tu objetivo es estructurar, mejorar y corregir prompts.
Debes mantener estrictamente el formato estructurado en 5 bloques en espaÃ±ol:
1. Rol y Contexto (Rol de programador principal en Rust/JS/HTML).
2. Meta TÃ©cnica RÃ­gida (QuÃ© se quiere hacer exactamente).
3. Restricciones y Reglas (Prohibido asumir, prohibido crear APIs externas innecesarias, cÃ³digo optimizado obligatoriamente para correr en un Pentium de 4GB RAM y 2 cores).
4. Formato de Salida (CÃ³digo limpio, comentarios inline).
5. Datos de Soporte (Mencionar archivos relevantes).

Se te provee el SYSTEM PROMPT global y local del proyecto que guiarÃ¡ al agente principal, junto a las MEMORIAS locales persistentes de limitaciones tÃ©cnicas del proyecto:

---
**[SYSTEM PROMPT DEL AGENTE PRINCIPAL (GLOBAL + LOCAL)]**
{}
---

---
**[MEMORIAS DEL PROYECTO ACTUAL (MEMORIES.md)]**
{}
---

Si el usuario te provee un prompt base y una retroalimentaciÃ³n/instrucciÃ³n adicional de ajuste, debes aplicarla sobre el prompt base y devolver el prompt final estructurado entero.
Adicionalmente, se te inyectarÃ¡ el historial reciente del chat para que entiendas de quÃ© elementos o archivos (como 'el botÃ³n azul') se venÃ­a hablando en mensajes anteriores, de modo que el prompt refinado mantenga la coherencia total. No agregues introducciones ni explicaciones; empieza directamente con el prompt final estructurado.", system_prompt_context, memories_content);

    let mut api_messages = vec![
        json!({ "role": "system", "content": refine_system_prompt }),
    ];

    // Cargar historial de chat si session_id estÃ¡ presente para dar contexto al refinador
    if let Some(ref s_id) = payload.session_id {
        let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", s_id));
        if chat_file.exists() {
            if let Ok(content) = fs::read_to_string(&chat_file) {
                if let Ok(session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                    // Tomar los Ãºltimos 6 mensajes para no saturar el contexto de refinado
                    let start_idx = session.messages.len().saturating_sub(6);
                    for m in &session.messages[start_idx..] {
                        let role = if m.role == "agent" { "assistant" } else { "user" };
                        // Sanitizar para evitar meter el reporte de auditorÃ­a completo
                        let clean_content = if m.content.contains("**[AuditorÃ­a de Herramientas Ejecutadas]**") {
                            m.content.split("**[AuditorÃ­a de Herramientas Ejecutadas]**").next().unwrap_or("").trim().to_string()
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
            api_messages.push(json!({ "role": "user", "content": format!("InstrucciÃ³n adicional de modificaciÃ³n:\n```\n{}\n```", fb) }));
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
                Json(json!({ "status": "error", "message": "Error decodificando respuesta de refinaciÃ³n." }))
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

    // 2. Cargar sesiÃ³n existente o crear una nueva
    let mut session = if chat_file.exists() {
        if let Ok(content) = fs::read_to_string(&chat_file) {
            serde_json::from_str::<crate::state::ChatSession>(&content).unwrap_or_else(|_| crate::state::ChatSession {
                id: session_id.clone(),
                title: "Nueva conversaciÃ³n".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            })
        } else {
            crate::state::ChatSession {
                id: session_id.clone(),
                title: "Nueva conversaciÃ³n".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            }
        }
    } else {
        // Generar tÃ­tulo descriptivo conciso usando DeepSeek V4 Flash
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        let prompt_title = format!(
            "Analiza el siguiente mensaje de usuario y genera un tÃ­tulo descriptivo muy corto (mÃ¡ximo 4 palabras) en espaÃ±ol que resuma el tema. No agregues comillas ni explicaciones:\n\n\"{}\"",
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
            println!("Abortando agente activo anterior debido a la recepciÃ³n de un nuevo mensaje de usuario...");
            handle.abort();
        }
        *handle_opt = None;
    }

    // 5. Preparar el agente activo
    {
        let mut status = state.active_agent.lock().unwrap();
        status.running = true;
        status.interrupted = false;
        status.current_session_id = Some(session_id.clone());
        
        // Mantener e inyectar el historial acumulado de pasos en la consola en lugar de borrarlo
        status.steps.clear();
        if let Some(ref prev_steps) = session.steps {
            status.steps.extend(prev_steps.clone());
        }
        
        status.steps.push(crate::state::AuditStep {
            step_type: "thinking".to_string(),
            title: "Reanudando Agente".to_string(),
            detail: format!("Procesando nueva instrucciÃ³n en la conversaciÃ³n activa. Proyecto: {:?}", payload.project_name),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        });
    }

    // 6. Correr el bucle del agente asÃ­ncronamente o en este hilo pero reportando pasos
    // Para que no bloquee y permita interrupciones en tiempo real, lo ejecutamos asÃ­ncronamente en una tarea de Tokio
    let state_clone = state.clone();
    let project_name_clone = payload.project_name.clone();
    let session_id_clone = session_id.clone();
    let session_messages_clone = session.messages.clone();

    let handle = tokio::spawn(async move {
        // Envolver run_agent_loop en su propio tokio::spawn para aislar y atrapar pÃ¡nicos
        let agent_task = tokio::spawn(run_agent_loop(
            session_messages_clone,
            project_name_clone,
            state_clone.clone(),
            deepseek_key(),
            voyage_key(),
            openrouter_key(),
            Some(session_id_clone.clone()),
        ));
        let run_result = match agent_task.await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(e)) => Err(format!("Error de ejecuciÃ³n: {}", e)),
            Err(join_err) => {
                if join_err.is_panic() {
                    // Obtener el payload del pÃ¡nico
                    let panic_any = join_err.into_panic();
                    // Convertir el payload a String segura
                    let panic_detail = if let Some(s) = panic_any.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_any.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(b) = panic_any.downcast_ref::<Vec<u8>>() {
                        // Convertir bytes a UTFâ€‘8 con pÃ©rdida de datos si es necesario
                        String::from_utf8_lossy(b).to_string()
                    } else {
                        // Fallback: representaciÃ³n de depuraciÃ³n
                        format!("{:?}", panic_any)
                    };
                    // Guardar en archivo de log persistente
                    let logs_dir = state_clone.base_workspace.join(".config").join("logs");
                    let _ = std::fs::create_dir_all(&logs_dir);
                    let log_path = logs_dir.join("panic.log");
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_path)
                        .and_then(|mut file| {
                            use std::io::Write;
                            writeln!(
                                file,
                                "[{}] PÃ¡nico crÃ­tico en el agente: {}",
                                chrono::Utc::now().to_rfc3339(),
                                panic_detail,
                            )
                        });
                    Err(format!("PÃ¡nico crÃ­tico en el agente: {}", panic_detail))
                } else {
                    Err(format!("Error crÃ­tico en la tarea de ejecuciÃ³n del agente: {}", join_err))
                }
            }

        };

        let (agent_reply, is_success) = match run_result {
            Ok(reply) => (reply, true),
            Err(err_msg) => {
                eprintln!("{}", err_msg);
                crate::agent::play_error_beep();
                (err_msg, false)
            }
        };

        // Registrar paso de finalizaciÃ³n o error en memoria
        {
            let mut status = state_clone.active_agent.lock().unwrap();
            status.running = false;
            if is_success {
                status.steps.push(crate::state::AuditStep {
                    step_type: "done".to_string(),
                    title: "EjecuciÃ³n finalizada".to_string(),
                    detail: "El agente terminÃ³ de responder y procesar herramientas.".to_string(),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
            } else {
                status.steps.push(crate::state::AuditStep {
                    step_type: "error".to_string(),
                    title: "Error en ejecuciÃ³n".to_string(),
                    detail: agent_reply.clone(),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
            }
        }

        // Guardar respuesta en la sesiÃ³n junto a la consola de auditorÃ­a
        let chat_file_async = state_clone.base_workspace.join(".config").join("chats").join(format!("{}.json", session_id_clone));
        if let Ok(content) = fs::read_to_string(&chat_file_async) {
            if let Ok(mut current_session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                // Obtener los pasos detallados de auditorÃ­a de herramientas (incluido el paso de finalizaciÃ³n/error)
                let active_steps = {
                    let status = state_clone.active_agent.lock().unwrap();
                    status.steps.clone()
                };

                // Guardar los pasos de la auditorÃ­a directamente de manera persistente en la sesiÃ³n
                current_session.steps = Some(active_steps);

                current_session.messages.push(crate::state::ChatMessage {
                    role: "agent".to_string(),
                    content: agent_reply,
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
                let _ = fs::write(&chat_file_async, serde_json::to_string_pretty(&current_session).unwrap());
            }
        }

        // Limpiar el abort handle al finalizar
        {
            let mut handle_opt = state_clone.abort_handle.lock().unwrap();
            *handle_opt = None;
        }
    });

    {
        let mut handle_opt = state.abort_handle.lock().unwrap();
        *handle_opt = Some(handle.abort_handle());
    }

    Json(json!({ "status": "ok", "session_id": session_id }))
}

async fn captcha_status(State(state): State<AppState>) -> impl IntoResponse {
    let cap = state.pending_captcha.lock().unwrap().clone();
    Json(cap)
}


#[derive(Deserialize)]
struct CaptchaSolveRequest {
    id: String,
    solved_content: String,
}

async fn captcha_solve(State(state): State<AppState>, Json(payload): Json<CaptchaSolveRequest>) -> impl IntoResponse {
    let mut cap = state.pending_captcha.lock().unwrap();
    if let Some(ref mut req) = *cap {
        if req.id == payload.id {
            req.solved_content = Some(payload.solved_content.clone());
            return Json(json!({ "status": "ok" }));
        }
    }
    Json(json!({ "status": "error", "message": "No se encontró el CAPTCHA o el ID no coincide." }))
}


// Captcha handling removed (unused)

// --- Handlers de control de escritorio ---
#[derive(Deserialize)]
struct MoveMouseRequest { x: i32, y: i32 }

async fn move_mouse_handler(State(state): State<AppState>, Json(payload): Json<MoveMouseRequest>) -> impl IntoResponse {
    let controller = state.desktop.lock().unwrap();
    match controller.move_mouse(payload.x, payload.y) {
        Ok(_) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

#[derive(Deserialize)]
struct ClickRequest { button: String }

async fn click_handler(State(state): State<AppState>, Json(payload): Json<ClickRequest>) -> impl IntoResponse {
    let controller = state.desktop.lock().unwrap();
    match controller.click(&payload.button) {
        Ok(_) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

#[derive(Deserialize)]
struct TypeTextRequest { text: String }

async fn type_text_handler(State(state): State<AppState>, Json(payload): Json<TypeTextRequest>) -> impl IntoResponse {
    let controller = state.desktop.lock().unwrap();
    match controller.type_text(&payload.text) {
        Ok(_) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

#[derive(Deserialize)]
struct LaunchRequest { path: String }

async fn launch_handler(State(state): State<AppState>, Json(payload): Json<LaunchRequest>) -> impl IntoResponse {
    let controller = state.desktop.lock().unwrap();
    match controller.launch_executable(&payload.path) {
        Ok(pid) => Json(json!({ "status": "ok", "pid": pid })),
        Err(e) => Json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

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
    };
    // Auto-descubrir proyectos locales por defecto
    discover_projects(&state);
    // Auto-descubrir proyectos locales por defecto

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
            eprintln!("El servidor de Axum terminÃ³ con un error: {}", e);
        }
    }
}

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
    let p = state.prompts.lock().unwrap().clone();
    Json(p)
}

#[derive(Deserialize)]
struct SavePromptsRequest {
    global_current: String,
    projects: HashMap<String, String>,
}


async fn save_prompts(State(state): State<AppState>, Json(payload): Json<SavePromptsRequest>) -> impl IntoResponse {
    let mut prompts = state.prompts.lock().unwrap();
    prompts.global_current = payload.global_current;
    prompts.projects = payload.projects;
    let _ = fs::write(&state.config_path, serde_json::to_string_pretty(&*prompts).unwrap());
    Json(json!({ "status": "ok" }))
}

async fn reset_global_prompt(State(state): State<AppState>) -> impl IntoResponse {
    let mut prompts = state.prompts.lock().unwrap();
    prompts.global_current = prompts.global_default.clone();
    let _ = fs::write(&state.config_path, serde_json::to_string_pretty(&*prompts).unwrap());
    Json(json!({ "status": "ok", "global_current": prompts.global_current.clone() }))
}

async fn summarize_chat_steps(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> impl IntoResponse {
    let chats_dir = state.base_workspace.join(".config").join("chats");
    let chat_file = chats_dir.join(format!("{}.json", session_id));

    if !chat_file.exists() {
        return Json(json!({ "status": "error", "message": "No se encontrÃ³ la sesiÃ³n de chat." }));
    }

    let mut session = match fs::read_to_string(&chat_file) {
        Ok(content) => match serde_json::from_str::<crate::state::ChatSession>(&content) {
            Ok(s) => s,
            Err(_) => return Json(json!({ "status": "error", "message": "Error al deserializar la sesiÃ³n." })),
        },
        Err(e) => return Json(json!({ "status": "error", "message": format!("Error leyendo sesiÃ³n: {}", e) })),
    };

    let steps_to_summarize = session.steps.clone().unwrap_or_default();
    if steps_to_summarize.is_empty() {
        return Json(json!({ "status": "error", "message": "No hay pasos que resumir en esta sesiÃ³n." }));
    }

    let mut steps_text = String::new();
    for (i, step) in steps_to_summarize.iter().enumerate() {
        steps_text.push_str(&format!(
            "Paso #{}: Tipo={}, TÃ­tulo={}\nDetalle: {}\n\n",
            i + 1, step.step_type, step.title, step.detail
        ));
    }

    let mut messages_text = String::new();
    for msg in &session.messages {
        messages_text.push_str(&format!("{}: {}\n\n", msg.role, msg.content));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let payload = json!({
        "model": "deepseek-v4-flash",
        "messages": [
            {
                "role": "system",
                "content": "Eres un auditor tÃ©cnico experto. Tu tarea es resumir el proceso de auditorÃ­a y los pasos de ejecuciÃ³n que te provee el usuario, teniendo en cuenta la conversaciÃ³n. El objetivo principal de este resumen es reducir drÃ¡sticamente el tamaÃ±o del contexto de ejecuciÃ³n del agente autÃ³nomo para ahorrar costos de tokens y optimizar su memoria a largo plazo, permitiÃ©ndole continuar su ejecuciÃ³n de forma fluida. Tu resumen en espaÃ±ol DEBE responder obligatoriamente de manera estructurada a: 1. Â¿QuÃ© estaba haciendo el agente en ese mismo instante? 2. Â¿Por quÃ© lo hacÃ­a? 3. Â¿QuÃ© le faltaba por hacer? 4. Â¿CuÃ¡l era el objetivo final y quÃ© objetivos secundarios ya se cumplieron? 5. SOBRE TODO, el 'CÃ“MO': Detalla minuciosamente el mÃ©todo tÃ©cnico empleado (herramientas usadas, archivos especÃ­ficos modificados o leÃ­dos, y resultados de compilaciÃ³n/operaciones). Redacta el contenido en formato markdown tÃ©cnico, muy directo y sin introducciones."
            },
            {
                "role": "user",
                "content": format!(
                    "--- HISTORIAL DE CONVERSACIÃ“N ---\n{}\n\n--- PASOS DETALLADOS A RESUMIR ---\n{}",
                    messages_text,
                    steps_text
                )
            }
        ]
    });

    match client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", deepseek_key()))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(res_val) = resp.json::<serde_json::Value>().await {
                    if let Some(summary_text) = res_val["choices"][0]["message"]["content"].as_str() {
                        let new_step = crate::state::AuditStep {
                            step_type: "thinking".to_string(),
                            title: "Resumen de Contexto (Forzado)".to_string(),
                            detail: summary_text.to_string(),
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        };
                        
                        session.steps = Some(vec![new_step.clone()]);
                        
                        session.messages.push(crate::state::ChatMessage {
                            role: "agent".to_string(),
                            content: format!("--- RESUMEN DE PASOS DE AUDITORÃA ---\n{}", summary_text),
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        });
                        
                        let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                        
                        {
                            let mut active = state.active_agent.lock().unwrap();
                            if active.current_session_id.as_deref() == Some(&session.id) {
                                active.steps = vec![new_step];
                            }
                        }

                        return Json(json!({ "status": "ok", "summary": summary_text }));
                    }
                }
            }
            Json(json!({ "status": "error", "message": "Error al leer la respuesta de la API de DeepSeek." }))
        }
        Err(e) => {
            Json(json!({ "status": "error", "message": format!("Error de conexiÃ³n: {}", e) }))
        }
    }
}


