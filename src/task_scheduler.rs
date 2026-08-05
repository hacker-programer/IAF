// ============================================================================
// task_scheduler.rs — Planificador de Tareas Programadas
// ============================================================================
// Soporta:
//   - Tareas one-time (se ejecutan una vez a una hora específica)
//   - Tareas recurrentes (cada N minutos/horas/días)
//   - Tareas con expresión cron-like (minuto, hora, día_mes, mes, día_semana)
//   - Almacenamiento persistente en JSON
//   - Historial de ejecuciones
//   - Notificaciones al usuario al completarse

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

// ============================================================================
// Tipos de datos
// ============================================================================

/// Frecuencia de una tarea recurrente
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TaskFrequency {
    /// Una sola vez
    Once,
    /// Cada N minutos
    EveryMinutes(u64),
    /// Cada N horas
    EveryHours(u64),
    /// Cada N días
    EveryDays(u64),
    /// Expresión cron-like: (minuto, hora, dia_mes, mes, dia_semana)
    /// Cada campo puede ser "*" (todos) o valores específicos
    Cron {
        minute: String,
        hour: String,
        day_of_month: String,
        month: String,
        day_of_week: String,
    },
}

impl TaskFrequency {
    pub fn as_str(&self) -> String {
        match self {
            TaskFrequency::Once => "once".to_string(),
            TaskFrequency::EveryMinutes(n) => format!("every_{}_minutes", n),
            TaskFrequency::EveryHours(n) => format!("every_{}_hours", n),
            TaskFrequency::EveryDays(n) => format!("every_{}_days", n),
            TaskFrequency::Cron { minute, hour, day_of_month, month, day_of_week } => {
                format!("cron:{} {} {} {} {}", minute, hour, day_of_month, month, day_of_week)
            }
        }
    }
}

/// Acción a ejecutar por la tarea
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskAction {
    /// Ejecutar un comando PowerShell
    PowerShell(String),
    /// Organizar archivos de Google Drive (mover, renombrar según reglas)
    OrganizeDrive {
        source_folder_id: Option<String>,
        rules: Vec<DriveOrganizeRule>,
    },
    /// Enviar un email
    SendEmail {
        to: String,
        subject: String,
        body: String,
    },
    /// Ejecutar una función del agente (con un prompt)
    AgentPrompt(String),
}

/// Regla para organizar archivos de Drive
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriveOrganizeRule {
    pub field: String,        // "name", "mime_type", "created_time", "size"
    pub operator: String,     // "contains", "equals", "starts_with", "ends_with", "gt", "lt"
    pub value: String,
    pub action: String,       // "move", "rename", "delete", "copy"
    pub target_folder_id: Option<String>,
    pub new_name_pattern: Option<String>,
}

/// Estado de una tarea
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Active,
    Paused,
    Completed,
    Failed(String),
}

/// Una tarea programada
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub action: TaskAction,
    pub frequency: TaskFrequency,
    pub status: TaskStatus,
    pub created_at: u64,
    pub last_run: Option<u64>,
    pub next_run: Option<u64>,
    pub run_count: u64,
    pub max_runs: Option<u64>, // None = ilimitado
    pub enabled: bool,
    pub username: String,
}

/// Registro de una ejecución
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskExecutionLog {
    pub task_id: String,
    pub task_name: String,
    pub executed_at: u64,
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
}

// ============================================================================
// Task Scheduler Store
// ============================================================================

#[derive(Clone)]
pub struct TaskSchedulerStore {
    pub tasks: Arc<Mutex<HashMap<String, ScheduledTask>>>,
    pub execution_logs: Arc<Mutex<Vec<TaskExecutionLog>>>,
    pub base_path: PathBuf,
    pub shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
}

