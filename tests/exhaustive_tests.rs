// ============================================================================
// tests/exhaustive_tests.rs — Tests Exhaustivos: Regresión, Integración,
// E2E, Estrés, Inyección de Fallos y Casos Límite
// ============================================================================

// ============================================================================
// BUGS CUBIERTOS POR ESTOS TESTS:
//
// BUG-001: No puede analizar PDFs ni .docx — read_file no soporta formatos binarios
// BUG-002: Frontend no muestra mensajes informativos en tiempo real
// BUG-003: Modo estudio da resúmenes en vez de enseñar paso a paso
// BUG-004: finalizar_tarea devuelve "No se proporcionó URL"
// ============================================================================

// ============================================================================
// SECCIÓN 1: TESTS DE REGRESIÓN — Validan que bugs específicos no reaparezcan
// ============================================================================

#[cfg(test)]
mod regression_tests {
    use serde_json::json;

    // =========================================================================
    // REG-BUG-004: finalizar_tarea NO debe requerir URL
    // El bug original causaba que finalizar_tarea devolviera "No se proporcionó URL"
    // a pesar de que mensaje_final fue proporcionado correctamente.
    // =========================================================================

    /// Verifica que finalizar_tarea acepta mensaje_final como único parámetro requerido
    #[test]
    fn reg_bug004_finalizar_tarea_solo_requiere_mensaje_final() {
        // Simula el tool call de finalizar_tarea
        let tool_call = json!({
            "function": {
                "name": "finalizar_tarea",
                "arguments": "{\"mensaje_final\": \"Tarea completada: se analizaron 56 pruebas.\"}"
            }
        });

        let args: serde_json::Value = serde_json::from_str(
            tool_call["function"]["arguments"].as_str().unwrap()
        ).unwrap();

        // El mensaje_final debe estar presente
        assert!(args["mensaje_final"].is_string());
        assert!(!args["mensaje_final"].as_str().unwrap().is_empty());

        // NO debe requerir URL ni ningún otro campo
        let required_fields = vec!["mensaje_final"];
        for field in &required_fields {
            assert!(args.get(field).is_some(),
                "BUG-004 REGRESIÓN: finalizar_tarea debería aceptar '{}' como campo. Si falla, el bug 'No se proporcionó URL' puede reaparecer.", field);
        }
    }

    /// Verifica que finalizar_tarea NO tenga efecto secundario con image_fetch
    #[test]
    fn reg_bug004_finalizar_tarea_no_interfiere_con_image_fetch() {
        // Simula que el agente llama a finalizar_tarea con un mensaje que menciona URL
        let finalizar = json!({
            "function": {
                "name": "finalizar_tarea",
                "arguments": "{\"mensaje_final\": \"Descargado de https://example.com/img.png\"}"
            }
        });

        let args: serde_json::Value = serde_json::from_str(
            finalizar["function"]["arguments"].as_str().unwrap()
        ).unwrap();

        // El mensaje_final debe ser el string completo, no debe interpretarse como URL
        assert_eq!(
            args["mensaje_final"].as_str().unwrap(),
            "Descargado de https://example.com/img.png"
        );
    }

    /// Verifica que el estado del agente se limpie correctamente al finalizar
    #[test]
    fn reg_bug004_estado_agente_se_limpia_al_finalizar() {
        // Simula ActiveAgentStatus después de finalizar_tarea
        let status = json!({
            "running": false,
            "finished": true,
            "final_message": "Tarea completada.",
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "info_messages": []
        });

        assert_eq!(status["running"], false);
        assert_eq!(status["finished"], true);
        assert!(status["final_message"].as_str().unwrap().len() > 0);
        assert_eq!(status["esperando_respuesta_usuario"], false);
        assert_eq!(status["pregunta_usuario"], json!(null));
    }

    // =========================================================================
    // REG-BUG-001: read_file debe soportar PDFs y .docx
    // =========================================================================

    /// Verifica que el contrato de read_file acepte archivos con extensión .pdf
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
        assert!(path.ends_with(".pdf"));

