use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::desktop::DesktopController;

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
    pub projects: HashMap<String, String>, // project_name -> local_prompt
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
    pub step_type: String, // "tool_call", "tool_result", "thinking", "error", "done"
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
// Tool Result Store — Reemplaza el truncado arbitrario con IDs
// ============================================================================

/// Almacena el resultado completo de una herramienta bajo un ID único.
/// En lugar de truncar resultados grandes a 25K chars y perder información,
/// el agente recibe un resumen + ID y puede paginar o liberar cuando quiera.
#[derive(Clone, Default)]
pub struct ToolResultStore {
    /// Map de call_id → (resultado_completo, timestamp, nombre_herramienta)
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
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Guarda un resultado completo y retorna un resumen truncado para el agente.
    /// Guarda un resultado completo y lo retorna ENTERO al agente.
    /// NO se trunca nada. El agente recibe el resultado completo y decide
    /// cuándo liberarlo con release_tool_result.
    /// Se adjunta un footer informativo con el ID por si quiere liberarlo después.
    pub fn store(&self, call_id: &str, tool_name: &str, full_content: &str) -> String {
        let entry = StoredToolResult {
            call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            full_content: full_content.to_string(),
            stored_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let total_chars = full_content.chars().count();

        {
            let mut entries = self.entries.lock().unwrap();
            entries.insert(call_id.to_string(), entry);
        }

        // Siempre devolver el contenido COMPLETO. El agente decide cuándo liberar.
        // Solo se añade un footer informativo con el ID para que sepa que puede
        // usar release_tool_result cuando ya no necesite este resultado.
        format!(
            "{}\n\n[ID: {} | {} caracteres | usa release_tool_result(\"{}\") cuando ya no necesites este resultado]",
            full_content,
            call_id,
            total_chars,
            call_id
        )
    }
            pages,
            call_id,
            call_id,
            call_id
        )
    }

    /// Recupera una página del resultado almacenado.
    /// page es 0-indexado, page_size en caracteres.
    pub fn fetch_page(&self, call_id: &str, page: usize, page_size: usize) -> Option<String> {
        let entries = self.entries.lock().unwrap();
        let entry = entries.get(call_id)?;

        let chars: Vec<char> = entry.full_content.chars().collect();
        let total_chars = chars.len();
        let total_pages = (total_chars as f64 / page_size as f64).ceil() as usize;

        if page >= total_pages {
            return Some(format!(
                "Página {} fuera de rango. El resultado tiene {} páginas (0-{}).",
                page, total_pages, total_pages.saturating_sub(1)
            ));
        }

        let start = page * page_size;
        let end = std::cmp::min(start + page_size, total_chars);
        let chunk: String = chars[start..end].iter().collect();

        Some(format!(
            "--- Página {}/{} (caracteres {}-{} de {}) ---\n{}",
            page + 1,
            total_pages,
            start + 1,
            end,
            total_chars,
            chunk
        ))
    }

    /// Libera un resultado de la memoria.
    pub fn release(&self, call_id: &str) -> bool {
        let mut entries = self.entries.lock().unwrap();
        entries.remove(call_id).is_some()
    }

    /// Libera todos los resultados más antiguos que `max_age_secs`.
    pub fn reap_old(&self, max_age_secs: u64) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut entries = self.entries.lock().unwrap();
        let before = entries.len();
        entries.retain(|_, v| now - v.stored_at < max_age_secs);
        before - entries.len()
    }

    /// Retorna la cantidad de resultados almacenados.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }
}

// ============================================================================
// Sub-Agent Manager — Múltiples agentes en paralelo
// ============================================================================

/// Estado de un sub-agente.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum SubAgentStatus {
    /// El sub-agente está ejecutándose.
    Running,
    /// Completado exitosamente.
    Completed,
    /// Falló con un mensaje de error.
    Failed(String),
    /// Fue cancelado por el agente principal o el usuario.
    Cancelled,
}

/// Handle para gestionar un sub-agente en ejecución.
#[derive(Clone, Serialize, Deserialize)]
pub struct SubAgentHandle {
    /// ID único del sub-agente.
    pub id: String,
    /// Descripción de la tarea asignada.
    pub task_description: String,
    pub project_name: Option<String>,
    /// Directorios/archivos a los que tiene acceso restringido.
    /// Si está vacío, tiene acceso completo (como el agente principal).
    pub allowed_paths: Vec<String>,
    /// Timestamp de inicio.
    pub started_at: u64,
    /// Estado actual.
    pub status: SubAgentStatus,
    /// Resultado final (si completado).
    pub result: Option<String>,
    /// Resumen del contexto heredado.
    pub context_summary: Option<String>,
    /// AbortHandle para cancelar la tarea.
    #[serde(skip)]
    pub abort_handle: Option<tokio::task::AbortHandle>,
}