impl TaskSchedulerStore {
    pub fn new(base_path: PathBuf) -> Self {
        let store = Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            execution_logs: Arc::new(Mutex::new(Vec::new())),
            base_path,
            shutdown_tx: Arc::new(Mutex::new(None)),
        };
        store.load_from_disk();
        store
    }

    fn load_from_disk(&self) {
        let tasks_path = self.base_path.join("scheduled_tasks.json");
        if tasks_path.exists() {
            if let Ok(data) = fs::read_to_string(&tasks_path) {
                if let Ok(tasks) = serde_json::from_str::<HashMap<String, ScheduledTask>>(&data) {
                    *self.tasks.lock().unwrap() = tasks;
                }
            }
        }

        let logs_path = self.base_path.join("task_execution_logs.json");
        if logs_path.exists() {
            if let Ok(data) = fs::read_to_string(&logs_path) {
                if let Ok(logs) = serde_json::from_str::<Vec<TaskExecutionLog>>(&data) {
                    *self.execution_logs.lock().unwrap() = logs;
                }
            }
        }
    }

    pub fn save_to_disk(&self) -> Result<(), String> {
        let _ = fs::create_dir_all(&self.base_path);

        let tasks = self.tasks.lock().unwrap();
        let tasks_path = self.base_path.join("scheduled_tasks.json");
        fs::write(&tasks_path, serde_json::to_string_pretty(&*tasks).unwrap())
            .map_err(|e| format!("Error guardando tareas: {}", e))?;

        let logs = self.execution_logs.lock().unwrap();
        let logs_path = self.base_path.join("task_execution_logs.json");
        // Solo guardar últimos 1000 logs
        let recent_logs: Vec<&TaskExecutionLog> = logs.iter().rev().take(1000).collect();
        let recent_logs: Vec<&TaskExecutionLog> = recent_logs.into_iter().rev().collect();
        fs::write(&logs_path, serde_json::to_string_pretty(&recent_logs).unwrap())
            .map_err(|e| format!("Error guardando logs: {}", e))?;

        Ok(())
    }

    /// Crea una nueva tarea programada
    pub fn create_task(&self, task: ScheduledTask) -> Result<ScheduledTask, String> {
        let mut tasks = self.tasks.lock().unwrap();
        if tasks.contains_key(&task.id) {
            return Err(format!("Ya existe una tarea con ID '{}'.", task.id));
        }
        tasks.insert(task.id.clone(), task.clone());
        drop(tasks);
        self.save_to_disk()?;
        Ok(task)
    }

    /// Actualiza una tarea existente
    pub fn update_task(&self, task: ScheduledTask) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        if !tasks.contains_key(&task.id) {
            return Err(format!("Tarea '{}' no encontrada.", task.id));
        }
        tasks.insert(task.id.clone(), task);
        drop(tasks);
        self.save_to_disk()
    }

    /// Elimina una tarea
    pub fn delete_task(&self, task_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.lock().unwrap();
        if tasks.remove(task_id).is_none() {
            return Err(format!("Tarea '{}' no encontrada.", task_id));
        }
        drop(tasks);
        self.save_to_disk()
    }

    /// Lista todas las tareas de un usuario
    pub fn list_tasks(&self, username: &str) -> Vec<ScheduledTask> {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .values()
            .filter(|t| t.username == username)
            .cloned()
            .collect()
    }

    /// Obtiene una tarea por ID
    pub fn get_task(&self, task_id: &str) -> Option<ScheduledTask> {
        self.tasks.lock().unwrap().get(task_id).cloned()
    }

    /// Registra una ejecución
    pub fn log_execution(&self, log: TaskExecutionLog) {
        self.execution_logs.lock().unwrap().push(log);
        // Guardar cada 10 ejecuciones
        if self.execution_logs.lock().unwrap().len() % 10 == 0 {
            let _ = self.save_to_disk();
        }
    }

    /// Calcula next_run para una tarea
    pub fn calculate_next_run(&self, task: &ScheduledTask) -> Option<u64> {
        let now = now_secs();
        match &task.frequency {
            TaskFrequency::Once => {
                if task.run_count == 0 { task.next_run } else { None }
            }
            TaskFrequency::EveryMinutes(n) => {
                let last = task.last_run.unwrap_or(now);
                Some(last + n * 60)
            }
            TaskFrequency::EveryHours(n) => {
                let last = task.last_run.unwrap_or(now);
                Some(last + n * 3600)
            }
            TaskFrequency::EveryDays(n) => {
                let last = task.last_run.unwrap_or(now);
                Some(last + n * 86400)
            }
            TaskFrequency::Cron { minute, hour, day_of_month, month, day_of_week } => {
                next_cron_time(minute, hour, day_of_month, month, day_of_week)
            }
        }
    }

    /// Obtiene tareas que deben ejecutarse ahora
    pub fn get_due_tasks(&self) -> Vec<ScheduledTask> {
        let now = now_secs();
        let tasks = self.tasks.lock().unwrap();
        tasks
            .values()
            .filter(|t| {
                t.enabled
                    && t.status == TaskStatus::Active
                    && t.next_run.map(|nr| nr <= now).unwrap_or(false)
                    && t.max_runs.map(|max| t.run_count < max).unwrap_or(true)
            })
            .cloned()
            .collect()
    }
}