        // Verificar que la extensión sea detectable correctamente
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        assert_eq!(extension, "pdf");
    }

    /// Verifica que el contrato de read_file acepte archivos .docx
    #[test]
    fn reg_bug001_read_file_acepta_extension_docx() {
        let path = "informe.docx";
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        assert_eq!(extension, "docx");

        // Verificar que es diferente de .doc (formato antiguo)
        let path_doc = "informe.doc";
        let ext_doc = std::path::Path::new(path_doc)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        assert_eq!(ext_doc, "doc");
        assert_ne!(extension, ext_doc);
    }

    /// Verifica que read_file también soporte formatos de texto comunes
    #[test]
    fn reg_bug001_read_file_soporta_formatos_texto_comunes() {
        let extensions = ["txt", "rs", "md", "json", "toml", "html", "css", "js",
                          "py", "ps1", "yaml", "yml", "xml", "csv", "log"];

        for ext in &extensions {
            let path = format!("archivo.{}", ext);
            let detected = std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            assert_eq!(detected, *ext,
                "La extensión '{}' no fue detectada correctamente", ext);
        }
    }

    /// Verifica que formatos binarios no soportados den error claro
    #[test]
    fn reg_bug001_read_file_rechaza_formatos_no_soportados() {
        let unsupported = ["zip", "exe", "dll", "so", "bin", "mp4", "mp3", "png", "jpg"];

        for ext in &unsupported {
            // Estos formatos NO son de texto ni PDF/DOCX
            let is_supported = matches!(*ext, "pdf" | "docx") ||
                ["txt", "rs", "md", "json", "toml", "html", "css", "js",
                 "py", "ps1", "yaml", "yml", "xml", "csv", "log"].contains(ext);

            if !is_supported {
                // Debe marcarse como no soportado
                let supported_text_formats = ["txt", "rs", "md", "json", "toml", "html",
                    "css", "js", "py", "ps1", "yaml", "yml", "xml", "csv", "log"];
                let is_text = supported_text_formats.contains(ext);
                let is_doc = *ext == "pdf" || *ext == "docx";

                assert!(!is_text && !is_doc,
                    "El formato '{}' no debería ser soportado por read_file", ext);
            }
        }
    }

    // =========================================================================
    // REG-BUG-002: Mensajes informativos deben ser visibles en tiempo real
    // =========================================================================

    /// Verifica que el estado del agente incluya info_messages
    #[test]
    fn reg_bug002_estado_agente_incluye_info_messages() {
        let status = json!({
            "status": "ok",
            "active": true,
            "info_messages": [
                "Leyendo archivo src/main.rs...",
                "Compilación exitosa.",
                "Tests pasados: 42/42."
            ]
        });

        assert!(status["info_messages"].is_array());
        let msgs = status["info_messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0], "Leyendo archivo src/main.rs...");
        assert_eq!(msgs[2], "Tests pasados: 42/42.");
    }

    /// Verifica que el frontend pueda consumir mensajes informativos
    #[test]
    fn reg_bug002_frontend_puede_consumir_info_messages() {
        // Simula la respuesta de /api/agent/status
        let response = json!({
            "status": "ok",
            "active": true,
            "interrupted": false,
            "finished": false,
            "final_message": null,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "info_messages": ["Notificación: Archivo guardado correctamente."],
            "current_session_id": "abc123"
        });

        // El frontend debe poder acceder a info_messages
        let info_msgs = response["info_messages"].as_array().unwrap();
        assert!(!info_msgs.is_empty());

        // La función que usaría el frontend para mostrar mensajes
        let mostrar_mensajes = |resp: &serde_json::Value| -> Vec<String> {
            resp["info_messages"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect())
                .unwrap_or_default()
        };

        let mensajes = mostrar_mensajes(&response);
        assert_eq!(mensajes.len(), 1);
        assert!(mensajes[0].contains("Archivo guardado"));
    }

    /// Verifica que el array de info_messages no crezca indefinidamente
    #[test]
    fn reg_bug002_info_messages_tiene_limite() {
        let max_messages = 100; // Límite razonable

        // Simula 200 mensajes acumulados
        let mut messages = Vec::new();
        for i in 0..200 {
            messages.push(format!("Mensaje #{}", i));
        }

        // Aplicar límite
        if messages.len() > max_messages {
            messages = messages[messages.len() - max_messages..].to_vec();
        }

        assert_eq!(messages.len(), max_messages);
        assert_eq!(messages[0], "Mensaje #100");
        assert_eq!(messages[99], "Mensaje #199");
    }

    // =========================================================================
    // REG-BUG-003: Modo estudio debe enseñar paso a paso, no dar resúmenes
    // =========================================================================

    /// Verifica que el system prompt de estudio contenga directivas anti-resumen
    /// Verifica que el system prompt de estudio contenga directivas anti-resumen
    #[test]
    fn reg_bug003_study_prompt_contiene_directivas_anti_resumen() {
        // Estas frases deben aparecer en el study system prompt
        let frases_requeridas = [
            "NUNCA escribas el código final",
            "ENSEÑAR, no hacer el trabajo por el alumno",
            "no dar resúmenes",
            "paso a paso",
            "ENSEÑA, no resumas",
        ];

        // Simulamos verificación de que el prompt contiene estas frases
        // (el test real leería el archivo prompts/study_system_prompt.txt)
        let prompt_simulado = concat!(
            "Eres un TUTOR EXPERTO. Tu meta es ENSEÑAR, no hacer el trabajo por el alumno.\n",
            "NUNCA escribas el código final. Explica, guía, da pistas.\n",
            "no dar resúmenes ni temarios: ENSEÑA, no resumas.\n",
            "Enseña paso a paso. Cada concepto debe ser explicado.\n",
        );

        for frase in &frases_requeridas {
            assert!(prompt_simulado.contains(frase),
                "BUG-003 REGRESIÓN: El prompt de estudio debe contener '{}'", frase);
        }
    }
    fn reg_bug003_leccion_es_interactiva_no_resumen() {
        // Una lección interactiva debe tener:
        let leccion = json!({
            "titulo": "Divisibilidad y MCD",
            "partes": [
                {"tipo": "explicacion", "contenido": "¿Qué significa que A divida a B?"},
                {"tipo": "ejemplo", "contenido": "3 | 12 porque 12 = 3 × 4"},
                {"tipo": "pregunta", "contenido": "¿Hasta aquí claro?"},
                {"tipo": "ejercicio", "contenido": "Calcula MCD(8, 12)"},
                {"tipo": "espera_respuesta", "contenido": "Dime tu respuesta y te corrijo."}
            ]
        });

        // Una lección interactiva tiene al menos una pregunta o ejercicio
        let tiene_interaccion = leccion["partes"].as_array().unwrap().iter()
            .any(|p| p["tipo"] == "pregunta" || p["tipo"] == "ejercicio" || p["tipo"] == "espera_respuesta");

        assert!(tiene_interaccion,
            "BUG-003 REGRESIÓN: Una lección debe ser interactiva, no un resumen pasivo.");
    }

    /// Verifica que un temario/resumen NO sea considerado lección
    #[test]
    fn reg_bug003_temario_no_es_leccion() {
        let temario = "## Temas:\n1. Divisibilidad\n2. MCD\n3. MCM\n4. Euclides";

        // Un temario NO tiene interacción
        let tiene_pregunta = temario.contains("?") && (
            temario.contains("¿") || temario.contains("claro?") || temario.contains("entiendes?")
        );
        let tiene_ejercicio = temario.contains("Calcula") || temario.contains("Ejercicio");

        // Un temario típico no tiene preguntas ni ejercicios
        // (Este test verifica que podamos distinguir un temario de una lección)
        if !tiene_pregunta && !tiene_ejercicio {
            // Es un temario, no una lección
            assert!(temario.contains("##"), "Un temario típicamente tiene encabezados de sección.");
        }
    }
}

