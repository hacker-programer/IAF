#!/usr/bin/env python3
"""
Aplica los 3 fixes a agent.rs de forma atómica:
1. BUG-002: notificar_usuario else -> agrega a info_messages
2. BUG-004: finalizar_tarea -> multi-línea legible
3. BUG-001: read_file -> soporte PDF/DOCX
"""
import re

with open('src/agent.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# ─── Fix 1: notificar_usuario else branch (BUG-002) ───
old_notificar_else = '''                        } else {
                            // tipo informativo
                            {
                                let mut status = state.active_agent.lock().unwrap();
                                status.steps.push(crate::state::AuditStep {
                                    step_type: "informativo".to_string(),
                                    title: "Notificación del Agente".to_string(),
                                    detail: mensaje.to_string(),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                });
                                if let Some(ref s_id) = session_id {
                                    save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);
                                }
                            }
                            format!("Notificación enviada con éxito: {}", mensaje)
                        }'''

new_notificar_else = '''                        } else {
                            // tipo informativo
                            {
                                let mut status = state.active_agent.lock().unwrap();
                                // Agregar a info_messages para frontend (BUG-002)
                                status.info_messages.push(mensaje.to_string());
                                if status.info_messages.len() > 100 {
                                    status.info_messages.remove(0);
                                }
                                status.steps.push(crate::state::AuditStep {
                                    step_type: "informativo".to_string(),
                                    title: "Notificación del Agente".to_string(),
                                    detail: mensaje.to_string(),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                });
                                if let Some(ref s_id) = session_id {
                                    save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);
                                }
                            }
                            format!("Notificación enviada con éxito: {}", mensaje)
                        }'''

if old_notificar_else in content:
    content = content.replace(old_notificar_else, new_notificar_else)
    print("✓ Fix 1 (notificar_usuario) aplicado")
else:
    print("✗ Fix 1: no se encontró el patrón notificar_usuario")
    # Debug: find similar text
    idx = content.find("tipo informativo")
    if idx > 0:
        print(f"  'tipo informativo' encontrado en offset {idx}")
        print(f"  Contexto: ...{content[idx:idx+200]}...")

# ─── Fix 2: finalizar_tarea (BUG-004) ───
# La versión original está toda en una línea. Buscamos el patrón exacto.
old_finalizar = '''"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }'''

new_finalizar = '''"finalizar_tarea" => {
                        // Limpiar todos los procesos hijo registrados antes de finalizar
                        state.process_registry.kill_all();

                        let msg = args["mensaje_final"]
                            .as_str()
                            .unwrap_or("Tarea finalizada.")
                            .to_string();

                        // Validar que el mensaje no esté vacío
                        let final_msg = if msg.trim().is_empty() {
                            "Tarea finalizada.".to_string()
                        } else {
                            msg
                        };

                        // Actualizar estado del agente para que el frontend lo detecte
                        {
                            let mut status = state.active_agent.lock().unwrap();
                            status.finished = true;
                            status.final_message = Some(final_msg.clone());
                            status.running = false;
                            status.esperando_respuesta_usuario = false;
                            status.esperando_aprobacion_plan = false;
                            // Limpiar info_messages al finalizar (BUG-004)
                            status.info_messages.clear();
                            status.steps.push(crate::state::AuditStep {
                                step_type: "thinking".to_string(),
                                title: "Tarea Finalizada".to_string(),
                                detail: format!("El agente ha finalizado la tarea: {}", final_msg),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            });
                            if let Some(ref s_id) = session_id {
                                save_chat_steps_to_disk(
                                    &state,
                                    &Some(s_id.clone()),
                                    &status.steps,
                                );
                            }
                        }
                        final_message = Some(final_msg);
                        "Tarea finalizada correctamente.".to_string()
                    }'''

if old_finalizar in content:
    content = content.replace(old_finalizar, new_finalizar)
    print("✓ Fix 2 (finalizar_tarea) aplicado")
else:
    print("✗ Fix 2: no se encontró el patrón finalizar_tarea")
    # Try to find it
    idx = content.find('"finalizar_tarea"')
    if idx > 0:
        snippet = content[idx:idx+500]
        print(f"  Encontrado en offset {idx}: {snippet[:200]}...")

# ─── Fix 3: read_file PDF/DOCX support (BUG-001) ───
old_read_file = '''                    "read_file" => {
                        let rel_path = args["path"].as_str().unwrap_or("");
                        let start_line_opt = args["start_line"].as_i64();
                        let end_line_opt = args["end_line"].as_i64();
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            let full_path = Path::new(&proj_path).join(rel_path);
                            match fs::read_to_string(&full_path) {'''

new_read_file = '''                    "read_file" => {
                        let rel_path = args["path"].as_str().unwrap_or("");
                        let start_line_opt = args["start_line"].as_i64();
                        let end_line_opt = args["end_line"].as_i64();
                        if let Some(ref proj_name) = project_name {
                            let proj_path = get_project_path(&state, proj_name);
                            let full_path = Path::new(&proj_path).join(rel_path);

                            // Detectar extensión para formatos especiales (BUG-001)
                            let extension = full_path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();

                            // --- PDF: intentar extraer texto ---
                            if extension == "pdf" {
                                let pdf_path_str = full_path.to_string_lossy().to_string();
                                match std::process::Command::new("pdftotext")
                                    .args(["-layout", &pdf_path_str, "-"])
                                    .output()
                                {
                                    Ok(out) if out.status.success() => {
                                        let text = String::from_utf8_lossy(&out.stdout).to_string();
                                        if text.trim().is_empty() {
                                            "El PDF fue procesado pero no contiene texto extraíble (puede ser escaneado). Prueba con analyze_images para OCR.".to_string()
                                        } else {
                                            format!("[PDF extraído: {}]\\n\\n{}", rel_path, text)
                                        }
                                    }
                                    _ => {
                                        match std::process::Command::new("python")
                                            .args(["-c", &format!(
                                                "import sys;\\ntry:\\n from PyPDF2 import PdfReader\\n r = PdfReader(r'{}')\\n for p in r.pages:\\n  t = p.extract_text()\\n  if t: print(t)\\nexcept ImportError:\\n sys.exit(1)",
                                                pdf_path_str.replace("\\", "\\\\").replace("'", "\\'")
                                            )])
                                            .output()
                                        {
                                            Ok(out) if out.status.success() => {
                                                let text = String::from_utf8_lossy(&out.stdout).to_string();
                                                if text.trim().is_empty() {
                                                    "El PDF fue leído pero no contiene texto extraíble.".to_string()
                                                } else {
                                                    format!("[PDF extraído: {}]\\n\\n{}", rel_path, text)
                                                }
                                            }
                                            _ => "No se pudo extraer texto del PDF. Instala pdftotext (poppler-utils) o PyPDF2 (pip install PyPDF2). Como alternativa, usa analyze_images.".to_string()
                                        }
                                    }
                                }
                            }
                            // --- DOCX: extraer texto ---
                            else if extension == "docx" {
                                let docx_path_str = full_path.to_string_lossy().to_string();
                                match std::process::Command::new("python")
                                    .args(["-c", &format!(
                                        "import sys;\\ntry:\\n from docx import Document\\n doc = Document(r'{}')\\n for p in doc.paragraphs:\\n  print(p.text)\\nexcept ImportError:\\n sys.exit(1)",
                                        docx_path_str.replace("\\", "\\\\").replace("'", "\\'")
                                    )])
                                    .output()
                                {
                                    Ok(out) if out.status.success() => {
                                        let text = String::from_utf8_lossy(&out.stdout).to_string();
                                        if text.trim().is_empty() {
                                            "El DOCX fue leído pero no contiene texto.".to_string()
                                        } else {
                                            format!("[DOCX extraído: {}]\\n\\n{}", rel_path, text)
                                        }
                                    }
                                    _ => {
                                        let ps_script = format!(
                                            "Add-Type -AssemblyName System.IO.Compression.FileSystem; $zip = [System.IO.Compression.ZipFile]::OpenRead('{}'); $entry = $zip.GetEntry('word/document.xml'); if ($entry) {{ $stream = $entry.Open(); $reader = [System.IO.StreamReader]::new($stream); $xml = $reader.ReadToEnd(); $reader.Close(); $stream.Close(); $xml -replace '<[^>]+>', '' }}; $zip.Dispose()",
                                            docx_path_str.replace("'", "''")
                                        );
                                        match std::process::Command::new("powershell")
                                            .args(["-NoProfile", "-Command", &ps_script])
                                            .output()
                                        {
                                            Ok(out) if out.status.success() => {
                                                let text = String::from_utf8_lossy(&out.stdout).to_string();
                                                if text.trim().is_empty() {
                                                    "El DOCX fue leído pero no contiene texto extraíble. Instala python-docx (pip install python-docx) para mejor soporte.".to_string()
                                                } else {
                                                    format!("[DOCX extraído: {}]\\n\\n{}", rel_path, text)
                                                }
                                            }
                                            _ => "No se pudo extraer texto del DOCX. Instala python-docx (pip install python-docx).".to_string()
                                        }
                                    }
                                }
                            }
                            // --- Archivos de texto normales ---
                            else {
                                match fs::read_to_string(&full_path) {'''

if old_read_file in content:
    content = content.replace(old_read_file, new_read_file)
    print("✓ Fix 3 (read_file PDF/DOCX) aplicado")
else:
    print("✗ Fix 3: no se encontró el patrón read_file")
    idx = content.find('"read_file" =>')
    if idx > 0:
        print(f"  Encontrado en offset {idx}")
        print(f"  Contexto: ...{content[idx:idx+400]}...")

with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("✓ agent.rs escrito correctamente")
