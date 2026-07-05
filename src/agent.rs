
use serde_json::{json, Value};
use std::error::Error;
use std::process::Command;
use crate::state::AppState;
use crate::scraper::{perform_search, scraper_clean_tags};
use std::fs;
use std::path::Path;
use base64::{engine::general_purpose, Engine as _};
use uuid::Uuid;

const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/v1/chat/completions";

pub async fn run_agent_loop(
    session_messages: Vec<crate::state::ChatMessage>,
    project_name: Option<String>,
    state: AppState,
    deepseek_key: &str,
    voyage_key: &str,
    openrouter_key: &str,
    session_id: Option<String>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let global_prompt = {
        let prompts = state.prompts.lock().unwrap();
        prompts.global_current.clone()
    };

    let local_prompt = project_name.as_ref().and_then(|name| {
        let prompts = state.prompts.lock().unwrap();
        prompts.projects.get(name).cloned()
    });

    let mut system_prompt = if let Some(local) = local_prompt {
        format!("{}\n\nProject Specific Prompt:\n{}", global_prompt, local)
    } else {
        global_prompt
    };
    system_prompt.push_str(
        "\n\nOBLIGACIÓN CRÍTICA DE INICIO - CREAR DOCUMENTACIÓN:\n\
         Tu primera e inmediata acción en esta sesión DEBE ser verificar si existe el archivo `DOCUMENTATION.md` en la raíz de tu proyecto actual.\n\
         - SI NO EXISTE: Debes crearlo INMEDIATAMENTE como tu primer paso técnico usando la herramienta `write_file_with_commit` antes de hacer cualquier otra modificación o análisis profundo de código.\n\
         - SI YA EXISTE: Debes leerlo obligatoriamente para orientarte en la arquitectura y actualizarlo si realizas algún cambio estructural.\n\
         \n\
         REQUISITOS DE DOCUMENTACIÓN EXHAUSTIVA:\n\
         Este archivo `DOCUMENTATION.md` NO puede ser un resumen superficial. Debe ser un mapa técnico detallado y exhaustivo de todo el proyecto, conteniendo:\n\
         1. Lista completa de archivos fuente clave del repositorio.\n\
         2. Nombre exacto de todas las estructuras (structs, enums, classes) y funciones principales de cada archivo, detallando su funcionamiento interno específico y dependencias.\n\
         3. Rangos de líneas exactos o aproximados donde se define cada componente importante.\n\
         \n\
         NOTA DE BÚSQUEDA DE CÓDIGO:\n\
         La herramienta `search_code` realiza búsquedas de texto local de coincidencia exacta por términos y palabras clave (ya no utiliza embeddings de VoyageAI). Por ende, el archivo `DOCUMENTATION.md` que crees debe ser rico en términos descriptivos clave (como 'MunicipalFinance', 'tax_system.rs', 'GameWorld', etc.) para que puedas usar `search_code` en el futuro y encontrar la ubicación exacta de cualquier componente en un instante sin necesidad de leer archivos grandes enteros."
    );
    system_prompt.push_str(
        "\n\nNOTA DE CONTEXTO: Para optimizar la memoria y la eficiencia, el sistema puede resumir los mensajes más antiguos del chat en una sola entrada con el encabezado `--- RESUMEN CONTEXTO ANTERIOR (Auto-comprimido por el sistema) ---`. Si encuentras este mensaje, debes interpretarlo como la continuación histórica y fidedigna de los acontecimientos y decisiones tomadas en el proyecto hasta ese momento."
    );

    let mut messages = vec![
        json!({ "role": "system", "content": system_prompt }),
    ];

    // Cargar todo el historial del chat excepto el último mensaje (que es el nuevo prompt del usuario)
    let len = session_messages.len();
    if len > 0 {
        for m in &session_messages[..len - 1] {
            let role = if m.role == "agent" { "assistant" } else { "user" };
            messages.push(json!({ "role": role, "content": m.content }));
        }

        // Inyectar memoria de ejecución reciente (pasos de auditoría de herramientas) si existen
        let steps = {
            let status = state.active_agent.lock().unwrap();
            status.steps.clone()
        };

        if !steps.is_empty() {
            let mut steps_text = String::new();
            // Tomar todos los pasos de auditoría desde el principio para evitar amnesia
            let start_idx = 0;
            for (i, step) in steps.iter().enumerate() {
                // Truncar de forma segura a 1500 caracteres sin romper UTF-8
                let detail_short = if step.detail.chars().count() > 1500 {
                    let truncated: String = step.detail.chars().take(1500).collect();
                    format!("{}... [Truncado en memoria]", truncated)
                } else {
                    step.detail.clone()
                };
                steps_text.push_str(&format!(
                    "Paso #{}: Tipo={}, Título={}\nDetalle: {}\n\n",
                    start_idx + i + 1, step.step_type, step.title, detail_short
                ));
            }

            if !steps_text.is_empty() {
                let context_msg = json!({
                    "role": "system",
                    "content": format!(
                        "--- MEMORIA DE EJECUCIÓN RECIENTE (ACCIONES ANTES DE SER INTERRUMPIDO) ---\n\
                         El agente estaba trabajando en esta sesión y fue interrumpido por el nuevo mensaje del usuario que leerás a continuación. \
                         Aquí tienes el registro técnico de las últimas acciones y herramientas ejecutadas antes del nuevo mensaje. \
                         Analízalo para saber qué archivos modificaste, qué errores obtuviste y qué descubriste para no perder el progreso:\n\n{}",
                        steps_text
                    )
                });
                messages.push(context_msg);
            }
        }

        // Cargar el último mensaje del usuario (el prompt activo)
        let last_msg = &session_messages[len - 1];
        let role = if last_msg.role == "agent" { "assistant" } else { "user" };
        messages.push(json!({ "role": role, "content": last_msg.content }));
    } else {
        // Por si acaso el historial estuviese vacío (no debería ocurrir)
        for m in session_messages {
            let role = if m.role == "agent" { "assistant" } else { "user" };
            messages.push(json!({ "role": role, "content": m.content }));
        }
    }

    let tools = vec![
        json!({
            "type": "function",
            "function": {
                "name": "search_google",
                "description": "Busca información en Google si necesitas datos actualizados.",
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
                "name": "read_file",
                "description": "Lee el contenido de un archivo dentro del proyecto. Permite especificar opcionalmente un rango de líneas (start_line y end_line, indexado desde 1) para leer solo una sección del archivo y ahorrar contexto.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "start_line": { "type": "integer", "description": "Línea inicial a leer (opcional, indexada desde 1)." },
                        "end_line": { "type": "integer", "description": "Línea final a leer (opcional, indexada desde 1, inclusiva)." }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file_with_commit",
                "description": "Modifica o crea un archivo en el proyecto y realiza un commit automático de GitHub. Permite especificar opcionalmente un rango de líneas (start_line y end_line, indexado desde 1) para modificar solo una sección del archivo y ahorrar contexto.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string", "description": "El nuevo contenido a escribir o bloque de reemplazo si se especifican líneas." },
                        "commit_message": { "type": "string" },
                        "start_line": { "type": "integer", "description": "Línea inicial a reemplazar (opcional, indexada desde 1)." },
                        "end_line": { "type": "integer", "description": "Línea final a reemplazar (opcional, indexada desde 1, inclusiva)." }
                    },
                    "required": ["path", "content", "commit_message"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "execute_powershell",
                "description": "Ejecuta comandos de PowerShell en el entorno del proyecto.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "timer": { "type": "integer", "description": "Duración del temporizador en segundos (máx 300). Si se especifica, el comando se ejecuta sin timeout y se inicia un temporizador independiente." }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_code",
                "description": "Busca fragmentos de código semánticamente usando embeddings de VoyageAI.",
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
                "name": "fork_and_clone_repo",
                "description": "Forkea y clona un repositorio de GitHub de terceros mediante GitHub CLI (gh).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "repo_url": { "type": "string" }
                    },
                    "required": ["repo_url"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "read_url",
                "description": "Accede y extrae el texto de una URL pública (página web o documentación).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" }
                    },
                    "required": ["url"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "check_github_cli",
                "description": "Ejecuta comandos de la CLI de GitHub (gh) para autenticarse, verificar credenciales o interactuar con issues, PRs y repositorios.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "notificar_usuario",
                "description": "Permite al agente comunicarse con el usuario durante su ejecución. Puede usarse para dar información o para pausar y hacer preguntas obligatorias de aclaración.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "tipo": { "type": "string", "enum": ["informativo", "pregunta"] },
                        "mensaje": { "type": "string" }
                    },
                    "required": ["tipo", "mensaje"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "finalizar_tarea",
                "description": "Indica explícitamente que el agente ha terminado de resolver la tarea y la da por finalizada.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "mensaje_final": { "type": "string", "description": "Mensaje final de resumen para el usuario detallando todo lo que se ha realizado." }
                    },
                    "required": ["mensaje_final"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "image_fetch",
                "description": "Descarga una imagen desde una URL, la guarda en disco y devuelve un identificador UUID y la ruta del archivo. NO muestra la imagen automáticamente; para verla usa image_view después.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "URL de la imagen a descargar" }
                    },
                    "required": ["url"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "image_view",
                "description": "Inyecta una imagen previamente descargada en el contexto del chat para que puedas verla. La imagen se codifica en Base64 y se envía como contenido multimodal. Usa image_release cuando ya no necesites verla para ahorrar tokens.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "UUID de la imagen obtenido de image_fetch" }
                    },
                    "required": ["id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "image_release",
                "description": "Elimina una imagen del contexto del chat (deja de enviarla a la API en las siguientes iteraciones). El archivo permanece en disco. Úsalo cuando ya no necesites ver la imagen para reducir costos de tokens.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "UUID de la imagen a liberar del contexto" }
                    },
                    "required": ["id"]
                }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "git_resolve_divergence",
                "description": "Resuelve una divergencia entre repositorio local y remoto. Usa 'keep_local' para sobrescribir remoto con local (push --force), 'keep_remote' para descartar local y usar remoto (reset --hard), 'merge_both' para fusionar ambos (pull --rebase --autostash).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["keep_local", "keep_remote", "merge_both"],
                            "description": "Acción para resolver la divergencia."
                        }
                    },
                    "required": ["action"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "analyze_images",
                "description": "Analiza una o varias imágenes locales con un modelo multimodal (Qwen2.5-VL) vía OpenRouter. Permite preguntar sobre el contenido visual, estilo, comparar imágenes, etc.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "image_paths": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Rutas a archivos de imagen locales."
                        },
                        "query": {
                            "type": "string",
                            "description": "Pregunta sobre las imágenes."
                        }
                    },
                    "required": ["image_paths", "query"]
                }
            }
        })
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .tcp_keepalive(std::time::Duration::from_secs(30))
        .build()?;
    let mut iteration = {
        let status = state.active_agent.lock().unwrap();
        status.steps.iter().filter(|s| s.step_type == "thinking").count()
    };

    let mut force_none_tool_choice = true;

    loop {
        // Verificar señal de interrupción
        {
            let status = state.active_agent.lock().unwrap();
            if status.interrupted {
                return Ok("Ejecución del agente interrumpida manualmente por el usuario.".to_string());
            }
        }

        iteration += 1;
        
        {
            let mut status = state.active_agent.lock().unwrap();
            status.steps.push(crate::state::AuditStep {
                step_type: "thinking".to_string(),
                title: format!("Paso de razonamiento {}", iteration),
                detail: "Llamando a DeepSeek para decidir siguientes pasos...".to_string(),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
            // Guardar pasos en disco en tiempo real
            save_chat_steps_to_disk(&state, &session_id, &status.steps);
        }

        // Comprimir el contexto activo acumulado en este turno si se vuelve demasiado grande
        compress_active_messages_if_needed(&state, &session_id, &mut messages, deepseek_key).await;

        // Sanar los mensajes para evitar errores de la API sobre roles "tool" huérfanos
        sanitize_messages_for_api(&mut messages);

        let _ = fs::write(
            state.base_workspace.join("debug_messages.json"),
            serde_json::to_string_pretty(&messages).unwrap_or_default()
        );

        let current_tool_choice = if force_none_tool_choice {
            "none"
        } else {
            "auto"
        };
        force_none_tool_choice = false;

        let mut attempts = 0;
        let res_val: Value = loop {
            attempts += 1;
            let res = client
                .post(DEEPSEEK_API_URL)
                .header("Authorization", format!("Bearer {}", deepseek_key))
                .header("Content-Type", "application/json")
                .json(&json!({
                    "model": "deepseek-v4-pro",
                    "messages": messages,
                    "tools": tools,
                    "tool_choice": current_tool_choice,
                    "thinking": { "type": "enabled" },
                    "reasoning_effort": "high"
                }))
                .send()
                .await;

            match res {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<Value>().await {
                            Ok(val) => break val,
                            Err(e) => {
                                if attempts >= 3 {
                                    return Err(Box::new(e));
                                }
                                println!("Advertencia: Error leyendo/parseando el cuerpo de la respuesta (intento {}/3): {}. Reintentando...", attempts, e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                            }
                        }
                    } else {
                        let status = resp.status();
                        let err_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        if attempts >= 3 {
                            return Err(format!("DeepSeek API returned error status {}: {}", status, err_text).into());
                        }
                        println!("Advertencia: La API retornó status {} (intento {}/3). Reintentando...", status, attempts);
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    }
                }
                Err(e) => {
                    if attempts >= 3 {
                        return Err(Box::new(e));
                    }
                    println!("Advertencia: Error de conexión HTTP (intento {}/3): {}. Reintentando...", attempts, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                }
            }
        };
        let choice = &res_val["choices"][0];
        if choice.is_null() {
            return Err(format!("DeepSeek API returned a response with no choices: {:?}", res_val).into());
        }
        let message_val = &choice["message"];

        let content = message_val["content"].as_str().unwrap_or("");
        if !content.is_empty() {
            {
                let mut status = state.active_agent.lock().unwrap();
                status.steps.push(crate::state::AuditStep {
                    step_type: "informativo".to_string(),
                    title: "Respuesta del Agente".to_string(),
                    detail: content.to_string(),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
                save_chat_steps_to_disk(&state, &session_id, &status.steps);
            }
            
            if let Some(ref s_id) = session_id {
                let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", s_id));
                if chat_file.exists() {
                    if let Ok(content_json) = fs::read_to_string(&chat_file) {
                        if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content_json) {
                            let is_duplicate = session.messages.last().map(|m| m.content == content && m.role == "agent").unwrap_or(false);
                            if !is_duplicate {
                                session.messages.push(crate::state::ChatMessage {
                                    role: "agent".to_string(),
                                    content: content.to_string(),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                });
                                let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                            }
                        }
                    }
                }
            }
        }

        if let Some(tool_calls) = message_val["tool_calls"].as_array() {
            messages.push(message_val.clone());
            let mut tool_responses = Vec::new();
            let mut final_message = None;

            for tool_call in tool_calls {
                // Verificar señal de interrupción antes de cada herramienta
                {
                    let status = state.active_agent.lock().unwrap();
                    if status.interrupted {
                        return Ok("Ejecución del agente interrumpida manualmente antes de ejecutar herramienta.".to_string());
                    }
                }

                let call_id = tool_call["id"].as_str().unwrap_or("");
                let func_name = tool_call["function"]["name"].as_str().unwrap_or("");
                let args_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                if func_name == "notificar_usuario" {
                    let tipo = args["tipo"].as_str().unwrap_or("informativo");
                    if tipo == "pregunta" {
                        force_none_tool_choice = true;
                    }
                }

                {
                    let mut status = state.active_agent.lock().unwrap();
                    status.steps.push(crate::state::AuditStep {
                        step_type: "tool_call".to_string(),
                        title: format!("Ejecutando herramienta: {}", func_name),
                        detail: format!("Argumentos: {}", args_str),
                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                    });
                    save_chat_steps_to_disk(&state, &session_id, &status.steps);
                }

                let tool_result = match func_name {
                    "search_google" => {
                        let query = args["query"].as_str().unwrap_or("");
                        match perform_search(query, state.pending_captcha.clone()).await {
                            Ok(res) => res,
                            Err(e) => format!("Error al buscar en Google: {}", e),
                        }
                    }
                    "read_file" => {
                        let rel_path = args["path"].as_str().unwrap_or("");
                        let start_line_opt = args["start_line"].as_i64();
                        let end_line_opt = args["end_line"].as_i64();
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            let full_path = Path::new(&proj_path).join(rel_path);
                            match fs::read_to_string(&full_path) {
                                Ok(content) => {
                                    if start_line_opt.is_some() || end_line_opt.is_some() {
                                        let lines: Vec<&str> = content.lines().collect();
                                        let total_lines = lines.len();
                                        let start = start_line_opt.unwrap_or(1).max(1) as usize;
                                        let end = end_line_opt.unwrap_or(total_lines as i64).max(1) as usize;
                                        let start_idx = start.saturating_sub(1);
                                        let end_idx = end.min(total_lines);
                                        if start_idx >= total_lines || start_idx > end_idx {
                                            format!("Error: El rango de líneas {}-{} es inválido para un archivo de {} líneas.", start, end, total_lines)
                                        } else {
                                            let chunk = lines[start_idx..end_idx].join("\n");
                                            format!("// Líneas {}-{} de {} en {}\n{}", start_idx + 1, end_idx, total_lines, rel_path, chunk)
                                        }
                                    } else {
                                        content
                                    }
                                }
                                Err(e) => format!("Error leyendo archivo: {}", e),
                            }
                        } else {
                            "No hay ningún proyecto activo seleccionado.".to_string()
                        }
                    }
                    "write_file_with_commit" => {
                        let rel_path = args["path"].as_str().unwrap_or("");
                        let content = args["content"].as_str().unwrap_or("");
                        let commit_msg = args["commit_message"].as_str().unwrap_or("Update by Agent");
                        let start_line_opt = args["start_line"].as_i64();
                        let end_line_opt = args["end_line"].as_i64();
                        
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            let full_path = Path::new(&proj_path).join(rel_path);

                            // --- PASO 1: Sincronizar con el repositorio remoto ANTES de realizar cualquier cambio local ---
                            let mut status_pull = Command::new("git")
                                .args(&["pull", "--rebase", "--autostash", "origin", "master"])
                                .current_dir(&proj_path)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .env("GIT_TERMINAL_PROMPT", "0")
                                .status();
                            
                            // Autocuración en caso de que git pull falle
                            if status_pull.as_ref().map(|s| !s.success()).unwrap_or(true) {
                                println!("Advertencia: git pull falló al inicio (repositorio posiblemente bloqueado o sucio). Iniciando autocuración...");
                                
                                // 1. Abortar cualquier rebase/merge en curso de forma silenciosa
                                let _ = Command::new("git")
                                    .args(&["rebase", "--abort"])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();
                                
                                let _ = Command::new("git")
                                    .args(&["merge", "--abort"])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();

                                // 2. Descartar cualquier cambio local sucio o pendiente en el working directory
                                let _ = Command::new("git")
                                    .args(&["reset", "--hard", "HEAD"])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();
                                
                                let _ = Command::new("git")
                                    .args(&["clean", "-fd"])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();

                                // 3. Forzar eliminación física de carpetas residuales y archivos lock
                                let rebase_merge_path = std::path::Path::new(&proj_path).join(".git").join("rebase-merge");
                                let rebase_apply_path = std::path::Path::new(&proj_path).join(".git").join("rebase-apply");
                                let index_lock_path = std::path::Path::new(&proj_path).join(".git").join("index.lock");
                                if rebase_merge_path.exists() {
                                    let _ = fs::remove_dir_all(&rebase_merge_path);
                                }
                                if rebase_apply_path.exists() {
                                    let _ = fs::remove_dir_all(&rebase_apply_path);
                                }
                                if index_lock_path.exists() {
                                    let _ = fs::remove_file(&index_lock_path);
                                }

                                // 4. Alinear el historial local forzadamente con el repositorio remoto
                                println!("Ejecutando git reset --hard origin/master para alinear el historial local con el remoto...");
                                let _ = Command::new("git")
                                    .args(&["reset", "--hard", "origin/master"])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();

                                // 5. Reintentar pull final
                                status_pull = Command::new("git")
                                    .args(&["pull", "--rebase", "--autostash", "origin", "master"])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();
                            }

                            let pull_success = status_pull.as_ref().map(|s| s.success()).unwrap_or(false);
                            if !pull_success {
                                play_error_beep();
                                return Ok(format!("Error crítico de Git: No se pudo sincronizar el repositorio con la versión remota antes de escribir. Git pull (rebase) falló definitivamente."));
                            }
                            
                            let mut write_success = false;
                            let mut write_err_msg = String::new();
                            let mut is_agent_error = false;
                            
                            if start_line_opt.is_some() || end_line_opt.is_some() {
                                // Edición por rango de líneas en archivo existente
                                match fs::read_to_string(&full_path) {
                                    Ok(orig_content) => {
                                        let line_ending = if orig_content.contains("\r\n") { "\r\n" } else { "\n" };
                                        let mut lines: Vec<String> = orig_content.split(line_ending).map(|s| s.to_string()).collect();
                                        let total_lines = lines.len();
                                        let start = start_line_opt.unwrap_or(1).max(1) as usize;
                                        let end = end_line_opt.unwrap_or(total_lines as i64).max(1) as usize;
                                        let start_idx = start.saturating_sub(1);
                                        let end_idx = end.min(total_lines);
                                        
                                        if start_idx > total_lines || start_idx > end_idx {
                                            write_err_msg = format!("Error: Rango de líneas {}-{} inválido para edición de un archivo de {} líneas.", start, end, total_lines);
                                            is_agent_error = true;
                                        } else {
                                            let replacement_lines: Vec<String> = content.split('\n').map(|s| s.replace('\r', "")).collect();
                                            lines.splice(start_idx..end_idx, replacement_lines);
                                            let new_content = lines.join(line_ending);
                                            match fs::write(&full_path, new_content) {
                                                Ok(_) => { write_success = true; }
                                                Err(e) => { write_err_msg = format!("Error de escritura: {}", e); }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        write_err_msg = format!("Error leyendo el archivo original para edición de líneas: {}", e);
                                    }
                                }
                            } else {
                                // Escritura completa normal (comportamiento original)
                                match fs::write(&full_path, content) {
                                    Ok(_) => { write_success = true; }
                                    Err(e) => { write_err_msg = format!("Error escribiendo archivo: {}", e); }
                                }
                            }
                            
                            if write_success {
                                let status_add = Command::new("git")
                                    .args(&["add", rel_path])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();
                                let status_commit = Command::new("git")
                                    .args(&["commit", "-m", commit_msg])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();
                                let status_push = Command::new("git")
                                    .arg("push")
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();

                                let push_success = status_push.as_ref().map(|s| s.success()).unwrap_or(false);
                                if !push_success {
                                    play_error_beep();
                                }

                                format!(
                                    "Archivo escrito correctamente. Git add: {:?}, Commit: {:?}, Push: {:?}",
                                    status_add, status_commit, status_push
                                )
                            } else {
                                if !is_agent_error {
                                    play_error_beep();
                                }
                                write_err_msg
                            }
                        } else {
                            "No hay ningún proyecto activo seleccionado.".to_string()
                        }
                    }
                    "execute_powershell" => {
                        let command = args["command"].as_str().unwrap_or("");
                        // Optional timer in seconds (max 300). If provided, we run the command without the default 30s timeout
                        let timer_opt = args.get("timer").and_then(|v| v.as_u64());
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            // Detect comandos que normalmente son de larga duración (ej. cargo run, npm start, python main.py)
                            let is_long_running = command.contains("cargo run")
                                || command.contains("npm start")
                                || (command.contains("python") && command.contains("main.py"));

                            // Si es de larga duración o se especificó un timer, usamos spawn sin bloquear
                            if is_long_running || timer_opt.is_some() {
                                match Command::new("powershell")
                                    .args(&["-Command", command])
                                    .current_dir(&proj_path)
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn() {
                                    Ok(child) => {
                                        let pid = child.id();
                                        // Si se pidió un timer, iniciamos una tarea background que avisa al agente cuando expira
                                        if let Some(seconds) = timer_opt {
                                            let pid_copy = pid;
                                            tokio::spawn(async move {
                                                tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
                                                println!("Timer de {}s expiró para PID {}", seconds, pid_copy);
                                            });
                                        }

                                        if is_long_running {
                                            json!({
                                                "message": "Comando de larga duración iniciado en background.",
                                                "pid": pid
                                            }).to_string()
                                        } else {
                                            // Esperamos salida con timeout de 30 s (solo si no hay timer explícito)
                                            let handle = tokio::task::spawn_blocking(move || child.wait_with_output());
                                            match tokio::time::timeout(tokio::time::Duration::from_secs(30), handle).await {
                                                Ok(join_res) => match join_res {
                                                    Ok(Ok(out)) => {
                                                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                                                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                                                        json!({
                                                            "stdout": stdout,
                                                            "stderr": stderr,
                                                            "exit_code": out.status.code(),
                                                            "pid": pid
                                                        }).to_string()
                                                    }
                                                    Ok(Err(e)) => json!({ "error": format!("Error de E/S ejecutando comando: {}", e) }).to_string(),
                                                    Err(e) => json!({ "error": format!("La tarea en segundo plano falló (JoinError): {}", e) }).to_string(),
                                                },
                                                Err(_) => json!({ "error": "El comando excedió el timeout de 30 segundos y continúa corriendo en segundo plano.", "pid": pid }).to_string(),
                                            }
                                        }
                                    }
                                    Err(e) => json!({ "error": format!("Error al iniciar PowerShell: {}", e) }).to_string(),
                                }
                            } else {
                                // Ruta tradicional con timeout de 30 s (comandos cortos)
                                let child = Command::new("powershell")
                                    .args(&["-Command", command])
                                    .current_dir(&proj_path)
                                    .output();
                                match child {
                                    Ok(out) => json!({
                                        "stdout": String::from_utf8_lossy(&out.stdout).to_string(),
                                        "stderr": String::from_utf8_lossy(&out.stderr).to_string(),
                                        "exit_code": out.status.code()
                                    }).to_string(),
                                    Err(e) => json!({ "error": format!("Error al ejecutar PowerShell: {}", e) }).to_string(),
                                }
                            }
                        } else {
                            json!({"error": "No hay ningún proyecto activo seleccionado."}).to_string()
                        }
                    }
                    "search_code" => {
                        let query = args["query"].as_str().unwrap_or("");
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            match semantic_code_search(&proj_path, query, voyage_key).await {
                                Ok(res) => res,
                                Err(e) => format!("Error en búsqueda semántica: {}", e),
                            }
                        } else {
                            json!({"error": "No hay ningún proyecto activo seleccionado."}).to_string()
                        }
                    }
                    "fork_and_clone_repo" => {
                        let repo_url = args["repo_url"].as_str().unwrap_or("");
                        let target_dir = state.base_workspace.clone();
                        // Run gh repo fork --clone
                        let output = Command::new("gh")
                            .args(&["repo", "fork", repo_url, "--clone"])
                            .current_dir(&target_dir)
                            .output();
                        match output {
                            Ok(out) => {
                                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                                
                                // Auto discover projects
                                discover_projects(&state);
 
                                format!("Fork & Clone output:\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr)
                            }
                            Err(e) => format!("Error corriendo gh CLI: {}", e),
                        }
                    }
                    "read_url" => {
                        let url = args["url"].as_str().unwrap_or("");
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36")
                            .timeout(std::time::Duration::from_secs(15))
                            .build();
                        
                        match client {
                            Ok(c) => {
                                match c.get(url).send().await {
                                    Ok(res) => {
                                        match res.text().await {
                                            Ok(body) => {
                                                // Limpiar etiquetas HTML básicas para no saturar tokens
                                                let cleaned = scraper_clean_tags(&body);
                                                if cleaned.len() > 8000 {
                                                    format!("{}... [Truncado por longitud]", safe_truncate(&cleaned, 8000))
                                                } else {
                                                    cleaned
                                                }
                                            }
                                            Err(e) => format!("Error obteniendo texto de respuesta: {}", e),
                                        }
                                    }
                                    Err(e) => format!("Error al conectar con la URL: {}", e),
                                }
                            }
                            Err(e) => format!("Error inicializando cliente HTTP: {}", e),
                        }
                    }
                    "check_github_cli" => {
                        let command = args["command"].as_str().unwrap_or("");
                        let working_dir = if let Some(ref proj_name) = project_name {
                            get_project_path(&state, proj_name)
                        } else {
                            state.base_workspace.to_string_lossy().to_string()
                        };
                        
                        let output = Command::new("gh")
                            .args(command.split_whitespace().collect::<Vec<&str>>()) // Dividir los argumentos de gh
                            .current_dir(&working_dir)
                            .output();
                        match output {
                            Ok(out) => {
                                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                                format!("GH CLI STDOUT:\n{}\nGH CLI STDERR:\n{}", stdout, stderr)
                            }
                            Err(e) => format!("Error ejecutando gh CLI: {}", e),
                        }
                    }
                    "notificar_usuario" => {
                        let tipo = args["tipo"].as_str().unwrap_or("informativo");
                        let mensaje = args["mensaje"].as_str().unwrap_or("");
                        
                        if let Some(ref s_id) = session_id {
                            let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", s_id));
                            if chat_file.exists() {
                                if let Ok(content_json) = fs::read_to_string(&chat_file) {
                                    if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content_json) {
                                        let is_duplicate = session.messages.last().map(|m| m.content == mensaje && m.role == "agent").unwrap_or(false);
                                        if !is_duplicate {
                                            session.messages.push(crate::state::ChatMessage {
                                                role: "agent".to_string(),
                                                content: mensaje.to_string(),
                                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                            });
                                            let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                                        }
                                    }
                                }
                            }
                        }
                        
                        if tipo == "pregunta" {
                            // Cambiar estado a esperando respuesta
                            {
                                let mut status = state.active_agent.lock().unwrap();
                                status.esperando_respuesta_usuario = true;
                                status.pregunta_usuario = Some(mensaje.to_string());
                                status.respuesta_usuario = None;
                                status.steps.push(crate::state::AuditStep {
                                    step_type: "thinking".to_string(),
                                    title: "Agente pausado".to_string(),
                                    detail: format!("Esperando respuesta a la pregunta: {}", mensaje),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                });
                            }
 
                            // Bloquear ciclo asíncronamente con un sleep no bloqueante de Tokio hasta que respuesta_usuario sea Some
                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                
                                // Comprobar si se envió señal de interrupción mientras esperaba
                                {
                                    let status = state.active_agent.lock().unwrap();
                                    if status.interrupted {
                                        return Ok("Ejecución del agente interrumpida mientras esperaba respuesta del usuario.".to_string());
                                    }
                                    if !status.esperando_respuesta_usuario {
                                        if let Some(ref respuesta) = status.respuesta_usuario {
                                            break format!("Respuesta del usuario: {}", respuesta);
                                        }
                                    }
                                }
                            }
                        } else {
                            // Informativo: solo registrar paso
                            {
                                let mut status = state.active_agent.lock().unwrap();
                                status.steps.push(crate::state::AuditStep {
                                    step_type: "informativo".to_string(),
                                    title: "Notificación del Agente".to_string(),
                                    detail: mensaje.to_string(),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                });
                                save_chat_steps_to_disk(&state, &session_id, &status.steps);
                            }
                            format!("Notificación enviada con éxito: {}", mensaje)
                        }
                    }
                    "finalizar_tarea" => {
                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();
                        final_message = Some(msg);
                        "Tarea finalizada correctamente.".to_string()
                    }
                    "image_fetch" => {
                        let url = args["url"].as_str().unwrap_or("");
                        if url.is_empty() {
                            json!({"error": "No se proporcionó URL"}).to_string()
                        } else {
                            let fetch_client = reqwest::Client::builder()
                                .user_agent("Mozilla/5.0")
                                .timeout(std::time::Duration::from_secs(30))
                                .build();
                            match fetch_client {
                                Ok(c) => {
                                    match c.get(url).send().await {
                                        Ok(resp) => {
                                            match resp.bytes().await {
                                                Ok(bytes) => {
                                                    let id = Uuid::new_v4().to_string();
                                                    // Determinar nombre del archivo desde la URL
                                                    let filename = reqwest::Url::parse(url)
                                                        .ok()
                                                        .and_then(|u| u.path_segments()
                                                            .and_then(|s| s.last().map(|s| s.to_string())))
                                                        .unwrap_or_else(|| "image.bin".to_string());
                                                    let safe_name = format!("{}_{}", &id[..8], filename);
                                                    let assets_dir = if let Some(ref proj_name) = project_name {
                                                        let proj_path = get_project_path(&state, proj_name);
                                                        Path::new(&proj_path).join("src").join("assets").join("images")
                                                    } else {
                                                        state.base_workspace.join("assets").join("images")
                                                    };
                                                    let _ = fs::create_dir_all(&assets_dir);
                                                    let full_path = assets_dir.join(&safe_name);
                                                    match fs::write(&full_path, &bytes) {
                                                        Ok(_) => {
                                                            let path_str = full_path.to_string_lossy().to_string();
                                                            {
                                                                let mut store = state.image_store.lock().unwrap();
                                                                store.insert(id.clone(), path_str.clone());
                                                            }
                                                            json!({
                                                                "id": id,
                                                                "path": path_str,
                                                                "message": "Imagen descargada y guardada. Usa image_view para verla."
                                                            }).to_string()
                                                        }
                                                        Err(e) => json!({"error": format!("Error guardando imagen: {}", e)}).to_string(),
                                                    }
                                                }
                                                Err(e) => json!({"error": format!("Error descargando bytes: {}", e)}).to_string(),
                                            }
                                        }
                                        Err(e) => json!({"error": format!("Error conectando a URL: {}", e)}).to_string(),
                                    }
                                }
                                Err(e) => json!({"error": format!("Error creando cliente HTTP: {}", e)}).to_string(),
                            }
                        }
                    }
                    "image_view" => {
                        let id = args["id"].as_str().unwrap_or("");
                        if id.is_empty() {
                    "image_view" => {
                        let id = args["id"].as_str().unwrap_or("");
                        if id.is_empty() {
                            json!({"error": "No se proporcionó ID de imagen"}).to_string()
                        } else {
                            let path_opt = {
                                let store = state.image_store.lock().unwrap();
                                store.get(id).cloned()
                            };
                            match path_opt {
                                Some(img_path) => {
                                    match fs::read(&img_path) {
                                        Ok(bytes) => {
                                            let b64 = general_purpose::STANDARD.encode(&bytes);
                                            let mime_type = mime_guess::from_path(&img_path)
                                                .first_or_octet_stream()
                                                .to_string();
                                            let data_url = format!("data:{};base64,{}", mime_type, b64);

                                            // Llamar a Qwen2.5-VL (DeepSeek no soporta vision)
                                            let api_key = openrouter_key;
                                            let body = json!({
                                                "model": "qwen/qwen2.5-vl-72b-instruct",
                                                "messages": [{
                                                    "role": "user",
                                                    "content": [
                                                        {"type": "text", "text": "Describe detalladamente esta imagen. Incluye elementos visuales, colores, composición, estilo y cualquier texto visible."},
                                                        {"type": "image_url", "image_url": {"url": data_url}}
                                                    ]
                                                }]
                                            });

                                            let client = reqwest::blocking::Client::new();
                                            match client
                                                .post("https://openrouter.ai/api/v1/chat/completions")
                                                .header("Authorization", format!("Bearer {}", api_key))
                                                .header("Content-Type", "application/json")
                                                .header("HTTP-Referer", "https://github.com/iaf")
                                                .json(&body)
                                                .timeout(std::time::Duration::from_secs(120))
                                                .send()
                                            {
                                                Ok(resp) if resp.status().is_success() => {
                                                    match resp.json::<serde_json::Value>() {
                                                        Ok(json_resp) => {
                                                            let description = json_resp["choices"][0]["message"]["content"]
                                                                .as_str().unwrap_or("(Sin respuesta del modelo)")
                                                                .to_string();
                                                            // Inyectar SOLO texto en el contexto (DeepSeek puede leer texto)
                                                            messages.push(json!({
                                                                "role": "user",
                                                                "content": format!("[Sistema] Imagen analizada (id: {}). Descripción:\n\n{}", id, description)
                                                            }));
                                                            json!({
                                                                "message": format!("Imagen '{}' analizada e inyectada en el contexto (solo texto, sin imagen). Usa image_release('{}') cuando no la necesites.", id, id)
                                                            }).to_string()
                                                        }
                                                        Err(e) => json!({"error": format!("Error parseando respuesta: {}", e)}).to_string(),
                                                    }
                                                }
                                                Ok(resp) => {
                                                    let st = resp.status();
                                                    let err = resp.text().unwrap_or_default();
                                                    json!({"error": format!("OpenRouter error {}: {}", st, err)}).to_string()
                                                }
                                                Err(e) => json!({"error": format!("Error de red: {}", e)}).to_string(),
                                            }
                                        }
                                        Err(e) => json!({"error": format!("Error leyendo archivo: {}", e)}).to_string(),
                                    }
                                }
                                None => json!({"error": format!("No se encontró imagen con id '{}'", id)}).to_string(),
                            }
                        }
                    }
                    "image_release" => {
                        let id = args["id"].as_str().unwrap_or("");
                        if id.is_empty() {
                            json!({"error": "No se proporcionó ID de imagen"}).to_string()
                        } else {
                            let marker = format!("(id: {})", id);
                            let before_len = messages.len();
                            messages.retain(|msg| {
                                // Formato texto plano (nuevo)
                                if let Some(text) = msg["content"].as_str() {
                                    if text.contains(&marker) {
                                        return false;
                                    }
                                }
                                // Formato array multimodal (antiguo)
                                if let Some(content_arr) = msg["content"].as_array() {
                                    for part in content_arr {
                                        if let Some(text) = part["text"].as_str() {
                                            if text.contains(&marker) {
                                                return false;
                                            }
                                        }
                                    }
                                }
                                true
                            });
                            let removed = before_len - messages.len();
                            if removed > 0 {
                                json!({"message": format!("Imagen '{}' eliminada del contexto.", id)}).to_string()
                            } else {
                                json!({"message": format!("Imagen '{}' no encontrada en contexto.", id)}).to_string()
                            }
                        }
                    }
                    "git_resolve_divergence" => {
                        let action = args["action"].as_str().unwrap_or("");
                        let proj_path = if let Some(ref proj_name) = project_name {
                            get_project_path(&state, proj_name)
                        } else {
                            return json!({"error": "No hay proyecto activo"}).to_string();
                        };
                        if action.is_empty() {
                            json!({"error": "Se requiere 'action': keep_local, keep_remote o merge_both"}).to_string()
                        } else {
                            match action {
                                "keep_local" => {
                                    match Command::new("git").args(&["push","origin","master","--force"]).current_dir(&proj_path).env("GIT_TERMINAL_PROMPT","0").output() {
                                        Ok(o) if o.status.success() => format!("✅ Push forzado exitoso.\n{}", String::from_utf8_lossy(&o.stdout).trim()),
                                        Ok(o) => format!("❌ Error push: {}", String::from_utf8_lossy(&o.stderr).trim()),
                                        Err(e) => format!("❌ Error: {}", e),
                                    }
                                }
                                "keep_remote" => {
                                    match Command::new("git").args(&["reset","--hard","origin/master"]).current_dir(&proj_path).env("GIT_TERMINAL_PROMPT","0").output() {
                                        Ok(o) if o.status.success() => "✅ Reset exitoso. Local coincide con origin/master.".to_string(),
                                        Ok(o) => format!("❌ Error reset: {}", String::from_utf8_lossy(&o.stderr).trim()),
                                        Err(e) => format!("❌ Error: {}", e),
                                    }
                                }
                                "merge_both" => {
                                    match Command::new("git").args(&["pull","--rebase","--autostash","origin","master"]).current_dir(&proj_path).env("GIT_TERMINAL_PROMPT","0").env("GIT_MERGE_AUTOEDIT","no").output() {
                                        Ok(o) if o.status.success() => format!("✅ Merge/rebase exitoso.\n{}", String::from_utf8_lossy(&o.stdout).trim()),
                                        Ok(o) => {
                                            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                                            if stderr.contains("CONFLICT") || stderr.contains("conflict") {
                                                let _ = Command::new("git").args(&["rebase","--abort"]).current_dir(&proj_path).env("GIT_TERMINAL_PROMPT","0").status();
                                                format!("⚠️ Conflictos. Rebase abortado.\n{}", stderr)
                                            } else { format!("❌ Error merge: {}", stderr) }
                                        }
                                        Err(e) => format!("❌ Error: {}", e),
                                    }
                                }
                                _ => format!("❌ Acción desconocida: '{}'. Usa keep_local, keep_remote o merge_both.", action),
                            }
                        }
                    }
                    "analyze_images" => {
                        let image_paths: Vec<String> = args.get("image_paths")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("Describe estas imágenes.");
                        if image_paths.is_empty() {
                            json!({"error": "Se requiere al menos una imagen"}).to_string()
                        } else {
                            let api_key = openrouter_key;
                            let mut content_parts: Vec<serde_json::Value> = Vec::new();
                            content_parts.push(json!({"type": "text", "text": query}));
                            let mut errors: Vec<String> = Vec::new();
                            let mut processed = 0usize;
                            for path_str in &image_paths {
                                let path = std::path::Path::new(path_str);
                                if !path.exists() { errors.push(format!("No encontrado: {}", path_str)); continue; }
                                match fs::read(path) {
                                    Ok(bytes) => {
                                        if bytes.len() > 4_500_000 { errors.push(format!(">4.5MB: {}", path_str)); continue; }
                                        let mime = match path.extension().and_then(|e| e.to_str()) {
                                            Some("jpg")|Some("jpeg") => "image/jpeg",
                                            Some("png") => "image/png",
                                            Some("gif") => "image/gif",
                                            Some("webp") => "image/webp",
                                            Some("bmp") => "image/bmp",
                                            _ => "image/png",
                                        };
                                        let b64 = general_purpose::STANDARD.encode(&bytes);
                                        content_parts.push(json!({"type": "image_url", "image_url": {"url": format!("data:{};base64,{}", mime, b64)}}));
                                        processed += 1;
                                    }
                                    Err(e) => errors.push(format!("Error {}: {}", path_str, e)),
                                }
                            }
                            if processed == 0 {
                                json!({"error": format!("No procesadas: {}", errors.join("; "))}).to_string()
                            } else {
                                let mut result_text = String::new();
                                if !errors.is_empty() { result_text.push_str(&format!("⚠️ {} errores: {}\n\n", errors.len(), errors.join("; "))); }
                                let body = json!({"model": "qwen/qwen2.5-vl-72b-instruct", "messages": [{"role": "user", "content": content_parts}]});
                                match reqwest::blocking::Client::new()
                                    .post("https://openrouter.ai/api/v1/chat/completions")
                                    .header("Authorization", format!("Bearer {}", api_key))
                                    .header("Content-Type", "application/json")
                                    .header("HTTP-Referer", "https://github.com/iaf")
                                    .json(&body).timeout(std::time::Duration::from_secs(120)).send()
                                {
                                    Ok(resp) if resp.status().is_success() => {
                                        match resp.json::<serde_json::Value>() {
                                            Ok(j) => {
                                                let c = j["choices"][0]["message"]["content"].as_str().unwrap_or("(Sin respuesta)");
                                                result_text.push_str(&format!("📷 Análisis de {} imagen(es):\n\n{}", processed, c));
                                            }
                                            Err(e) => result_text.push_str(&format!("❌ Error parseando: {}", e)),
                                        }
                                    }
                                    Ok(resp) => { result_text.push_str(&format!("❌ OpenRouter error {}: {}", resp.status(), resp.text().unwrap_or_default())); }
                                    Err(e) => result_text.push_str(&format!("❌ Error de red: {}", e)),
                                }
                                result_text
                            }
                        }
                    }
                    _ => "Herramienta desconocida".to_string(),
                };

                {
                    let mut status = state.active_agent.lock().unwrap();
                    status.steps.push(crate::state::AuditStep {
                        step_type: "tool_result".to_string(),
                        title: format!("Resultado de: {}", func_name),
                        detail: if tool_result.len() > 300 {
                            format!("{}... [Truncado]", safe_truncate(&tool_result, 300))
                        } else {
                            tool_result.clone()
                        },
                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                    });
                    save_chat_steps_to_disk(&state, &session_id, &status.steps);
                }

                let display_result = if tool_result.len() > 25000 {
                    format!(
                        "{}... [TRUNCADO POR EL SISTEMA. El resultado es demasiado grande ({} caracteres). Para leer archivos, utiliza parámetros start_line y end_line en 'read_file'. Para comandos de PowerShell, filtra la salida usando select, grep o head/tail.]",
                        safe_truncate(&tool_result, 20000),
                        tool_result.len()
                    )
                } else {
                    tool_result.clone()
                };

                tool_responses.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": display_result
                }));
            }

            for tr in tool_responses {
                messages.push(tr);
            }

            if let Some(msg) = final_message {
                return Ok(msg);
            }
        } else {
            messages.push(message_val.clone());
            messages.push(json!({
                "role": "user",
                "content": "Has respondido con texto pero no has ejecutado ninguna herramienta. Si has finalizado la tarea por completo, llama obligatoriamente a la herramienta 'finalizar_tarea'. Si todavía necesitas realizar cambios, ejecutar comandos o leer archivos, hazlo llamando a la herramienta correspondiente."
            }));
        }
    }
}