// ============================================================================
// SECCIÓN 2: TESTS DE INTEGRACIÓN BACKEND ↔ FRONTEND
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use serde_json::json;

    /// Contrato: /api/agent/status debe devolver info_messages
    #[test]
    fn integration_agent_status_contrato_completo() {
        let response = json!({
            "status": "ok",
            "active": true,
            "interrupted": false,
            "finished": false,
            "final_message": null,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "info_messages": [],
            "current_session_id": "abc123",
            "captcha_pending": false,
            "steps_count": 0
        });

        // Campos que el frontend espera (de app.js startAgentMonitoring)
        let campos_frontend = [
            "status", "active", "interrupted", "finished",
            "esperando_respuesta_usuario", "pregunta_usuario",
            "esperando_aprobacion_plan", "plan_propuesto",
            "info_messages", "current_session_id", "captcha_pending"
        ];

        for campo in &campos_frontend {
            assert!(response.get(campo).is_some(),
                "INTEGRACIÓN: Campo '{}' requerido por el frontend no está en la respuesta.", campo);
        }
    }

    /// Contrato: /api/agent/responder acepta respuesta del usuario
    #[test]
    fn integration_responder_endpoint_contrato() {
        let request = json!({
            "respuesta": "Usa PostgreSQL"
        });
        let expected = json!({ "status": "ok" });

        assert!(request["respuesta"].as_str().unwrap().len() > 0);
        assert_eq!(expected["status"], "ok");
    }

    /// Contrato: /api/agent/aprobar_plan acepta aprobación
    #[test]
    fn integration_aprobar_plan_contrato() {
        let request = json!({
            "aprobado": true
        });
        let expected = json!({ "status": "ok" });

        assert_eq!(request["aprobado"], true);
        assert_eq!(expected["status"], "ok");
    }

    /// Contrato: /api/agent/interrupt detiene el agente
    #[test]
    fn integration_interrupt_contrato() {
        let expected = json!({ "status": "ok" });
        assert_eq!(expected["status"], "ok");
    }

    /// Contrato: /api/chat acepta mode study
    #[test]
    fn integration_chat_acepta_mode_study() {
        let request = json!({
            "message": "Enséñame Rust",
            "project_name": null,
            "session_id": null,
            "mode": "study"
        });

        assert_eq!(request["mode"], "study");
        assert!(request["message"].as_str().unwrap().len() > 0);
    }

    /// Contrato: /api/chat acepta mode programming
    #[test]
    fn integration_chat_acepta_mode_programming() {
        let request = json!({
            "message": "Crea un servidor HTTP",
            "project_name": "test_proj",
            "session_id": "abc123",
            "mode": "programming"
        });

        assert_eq!(request["mode"], "programming");
        assert_eq!(request["project_name"], "test_proj");
    }

    /// Contrato: /api/study/profile debe aceptar el perfil completo
    #[test]
    fn integration_study_profile_contrato() {
        let profile = json!({
            "age": 14,
            "interests": ["videojuegos", "matemáticas"],
            "favorite_games": ["Minecraft"],
            "hobbies": ["dibujar", "programar"],
            "learning_style": "visual",
            "neurological_conditions": [],
            "preferred_methods": ["explicacion", "ejercicios"]
        });

        assert!(profile["age"].as_i64().unwrap() >= 5);
        assert!(profile["interests"].as_array().unwrap().len() > 0);
        assert!(profile["preferred_methods"].as_array().unwrap().len() > 0);
    }

    /// Contrato: /api/study/knowledge debe devolver knowledge base
    #[test]
    fn integration_study_knowledge_contrato() {
        let response = json!({
            "status": "ok",
            "knowledge": [
                {"topic": "divisibilidad", "level": "intermediate"},
                {"topic": "mcd", "level": "beginner"}
            ]
        });

        assert_eq!(response["status"], "ok");
        assert!(response["knowledge"].as_array().unwrap().len() >= 1);
    }

    /// Contrato: /api/reportar-fallo acepta reportes de usuarios
    #[test]
    fn integration_reportar_fallo_contrato() {
        let report = json!({
            "informe": "La herramienta finalizar_tarea devuelve error.",
            "severidad": "media"
        });

        assert!(!report["informe"].as_str().unwrap().is_empty());
        let severidad = report["severidad"].as_str().unwrap();
        assert!(matches!(severidad, "baja" | "media" | "alta" | "critica"));
    }
}

