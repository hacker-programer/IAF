
use serde_json::{json, Value};
use std::error::Error;
use std::process::Command;
use crate::state::AppState;
use crate::validator::validate_file_after_write;
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
        "\n\nOBLIGACIÃ“N CRÃTICA DE INICIO - CREAR DOCUMENTACIÃ“N:\n\
         Tu primera e inmediata acciÃ³n en esta sesiÃ³n DEBE ser verificar si existe el archivo `DOCUMENTATION.md` en la raÃ­z de tu proyecto actual.\n\
         - SI NO EXISTE: Debes crearlo INMEDIATAMENTE como tu primer paso tÃ©cnico usando la herramienta `write_file_with_commit` antes de hacer cualquier otra modificaciÃ³n o anÃ¡lisis profundo de cÃ³digo.\n\
         - SI YA EXISTE: Debes leerlo obligatoriamente para orientarte en la arquitectura y actualizarlo si realizas algÃºn cambio estructural.\n\
         \n\
         REQUISITOS DE DOCUMENTACIÃ“N EXHAUSTIVA:\n\
         Este archivo `DOCUMENTATION.md` NO puede ser un resumen superficial. Debe ser un mapa tÃ©cnico detallado y exhaustivo de todo el proyecto, conteniendo:\n\
         1. Lista completa de archivos fuente clave del repositorio.\n\
         2. Nombre exacto de todas las estructuras (structs, enums, classes) y funciones principales de cada archivo, detallando su funcionamiento interno especÃ­fico y dependencias.\n\
         3. Rangos de lÃ­neas exactos o aproximados donde se define cada componente importante.\n\
         \n\
         NOTA DE BÃšSQUEDA DE CÃ“DIGO:\n\
         La herramienta `search_code` realiza bÃºsquedas de texto local de coincidencia exacta por tÃ©rminos y palabras clave (ya no utiliza embeddings de VoyageAI). Por ende, el archivo `DOCUMENTATION.md` que crees debe ser rico en tÃ©rminos descriptivos clave (como 'MunicipalFinance', 'tax_system.rs', 'GameWorld', etc.) para que puedas usar `search_code` en el futuro y encontrar la ubicaciÃ³n exacta de cualquier componente en un instante sin necesidad de leer archivos grandes enteros."
    );
    system_prompt.push_str(
        "\n\nNOTA DE CONTEXTO: Para optimizar la memoria y la eficiencia, el sistema puede resumir los mensajes mÃ¡s antiguos del chat en una sola entrada con el encabezado `--- RESUMEN CONTEXTO ANTERIOR (Auto-comprimido por el sistema) ---`. Si encuentras este mensaje, debes interpretarlo como la continuaciÃ³n histÃ³rica y fidedigna de los acontecimientos y decisiones tomadas en el proyecto hasta ese momento."
    );

    let mut messages = vec![
        json!({ "role": "system", "content": system_prompt }),
    ];

    // Cargar todo el historial del chat excepto el Ãºltimo mensaje (que es el nuevo prompt del usuario)
    let len = session_messages.len();
    if len > 0 {
        for m in &session_messages[..len - 1] {
            let role = if m.role == "agent" { "assistant" } else { "user" };
            messages.push(json!({ "role": role, "content": m.content }));
        }

        // Inyectar memoria de ejecuciÃ³n reciente (pasos de auditorÃ­a de herramientas) si existen
        let steps = {
            let status = state.active_agent.lock().unwrap();
            status.steps.clone()
        };

        if !steps.is_empty() {
            let mut steps_text = String::new();
            // Tomar todos los pasos de auditorÃ­a desde el principio para evitar amnesia
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
                    "Paso #{}: Tipo={}, TÃ­tulo={}\nDetalle: {}\n\n",
                    start_idx + i + 1, step.step_type, step.title, detail_short
                ));
            }

            if !steps_text.is_empty() {
                let context_msg = json!({
                    "role": "system",
                    "content": format!(
                        "--- MEMORIA DE EJECUCIÃ“N RECIENTE (ACCIONES ANTES DE SER INTERRUMPIDO) ---\n\
                         El agente estaba trabajando en esta sesiÃ³n y fue interrumpido por el nuevo mensaje del usuario que leerÃ¡s a continuaciÃ³n. \
                         AquÃ­ tienes el registro tÃ©cnico de las Ãºltimas acciones y herramientas ejecutadas antes del nuevo mensaje. \
                         AnalÃ­zalo para saber quÃ© archivos modificaste, quÃ© errores obtuviste y quÃ© descubriste para no perder el progreso:\n\n{}",
                        steps_text
                    )
                });
                messages.push(context_msg);
            }
        }

        // Cargar el Ãºltimo mensaje del usuario (el prompt activo)
        let last_msg = &session_messages[len - 1];
        let role = if last_msg.role == "agent" { "assistant" } else { "user" };
        messages.push(json!({ "role": role, "content": last_msg.content }));
    } else {
        // Por si acaso el historial estuviese vacÃ­o (no deberÃ­a ocurrir)
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
                "description": "Busca informaciÃ³n en Google si necesitas datos actualizados.",
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
                "description": "Lee el contenido de un archivo dentro del proyecto. Permite especificar opcionalmente un rango de lÃ­neas (start_line y end_line, indexado desde 1) para leer solo una secciÃ³n del archivo y ahorrar contexto.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "start_line": { "type": "integer", "description": "LÃ­nea inicial a leer (opcional, indexada desde 1)." },
                        "end_line": { "type": "integer", "description": "LÃ­nea final a leer (opcional, indexada desde 1, inclusiva)." }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file_with_commit",
                "description": "Modifica o crea un archivo en el proyecto y realiza un commit automÃ¡tico de GitHub. Permite especificar opcionalmente un rango de lÃ­neas (start_line y end_line, indexado desde 1) para modificar solo una secciÃ³n del archivo y ahorrar contexto.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string", "description": "El nuevo contenido a escribir o bloque de reemplazo si se especifican lÃ­neas." },
                        "commit_message": { "type": "string" },
                        "start_line": { "type": "integer", "description": "LÃ­nea inicial a reemplazar (opcional, indexada desde 1)." },
                        "end_line": { "type": "integer", "description": "LÃ­nea final a reemplazar (opcional, indexada desde 1, inclusiva)." }
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
                        "timer": { "type": "integer", "description": "DuraciÃ³n del temporizador en segundos (mÃ¡x 300). Si se especifica, el comando se ejecuta sin timeout y se inicia un temporizador independiente." }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_code",
                "description": "Busca fragmentos de cÃ³digo mediante coincidencia local de palabras clave en archivos del proyecto (NO usa VoyageAI embeddings; es bÃºsqueda de texto exacta).",
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
                "description": "Accede y extrae el texto de una URL pÃºblica (pÃ¡gina web o documentaciÃ³n).",
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
                "description": "Permite al agente comunicarse con el usuario durante su ejecuciÃ³n. Puede usarse para dar informaciÃ³n o para pausar y hacer preguntas obligatorias de aclaraciÃ³n.",
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
                "description": "Indica explÃ­citamente que el agente ha terminado de resolver la tarea y la da por finalizada.",
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
                "description": "Descarga una imagen desde una URL, la guarda en disco y devuelve un identificador UUID y la ruta del archivo. NO muestra la imagen automÃ¡ticamente; para verla usa image_view despuÃ©s.",
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
                "description": "Inyecta una imagen previamente descargada en el contexto del chat para que puedas verla. La imagen se codifica en Base64 y se envÃ­a como contenido multimodal. Usa image_release cuando ya no necesites verla para ahorrar tokens.",
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
                "description": "Elimina una imagen del contexto del chat (deja de enviarla a la API en las siguientes iteraciones). El archivo permanece en disco. Ãšsalo cuando ya no necesites ver la imagen para reducir costos de tokens.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "UUID de la imagen a liberar del contexto" }
                    },
                    "required": ["id"]
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
                            "description": "AcciÃ³n para resolver la divergencia."
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
                "description": "Analiza una o varias imÃ¡genes locales con un modelo multimodal (Qwen2.5-VL) vÃ­a OpenRouter. Permite preguntar sobre el contenido visual, estilo, comparar imÃ¡genes, etc.",
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
                            "description": "Pregunta sobre las imÃ¡genes."
                        }
                    },
                    "required": ["image_paths", "query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "kill_process",
                "description": "Mata de forma segura un proceso que fue spawnado previamente con execute_powershell. Solo puede matar procesos registrados internamente (los que vos mismo spawnaste). Recibe el PID exacto devuelto por execute_powershell. IMPORTANTE: Esta es la ÃšNICA forma permitida de matar procesos. No uses taskkill ni Stop-Process.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pid": { "type": "integer", "description": "PID del proceso a matar, tal como fue devuelto por execute_powershell." }
                    },
                    "required": ["pid"]
                }
            }
        })
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .tcp_keepalive(std::time::Duration::from_secs(30))
        .build()?;
    let mut iteration = {
        state.active_agent.lock().unwrap().steps.iter().filter(|s| s.step_type == "thinking").count()
    };
    loop {
        // Verificar seÃ±al de interrupciÃ³n
        {
            let status = state.active_agent.lock().unwrap();
            if status.interrupted {
                state.process_registry.kill_all();
                return Ok("EjecuciÃ³n del agente interrumpida manualmente por el usuario.".to_string());
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

        // Sanar los mensajes para evitar errores de la API sobre roles "tool" huÃ©rfanos
        sanitize_messages_for_api(&mut messages);

        // Rate-limiting: solo escribir debug_messages.json cada 5 iteraciones para reducir I/O
        if iteration % 5 == 0 {
            let _ = fs::write(
                state.base_workspace.join("debug_messages.json"),
                serde_json::to_string_pretty(&messages).unwrap_or_default()
            );
        }
        if iteration > 500 {
            let _ = fs::write(
                state.base_workspace.join("debug_messages.json"),
                serde_json::to_string_pretty(&messages).unwrap_or_default()
            );
            state.process_registry.kill_all();
            return Ok(format!(
                "LÃMITE DE SEGURIDAD ALCANZADO: El agente ha ejecutado {} iteraciones. \
                Se ha detenido automÃ¡ticamente para evitar bucles infinitos. \
                RevisÃ¡ debug_messages.json para ver el estado del contexto.",
                iteration
            ));
        }

        // LÃ­mite de pasos de auditorÃ­a: mÃ¡ximo 10000 para evitar archivos JSON gigantes
        {
            let status = state.active_agent.lock().unwrap();
            if status.steps.len() > 10000 {
                drop(status);
                let mut status = state.active_agent.lock().unwrap();
                // Truncar a los Ãºltimos 5000 pasos
                let total = status.steps.len();
                status.steps = status.steps.split_off(total - 5000);
                status.steps.insert(0, crate::state::AuditStep {
                    step_type: "thinking".to_string(),
                    title: "Pasos truncados".to_string(),
                    detail: format!("Se eliminaron {} pasos antiguos para evitar crecimiento excesivo del archivo de auditorÃ­a.", total - 5000),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
            }
        }

        let force_none_tool_choice = false;
        let current_tool_choice = if force_none_tool_choice { "none" } else { "auto" };

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
                        println!("Advertencia: La API retornÃ³ status {} (intento {}/3). Reintentando...", status, attempts);
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    }
                }
                Err(e) => {
                    if attempts >= 3 {
                        return Err(Box::new(e));
                    }
                    println!("Advertencia: Error de conexiÃ³n HTTP (intento {}/3): {}. Reintentando...", attempts, e);
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
                // Verificar seÃ±al de interrupciÃ³n antes de cada herramienta
                {
                    let status = state.active_agent.lock().unwrap();
                    if status.interrupted {
                        state.process_registry.kill_all();
                        return Ok("EjecuciÃ³n del agente interrumpida manualmente antes de ejecutar herramienta.".to_string());
                    }
                }

                let call_id = tool_call["id"].as_str().unwrap_or("");
                let func_name = tool_call["function"]["name"].as_str().unwrap_or("");
                let args_str = tool_call["function"]["arguments"].as_str().unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                if func_name == "notificar_usuario" {
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
                                            format!("Error: El rango de lÃ­neas {}-{} es invÃ¡lido para un archivo de {} lÃ­neas.", start, end, total_lines)
                                        } else {
                                            let chunk = lines[start_idx..end_idx].join("\n");
                                            format!("// LÃ­neas {}-{} de {} en {}\n{}", start_idx + 1, end_idx, total_lines, rel_path, chunk)
                                        }
                                    } else {
                                        content
                                    }
                                }
                                Err(e) => format!("Error leyendo archivo: {}", e),
                            }
                        } else {
                            "No hay ningÃºn proyecto activo seleccionado.".to_string()
                        }
                    }
                    "write_file_with_commit" => {
                        'write_handler: {
                        let rel_path = args["path"].as_str().unwrap_or("");
                        let commit_msg = args["commit_message"].as_str().unwrap_or("Update by Agent");
                        let start_line_opt = args["start_line"].as_i64();
                        let end_line_opt = args["end_line"].as_i64();
                        
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            let full_path = Path::new(&proj_path).join(rel_path);

                            // --- PASO 1: Sincronizar con el repositorio remoto ANTES de realizar cualquier cambio local ---
                            // --- PASO 0: Verificar que el repositorio tiene un remote 'origin' configurado ---
                            // Si no existe, intentar crearlo. Si no se puede, abortar sin tocar archivos locales.
                            let remote_check = Command::new("git")
                                .args(&["remote", "get-url", "origin"])
                                .current_dir(&proj_path)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .env("GIT_TERMINAL_PROMPT", "0")
                                .status();

                            let has_remote = remote_check.as_ref().map(|s| s.success()).unwrap_or(false);

                            if !has_remote {
                                println!("PASO 0: No se detectÃ³ remote 'origin'. Intentando crear repositorio en GitHub...");
                                // Intentar crear el repo en GitHub y configurar origin
                                let gh_result = Command::new("gh")
                                    .args(&["repo", "create", "--source=.", "--push", "--remote=origin", "--public"])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();

                                if gh_result.as_ref().map(|s| s.success()).unwrap_or(false) {
                                    println!("PASO 0: Repositorio creado exitosamente en GitHub. Continuando sincronizaciÃ³n...");
                                } else {
                                    // Verificar si gh estÃ¡ instalado
                                    let gh_available = Command::new("gh")
                                        .args(&["--version"])
                                        .stdin(std::process::Stdio::null())
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .status()
                                        .map(|s| s.success())
                                        .unwrap_or(false);

                                    let error_msg = if gh_available {
                                        format!(
                                            "ERROR DE SINCRONIZACIÃ“N: El proyecto '{}' no tiene un repositorio remoto 'origin' configurado. \
                                            Se intentÃ³ crear uno con 'gh repo create' pero fallÃ³. \
                                            \n\nPara continuar, necesitÃ¡s una de estas opciones:\n\
                                            1. Ejecutar manualmente: cd \"{}\" && gh repo create --source=. --push --remote=origin --public\n\
                                            2. O configurar un remote manualmente: cd \"{}\" && git remote add origin <URL>\n\
                                            3. O crear un repo en GitHub y vincularlo manualmente.\n\n\
                                            Tus archivos locales NO fueron modificados.",
                                            proj_name, proj_path, proj_path
                                        )
                                    } else {
                                        format!(
                                            "ERROR DE SINCRONIZACIÃ“N: El proyecto '{}' no tiene un repositorio remoto 'origin' configurado \
                                            y GitHub CLI (gh) no estÃ¡ instalado en este sistema.\n\n\
                                            Para continuar, necesitÃ¡s:\n\
                                            1. Instalar GitHub CLI: winget install GitHub.cli\n\
                                            2. Autenticarte: gh auth login\n\
                                            3. Luego ejecutar: cd \"{}\" && gh repo create --source=. --push --remote=origin --public\n\n\
                                            O configurar un remote manualmente: cd \"{}\" && git remote add origin <URL>\n\n\
                                            Tus archivos locales NO fueron modificados.",
                                            proj_name, proj_path, proj_path
                                        )
                                    };

                                    // NO retornar error que termine la sesiÃ³n. Devolverlo como resultado de herramienta
                                    // para que el agente pueda informar al usuario y tomar acciÃ³n alternativa.
                                    play_error_beep();
                                    // NO retornar error que termine la sesiÃ³n. Usamos labeled block para
                                    // que el error sea el resultado de la herramienta, no el fin del agente.
                                    play_error_beep();
                                    break 'write_handler error_msg;
                                }
                            }

                            // --- PASO 1: Sincronizar con el repositorio remoto ---
                            let mut status_pull = Command::new("git")
                                .args(&["pull", "--rebase", "--autostash", "origin", "master"])
                                .current_dir(&proj_path)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .env("GIT_TERMINAL_PROMPT", "0")
                                .status();
                            // AutocuraciÃ³n SEGURA en caso de que git pull falle (remote ya verificado)
                            if status_pull.as_ref().map(|s| !s.success()).unwrap_or(true) {
                                println!("Advertencia: git pull fallÃ³ al inicio. Iniciando autocuraciÃ³n SEGURA (remote verificado)...");
                                
                                // 1. Abortar cualquier rebase/merge en curso
                                let _ = Command::new("git")
                                    .args(&["rebase", "--abort"])
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

                                // 2. Resetear a HEAD (seguro: solo descarta cambios locales en staging/working,
                                //    no borra archivos untracked como lo hacÃ­a git clean -fd)
                                let _ = Command::new("git")
                                    .args(&["reset", "--hard", "HEAD"])
                                    .current_dir(&proj_path)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .status();

                                // 3. Eliminar lock files residuales (nunca git clean -fd)
                                let index_lock_path = std::path::Path::new(&proj_path).join(".git").join("index.lock");
                                let rebase_merge_path = std::path::Path::new(&proj_path).join(".git").join("rebase-merge");
                                let rebase_apply_path = std::path::Path::new(&proj_path).join(".git").join("rebase-apply");
                                if index_lock_path.exists() { let _ = fs::remove_file(&index_lock_path); }
                                if rebase_merge_path.exists() { let _ = fs::remove_dir_all(&rebase_merge_path); }
                                if rebase_apply_path.exists() { let _ = fs::remove_dir_all(&rebase_apply_path); }

                                // 4. Alinear con remote (SEGURO: remote ya fue verificado en PASO 0)
                                println!("Ejecutando git reset --hard origin/master (remote verificado)...");
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
                                // NO retornar Err que termine la sesiÃ³n. Usamos break del labeled block.
                                break 'write_handler format!("Error de Git: No se pudo sincronizar con origin/master. \
                                    El remote existe (verificado en PASO 0) pero git pull fallÃ³. \
                                    Posibles causas: branch 'master' no existe en remote, conflictos irresolubles, \
                                    o problemas de red. IntentÃ¡ hacer push inicial si es un repo nuevo.");
                            }
                            
                            let mut write_success = false;
                            let mut write_err_msg = String::new();
                            let mut is_agent_error = false;
                            
                            if start_line_opt.is_some() || end_line_opt.is_some() {
                                // EdiciÃ³n por rango de lÃ­neas en archivo existente
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
                                            write_err_msg = format!("Error: Rango de lÃ­neas {}-{} invÃ¡lido para ediciÃ³n de un archivo de {} lÃ­neas.", start, end, total_lines);
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
                                        write_err_msg = format!("Error leyendo el archivo original para ediciÃ³n de lÃ­neas: {}", e);
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

                                    let full_path_str = full_path.to_string_lossy().to_string();
                                    let validation = validate_file_after_write(&full_path_str, "");
                                    let mut msg = format!(
                                        "Archivo escrito correctamente. Git add: {:?}, Commit: {:?}, Push: {:?}",
                                        status_add, status_commit, status_push
                                    );
                                    msg.push_str(&validation.to_message());
                                    msg
                            } else {
                                if !is_agent_error {
                                    play_error_beep();
                                }
                                write_err_msg
                            }
                        } else {
                            "No hay ningÃºn proyecto activo seleccionado.".to_string()
                        }
                        } // Fin de 'write_handler labeled block
                    }
                    "execute_powershell" => {
                        let command = args["command"].as_str().unwrap_or("");

                        // ========== SANITIZACIÃ“N DE SEGURIDAD ==========
                        // ========== SANITIZACIÃ“N DE SEGURIDAD ==========
                        // Bloquear comandos que intentan matar procesos del sistema.
                        // Esto protege al servidor principal de ser terminado accidentalmente.
                        let command_lower = command.to_lowercase();
                        let forbidden_patterns = [
                            "taskkill",
                            "stop-process",
                            "tskill",
                            "wmic process delete",
                            "wmic process where",
                            "get-process",
                            "kill ",
                            "-name rustc",
                            "-name cargo",
                            "-name iaf",
                            "-im rustc",
                            "-im cargo",
                            "-im iaf",
                            "/im rustc",
                            "/im cargo",
                            "/im iaf",
                        ];
                        let mut blocked_reason: Option<String> = None;
                        for pattern in &forbidden_patterns {
                            if command_lower.contains(pattern) {
                                blocked_reason = Some(format!(
                                    "COMANDO BLOQUEADO POR SEGURIDAD: El comando contiene '{}'. \
                                    Para matar procesos, usÃ¡ la herramienta `kill_process` con el PID. \
                                    EstÃ¡ prohibido usar taskkill, Stop-Process o cualquier comando \
                                    que pueda matar procesos del sistema o al servidor principal.",
                                    pattern
                                ));
                                break;
                            }
                        }
                        if let Some(reason) = blocked_reason {
                            json!({"error": reason}).to_string()
                        } else {

                        // ========== FIN SANITIZACIÃ“N ==========
                        let timer_opt = args.get("timer").and_then(|v| v.as_u64());
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            // Detect comandos que normalmente son de larga duraciÃ³n (ej. cargo run, npm start, python main.py)
                            let is_long_running = command.contains("cargo run")
                                || command.contains("npm start")
                                || (command.contains("python") && command.contains("main.py"));

                            // Si es de larga duraciÃ³n o se especificÃ³ un timer, usamos spawn sin bloquear
                            if is_long_running || timer_opt.is_some() {
                                match Command::new("powershell")
                                    .args(&["-Command", command])
                                    .current_dir(&proj_path)
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn() {
                                    Ok(child) => {
                                        let pid = child.id();
                                        // REGISTRAR EL PID EN EL PROCESS REGISTRY
                                        state.process_registry.register(pid);
                                        // Si se pidiÃ³ un timer, iniciamos una tarea background que avisa al agente cuando expira
                                        if let Some(seconds) = timer_opt {
                                            let pid_copy = pid;
                                            tokio::spawn(async move {
                                                tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
                                                println!("Timer de {}s expirÃ³ para PID {}", seconds, pid_copy);
                                            });
                                        }

                                        if is_long_running {
                                            json!({
                                                "message": "Comando de larga duraciÃ³n iniciado en background.",
                                                "pid": pid
                                            }).to_string()
                                        } else {
                                            // Esperamos salida con timeout de 30â€¯s (solo si no hay timer explÃ­cito)
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
                                                    Err(e) => json!({ "error": format!("La tarea en segundo plano fallÃ³ (JoinError): {}", e) }).to_string(),
                                                },
                                                Err(_) => json!({ "error": "El comando excediÃ³ el timeout de 30 segundos y continÃºa corriendo en segundo plano.", "pid": pid }).to_string(),
                                            }
                                        }
                                    }
                                    Err(e) => json!({ "error": format!("Error al iniciar PowerShell: {}", e) }).to_string(),
                                }
                            } else {
                                // Ruta tradicional con timeout de 30â€¯s (comandos cortos)
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
                            json!({"error": "No hay ningÃºn proyecto activo seleccionado."}).to_string()
                        }
                        } // Fin del else de bloqueo de comandos (blocked_reason)
                    }
                    "search_code" => {
                        let query = args["query"].as_str().unwrap_or("");
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            match search_code_in_project(&proj_path, query, voyage_key).await {
                                Ok(res) => res,
                                Err(e) => format!("Error en bÃºsqueda semÃ¡ntica: {}", e),
                            }
                        } else {
                            json!({"error": "No hay ningÃºn proyecto activo seleccionado."}).to_string()
                        }
                    }
                    "kill_process" => {
                        let pid = args["pid"].as_u64().unwrap_or(0) as u32;
                        if pid == 0 {
                            json!({"error": "PID invÃ¡lido: debe ser un entero positivo."}).to_string()
                        } else {
                            // Usar el ProcessRegistry para matar de forma segura
                            state.process_registry.safe_kill(pid)
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
                            Ok(client) => {
                                match client.get(url).send().await {
                                    Ok(resp) => {
                                        match resp.text().await {
                                            Ok(text) => scraper_clean_tags(&text),
                                            Err(e) => format!("Error leyendo respuesta: {}", e),
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
                        
                        let parsed_args = parse_shell_args(command);
                        let output = Command::new("gh")
                            .args(&parsed_args)
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
                                if let Some(ref s_id) = session_id {
                                    save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);
                                }
                            }
 
                            // Bloquear ciclo asÃ­ncronamente con un sleep no bloqueante de Tokio hasta que respuesta_usuario sea Some
                            let respuesta = loop {
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                
                                // Comprobar si se enviÃ³ seÃ±al de interrupciÃ³n mientras esperaba
                                {
                                    let status = state.active_agent.lock().unwrap();
                                    if status.interrupted {
                                        state.process_registry.kill_all();
                                        return Ok("EjecuciÃ³n del agente interrumpida mientras esperaba respuesta del usuario.".to_string());
                                    }
                                    if !status.esperando_respuesta_usuario {
                                        if let Some(ref respuesta) = status.respuesta_usuario {
                                            break respuesta.clone();
                                        }
                                    }
                                }
                            };
                            format!("Respuesta del usuario: {}", respuesta)
                        } else {
                            // tipo informativo
                            {
                                let mut status = state.active_agent.lock().unwrap();
                                status.steps.push(crate::state::AuditStep {
                                    step_type: "informativo".to_string(),
                                    title: "NotificaciÃ³n del Agente".to_string(),
                                    detail: mensaje.to_string(),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                });
                                if let Some(ref s_id) = session_id {
                                    save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);
                                }
                            }
                            format!("NotificaciÃ³n enviada con Ã©xito: {}", mensaje)
                        }
                    }
                    "finalizar_tarea" => {
                        // Limpiar todos los procesos hijo registrados antes de finalizar
                        state.process_registry.kill_all();
                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();
                        final_message = Some(msg);
                        "Tarea finalizada correctamente.".to_string()
                    }
                    "image_fetch" => {
                        let url = args["url"].as_str().unwrap_or("");
                        if url.is_empty() {
                            json!({"error": "No se proporcionÃ³ URL"}).to_string()
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
                            json!({"error": "No se proporcionÃ³ ID de imagen"}).to_string()
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
                                                        {"type": "text", "text": "Describe detalladamente esta imagen. Incluye elementos visuales, colores, composiciÃ³n, estilo y cualquier texto visible."},
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
                                                                "content": format!("[Sistema] Imagen analizada (id: {}). DescripciÃ³n:\n\n{}", id, description)
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
                                None => json!({"error": format!("No se encontrÃ³ imagen con id '{}'", id)}).to_string(),
                            }
                        }
                    }
                    "image_release" => {
                        let id = args["id"].as_str().unwrap_or("");
                        if id.is_empty() {
                            json!({"error": "No se proporcionÃ³ ID de imagen"}).to_string()
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
                            return Ok(json!({"error": "No hay proyecto activo"}).to_string());
                        };
                        if action.is_empty() {
                            json!({"error": "Se requiere 'action': keep_local, keep_remote o merge_both"}).to_string()
                        } else {
                            match action {
                                "keep_local" => {
                                    match Command::new("git").args(&["push","origin","master","--force"]).current_dir(&proj_path).env("GIT_TERMINAL_PROMPT","0").output() {
                                        Ok(o) if o.status.success() => format!("âœ… Push forzado exitoso.\n{}", String::from_utf8_lossy(&o.stdout).trim()),
                                        Ok(o) => format!("âŒ Error push: {}", String::from_utf8_lossy(&o.stderr).trim()),
                                        Err(e) => format!("âŒ Error: {}", e),
                                    }
                                }
                                "keep_remote" => {
                                    match Command::new("git").args(&["reset","--hard","origin/master"]).current_dir(&proj_path).env("GIT_TERMINAL_PROMPT","0").output() {
                                        Ok(o) if o.status.success() => "âœ… Reset exitoso. Local coincide con origin/master.".to_string(),
                                        Ok(o) => format!("âŒ Error reset: {}", String::from_utf8_lossy(&o.stderr).trim()),
                                        Err(e) => format!("âŒ Error: {}", e),
                                    }
                                }
                                "merge_both" => {
                                    match Command::new("git").args(&["pull","--rebase","--autostash","origin","master"]).current_dir(&proj_path).env("GIT_TERMINAL_PROMPT","0").env("GIT_MERGE_AUTOEDIT","no").output() {
                                        Ok(o) if o.status.success() => format!("âœ… Merge/rebase exitoso.\n{}", String::from_utf8_lossy(&o.stdout).trim()),
                                        Ok(o) => {
                                            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
                                            if stderr.contains("CONFLICT") || stderr.contains("conflict") {
                                                let _ = Command::new("git").args(&["rebase","--abort"]).current_dir(&proj_path).env("GIT_TERMINAL_PROMPT","0").status();
                                                format!("âš ï¸ Conflictos. Rebase abortado.\n{}", stderr)
                                            } else { format!("âŒ Error merge: {}", stderr) }
                                        }
                                        Err(e) => format!("âŒ Error: {}", e),
                                    }
                                }
                                _ => format!("âŒ AcciÃ³n desconocida: '{}'. Usa keep_local, keep_remote o merge_both.", action),
                            }
                        }
                    }
                    "analyze_images" => {
                        let image_paths: Vec<String> = args.get("image_paths")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("Describe estas imÃ¡genes.");
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
                                if !errors.is_empty() { result_text.push_str(&format!("âš ï¸ {} errores: {}\n\n", errors.len(), errors.join("; "))); }
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
                                                if let Some(choices) = j["choices"].as_array() {
                                                    if let Some(first) = choices.first() {
                                                        if let Some(msg) = first["message"].as_object() {
                                                            if let Some(content) = msg["content"].as_str() {
                                                                result_text.push_str(content);
                                                            }
                                                        }
                                                    }
                                                }
                                                if result_text.is_empty() {
                                                    result_text.push_str(&format!("Respuesta: {:?}", j));
                                                }
                                                result_text
                                            }
                                            Err(e) => format!("Error parseando respuesta: {}", e),
                                        }
                                    }
                                    Ok(resp) => format!("Error HTTP {}: {}", resp.status(), resp.text().unwrap_or_default()),
                                    Err(e) => format!("Error de conexiÃ³n: {}", e),
                                }
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
                        "{}... [VISUALIZACIÓN PARCIAL — El archivo en disco NO está truncado. Solo se muestra una parte de la respuesta de la herramienta por ser demasiado grande ({} caracteres). Para leer archivos, utiliza parÃ¡metros start_line y end_line en 'read_file'. Para comandos de PowerShell, filtra la salida usando select, grep o head/tail.]",
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
                state.process_registry.kill_all();
                return Ok(msg);
            }
        } else {
            messages.push(message_val.clone());
            messages.push(json!({
                "role": "user",
                "content": "Has respondido con texto pero no has ejecutado ninguna herramienta. Si has finalizado la tarea por completo, llama obligatoriamente a la herramienta 'finalizar_tarea'. Si todavÃ­a necesitas realizar cambios, ejecutar comandos o leer archivos, hazlo llamando a la herramienta correspondiente."
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
    // Preservar proyectos locales agregados manualmente; solo eliminar los auto-descubiertos
    projs.retain(|p| p.is_local);
    if let Ok(entries) = fs::read_dir(&state.base_workspace) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name != ".git" && name != "target" && name != "public" && name != ".config" {
                        if !projs.iter().any(|p| p.name == name) {
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
}

async fn search_code_in_project(proj_path: &str, query: &str, voyage_key: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    semantic_code_search(proj_path, query, voyage_key).await
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
                                
                                // Calcular lÃ­neas exactas del fragmento
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
            "--- Matches (score: {:.2}) in {} [LÃ­neas {}-{}] ---\n{}\n\n",
            score, file, start_line, end_line, chunk
        ));
    }

    if result_summary.is_empty() {
        Ok("No se encontraron fragmentos de cÃ³digo que coincidan con la bÃºsqueda.".to_string())
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
            // Si ha pasado por 3 o mÃ¡s iteraciones de razonamiento, truncarlo
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
                        content_str.len().min(2000) // Contar solo 2000 si estÃ¡ en el periodo de gracia de 3 iteraciones
                    }
                }
                _ => 0,
            }
        })
        .sum();

    if total_len > 500000 && messages.len() >= 4 {
        // Registrar paso en auditorÃ­a
        {
            let mut status = state.active_agent.lock().unwrap();
            status.steps.push(crate::state::AuditStep {
                step_type: "thinking".to_string(),
                title: "CompresiÃ³n de Contexto Activo".to_string(),
                detail: format!(
                    "El contexto de ejecuciÃ³n actual supera los {} caracteres. Comprimiendo el historial activo para evitar sobrecarga...",
                    total_len
                ),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
            save_chat_steps_to_disk(state, session_id_opt, &status.steps);
        }

        // Dejar el primer mensaje (System Prompt) y los Ãºltimos 2 mensajes sin comprimir
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

        // Llamar a DeepSeek V4 Flash para compresiÃ³n
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
                    "content": "Eres un arquitecto de software y programador principal. Tu tarea es resumir el historial de esta ejecuciÃ³n activa para que el agente de desarrollo (que leerÃ¡ este resumen como su contexto histÃ³rico) pueda continuar trabajando de forma fluida sin perder el hilo y sin exceder su lÃ­mite de tokens. El resumen debe estar estructurado en espaÃ±ol bajo los siguientes puntos:\n1. Â¿QuÃ© estaba haciendo el agente y cuÃ¡l era su objetivo activo?\n2. Â¿QuÃ© le faltaba por hacer o quÃ© quedÃ³ pendiente/a medias?\n3. Â¿CÃ³mo lo estaba haciendo? (Estrategia tÃ©cnica y enfoque empleado).\n4. Â¿QuÃ© archivos estaba editando o analizando activamente?\n5. Â¿QuÃ© conocimientos, descubrimientos o conclusiones sobre el cÃ³digo ya tiene claros el agente (para evitar redundancia)?\n\nRedÃ¡ctalo en un formato directo, estructurado y altamente tÃ©cnico, sin saludos ni preÃ¡mbulos."
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
                                    "--- RESUMEN CONTEXTO DE EJECUCIÃ“N ACTIVA (Auto-comprimido por el sistema) ---\nEste es un resumen de las acciones y resultados de herramientas anteriores en esta ejecuciÃ³n para mantener la eficiencia:\n\n{}",
                                    summary_text
                                )
                            });

                            let last_messages = messages.split_off(split_idx);
                            let system_prompt = messages.remove(0); // Remover el system prompt temporalmente
                            messages.clear();
                            messages.push(system_prompt); // Volver a poner el system prompt en el Ã­ndice 0
                            messages.push(summary_msg); // Poner el resumen
                            messages.extend(last_messages); // AÃ±adir los Ãºltimos 4 mensajes

                            // Guardar en el archivo JSON de la conversaciÃ³n en disco de forma persistente
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

                            // Registrar Ã©xito en auditorÃ­a
                            {
                                let mut status = state.active_agent.lock().unwrap();
                                status.steps.push(crate::state::AuditStep {
                                    step_type: "thinking".to_string(),
                                    title: "Contexto Activo Comprimido".to_string(),
                                    detail: "El contexto de la ejecuciÃ³n activa ha sido comprimido exitosamente para ahorrar tokens.".to_string(),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                });
                                save_chat_steps_to_disk(state, session_id_opt, &status.steps);
                            }
                            return;
                        }
                    }
                    // Si llegamos aquÃ­, la compresiÃ³n fallÃ³ o fue incompleta
                    // Si llegamos aquÃ­, la compresiÃ³n fallÃ³ o fue incompleta
                    // Fallback: truncar mensajes viejos de forma agresiva
                    if messages.len() > 10 {
                        // Mantener system prompt + Ãºltimos 4 mensajes
                        let keep_start = 1; // system prompt
                        let keep_end = messages.len().saturating_sub(4);
                        if keep_end > keep_start {
                            // Insertar un marcador de truncado
                            let marker = json!({
                                "role": "user",
                                "content": "[Contexto truncado automÃ¡ticamente para mantenerse dentro del lÃ­mite de tokens]"
                            });
                            let system = messages[0].clone();
                            let last_few: Vec<_> = messages[keep_end..].to_vec();
                            messages.clear();
                            messages.push(system);
                            messages.push(marker);
                            messages.extend(last_few);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Advertencia: FallÃ³ la llamada a la API para comprimir contexto activo: {}", e);
            }
        }
    }
}
/// Parsea una lÃ­nea de comandos shell respetando comillas dobles y simples.
/// Ej: 'gh repo create "my repo" --public' â†’ ["gh", "repo", "create", "my repo", "--public"]
fn parse_shell_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    
    for ch in input.chars() {
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

pub fn play_error_beep() {
    // Reproducir un beep del sistema para alertar al usuario
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("powershell")
            .args(&["-c", "[System.Console]::Beep(800, 200)"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

fn sanitize_messages_for_api(messages: &mut Vec<serde_json::Value>) {
    let mut i = 0;
    while i < messages.len() {
        // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // 1. Los mensajes con content tipo array (multimodal con
        //    image_url) se preservan intactos. DeepSeek los soporta
        //    correctamente.
        // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

        // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // 2. Sanar mensajes de herramienta huÃ©rfanos
        // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        if messages[i]["role"] == "tool" {
            // Escanear hacia atrÃ¡s buscando el primer mensaje que no sea de tipo "tool"
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
                println!("Sanando mensaje de herramienta huÃ©rfano en el Ã­ndice {}...", i);
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
