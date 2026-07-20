#!/usr/bin/env python3
"""
Aplica 3 fixes a agent.rs con verificacion de balance de llaves.
Fix 1 (BUG-002): info_messages.push en notificar_usuario
Fix 2 (BUG-004): finalizar_tarea multi-linea
Fix 3 (BUG-001): read_file PDF/DOCX
"""
import sys

def count_braces(text):
    """Cuenta balance de llaves. Retorna (abiertas, cerradas, diff)."""
    opens = text.count(b'{')
    closes = text.count(b'}')
    return opens, closes, opens - closes

with open('src/agent.rs', 'rb') as f:
    data = f.read()

original_opens, original_closes, original_diff = count_braces(data)
print(f"Original: {original_opens} {{, {original_closes} }}, diff={original_diff}")

LE = b'\r\n'
changes = 0

# ============================================================
# FIX 1: notificar_usuario -> agregar info_messages (BUG-002)
# ============================================================
# Buscar: "status.steps.push(crate::state::AuditStep {" dentro del else de notificar_usuario
# El marcador exacto:
marker1 = b"                                status.steps.push(crate::state::AuditStep {\r\n                                    step_type: \"informativo\""
# Insertar ANTES de este marcador:
insert1 = (
    b"                                // Agregar a info_messages para frontend (BUG-002)" + LE +
    b"                                status.info_messages.push(mensaje.to_string());" + LE +
    b"                                if status.info_messages.len() > 100 {" + LE +
    b"                                    status.info_messages.remove(0);" + LE +
    b"                                }" + LE
)

idx = data.find(marker1)
if idx >= 0:
    data = data[:idx] + insert1 + data[idx:]
    print("[OK] Fix 1: info_messages en notificar_usuario")
    changes += 1
else:
    print("[FAIL] Fix 1: marker not found")
    # Debug: buscar "informativo"
    idx2 = data.find(b'"informativo"')
    if idx2 >= 0:
        print(f"  'informativo' at offset {idx2}: ...{data[idx2-80:idx2+100]}...")

# ============================================================
# FIX 2: finalizar_tarea -> multi-linea (BUG-004)
# ============================================================
old2 = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }'