/// Gestor de sub-agentes. Permite spawnear múltiples agentes en paralelo
/// con restricciones de directorios y contexto heredado.
#[derive(Clone)]
pub struct SubAgentManager {
    /// Sub-agentes activos y completados.
    pub agents: Arc<Mutex<HashMap<String, SubAgentHandle>>>,
    /// Límite máximo de sub-agentes concurrentes (dinámico según hardware).
    pub max_parallel: Arc<Mutex<usize>>,
}

impl SubAgentManager {
    pub fn new() -> Self {
        // Detectar hardware para escalar dinámicamente el paralelismo
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2);

        // En hardware mínimo (2 cores), permitir 1 sub-agente.
        // En hardware mejor, escalar hasta 8.
        let max = if num_cpus <= 2 {
            1
        } else if num_cpus <= 4 {
            2
        } else if num_cpus <= 8 {
            4
        } else {
            // 16+ cores: permitir hasta 8 sub-agentes concurrentes
            8
        };

        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            max_parallel: Arc::new(Mutex::new(max)),
        }
    }

    /// Registra un nuevo sub-agente como Running.
    pub fn register(
        &self,
        id: String,
        task_description: String,
        project_name: Option<String>,
        allowed_paths: Vec<String>,
        context_summary: Option<String>,
        abort_handle: Option<tokio::task::AbortHandle>,
    ) {
        let mut agents = self.agents.lock().unwrap();
        agents.insert(
            id.clone(),
            SubAgentHandle {
                id,
                task_description,
                project_name,
                allowed_paths,
                started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                status: SubAgentStatus::Running,
                result: None,
                context_summary,
                abort_handle,
            },
        );
    }

    /// Actualiza el estado de un sub-agente.
    pub fn update_status(&self, id: &str, status: SubAgentStatus, result: Option<String>) {
        let mut agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get_mut(id) {
            agent.status = status;
            agent.result = result;
        }
    }

    /// Cancela un sub-agente por ID.
    pub fn cancel(&self, id: &str) -> bool {
        let mut agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get_mut(id) {
            if agent.status == SubAgentStatus::Running {
                if let Some(ref handle) = agent.abort_handle {
                    handle.abort();
                }
                agent.status = SubAgentStatus::Cancelled;
                agent.result = Some("Cancelado por el agente principal.".to_string());
                return true;
            }
        }
        false
    }

    /// Cancela todos los sub-agentes en ejecución.
    pub fn cancel_all(&self) {
        let mut agents = self.agents.lock().unwrap();
        for (_, agent) in agents.iter_mut() {
            if agent.status == SubAgentStatus::Running {
                if let Some(ref handle) = agent.abort_handle {
                    handle.abort();
                }
                agent.status = SubAgentStatus::Cancelled;
                agent.result = Some("Cancelado: el agente principal finalizó o fue interrumpido.".to_string());
            }
        }
    }

    /// Retorna el número de sub-agentes actualmente en ejecución.
    pub fn running_count(&self) -> usize {
        let agents = self.agents.lock().unwrap();
        agents
            .values()
            .filter(|a| a.status == SubAgentStatus::Running)
            .count()
    }

    /// Retorna true si se pueden spawnear más sub-agentes.
    pub fn can_spawn(&self) -> bool {
        self.running_count() < *self.max_parallel.lock().unwrap()
    }

    /// Obtiene el estado de todos los sub-agentes formateado para el agente principal.
    pub fn status_summary(&self) -> String {
        let agents = self.agents.lock().unwrap();
        if agents.is_empty() {
            return "No hay sub-agentes registrados.".to_string();
        }

        let mut lines: Vec<String> = Vec::with_capacity(agents.len() + 1);
        lines.push(format!(
            "=== SUB-AGENTES ({} total, {} activos, máx paralelos: {}) ===",
            agents.len(),
            agents.values().filter(|a| a.status == SubAgentStatus::Running).count(),
            *self.max_parallel.lock().unwrap()
        ));

        // Ordenar por timestamp (más reciente primero)
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
                if r.chars().count() > 150 {
                    format!(": {}", r.chars().take(150).collect::<String>() + "...")
                } else {
                    format!(": {}", r)
                }
            }).unwrap_or_default();

            let paths_str = if agent.allowed_paths.is_empty() {
                "acceso completo".to_string()
            } else {
                format!("restringido a: {}", agent.allowed_paths.join(", "))
            };

            lines.push(format!(
                "  [{}] {} — {} ({}){}",
                agent.id,
                agent.task_description.chars().take(80).collect::<String>(),
                status_str,
                paths_str,
                result_preview
            ));
        }

        lines.join("\n")
    }

    /// Limpia sub-agentes completados/cancelados/fallidos más antiguos que `max_age_secs`.
    pub fn reap_old(&self, max_age_secs: u64) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut agents = self.agents.lock().unwrap();
        let before = agents.len();
        agents.retain(|_, a| {
            a.status == SubAgentStatus::Running || (now - a.started_at) < max_age_secs
        });
        before - agents.len()
    }
}

// ============================================================================
// Process Registry
// ============================================================================

