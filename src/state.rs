use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::desktop::DesktopController;
use crate::auth::{UserStore, ChallengeStore, SessionStore};

// ============================================================================
// Proyectos y Configuración
// ============================================================================

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
    pub projects: HashMap<String, String>,
}

// ============================================================================
// Sesiones de Chat
// ============================================================================

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

// ============================================================================
// Auditoría
// ============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct AuditStep {
    pub step_type: String,
    pub title: String,
    pub detail: String,
    pub timestamp: u64,
}

// ============================================================================
// Estado del Agente Activo
// ============================================================================

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

// ============================================================================
// Almacén de Contexto
// ============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub entry_type: String,
    pub summary: String,
    pub full_content: String,
    pub created_at: u64,
}

// ============================================================================
// CAPTCHA
// ============================================================================

#[derive(Clone, Serialize, Deserialize)]
pub struct CaptchaRequest {
    pub id: String,
    pub sitekey: String,
    pub url: String,
    pub solved_content: Option<String>,
}

// ============================================================================
// Tool Result Store
// ============================================================================

#[derive(Clone, Default)]
pub struct ToolResultStore {
    pub entries: Arc<Mutex<HashMap<String, StoredToolResult>>>,
}

#[derive(Clone)]
pub struct StoredToolResult {
    pub call_id: String,
    pub tool_name: String,
    pub full_content: String,
    pub stored_at: u64,
}

impl ToolResultStore {
    pub fn new() -> Self {
        Self { entries: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn store(&self, call_id: &str, tool_name: &str, full_content: &str) -> String {
        let entry = StoredToolResult {
            call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            full_content: full_content.to_string(),
            stored_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };
        let total_chars = full_content.chars().count();
        {
            let mut entries = self.entries.lock().unwrap();
            entries.insert(call_id.to_string(), entry);
        }
        format!(
            "{}\n\n[ID: {} | {} caracteres | usa release_tool_result(\"{}\") cuando ya no necesites este resultado]",
            full_content, call_id, total_chars, call_id
        )
    }

    pub fn fetch_page(&self, call_id: &str, page: usize, page_size: usize) -> Option<String> {
        let entries = self.entries.lock().unwrap();
        let entry = entries.get(call_id)?;
        let chars: Vec<char> = entry.full_content.chars().collect();
        let total_chars = chars.len();
        let total_pages = (total_chars as f64 / page_size as f64).ceil() as usize;
        if page >= total_pages {
            return Some(format!("Página {} fuera de rango. El resultado tiene {} páginas (0-{}).",
                page, total_pages, total_pages.saturating_sub(1)));
        }
        let start = page * page_size;
        let end = std::cmp::min(start + page_size, total_chars);
        let chunk: String = chars[start..end].iter().collect();
        Some(format!("--- Página {}/{} (caracteres {}-{} de {}) ---\n{}",
            page + 1, total_pages, start + 1, end, total_chars, chunk))
    }

    pub fn release(&self, call_id: &str) -> bool {
        self.entries.lock().unwrap().remove(call_id).is_some()
    }

    pub fn reap_old(&self, max_age_secs: u64) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let mut entries = self.entries.lock().unwrap();
        let before = entries.len();
        entries.retain(|_, v| now - v.stored_at < max_age_secs);
        before - entries.len()
    }

    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }
}

