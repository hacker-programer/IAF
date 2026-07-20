#!/usr/bin/env python3
"""Aplica los 3 fixes a agent.rs de forma atómica."""
import sys

with open('src/agent.rs', 'rb') as f:
    data = f.read()

changes = 0

# ─── Fix 1: notificar_usuario else branch (BUG-002) ───
# Search for the exact bytes around "tipo informativo"
old1_start = b'                        } else {\r\r\n                            // tipo informativo\r\r\n                            {\r\r\n                                let mut status = state.active_agent.lock().unwrap();\r\r\n                                status.steps.push'
# Find it
idx = data.find(old1_start)
if idx >= 0:
    # Find the end: "format!("Notificación enviada"
    end_marker = b'format!(\"Notificaci'
    end_idx = data.find(end_marker, idx)
    if end_idx >= 0:
        # Find end of this line + closing braces
        rest = data[end_idx:]
        close_idx = rest.find(b'\r\r\n                        }\r\r\n                    }')
        if close_idx >= 0:
            end_pos = end_idx + close_idx + len(b'\r\r\n                        }\r\r\n                    }')
        else:
            end_pos = end_idx + 200  # fallback
    else:
        end_pos = idx + 600
else:
    print("[FAIL] Fix 1: no encontrado")
    end_pos = None

if end_pos:
    new1 = b'''                        } else {\r\r\n                            // tipo informativo\r\r\n                            {\r\r\n                                let mut status = state.active_agent.lock().unwrap();\r\r\n                                // Agregar a info_messages para frontend (BUG-002)\r\r\n                                status.info_messages.push(mensaje.to_string());\r\r\n                                if status.info_messages.len() > 100 {\r\r\n                                    status.info_messages.remove(0);\r\r\n                                }\r\r\n                                status.steps.push(crate::state::AuditStep {\r\r\n                                    step_type: \"informativo\".to_string(),\r\r\n                                    title: \"Notificaci\u00f3n del Agente\".to_string(),\r\r\n                                    detail: mensaje.to_string(),\r\r\n                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),\r\r\n                                });\r\r\n                                if let Some(ref s_id) = session_id {\r\r\n                                    save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);\r\r\n                                }\r\r\n                            }\r\r\n                            format!(\"Notificaci\u00f3n enviada con \u00e9xito: {}\", mensaje)\r\r\n                        }\r\r\n                    }'''
    data = data[:idx] + new1 + data[end_pos:]
    print("[OK] Fix 1 (notificar_usuario + info_messages)")
    changes += 1
else:
    print("[FAIL] Fix 1: patron no encontrado")

# ─── Fix 2: finalizar_tarea (BUG-004) ───
old2 = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar'
idx = data.find(old2)
if idx >= 0:
    end2 = data.find(b'"image_fetch"', idx)
    if end2 < 0:
        end2 = idx + 2000
    new2 = b'"finalizar_tarea" => {\r\r\n                        // Limpiar todos los procesos hijo registrados antes de finalizar\r\r\n                        state.process_registry.kill_all();\r\r\n\r\r\n                        let msg = args[\"mensaje_final\"]\r\r\n                            .as_str()\r\r\n                            .unwrap_or(\"Tarea finalizada.\")\r\r\n                            .to_string();\r\r\n\r\r\n                        // Validar que el mensaje no est\u00e9 vac\u00edo\r\r\n                        let final_msg = if msg.trim().is_empty() {\r\r\n                            \"Tarea finalizada.\".to_string()\r\r\n                        } else {\r\r\n                            msg\r\r\n                        };\r\r\n\r\r\n                        // Actualizar estado del agente para que el frontend lo detecte\r\r\n                        {\r\r\n                            let mut status = state.active_agent.lock().unwrap();\r\r\n                            status.finished = true;\r\r\n                            status.final_message = Some(final_msg.clone());\r\r\n                            status.running = false;\r\r\n                            status.esperando_respuesta_usuario = false;\r\r\n                            status.esperando_aprobacion_plan = false;\r\r\n                            // Limpiar info_messages al finalizar (BUG-004)\r\r\n                            status.info_messages.clear();\r\r\n                            status.steps.push(crate::state::AuditStep {\r\r\n                                step_type: \"thinking\".to_string(),\r\r\n                                title: \"Tarea Finalizada\".to_string(),\r\r\n                                detail: format!(\"El agente ha finalizado la tarea: {}\", final_msg),\r\r\n                                timestamp: std::time::SystemTime::now()\r\r\n                                    .duration_since(std::time::UNIX_EPOCH)\r\r\n                                    .unwrap()\r\r\n                                    .as_secs(),\r\r\n                            });\r\r\n                            if let Some(ref s_id) = session_id {\r\r\n                                save_chat_steps_to_disk(\r\r\n                                    &state,\r\r\n                                    &Some(s_id.clone()),\r\r\n                                    &status.steps,\r\r\n                                );\r\r\n                            }\r\r\n                        }\r\r\n                        final_message = Some(final_msg);\r\r\n                        \"Tarea finalizada correctamente.\".to_string()\r\r\n                    }\r\r\n'
    data = data[:idx] + new2 + data[end2:]
    print("[OK] Fix 2 (finalizar_tarea)")
    changes += 1
