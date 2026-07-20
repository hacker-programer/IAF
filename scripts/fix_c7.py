with open('src/agent.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Bloque exacto a reemplazar (lineas 1997-2025)
old_block = '''if let Some(ref session_id) = *session_id_opt {
                                let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", session_id));
                                if chat_file.exists() {
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

new_block = '''if let Some(ref session_id) = *session_id_opt {
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

if old_block in content:
    content = content.replace(old_block, new_block)
    print('C7-FIX: OK')
else:
    print('C7-FIX: FALLO - bloque no encontrado')
    # Buscar aproximado
    idx = content.find('if let Some(ref session_id) = *session_id_opt {')
    if idx >= 0:
        # Encontrar el bloque desde ahi (buscar en la zona de compresion)
        chunk = content[idx:idx+1500]
        if 'Guardar en el archivo JSON' in chunk:
            print('  Bloque encontrado cerca de pos', idx)
        else:
            print('  No es el bloque de compresion, pos:', idx)

with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print('Guardado')
