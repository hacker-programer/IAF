# Script to add Google Drive tools to agent.rs
$agent_path = "C:\Users\Fa\Desktop\Auto IAF\IAF\src\agent.rs"
$content = Get-Content $agent_path -Raw

# === STEP 1: Add import after 'use crate::sub_agent;' ===
$import_line = "use crate::google_drive::GoogleDriveClient;"
$content = $content -replace '(use crate::sub_agent;)', "`${1}`n$import_line"

# === STEP 2: Insert tool definitions between no_sync and reportar_fallo ===
# We find the line with 'reportar_fallo' and insert before it
$tool_defs = @'

        json!({
            "type": "function",
            "function": {
                "name": "google_drive_list",
                "description": "Lista archivos y carpetas de Google Drive. Permite buscar por nombre (query), filtrar por carpeta padre (parent_id), y limitar resultados. Requiere que el usuario haya vinculado su cuenta de Google previamente.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Busqueda opcional por nombre (ej: 'name contains proyecto')." },
                        "parent_id": { "type": "string", "description": "ID de la carpeta padre (None = raiz de Drive)." },
                        "max_results": { "type": "integer", "description": "Maximo de resultados a devolver (default 100, max 1000)." }
                    },
                    "required": []
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "google_drive_download",
                "description": "Descarga un archivo de Google Drive al proyecto local. Google Docs se exportan como DOCX, Google Sheets como XLSX, Google Slides como PPTX. Archivos .drawio se descargan como XML editable.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_id": { "type": "string", "description": "ID del archivo en Google Drive." },
                        "save_path": { "type": "string", "description": "Ruta local donde guardar el archivo (relativa al proyecto)." }
                    },
                    "required": ["file_id", "save_path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "google_drive_read_content",
                "description": "Lee el contenido textual de un archivo de Google Drive directamente (sin descargarlo a disco). Para Google Docs extrae texto plano, para Sheets extrae CSV, para archivos .drawio lee el XML. Ideal para leer contenido sin archivos intermedios.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_id": { "type": "string", "description": "ID del archivo en Google Drive." }
                    },
                    "required": ["file_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "google_drive_metadata",
                "description": "Obtiene los metadatos de un archivo/carpeta de Google Drive (nombre, tipo MIME, fechas, tamano, padres, si es drawio, etc.).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_id": { "type": "string", "description": "ID del archivo en Google Drive." }
                    },
                    "required": ["file_id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "google_drive_create_folder",
                "description": "Crea una carpeta en Google Drive.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "folder_name": { "type": "string", "description": "Nombre de la carpeta a crear." },
                        "parent_id": { "type": "string", "description": "ID de la carpeta padre (None = raiz de Drive)." }
                    },
                    "required": ["folder_name"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "google_drive_upload",
                "description": "Sube un archivo local a Google Drive.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "local_path": { "type": "string", "description": "Ruta local del archivo a subir (relativa al proyecto)." },
                        "drive_name": { "type": "string", "description": "Nombre opcional con el que se guardara en Drive (si no, usa el nombre original)." },
                        "parent_id": { "type": "string", "description": "ID de la carpeta padre en Drive (None = raiz)." }
                    },
                    "required": ["local_path"]
                }
            }
        }),
'@

# Insert tool_defs before reportar_fallo
$content = $content -replace '(        json!\(\{[\s\S]*?"name": "reportar_fallo")', "$tool_defs`r`n`$1"