fn save_chat_steps_to_disk(state: &AppState, session_id_opt: &Option<String>, steps: &[crate::state::AuditStep]) {
    if let Some(ref session_id) = *session_id_opt {
        let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", session_id));
        if chat_file.exists() {
            if let Ok(content) = fs::read_to_string(&chat_file) {
                if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                    session.steps = Some(steps.to_vec());
                    let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                }
            }
        }
    }
}

fn get_project_path(state: &AppState, name: &str) -> String {
    let projs = state.projects.lock().unwrap();
    projs.iter()
        .find(|p| p.name == name)
        .map(|p| p.path.clone())
        .unwrap_or_else(|| state.base_workspace.join(name).to_string_lossy().to_string())
}

pub fn discover_projects(state: &AppState) {
    let mut projs = state.projects.lock().unwrap();
    projs.clear();
    if let Ok(entries) = fs::read_dir(&state.base_workspace) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name != ".git" && name != "target" && name != "public" {
                        projs.push(crate::state::Project {
                            name: name.to_string(),
                            path: path.to_string_lossy().to_string(),
                            is_local: false,
                        });
                    }
                }
            }
        }
    }
}

async fn semantic_code_search(proj_path: &str, query: &str, _voyage_key: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
    
    let mut matches = Vec::new();

    // Iterate codebase files
    for entry in walkdir::WalkDir::new(proj_path)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ["rs", "js", "ts", "py", "json", "md", "html", "css", "toml"].contains(&ext) {
                if let Ok(content) = fs::read_to_string(path) {
                    let relative_path = path.strip_prefix(proj_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                        
                    // Split content by paragraphs or blocks of lines (e.g. double newlines)
                    let chunks: Vec<&str> = content.split("\n\n").collect();
                    let base_ptr = content.as_ptr() as usize;
                    for chunk in chunks {
                        let chunk_trimmed = chunk.trim();
                        if chunk_trimmed.len() > 10 {
                            let chunk_lower = chunk_trimmed.to_lowercase();
                            let mut score = 0.0;
                            
                            // 1. Match exact query
                            if chunk_lower.contains(&query_lower) {
                                score += 10.0;
                            }
                            
                            // 2. Match keywords
                            let mut keyword_matches = 0;
                            for word in &query_words {
                                if chunk_lower.contains(word) {
                                    keyword_matches += 1;
                                    score += 1.0;
                                }
                            }
                            
                            // Only include if we have at least one keyword match or exact match
                            if score > 0.0 {
                                // Normalize score based on match ratio
                                let keyword_ratio = if !query_words.is_empty() {
                                    keyword_matches as f32 / query_words.len() as f32
                                } else {
                                    1.0
                                };
                                let final_score = score * keyword_ratio;
                                
                                // Calcular líneas exactas del fragmento
                                let chunk_ptr = chunk.as_ptr() as usize;
                                let byte_offset = chunk_ptr - base_ptr;
                                let prefix = &content[..byte_offset];
                                let start_line = prefix.lines().count() + 1;
                                let end_line = start_line + chunk.lines().count() - 1;
                                
                                matches.push((final_score, relative_path.clone(), start_line, end_line, chunk_trimmed.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort matches by score descending
    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    // Deduplicate identical chunks
    matches.dedup_by(|a, b| a.1 == b.1 && a.4 == b.4);

    let mut result_summary = String::new();
    for (score, file, start_line, end_line, chunk) in matches.into_iter().take(8) {
        result_summary.push_str(&format!(
            "--- Matches (score: {:.2}) in {} [Líneas {}-{}] ---\n{}\n\n",
            score, file, start_line, end_line, chunk
        ));
    }

    if result_summary.is_empty() {
        Ok("No se encontraron fragmentos de código que coincidan con la búsqueda.".to_string())
    } else {
        Ok(result_summary)
    }
}

fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        s
    } else {
        let mut end = max_bytes;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}

fn truncate_old_tool_responses(messages: &mut Vec<serde_json::Value>) {
    let mut assistant_count = 0;
    for i in 0..messages.len() {
        if messages[i]["role"] == "assistant" {
            assistant_count += 1;
            // Si ha pasado por 3 o más iteraciones de razonamiento, truncarlo
            if assistant_count >= 3 {
                if let Some(content_val) = messages[i].get_mut("content") {
                    if let Some(content_str) = content_val.as_str() {
                        if content_str.len() > 3000 {
                            let truncated = format!(
                                "{}... [Truncado por el sistema tras 3 iteraciones para ahorrar contexto]",
                                safe_truncate(content_str, 2000)
                            );
                            *content_val = json!(truncated);
                        }
                    }
                }
            }
        }
    }
}

async fn compress_active_messages_if_needed(
    state: &AppState,
    session_id_opt: &Option<String>,
    messages: &mut Vec<serde_json::Value>,
    deepseek_key: &str,
) {
    // Primero, truncar de forma agresiva cualquier resultado de herramienta antiguo para liberar contexto
    truncate_old_tool_responses(messages);

    let total_len: usize = messages.iter()
        .map(|m| {
            let role = m["role"].as_str().unwrap_or("");
            match role {
                "system" => 0, // Excluir prompt del sistema
                "user" | "assistant" => m["content"].as_str().unwrap_or("").len(),
                "tool" => {
                    let content_str = m["content"].as_str().unwrap_or("");
                    if content_str.contains("Truncado por el sistema tras 3 iteraciones") {
                        content_str.len()
                    } else {
                        content_str.len().min(2000) // Contar solo 2000 si está en el periodo de gracia de 3 iteraciones
                    }
                }
                _ => 0,
            }
        })
        .sum();

    if total_len > 500000 && messages.len() >= 4 {
        // Registrar paso en auditoría
        {
            let mut status = state.active_agent.lock().unwrap();
            status.steps.push(crate::state::AuditStep {
                step_type: "thinking".to_string(),
                title: "Compresión de Contexto Activo".to_string(),
                detail: format!(
                    "El contexto de ejecución actual supera los {} caracteres. Comprimiendo el historial activo para evitar sobrecarga...",
                    total_len
                ),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
            save_chat_steps_to_disk(state, session_id_opt, &status.steps);
        }

        // Dejar el primer mensaje (System Prompt) y los últimos 2 mensajes sin comprimir
        let split_idx = messages.len() - 2;
        let messages_to_compress = &messages[1..split_idx];
        
        let mut history_text = String::new();
        for m in messages_to_compress {
            let role = m["role"].as_str().unwrap_or("");
            let content = m["content"].as_str().unwrap_or("");
            let role_str = match role {
                "system" => "Sistema",
                "user" => "Usuario",
                "assistant" => "Agente",
                "tool" => "Herramienta",
                _ => role,
            };
            history_text.push_str(&format!("{}: {}\n\n", role_str, content));
        }

        // Llamar a DeepSeek V4 Flash para compresión
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let payload = json!({
            "model": "deepseek-v4-flash",
            "messages": [
                {
                    "role": "system",
                    "content": "Eres un arquitecto de software y programador principal. Tu tarea es resumir el historial de esta ejecución activa para que el agente de desarrollo (que leerá este resumen como su contexto histórico) pueda continuar trabajando de forma fluida sin perder el hilo y sin exceder su límite de tokens. El resumen debe estar estructurado en español bajo los siguientes puntos:\n1. ¿Qué estaba haciendo el agente y cuál era su objetivo activo?\n2. ¿Qué le faltaba por hacer o qué quedó pendiente/a medias?\n3. ¿Cómo lo estaba haciendo? (Estrategia técnica y enfoque empleado).\n4. ¿Qué archivos estaba editando o analizando activamente?\n5. ¿Qué conocimientos, descubrimientos o conclusiones sobre el código ya tiene claros el agente (para evitar redundancia)?\n\nRedáctalo en un formato directo, estructurado y altamente técnico, sin saludos ni preámbulos."
                },
                {
                    "role": "user",
                    "content": history_text
                }
            ]
        });

        match client
            .post(DEEPSEEK_API_URL)
            .header("Authorization", format!("Bearer {}", deepseek_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(res_val) = resp.json::<serde_json::Value>().await {
                        if let Some(summary_text) = res_val["choices"][0]["message"]["content"].as_str() {
                            let summary_msg = json!({
                                "role": "user",
                                "content": format!(
                                    "--- RESUMEN CONTEXTO DE EJECUCIÓN ACTIVA (Auto-comprimido por el sistema) ---\nEste es un resumen de las acciones y resultados de herramientas anteriores en esta ejecución para mantener la eficiencia:\n\n{}",
                                    summary_text
                                )
                            });

                            let last_messages = messages.split_off(split_idx);
                            let system_prompt = messages.remove(0); // Remover el system prompt temporalmente
                            messages.clear();
                            messages.push(system_prompt); // Volver a poner el system prompt en el índice 0
                            messages.push(summary_msg); // Poner el resumen
                            messages.extend(last_messages); // Añadir los últimos 4 mensajes

                            // Guardar en el archivo JSON de la conversación en disco de forma persistente
                            if let Some(ref session_id) = *session_id_opt {
                                let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", session_id));
                                if chat_file.exists() {
                                    if let Ok(content) = fs::read_to_string(&chat_file) {
                                        if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                                            let mut disk_messages = Vec::new();
                                            for m in messages.iter() {
                                                let role = m["role"].as_str().unwrap_or("");
                                                let content_str = m["content"].as_str().unwrap_or("");
                                                if role == "user" {
                                                    disk_messages.push(crate::state::ChatMessage {
                                                        role: "user".to_string(),
                                                        content: content_str.to_string(),
                                                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                                    });
                                                } else if role == "assistant" {
                                                    disk_messages.push(crate::state::ChatMessage {
                                                        role: "agent".to_string(),
                                                        content: content_str.to_string(),
                                                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                                    });
                                                }
                                            }
                                            session.messages = disk_messages;
                                            let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                                        }
                                    }
                                }
                            }

                            // Registrar éxito en auditoría
                            {
                                let mut status = state.active_agent.lock().unwrap();
                                status.steps.push(crate::state::AuditStep {
                                    step_type: "thinking".to_string(),
                                    title: "Contexto Activo Comprimido".to_string(),
                                    detail: "El contexto de la ejecución activa ha sido comprimido exitosamente para ahorrar tokens.".to_string(),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                });
                                save_chat_steps_to_disk(state, session_id_opt, &status.steps);
                            }
                            return;
                        }
                    }
                }
                eprintln!("Advertencia: La respuesta de la API de compresión activa no fue exitosa.");
            }
            Err(e) => {
                eprintln!("Advertencia: Falló la llamada a la API para comprimir contexto activo: {}", e);
            }
        }
    }
}

fn sanitize_messages_for_api(messages: &mut Vec<serde_json::Value>) {
    let mut i = 0;
    while i < messages.len() {
        // ─────────────────────────────────────────────────────────────
        // 1. Los mensajes con content tipo array (multimodal con
        //    image_url) se preservan intactos. DeepSeek los soporta
        //    correctamente.
        // ─────────────────────────────────────────────────────────────

        // ─────────────────────────────────────────────────────────────
        // 2. Sanar mensajes de herramienta huérfanos
        // ─────────────────────────────────────────────────────────────
        // ─────────────────────────────────────────────────────────────
        if messages[i]["role"] == "tool" {
            // Escanear hacia atrás buscando el primer mensaje que no sea de tipo "tool"
            let mut has_valid_parent = false;
            let mut j = i;
            while j > 0 {
                j -= 1;
                if messages[j]["role"] == "tool" {
                    continue;
                }
                if messages[j]["role"] == "assistant" && messages[j]["tool_calls"].is_array() {
                    // Verificar si el assistant contiene el tool_call_id de la herramienta actual
                    let current_call_id = messages[i]["tool_call_id"].as_str().unwrap_or("");
                    if let Some(tool_calls) = messages[j]["tool_calls"].as_array() {
                        let has_id = tool_calls.iter().any(|tc| tc["id"] == current_call_id);
                        if has_id {
                            has_valid_parent = true;
                        }
                    }
                }
                break;
            }
            
            if !has_valid_parent {
                println!("Sanando mensaje de herramienta huérfano en el índice {}...", i);
                if let Some(obj) = messages[i].as_object_mut() {
                    // Convertir a rol "user" para evitar el error de la API
                    obj.insert("role".to_string(), json!("user"));
                    // Eliminar tool_call_id
                    obj.remove("tool_call_id");
                    // Darle formato de resultado
                    if let Some(content) = obj.get_mut("content") {
                        if let Some(text) = content.as_str() {
                            *content = json!(format!("[Resultado de herramienta] {}", text));
                        }
                    }
                }
            }
        }
        i += 1;
    }
}


pub fn play_error_beep() {
    print!("\x07");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let _ = std::process::Command::new("powershell")
        .args(&["-Command", "[System.Console]::Beep(1000, 500)"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}
