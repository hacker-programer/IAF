with open('src/agent.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# C5: finalizar_tarea
for i, line in enumerate(lines):
    if '"finalizar_tarea" => {' in line:
        # Encontrar el final del bloque
        brace_count = 0
        end = i
        for j in range(i, len(lines)):
            brace_count += lines[j].count('{') - lines[j].count('}')
            if brace_count == 0 and j > i:
                end = j
                break
        print(f'C5: bloque {i+1}-{end+1}')
        new_block = '''                    "finalizar_tarea" => {
                        // Limpiar todos los procesos hijo registrados antes de finalizar
                        state.process_registry.kill_all();
                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();
                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte
                        {
                            let mut status = state.active_agent.lock().unwrap();
                            status.finished = true;
                            status.final_message = Some(msg.clone());
                            status.running = false;
                            status.steps.push(crate::state::AuditStep {
                                step_type: "thinking".to_string(),
                                title: "Tarea Finalizada".to_string(),
                                detail: format!("El agente ha finalizado la tarea: {}", msg),
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                            });
                            if let Some(ref s_id) = session_id {
                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);
                            }
                        }
                        final_message = Some(msg);
                        "Tarea finalizada correctamente.".to_string()
                    }
'''
        lines[i:end+1] = new_block.split('\n')
        print('C5: OK')
        break

# C6: retorno final_message
for i, line in enumerate(lines):
    if 'if let Some(msg) = final_message {' in line:
        brace_count = 0
        end = i
        for j in range(i, len(lines)):
            brace_count += lines[j].count('{') - lines[j].count('}')
            if brace_count == 0 and j > i:
                end = j
                break
        print(f'C6: bloque {i+1}-{end+1}')
        new_block = '''            if let Some(msg) = final_message {
                // Asegurar que el estado refleje la finalizacion
                {
                    let mut status = state.active_agent.lock().unwrap();
                    status.finished = true;
                    status.final_message = Some(msg.clone());
                    status.running = false;
                }
                state.process_registry.kill_all();
                return Ok(msg);
            }'''
        lines[i:end+1] = new_block.split('\n')
        print('C6: OK')
        break

# C7: Bloque de compresion - reemplazar el bloque completo
for i, line in enumerate(lines):
    if 'Guardar en el archivo JSON de la conversaci' in line:
        start = i + 1  # if let Some(ref session_id) = *session_id_opt {
        # Contar llaves para encontrar el final del if let Some(ref session_id)
        brace_count = 0
        end = start
        for j in range(start, len(lines)):
            brace_count += lines[j].count('{') - lines[j].count('}')
            if brace_count == 0 and j > start:
                end = j
                break
        print(f'C7: bloque {start+1}-{end+1}')
        
        # Reconstruir el bloque
        new_block = '''                            if let Some(ref session_id) = *session_id_opt {
                                if let Some(chat_file) = find_chat_file_by_session_id(&state.base_workspace, session_id) {
                                    if let Ok(content) = fs::read_to_string(&chat_file) {
                                        if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                                            let mut disk_messages = Vec::new();
                                            for m in messages.iter() {
                                                let role = m["role"].as_str().unwrap_or("");
                                                let content_str = m["content"].as_str().unwrap_or("");
                                                if role == "user" {
                                                    disk_messages.push(crate::state::ChatMessage {
                                                        role: "user".to_string(),
                                                        content: content_str.to_string(),
                                                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                                    });
                                                } else if role == "assistant" {
                                                    disk_messages.push(crate::state::ChatMessage {
                                                        role: "agent".to_string(),
                                                        content: content_str.to_string(),
                                                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                                    });
                                                }
                                            }
                                            session.messages = disk_messages;
                                            let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                                        }
                                    }
                                }
                            }'''
        lines[start:end+1] = new_block.split('\n')
        print('C7: OK')
        break

with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.writelines(lines)
print('Guardado')