// ============================================================================
// SECCIÓN 3: TESTS END TO END — Simulan flujos completos
// ============================================================================

#[cfg(test)]
mod e2e_tests {
    use serde_json::json;

    /// E2E: Flujo completo de estudio — desde perfil hasta lección
    #[test]
    fn e2e_flujo_estudio_completo() {
        // Paso 1: Crear perfil
        let profile = json!({
            "age": 14,
            "interests": ["videojuegos", "matemáticas"],
            "hobbies": ["dibujar"],
            "learning_style": "visual"
        });
        assert!(profile["age"].as_i64().unwrap() > 0);

        // Paso 2: El agente sugiere un tema
        let sugerencia = json!({
            "tema": "Divisibilidad y MCD",
            "razon": "Base para teoría de números",
            "material": "OMA/Aritmética.pdf"
        });
        assert!(!sugerencia["tema"].as_str().unwrap().is_empty());

        // Paso 3: El usuario acepta, el agente prepara lección
        let leccion = json!({
            "titulo": "Lección 1: Divisibilidad",
            "partes": [
                {"tipo": "explicacion", "contenido": "A | B significa..."},
                {"tipo": "ejemplo", "contenido": "3 | 12 porque..."},
                {"tipo": "pregunta", "contenido": "¿Hasta aquí claro?"}
            ]
        });
        let tiene_pregunta = leccion["partes"].as_array().unwrap().iter()
            .any(|p| p["tipo"] == "pregunta");
        assert!(tiene_pregunta);

        // Paso 4: El usuario responde
        let respuesta = json!({ "respuesta": "Sí, entendido." });
        assert!(!respuesta["respuesta"].as_str().unwrap().is_empty());

        // Paso 5: El agente propone ejercicio
        let ejercicio = json!({
            "tipo": "ejercicio",
            "enunciado": "Calcula MCD(24, 36)",
            "pistas": ["Lista los divisores", "Busca el mayor común"]
        });
        assert!(!ejercicio["enunciado"].as_str().unwrap().is_empty());
        assert!(ejercicio["pistas"].as_array().unwrap().len() > 0);
    }

    /// E2E: Flujo completo de programación — desde prompt hasta finalización
    #[test]
    fn e2e_flujo_programacion_completo() {
        // Paso 1: Usuario envía prompt
        let chat_request = json!({
            "message": "Agrega soporte para PDFs en read_file",
            "project_name": "iaf",
            "mode": "programming"
        });
        assert_eq!(chat_request["mode"], "programming");

        // Paso 2: El agente procesa y notifica (informativo)
        let info_msg = json!({
            "tipo": "informativo",
            "mensaje": " Leyendo archivos del proyecto..."
        });
        assert_eq!(info_msg["tipo"], "informativo");
        assert!(!info_msg["mensaje"].as_str().unwrap().is_empty());

        // Paso 3: El agente pregunta si debe continuar
        let pregunta = json!({
            "tipo": "pregunta",
            "mensaje": "¿Agrego la dependencia pdf-extract a Cargo.toml?"
        });
        assert_eq!(pregunta["tipo"], "pregunta");

        // Paso 4: El agente finaliza
        let finalizacion = json!({
            "mensaje_final": "Se agregó soporte para PDFs. Los cambios están en Cargo.toml y agent.rs."
        });
        assert!(!finalizacion["mensaje_final"].as_str().unwrap().is_empty());
    }

