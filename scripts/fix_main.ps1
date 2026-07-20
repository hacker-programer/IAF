# Script para aplicar correcciones a main.rs
$file = "src\main.rs"
$content = Get-Content $file -Raw

# Cambio 1: get_agent_status - agregar finished y final_message
$content = $content -replace '(?s)("running": status\.running,\s+"interrupted": status\.interrupted,)',
    '"running": status.running,`n        "finished": status.finished,`n        "final_message": status.final_message,`n        "interrupted": status.interrupted,'

# Cambio 2: No limpiar steps si conversacion existente
$content = $content -replace '(?s)(agent\.steps\.clear\(\);)\s+(agent\.thinking_content\.clear\(\);)\s+(agent\.esperando_respuesta_usuario)',
    'agent.finished = false;`n            agent.final_message = None;`n            // BUG FIX: Solo limpiar steps si es conversacion NUEVA`n            if chat_file.is_some() { if let Some(ref steps) = session.steps { agent.steps = steps.clone(); } } else { agent.steps.clear(); }`n            agent.thinking_content.clear();`n            agent.esperando_respuesta_usuario'

# Cambio 3: Al terminar el agente, setear finished y final_message
$content = $content -replace '(?s)(let mut ag = state_bg\.active_agent\.lock\(\)\.unwrap\(\);)\s+(ag\.running = false;)',
    'let mut ag = state_bg.active_agent.lock().unwrap();`n                ag.running = false;`n                if !ag.finished { ag.finished = true; ag.final_message = match &result { Ok(resp) => Some(resp.clone()), Err(e) => Some(format!("Error: {}", e)), }; }'

# Guardar
[System.IO.File]::WriteAllText((Resolve-Path $file).Path, $content, (New-Object System.Text.UTF8Encoding $false))
Write-Output "Script ejecutado"