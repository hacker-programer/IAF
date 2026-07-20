#!/usr/bin/env python3
"""
Aplica 3 fixes a agent.rs con verificacion de balance de llaves.
Usa raw strings para evitar problemas de escape.
"""
import sys

def count_braces(text):
    opens = text.count(b'{')
    closes = text.count(b'}')
    return opens, closes, opens - closes

with open('src/agent.rs', 'rb') as f:
    data = f.read()

original_diff = count_braces(data)[2]
print(f"Original diff: {original_diff}")

LE = b'\r\n'
changes = 0

# ============================================================
# FIX 1: BUG-002 - info_messages.push en notificar_usuario
# ============================================================
marker1 = b'status.steps.push(crate::state::AuditStep {\r\n                                    step_type: "informativo"'
insert1 = (
    b'// Agregar a info_messages para frontend (BUG-002)' + LE +
    b'                                status.info_messages.push(mensaje.to_string());' + LE +
    b'                                if status.info_messages.len() > 100 {' + LE +
    b'                                    status.info_messages.remove(0);' + LE +
    b'                                }' + LE +
    b'                                '
)

idx = data.find(marker1)
if idx >= 0:
    data = data[:idx] + insert1 + data[idx:]
    print("[OK] Fix 1: info_messages en notificar_usuario")
    changes += 1
else:
    print("[FAIL] Fix 1: marker not found")

# ============================================================
# FIX 2: BUG-004 - finalizar_tarea multi-linea
# ============================================================
old2 = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }'

if old2 in data:
    new2 = (
        b'"finalizar_tarea" => {' + LE +
        b'                        state.process_registry.kill_all();' + LE +
        LE +
        b'                        let msg = args["mensaje_final"]' + LE +
        b'                            .as_str()' + LE +
        b'                            .unwrap_or("Tarea finalizada.")' + LE +
        b'                            .to_string();' + LE +
        LE +
        b'                        let final_msg = if msg.trim().is_empty() {' + LE +
        b'                            "Tarea finalizada.".to_string()' + LE +
        b'                        } else {' + LE +
        b'                            msg' + LE +
        b'                        };' + LE +
        LE +
        b'                        {' + LE +
        b'                            let mut status = state.active_agent.lock().unwrap();' + LE +
        b'                            status.finished = true;' + LE +
        b'                            status.final_message = Some(final_msg.clone());' + LE +
        b'                            status.running = false;' + LE +
        b'                            status.esperando_respuesta_usuario = false;' + LE +
        b'                            status.esperando_aprobacion_plan = false;' + LE +
        b'                            status.info_messages.clear();' + LE +
        b'                            status.steps.push(crate::state::AuditStep {' + LE +
        b'                                step_type: "thinking".to_string(),' + LE +
        b'                                title: "Tarea Finalizada".to_string(),' + LE +
        b'                                detail: format!("El agente ha finalizado la tarea: {}", final_msg),' + LE +
        b'                                timestamp: std::time::SystemTime::now()' + LE +
        b'                                    .duration_since(std::time::UNIX_EPOCH)' + LE +
        b'                                    .unwrap()' + LE +
        b'                                    .as_secs(),' + LE +
        b'                            });' + LE +
        b'                            if let Some(ref s_id) = session_id {' + LE +
        b'                                save_chat_steps_to_disk(' + LE +
        b'                                    &state,' + LE +
        b'                                    &Some(s_id.clone()),' + LE +
        b'                                    &status.steps,' + LE +
        b'                                );' + LE +
        b'                            }' + LE +
        b'                        }' + LE +
        b'                        final_message = Some(final_msg);' + LE +
        b'                        "Tarea finalizada correctamente.".to_string()' + LE +
        b'                    }'
    )
    data = data.replace(old2, new2)
    print("[OK] Fix 2: finalizar_tarea multi-linea")
    changes += 1
else:
    print("[FAIL] Fix 2: pattern not found")

