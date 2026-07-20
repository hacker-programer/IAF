#!/usr/bin/env python3
"""Fix 1 (BUG-002) y Fix 2 (BUG-004) solamente."""
import sys

with open('src/agent.rs', 'rb') as f:
    data = f.read()

LE = b'\r\n'
orig = data.count(b'{') - data.count(b'}')
print(f'Balance original: {orig}')

# === FIX 1: info_messages en notificar_usuario ===
m1 = b'status.steps.push(crate::state::AuditStep {\r\n                                    step_type: "informativo"'
i1 = (b'// Agregar a info_messages para frontend (BUG-002)' + LE +
      b'                                status.info_messages.push(mensaje.to_string());' + LE +
      b'                                if status.info_messages.len() > 100 {' + LE +
      b'                                    status.info_messages.remove(0);' + LE +
      b'                                }' + LE +
      b'                                ')
idx = data.find(m1)
if idx >= 0:
    data = data[:idx] + i1 + data[idx:]
    print('[OK] Fix 1: info_messages')
else:
    print('[FAIL] Fix 1')
    sys.exit(1)

# === FIX 2: finalizar_tarea multi-linea ===
o2 = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }'

if o2 in data:
    n2 = (b'"finalizar_tarea" => {' + LE +
          b'                        state.process_registry.kill_all();' + LE + LE +
          b'                        let msg = args["mensaje_final"]' + LE +
          b'                            .as_str()' + LE +
          b'                            .unwrap_or("Tarea finalizada.")' + LE +
          b'                            .to_string();' + LE + LE +
          b'                        let final_msg = if msg.trim().is_empty() {' + LE +
          b'                            "Tarea finalizada.".to_string()' + LE +
          b'                        } else {' + LE +
          b'                            msg' + LE +
          b'                        };' + LE + LE +
          b'                        {' + LE +
          b'                            let mut status = state.active_agent.lock().unwrap();' + LE +
          b'                            status.finished = true;' + LE +
          b'                            status.final_message = Some(final_msg.clone());' + LE +
          b'                            status.running = false;' + LE +
          b'                            status.esperando_respuesta_usuario = false;' + LE +
          b'                            status.esperando_aprobacion_plan = false;' + LE +
          b'                            status.info_messages.clear();' + LE +
          b'                            status.steps.push(crate::state::AuditStep {' + LE +
          b'                                step_type: "thinking".to_string(),' + LE +
          b'                                title: "Tarea Finalizada".to_string(),' + LE +
          b'                                detail: format!("El agente ha finalizado la tarea: {}", final_msg),' + LE +
          b'                                timestamp: std::time::SystemTime::now()' + LE +
          b'                                    .duration_since(std::time::UNIX_EPOCH)' + LE +
          b'                                    .unwrap()' + LE +
          b'                                    .as_secs(),' + LE +
          b'                            });' + LE +
          b'                            if let Some(ref s_id) = session_id {' + LE +
          b'                                save_chat_steps_to_disk(' + LE +
          b'                                    &state,' + LE +
          b'                                    &Some(s_id.clone()),' + LE +
          b'                                    &status.steps,' + LE +
          b'                                );' + LE +
          b'                            }' + LE +
          b'                        }' + LE +
          b'                        final_message = Some(final_msg);' + LE +
          b'                        "Tarea finalizada correctamente.".to_string()' + LE +
          b'                    }')
    data = data.replace(o2, n2)
    print('[OK] Fix 2: finalizar_tarea')
else:
    print('[FAIL] Fix 2')

bal = data.count(b'{') - data.count(b'}')
print(f'Balance final: {bal}')
if bal != orig:
    print(f'[ERROR] Balance cambio: {orig} -> {bal}')
    sys.exit(1)

with open('src/agent.rs', 'wb') as f:
    f.write(data)
print('[DONE] Archivo escrito')
