use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::process::Child;

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
    pub current_session_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub entry_type: String,
    pub summary: String,
    pub full_content: String,
    pub created_at: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CaptchaRequest {
    pub id: String,
    pub sitekey: String,
    pub url: String,
    pub solved_content: Option<String>,
}

/// Registro de procesos hijo spawnados por el agente.
/// La clave es el PID del proceso (u32) y el valor es el handle Child.
/// Esto permite matar procesos de forma segura sin depender de taskkill,
/// evitando matar accidentalmente al servidor principal.
#[derive(Clone)]
pub struct ProcessRegistry {
    pub processes: Arc<Mutex<HashMap<u32, Child>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registra un proceso hijo. Devuelve su PID.
    pub fn register(&self, child: Child) -> u32 {
        let pid = child.id();
        let mut procs = self.processes.lock().unwrap();
        procs.insert(pid, child);
        pid
    }

    /// Mata un proceso por PID y lo remueve del registro.
    /// Retorna true si existía y se pudo matar, false si no existía.
    pub fn kill(&self, pid: u32) -> bool {
        let mut procs = self.processes.lock().unwrap();
        if let Some(mut child) = procs.remove(&pid) {
            let _ = child.kill();
            let _ = child.wait();
            true
        } else {
            false
        }
    }

    /// Mata y limpia TODOS los procesos registrados.
    /// Se llama al finalizar la sesión del agente.
    pub fn kill_all(&self) {
        let mut procs = self.processes.lock().unwrap();
        for (_pid, mut child) in procs.drain() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Limpia procesos que ya terminaron (zombies) del registro.
    /// Retorna cuántos procesos fueron limpiados.
    pub fn reap(&self) -> usize {
        let mut procs = self.processes.lock().unwrap();
        let before = procs.len();
        procs.retain(|_pid, child| {
            match child.try_wait() {
                Ok(Some(_)) => false, // ya terminó, remover
                _ => true,            // sigue corriendo o error, mantener
            }
        });
        before - procs.len()
    }

    /// Retorna true si un PID existe en el registro.
    pub fn contains(&self, pid: u32) -> bool {
        let procs = self.processes.lock().unwrap();
        procs.contains_key(&pid)
    }

    /// Retorna la cantidad de procesos actualmente registrados.
    pub fn len(&self) -> usize {
        let procs = self.processes.lock().unwrap();
        procs.len()
    }
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
    pub context_store: Arc<Mutex<HashMap<String, ContextEntry>>>,
    /// Registro seguro de procesos hijo spawnados por el agente.
    /// Permite matar procesos por handle sin riesgo de matar al servidor.
    pub process_registry: ProcessRegistry,
}