/// Registro seguro de procesos hijo spawnados por el agente.
/// Almacena los PIDs de procesos que nosotros spawnamos.
/// Para matar un proceso, verifica que:
/// 1. El PID está en nuestro registro (lo spawnamos nosotros)
/// 2. El proceso existe y su parent PID es el de este servidor
/// Solo entonces ejecuta taskkill.
/// Esto elimina el riesgo de matar al servidor principal por error.
#[derive(Clone)]
pub struct ProcessRegistry {
    /// PIDs que spawnamos, con timestamp de creación
    pub spawned: Arc<Mutex<HashSet<u32>>>,
    /// PID del servidor (cacheado al iniciar)
    pub server_pid: u32,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            spawned: Arc::new(Mutex::new(HashSet::new())),
            server_pid: std::process::id(),
        }
    }

    /// Registra un PID como spawnado por nosotros.
    pub fn register(&self, pid: u32) {
        let mut spawned = self.spawned.lock().unwrap();
        spawned.insert(pid);
    }

    /// Intenta matar un proceso de forma segura.
    /// Retorna un mensaje descriptivo del resultado.
    pub fn safe_kill(&self, pid: u32) -> String {
        // Paso 1: Verificar que el PID está en nuestro registro
        {
            let spawned = self.spawned.lock().unwrap();
            if !spawned.contains(&pid) {
                return format!(
                    "ERROR DE SEGURIDAD: El PID {} no fue spawnado por este agente. \
                    No se permite matar procesos arbitrarios. \
                    Solo podés matar procesos que hayas spawnado con execute_powershell.",
                    pid
                );
            }
        }

        // Paso 2: Verificar que el proceso existe y es hijo de este servidor
        let parent_pid = match get_parent_pid(pid) {
            Some(ppid) => ppid,
            None => {
                // El proceso ya no existe, limpiar del registro
                let mut spawned = self.spawned.lock().unwrap();
                spawned.remove(&pid);
                return format!(
                    "El proceso con PID {} ya no existe (posiblemente ya terminó). \
                    Se ha limpiado del registro.",
                    pid
                );
            }
        };

        if parent_pid != self.server_pid {
            return format!(
                "ERROR DE SEGURIDAD: El proceso con PID {} tiene parent PID {} \
                pero el servidor es PID {}. No es un hijo directo del servidor. \
                Matarlo podría afectar al sistema. Operación cancelada.",
                pid, parent_pid, self.server_pid
            );
        }

        // Paso 3: Matar el proceso
        let output = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                // Limpiar del registro
                let mut spawned = self.spawned.lock().unwrap();
                spawned.remove(&pid);
                if out.status.success() {
                    format!("Proceso PID {} matado exitosamente.\nstdout: {}\nstderr: {}", pid, stdout, stderr)
                } else {
                    format!("taskkill retornó error para PID {}.\nstdout: {}\nstderr: {}", pid, stdout, stderr)
                }
            }
            Err(e) => {
                format!("Error al ejecutar taskkill para PID {}: {}", pid, e)
            }
        }
    }

    /// Mata todos los procesos registrados que sigan siendo hijos del servidor.
    /// Se llama al finalizar la sesión del agente.
    pub fn kill_all(&self) {
        let pids: Vec<u32> = {
            let spawned = self.spawned.lock().unwrap();
            spawned.iter().cloned().collect()
        };

        for pid in pids {
            // Verificar parent PID antes de matar
            if let Some(parent_pid) = get_parent_pid(pid) {
                if parent_pid == self.server_pid {
                    let _ = Command::new("taskkill")
                        .args(&["/PID", &pid.to_string(), "/F"])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                }
            }
        }

        // Limpiar todo el registro
        let mut spawned = self.spawned.lock().unwrap();
        spawned.clear();
    }

    /// Limpia procesos que ya terminaron del registro.
    pub fn reap(&self) -> usize {
        let mut spawned = self.spawned.lock().unwrap();
        let before = spawned.len();
        spawned.retain(|&pid| {
            get_parent_pid(pid).is_some() // solo mantiene si el proceso existe
        });
        before - spawned.len()
    }

    /// Retorna true si un PID está registrado.
    pub fn contains(&self, pid: u32) -> bool {
        let spawned = self.spawned.lock().unwrap();
        spawned.contains(&pid)
    }
}

/// Obtiene el ParentProcessId de un proceso Windows usando Get-Process (PowerShell).
/// Reemplaza al obsoleto wmic.
/// Retorna None si el proceso no existe.
fn get_parent_pid(pid: u32) -> Option<u32> {
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-Command",
            &format!(
                "(Get-Process -Id {} -ErrorAction SilentlyContinue).ParentProcessId",
                pid
            ),
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse::<u32>().ok()
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
    /// Registro seguro de procesos hijo spawnados por el agente.
    /// Permite matar procesos de forma segura con validación de parent PID.
    pub process_registry: ProcessRegistry,
    /// Almacén de resultados completos de herramientas (reemplaza truncado arbitrario).
    /// El agente recibe IDs y puede paginar/liberar resultados bajo demanda.
    pub tool_results: ToolResultStore,
    /// Gestor de sub-agentes para trabajo paralelo.
    pub sub_agents: SubAgentManager,
}
