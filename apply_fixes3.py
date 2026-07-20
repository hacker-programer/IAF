with open('src/agent.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# FIX 2: info_messages in notificar_usuario
# Search more broadly - find '// tipo informativo' then add info_messages before the first status.steps.push
for i, line in enumerate(lines):
    if '// tipo informativo' in line:
        # Find the next 'status.steps.push' within 15 lines
        for j in range(i, min(i+15, len(lines))):
            if 'status.steps.push' in lines[j]:
                indent2 = '                                '
                lines.insert(j, f'{indent2}status.info_messages.push(mensaje.to_string());\n')
                lines.insert(j+1, f'{indent2}if status.info_messages.len() > 100 {{ status.info_messages.remove(0); }}\n')
                print(f'Fix 2 applied before line {j+1}: added info_messages.push')
                break
        break

# Count braces in the file and find remaining imbalance
with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.writelines(lines)

with open('src/agent.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Count structural braces (ignoring strings)
depth = 0
for ch in content:
    if ch == '{': depth += 1
    elif ch == '}': depth -= 1

print(f'Lines: {len(lines)}')
print(f'Balance: {depth}')

# If balance > 0, add closing braces at the end
if depth > 0:
    with open('src/agent.rs', 'a', encoding='utf-8') as f:
        for _ in range(depth):
            f.write('}\n')
    print(f'Added {depth} closing braces at end of file')
    
# Verify final balance
with open('src/agent.rs', 'r', encoding='utf-8') as f:
    final = f.read()
fd = sum(1 for c in final if c == '{') - sum(1 for c in final if c == '}')
print(f'Final balance: {fd}')
