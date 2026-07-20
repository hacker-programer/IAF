with open('src/main.rs', 'r', encoding='utf-8-sig') as f:
    content = f.read()

# Cambio M1: get_agent_status - agregar finished y final_message
old_m1 = '''"running": status.running,
        "interrupted": status.interrupted,'''
new_m1 = '''"running": status.running,
        "finished": status.finished,
        "final_message": status.final_message,
        "interrupted": status.interrupted,'''

if old_m1 in content:
    content = content.replace(old_m1, new_m1)
    print('M1 (get_agent_status): OK')
else:
    print('M1: FALLO')

# Cambio M2: No limpiar agent.steps en conversaciones existentes
old_m2 = '''            agent.steps.clear();
            agent.thinking_content.clear();'''
new_m2 = '''            agent.finished = false;
            agent.final_message = None;
            // BUG FIX: Solo limpiar steps si es conversacion NUEVA. Si es existente, cargar desde sesion.
            if chat_file.is_some() {
                if let Some(ref steps) = session.steps { agent.steps = steps.clone(); }
            } else {
                agent.steps.clear();
            }
            agent.thinking_content.clear();'''

if old_m2 in content:
    content = content.replace(old_m2, new_m2)
    print('M2 (preservar steps): OK')
else:
    print('M2: FALLO')

# Cambio M3: Al terminar el agente, setear finished y final_message
old_m3 = '''                let mut ag = state_bg.active_agent.lock().unwrap();
                ag.running = false;'''
new_m3 = '''                let mut ag = state_bg.active_agent.lock().unwrap();
                ag.running = false;
                if !ag.finished { ag.finished = true; ag.final_message = match &result { Ok(resp) => Some(resp.clone()), Err(e) => Some(format!("Error: {}", e)), }; }'''

if old_m3 in content:
    content = content.replace(old_m3, new_m3)
    print('M3 (finished al terminar): OK')
else:
    print('M3: FALLO')

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print('main.rs guardado')
