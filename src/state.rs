use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::desktop::DesktopController;
#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub is_local: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub global_default: String,
    pub global_current: String,
    pub projects: HashMap<String, String>, // project_name -> local_prompt
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub project_name: Option<String>,
    pub steps: Option<Vec<AuditStep>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuditStep {
    pub step_type: String, // "tool_call", "tool_result", "thinking", "error", "done"
    pub title: String,
    pub detail: String,
    pub timestamp: u64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ActiveAgentStatus {
    pub running: bool,
    pub interrupted: bool,
    pub esperando_respuesta_usuario: bool,
    pub pregunta_usuario: Option<String>,
    pub respuesta_usuario: Option<String>,
    pub esperando_aprobacion_plan: bool,
    pub plan_propuesto: Option<String>,
    pub thinking_content: Vec<String>,
    pub steps: Vec<AuditStep>,
#[derive(Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub entry_type: String,      // "file_read", "command_exec", "file_write"
    pub summary: String,         // Resumen corto (1-2 líneas)
    pub full_content: String,    // Contenido completo
    pub created_at: u64,         // Timestamp Unix
}

#[derive(Clone)]
pub struct AppState {
    pub config_path: PathBuf,
    pub prompts: Arc<Mutex<PromptConfig>>,
    pub projects: Arc<Mutex<Vec<Project>>>,
    pub base_workspace: PathBuf,
    pub pending_captcha: Arc<Mutex<Option<CaptchaRequest>>>,
    pub active_agent: Arc<Mutex<ActiveAgentStatus>>,
    pub abort_handle: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
    pub desktop: Arc<Mutex<DesktopController>>,
    pub image_store: Arc<Mutex<HashMap<String, String>>>,
    pub context_store: Arc<Mutex<HashMap<String, ContextEntry>>>,  // ID -> contenido para gestion de contexto
}