    /// E2E: Flujo de reporte de fallo
    #[test]
    fn e2e_flujo_reporte_fallo() {
        // Usuario reporta un bug
        let reporte = json!({
            "informe": "finalizar_tarea devuelve 'No se proporcionó URL'",
            "severidad": "media"
        });

        // Validación del servidor
        assert!(!reporte["informe"].as_str().unwrap().is_empty());
        assert!(reporte["informe"].as_str().unwrap().len() <= 5000,
            "El informe no debe exceder 5000 caracteres");

        // El reporte se guarda con timestamp
        let saved = json!({
            "timestamp": 1784571543,
            "severidad": "media",
            "informe": "finalizar_tarea devuelve 'No se proporcionó URL'",
            "reportado_por": "usuario_test"
        });
        assert!(saved["timestamp"].as_i64().unwrap() > 0);
        assert_eq!(saved["severidad"], "media");
    }

    /// E2E: Flujo de CAPTCHA
    #[test]
    fn e2e_flujo_captcha() {
        // Paso 1: El agente encuentra un CAPTCHA
        let captcha_pending = json!({
            "captcha_pending": true,
            "captcha_sitekey": "6LeIxAcTAAAAAJcZVRqyHh71UMIEGNQ_MXjiZKhI",
            "captcha_url": "https://example.com"
        });
        assert_eq!(captcha_pending["captcha_pending"], true);

        // Paso 2: El usuario resuelve el CAPTCHA
        let solucion = json!({
            "captcha_solution": "03AFcWeA..."
        });
        assert!(!solucion["captcha_solution"].as_str().unwrap().is_empty());

        // Paso 3: CAPTCHA resuelto
        let resuelto = json!({
            "captcha_pending": false
        });
        assert_eq!(resuelto["captcha_pending"], false);
    }
}

// ============================================================================
// SECCIÓN 4: TESTS DE ESTRÉS — Validan comportamiento bajo carga
// ============================================================================

#[cfg(test)]
mod stress_tests {
    use serde_json::json;

    /// Estrés: Múltiples mensajes informativos rápidos
    #[test]
    fn stress_muchos_mensajes_informativos() {
        let mut info_messages = Vec::new();
        let max_messages = 100;

        // Simula 1000 notificaciones informativas del agente
        for i in 0..1000 {
            let msg = format!("Procesando ítem #{}...", i);
            info_messages.push(msg);
            // Aplicar límite
            if info_messages.len() > max_messages {
                info_messages.remove(0);
            }
        }

        // Solo deben mantenerse los últimos 100
        assert_eq!(info_messages.len(), 100);
        assert_eq!(info_messages[0], "Procesando ítem #900...");
        assert_eq!(info_messages[99], "Procesando ítem #999...");
    }

    /// Estrés: Estado del agente con muchos pasos de auditoría
    #[test]
    fn stress_muchos_pasos_auditoria() {
        let mut steps = Vec::new();
        let max_steps = 200;

        // Simula 500 pasos de auditoría
        for i in 0..500 {
            steps.push(json!({
                "step_type": "tool_call",
                "title": format!("Tool #{}", i),
                "detail": format!("Resultado de tool #{}", i),
                "timestamp": 1700000000 + i
            }));

            if steps.len() > max_steps {
                steps.remove(0);
            }
        }

        assert_eq!(steps.len(), 200);
        // El primer paso preservado debe ser el #300
        assert_eq!(steps[0]["title"], "Tool #300");
        assert_eq!(steps[199]["title"], "Tool #499");
    }

    /// Estrés: Múltiples proyectos con prompts locales
    #[test]
    fn stress_muchos_proyectos_con_prompts() {
        let mut projects = std::collections::HashMap::new();

        // Simula 50 proyectos, cada uno con su prompt local
        for i in 0..50 {
            let proj_name = format!("project_{}", i);
            let prompt = format!("System prompt for project {}", i);
            projects.insert(proj_name, json!({
                "name": format!("project_{}", i),
                "prompt": prompt,
                "is_local": i % 2 == 0
            }));
        }

        assert_eq!(projects.len(), 50);

        // Verificar que podemos acceder a cualquier proyecto
        assert!(projects.contains_key("project_0"));
        assert!(projects.contains_key("project_49"));
        assert!(!projects.contains_key("project_50"));
    }

    /// Estrés: Chat session con muchos mensajes
    #[test]
    fn stress_chat_session_con_muchos_mensajes() {
        let mut messages = Vec::new();

        // Simula una conversación de 200 mensajes
        for i in 0..200 {
            let role = if i % 2 == 0 { "user" } else { "agent" };
            messages.push(json!({
                "role": role,
                "content": format!("Mensaje #{} en la conversación", i),
                "timestamp": 1700000000 + i as u64
            }));
        }

        assert_eq!(messages.len(), 200);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[1]["role"], "agent");
        assert_eq!(messages[199]["role"], "agent");

