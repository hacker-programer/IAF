import sys

with open('src/agent.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# FIX 1: PDF/DOCX support in read_file
for i, line in enumerate(lines):
    if i >= 655 and 'match fs::read_to_string(&full_path)' in line and 'read_file' not in line:
        indent = '                            '
        lines[i] = f'{indent}let ext = full_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();\n'
        lines.insert(i+1, f'{indent}if ext == "pdf" || ext == "docx" {{\n')
        lines.insert(i+2, f'{indent}    let path_str = full_path.to_string_lossy().to_string();\n')
        lines.insert(i+3, f'{indent}    if ext == "pdf" {{\n')
        lines.insert(i+4, f'{indent}        match std::process::Command::new("pdftotext").args(["-layout", &path_str, "-"]).output() {{\n')
        lines.insert(i+5, f'{indent}            Ok(out) if out.status.success() => {{\n')
        lines.insert(i+6, f'{indent}                let t = String::from_utf8_lossy(&out.stdout).to_string();\n')
        lines.insert(i+7, f'{indent}                if t.trim().is_empty() {{ "PDF sin texto extraible. Usa analyze_images para ver el PDF.".to_string() }}\n')
        lines.insert(i+8, f'{indent}                else {{ format!("[PDF: {{}}]\n\n{{}}", rel_path, t) }}\n')
        lines.insert(i+9, f'{indent}            }}\n')
        lines.insert(i+10, f'{indent}            _ => "No se pudo leer el PDF. Instala pdftotext o PyPDF2. Usa analyze_images como alternativa.".to_string()\n')
        lines.insert(i+11, f'{indent}        }}\n')
        lines.insert(i+12, f'{indent}    }} else {{\n')
        lines.insert(i+13, f'{indent}        "El archivo DOCX no se puede leer directamente. Instala python-docx: pip install python-docx. Usa analyze_images como alternativa.".to_string()\n')
        lines.insert(i+14, f'{indent}    }}\n')
        lines.insert(i+15, f'{indent}}} else {{\n')
        lines.insert(i+16, f'{indent}    match fs::read_to_string(&full_path) {{\n')
        print(f'Fix 1 applied at line {i+1}')
        break

# FIX 2: info_messages in notificar_usuario else branch
for i, line in enumerate(lines):
    if '// tipo informativo' in line:
        for j in range(i, min(i+30, len(lines))):
            if 'status.steps.push(crate::state::AuditStep' in lines[j] and 'informativo' in lines[j]:
                indent2 = '                                '
                lines.insert(j, f'{indent2}status.info_messages.push(mensaje.to_string());\n')
                lines.insert(j+1, f'{indent2}if status.info_messages.len() > 100 {{ status.info_messages.remove(0); }}\n')
                print(f'Fix 2 applied at line {j+1}')
                break
        break

# FIX 3: refactor finalizar_tarea
for i, line in enumerate(lines):
    if '"finalizar_tarea" =>' in line and 'name' not in line and 'content' not in line:
        j = i + 1
        while j < len(lines) and '"image_fetch"' not in lines[j]:
            j += 1
        indent3 = '                        '
        new_code = f'''{indent3}state.process_registry.kill_all();
{indent3}let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();
{indent3}let final_msg = if msg.trim().is_empty() {{ "Tarea finalizada.".to_string() }} else {{ msg }};
{indent3}{{ let mut status = state.active_agent.lock().unwrap();
{indent3}    status.finished = true; status.final_message = Some(final_msg.clone());
{indent3}    status.running = false; status.esperando_respuesta_usuario = false;
{indent3}    status.esperando_aprobacion_plan = false; status.info_messages.clear();
{indent3}    status.steps.push(crate::state::AuditStep {{
{indent3}        step_type: "thinking".to_string(), title: "Tarea Finalizada".to_string(),
{indent3}        detail: format!("El agente ha finalizado la tarea: {{}}", final_msg),
{indent3}        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
{indent3}    }});
{indent3}    if let Some(ref s_id) = session_id {{ save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps); }}
{indent3}}}
{indent3}final_message = Some(final_msg);
{indent3}"Tarea finalizada correctamente.".to_string()
'''
        new_lines = new_code.splitlines(keepends=True)
        for k in range(len(new_lines)):
            if not new_lines[k].endswith('\n'):
                new_lines[k] += '\n'
        lines[i+1:j] = new_lines
        print(f'Fix 3 applied at line {i+1}')
        break

# Write fixed file
with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.writelines(lines)

# Verify balance
with open('src/agent.rs', 'r', encoding='utf-8') as f:
    content = f.read()
depth = sum(1 for c in content if c == '{') - sum(1 for c in content if c == '}')
print(f'Final lines: {len(lines)}')
print(f'Final balance: {depth}')
