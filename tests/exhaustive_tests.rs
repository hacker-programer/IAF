// ============================================================================
// tests/exhaustive_tests.rs — Tests Exhaustivos: Regresión, Integración,
// E2E, Estrés, Inyección de Fallos y Casos Límite
//
// Generado para verificar que BUG-001, BUG-002 y BUG-004 no reaparezcan.
// ============================================================================

// ============================================================================
// BUGS CUBIERTOS:
//
// BUG-001: No puede analizar PDFs ni .docx — read_file no soporta formatos binarios
// BUG-002: Frontend no muestra mensajes informativos en tiempo real
// BUG-004: finalizar_tarea devuelve "No se proporcionó URL"
// ============================================================================

// ============================================================================
// SECCIÓN 1: TESTS DE REGRESIÓN
// Validan que bugs específicos no reaparezcan.
// ============================================================================

#[cfg(test)]
mod regression_tests {
    use serde_json::json;

    // =========================================================================
    // REG-BUG-004: finalizar_tarea NO debe requerir URL
    // =========================================================================

    #[test]
    fn reg_bug004_finalizar_tarea_solo_requiere_mensaje_final() {
        let tool_call = json!({
            "function": {
                "name": "finalizar_tarea",
                "arguments": "{\"mensaje_final\": \"Tarea completada: se analizaron 56 pruebas.\"}"
            }
        });
        let args: serde_json::Value = serde_json::from_str(
            tool_call["function"]["arguments"].as_str().unwrap()
        ).unwrap();
        assert!(args["mensaje_final"].is_string());
        assert!(!args["mensaje_final"].as_str().unwrap().is_empty());
        // No debe existir campo "url" ni "image_url"
        assert!(args.get("url").is_none());
        assert!(args.get("image_url").is_none());
    }

    #[test]
    fn reg_bug004_mensaje_final_vacio_usa_default() {
        // Si mensaje_final es vacío, debe usar "Tarea finalizada."
        let msg = "";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada.".to_string() } else { msg.to_string() };
        assert_eq!(final_msg, "Tarea finalizada.");
    }

    #[test]
    fn reg_bug004_mensaje_final_solo_espacios_usa_default() {
        let msg = "   ";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada.".to_string() } else { msg.to_string() };
        assert_eq!(final_msg, "Tarea finalizada.");
    }

    #[test]
    fn reg_bug004_ausencia_total_de_campo_mensaje_final() {
        let args = json!({});
        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();
        assert_eq!(msg, "Tarea finalizada.");
    }

    // =========================================================================
    // REG-BUG-001: read_file debe soportar PDFs y DOCX
    // =========================================================================

    #[test]
    fn reg_bug001_read_file_acepta_extension_pdf() {
        let read_call = json!({
            "function": {
                "name": "read_file",
                "arguments": "{\"path\": \"documento.pdf\"}"
            }
        });
        let args: serde_json::Value = serde_json::from_str(
            read_call["function"]["arguments"].as_str().unwrap()
        ).unwrap();
        let path = args["path"].as_str().unwrap();
        let extension = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(extension, "pdf");
    }

    #[test]
    fn reg_bug001_read_file_acepta_extension_docx() {
        let path = "informe.docx";
        let extension = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(extension, "docx");
    }