// ============================================================================
// Sub-Agent Manager
// ============================================================================

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum SubAgentStatus {
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SubAgentHandle {
    pub id: String,
    pub task_description: String,
    pub project_name: Option<String>,
    pub allowed_paths: Vec<String>,
    pub started_at: u64,
    pub status: SubAgentStatus,
    pub result: Option<String>,
    pub context_summary: Option<String>,
    #[serde(skip)]
    pub abort_handle: Option<tokio::task::AbortHandle>,
}

#[derive(Clone)]
pub struct SubAgentManager {
    pub agents: Arc<Mutex<HashMap<String, SubAgentHandle>>>,
    pub max_parallel: Arc<Mutex<usize>>,
}

impl SubAgentManager {
    pub fn new() -> Self {
        let num_cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
        let max = if num_cpus <= 2 { 1 } else if num_cpus <= 4 { 2 } else if num_cpus <= 8 { 4 } else { 8 };
        Self { agents: Arc::new(Mutex::new(HashMap::new())), max_parallel: Arc::new(Mutex::new(max)) }
    }

    pub fn register(&self, id: String, task_description: String, project_name: Option<String>,
        allowed_paths: Vec<String>, context_summary: Option<String>, abort_handle: Option<tokio::task::AbortHandle>) {
        let mut agents = self.agents.lock().unwrap();
        agents.insert(id.clone(), SubAgentHandle {
            id, task_description, project_name, allowed_paths,
            started_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            status: SubAgentStatus::Running, result: None, context_summary, abort_handle,
        });
    }

    pub fn update_status(&self, id: &str, status: SubAgentStatus, result: Option<String>) {
        let mut agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get_mut(id) { agent.status = status; agent.result = result; }
    }

    pub fn cancel(&self, id: &str) -> bool {
        let mut agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get_mut(id) {
            if agent.status == SubAgentStatus::Running {
                if let Some(ref handle) = agent.abort_handle { handle.abort(); }
                agent.status = SubAgentStatus::Cancelled;
                agent.result = Some("Cancelado por el agente principal.".to_string());
                return true;
            }
        }
        false
    }

    pub fn cancel_all(&self) {
        let mut agents = self.agents.lock().unwrap();
        for (_, agent) in agents.iter_mut() {
            if agent.status == SubAgentStatus::Running {
                if let Some(ref handle) = agent.abort_handle { handle.abort(); }
                agent.status = SubAgentStatus::Cancelled;
                agent.result = Some("Cancelado: el agente principal finalizó o fue interrumpido.".to_string());
            }
        }
    }

    pub fn running_count(&self) -> usize {
        self.agents.lock().unwrap().values().filter(|a| a.status == SubAgentStatus::Running).count()
    }

    pub fn can_spawn(&self) -> bool {
        self.running_count() < *self.max_parallel.lock().unwrap()
    }

    pub fn status_summary(&self) -> String {
        let agents = self.agents.lock().unwrap();
        if agents.is_empty() { return "No hay sub-agentes registrados.".to_string(); }
        let mut lines: Vec<String> = Vec::with_capacity(agents.len() + 1);
        lines.push(format!("=== SUB-AGENTES ({} total, {} activos, máx paralelos: {}) ===",
            agents.len(), agents.values().filter(|a| a.status == SubAgentStatus::Running).count(),
            *self.max_parallel.lock().unwrap()));
        let mut sorted: Vec<&SubAgentHandle> = agents.values().collect();
        sorted.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        for agent in &sorted {
            let status_str = match &agent.status {
                SubAgentStatus::Running => "🏃 EN EJECUCIÓN",
                SubAgentStatus::Completed => "✅ COMPLETADO",
                SubAgentStatus::Failed(e) => &format!("❌ FALLÓ: {}", e),
                SubAgentStatus::Cancelled => "🚫 CANCELADO",
            };
            let result_preview = agent.result.as_ref().map(|r| {
                if r.chars().count() > 150 { format!(": {}", r.chars().take(150).collect::<String>() + "...") }
                else { format!(": {}", r) }
            }).unwrap_or_default();
            let paths_str = if agent.allowed_paths.is_empty() { "acceso completo".to_string() }
                else { format!("restringido a: {}", agent.allowed_paths.join(", ")) };
            lines.push(format!("  [{}] {} — {} ({}){}", agent.id,
                agent.task_description.chars().take(80).collect::<String>(), status_str, paths_str, result_preview));
        }
        lines.join("\n")
    }

    pub fn reap_old(&self, max_age_secs: u64) -> usize {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let mut agents = self.agents.lock().unwrap();
        let before = agents.len();
        agents.retain(|_, a| a.status == SubAgentStatus::Running || (now - a.started_at) < max_age_secs);
        before - agents.len()
    }
}

// ============================================================================
// Process Registry
// ============================================================================

#[derive(Clone)]
pub struct ProcessRegistry {
    pub spawned: Arc<Mutex<HashSet<u32>>>,
    pub server_pid: u32,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self { spawned: Arc::new(Mutex::new(HashSet::new())), server_pid: std::process::id() }
    }

    pub fn register(&self, pid: u32) {
        self.spawned.lock().unwrap().insert(pid);
    }

    pub fn safe_kill(&self, pid: u32) -> String {
        {
            let spawned = self.spawned.lock().unwrap();
            if !spawned.contains(&pid) {
                return format!("ERROR DE SEGURIDAD: El PID {} no fue spawnado por este agente.", pid);
            }
        }
        let parent_pid = match get_parent_pid(pid) {
            Some(ppid) => ppid,
            None => { self.spawned.lock().unwrap().remove(&pid); return format!("El proceso con PID {} ya no existe.", pid); }
        };
        if parent_pid != self.server_pid {
            return format!("ERROR DE SEGURIDAD: El proceso con PID {} no es hijo directo del servidor.", pid);
        }
        let output = Command::new("taskkill").args(&["/PID", &pid.to_string(), "/F"]).output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.spawned.lock().unwrap().remove(&pid);
                if out.status.success() {
                    format!("Proceso PID {} matado exitosamente.\nstdout: {}\nstderr: {}", pid, stdout, stderr)
                } else {
                    format!("taskkill retornó error para PID {}.\nstdout: {}\nstderr: {}", pid, stdout, stderr)
                }
            }
            Err(e) => format!("Error al ejecutar taskkill para PID {}: {}", pid, e),
        }
    }

    pub fn kill_all(&self) {
        let pids: Vec<u32> = self.spawned.lock().unwrap().iter().cloned().collect();
        for pid in pids {
            if let Some(parent_pid) = get_parent_pid(pid) {
                if parent_pid == self.server_pid {
                    let _ = Command::new("taskkill").args(&["/PID", &pid.to_string(), "/F"])
                        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
                }
            }
        }
        self.spawned.lock().unwrap().clear();
    }

    pub fn reap(&self) -> usize {
        let mut spawned = self.spawned.lock().unwrap();
        let before = spawned.len();
        spawned.retain(|&pid| get_parent_pid(pid).is_some());
        before - spawned.len()
    }

    pub fn contains(&self, pid: u32) -> bool {
        self.spawned.lock().unwrap().contains(&pid)
    }
}

fn get_parent_pid(pid: u32) -> Option<u32> {
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command",
            &format!("(Get-Process -Id {} -ErrorAction SilentlyContinue).ParentProcessId", pid)])
        .output().ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse::<u32>().ok()
}

// ============================================================================
// AppState — Estado Global de la Aplicación
// ============================================================================

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
    pub process_registry: ProcessRegistry,
    pub tool_results: ToolResultStore,
    pub sub_agents: SubAgentManager,
    pub user_store: UserStore,
    pub challenge_store: ChallengeStore,
    pub session_store: SessionStore,
    pub study_engine: crate::study::StudyEngine,
    pub sync_store: crate::sync::SyncStore,
    pub connected_clients: Arc<Mutex<HashMap<String, crate::client_protocol::ConnectedClient>>>,
    pub client_pending_requests: Arc<Mutex<HashMap<String, Vec<crate::client_protocol::ClientRequest>>>>,
    pub client_responses: Arc<Mutex<HashMap<String, crate::client_protocol::ClientResponse>>>,
    /// true si este AppState sirve al puerto 80 (admin local sin auth)
    pub port_80: bool,
}