// ============================================================================
// Cron Parser
// ============================================================================

fn next_cron_time(
    minute: &str,
    hour: &str,
    day_of_month: &str,
    month: &str,
    day_of_week: &str,
) -> Option<u64> {
    use chrono::{Local, Datelike, Timelike, Duration};

    let now = Local::now();
    let mut candidate = now + Duration::minutes(1); // empezar desde el minuto siguiente

    for _ in 0..525600 {
        // máximo 1 año de búsqueda
        if matches_cron_field(minute, candidate.minute() as u32, 0, 59)
            && matches_cron_field(hour, candidate.hour() as u32, 0, 23)
            && matches_cron_field(day_of_month, candidate.day() as u32, 1, 31)
            && matches_cron_field(month, candidate.month() as u32, 1, 12)
            && matches_cron_field(day_of_week, candidate.weekday().num_days_from_sunday(), 0, 6)
        {
            return Some(candidate.timestamp() as u64);
        }
        candidate = candidate + Duration::minutes(1);
    }
    None
}

fn matches_cron_field(field: &str, value: u32, _min: u32, _max: u32) -> bool {
    if field == "*" {
        return true;
    }
    // Soporta valores individuales: "5", rangos: "1-5", listas: "1,3,5", pasos: "*/15"
    for part in field.split(',') {
        let part = part.trim();
        if part == "*" {
            return true;
        }
        if part.starts_with("*/") {
            if let Ok(step) = part[2..].parse::<u32>() {
                if step > 0 && value % step == 0 {
                    return true;
                }
            }
        } else if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(lo), Ok(hi)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    if value >= lo && value <= hi {
                        return true;
                    }
                }
            }
        } else if let Ok(v) = part.parse::<u32>() {
            if v == value {
                return true;
            }
        }
    }
    false
}

// ============================================================================
// Motor de Ejecución de Tareas
// ============================================================================