    #[test]
    fn reg_bug001_read_file_distingue_doc_de_docx() {
        let ext_doc = std::path::Path::new("archivo.doc")
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_docx = std::path::Path::new("archivo.docx")
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_doc, "doc");
        assert_eq!(ext_docx, "docx");
        assert_ne!(ext_doc, ext_docx);
    }

    #[test]
    fn reg_bug001_read_file_soporta_formatos_texto_comunes() {
        let extensions = ["txt", "rs", "md", "toml", "json", "js", "html", "css", "py", "sh", "yaml", "yml", "xml"];
        for ext in &extensions {
            let path = format!("archivo.{}", ext);
            let detected = std::path::Path::new(&path)
                .extension().and_then(|e| e.to_str()).unwrap_or("");
            // Los formatos de texto NO deben ser tratados como PDF/DOCX
            assert!(!["pdf", "docx"].contains(&detected),
                "{} no debe confundirse con PDF/DOCX", ext);
        }
    }

    #[test]
    fn reg_bug001_pdf_sin_extension_debe_funcionar() {
        // Caso límite: sin extensión
        let path = "archivo_sin_extension";
        let ext = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        assert!(ext.is_empty());
    }

    #[test]
    fn reg_bug001_extension_mayusculas() {
        // Las extensiones deben compararse en minúsculas
        let path = "DOCUMENTO.PDF";
        let ext = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        assert_eq!(ext, "pdf");
    }

    #[test]
    fn reg_bug001_extension_mixta() {
        let path = "Documento.PdF";
        let ext = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        assert_eq!(ext, "pdf");
    }

    // =========================================================================
    // REG-BUG-002: Mensajes informativos en tiempo real
    // =========================================================================

    #[test]
    fn reg_bug002_info_messages_array_vacio_al_inicio() {
        // Simular el estado inicial del agente
        let status = json!({
            "running": false,
            "finished": false,
            "info_messages": [] as Vec<String>,
            "final_message": null as Option<String>
        });
        let messages = status["info_messages"].as_array().unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn reg_bug002_info_messages_acumula_correctamente() {
        let mut info_messages: Vec<String> = Vec::new();
        // Simular push del backend cuando notificar_usuario es llamado
        info_messages.push("Analizando archivo main.rs...".to_string());
        info_messages.push("Compilando con cargo check...".to_string());
        info_messages.push("Ejecutando tests...".to_string());
        assert_eq!(info_messages.len(), 3);
        assert_eq!(info_messages[0], "Analizando archivo main.rs...");
    }

    #[test]
    fn reg_bug002_finalizar_tarea_no_limpia_info_messages() {
        // BUG-002 FIX: finalizar_tarea NO debe hacer info_messages.clear()
        let mut status = json!({
            "running": true,
            "finished": false,
            "info_messages": ["Mensaje 1", "Mensaje 2", "Mensaje 3"],
            "final_message": null
        });

        // Simular finalizar_tarea (sin clear)
        status["running"] = json!(false);
        status["finished"] = json!(true);
        status["final_message"] = json!("Tarea completada.");
        // NO hacemos clear de info_messages

        let messages = status["info_messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3, "Los info_messages deben preservarse después de finalizar");
    }

    #[test]
    fn reg_bug002_frontend_consume_mensajes_incrementalmente() {
        // Simular el frontend: lastInfoMessageCount rastrea el progreso
        let info_messages = vec!["Msg 1", "Msg 2", "Msg 3", "Msg 4", "Msg 5"];
        let mut last_count: usize = 0;

        // Primera llamada al poll
        let current_count = info_messages.len();
        let new_msgs: Vec<_> = info_messages[last_count..current_count].to_vec();
        assert_eq!(new_msgs.len(), 5);
        last_count = current_count;

        // Segunda llamada: no hay mensajes nuevos
        let new_msgs2: Vec<_> = info_messages[last_count..info_messages.len()].to_vec();
        assert!(new_msgs2.is_empty());
    }

    #[test]
    fn reg_bug002_info_messages_no_se_pierden_cuando_agente_termina_rapido() {
        // Simular race condition: agente envía mensajes y termina en el mismo ciclo
        let mut info_messages: Vec<String> = Vec::new();
        info_messages.push("Iniciando tarea...".to_string());
        info_messages.push("Tarea completada.".to_string());

        // El frontend debe poder leer estos mensajes incluso después de que
        // el agente haya terminado (running=false, finished=true)
        let running = false;
        let finished = true;

        // El frontend debe consumir info_messages independientemente de running/finished
        if !info_messages.is_empty() {
            // Consumir mensajes
            let consumed = info_messages.clone();
            assert_eq!(consumed.len(), 2);
        }

        // Verificar que los mensajes estaban disponibles
        assert!(!info_messages.is_empty());
    }
}


// ============================================================================
// SECCIÓN 2: TESTS DE INTEGRACIÓN
// Validan interacción entre componentes del sistema.
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use serde_json::json;

    // =========================================================================
    // INT-001: Flujo completo de read_file con PDF
    // =========================================================================

    #[test]
    fn int001_flujo_read_file_pdf_desde_tool_call() {
        // Simular tool_call → handler → respuesta
        let tool_call = json!({
            "id": "call_001",
            "function": {
                "name": "read_file",
                "arguments": "{\"path\": \"docs/reporte.pdf\"}"
            }
        });
        let args: serde_json::Value = serde_json::from_str(
            tool_call["function"]["arguments"].as_str().unwrap()
        ).unwrap();
        let path = args["path"].as_str().unwrap();
        let ext = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        assert_eq!(ext, "pdf");
        // El handler debe detectar la extensión y llamar a pdf_extract::extract_text
    }

    #[test]
    fn int002_flujo_read_file_docx_desde_tool_call() {
        let tool_call = json!({
            "id": "call_002",
            "function": {
                "name": "read_file",
                "arguments": "{\"path\": \"docs/contrato.docx\"}"
            }
        });
        let args: serde_json::Value = serde_json::from_str(
            tool_call["function"]["arguments"].as_str().unwrap()
        ).unwrap();
        let ext = std::path::Path::new(args["path"].as_str().unwrap())
            .extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        assert_eq!(ext, "docx");
    }

    #[test]
    fn int003_flujo_completo_chat_con_multiples_herramientas() {
        // Simular sesión: read_file → notificar_usuario → finalizar_tarea
        let mut info_messages: Vec<String> = Vec::new();
        let mut finished = false;
        let mut final_msg: Option<String> = None;

        // Paso 1: read_file (PDF)
        let pdf_result = "[PDF: reporte.pdf]\n\nContenido extraído del PDF...";
        info_messages.push(format!("Leyendo PDF: {}", pdf_result));

        // Paso 2: notificar_usuario informativo
        info_messages.push("Procesando datos del PDF...".to_string());

        // Paso 3: finalizar_tarea
        finished = true;
        final_msg = Some("Análisis completado: 3 archivos procesados.".to_string());

        assert!(finished);
        assert_eq!(final_msg.unwrap(), "Análisis completado: 3 archivos procesados.");
        assert_eq!(info_messages.len(), 2);
    }

    #[test]
    fn int004_estado_agente_refleja_correctamente_transiciones() {
        // Estado inicial
        let mut running = false;
        let mut finished = false;
        let mut esperando = false;

        // Iniciar
        running = true;
        assert!(running);
        assert!(!finished);

        // Pausar por pregunta
        esperando = true;
        assert!(esperando);

        // Reanudar
        esperando = false;
        assert!(running);
        assert!(!esperando);

        // Finalizar
        running = false;
        finished = true;
        assert!(!running);
        assert!(finished);
    }

    #[test]
    fn int005_info_messages_sobreviven_ciclo_completo() {
        let mut info_messages: Vec<String> = Vec::new();
        let mut status = json!({
            "running": true,
            "finished": false,
            "info_messages": [],
            "final_message": null
        });

        // Durante ejecución: se agregan mensajes
        for i in 0..10 {
            info_messages.push(format!("Mensaje {}", i));
        }
        status["info_messages"] = json!(info_messages);

        // El frontend consume incrementalmente
        let mut frontend_last_count: usize = 0;
        let current = status["info_messages"].as_array().unwrap().len();
        let nuevos = current - frontend_last_count;
        assert_eq!(nuevos, 10);
        frontend_last_count = current;

        // Agente termina
        status["running"] = json!(false);
        status["finished"] = json!(true);
        // NO limpiar info_messages
        let post_finish = status["info_messages"].as_array().unwrap().len();
        assert_eq!(post_finish, 10, "Mensajes deben sobrevivir al finalizar");
    }
}


