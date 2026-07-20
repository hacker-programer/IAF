with open('src/agent.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# FIX 2: info_messages in notificar_usuario else branch
# Find the else branch of notificar_usuario by looking for '// tipo informativo'
fixed_2 = False
for i, line in enumerate(lines):
    if '// tipo informativo' in line:
        # Find the status.steps.push a few lines below
        for j in range(i, min(i+20, len(lines))):
            if 'status.steps.push(crate::state::AuditStep' in lines[j] and 'informativo' in lines[j]:
                indent2 = '                                '
                lines.insert(j, f'{indent2}status.info_messages.push(mensaje.to_string());\n')
                lines.insert(j+1, f'{indent2}if status.info_messages.len() > 100 {{ status.info_messages.remove(0); }}\n')
                print(f'Fix 2 applied at line {j+1}')
                fixed_2 = True
                break
        if fixed_2:
            break

if not fixed_2:
    print('Fix 2 NOT FOUND - searching for notificar_usuario else pattern')
    for i, line in enumerate(lines):
        if 'notificar_usuario' in line and '=>' in line and 'name' not in line:
            print(f'  notificar_usuario at line {i+1}')
        if '// tipo informativo' in line:
            print(f'  tipo informativo at line {i+1}')
            # Print surrounding lines
            for k in range(i, min(i+15, len(lines))):
                print(f'    {k+1}: {lines[k].rstrip()[:100]}')

# FIX 1b: Add missing closing brace before the if-let else
# The Fix 1 added extra nesting but missed one closing }
# After match fs::read_to_string closes, we need } to close else of if ext
# Pattern: find the match close that's followed by } else { at if-let level
for i, line in enumerate(lines):
    # Look for the pattern: after read_file PDF/DOCX else block, 
    # the match fs::read_to_string closes, then we need } for else of if ext
    if i >= 700 and 'match fs::read_to_string(&full_path)' in line:
        # This is the match inside the else of if ext
        # Find its closing } - it should be several lines below
        depth = 0
        for j in range(i, min(i+60, len(lines))):
            for ch in lines[j]:
                if ch == '{': depth += 1
                elif ch == '}': depth -= 1
            if depth == 0 and j > i:
                # This line closes the match
                # The NEXT non-empty line should be '} else {' for if let
                # We need to insert } before that
                next_line = j + 1
                while next_line < len(lines) and lines[next_line].strip() == '':
                    next_line += 1
                if next_line < len(lines) and '} else {' in lines[next_line]:
                    # Insert closing brace for else of if ext
                    # Indentation should match the if ext else block (28 spaces)
                    lines.insert(next_line, '                            }\n')
                    print(f'Fix 1b: added missing }} at line {next_line+1}')
                break
        break

# Write fixed file
with open('src/agent.rs', 'w', encoding='utf-8') as f:
    f.writelines(lines)

# Verify
with open('src/agent.rs', 'r', encoding='utf-8') as f:
    content = f.read()
depth = sum(1 for c in content if c == '{') - sum(1 for c in content if c == '}')
print(f'Final lines: {len(lines)}')
print(f'Final balance: {depth}')
