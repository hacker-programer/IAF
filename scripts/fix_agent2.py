import re

with open('src/agent.rs', 'r', encoding='utf-8-sig') as f:
    lines = f.readlines()

# Encontrar y reemplazar bloques con join(format!("{}.json", s_id))
i = 0
replaced = 0
while i < len(lines):
    if 'join(format!("{}.json", s_id))' in lines[i]:
        # Encontrar el inicio: if let Some(ref s_id) = session_id {
        start = i - 1
        while start > 0 and 'if let Some(ref s_id) = session_id {' not in lines[start]:
            start -= 1
        
        # Encontrar el final contando llaves
        brace_count = 0
        end = start
        for j in range(start, len(lines)):
            brace_count += lines[j].count('{') - lines[j].count('}')
            if brace_count == 0 and j > start:
                end = j
                break
        
        # Determinar indentacion y variable
        indent = '            '  # 12 espacios
        var = 'content'
        block_text = ''.join(lines[start:end+1])
        if 'mensaje' in block_text:
            indent = '                        '  # 24 espacios
            var = 'mensaje'
        
        # Reemplazar
        new_lines = [
            f'{indent}if let Some(ref s_id) = session_id {{\n',
            f'{indent}    save_agent_message_to_disk(&state, s_id, "agent", &{var});\n',
            f'{indent}}}\n'
        ]
        lines[start:end+1] = new_lines
        replaced += 1
        i = start + len(new_lines)
    else:
        i += 1

print(f'Bloques reemplazados: {replaced}')

with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.writelines(lines)
print('Guardado')
