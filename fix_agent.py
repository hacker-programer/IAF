#!/usr/bin/env python3
"""Apply 3 fixes to agent.rs atomically."""
import sys

with open('src/agent.rs', 'rb') as f:
    data = f.read()

changes = 0

# --- Fix 1: notificar_usuario -> add info_messages (BUG-002) ---
# Insert before status.steps.push in the "tipo informativo" branch
f1_marker = b"                                status.steps.push(crate::state::AuditStep {\r\n                                    step_type: \"informativo\""
f1_insert = (b"                                // Agregar a info_messages para frontend (BUG-002)\r\n"
             b"                                status.info_messages.push(mensaje.to_string());\r\n"
             b"                                if status.info_messages.len() > 100 {\r\n"
             b"                                    status.info_messages.remove(0);\r\n"
             b"                                }\r\n")

idx = data.find(f1_marker)
if idx >= 0:
    data = data[:idx] + f1_insert + data[idx:]
    print("[OK] Fix 1: info_messages")
    changes += 1
else:
    print("[FAIL] Fix 1")

# --- Fix 2: finalizar_tarea -> multi-line (BUG-004) ---
f2_old = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }                    "image_fetch" => {'

f2_new = (b'"finalizar_tarea" => {\r\n'
          b'                        // Limpiar todos los procesos hijo registrados antes de finalizar\r\n'
          b'                        state.process_registry.kill_all();\r\n'
          b'\r\n'
          b'                        let msg = args["mensaje_final"]\r\n'
          b'                            .as_str()\r\n'
          b'                            .unwrap_or("Tarea finalizada.")\r\n'
          b'                            .to_string();\r\n'
          b'\r\n'
          b'                        // Validar que el mensaje no este vacio\r\n'
          b'                        let final_msg = if msg.trim().is_empty() {\r\n'
          b'                            "Tarea finalizada.".to_string()\r\n'
          b'                        } else {\r\n'
          b'                            msg\r\n'
          b'                        };\r\n'
          b'\r\n'
          b'                        // Actualizar estado del agente para que el frontend lo detecte\r\n'
          b'                        {\r\n'
          b'                            let mut status = state.active_agent.lock().unwrap();\r\n'
          b'                            status.finished = true;\r\n'
          b'                            status.final_message = Some(final_msg.clone());\r\n'
          b'                            status.running = false;\r\n'
          b'                            status.esperando_respuesta_usuario = false;\r\n'
          b'                            status.esperando_aprobacion_plan = false;\r\n'
          b'                            // Limpiar info_messages al finalizar (BUG-004)\r\n'
          b'                            status.info_messages.clear();\r\n'
          b'                            status.steps.push(crate::state::AuditStep {\r\n'
          b'                                step_type: "thinking".to_string(),\r\n'
          b'                                title: "Tarea Finalizada".to_string(),\r\n'
          b'                                detail: format!("El agente ha finalizado la tarea: {}", final_msg),\r\n'
          b'                                timestamp: std::time::SystemTime::now()\r\n'
          b'                                    .duration_since(std::time::UNIX_EPOCH)\r\n'
          b'                                    .unwrap()\r\n'
          b'                                    .as_secs(),\r\n'
          b'                            });\r\n'
          b'                            if let Some(ref s_id) = session_id {\r\n'
          b'                                save_chat_steps_to_disk(\r\n'
          b'                                    &state,\r\n'
          b'                                    &Some(s_id.clone()),\r\n'
          b'                                    &status.steps,\r\n'
          b'                                );\r\n'
          b'                            }\r\n'
          b'                        }\r\n'
          b'                        final_message = Some(final_msg);\r\n'
          b'                        "Tarea finalizada correctamente.".to_string()\r\n'
          b'                    }\r\n'
          b'                    "image_fetch" => {')

if f2_old in data:
    data = data.replace(f2_old, f2_new)
    print("[OK] Fix 2: finalizar_tarea")
    changes += 1
else:
    print("[FAIL] Fix 2: pattern not found")

# --- Fix 3: read_file -> PDF/DOCX support (BUG-001) ---
f3_old = b'                            match fs::read_to_string(&full_path) {'

f3_new = (b'                            // Detectar extension para formatos especiales (BUG-001)\r\n'
          b'                            let extension = full_path\r\n'
          b'                                .extension()\r\n'
          b'                                .and_then(|e| e.to_str())\r\n'
          b'                                .unwrap_or("")\r\n'
          b'                                .to_lowercase();\r\n'
          b'\r\n'
          b'                            // --- PDF ---\r\n'
          b'                            if extension == "pdf" {\r\n'
          b'                                let pdf_path_str = full_path.to_string_lossy().to_string();\r\n'
          b'                                match std::process::Command::new("pdftotext")\r\n'
          b'                                    .args(["-layout", &pdf_path_str, "-"])\r\n'
          b'                                    .output()\r\n'
          b'                                {\r\n'
          b'                                    Ok(out) if out.status.success() => {\r\n'
          b'                                        let text = String::from_utf8_lossy(&out.stdout).to_string();\r\n'
          b'                                        if text.trim().is_empty() {\r\n'
          b'                                            "El PDF fue procesado pero no contiene texto extraible (puede ser escaneado). Prueba con analyze_images para OCR.".to_string()\r\n'
          b'                                        } else {\r\n'
          b'                                            format!("[PDF extraido: {}]\\n\\n{}", rel_path, text)\r\n'
          b'                                        }\r\n'
          b'                                    }\r\n'
          b'                                    _ => "No se pudo extraer texto del PDF. Instala pdftotext (poppler-utils) o PyPDF2 (pip install PyPDF2). Como alternativa, usa analyze_images.".to_string()\r\n'
          b'                                }\r\n'
          b'                            }\r\n'
          b'                            // --- DOCX ---\r\n'
          b'                            else if extension == "docx" {\r\n'
          b'                                let docx_path_str = full_path.to_string_lossy().to_string();\r\n'
          b'                                match std::process::Command::new("python")\r\n'
          b'                                    .args(["-c", &format!(\r\n'
          b'                                        "import sys;\\ntry:\\n from docx import Document\\n doc = Document(r'"'"'{}'"'"')\\n for p in doc.paragraphs:\\n  print(p.text)\\nexcept ImportError:\\n sys.exit(1)",\r\n'
          b'                                        docx_path_str.replace("\\", "\\\\").replace("'"'"'", "\\'"'"'")\r\n'
          b'                                    )])\r\n'
          b'                                    .output()\r\n'
          b'                                {\r\n'
          b'                                    Ok(out) if out.status.success() => {\r\n'
          b'                                        let text = String::from_utf8_lossy(&out.stdout).to_string();\r\n'
          b'                                        if text.trim().is_empty() {\r\n'
          b'                                            "El DOCX fue leido pero no contiene texto.".to_string()\r\n'
          b'                                        } else {\r\n'
          b'                                            format!("[DOCX extraido: {}]\\n\\n{}", rel_path, text)\r\n'
          b'                                        }\r\n'
          b'                                    }\r\n'
          b'                                    _ => "No se pudo extraer texto del DOCX. Instala python-docx (pip install python-docx).".to_string()\r\n'
          b'                                }\r\n'
          b'                            }\r\n'
          b'                            // --- Archivos de texto normales ---\r\n'
          b'                            else {\r\n'
          b'                                match fs::read_to_string(&full_path) {')

if f3_old in data:
    data = data.replace(f3_old, f3_new)
    # Need to close the else block: find the original match closing and add }
    # Pattern: after Err(e) line, the match closes with }, then } else { for project
    f3_close_marker = b'                                Err(e) => format!("Error leyendo archivo: {}", e),'
    close_idx = data.find(f3_close_marker)
    if close_idx > 0:
        # Find the next } (match close) - it should be at ~28 spaces indentation
        after_err = data[close_idx:]
        match_close = after_err.find(b'\r\n                            }')
        if match_close > 0:
            insert_pos = close_idx + match_close + len(b'\r\n                            }')
            data = data[:insert_pos] + b'\r\n                            }' + data[insert_pos:]
            print("[OK] Fix 3: read_file PDF/DOCX")
            changes += 1
        else:
            print("[WARN] Fix 3: applied but couldn't find match close")
            changes += 1
    else:
        print("[WARN] Fix 3: applied but couldn't find Err handler")
        changes += 1
else:
    print("[FAIL] Fix 3: pattern not found")

# Save
with open('src/agent.rs', 'wb') as f:
    f.write(data)

# Verify
opens = data.count(b'{')
closes = data.count(b'}')
print(f"\n[DONE] {changes}/3 fixes applied")
print(f"Braces: opens={opens}, closes={closes}, diff={opens - closes}")
print(f"Lines: {data.count(chr(10).encode())}")
