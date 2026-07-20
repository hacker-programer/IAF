#!/usr/bin/env python3
"""Aplica 3 fixes a agent.rs con reemplazo binario exacto."""
import sys

with open('src/agent.rs', 'rb') as f:
    data = f.read()

changes = 0

# ─── Fix 1: notificar_usuario -> agregar info_messages ───
# Patron: "status.steps.push" dentro del else de notificar_usuario
# Insertar info_messages.push justo antes de status.steps.push
marker = b"                                status.steps.push(crate::state::AuditStep {\r\r\n                                    step_type: \"informativo\""
insertion = b"                                // Agregar a info_messages para frontend (BUG-002)\r\r\n                                status.info_messages.push(mensaje.to_string());\r\r\n                                if status.info_messages.len() > 100 {\r\r\n                                    status.info_messages.remove(0);\r\r\n                                }\r\r\n"
idx = data.find(marker)
if idx >= 0:
    data = data[:idx] + insertion + data[idx:]
    print("[OK] Fix 1: info_messages en notificar_usuario")
    changes += 1
else:
    print("[FAIL] Fix 1: marker not found")

# ─── Fix 2: finalizar_tarea -> multi-line ───
old2 = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }'
if old2 in data:
    new2 = b'"finalizar_tarea" => {\r\r\n                        // Limpiar todos los procesos hijo registrados antes de finalizar\r\r\n                        state.process_registry.kill_all();\r\r\n\r\r\n                        let msg = args["mensaje_final"]\r\r\n                            .as_str()\r\r\n                            .unwrap_or("Tarea finalizada.")\r\r\n                            .to_string();\r\r\n\r\r\n                        // Validar que el mensaje no este vacio\r\r\n                        let final_msg = if msg.trim().is_empty() {\r\r\n                            "Tarea finalizada.".to_string()\r\r\n                        } else {\r\r\n                            msg\r\r\n                        };\r\r\n\r\r\n                        // Actualizar estado del agente para que el frontend lo detecte\r\r\n                        {\r\r\n                            let mut status = state.active_agent.lock().unwrap();\r\r\n                            status.finished = true;\r\r\n                            status.final_message = Some(final_msg.clone());\r\r\n                            status.running = false;\r\r\n                            status.esperando_respuesta_usuario = false;\r\r\n                            status.esperando_aprobacion_plan = false;\r\r\n                            // Limpiar info_messages al finalizar (BUG-004)\r\r\n                            status.info_messages.clear();\r\r\n                            status.steps.push(crate::state::AuditStep {\r\r\n                                step_type: "thinking".to_string(),\r\r\n                                title: "Tarea Finalizada".to_string(),\r\r\n                                detail: format!("El agente ha finalizado la tarea: {}", final_msg),\r\r\n                                timestamp: std::time::SystemTime::now()\r\r\n                                    .duration_since(std::time::UNIX_EPOCH)\r\r\n                                    .unwrap()\r\r\n                                    .as_secs(),\r\r\n                            });\r\r\n                            if let Some(ref s_id) = session_id {\r\r\n                                save_chat_steps_to_disk(\r\r\n                                    &state,\r\r\n                                    &Some(s_id.clone()),\r\r\n                                    &status.steps,\r\r\n                                );\r\r\n                            }\r\r\n                        }\r\r\n                        final_message = Some(final_msg);\r\r\n                        "Tarea finalizada correctamente.".to_string()\r\r\n                    }'
    data = data.replace(old2, new2)
    print("[OK] Fix 2: finalizar_tarea multi-linea")
    changes += 1
else:
    print("[FAIL] Fix 2: pattern not found")

# ─── Fix 3: read_file -> PDF/DOCX ───
old3 = b'                            match fs::read_to_string(&full_path) {'
if old3 in data:
    new3 = b'''                            // Detectar extension para formatos especiales (BUG-001)
                            let extension = full_path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();

                            // --- PDF ---
                            if extension == "pdf" {
                                let pdf_path_str = full_path.to_string_lossy().to_string();
                                match std::process::Command::new("pdftotext")
                                    .args(["-layout", &pdf_path_str, "-"])
                                    .output()
                                {
                                    Ok(out) if out.status.success() => {
                                        let text = String::from_utf8_lossy(&out.stdout).to_string();
                                        if text.trim().is_empty() {
                                            "El PDF fue procesado pero no contiene texto extraible (puede ser escaneado). Prueba con analyze_images para OCR.".to_string()
                                        } else {
                                            format!("[PDF extraido: {}]\\n\\n{}", rel_path, text)
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
                                                    "El PDF fue leido pero no contiene texto extraible.".to_string()
                                                } else {
                                                    format!("[PDF extraido: {}]\\n\\n{}", rel_path, text)
                                                }
                                            }
                                            _ => "No se pudo extraer texto del PDF. Instala pdftotext (poppler-utils) o PyPDF2 (pip install PyPDF2). Como alternativa, usa analyze_images.".to_string()
                                        }
                                    }
                                }
                            }
                            // --- DOCX ---
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
                                            "El DOCX fue leido pero no contiene texto.".to_string()
                                        } else {
                                            format!("[DOCX extraido: {}]\\n\\n{}", rel_path, text)
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
                                                    "El DOCX fue leido pero no contiene texto extraible. Instala python-docx (pip install python-docx) para mejor soporte.".to_string()
                                                } else {
                                                    format!("[DOCX extraido: {}]\\n\\n{}", rel_path, text)
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
    data = data.replace(old3, new3)
    # Need to close the else block: add } before the final closing
    # Find the Err(e) => ... line that closes the original match
    close_marker = b"                            }"
    # The new else block needs an extra closing brace. Find where to add it.
    # The old structure was: match fs::read_to_string { Ok => {...}, Err => format!(...) }
    # The new: else { match fs::read_to_string { Ok => {...}, Err => format!(...) } }
    # We need to add } after the Err arm to close the else.
    # Find: '                        } else {\r\r\n                            "No hay' 
    err_marker = b'"No hay ning'
    err_idx = data.find(err_marker)
    if err_idx > 0:
        # Find the closing } of the outer if-let
        # Structure: ...Err(e) => ... }  } else { "No hay..."
        # We need to find the pattern: \n                        } else {\n                            "No hay
        # And add } before it
        else_marker = b'                        } else {\r\r\n                            "No hay ning'
        else_idx = data.find(else_marker)
        if else_idx > 0:
            data = data[:else_idx] + b'                        }\r\r\n' + data[else_idx:]
            print("[OK] Fix 3: read_file PDF/DOCX support")
            changes += 1
        else:
            print("[WARN] Fix 3 applied but couldn't close else block")
            changes += 1
    else:
        print("[WARN] Fix 3 applied but couldn't find err marker")
        changes += 1
else:
    print("[FAIL] Fix 3: pattern not found")

with open('src/agent.rs', 'wb') as f:
    f.write(data)

print(f"\n[DONE] {changes}/3 fixes applied")
