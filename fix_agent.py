import sys

with open('src/agent.rs', 'rb') as f:
    data = f.read()

# Fix 1: info_messages in notificar_usuario else
m1 = b'                                status.steps.push(crate::state::AuditStep {\r\n                                    step_type: "informativo"'
ins1 = b'                                status.info_messages.push(mensaje.to_string());\r\n                                if status.info_messages.len() > 100 { status.info_messages.remove(0); }\r\n                                '
idx = data.find(m1)
if idx >= 0:
    data = data[:idx] + ins1 + data[idx:]
    print('Fix 1 OK')
else:
    print('Fix 1 FAIL')

# Fix 2: finalizar_tarea multi-line
f2_old = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }                    "image_fetch" => {'
f2_new = b'"finalizar_tarea" => {\r\n                        state.process_registry.kill_all();\r\n                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();\r\n                        let final_msg = if msg.trim().is_empty() { "Tarea finalizada.".to_string() } else { msg };\r\n                        { let mut status = state.active_agent.lock().unwrap();\r\n                            status.finished = true; status.final_message = Some(final_msg.clone());\r\n                            status.running = false; status.esperando_respuesta_usuario = false;\r\n                            status.esperando_aprobacion_plan = false; status.info_messages.clear();\r\n                            status.steps.push(crate::state::AuditStep {\r\n                                step_type: "thinking".to_string(), title: "Tarea Finalizada".to_string(),\r\n                                detail: format!("El agente ha finalizado la tarea: {}", final_msg),\r\n                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),\r\n                            });\r\n                            if let Some(ref s_id) = session_id { save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps); }\r\n                        }\r\n                        final_message = Some(final_msg);\r\n                        "Tarea finalizada correctamente.".to_string()\r\n                    }\r\n                    "image_fetch" => {'
if f2_old in data:
    data = data.replace(f2_old, f2_new)
    print('Fix 2 OK')
else:
    print('Fix 2 FAIL')

# Fix 3: read_file PDF/DOCX detection
f3_old = b'                            match fs::read_to_string(&full_path) {'
f3_new = b'                            let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();\r\n                            if ext == "pdf" || ext == "docx" {\r\n                                let path_str = full_path.to_string_lossy().to_string();\r\n                                if ext == "pdf" {\r\n                                    match std::process::Command::new("pdftotext").args(["-layout", &path_str, "-"]).output() {\r\n                                        Ok(out) if out.status.success() => {\r\n                                            let t = String::from_utf8_lossy(&out.stdout).to_string();\r\n                                            if t.trim().is_empty() { "PDF sin texto extraible. Usa analyze_images para OCR.".to_string() }\r\n                                            else { format!("[PDF: {}]\\n\\n{}", rel_path, t) }\r\n                                        }\r\n                                        _ => "No se pudo leer el PDF. Instala pdftotext o PyPDF2. Usa analyze_images como alternativa.".to_string()\r\n                                    }\r\n                                } else {\r\n                                    "El archivo DOCX no se puede leer directamente. Instala python-docx (pip install python-docx) o usa analyze_images para analizarlo visualmente.".to_string()\r\n                                }\r\n                            } else {\r\n                                match fs::read_to_string(&full_path) {'
if f3_old in data:
    data = data.replace(f3_old, f3_new)
    # Close the else block: add } after the match closing }
    close_marker = b'                                Err(e) => format!("Error leyendo archivo: {}", e),'
    ci = data.find(close_marker)
    if ci > 0:
        after = data[ci:]
        mc = after.find(b'\r\n                            }')
        if mc > 0:
            ip = ci + mc + len(b'\r\n                            }')
            data = data[:ip] + b'\r\n                            }' + data[ip:]
    print('Fix 3 OK')
else:
    print('Fix 3 FAIL')

with open('src/agent.rs', 'wb') as f:
    f.write(data)

o = data.count(b'{')
c = data.count(b'}')
print('Done. Opens=%d, Closes=%d, Diff=%d, Lines=%d' % (o, c, o-c, data.count(b'\n')))
