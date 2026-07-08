//! Módulo de sub-agentes para ejecución paralela.
//!
//! Permite al agente principal spawnear múltiples sub-agentes que trabajan
//! en paralelo en tareas independientes, cada uno con:
//! - Restricción de directorios/archivos (evita colisiones)
//! - Contexto heredado (resumen del agente principal)
//! - Canal de resultados para comunicación asíncrona

use std::path::Path;
use serde_json::{json, Value};
use uuid::Uuid;
use crate::state::{AppState, SubAgentStatus};

/// Contexto que el agente principal pasa a un sub-agente.
#[derive(Clone)]
pub struct SubAgentContext {
    /// Resumen de lo que el agente principal ha hecho y sabe.
    pub summary: String,
    /// Archivos/directorios a los que el sub-agente tiene acceso.
    /// Si está vacío, acceso completo.
    pub allowed_paths: Vec<String>,
    /// Descripción de la tarea.
    pub task: String,
    /// Nombre del proyecto.
    pub project_name: Option<String>,
    /// ID único asignado.
    pub id: String,
}

/// Spawnea un sub-agente en una tarea tokio independiente.
/// Retorna el ID del sub-agente.
pub fn spawn_sub_agent(
    state: &AppState,
    task_description: &str,
    project_name: Option<String>,
    allowed_paths: Vec<String>,
    context_summary: Option<String>,
    deepseek_key: String,
) -> Result<String, String> {
    let sub_agents = &state.sub_agents;

    // Verificar límite de paralelismo
    if !sub_agents.can_spawn() {
        return Err(format!(
            "Límite de sub-agentes concurrentes alcanzado (máx: {}). \
            Espera a que alguno termine o cancela uno existente con kill_sub_agent.",
            *sub_agents.max_parallel.lock().unwrap()
        ));
    }
    let id = Uuid::new_v4().to_string();
    let id_short = &id[..8];

    let ctx = SubAgentContext {
        summary: context_summary.unwrap_or_else(|| "Sin contexto heredado.".to_string()),
        allowed_paths: allowed_paths.clone(),
        task: task_description.to_string(),
        project_name: project_name.clone(),
        id: id.clone(),
    };

    // Clonar lo necesario antes de mover ctx al async block
    let allowed_paths_display = if ctx.allowed_paths.is_empty() {
        "acceso completo".to_string()
    } else {
        ctx.allowed_paths.join(", ")
    };
    let summary_clone = ctx.summary.clone();

    let state_clone = state.clone();
    let deepseek_key_clone = deepseek_key.clone();
    let id_clone = id.clone();
    let sub_agents_clone = state.sub_agents.clone();

    // Spawnear la tarea
    let handle = tokio::spawn(async move {
        let result = run_sub_agent(
            &state_clone,
            ctx,
            &deepseek_key_clone,
        )
        .await;

        match result {
            Ok(msg) => {
                sub_agents_clone.update_status(&id_clone, SubAgentStatus::Completed, Some(msg));
            }
            Err(e) => {
                sub_agents_clone.update_status(&id_clone, SubAgentStatus::Failed(e.to_string()), None);
            }
        }
    });

    // Registrar el sub-agente
    sub_agents.register(
        id.clone(),
        task_description.to_string(),
        project_name,
        allowed_paths,
        Some(summary_clone),
        Some(handle.abort_handle()),
    );

    Ok(format!(
        "✅ Sub-agente [{}] spawneado exitosamente.\n\
         Tarea: {}\n\
         Archivos permitidos: {}\n\
         Usa check_sub_agent(\"{}\") para ver su progreso.\n\
         Usa kill_sub_agent(\"{}\") para cancelarlo.",
        id_short,
        task_description,
        allowed_paths_display,
        id_short,
        id_short
    ))

    let path = Path::new(file_path);
    let normalized = path.to_string_lossy().to_lowercase();

    for allowed in allowed_paths {
        let allowed_norm = allowed.to_lowercase();
        if normalized.starts_with(&allowed_norm) {
            return true;
        }
        // También permitir si el path normalizado contiene el path permitido
        if allowed_norm.contains(&normalized) || normalized.contains(&allowed_norm) {
            return true;
        }
    }

    false
}

