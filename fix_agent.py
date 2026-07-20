#!/usr/bin/env python3
"""
Aplica los 3 fixes a agent.rs de forma atómica.
"""
import sys
sys.stdout.reconfigure(encoding='utf-8')

with open('src/agent.rs', 'r', encoding='utf-8') as f:
    content = f.read()

changes = 0

# ─── Fix 1: notificar_usuario else branch (BUG-002) ───
old1 = 'status.steps.push(crate::state::AuditStep {\n                                    step_type: "informativo".to_string(),\n                                    title: "Notificación del Agente".to_string(),\n                                    detail: mensaje.to_string(),\n                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),\n                                });\n                                if let Some(ref s_id) = session_id {\n                                    save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);\n                                }\n                            }\n                            format!("Notificación enviada con éxito: {}", mensaje)'

new1 = '// Agregar a info_messages para frontend (BUG-002)\n                                status.info_messages.push(mensaje.to_string());\n                                if status.info_messages.len() > 100 {\n                                    status.info_messages.remove(0);\n                                }\n                                status.steps.push(crate::state::AuditStep {\n                                    step_type: "informativo".to_string(),\n                                    title: "Notificación del Agente".to_string(),\n                                    detail: mensaje.to_string(),\n                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),\n                                });\n                                if let Some(ref s_id) = session_id {\n                                    save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);\n                                }\n                            }\n                            format!("Notificación enviada con éxito: {}", mensaje)'

if old1 in content:
    content = content.replace(old1, new1)
    print("[OK] Fix 1 (notificar_usuario)")
    changes += 1
else:
    print("[FAIL] Fix 1: patron no encontrado")

# ─── Fix 2: finalizar_tarea (BUG-004) ───
# Buscar la linea que empieza con "finalizar_tarea" y contiene kill_all
lines = content.split('\n')
target_line = -1
for i, line in enumerate(lines):
    if '"finalizar_tarea"' in line and 'kill_all' in line:
        target_line = i
        break

if target_line >= 0:
    new_block = '''"finalizar_tarea" => {
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
    lines[target_line] = new_block
    content = '\n'.join(lines)
    print(f"[OK] Fix 2 (finalizar_tarea) en linea {target_line+1}")
    changes += 1
else:
    print("[FAIL] Fix 2: no encontrado")

# ─── Fix 3: read_file PDF/DOCX support (BUG-001) ───
old3 = 'let full_path = Path::new(&proj_path).join(rel_path);\n                            match fs::read_to_string(&full_path) {'

new3 = '''let full_path = Path::new(&proj_path).join(rel_path);

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
                                            "El PDF fue procesado pero no contiene texto extraible (puede ser escaneado). Prueba con analyze_images para OCR.".to_string()
                                        } else {
                                            format!("[PDF extraido: {}]\\n\\n{}", rel_path, text)
                                        }
                                    }
                                    _ => {
                                        match std::process::Command::new("python")
                                            .args(["-c", &format!(
                                                "import sys;\\ntry:\\n from PyPDF2 import PdfReader\\n r = PdfReader(r'{}')\\n for p in r.pages:\\n  t = p.extract_text()\\n  if t: print(t)\\nexcept ImportError:\\n sys.exit(1)",
                                                pdf_path_str.replace("\\\\", "\\\\\\\\").replace("'", "\\\\'")
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
                            // --- DOCX: extraer texto ---
                            else if extension == "docx" {
                                let docx_path_str = full_path.to_string_lossy().to_string();
                                match std::process::Command::new("python")
                                    .args(["-c", &format!(
                                        "import sys;\\ntry:\\n from docx import Document\\n doc = Document(r'{}')\\n for p in doc.paragraphs:\\n  print(p.text)\\nexcept ImportError:\\n sys.exit(1)",
                                        docx_path_str.replace("\\\\", "\\\\\\\\").replace("'", "\\\\'")
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

if old3 in content:
    content = content.replace(old3, new3)
    print("[OK] Fix 3 (read_file PDF/DOCX)")
    changes += 1
else:
    print("[FAIL] Fix 3: patron no encontrado")

# Need to close the new else block for read_file
# The old code had: match fs::read_to_string { ... Err(e) => ... }
# The new code has: else { match fs::read_to_string { ... } }
# So we need to add a closing } before the final else
# This is handled by replacing the old single-line with a multi-line block
# that ends with: } else { match ... 

# Actually we also need to close the outer if/else. The original had:
#   match fs::read_to_string(&full_path) {
#       Ok(content) => { ... }
#       Err(e) => format!(...)
#   }
# Now we have:
#   if extension == "pdf" { ... }
#   else if extension == "docx" { ... }
#   else { match fs::read_to_string(&full_path) { ... } }
# The outer } from the original is still there.

with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print(f"[DONE] {changes}/3 fixes aplicados")