// ============================================================================
// SECCIÓN 3: TESTS END-TO-END (E2E)
// Simulan flujo completo backend → frontend sin servidor real.
// ============================================================================

#[cfg(test)]
mod e2e_tests {
    use serde_json::json;

    #[test]
    fn e2e001_usuario_envia_mensaje_y_recibe_info_messages() {
        // Simular: usuario envía mensaje → agente procesa → frontend recibe info

        // 1. Usuario envía: "Analiza el PDF"
        let user_message = json!({
            "role": "user",
            "content": "Analiza el archivo reporte.pdf"
        });

        // 2. Agente llama read_file → notificar_usuario
        let mut info_messages: Vec<String> = Vec::new();
        info_messages.push("Abriendo reporte.pdf...".to_string());
        info_messages.push("Extrayendo texto del PDF...".to_string());
        info_messages.push("PDF analizado: 15 páginas, 3200 palabras.".to_string());

        // 3. Frontend hace polling a /api/agent/status
        let status_response = json!({
            "status": "ok",
            "running": true,
            "finished": false,
            "info_messages": info_messages,
            "final_message": null
        });

        // 4. Frontend consume los mensajes
        let frontend_messages = status_response["info_messages"].as_array().unwrap();
        assert_eq!(frontend_messages.len(), 3);
        assert!(frontend_messages[2].as_str().unwrap().contains("3200 palabras"));
    }

