#!/usr/bin/env python3
"""Aplica 3 fixes a agent.rs - version corregida con line endings correctos."""
import sys

with open('src/agent.rs', 'rb') as f:
    data = f.read()

changes = 0
LE = b'\r\n'  # Windows line ending

# ─── Fix 1: notificar_usuario -> agregar info_messages ───
# Insertar info_messages.push justo antes de status.steps.push en el else de notificar_usuario
marker = b"                                status.steps.push(crate::state::AuditStep {" + LE + b"                                    step_type: \"informativo\""
insertion = b"                                // Agregar a info_messages para frontend (BUG-002)" + LE + b"                                status.info_messages.push(mensaje.to_string());" + LE + b"                                if status.info_messages.len() > 100 {" + LE + b"                                    status.info_messages.remove(0);" + LE + b"                                }" + LE
idx = data.find(marker)
if idx >= 0:
    data = data[:idx] + insertion + data[idx:]
    print("[OK] Fix 1: info_messages en notificar_usuario")
    changes += 1
else:
    print("[FAIL] Fix 1: marker not found at expected position")
    # Try just searching for "informativo" in the match handler area
    match_handler_marker = b"// tipo informativo"
    idx2 = data.find(match_handler_marker)
    if idx2 >= 0:
        print(f"  Found '// tipo informativo' at offset {idx2}")
        print(f"  Context: {data[idx2-50:idx2+300]}")

# ─── Fix 2: finalizar_tarea -> multi-line ───
old2 = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }'
if old2 in data:
    new2 = b'"finalizar_tarea" => {' + LE + b'                        state.process_registry.kill_all();' + LE + LE + b'                        let msg = args["mensaje_final"]' + LE + b'                            .as_str()' + LE + b'                            .unwrap_or("Tarea finalizada.")' + LE + b'                            .to_string();' + LE + LE + b'                        let final_msg = if msg.trim().is_empty() {' + LE + b'                            "Tarea finalizada.".to_string()' + LE + b'                        } else {' + LE + b'                            msg' + LE + b'                        };' + LE + LE + b'                        {' + LE + b'                            let mut status = state.active_agent.lock().unwrap();' + LE + b'                            status.finished = true;' + LE + b'                            status.final_message = Some(final_msg.clone());' + LE + b'                            status.running = false;' + LE + b'                            status.esperando_respuesta_usuario = false;' + LE + b'                            status.esperando_aprobacion_plan = false;' + LE + b'                            status.info_messages.clear();' + LE + b'                            status.steps.push(crate::state::AuditStep {' + LE + b'                                step_type: "thinking".to_string(),' + LE + b'                                title: "Tarea Finalizada".to_string(),' + LE + b'                                detail: format!("El agente ha finalizado la tarea: {}", final_msg),' + LE + b'                                timestamp: std::time::SystemTime::now()' + LE + b'                                    .duration_since(std::time::UNIX_EPOCH)' + LE + b'                                    .unwrap()' + LE + b'                                    .as_secs(),' + LE + b'                            });' + LE + b'                            if let Some(ref s_id) = session_id {' + LE + b'                                save_chat_steps_to_disk(' + LE + b'                                    &state,' + LE + b'                                    &Some(s_id.clone()),' + LE + b'                                    &status.steps,' + LE + b'                                );' + LE + b'                            }' + LE + b'                        }' + LE + b'                        final_message = Some(final_msg);' + LE + b'                        "Tarea finalizada correctamente.".to_string()' + LE + b'                    }'
    data = data.replace(old2, new2)
    print("[OK] Fix 2: finalizar_tarea multi-linea")
    changes += 1
else:
    print("[FAIL] Fix 2: pattern not found")

# ─── Fix 3: read_file -> PDF/DOCX support ───
old3 = b'                            match fs::read_to_string(&full_path) {'
if old3 in data:
    new3 = b'                            let extension = full_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();' + LE + b'                            if extension == "pdf" {' + LE + b'                                let p = full_path.to_string_lossy().to_string();' + LE + b'                                match std::process::Command::new("pdftotext").args(["-layout", &p, "-"]).output() {' + LE + b'                                    Ok(out) if out.status.success() => {' + LE + b'                                        let t = String::from_utf8_lossy(&out.stdout).to_string();' + LE + b'                                        if t.trim().is_empty() { "PDF sin texto extraible. Usa analyze_images.".to_string() }' + LE + b'                                        else { format!("[PDF: {}]\\n\\n{}", rel_path, t) }' + LE + b'                                    }' + LE + b'                                    _ => "No se pudo leer el PDF. Instala pdftotext o PyPDF2.".to_string()' + LE + b'                                }' + LE + b'                            } else if extension == "docx" {' + LE + b'                                let d = full_path.to_string_lossy().to_string();' + LE + b'                                let ps = format!("Add-Type -AssemblyName System.IO.Compression.FileSystem; $z=[System.IO.Compression.ZipFile]::OpenRead(\'{}\'); $e=$z.GetEntry(\'word/document.xml\'); if($e){{$s=$e.Open();$r=[System.IO.StreamReader]::new($s);$x=$r.ReadToEnd();$r.Close();$s.Close();$x -replace \'<[^>]+>\',\'\'}};$z.Dispose()", d.replace("\'","\'\'"));' + LE + b'                                match std::process::Command::new("powershell").args(["-NoProfile","-Command",&ps]).output() {' + LE + b'                                    Ok(out) if out.status.success() => {' + LE + b'                                        let t = String::from_utf8_lossy(&out.stdout).to_string();' + LE + b'                                        if t.trim().is_empty() { "DOCX sin texto. Instala python-docx.".to_string() }' + LE + b'                                        else { format!("[DOCX: {}]\\n\\n{}", rel_path, t) }' + LE + b'                                    }' + LE + b'                                    _ => "No se pudo leer el DOCX. Instala python-docx.".to_string()' + LE + b'                                }' + LE + b'                            } else {' + LE + b'                                match fs::read_to_string(&full_path) {'
    data = data.replace(old3, new3)
    # Need to close the else block for the extension if/else chain
    # After Err(e) => ..., there should be a } to close the else { match }
    # The original structure: match fs::read_to_string { Ok => {}, Err => {} }
    #                            } else { "No hay ningun..."
    # The new structure:   else { match fs::read_to_string { Ok => {}, Err => {} } }
    #                            } else { "No hay ningun..."
    # So after Err => format!(...), we need an extra }
    err_close = b'                                Err(e) => format!("Error leyendo archivo: {}", e),'
    err_idx = data.find(err_close)
    if err_idx > 0:
        # Find the next }
        after_err = data[err_idx:]
        next_close = after_err.find(b'                            }')
        if next_close > 0:
            insert_pos = err_idx + next_close + len(b'                            }')
            data = data[:insert_pos] + LE + b'                            }' + data[insert_pos:]
            print("[OK] Fix 3: read_file PDF/DOCX + closed else block")
            changes += 1
        else:
            print("[WARN] Fix 3: couldn't find closing brace for else block")
            changes += 1
    else:
        print("[WARN] Fix 3: couldn't find Err handler to close")
        changes += 1
else:
    print("[FAIL] Fix 3: pattern not found")

with open('src/agent.rs', 'wb') as f:
    f.write(data)

print(f"\n[DONE] {changes}/3 fixes applied")