else:
    print("[FAIL] Fix 2: no encontrado")

# ─── Fix 3: read_file PDF/DOCX (BUG-001) ───
old3 = b'let full_path = Path::new(&proj_path).join(rel_path);\r\r\n                            match fs::read_to_string(&full_path) {'
idx = data.find(old3)
if idx >= 0:
    new3 = b'''let full_path = Path::new(&proj_path).join(rel_path);\r\r\n\r\r\n                            // Detectar extensi\u00f3n para formatos especiales (BUG-001)\r\r\n                            let extension = full_path\r\r\n                                .extension()\r\r\n                                .and_then(|e| e.to_str())\r\r\n                                .unwrap_or("")\r\r\n                                .to_lowercase();\r\r\n\r\r\n                            // --- PDF: intentar extraer texto ---\r\r\n                            if extension == "pdf" {\r\r\n                                let pdf_path_str = full_path.to_string_lossy().to_string();\r\r\n                                match std::process::Command::new("pdftotext")\r\r\n                                    .args(["-layout", &pdf_path_str, "-"])\r\r\n                                    .output()\r\r\n                                {\r\r\n                                    Ok(out) if out.status.success() => {\r\r\n                                        let text = String::from_utf8_lossy(&out.stdout).to_string();\r\r\n                                        if text.trim().is_empty() {\r\r\n                                            "El PDF fue procesado pero no contiene texto extra\u00edble (puede ser escaneado). Prueba con analyze_images para OCR.".to_string()\r\r\n                                        } else {\r\r\n                                            format!("[PDF extra\u00eddo: {}]\\n\\n{}", rel_path, text)\r\r\n                                        }\r\r\n                                    }\r\r\n                                    _ => {\r\r\n                                        match std::process::Command::new("python")\r\r\n                                            .args(["-c", &format!(\r\r\n                                                "import sys;\\ntry:\\n from PyPDF2 import PdfReader\\n r = PdfReader(r\'{}\')\\n for p in r.pages:\\n  t = p.extract_text()\\n  if t: print(t)\\nexcept ImportError:\\n sys.exit(1)",\r\r\n                                                pdf_path_str.replace("\\\\", "\\\\\\\\").replace("\'", "\\\\\'")\r\r\n                                            )])\r\r\n                                            .output()\r\r\n                                        {\r\r\n                                            Ok(out) if out.status.success() => {\r\r\n                                                let text = String::from_utf8_lossy(&out.stdout).to_string();\r\r\n                                                if text.trim().is_empty() {\r\r\n                                                    "El PDF fue le\u00eddo pero no contiene texto extra\u00edble.".to_string()\r\r\n                                                } else {\r\r\n                                                    format!("[PDF extra\u00eddo: {}]\\n\\n{}", rel_path, text)\r\r\n                                                }\r\r\n                                            }\r\r\n                                            _ => "No se pudo extraer texto del PDF. Instala pdftotext (poppler-utils) o PyPDF2 (pip install PyPDF2). Como alternativa, usa analyze_images.".to_string()\r\r\n                                        }\r\r\n                                    }\r\r\n                                }\r\r\n                            }\r\r\n                            // --- DOCX: extraer texto ---\r\r\n                            else if extension == "docx" {\r\r\n                                let docx_path_str = full_path.to_string_lossy().to_string();\r\r\n                                match std::process::Command::new("python")\r\r\n                                    .args(["-c", &format!(\r\r\n                                        "import sys;\\ntry:\\n from docx import Document\\n doc = Document(r\'{}\')\\n for p in doc.paragraphs:\\n  print(p.text)\\nexcept ImportError:\\n sys.exit(1)",\r\r\n                                        docx_path_str.replace("\\\\", "\\\\\\\\").replace("\'", "\\\\\'")\r\r\n                                    )])\r\r\n                                    .output()\r\r\n                                {\r\r\n                                    Ok(out) if out.status.success() => {\r\r\n                                        let text = String::from_utf8_lossy(&out.stdout).to_string();\r\r\n                                        if text.trim().is_empty() {\r\r\n                                            "El DOCX fue le\u00eddo pero no contiene texto.".to_string()\r\r\n                                        } else {\r\r\n                                            format!("[DOCX extra\u00eddo: {}]\\n\\n{}", rel_path, text)\r\r\n                                        }\r\r\n                                    }\r\r\n                                    _ => {\r\r\n                                        let ps_script = format!(\r\r\n                                            "Add-Type -AssemblyName System.IO.Compression.FileSystem; $zip = [System.IO.Compression.ZipFile]::OpenRead(\'{}\'); $entry = $zip.GetEntry(\'word/document.xml\'); if ($entry) {{ $stream = $entry.Open(); $reader = [System.IO.StreamReader]::new($stream); $xml = $reader.ReadToEnd(); $reader.Close(); $stream.Close(); $xml -replace \'<[^>]+>\', \'\' }}; $zip.Dispose()",\r\r\n                                            docx_path_str.replace("\'", "\'\'")\r\r\n                                        );\r\r\n                                        match std::process::Command::new("powershell")\r\r\n                                            .args(["-NoProfile", "-Command", &ps_script])\r\r\n                                            .output()\r\r\n                                        {\r\r\n                                            Ok(out) if out.status.success() => {\r\r\n                                                let text = String::from_utf8_lossy(&out.stdout).to_string();\r\r\n                                                if text.trim().is_empty() {\r\r\n                                                    "El DOCX fue le\u00eddo pero no contiene texto extra\u00edble. Instala python-docx (pip install python-docx) para mejor soporte.".to_string()\r\r\n                                                } else {\r\r\n                                                    format!("[DOCX extra\u00eddo: {}]\\n\\n{}", rel_path, text)\r\r\n                                                }\r\r\n                                            }\r\r\n                                            _ => "No se pudo extraer texto del DOCX. Instala python-docx (pip install python-docx).".to_string()\r\r\n                                        }\r\r\n                                    }\r\r\n                                }\r\r\n                            }\r\r\n                            // --- Archivos de texto normales ---\r\r\n                            else {\r\r\n                                match fs::read_to_string(&full_path) {'''
    data = data[:idx] + new3 + data[idx + len(old3):]
    print("[OK] Fix 3 (read_file PDF/DOCX)")
    changes += 1
else:
    print("[FAIL] Fix 3: no encontrado")

with open('src/agent.rs', 'wb') as f:
    f.write(data)

print(f"[DONE] {changes}/3 fixes aplicados")
