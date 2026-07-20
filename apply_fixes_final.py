import subprocess

# Restore clean file
result = subprocess.run(['git', 'show', '1f5e228:src/agent.rs'], capture_output=True, text=True, encoding='utf-8', errors='replace')
lines = result.stdout.splitlines(keepends=True)
print(f'Restored: {len(lines)} lines')

# FIX 1: PDF/DOCX in read_file
for i, line in enumerate(lines):
    if i >= 655 and 'match fs::read_to_string(&full_path)' in line:
        indent = '                            '
        replacement = [
            f'{indent}let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();\n',
            f'{indent}if ext == "pdf" || ext == "docx" {{\n',
            f'{indent}    let path_str = full_path.to_string_lossy().to_string();\n',
            f'{indent}    if ext == "pdf" {{\n',
            f'{indent}        match std::process::Command::new("pdftotext").args(["-layout", &path_str, "-"]).output() {{\n',
            f'{indent}            Ok(out) if out.status.success() => {{\n',
            f'{indent}                let t = String::from_utf8_lossy(&out.stdout).to_string();\n',
            f'{indent}                if t.trim().is_empty() {{ "PDF sin texto extraible. Usa analyze_images para ver el PDF.".to_string() }}\n',
            f'{indent}                else {{ format!("[PDF: {{}}]\n\n{{}}", rel_path, t) }}\n',
            f'{indent}            }}\n',
            f'{indent}            _ => "No se pudo leer el PDF. Instala pdftotext o PyPDF2. Usa analyze_images como alternativa.".to_string()\n',
            f'{indent}        }}\n',
            f'{indent}    }} else {{\n',
            f'{indent}        "El archivo DOCX no se puede leer directamente. Instala python-docx: pip install python-docx. Usa analyze_images como alternativa.".to_string()\n',
            f'{indent}    }}\n',
            f'{indent}}} else {{\n',
            f'{indent}    match fs::read_to_string(&full_path) {{\n',
        ]
        lines[i:i+1] = replacement
        print(f'Fix 1 applied at line {i+1}')
        break

# FIX 2: info_messages in notificar_usuario
for i, line in enumerate(lines):
    if '// tipo informativo' in line:
        for j in range(i, min(i+15, len(lines))):
            if 'status.steps.push' in lines[j]:
                ind2 = '                                '
                lines.insert(j, f'{ind2}status.info_messages.push(mensaje.to_string());\n')
                lines.insert(j+1, f'{ind2}if status.info_messages.len() > 100 {{ status.info_messages.remove(0); }}\n')
                print(f'Fix 2 applied before line {j+1}')
                break
        break

# FIX 3: refactor finalizar_tarea
for i, line in enumerate(lines):
    if '"finalizar_tarea" =>' in line and '"name"' not in line:
        j = i + 1
        while j < len(lines) and '"image_fetch"' not in lines[j]:
            j += 1
        ind3 = '                        '
        new_code = [
            f'{ind3}state.process_registry.kill_all();\n',
            f'{ind3}let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();\n',
            f'{ind3}let final_msg = if msg.trim().is_empty() {{ "Tarea finalizada.".to_string() }} else {{ msg }};\n',
            f'{ind3}{{ let mut status = state.active_agent.lock().unwrap();\n',
            f'{ind3}    status.finished = true; status.final_message = Some(final_msg.clone());\n',
            f'{ind3}    status.running = false; status.esperando_respuesta_usuario = false;\n',
            f'{ind3}    status.esperando_aprobacion_plan = false; status.info_messages.clear();\n',
            f'{ind3}    status.steps.push(crate::state::AuditStep {{\n',
            f'{ind3}        step_type: "thinking".to_string(), title: "Tarea Finalizada".to_string(),\n',
            f'{ind3}        detail: format!("El agente ha finalizado la tarea: {{}}", final_msg),\n',
            f'{ind3}        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),\n',
            f'{ind3}    }});\n',
            f'{ind3}    if let Some(ref s_id) = session_id {{ save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps); }}\n',
            f'{ind3}}}\n',
            f'{ind3}final_message = Some(final_msg);\n',
            f'{ind3}"Tarea finalizada correctamente.".to_string()\n',
            f'{ind3}}}\n',
        ]
        lines[i+1:j] = new_code
        print(f'Fix 3 applied at line {i+1}')
        break

# Verify balance
content = ''.join(lines)
depth = sum(1 for c in content if c == '{') - sum(1 for c in content if c == '}')
print(f'Final lines: {len(lines)}')
print(f'Final balance: {depth}')

with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print('Done!')