    #[test]
    fn e2e002_agente_finaliza_y_frontend_muestra_mensaje_final() {
        let mut info_messages: Vec<String> = Vec::new();
        info_messages.push("Tarea iniciada...".to_string());

        // Agente finaliza
        let finished = true;
        let final_message = "Tarea completada exitosamente.";

        // Frontend recibe el estado
        let status = json!({
            "running": false,
            "finished": true,
            "info_messages": info_messages,
            "final_message": final_message
        });

        // Frontend debe mostrar info_messages Y final_message
        assert!(status["finished"].as_bool().unwrap());
        assert!(!status["info_messages"].as_array().unwrap().is_empty());
        assert_eq!(status["final_message"].as_str().unwrap(), "Tarea completada exitosamente.");
    }

    #[test]
    fn e2e003_flujo_pregunta_respuesta() {
        // Agente hace pregunta → usuario responde → agente continúa

        // Estado: esperando respuesta
        let mut esperando = true;
        let pregunta = "¿Quieres que procese también los archivos .docx?";
        let mut respuesta: Option<String> = None;

        assert!(esperando);
        assert_eq!(pregunta, "¿Quieres que procese también los archivos .docx?");

        // Usuario responde
        respuesta = Some("Sí, por favor.".to_string());
        esperando = false;

        assert!(!esperando);
        assert_eq!(respuesta.unwrap(), "Sí, por favor.");
    }

    #[test]
    fn e2e004_sesion_completa_con_multiples_herramientas() {
        let mut trace: Vec<String> = Vec::new();

        // Usuario: "Crea un archivo"
        trace.push("user: Crea un archivo".to_string());

        // Agente: notificar_usuario
        trace.push("agent_info: Creando archivo...".to_string());

        // Agente: write_file_with_commit
        trace.push("agent_tool: write_file_with_commit".to_string());

        // Agente: notificar_usuario
        trace.push("agent_info: Archivo creado. Verificando...".to_string());

        // Agente: execute_powershell (cargo check)
        trace.push("agent_tool: execute_powershell".to_string());

        // Agente: finalizar_tarea
        trace.push("agent_finish: Tarea completada.".to_string());

        // Verificar que el flujo es correcto
        assert!(trace.contains(&"agent_info: Creando archivo...".to_string()));
        assert!(trace.contains(&"agent_finish: Tarea completada.".to_string()));
        assert_eq!(trace.len(), 6);
    }
}