if old2 in data:
    new2 = (
        b'"finalizar_tarea" => {' + LE +
        b'                        state.process_registry.kill_all();' + LE +
        b'' + LE +
        b'                        let msg = args["mensaje_final"]' + LE +
        b'                            .as_str()' + LE +
        b'                            .unwrap_or("Tarea finalizada.")' + LE +
        b'                            .to_string();' + LE +
        b'' + LE +
        b'                        let final_msg = if msg.trim().is_empty() {' + LE +
        b'                            "Tarea finalizada.".to_string()' + LE +
        b'                        } else {' + LE +
        b'                            msg' + LE +
        b'                        };' + LE +
        b'' + LE +
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
# FIX 3: read_file -> PDF/DOCX (BUG-001)
# ============================================================
old3 = b'                            match fs::read_to_string(&full_path) {'
if old3 in data:
    new3 = (
        b'                            // Detectar extension para formatos especiales (BUG-001)' + LE +
        b'                            let extension = full_path' + LE +
        b'                                .extension()' + LE +
        b'                                .and_then(|e| e.to_str())' + LE +
        b'                                .unwrap_or("")' + LE +
        b'                                .to_lowercase();' + LE +
        b'' + LE +
        b'                            if extension == "pdf" {' + LE +
        b'                                let pdf_path_str = full_path.to_string_lossy().to_string();' + LE +
        b'                                match std::process::Command::new("pdftotext")' + LE +
        b'                                    .args(["-layout", &pdf_path_str, "-"])' + LE +
        b'                                    .output()' + LE +
        b'                                {' + LE +
        b'                                    Ok(out) if out.status.success() => {' + LE +
        b'                                        let text = String::from_utf8_lossy(&out.stdout).to_string();' + LE +
        b'                                        if text.trim().is_empty() {' + LE +
        b'                                            "El PDF fue procesado pero no contiene texto extraible (puede ser escaneado). Prueba con analyze_images para OCR.".to_string()' + LE +
        b'                                        } else {' + LE +
        b'                                            format!("[PDF extraido: {}]\\n\\n{}", rel_path, text)' + LE +
        b'                                        }' + LE +
        b'                                    }' + LE +
        b'                                    _ => "No se pudo extraer texto del PDF. Instala pdftotext (poppler-utils) o PyPDF2 (pip install PyPDF2). Como alternativa, usa analyze_images.".to_string()' + LE +
        b'                                }' + LE +
        b'                            } else if extension == "docx" {' + LE +
        b'                                let docx_path_str = full_path.to_string_lossy().to_string();' + LE +
        b'                                let ps_script = format!("Add-Type -AssemblyName System.IO.Compression.FileSystem; $zip = [System.IO.Compression.ZipFile]::OpenRead(\\'{}\\'); $entry = $zip.GetEntry(\\'word/document.xml\\'); if ($entry) {{ $stream = $entry.Open(); $reader = [System.IO.StreamReader]::new($stream); $xml = $reader.ReadToEnd(); $reader.Close(); $stream.Close(); $xml -replace \\'<[^>]+>\\', \\'\\' }}; $zip.Dispose()", docx_path_str.replace("\\'", "\\'\\'"));' + LE +
        b'                                match std::process::Command::new("powershell")' + LE +
        b'                                    .args(["-NoProfile", "-Command", &ps_script])' + LE +
        b'                                    .output()' + LE +
        b'                                {' + LE +
        b'                                    Ok(out) if out.status.success() => {' + LE +
        b'                                        let text = String::from_utf8_lossy(&out.stdout).to_string();' + LE +
        b'                                        if text.trim().is_empty() {' + LE +
        b'                                            "El DOCX fue leido pero no contiene texto extraible. Instala python-docx (pip install python-docx) para mejor soporte.".to_string()' + LE +
        b'                                        } else {' + LE +
        b'                                            format!("[DOCX extraido: {}]\\n\\n{}", rel_path, text)' + LE +
        b'                                        }' + LE +
        b'                                    }' + LE +
        b'                                    _ => "No se pudo extraer texto del DOCX. Instala python-docx (pip install python-docx).".to_string()' + LE +
        b'                                }' + LE +
        b'                            } else {' + LE +
        b'                                match fs::read_to_string(&full_path) {'
    )
    data = data.replace(old3, new3)
    
    # CRUCIAL: cerrar el else { match ... } }
    # El codigo original tenia:
    #   match fs::read_to_string(&full_path) {
    #       Ok(content) => { ... }
    #       Err(e) => format!(...)
    #   }
    # Ahora tenemos:
    #   if ext == "pdf" { ... }
    #   else if ext == "docx" { ... }
    #   else {
    #       match fs::read_to_string(&full_path) {
    #           Ok(content) => { ... }
    #           Err(e) => format!(...)
    #       }
    #   }
    # Necesitamos un } adicional para cerrar el else {
    # Buscar el patron: despues del Err del match, viene "}" que cierra el match,
    # luego "                        } else {" que es el if-let del proj_name.
    # Necesitamos insertar "                        }" antes de ese else.
    else_marker = b'                        } else {\r\n                            "No hay ning'
    idx_else = data.find(else_marker)
    if idx_else > 0:
        data = data[:idx_else] + b'                        }\r\n' + data[idx_else:]
        print("[OK] Fix 3: read_file PDF/DOCX + cierre de else block agregado")
    else:
        print("[WARN] Fix 3: no se encontro el cierre del else block - buscando alternativa...")
        # Intentar otro marcador
        alt_marker = b'"No hay ning'
        idx_alt = data.find(alt_marker)
        if idx_alt > 0:
            # Buscar hacia atras: '                        } else {\r\n                            "No hay ning'
            # Retroceder para encontrar el inicio de la linea
            search_start = max(0, idx_alt - 100)
            prefix = data[search_start:idx_alt]
            last_else = prefix.rfind(b'} else {')
            if last_else >= 0:
                insert_pos = search_start + last_else
                data = data[:insert_pos] + b'                        }\r\n' + data[insert_pos:]
                print("[OK] Fix 3: cierre alternativo aplicado")
            else:
                print("[ERROR] Fix 3: no se pudo encontrar donde insertar el cierre")
        else:
            print("[ERROR] Fix 3: no se encontro 'No hay ning'")
    
    changes += 1
else:
    print("[FAIL] Fix 3: pattern not found")

# ============================================================
# VERIFICACION FINAL
# ============================================================
final_opens, final_closes, final_diff = count_braces(data)
print(f"\nFinal: {final_opens} {{, {final_closes} }}, diff={final_diff}")
print(f"Cambio en balance: {original_diff} -> {final_diff}")

# Verificar balance
expected_change = 0  # Todos los fixes deben mantener el balance
if final_diff == original_diff:
    print("[OK] Balance de llaves MANTENIDO")
else:
    print(f"[ERROR] Balance de llaves ALTERADO: diff cambió de {original_diff} a {final_diff}")
    print("        NO se escribirá el archivo para evitar corrupción.")
    sys.exit(1)

with open('src/agent.rs', 'wb') as f:
    f.write(data)

print(f"\n[DONE] {changes}/3 fixes aplicados. Archivo escrito correctamente.")