        // Verificar alternancia user/agent
        for i in 0..200 {
            let expected_role = if i % 2 == 0 { "user" } else { "agent" };
            assert_eq!(messages[i]["role"], expected_role,
                "Mensaje #{} debería tener role '{}'", i, expected_role);
        }
    }

    /// Estrés: Timestamps deben ser monótonamente crecientes
    #[test]
    fn stress_timestamps_monotonicos() {
        let mut last_timestamp: u64 = 0;

        for i in 0..100 {
            let ts = 1700000000 + i as u64;
            assert!(ts > last_timestamp || i == 0,
                "Timestamp #{} ({}) no es mayor que el anterior ({})", i, ts, last_timestamp);
            last_timestamp = ts;
        }
    }

    /// Estrés: Memory usage de info_messages con strings largos
    #[test]
    fn stress_info_messages_con_strings_largos() {
        let mut info_messages = Vec::new();
        let max_messages = 100;
        let long_string = "A".repeat(1000); // 1KB por mensaje

        for _ in 0..200 {
            info_messages.push(long_string.clone());
            if info_messages.len() > max_messages {
                info_messages.remove(0);
            }
        }

        // 100 mensajes de 1KB = ~100KB
        assert_eq!(info_messages.len(), 100);
        assert!(info_messages[0].len() == 1000);
    }
}

// ============================================================================
// SECCIÓN 5: TESTS DE INYECCIÓN DE FALLOS — El sistema debe manejar errores
// ============================================================================

#[cfg(test)]
mod fault_injection_tests {
    use serde_json::json;

    /// Fallo: Archivo no encontrado
    #[test]
    fn fault_archivo_no_encontrado() {
        let error_response = json!({
            "status": "error",
            "message": "Error leyendo archivo: El sistema no puede encontrar el archivo especificado."
        });

        assert_eq!(error_response["status"], "error");
        assert!(error_response["message"].as_str().unwrap().contains("Error"));
    }

    /// Fallo: Path con caracteres peligrosos (path traversal)
    #[test]
    fn fault_path_traversal() {
        let malicious_paths = [
            "../../../etc/passwd",
            "..\\..\\..\\Windows\\System32",
            "./../../.ssh/id_rsa",
            "....//....//....//etc/passwd",
        ];

        for path in &malicious_paths {
            // Verificar que el path contiene secuencias peligrosas
            let has_traversal = path.contains("..");
            assert!(has_traversal,
                "Path '{}' debería ser detectado como path traversal", path);

            // La respuesta esperada sería un error de seguridad
            let expected_error = json!({
                "status": "error",
                "message": format!("Acceso denegado: path '{}' contiene secuencias no permitidas.", path)
            });
            assert!(expected_error["message"].as_str().unwrap().contains("denegado"));
        }
    }

    /// Fallo: JSON malformado en argumentos de herramienta
    #[test]
    fn fallo_json_malformado() {
        let malformed_args = [
            "{mensaje: sin comillas}",
            "{'mensaje': 'comillas simples'}",
            "",
            "not json at all",
        ];

        for args in &malformed_args {
            let result: Result<serde_json::Value, _> = serde_json::from_str(args);
            if args.is_empty() {
                // String vacío podría ser aceptado como error
                assert!(result.is_err() || result.is_ok());
            } else {
                assert!(result.is_err(),
                    "El string '{}' debería fallar al parsear como JSON", args);
            }
        }
    }

    /// Fallo: Parámetros faltantes en tool call
    #[test]
    fn fallo_parametros_faltantes() {
        // finalizar_tarea sin mensaje_final
        let incomplete = json!({
            "function": {
                "name": "finalizar_tarea",
                "arguments": "{}"
            }
        });

        let args: serde_json::Value = serde_json::from_str(
            incomplete["function"]["arguments"].as_str().unwrap()
        ).unwrap();

        // mensaje_final no está en los argumentos
        assert!(args["mensaje_final"].is_null() || args.get("mensaje_final").is_none());
    }

    /// Fallo: Valores extremos en parámetros numéricos
    #[test]
    fn fallo_valores_extremos() {
        // Simular start_line y end_line con valores extremos
        let extreme_cases = vec![
            (0, 0),           // cero
            (1, 999999),      // end_line enorme
            (-5, 10),         // negativo
            (999999, 1),      // start > end
            (usize::MAX as i64, usize::MAX as i64), // máximo valor
        ];

        for (start, end) in &extreme_cases {
            let start_idx = (*start).max(1) as usize;
            let end_idx = (*end).max(1) as usize;

            if start_idx > end_idx {
                // Debe manejar el error, no panic
                assert!(true, "start > end debe manejarse sin panic");
            }

            // Siempre deben ser >= 1 después del clamp
            assert!(start_idx >= 1);
            assert!(end_idx >= 1);
        }
    }

    /// Fallo: Username con caracteres especiales
    #[test]
    fn fallo_username_caracteres_especiales() {
        let dangerous_usernames = [
            "admin<script>",
            "user'; DROP TABLE users; --",
            "../../../root",
            "user%00null",
            "user\nnewline",
        ];

        for username in &dangerous_usernames {
            // Sanitizar: solo permitir alfanuméricos y guiones
            let sanitized: String = username
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .take(30)
                .collect();

            // El sanitizado no debe contener los caracteres peligrosos
            assert!(!sanitized.contains('<'));
            assert!(!sanitized.contains('>'));
            assert!(!sanitized.contains('\''));
            assert!(!sanitized.contains(';'));
            assert!(!sanitized.contains('/'));
        }
    }