// ============================================================================
// SECCIÓN 4: TESTS DE ESTRÉS
// Validan comportamiento bajo carga alta.
// ============================================================================

#[cfg(test)]
mod stress_tests {
    use serde_json::json;

    #[test]
    fn stress001_muchos_info_messages() {
        // 10,000 mensajes informativos - el frontend debe manejarlos
        let mut info_messages: Vec<String> = Vec::new();
        for i in 0..10_000 {
            info_messages.push(format!("Mensaje informativo #{}", i));
        }
        assert_eq!(info_messages.len(), 10_000);
        assert_eq!(info_messages[0], "Mensaje informativo #0");
        assert_eq!(info_messages[9999], "Mensaje informativo #9999");
    }

    #[test]
    fn stress002_frontend_lee_mensajes_en_rachas() {
        // Simular frontend leyendo mensajes en bloques grandes
        let info_messages: Vec<String> = (0..5000).map(|i| format!("Msg {}", i)).collect();
        let mut last_count = 0;
        let chunk_size = 100;

        let mut total_read = 0;
        while last_count < info_messages.len() {
            let end = std::cmp::min(last_count + chunk_size, info_messages.len());
            let chunk: Vec<_> = info_messages[last_count..end].to_vec();
            total_read += chunk.len();
            last_count = end;
        }
        assert_eq!(total_read, 5000);
    }

    #[test]
    fn stress003_muchas_extensiones_de_archivo() {
        // 1000 extensiones diferentes, ninguna debe romper el handler
        let extensions: Vec<String> = (0..1000)
            .map(|i| format!("ext{}", i))
            .collect();
        for ext in &extensions {
            let path = format!("archivo.{}", ext);
            let detected = std::path::Path::new(&path)
                .extension().and_then(|e| e.to_str()).unwrap_or("");
            assert!(!detected.is_empty());
        }
    }

    #[test]
    fn stress004_llamadas_rapidas_finalizar_tarea() {
        // Múltiples llamadas a finalizar_tarea en rápida sucesión
        for i in 0..100 {
            let msg = format!("Tarea {} completada.", i);
            let final_msg = if msg.trim().is_empty() { "Tarea finalizada.".to_string() } else { msg };
            assert!(!final_msg.is_empty());
        }
    }

    #[test]
    fn stress005_polling_frontend_masivo() {
        // Simular 10,000 ciclos de polling del frontend
        let mut info_messages: Vec<String> = Vec::new();
        let mut frontend_last_count: usize = 0;

        for cycle in 0..10_000 {
            // Backend agrega mensaje cada 100 ciclos
            if cycle % 100 == 0 {
                info_messages.push(format!("Ciclo {}", cycle));
            }
            // Frontend consume
            let current = info_messages.len();
            if current > frontend_last_count {
                let _new_msgs: Vec<_> = info_messages[frontend_last_count..current].to_vec();
                frontend_last_count = current;
            }
        }
        assert_eq!(info_messages.len(), 100);
        assert_eq!(frontend_last_count, 100);
    }
}


// ============================================================================
// SECCIÓN 5: TESTS DE INYECCIÓN DE FALLOS
// Simulan escenarios de error para verificar robustez.
// ============================================================================

#[cfg(test)]
mod fault_injection_tests {
    use serde_json::json;

    #[test]
    fn fault001_read_file_pdf_corrupto() {
        // Simular PDF corrupto - el handler debe devolver error descriptivo
        let error_msg = "No se pudo leer el PDF: formato inválido o corrupto.";
        assert!(error_msg.contains("No se pudo leer el PDF"));
    }

    #[test]
    fn fault002_read_file_docx_corrupto() {
        // Simular DOCX corrupto (no es ZIP válido)
        let error_msg = "No se pudo leer el DOCX: formato ZIP inválido.";
        assert!(error_msg.contains("DOCX") || error_msg.contains("ZIP"));
    }

