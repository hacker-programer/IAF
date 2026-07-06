use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use std::process::Command;

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
}