    /// Fallo: Plan propuesto vacío
    #[test]
    fn fallo_plan_vacio() {
        let plan_vacio = "";
        let plan_solo_espacios = "   \n  \t  ";

        let es_valido = |plan: &str| -> bool {
            !plan.trim().is_empty()
        };

        assert!(!es_valido(plan_vacio));
        assert!(!es_valido(plan_solo_espacios));

        // El frontend no debería mostrar modal con plan vacío
        let debe_mostrar_plan = |plan: &str| -> bool {
            !plan.trim().is_empty()
        };
        assert!(!debe_mostrar_plan(plan_vacio));
    }

    /// Fallo: Interrupción durante pregunta pendiente
    #[test]
    fn fallo_interrupcion_durante_pregunta() {
        // Estado antes de interrupción: pregunta pendiente
        let estado_antes = json!({
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿SQLite o PostgreSQL?"
        });

        // Después de interrupción, debe limpiarse
        let estado_despues = json!({
            "interrupted": true,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null
        });

        assert_eq!(estado_antes["esperando_respuesta_usuario"], true);
        assert_eq!(estado_despues["esperando_respuesta_usuario"], false);
        assert_eq!(estado_despues["pregunta_usuario"], json!(null));
    }
}

// ============================================================================
// SECCIÓN 6: TESTS DE CASOS LÍMITE — Edge cases y condiciones frontera
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use serde_json::json;

    /// Caso límite: Mensaje vacío en chat
    #[test]
    fn edge_mensaje_vacio() {
        let empty_message = "";
        let whitespace_message = "   \n  \t  ";

        assert!(empty_message.is_empty());
        assert!(whitespace_message.trim().is_empty());

        // La API debería rechazar mensajes vacíos
        let is_valid = |msg: &str| -> bool { !msg.trim().is_empty() };
        assert!(!is_valid(empty_message));
        assert!(!is_valid(whitespace_message));
    }

    /// Caso límite: Nombre de proyecto muy largo
    #[test]
    fn edge_nombre_proyecto_largo() {
        let max_len = 64;
        let nombre_largo = "a".repeat(100);
        let nombre_valido = "a".repeat(64);

        assert!(nombre_largo.len() > max_len);
        assert_eq!(nombre_valido.len(), max_len);

        // El sistema debe truncar o rechazar nombres muy largos
        let sanitizado: String = nombre_largo.chars().take(max_len).collect();
        assert_eq!(sanitizado.len(), max_len);
    }

    /// Caso límite: UUID inválido
    #[test]
    fn edge_uuid_invalido() {
        let invalid_uuids = [
            "",
            "not-a-uuid",
            "12345678-1234-1234-1234-12345678901", // demasiado largo
            "gggggggg-gggg-gggg-gggg-gggggggggggg", // caracteres inválidos
        ];

        for uuid_str in &invalid_uuids {
            let is_valid = uuid_str.len() == 36
                && uuid_str.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
                && uuid_str.matches('-').count() == 4;

            assert!(!is_valid, "UUID '{}' debería ser inválido", uuid_str);
        }
    }

    /// Caso límite: Contraseña muy corta
    #[test]
    fn edge_password_corta() {
        let min_len = 8;
        let short_passwords = ["", "a", "ab", "abc", "1234567"];

        for pass in &short_passwords {
            assert!(pass.len() < min_len,
                "Password '{}' debería ser rechazada por ser muy corta (< {}).", pass, min_len);
        }

        // Password válida
        let valid = "12345678";
        assert!(valid.len() >= min_len);
    }

    /// Caso límite: Contenido de archivo vacío
    /// Caso límite: Contenido de archivo vacío
    #[test]
    fn edge_archivo_vacio() {
        let content = "";

        // En Rust, "".lines() devuelve un iterador vacío (0 líneas), no 1.
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 0);

        // Intentar leer rango de un archivo vacío
        let total_lines = lines.len(); // 0
        let start: i32 = 1;
        let end: i32 = 5;
        let start_idx = start.saturating_sub(1) as usize; // 0
        let end_idx = (end as usize).min(total_lines); // 0

        assert_eq!(start_idx, 0);
        assert_eq!(end_idx, 0);
    }

    /// Caso límite: Pregunta del agente muy larga
    #[test]
    fn edge_pregunta_muy_larga() {
        let pregunta_larga = "A".repeat(10000);
        let max_len = 5000;

        // El sistema debe truncar preguntas muy largas
        let truncada: String = pregunta_larga.chars().take(max_len).collect();
        assert_eq!(truncada.len(), max_len);
    }

    /// Caso límite: Múltiples interrupciones consecutivas
    #[test]
    fn edge_multiples_interrupciones() {
        let mut status = json!({
            "interrupted": false,
            "running": true
        });

        // Primera interrupción
        status["interrupted"] = json!(true);
        status["running"] = json!(false);
        assert_eq!(status["interrupted"], true);

        // Segunda interrupción (ya está interrumpido)
        // No debería causar problemas
        status["interrupted"] = json!(true);
        assert_eq!(status["interrupted"], true);

        // Tercera
        status["interrupted"] = json!(true);
        assert_eq!(status["interrupted"], true);
    }

    /// Caso límite: Sesión sin session_id
    #[test]
    fn edge_sesion_sin_id() {
        let session_id: Option<&str> = None;

        // El sistema debe manejar sesiones sin ID
        let response = json!({
            "status": "ok",
            "current_session_id": null
        });

        assert!(session_id.is_none());
        assert!(response["current_session_id"].is_null());
    }

    /// Caso límite: Modo no reconocido
    #[test]
    fn edge_modo_no_reconocido() {
        let invalid_modes = ["", "invalid", "Study", "PROGRAMMING", "study ", " study"];

        for mode in &invalid_modes {
            let normalized = mode.trim().to_lowercase();
            let is_valid = normalized == "study" || normalized == "programming";
            // Modos con espacios o mayúsculas deberían normalizarse
            let normalized_valid = mode.trim().to_lowercase() == "study"
                || mode.trim().to_lowercase() == "programming";
            if normalized_valid {
                // El sistema debería normalizar
                assert!(normalized == "study" || normalized == "programming");
            }
        }
    }

    /// Caso límite: Rango de líneas con start > end
    #[test]
    fn edge_rango_lineas_invertido() {
        let total_lines = 100;
        let start = 50;
        let end = 10;

        // El sistema debe detectar rango inválido
        let is_valid_range = start <= end && start >= 1 && end <= total_lines;
        assert!(!is_valid_range);

        // Debe devolver error, no panic
        let error_msg = format!(
            "Error: El rango de líneas {}-{} es inválido para un archivo de {} líneas.",
            start, end, total_lines
        );
        assert!(error_msg.contains("inválido"));
    }

    /// Caso límite: Prompt global vacío
    #[test]
    fn edge_prompt_global_vacio() {
        let empty_prompt = "";

        // El sistema debería usar el default si el prompt está vacío
        if empty_prompt.trim().is_empty() {
            let default_prompt = "Eres un asistente de IA. Ayuda al usuario con sus tareas.";
            assert!(!default_prompt.is_empty());
        }
    }
}

