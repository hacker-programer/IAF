with open('src/agent.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# C5: Reemplazar finalizar_tarea
old_c5 = '''"finalizar_tarea" => {
                        // Limpiar todos los procesos hijo registrados antes de finalizar
                        state.process_registry.kill_all();
                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();
                        final_message = Some(msg);
                        "Tarea finalizada correctamente.".to_string()
                    }'''

new_c5 = '''"finalizar_tarea" => {
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
                    }'''

if old_c5 in content:
    content = content.replace(old_c5, new_c5)
    print('C5: OK')
else:
    print('C5: FALLO - texto no encontrado')
    # Buscar la linea
    idx = content.find('"finalizar_tarea"')
    if idx >= 0:
        print(content[idx:idx+500])

# C6: Reemplazar retorno final_message
old_c6 = '''if let Some(msg) = final_message {
                state.process_registry.kill_all();
                return Ok(msg);
            }'''

new_c6 = '''if let Some(msg) = final_message {
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

if old_c6 in content:
    content = content.replace(old_c6, new_c6)
    print('C6: OK')
else:
    print('C6: FALLO - texto no encontrado')

# C7: Reemplazar bloque de compresion de contexto
old_c7_start = '''if let Some(ref session_id) = *session_id_opt {
                                let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", session_id));'''
new_c7_start = '''if let Some(ref session_id) = *session_id_opt {
                                if let Some(chat_file) = find_chat_file_by_session_id(&state.base_workspace, session_id) {'''

if old_c7_start in content:
    content = content.replace(old_c7_start, new_c7_start)
    # Tambien necesitamos cerrar el if extra: buscar el ultimo } del bloque y agregar otro }
    # NOTA: esto requiere matching de llaves, lo hare en otro paso
    print('C7: inicio reemplazado')
else:
    print('C7: FALLO - texto no encontrado')

with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print('Guardado')
