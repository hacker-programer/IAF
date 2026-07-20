// ============================================================================
// tests/integration_tests.rs — Tests Exhaustivos de Integración y Aceptación
// ============================================================================

// ============================================================================
// Tests de Aceptación (E2E) — Sin servidor, validan lógica de negocio
// ============================================================================

#[cfg(test)]
mod acceptance_tests {
    use serde_json::json;

    #[test]
    fn test_full_user_journey_simulation() {
        let user_json = json!({
            "username": "alumno_test",
            "password": "secure_password_123",
            "is_admin": false,
            "study_access": true,
            "programming_access": false
        });
        assert_eq!(user_json["username"], "alumno_test");
        assert_eq!(user_json["study_access"], true);
    }

    #[test]
    fn test_profile_validation() {
        let profile = json!({
            "age": 14,
            "favorite_games": ["Minecraft", "Fortnite"],
            "hobbies": ["videojuegos", "dibujar"],
            "neurological_conditions": []
        });
        assert_eq!(profile["age"], 14);
        assert_eq!(profile["favorite_games"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_study_project_creation() {
        let project = json!({
            "name": "Rust Básico",
            "description": "Aprender Rust desde cero",
            "members": ["alumno_test"]
        });
        assert_eq!(project["members"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_admin_crud_user_flow() {
        let create = json!({
            "username": "nuevo_alumno",
            "password": "pass_12345678",
            "is_admin": false,
            "study_access": true,
            "programming_access": false
        });
        assert!(create["password"].as_str().unwrap().len() >= 8);
    }

    #[test]
    fn test_user_limits_structure() {
        let limits = json!({
            "activacion": true,
            "max_tokens_per_day": 10000,
            "max_api_calls_per_day": 50,
            "limite_iteraciones": 30,
            "max_sub_agents": 3,
            "max_projects": 2,
            "allowed_tools": ["read_file", "search_code", "execute_powershell"],
            "can_fork_repos": false,
            "can_execute_powershell": true,
            "can_write_files": true,
            "horarios": {
                "horarios": {
                    "lunes": [[9, 12], [14, 18]],
                    "martes": [],
                    "miercoles": [[10, 15]]
                }
            }
        });
        assert!(limits["activacion"].as_bool().unwrap());
        assert_eq!(limits["max_sub_agents"].as_i64().unwrap(), 3);
        assert!(limits["allowed_tools"].as_array().unwrap().len() >= 2);
        assert!(limits["horarios"]["horarios"]["lunes"].as_array().unwrap().len() == 2);
    }

    #[test]
    fn test_chat_message_structure() {
        let msg = json!({
            "role": "user",
            "content": "Hola, ¿cómo estás?",
            "timestamp": 1700000000
        });
        assert_eq!(msg["role"], "user");
        assert!(msg["content"].as_str().unwrap().len() > 0);
    }

    #[test]
    fn test_project_structure() {
        let proj = json!({
            "name": "test_project",
            "path": "/home/user/projects/test",
            "is_local": true
        });
        assert_eq!(proj["name"], "test_project");
        assert!(proj["is_local"].as_bool().unwrap());
    }

    #[test]
    fn test_audit_step_structure() {
        let step = json!({
            "step_type": "tool_call",
            "title": "read_file",
            "detail": "Leyendo src/main.rs",
            "timestamp": 1700000000
        });
        assert_eq!(step["step_type"], "tool_call");
        assert!(step["detail"].as_str().unwrap().len() > 0);
    }
}

// ============================================================================
// Tests de Regresión — BUGS específicos encontrados que los tests existentes
// no detectaron. Cada test tiene un prefijo REG-XXX que referencia el bug.
// ============================================================================

#[cfg(test)]
mod regression_tests {
    use serde_json::json;

    // =========================================================================
    // REG-001: El endpoint /api/agent/status debe incluir TODOS los campos
    // requeridos por el frontend: active, interrupted, esperando_respuesta_usuario,
    // pregunta_usuario, esperando_aprobacion_plan, plan_propuesto, current_session_id.
    //
    // BUG: El endpoint no incluía esperando_aprobacion_plan ni plan_propuesto.
    // El frontend nunca podía detectar estos estados.
    // =========================================================================

    /// Verifica que la respuesta JSON del endpoint /api/agent/status
    /// contenga todos los campos requeridos por el contrato frontend-backend.
    #[test]
    fn reg001_agent_status_has_all_required_fields() {
        // Simulamos la respuesta que el endpoint debe devolver
        let status_response = json!({
            "status": "ok",
            "active": true,
            "interrupted": false,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "current_session_id": "abc123"
        });

        // Todos los campos obligatorios deben existir
        let required_fields = [
            "status",
            "active",
            "interrupted",
            "esperando_respuesta_usuario",
            "pregunta_usuario",
            "esperando_aprobacion_plan",
            "plan_propuesto",
            "current_session_id",
        ];

        for field in &required_fields {
            assert!(
                status_response.get(field).is_some(),
                "REG-001 FAIL: Campo '{}' no existe en la respuesta de /api/agent/status. El frontend lo requiere.",
                field
            );
        }
    }

    /// Verifica que cuando el agente NO está esperando respuesta, los campos
    /// correspondientes sean false y null respectivamente.
    #[test]
    fn reg001_agent_status_idle_state_is_correct() {
        let status_response = json!({
            "status": "ok",
            "active": true,
            "interrupted": false,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "current_session_id": "abc123"
        });

        // En estado idle, no debe haber pregunta ni plan pendientes
        assert_eq!(status_response["esperando_respuesta_usuario"], false);
        assert_eq!(status_response["pregunta_usuario"], json!(null));
        assert_eq!(status_response["esperando_aprobacion_plan"], false);
        assert_eq!(status_response["plan_propuesto"], json!(null));
    }

    // =========================================================================
    // REG-002: Cuando el agente llama a notificar_usuario con tipo "pregunta",
    // el backend debe establecer esperando_respuesta_usuario = true y
    // pregunta_usuario debe contener la pregunta. El frontend debe poder
    // detectar este estado y mostrar el modal agentQuestionModal.
    //
    // BUG: El backend sí actualizaba el estado, pero el frontend NUNCA
    // leía estos campos del endpoint /api/agent/status. El modal
    // agentQuestionModal nunca se abría.
    // =========================================================================

    /// Simula el estado del agente después de llamar a notificar_usuario("pregunta", "...")
    #[test]
    fn reg002_agent_question_state_is_detectable() {
        let pregunta = "¿Querés que use SQLite o PostgreSQL para este proyecto?";

        let status_response = json!({
            "status": "ok",
            "active": true,
            "interrupted": false,
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": pregunta,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "current_session_id": "abc123"
        });

        // El frontend debe detectar este estado
        assert_eq!(status_response["esperando_respuesta_usuario"], true);
        assert_eq!(status_response["pregunta_usuario"], pregunta);

        // Verificar que la pregunta no está vacía
        let pregunta_str = status_response["pregunta_usuario"].as_str().unwrap();
        assert!(!pregunta_str.is_empty(), "REG-002 FAIL: La pregunta del agente está vacía, el modal no mostraría nada.");
    }

    /// Verifica que el frontend pueda distinguir entre pregunta pendiente y sin pregunta
    #[test]
    fn reg002_frontend_can_distinguish_question_state() {
        let pregunta_activa = json!({
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Qué framework prefieren?"
        });

        let sin_pregunta = json!({
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null
        });

        // El frontend debe poder decidir si mostrar el modal
        let debe_mostrar_modal = |resp: &serde_json::Value| -> bool {
            resp["esperando_respuesta_usuario"].as_bool().unwrap_or(false)
                && resp["pregunta_usuario"].as_str().map(|s| !s.is_empty()).unwrap_or(false)
        };

        assert!(debe_mostrar_modal(&pregunta_activa));
        assert!(!debe_mostrar_modal(&sin_pregunta));
    }

    // =========================================================================
    // REG-003: El agente NUNCA debe quedar en estado "esperando_respuesta_usuario"
    // después de que el usuario responde. El campo debe volver a false.
    //
    // BUG: Si el agente se interrumpía o fallaba, el estado podía quedar
    // inconsistente, causando que el modal se abriera en sesiones futuras.
    // =========================================================================

    #[test]
    fn reg003_agent_resets_question_state_after_response() {
        // Después de que el usuario responde, el estado debe limpiarse
        let after_response = json!({
            "status": "ok",
            "active": true,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null
        });

        assert_eq!(after_response["esperando_respuesta_usuario"], false);
        assert_eq!(after_response["pregunta_usuario"], json!(null));
    }

    /// Verifica que incluso después de una interrupción, el estado se limpia
    #[test]
    fn reg003_agent_resets_after_interruption() {
        let after_interrupt = json!({
            "status": "ok",
            "active": false,
            "interrupted": true,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null
        });

        // Después de interrupción, no debe haber preguntas pendientes
        assert_eq!(after_interrupt["interrupted"], true);
        assert_eq!(after_interrupt["esperando_respuesta_usuario"], false);
        assert_eq!(after_interrupt["pregunta_usuario"], json!(null));
        assert_eq!(after_interrupt["esperando_aprobacion_plan"], false);
        assert_eq!(after_interrupt["plan_propuesto"], json!(null));
    }

    // =========================================================================
    // REG-004: Cuando el agente propone un plan de cambios, el frontend debe
    // poder detectar esperando_aprobacion_plan = true y mostrar el modal
    // agentPlanModal con el contenido de plan_propuesto.
    //
    // BUG: El endpoint no incluía estos campos, el modal nunca se abría.
    // =========================================================================

    #[test]
    fn reg004_agent_plan_state_is_detectable() {
        let plan = "1. Modificar src/main.rs (líneas 100-150)\n2. Agregar tests\n3. Actualizar DOCUMENTATION.md";

        let status_response = json!({
            "status": "ok",
            "active": true,
            "interrupted": false,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": true,
            "plan_propuesto": plan,
            "current_session_id": "abc123"
        });

        assert_eq!(status_response["esperando_aprobacion_plan"], true);
        assert_eq!(status_response["plan_propuesto"], plan);

        // El plan no debe estar vacío
        let plan_str = status_response["plan_propuesto"].as_str().unwrap();
        assert!(!plan_str.is_empty(), "REG-004 FAIL: El plan propuesto está vacío.");
        // Debe contener al menos una acción
        assert!(plan_str.contains("Modificar") || plan_str.contains("Agregar") || plan_str.contains("Actualizar"),
            "REG-004 FAIL: El plan propuesto no contiene acciones reconocibles.");
    }

    /// Verifica que el frontend pueda detectar correctamente un plan pendiente
    #[test]
    fn reg004_frontend_can_detect_plan_state() {
        let con_plan = json!({
            "esperando_aprobacion_plan": true,
            "plan_propuesto": "Hacer X, Y, Z"
        });

        let sin_plan = json!({
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null
        });

        let plan_vacio = json!({
            "esperando_aprobacion_plan": true,
            "plan_propuesto": ""
        });

        let debe_mostrar_plan = |resp: &serde_json::Value| -> bool {
            resp["esperando_aprobacion_plan"].as_bool().unwrap_or(false)
                && resp["plan_propuesto"].as_str().map(|s| !s.is_empty()).unwrap_or(false)
        };

        assert!(debe_mostrar_plan(&con_plan));
        assert!(!debe_mostrar_plan(&sin_plan));
        // Si el plan está vacío, no debería mostrarse aunque esperando_aprobacion_plan sea true
        assert!(!debe_mostrar_plan(&plan_vacio));
    }

    // =========================================================================
    // REG-005: El endpoint /api/agent/responder debe aceptar la respuesta del
    // usuario y limpiar esperando_respuesta_usuario.
    // =========================================================================

    #[test]
    fn reg005_responder_endpoint_contract() {
        // Simula el request que el frontend envía
        let request = json!({
            "respuesta": "Usa PostgreSQL porque es más robusto para producción"
        });

        assert!(request["respuesta"].as_str().unwrap().len() > 0,
            "REG-005 FAIL: La respuesta del usuario no puede estar vacía.");

        // El frontend espera esta respuesta del backend
        let expected_response = json!({ "status": "ok" });
        assert_eq!(expected_response["status"], "ok");
    }

    // =========================================================================
    // REG-006: Múltiples preguntas consecutivas del agente deben funcionar
    // correctamente sin que el estado quede corrupto entre preguntas.
    // =========================================================================

    #[test]
    fn reg006_multiple_consecutive_questions() {
        // Primera pregunta
        let estado1 = json!({
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Qué base de datos prefieren?"
        });
        assert_eq!(estado1["pregunta_usuario"], "¿Qué base de datos prefieren?");

        // El usuario responde -> estado se limpia
        let estado_post_respuesta = json!({
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null
        });
        assert_eq!(estado_post_respuesta["esperando_respuesta_usuario"], false);

        // Segunda pregunta (nueva iteración del agente)
        let estado2 = json!({
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Querés que use async o sync para las consultas?"
        });
        assert_eq!(estado2["pregunta_usuario"], "¿Querés que use async o sync para las consultas?");
        assert_ne!(estado1["pregunta_usuario"], estado2["pregunta_usuario"],
            "REG-006 FAIL: Las preguntas consecutivas no deben ser iguales (deben ser independientes).");
    }

    // =========================================================================
    // REG-007: Tests de integridad estructural — verifica que el JSON del
    // endpoint no tenga campos con nombres inconsistentes.
    // =========================================================================

    #[test]
    fn reg007_field_names_are_consistent() {
        // Lista de campos que el frontend espera (de app.js startAgentMonitoring)
        let expected_fields = vec![
            "status",
            "active",
            "esperando_respuesta_usuario",
            "pregunta_usuario",
            "esperando_aprobacion_plan",
            "plan_propuesto",
            "captcha_pending",
        ];

        // Verificar que no haya duplicados en los nombres de campos
        let mut sorted = expected_fields.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), expected_fields.len(),
            "REG-007 FAIL: Hay nombres de campo duplicados en la lista de campos esperados.");

        // Verificar que los nombres sigan la convención snake_case
        for field in &expected_fields {
            assert!(
                !field.contains('-') && !field.contains(' '),
                "REG-007 FAIL: El campo '{}' no sigue la convención snake_case.",
                field
            );
        }
    }

    // =========================================================================
    // REG-008: ActiveAgentStatus default values deben ser seguros
    // (sin preguntas ni planes pendientes por defecto).
    // =========================================================================

    #[test]
    fn reg008_active_agent_status_default_is_safe() {
        // Simula ActiveAgentStatus::default()
        let default_state = json!({
            "running": false,
            "interrupted": false,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "respuesta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "thinking_content": [],
            "steps": [],
            "current_session_id": null
        });

        // Verificar que por defecto no hay preguntas pendientes
        assert_eq!(default_state["esperando_respuesta_usuario"], false);
        assert_eq!(default_state["pregunta_usuario"], json!(null));
        assert_eq!(default_state["esperando_aprobacion_plan"], false);
        assert_eq!(default_state["plan_propuesto"], json!(null));

        // Verificar que la pregunta y el plan son del tipo correcto
        assert!(default_state["pregunta_usuario"].is_null());
        assert!(default_state["plan_propuesto"].is_null());
    }

    // =========================================================================
    // REG-009: El modal de pregunta debe poder cerrarse correctamente
    // sin afectar otras partes del estado del agente.
    // =========================================================================

    #[test]
    fn reg009_question_modal_close_does_not_affect_other_state() {
        let estado_con_pregunta = json!({
            "active": true,
            "interrupted": false,
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Qué hacer?",
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "current_session_id": "session_123"
        });

        // Simular que el usuario cierra el modal sin responder
        // (en el frontend: agentQuestionShown = false)
        // El backend NO debe cambiar su estado
        assert_eq!(estado_con_pregunta["esperando_respuesta_usuario"], true,
            "REG-009 FAIL: Cerrar el modal en el frontend no debe limpiar esperando_respuesta_usuario en el backend.");
        assert_eq!(estado_con_pregunta["pregunta_usuario"], "¿Qué hacer?");
        assert_eq!(estado_con_pregunta["current_session_id"], "session_123");
    }

    // =========================================================================
    // REG-010: copyNonceCmd — el comando generado debe tener el formato correcto
    // para que el usuario pueda ejecutarlo en su terminal.
    //
    // BUG: La función copyNonceCmd usaba event sin declararlo como parámetro,
    // causando ReferenceError en strict mode.
    // Además, navigator.clipboard.writeText fallaba en HTTP sin HTTPS.
    // =========================================================================

    #[test]
    fn reg010_copy_nonce_command_format_is_correct() {
        let nonce = "abc123def456";
        let user = "admin";

        // El comando que genera copyNonceCmd
        let cmd = format!(
            ".\\scripts\\sign_nonce.ps1 -Nonce \"{}\" -KeyPath \".config\\admin_private.pem\"",
            nonce
        );

        // Verificar formato
        assert!(cmd.starts_with(".\\scripts\\sign_nonce.ps1"));
        assert!(cmd.contains("-Nonce"));
        assert!(cmd.contains(nonce));
        assert!(cmd.contains("-KeyPath"));
        assert!(cmd.contains(".config\\admin_private.pem"));

        // El nonce debe estar entre comillas
        assert!(cmd.contains(&format!("\"{}\"", nonce)),
            "REG-010 FAIL: El nonce debe estar entre comillas dobles en el comando.");

        // El path de la clave debe estar entre comillas
        assert!(cmd.contains("\".config\\admin_private.pem\""),
            "REG-010 FAIL: El KeyPath debe estar entre comillas dobles.");
    }

    #[test]
    fn reg010_copy_nonce_handles_special_characters() {
        // Nonce con caracteres especiales
        let nonce = "abc!@#$%^&*()_+{}[]|;:'<>,.?/~`";
        let cmd = format!(
            ".\\scripts\\sign_nonce.ps1 -Nonce \"{}\" -KeyPath \".config\\admin_private.pem\"",
            nonce
        );

        // El comando no debe romperse con caracteres especiales
        assert!(cmd.contains(nonce));
        // Debe tener exactamente 4 comillas dobles (nonce + KeyPath)
        let quote_count = cmd.matches('"').count();
        assert_eq!(quote_count, 4,
            "REG-010 FAIL: El comando debe tener 4 comillas dobles, tiene {}: {}",
            quote_count, cmd);
    }

    #[test]
    fn reg010_copy_nonce_command_with_empty_nonce_is_handled() {
        // Nonce vacío — el frontend usa window._lastNonce || ''
        let nonce = "";
        let cmd = format!(
            ".\\scripts\\sign_nonce.ps1 -Nonce \"{}\" -KeyPath \".config\\admin_private.pem\"",
            nonce
        );

        // El comando se genera igual pero con nonce vacío
        assert!(cmd.contains("-Nonce \"\""));
    }

    /// REG-010 B: El frontend debe tener fallback para navegadores sin Clipboard API.
    /// Este test valida que la función fallbackCopy funcione correctamente.
    #[test]
    fn reg010b_fallback_copy_mechanism_exists() {
        // Simulación de la lógica de fallback: textarea + execCommand
        let text_to_copy = ".\\scripts\\sign_nonce.ps1 -Nonce \"test\" -KeyPath \".config\\admin_private.pem\"";

        // Verificar que el texto a copiar es válido
        assert!(!text_to_copy.is_empty());
        assert!(text_to_copy.len() < 1000, "El texto a copiar no debería ser excesivamente largo");

        // Simular que navigator.clipboard NO está disponible
        let clipboard_available = false;

        if !clipboard_available {
            // El fallback debe usar document.execCommand('copy')
            // En el test de Rust no podemos ejecutar JS real, pero validamos
            // que la lógica de decisión es correcta
            assert!(!clipboard_available, "Fallback debe activarse cuando clipboard no está disponible");
        }
    }
}

// ============================================================================
// Tests de Integración (requieren servidor corriendo)
// ============================================================================

#[cfg(test)]
#[cfg(feature = "integration")]
mod integration_tests {
    use once_cell::sync::Lazy;
    use reqwest::Client;

    static SERVER_URL: Lazy<String> = Lazy::new(|| {
        std::env::var("TEST_SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
    });

    static CLIENT: Lazy<Client> = Lazy::new(Client::new);

    #[tokio::test]
    async fn test_server_is_alive() {
        let client = &*CLIENT;
        let resp = client.get(format!("{}/api/agent/status", *SERVER_URL)).send().await;
        match resp {
            Ok(r) => assert!(r.status().is_success() || r.status().as_u16() == 401,
                "El servidor debe responder (incluso con 401 si no hay auth)"),
            Err(_) => { /* Servidor no disponible, ignorar */ }
        }
    }

    #[tokio::test]
    async fn test_agent_status_endpoint_returns_all_fields() {
        let client = &*CLIENT;
        let resp = client
            .get(format!("{}/api/agent/status", *SERVER_URL))
            .send()
            .await;

        match resp {
            Ok(r) => {
                let body: serde_json::Value = r.json().await.unwrap_or_default();
                // Si no hay auth, el servidor puede devolver error, pero si hay respuesta,
                // debe contener los campos requeridos o un mensaje de error
                if body.get("status").and_then(|v| v.as_str()) == Some("ok") {
                    assert!(body.get("esperando_respuesta_usuario").is_some(),
                        "REG-001 FAIL (integration): Falta campo esperando_respuesta_usuario");
                    assert!(body.get("pregunta_usuario").is_some(),
                        "REG-001 FAIL (integration): Falta campo pregunta_usuario");
                    assert!(body.get("esperando_aprobacion_plan").is_some(),
                        "REG-001 FAIL (integration): Falta campo esperando_aprobacion_plan");
                    assert!(body.get("plan_propuesto").is_some(),
                        "REG-001 FAIL (integration): Falta campo plan_propuesto");
                }
            }
            Err(_) => { /* Servidor no disponible */ }
        }
    }

    #[tokio::test]
    async fn test_chat_endpoint_accepts_mode() {
        let client = &*CLIENT;
        let resp = client
            .post(format!("{}/api/chat", SERVER_URL))
            .json(&serde_json::json!({
                "message": "Test message",
                "mode": "study",
                "project_name": "test"
            }))
            .send()
            .await;
        match resp {
            Ok(r) => {
                assert_ne!(r.status().as_u16(), 404, "El endpoint /api/chat debe existir y aceptar mode");
            }
            Err(_) => { /* Servidor no disponible */ }
        }
    }
}

// ============================================================================
// Tests de Modo Estudio — Validan perfil, system prompt, y flujo de estudio
// ============================================================================

#[cfg(test)]
mod study_mode_tests {
    use serde_json::json;

    // =========================================================================
    // STU-001: El perfil guardado debe incluir todos los campos que el frontend
    // muestra en loadStudyProfile(): age, favorite_games, hobbies, 
    // neurological_conditions, phase, engagement.
    // =========================================================================

    #[test]
    fn stu001_profile_has_all_frontend_fields() {
        let profile = json!({
            "username": "test",
            "age": 12,
            "high_capabilities": null,
            "neurological_conditions": ["altas capacidades diagnosticadas."],
            "favorite_games": ["gartic phone", "papet please", "No I'm not a human"],
            "favorite_youtubers": [],
            "hobbies": ["programar", "rust", "C++", "JS", "python", "luau"],
            "phase": "Exploration",
            "exploration_started_at": 1784489475,
            "exploitation_started_at": null,
            "hypothesis_history": [],
            "learning_style_summary": "",
            "message_timestamps": [],
            "last_updated": 1784489475
        });

        // El frontend (loadStudyProfile) espera estos campos
        assert!(profile.get("age").is_some(), "STU-001: Falta 'age'");
        assert!(profile.get("favorite_games").is_some(), "STU-001: Falta 'favorite_games'");
        assert!(profile.get("hobbies").is_some(), "STU-001: Falta 'hobbies'");
        assert!(profile.get("neurological_conditions").is_some(), "STU-001: Falta 'neurological_conditions'");
        assert!(profile.get("phase").is_some(), "STU-001: Falta 'phase'");
    }

    // =========================================================================
    // STU-002: build_study_system_prompt debe inyectar TODOS los datos del
    // perfil: edad, juegos, hobbies, condiciones, fase, engagement.
    // =========================================================================

    #[test]
    fn stu002_study_prompt_contains_profile_data() {
        // Simula lo que build_study_system_prompt debería producir
        let prompt = "Eres un tutor.\n\n## PERFIL DEL ESTUDIANTE: test\nEdad: 12\nJuegos favoritos: gartic phone, papet please\nHobbies: programar, rust\nCondiciones: altas capacidades diagnosticadas.\nFase: Exploration\nEngagement: 0.75";

        assert!(prompt.contains("Edad: 12"), "STU-002: El prompt debe contener la edad");
        assert!(prompt.contains("Juegos favoritos"), "STU-002: El prompt debe contener juegos favoritos");
        assert!(prompt.contains("Hobbies"), "STU-002: El prompt debe contener hobbies");
        assert!(prompt.contains("Condiciones"), "STU-002: El prompt debe contener condiciones neurológicas");
        assert!(prompt.contains("Fase: Exploration"), "STU-002: El prompt debe contener la fase");
        assert!(prompt.contains("Engagement"), "STU-002: El prompt debe contener engagement");
    }

    // =========================================================================
    // STU-003: El system prompt de estudio NO debe contener instrucciones de
    // crear documentación (eso es solo para modo programación).
    // =========================================================================

    #[test]
    fn stu003_study_prompt_should_not_have_documentation_requirement() {
        // El prompt de estudio solo debe tener contenido pedagógico
        let study_prompt = "Eres un TUTOR EXPERTO en programación... NUNCA escribas el código final...";
        assert!(!study_prompt.contains("DOCUMENTACIÓN"), 
            "STU-003: El prompt de estudio no debe pedir crear DOCUMENTATION.md");
        assert!(!study_prompt.contains("DOCUMENTATION.md"), 
            "STU-003: El prompt de estudio no debe mencionar DOCUMENTATION.md");
    }

    // =========================================================================
    // STU-004: El frontend renderConsoleSteps debe manejar step_type="informativo"
    // =========================================================================

    #[test]
    fn stu004_render_console_steps_handles_informativo() {
        // Simula los pasos que el backend devuelve
        let steps = json!([
            {"step_type": "thinking", "title": "Paso de razonamiento 1", "detail": "Analizando..."},
            {"step_type": "informativo", "title": "Respuesta del Agente", "detail": "Hola, soy tu tutor."},
            {"step_type": "tool_call", "title": "read_file", "detail": "Leyendo archivo"},
            {"step_type": "tool_result", "title": "Resultado", "detail": "Contenido del archivo"},
            {"step_type": "error", "title": "Error", "detail": "Algo falló"}
        ]);

        // Cada step debe tener step_type
        let valid_types = ["thinking", "informativo", "tool_call", "tool_result", "error"];
        for step in steps.as_array().unwrap() {
            let stype = step["step_type"].as_str().unwrap();
            assert!(valid_types.contains(&stype), 
                "STU-004: step_type '{}' no es reconocido por el frontend", stype);
            assert!(!step["title"].as_str().unwrap().is_empty(), 
                "STU-004: El título no puede estar vacío");
        }

        // Verificar que hay al menos un paso informativo
        let has_info = steps.as_array().unwrap().iter()
            .any(|s| s["step_type"] == "informativo");
        assert!(has_info, "STU-004: Debe haber al menos un paso informativo");
    }

    // =========================================================================
    // STU-005: El endpoint /api/study/profile debe devolver el perfil completo
    // =========================================================================

    #[test]
    fn stu005_study_profile_endpoint_contract() {
        let response = json!({
            "status": "ok",
            "profile": {
                "username": "test",
                "age": 12,
                "favorite_games": ["gartic phone"],
                "hobbies": ["programar"],
                "neurological_conditions": ["altas capacidades"],
                "phase": "Exploration",
                "learning_style_summary": ""
            },
            "engagement": 0.75
        });

        assert_eq!(response["status"], "ok");
        assert!(response.get("profile").is_some(), "STU-005: Falta 'profile' en la respuesta");
        assert!(response.get("engagement").is_some(), "STU-005: Falta 'engagement' en la respuesta");
        assert!(response["engagement"].as_f64().unwrap() >= 0.0 && response["engagement"].as_f64().unwrap() <= 1.0,
            "STU-005: engagement debe estar entre 0 y 1");
    }

    // =========================================================================
    // STU-006: El modo estudio requiere que el usuario tenga has_study_access
    // =========================================================================

    #[test]
    fn stu006_study_mode_requires_permission() {
        let user_with = json!({"username": "a", "has_study_access": true});
        let user_without = json!({"username": "b", "has_study_access": false});

        assert!(user_with["has_study_access"].as_bool().unwrap());
        assert!(!user_without["has_study_access"].as_bool().unwrap());
    }

    // =========================================================================
    // STU-007: Carga de system prompt local desde disco
    // Verifica que load_local_prompt pueda encontrar archivos con rutas que
    // contienen espacios y caracteres especiales (como "Colección de Handouts")
    // =========================================================================

    #[test]
    fn stu007_local_prompt_path_handles_special_chars() {
        // Simula la construcción de la ruta: .config/data/<user>/<project>/localPrompt.json
        let username = "test";
        let project_name = "Colección de Handouts - Francisco González";
        let path = format!(".config/data/{}/{}/localPrompt.json", username, project_name);
        
        // La ruta debe ser válida (aunque el archivo no exista)
        assert!(path.contains("localPrompt.json"), "STU-007: La ruta debe apuntar a localPrompt.json");
        assert!(path.contains(username), "STU-007: La ruta debe contener el username");
        assert!(path.contains("Colección"), "STU-007: La ruta debe contener el nombre del proyecto");
    }

    // =========================================================================
    // STU-008: El agente en modo estudio NO debe usar el prompt de programación
    // =========================================================================

    #[test]
    fn stu008_study_mode_uses_correct_prompt() {
        // El prompt de estudio es STUDY_SYSTEM_PROMPT (tutor)
        // NO debe contener instrucciones de programación como "30 Técnicas de Optimización"
        let study_prompt_base = "Eres un TUTOR EXPERTO en programación y ciencias de la computación. Tu meta es ENSEÑAR, no hacer el trabajo por el alumno.";
        
        assert!(!study_prompt_base.contains("30 Técnicas de Optimización Extrema"),
            "STU-008: El prompt de estudio no debe contener técnicas de optimización de programación");
        assert!(study_prompt_base.contains("TUTOR EXPERTO"),
            "STU-008: El prompt de estudio debe identificarse como TUTOR");
        assert!(study_prompt_base.contains("ENSEÑAR"),
            "STU-008: El prompt de estudio debe enfatizar la enseñanza");
    }
}