/// Ejecuta un sub-agente con un conjunto limitado de iteraciones.
async fn run_sub_agent(
    state: &AppState,
    ctx: SubAgentContext,
    deepseek_key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let system_prompt = format!(
        "Eres un SUB-AGENTE de desarrollo (DeepSeek V4 Pro) trabajando en una tarea específica.\n\
         \n\
         CONTEXTO HEREDADO DEL AGENTE PRINCIPAL:\n\
         {}\n\
         \n\
         TU TAREA ESPECÍFICA:\n\
         {}\n\
         \n\
         RESTRICCIONES:\n\
         - Solo puedes modificar archivos en: {}\n\
         - Tienes un máximo de 15 iteraciones.\n\
         - Cuando termines (éxito o fallo), DEBES llamar a finalizar_tarea.\n\
         - No puedes spawnear otros sub-agentes.\n\
         - Reporta tus hallazgos de forma concisa.\n\
         \n\
         REGLAS:\n\
         - Antes de actuar, piensa en <thinking> tags.\n\
         - Usa read_file para entender el código existente.\n\
         - Usa write_file_with_commit para modificar archivos.\n\
         - Usa execute_powershell para ejecutar comandos.\n\
         - Si encuentras un problema que no puedes resolver, repórtalo y finaliza.",
        ctx.summary,
        ctx.task,
        if ctx.allowed_paths.is_empty() { "todos los archivos".to_string() } else { ctx.allowed_paths.join(", ") }
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .tcp_keepalive(std::time::Duration::from_secs(30))
        .build()?;

    let tools = build_sub_agent_tools();

    let mut messages = vec![
        json!({ "role": "system", "content": system_prompt }),
        json!({ "role": "user", "content": format!("Realiza la siguiente tarea: {}", ctx.task) }),
    ];

    let max_iterations = 15;
    let mut iteration = 0;

    loop {
        iteration += 1;
        if iteration > max_iterations {
            return Ok(format!(
                "Límite de {} iteraciones alcanzado. Tarea: {}",
                max_iterations, ctx.task
            ));
        }

        // Verificar interrupción
        {
            let status = state.active_agent.lock().unwrap();
            if status.interrupted {
                return Ok("Sub-agente interrumpido: el agente principal fue interrumpido.".to_string());
            }
        }

        // Comprimir si es necesario
        if messages.len() > 20 {
            let keep_recent = 6;
            let system = messages[0].clone();
            let recent: Vec<_> = messages[messages.len() - keep_recent..].to_vec();
            messages.clear();
            messages.push(system);
            messages.push(json!({
                "role": "user",
                "content": "[Contexto intermedio truncado para ahorrar tokens. Continúa desde donde estabas.]"
            }));
            messages.extend(recent);
        }

        let res = client
            .post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", deepseek_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": "deepseek-v4-pro",
                "messages": messages,
                "tools": tools,
                "tool_choice": "auto",
                "thinking": { "type": "enabled" },
                "reasoning_effort": "medium"
            }))
            .send()
            .await;

        let res_val: Value = match res {
            Ok(resp) if resp.status().is_success() => {
                match resp.json().await {
                    Ok(val) => val,
                    Err(e) => {
                        return Err(format!("Error parseando respuesta: {}", e).into());
                    }
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let err = resp.text().await.unwrap_or_default();
                return Err(format!("API error {}: {}", status, err).into());
            }
            Err(e) => {
                return Err(format!("Error de conexión: {}", e).into());
            }
        };

        let choice = &res_val["choices"][0];
        if choice.is_null() {
            return Err("API retornó respuesta sin choices".into());
        }

        let message_val = &choice["message"];

        if let Some(tool_calls) = message_val["tool_calls"].as_array() {
            messages.push(message_val.clone());

            for tool_call in tool_calls {
                let call_id = tool_call["id"].as_str().unwrap_or("");
                let func_name = tool_call["function"]["name"].as_str().unwrap_or("");
                let args_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                let result = execute_sub_agent_tool(
                    func_name,
                    &args,
                    &ctx,
                    state,
                )
                .await;

                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": result
                }));
            }
        } else {
            let content = message_val["content"].as_str().unwrap_or("");
            messages.push(message_val.clone());

            if !content.is_empty() {
                if content.contains("finalizar_tarea") || content.contains("tarea completada")
                    || content.contains("Tarea finalizada") || content.contains("he terminado")
                {
                    return Ok(format!(
                        "Sub-agente [{}] completó: {}",
                        &ctx.id[..8],
                        content.chars().take(500).collect::<String>()
                    ));
                }

                messages.push(json!({
                    "role": "user",
                    "content": "Si has terminado tu tarea, llama a finalizar_tarea. Si no, continúa trabajando con las herramientas disponibles."
                }));
            }
        }
    }
}