# === STEP 3: Insert tool handlers in dispatch section ===
# Insert after 'kill_sub_agent' handler (before 'fork_and_clone_repo')
$drive_handlers = @'

                    "google_drive_list" => {
                        let query = args["query"].as_str();
                        let parent_id = args["parent_id"].as_str();
                        let max_results = args["max_results"].as_u64().map(|v| v as usize);
                        let drive_client = GoogleDriveClient::new(state.google_auth.clone());
                        match drive_client.list_files(username, query, parent_id, max_results).await {
                            Ok(result) => {
                                let files_display = result.files.as_ref().map(|files| {
                                    if files.is_empty() {
                                        "No se encontraron archivos.".to_string()
                                    } else {
                                        let mut lines: Vec<String> = Vec::with_capacity(files.len() + 1);
                                        lines.push(format!("=== {} archivos encontrados ===", files.len()));
                                        for f in files {
                                            let tipo = if f.is_folder { "[CARPETA]" }
                                                else if f.is_drawio { "[DRAW.IO]" }
                                                else if f.is_google_doc { "[GOOGLE DOC]" }
                                                else if f.is_google_sheet { "[GOOGLE SHEET]" }
                                                else { "[ARCHIVO]" };
                                            let id_trunc = if f.id.len() > 12 { format!("{}...", &f.id[..12]) } else { f.id.clone() };
                                            lines.push(format!("  {} {} | ID: {} | MIME: {} | Mod: {}",
                                                tipo, f.name, id_trunc, f.mime_type,
                                                f.modified_time.as_deref().unwrap_or("?")));
                                        }
                                        lines.join("\n")
                                    }
                                }).unwrap_or_else(|| "Sin archivos.".to_string());
                                format!("{}\nMensaje: {}", files_display, result.message)
                            }
                            Err(e) => format!("Error listando archivos de Google Drive: {}", e),
                        }
                    }
                    "google_drive_download" => {
                        let file_id = args["file_id"].as_str().unwrap_or("");
                        let save_path = args["save_path"].as_str().unwrap_or("");
                        if file_id.is_empty() || save_path.is_empty() {
                            json!({"error": "file_id y save_path son requeridos."}).to_string()
                        } else {
                            let full_save_path = if let Some(ref proj_name) = project_name {
                                let proj_path = get_project_path(&state, proj_name);
                                format!("{}\\{}", proj_path, save_path)
                            } else {
                                save_path.to_string()
                            };
                            let drive_client = GoogleDriveClient::new(state.google_auth.clone());
                            match drive_client.download_file(username, file_id, &full_save_path).await {
                                Ok(result) => {
                                    let mut msg = format!("Descarga exitosa: {}", result.message);
                                    if let Some(ref path) = result.download_path { msg.push_str(&format!("\nGuardado en: {}", path)); }
                                    if let Some(ref f) = result.file {
                                        if f.is_drawio { msg.push_str("\nNOTA: Este es un archivo .drawio (XML). Podes leerlo con read_file para ver/editar el diagrama."); }
                                    }
                                    msg
                                }
                                Err(e) => format!("Error descargando de Google Drive: {}", e),
                            }
                        }
                    }
                    "google_drive_read_content" => {
                        let file_id = args["file_id"].as_str().unwrap_or("");
                        if file_id.is_empty() {
                            json!({"error": "file_id es requerido."}).to_string()
                        } else {
                            let drive_client = GoogleDriveClient::new(state.google_auth.clone());
                            match drive_client.read_file_content(username, file_id).await {
                                Ok(result) => {
                                    let mut msg = result.message;
                                    if let Some(ref f) = result.file {
                                        let label = if f.is_drawio { "[DRAW.IO XML]" }
                                            else if f.is_google_doc { "[GOOGLE DOC - TEXTO]" }
                                            else if f.is_google_sheet { "[GOOGLE SHEET - CSV]" }
                                            else { "[ARCHIVO]" };
                                        msg.push_str(&format!("\nArchivo: {} ({})", f.name, label));
                                    }
                                    if let Some(ref content) = result.content {
                                        msg.push_str(&format!("\n\n{}", content));
                                    }
                                    msg
                                }
                                Err(e) => format!("Error leyendo contenido de Google Drive: {}", e),
                            }
                        }
                    }
                    "google_drive_metadata" => {
                        let file_id = args["file_id"].as_str().unwrap_or("");
                        if file_id.is_empty() {
                            json!({"error": "file_id es requerido."}).to_string()
                        } else {
                            let drive_client = GoogleDriveClient::new(state.google_auth.clone());
                            match drive_client.get_file_metadata(username, file_id).await {
                                Ok(result) => {
                                    match result.file {
                                        Some(f) => {
                                            format!(
                                                "Metadatos de '{}':\n  ID: {}\n  MIME: {}\n  Tipo: {}\n  Es carpeta: {}\n  Es Google Doc: {}\n  Es Google Sheet: {}\n  Es Draw.io: {}\n  Creado: {}\n  Modificado: {}\n  Tamano: {}\n  WebView: {}\n  Padres: {:?}",
                                                f.name, f.id, f.mime_type,
                                                if f.is_folder { "Carpeta" } else if f.is_drawio { "Archivo Draw.io" } else { "Archivo" },
                                                f.is_folder, f.is_google_doc, f.is_google_sheet, f.is_drawio,
                                                f.created_time.as_deref().unwrap_or("?"),
                                                f.modified_time.as_deref().unwrap_or("?"),
                                                f.size.as_deref().unwrap_or("?"),
                                                f.web_view_link.as_deref().unwrap_or("N/A"),
                                                f.parents.as_ref().map(|p| p.join(", ")).unwrap_or_else(|| "[]".to_string())
                                            )
                                        }
                                        None => "No se encontraron metadatos.".to_string(),
                                    }
                                }
                                Err(e) => format!("Error obteniendo metadatos de Google Drive: {}", e),
                            }
                        }
                    }
                    "google_drive_create_folder" => {
                        let folder_name = args["folder_name"].as_str().unwrap_or("");
                        let parent_id = args["parent_id"].as_str();
                        if folder_name.is_empty() {
                            json!({"error": "folder_name es requerido."}).to_string()
                        } else {
                            let drive_client = GoogleDriveClient::new(state.google_auth.clone());
                            match drive_client.create_folder(username, folder_name, parent_id).await {
                                Ok(result) => {
                                    let mut msg = result.message;
                                    if let Some(ref f) = result.file {
                                        msg.push_str(&format!("\n  Nombre: {}\n  ID: {}", f.name, f.id));
                                    }
                                    msg
                                }
                                Err(e) => format!("Error creando carpeta en Google Drive: {}", e),
                            }
                        }
                    }
                    "google_drive_upload" => {
                        let local_path = args["local_path"].as_str().unwrap_or("");
                        let drive_name = args["drive_name"].as_str();
                        let parent_id = args["parent_id"].as_str();
                        if local_path.is_empty() {
                            json!({"error": "local_path es requerido."}).to_string()
                        } else {
                            let full_local_path = if let Some(ref proj_name) = project_name {
                                let proj_path = get_project_path(&state, proj_name);
                                format!("{}\\{}", proj_path, local_path)
                            } else {
                                local_path.to_string()
                            };
                            let drive_client = GoogleDriveClient::new(state.google_auth.clone());
                            match drive_client.upload_file(username, &full_local_path, drive_name, parent_id, None).await {
                                Ok(result) => {
                                    let mut msg = result.message;
                                    if let Some(ref f) = result.file {
                                        msg.push_str(&format!("\n  Nombre en Drive: {}\n  ID: {}\n  MIME: {}", f.name, f.id, f.mime_type));
                                    }
                                    msg
                                }
                                Err(e) => format!("Error subiendo archivo a Google Drive: {}", e),
                            }
                        }
                    }
'@

# Insert drive_handlers before 'fork_and_clone_repo'
$content = $content -replace '(                    "fork_and_clone_repo" =>)', "$drive_handlers`r`n`$1"

Set-Content -Path $agent_path -Value $content -NoNewline
Write-Host "agent.rs updated successfully with Google Drive tools"