# ============================================================
# FIX 3: BUG-001 - read_file PDF/DOCX
# ============================================================
old3 = b'                            match fs::read_to_string(&full_path) {'
if old3 in data:
    # Version simplificada: solo agrega soporte basico
    new3 = (
        b'                            let extension = full_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();' + LE +
        b'                            if extension == "pdf" {' + LE +
        b'                                let pdf_path_str = full_path.to_string_lossy().to_string();' + LE +
        b'                                match std::process::Command::new("pdftotext").args(["-layout", &pdf_path_str, "-"]).output() {' + LE +
        b'                                    Ok(out) if out.status.success() => {' + LE +
        b'                                        let t = String::from_utf8_lossy(&out.stdout).to_string();' + LE +
        b'                                        if t.trim().is_empty() { "PDF sin texto extraible. Usa analyze_images.".to_string() }' + LE +
        b'                                        else { format!("[PDF: {}]\\n\\n{}", rel_path, t) }' + LE +
        b'                                    }' + LE +
        b'                                    _ => "No se pudo leer el PDF. Instala pdftotext o PyPDF2.".to_string()' + LE +
        b'                                }' + LE +
        b'                            } else if extension == "docx" {' + LE +
        b'                                let docx_path_str = full_path.to_string_lossy().to_string();' + LE +
        b'                                let ps = std::format!("Add-Type -As System.IO.Compression.FileSystem; $z=[IO.Compression.ZipFile]::OpenRead(\\'{}\\'); $e=$z.GetEntry(\\'word/document.xml\\'); if($e){{$s=$e.Open();$r=[IO.StreamReader]::new($s);$x=$r.ReadToEnd();$r.Close();$s.Close();$x -replace \\'<[^>]+>\\',\\'\\'}};$z.Dispose()", docx_path_str.replace("\\'","\\'\\'"));' + LE +
        b'                                match std::process::Command::new("powershell").args(["-NoProfile","-Command",&ps]).output() {' + LE +
        b'                                    Ok(out) if out.status.success() => {' + LE +
        b'                                        let t = String::from_utf8_lossy(&out.stdout).to_string();' + LE +
        b'                                        if t.trim().is_empty() { "DOCX sin texto. Instala python-docx.".to_string() }' + LE +
        b'                                        else { format!("[DOCX: {}]\\n\\n{}", rel_path, t) }' + LE +
        b'                                    }' + LE +
        b'                                    _ => "No se pudo leer el DOCX. Instala python-docx.".to_string()' + LE +
        b'                                }' + LE +
        b'                            } else {' + LE +
        b'                                match fs::read_to_string(&full_path) {'
    )
    data = data.replace(old3, new3)

    # Cerrar el else { match ... } }
    else_marker = b'} else {\r\n                            "No hay ning'
    idx_else = data.find(else_marker)
    if idx_else > 0:
        data = data[:idx_else] + b'                        }\r\n' + data[idx_else:]
        print("[OK] Fix 3: read_file PDF/DOCX + cierre else")
        changes += 1
    else:
        print("[WARN] Fix 3: cierre no encontrado. Intentando patron alternativo...")
        alt = b'"No hay ning'
        idx_alt = data.find(alt)
        if idx_alt > 0:
            before = data[idx_alt-200:idx_alt]
            last_else = before.rfind(b'} else {')
            if last_else >= 0:
                insert_pos = idx_alt - 200 + last_else
                data = data[:insert_pos] + b'                        }\r\n' + data[insert_pos:]
                print("[OK] Fix 3: cierre alternativo aplicado")
                changes += 1
            else:
                print("[ERROR] Fix 3: no se encontro } else {")
        else:
            print("[ERROR] Fix 3: patron alternativo no encontrado")
else:
    print("[FAIL] Fix 3: pattern not found")

# ============================================================
# VERIFICACION
# ============================================================
final_diff = count_braces(data)[2]
print(f"Balance: {original_diff} -> {final_diff}")

if final_diff == original_diff:
    with open('src/agent.rs', 'wb') as f:
        f.write(data)
    print(f"[OK] {changes}/3 fixes aplicados. Balance mantenido.")
else:
    print(f"[ERROR] Balance ALTERADO. Archivo NO escrito.")
    sys.exit(1)