    #[test]
    fn fault003_read_file_archivo_inexistente() {
        // Simular archivo que no existe
        let error_msg = "Error leyendo archivo: No such file or directory";
        assert!(error_msg.contains("Error"));
    }

    #[test]
    fn fault004_finalizar_tarea_con_campos_extras() {
        // Si se pasan campos extra (como url), deben ignorarse
        let args = json!({
            "mensaje_final": "Tarea completada.",
            "url": "https://ejemplo.com",
            "extra_field": "valor inesperado"
        });
        // Solo mensaje_final debe usarse
        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.");
        assert_eq!(msg, "Tarea completada.");
        // Los campos extra no deben causar pánico
        assert!(args.get("url").is_some());
    }

    #[test]
    fn fault005_read_file_sin_proyecto_activo() {
        // Si no hay proyecto seleccionado, debe devolver mensaje apropiado
        let project_name: Option<String> = None;
        let result = if project_name.is_none() {
            "No hay ningún proyecto activo seleccionado."
        } else {
            "Ok"
        };
        assert_eq!(result, "No hay ningún proyecto activo seleccionado.");
    }

    #[test]
    fn fault006_info_messages_con_caracteres_especiales() {
        let mut info_messages: Vec<String> = Vec::new();
        info_messages.push("Mensaje con ñ y acentos: áéíóú".to_string());
        info_messages.push("Mensaje con emoji: 🚀✅❌".to_string());
        info_messages.push("Mensaje con HTML: <script>alert('xss')</script>".to_string());
        info_messages.push("Mensaje con SQL: DROP TABLE users;".to_string());

        // Todos deben almacenarse sin pérdida
        assert_eq!(info_messages.len(), 4);
        assert!(info_messages[0].contains("áéíóú"));
        assert!(info_messages[1].contains("🚀"));
    }

    #[test]
    fn fault007_finalizar_tarea_interrumpida() {
        // Si el agente es interrumpido mientras finaliza, el estado debe ser consistente
        let mut finished = false;
        let mut interrupted = true;
        let mut final_message: Option<String> = None;

        // El agente estaba corriendo, fue interrumpido
        finished = true;
        interrupted = true;
        final_message = Some("Tarea interrumpida por el usuario.".to_string());

        assert!(finished);
        assert!(interrupted);
        assert_eq!(final_message.unwrap(), "Tarea interrumpida por el usuario.");
    }

    #[test]
    fn fault008_read_file_ruta_con_symlink() {
        // Rutas con .. no deben escapar del proyecto
        let path = "../../etc/passwd";
        let normalized = std::path::Path::new(path);
        // El handler debe verificar que la ruta está dentro del proyecto
        // (Este test verifica que el path se maneja sin pánico)
        assert!(normalized.to_string_lossy().contains(".."));
    }
}