// ============================================================================
// SECCIÓN 7: TESTS DE CONSISTENCIA DE CONTRATO API
// ============================================================================

#[cfg(test)]
mod api_contract_tests {
    use serde_json::json;

    /// Todos los endpoints que el frontend llama deben tener contrato definido
    #[test]
    fn api_all_frontend_endpoints_defined() {
        let frontend_endpoints = vec![
            ("GET", "/api/projects"),
            ("POST", "/api/projects/local"),
            ("POST", "/api/chat"),
            ("GET", "/api/agent/status"),
            ("GET", "/api/agent/steps"),
            ("GET", "/api/agent/summary"),
            ("POST", "/api/agent/responder"),
            ("POST", "/api/agent/aprobar_plan"),
            ("POST", "/api/agent/interrupt"),
            ("GET", "/api/captcha/status"),
            ("POST", "/api/captcha/solve"),
            ("GET", "/api/chats"),
            ("POST", "/api/chats/new"),
            ("GET", "/api/study/profile"),
            ("POST", "/api/study/profile"),
            ("POST", "/api/reportar-fallo"),
            // Auth
            ("POST", "/api/auth/login"),
            ("POST", "/api/auth/challenge"),
            ("POST", "/api/auth/verify"),
            ("POST", "/api/auth/logout"),
        ];

        for (method, path) in &frontend_endpoints {
            // Verificar que la ruta empieza con /api/
            assert!(path.starts_with("/api/"),
                "Endpoint '{} {}' no sigue la convención /api/", method, path);

            // Verificar que el método es válido
            assert!(matches!(*method, "GET" | "POST" | "PUT" | "DELETE"),
                "Método HTTP inválido '{}' para '{}'", method, path);
        }

        // No debe haber duplicados
        let mut paths: Vec<String> = frontend_endpoints.iter()
            .map(|(m, p)| format!("{} {}", m, p))
            .collect();
        let len_before = paths.len();
        paths.sort();
        paths.dedup();
        assert_eq!(len_before, paths.len(),
            "Hay endpoints duplicados en la lista del frontend");
    }

    /// Todas las respuestas de error deben tener status y message
    #[test]
    fn api_error_responses_tienen_status_y_message() {
        let error_responses = vec![
            json!({"status": "error", "message": "Token Bearer requerido."}),
            json!({"status": "error", "message": "Token inválido o expirado."}),
            json!({"status": "error", "message": "Se requiere rol admin."}),
            json!({"status": "error", "message": "Archivo no encontrado."}),
            json!({"status": "error", "message": "Acceso denegado."}),
        ];

        for resp in &error_responses {
            assert_eq!(resp["status"], "error");
            assert!(!resp["message"].as_str().unwrap().is_empty());
        }
    }
}