/// Construye las herramientas disponibles para un sub-agente (subconjunto restringido).
fn build_sub_agent_tools() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Lee el contenido de un archivo. Permite especificar rango de líneas.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "start_line": { "type": "integer" },
                        "end_line": { "type": "integer" }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file_with_commit",
                "description": "Modifica o crea un archivo y realiza commit en GitHub.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" },
                        "commit_message": { "type": "string" },
                        "start_line": { "type": "integer" },
                        "end_line": { "type": "integer" }
                    },
                    "required": ["path", "content", "commit_message"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "execute_powershell",
                "description": "Ejecuta comandos de PowerShell.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "timer": { "type": "integer" }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_code",
                "description": "Busca fragmentos de código por palabras clave.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "finalizar_tarea",
                "description": "Finaliza la tarea del sub-agente y reporta el resultado.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "mensaje_final": { "type": "string" }
                    },
                    "required": ["mensaje_final"]
                }
            }
        }),
    ]
}

/// Ejecuta una herramienta para un sub-agente con restricciones de path.
async fn execute_sub_agent_tool(
    func_name: &str,
    args: &Value,
    ctx: &SubAgentContext,
    state: &AppState,
) -> String {
    match func_name {
        "read_file" => {
            let rel_path = args["path"].as_str().unwrap_or("");
            if !is_path_allowed(rel_path, &ctx.allowed_paths) {
                return format!(
                    "⛔ ACCESO DENEGADO: El archivo '{}' no está en tus paths permitidos: {:?}. \
                    Solo puedes acceder a archivos dentro de esos directorios.",
                    rel_path, ctx.allowed_paths
                );
            }

            let start_line_opt = args["start_line"].as_i64();
            let end_line_opt = args["end_line"].as_i64();

            if let Some(ref proj_name) = ctx.project_name {
                let proj_path = get_project_path_from_state(state, proj_name);
                let full_path = Path::new(&proj_path).join(rel_path);
                match std::fs::read_to_string(&full_path) {
                    Ok(content) => {
                        if start_line_opt.is_some() || end_line_opt.is_some() {
                            let lines: Vec<&str> = content.lines().collect();
                            let total = lines.len();
                            let start = start_line_opt.unwrap_or(1).max(1) as usize;
                            let end = end_line_opt.unwrap_or(total as i64).max(1) as usize;
                            let si = start.saturating_sub(1);
                            let ei = end.min(total);
                            if si >= total || si > ei {
                                format!("Rango inválido {}-{} para {} líneas.", start, end, total)
                            } else {
                                format!(
                                    "// Líneas {}-{} de {} en {}\n{}",
                                    si + 1, ei, total, rel_path,
                                    lines[si..ei].join("\n")
                                )
                            }
                        } else {
                            content
                        }
                    }
                    Err(e) => format!("Error leyendo archivo: {}", e),
                }
            } else {
                "No hay proyecto activo.".to_string()
            }
        }

        "write_file_with_commit" => {
            let rel_path = args["path"].as_str().unwrap_or("");
            if !is_path_allowed(rel_path, &ctx.allowed_paths) {
                return format!(
                    "⛔ ACCESO DENEGADO: No puedes escribir en '{}'. Paths permitidos: {:?}",
                    rel_path, ctx.allowed_paths
                );
            }

            let content = args["content"].as_str().unwrap_or("");
            let commit_msg = args["commit_message"].as_str().unwrap_or("sub-agent update");

            if let Some(ref proj_name) = ctx.project_name {
                let proj_path = get_project_path_from_state(state, proj_name);
                let full_path = Path::new(&proj_path).join(rel_path);

                if let Some(parent) = full_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                match std::fs::write(&full_path, content) {
                    Ok(_) => {
                        let _ = std::process::Command::new("git")
                            .args(&["add", rel_path])
                            .current_dir(&proj_path)
                            .stdin(std::process::Stdio::null())
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .status();
                        let _ = std::process::Command::new("git")
                            .args(&["commit", "-m", commit_msg])
                            .current_dir(&proj_path)
                            .stdin(std::process::Stdio::null())
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .status();
                        let _ = std::process::Command::new("git")
                            .arg("push")
                            .current_dir(&proj_path)
                            .stdin(std::process::Stdio::null())
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .status();

                        format!("✅ Archivo '{}' escrito y commiteado: {}", rel_path, commit_msg)
                    }
                    Err(e) => format!("Error escribiendo archivo: {}", e),
                }
            } else {
                "No hay proyecto activo.".to_string()
            }
        }

        "execute_powershell" => {
            let command = args["command"].as_str().unwrap_or("");

            let cmd_lower = command.to_lowercase();
            if cmd_lower.contains("taskkill") || cmd_lower.contains("stop-process") {
                return "[BLOQUEADO] Comando potencialmente peligroso bloqueado.".to_string();
            }

            if let Some(ref proj_name) = ctx.project_name {
                let proj_path = get_project_path_from_state(state, proj_name);
                let output = std::process::Command::new("powershell")
                    .args(&["-Command", command])
                    .current_dir(&proj_path)
                    .output();

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        let exit = out.status.code();
                        if stdout.len() + stderr.len() > 5000 {
                            format!(
                                "exit_code: {:?}\nstdout (truncado): {}...\nstderr: {}",
                                exit,
                                &stdout[..std::cmp::min(3000, stdout.len())],
                                &stderr[..std::cmp::min(1000, stderr.len())]
                            )
                        } else {
                            format!("exit_code: {:?}\nstdout:\n{}\nstderr:\n{}", exit, stdout, stderr)
                        }
                    }
                    Err(e) => format!("Error ejecutando comando: {}", e),
                }
            } else {
                "No hay proyecto activo.".to_string()
            }
        }

        "search_code" => {
            let query = args["query"].as_str().unwrap_or("");
            if let Some(ref proj_name) = ctx.project_name {
                let proj_path = get_project_path_from_state(state, proj_name);
                match crate::agent::search_code_in_project(&proj_path, query, "").await {
                    Ok(results) => results,
                    Err(e) => format!("Error en búsqueda: {}", e),
                }
            } else {
                "No hay proyecto activo.".to_string()
            }
        }

        "finalizar_tarea" => {
            let msg = args["mensaje_final"].as_str().unwrap_or("Tarea completada.");
            format!("TAREA FINALIZADA: {}", msg)
        }

        _ => format!("Herramienta desconocida: {}", func_name),
    }
}

/// Helper para obtener el path de un proyecto desde el AppState.
fn get_project_path_from_state(state: &AppState, name: &str) -> String {
    let projs = state.projects.lock().unwrap();
    projs.iter()
        .find(|p| p.name == name)
        .map(|p| p.path.clone())
        .unwrap_or_else(|| state.base_workspace.join(name).to_string_lossy().to_string())
}