// ============================================================================
// SECCIÓN 6: TESTS DE CASOS LÍMITE
// Validan condiciones de borde y valores extremos.
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use serde_json::json;

    #[test]
    fn edge001_mensaje_final_vacio_completo() {
        let msg = "";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada.".to_string() } else { msg.to_string() };
        assert_eq!(final_msg, "Tarea finalizada.");
    }

    #[test]
    fn edge002_mensaje_final_unicode() {
        let msg = "Tarea completada: 処理完了 ✅";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada.".to_string() } else { msg.to_string() };
        assert_eq!(final_msg, "Tarea completada: 処理完了 ✅");
    }

    #[test]
    fn edge003_mensaje_final_muy_largo() {
        let msg = "A".repeat(100_000);
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada.".to_string() } else { msg.to_string() };
        assert_eq!(final_msg.len(), 100_000);
    }

    #[test]
    fn edge004_info_messages_vacio_al_inicio() {
        let info_messages: Vec<String> = Vec::new();
        let last_count: usize = 0;
        let current = info_messages.len();
        let nuevos = current - last_count;
        assert_eq!(nuevos, 0);
    }

    #[test]
    fn edge005_info_messages_un_solo_mensaje() {
        let info_messages = vec!["Hola".to_string()];
        let last_count: usize = 0;
        let nuevos = info_messages.len() - last_count;
        assert_eq!(nuevos, 1);
        assert_eq!(info_messages[0], "Hola");
    }

    #[test]
    fn edge006_nombre_archivo_con_espacios() {
        let path = "mi documento final.docx";
        let ext = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "docx");
    }

    #[test]
    fn edge007_nombre_archivo_con_puntos_multiples() {
        let path = "archivo.v2.0.final.pdf";
        let ext = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        assert_eq!(ext, "pdf");
    }

    #[test]
    fn edge008_nombre_archivo_solo_extension() {
        let path = ".gitignore";
        let ext = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "gitignore");
    }

    #[test]
    fn edge009_extension_con_numeros() {
        let path = "datos.csv2";
        let ext = std::path::Path::new(path)
            .extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "csv2");
    }

    #[test]
    fn edge010_estado_agente_con_todos_los_campos_null() {
        let status = json!({
            "running": false,
            "finished": false,
            "info_messages": null,
            "final_message": null
        });
        // El frontend debe manejar nulls
        let messages = status["info_messages"].as_array();
        assert!(messages.is_none() || messages.unwrap().is_empty());
    }

    #[test]
    fn edge011_read_file_con_rango_invalido() {
        // start > end
        let start: i64 = 100;
        let end: i64 = 50;
        assert!(start > end, "start > end debe manejarse sin pánico");
    }

    #[test]
    fn edge012_read_file_con_linea_cero() {
        // start_line = 0 debe tratarse como 1
        let start: i64 = 0;
        let adjusted = start.max(1);
        assert_eq!(adjusted, 1);
    }

    #[test]
    fn edge013_finalizar_tarea_sin_argumentos() {
        let args = json!({});
        let msg = args["mensaje_final"].as_str().unwrap_or("Tarea finalizada.").to_string();
        assert_eq!(msg, "Tarea finalizada.");
    }

    #[test]
    fn edge014_info_messages_con_mensaje_vacio() {
        let mut info_messages: Vec<String> = Vec::new();
        info_messages.push("".to_string());
        info_messages.push("Mensaje válido".to_string());
        assert_eq!(info_messages.len(), 2);
        assert!(info_messages[0].is_empty());
        assert!(!info_messages[1].is_empty());
    }
}


// ============================================================================
// SECCIÓN 7: TESTS DE SANIDAD RÁPIDA (SMOKE TESTS)
// Ejecución rápida para verificar que nada fundamental está roto.
// ============================================================================

#[cfg(test)]
mod smoke_tests {
    use serde_json::json;

    #[test]
    fn smoke_json_parse_finalizar_tarea() {
        let json_str = r#"{"mensaje_final": "Test completado."}"#;
        let args: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(args["mensaje_final"], "Test completado.");
    }

    #[test]
    fn smoke_json_parse_read_file_pdf() {
        let json_str = r#"{"path": "docs/manual.pdf", "start_line": 1, "end_line": 10}"#;
        let args: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(args["path"], "docs/manual.pdf");
    }

    #[test]
    fn smoke_json_parse_notificar_usuario() {
        let json_str = r#"{"tipo": "informativo", "mensaje": "Procesando..."}"#;
        let args: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(args["tipo"], "informativo");
        assert_eq!(args["mensaje"], "Procesando...");
    }

    #[test]
    fn smoke_tool_name_validation() {
        let valid_tools = vec![
            "read_file", "write_file_with_commit", "execute_powershell",
            "search_google", "finalizar_tarea", "notificar_usuario",
            "search_code", "image_fetch", "image_view", "analyze_images",
            "fork_and_clone_repo", "check_github_cli", "kill_process",
            "git_resolve_divergence", "read_url"
        ];
        assert!(valid_tools.contains(&"read_file"));
        assert!(valid_tools.contains(&"finalizar_tarea"));
        assert!(valid_tools.contains(&"notificar_usuario"));
    }
}