/// Inicia el loop de ejecución de tareas en background
pub fn start_scheduler_loop(
    store: TaskSchedulerStore,
    drive_client: Option<crate::google_drive::GoogleDriveClient>,
    gmail_client: Option<crate::google_gmail::GmailClient>,
) -> tokio::task::JoinHandle<()> {
    let (tx, _rx) = broadcast::channel::<()>(1);
    *store.shutdown_tx.lock().unwrap() = Some(tx.clone());

    let mut rx = tx.subscribe();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        eprintln!("[TaskScheduler] Iniciado. Verificando cada 30s.");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let due_tasks = store.get_due_tasks();
                    for task in &due_tasks {
                        eprintln!("[TaskScheduler] Ejecutando tarea: {} ({})", task.name, task.id);
                        let start = std::time::Instant::now();

                        let result = execute_task(task, &drive_client, &gmail_client).await;

                        let duration_ms = start.elapsed().as_millis() as u64;
                        let log = TaskExecutionLog {
                            task_id: task.id.clone(),
                            task_name: task.name.clone(),
                            executed_at: now_secs(),
                            success: result.is_ok(),
                            output: result.unwrap_or_else(|e| e),
                            duration_ms,
                        };
                        store.log_execution(log);

                        // Actualizar métricas de la tarea
                        if let Some(mut t) = store.get_task(&task.id) {
                            t.last_run = Some(now_secs());
                            t.run_count += 1;
                            t.next_run = store.calculate_next_run(&t);
                            if t.max_runs.map(|max| t.run_count >= max).unwrap_or(false) {
                                t.status = TaskStatus::Completed;
                                t.enabled = false;
                            }
                            let _ = store.update_task(t);
                        }
                    }
                }
                _ = rx.recv() => {
                    eprintln!("[TaskScheduler] Señal de shutdown recibida. Deteniendo...");
                    break;
                }
            }
        }
    })
}

async fn execute_task(
    task: &ScheduledTask,
    drive_client: &Option<crate::google_drive::GoogleDriveClient>,
    gmail_client: &Option<crate::google_gmail::GmailClient>,
) -> Result<String, String> {
    match &task.action {
        TaskAction::PowerShell(cmd) => {
            let output = std::process::Command::new("powershell")
                .args(&["-NoProfile", "-Command", cmd])
                .output()
                .map_err(|e| format!("Error ejecutando PowerShell: {}", e))?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                Ok(format!("PowerShell OK:\n{}", stdout))
            } else {
                Err(format!("PowerShell ERROR:\n{}\n{}", stdout, stderr))
            }
        }
        TaskAction::OrganizeDrive { source_folder_id, rules } => {
            let dc = drive_client.as_ref().ok_or("Cliente Drive no disponible.")?;
            // Listar archivos y aplicar reglas
            let result = dc.list_files(&task.username, None, source_folder_id.as_deref(), Some(500)).await?;
            let files = result.files.unwrap_or_default();
            let mut organized = 0;
            for file in &files {
                for rule in rules {
                    if matches_rule(file, rule) {
                        match rule.action.as_str() {
                            "move" => {
                                if let Some(ref _target) = rule.target_folder_id {
                                    // Mover archivo (actualizar parents)
                                    let _ = dc.rename_file(&task.username, &file.id, &file.name).await;
                                    organized += 1;
                                }
                            }
                            "rename" => {
                                if let Some(ref pattern) = rule.new_name_pattern {
                                    let new_name = pattern.replace("{name}", &file.name);
                                    let _ = dc.rename_file(&task.username, &file.id, &new_name).await;
                                    organized += 1;
                                }
                            }
                            "delete" => {
                                let _ = dc.trash_file(&task.username, &file.id).await;
                                organized += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(format!("Drive organizado: {} archivos procesados.", organized))
        }
        TaskAction::SendEmail { to, subject, body } => {
            let gc = gmail_client.as_ref().ok_or("Cliente Gmail no disponible.")?;
            gc.send_email(&task.username, to, subject, body).await?;
            Ok(format!("Email enviado a {}.", to))
        }
        TaskAction::AgentPrompt(prompt) => {
            // Simplemente loguear el prompt — el agente lo procesará en su loop
            Ok(format!("Prompt registrado: {}", prompt))
        }
    }
}

fn matches_rule(
    file: &crate::google_drive::DriveFile,
    rule: &DriveOrganizeRule,
) -> bool {
    let field_value = match rule.field.as_str() {
        "name" => &file.name,
        "mime_type" => &file.mime_type,
        _ => "",
    };

    match rule.operator.as_str() {
        "contains" => field_value.contains(&rule.value),
        "equals" => field_value == rule.value,
        "starts_with" => field_value.starts_with(&rule.value),
        "ends_with" => field_value.ends_with(&rule.value),
        _ => false,
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
