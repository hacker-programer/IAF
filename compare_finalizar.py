with open('src/agent.rs','rb') as f:
    data = f.read()

# Find the handler (skip the tool definition)
idx1 = data.find(b'"finalizar_tarea"')
idx2 = data.find(b'"finalizar_tarea"', idx1 + 1)

# The handler starts at idx2
# Find where it ends - next handler starts with "image_fetch"
end = data.find(b'"image_fetch"', idx2)
actual = data[idx2:end].rstrip()

# The old2 pattern from fix_12.py
old2 = b'"finalizar_tarea" => {                        // Limpiar todos los procesos hijo registrados antes de finalizar                        state.process_registry.kill_all();                        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();                        // Notificar finalizacion en el estado del agente para que el frontend lo detecte                        {                            let mut status = state.active_agent.lock().unwrap();                            status.finished = true;                            status.final_message = Some(msg.clone());                            status.running = false;                            status.steps.push(crate::state::AuditStep {                                step_type: "thinking".to_string(),                                title: "Tarea Finalizada".to_string(),                                detail: format!("El agente ha finalizado la tarea: {}", msg),                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),                            });                            if let Some(ref s_id) = session_id {                                save_chat_steps_to_disk(&state, &Some(s_id.clone()), &status.steps);                            }                        }                        final_message = Some(msg);                        "Tarea finalizada correctamente.".to_string()                    }'

print("old2 length:", len(old2))
print("actual length:", len(actual))
print("Match:", old2 == actual)

if old2 != actual:
    # Find first difference
    for i in range(min(len(old2), len(actual))):
        if old2[i] != actual[i]:
            print(f"First diff at byte {i}: old2={old2[i]:02x} actual={actual[i]:02x}")
            print(f"  old2[{i-5}:{i+20}] = {repr(old2[max(0,i-5):i+20])}")
            print(f"  actual[{i-5}:{i+20}] = {repr(actual[max(0,i-5):i+20])}")
            break
    if len(old2) != len(actual):
        print(f"Length diff: old2={len(old2)}, actual={len(actual)}")
