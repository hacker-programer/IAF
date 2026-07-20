with open('src/agent.rs', 'r', encoding='utf-8-sig') as f:
    content = f.read()

# C1: PathBuf
content = content.replace('use std::path::Path;', 'use std::path::{Path, PathBuf};')

# C2: Reemplazar save_chat_steps_to_disk
old_c2_start = content.find('fn save_chat_steps_to_disk')
old_c2_end = content.find('fn get_project_path')
assert old_c2_start >= 0 and old_c2_end > old_c2_start

new_c2 = '''
fn save_chat_steps_to_disk(state: &AppState, session_id_opt: &Option<String>, steps: &[crate::state::AuditStep]) {
    if let Some(ref session_id) = *session_id_opt {
        if let Some(chat_file) = find_chat_file_by_session_id(&state.base_workspace, session_id) {
            if let Ok(content) = fs::read_to_string(&chat_file) {
                if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                    session.steps = Some(steps.to_vec());
                    let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                }
            }
        }
    }
}

fn find_chat_file_by_session_id(base_workspace: &Path, session_id: &str) -> Option<PathBuf> {
    let chats_dir = base_workspace.join(".config").join("chats");
    if !chats_dir.exists() { return None; }
    if let Ok(entries) = std::fs::read_dir(&chats_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub_entry in sub_entries.filter_map(|e| e.ok()) {
                        let sub_path = sub_entry.path();
                        if sub_path.is_file() {
                            if let Some(fname) = sub_path.file_stem().and_then(|s| s.to_str()) {
                                if fname.contains(session_id) and sub_path.extension().and_then(|e| e.to_str()) == Some("json") {
                                    return Some(sub_path);
                                }
                            }
                        }
                    }
                }
            } else if path.is_file() {
                if let Some(fname) = path.file_stem().and_then(|s| s.to_str()) {
                    if fname.contains(session_id) and path.extension().and_then(|e| e.to_str()) == Some("json") {
                        return Some(path);
                    }
                }
            }
        }
    }
    let old_format = chats_dir.join(format!("{}.json", session_id));
    if old_format.exists() { return Some(old_format); }
    None
}

fn save_agent_message_to_disk(state: &AppState, session_id: &str, role: &str, content: &str) {
    if let Some(chat_file) = find_chat_file_by_session_id(&state.base_workspace, session_id) {
        if let Ok(file_content) = fs::read_to_string(&chat_file) {
            if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&file_content) {
                let is_duplicate = session.messages.last()
                    .map(|m| m.content == content && m.role == role).unwrap_or(false);
                if !is_duplicate {
                    session.messages.push(crate::state::ChatMessage {
                        role: role.to_string(), content: content.to_string(),
                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                    });
                    if let Some(parent) = chat_file.parent() { let _ = fs::create_dir_all(parent); }
                    let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                }
            }
        }
    }
}

'''
content = content[:old_c2_start] + new_c2 + content[old_c2_end:]

# C3+C4: Reemplazar bloques inline con join(format!("{}.json", s_id))
lines = content.split('\n')
i = 0
replaced = 0
while i < len(lines):
    if 'join(format!("{}.json", s_id))' in lines[i]:
        start = i - 1
        while start > 0 and 'if let Some(ref s_id) = session_id {' not in lines[start]:
            start -= 1
        brace_count = 0
        end = start
        for j in range(start, len(lines)):
            brace_count += lines[j].count('{') - lines[j].count('}')
            if brace_count == 0 and j > start:
                end = j
                break
        block_text = '\n'.join(lines[start:end+1])
        indent = '            '
        var = 'content'
        if 'mensaje' in block_text:
            indent = '                        '
            var = 'mensaje'
        new_lines = [
            f'{indent}if let Some(ref s_id) = session_id {{',
            f'{indent}    save_agent_message_to_disk(&state, s_id, "agent", &{var});',
            f'{indent}}}'
        ]
        lines[start:end+1] = new_lines
        replaced += 1
        i = start + len(new_lines)
    else:
        i += 1
content = '\n'.join(lines)
print(f'C3+C4: {replaced} bloques reemplazados')

with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print('Fase 1 guardada (C1-C4)')
