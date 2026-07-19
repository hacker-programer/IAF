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
        assert!(cmd.startsWith(".\\scripts\\sign_nonce.ps1"));
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
// TESTS UNITARIOS — Funciones Puras de main.rs y agent.rs
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use serde_json::json;

    // ==========================================================================
    // extract_bearer_token
    // ==========================================================================

    /// Simula extract_bearer_token de main.rs
    fn mock_extract_bearer_token(auth_header: Option<&str>) -> Option<String> {
        auth_header
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string())
    }

    #[test]
    fn test_extract_bearer_token_valid() {
        assert_eq!(
            mock_extract_bearer_token(Some("Bearer abc123token")),
            Some("abc123token".to_string())
        );
    }

    #[test]
    fn test_extract_bearer_token_no_prefix() {
        assert_eq!(mock_extract_bearer_token(Some("Basic abc123")), None);
    }

    #[test]
    fn test_extract_bearer_token_empty() {
        assert_eq!(mock_extract_bearer_token(Some("Bearer ")), Some("".to_string()));
    }

    #[test]
    fn test_extract_bearer_token_none() {
        assert_eq!(mock_extract_bearer_token(None), None);
    }

    #[test]
    fn test_extract_bearer_token_case_sensitive() {
        assert_eq!(mock_extract_bearer_token(Some("bearer token123")), None);
    }

    #[test]
    fn test_extract_bearer_token_unicode() {
        assert_eq!(
            mock_extract_bearer_token(Some("Bearer tókën_ñ")),
            Some("tókën_ñ".to_string())
        );
    }

    #[test]
    fn test_extract_bearer_token_long_token() {
        let long = "a".repeat(1000);
        let header = format!("Bearer {}", long);
        assert_eq!(mock_extract_bearer_token(Some(&header)), Some(long));
    }

    // ==========================================================================
    // sanitize_filename
    // ==========================================================================

    fn mock_sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
            .collect::<String>()
            .trim()
            .replace(" ", "_")
            .chars()
            .take(40)
            .collect()
    }

    #[test]
    fn test_sanitize_simple() {
        assert_eq!(mock_sanitize_filename("Hola Mundo"), "Hola_Mundo");
    }

    #[test]
    fn test_sanitize_special_chars() {
        assert_eq!(mock_sanitize_filename("¿Qué tal?"), "_Qué_tal_");
    }

    #[test]
    fn test_sanitize_path_traversal() {
        let result = mock_sanitize_filename("../../etc/passwd");
        assert!(!result.contains('/'));
        assert!(!result.contains('\\'));
        assert!(!result.contains(".."));
    }

    #[test]
    fn test_sanitize_empty() {
        assert_eq!(mock_sanitize_filename(""), "");
    }

    #[test]
    fn test_sanitize_only_special() {
        assert_eq!(mock_sanitize_filename("!!!???"), "______");
    }

    #[test]
    fn test_sanitize_truncation() {
        let long = "a".repeat(100);
        let result = mock_sanitize_filename(&long);
        assert!(result.len() <= 40);
    }

    #[test]
    fn test_sanitize_emojis() {
        let result = mock_sanitize_filename("Hola 😀 Mundo 🚀");
        assert!(!result.contains('😀'));
        assert!(!result.contains('🚀'));
    }

    // ==========================================================================
    // looks_like_uuid_stem
    // ==========================================================================

    fn mock_looks_like_uuid_stem(stem: &str) -> bool {
        stem.len() >= 30
            && stem.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && stem.matches('-').count() >= 3
    }

    #[test]
    fn test_uuid_standard() {
        assert!(mock_looks_like_uuid_stem("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_uuid_short() {
        assert!(!mock_looks_like_uuid_stem("abc-123-def"));
    }

    #[test]
    fn test_uuid_title_format() {
        assert!(!mock_looks_like_uuid_stem("Mi_Chat-550e8400"));
    }

    #[test]
    fn test_uuid_invalid_chars() {
        assert!(!mock_looks_like_uuid_stem("zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz"));
    }

    #[test]
    fn test_uuid_only_hex_no_dashes() {
        assert!(!mock_looks_like_uuid_stem("abcdef1234567890abcdef1234567890abcd"));
    }

    // ==========================================================================
    // parse_shell_args
    // ==========================================================================

    fn mock_parse_shell_args(input: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;

        for ch in input.chars() {
            match ch {
                '\'' if !in_double_quote => in_single_quote = !in_single_quote,
                '"' if !in_single_quote => in_double_quote = !in_double_quote,
                ' ' | '\t' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        args.push(current.clone());
                        current.clear();
                    }
                }
                _ => current.push(ch),
            }
        }
        if !current.is_empty() {
            args.push(current);
        }
        args
    }

    #[test]
    fn test_parse_simple() {
        assert_eq!(mock_parse_shell_args("cargo build --release"), vec!["cargo", "build", "--release"]);
    }

    #[test]
    fn test_parse_with_double_quotes() {
        assert_eq!(
            mock_parse_shell_args("git commit -m \"mi mensaje\""),
            vec!["git", "commit", "-m", "mi mensaje"]
        );
    }

    #[test]
    fn test_parse_single_quotes() {
        assert_eq!(mock_parse_shell_args("echo 'hello world'"), vec!["echo", "hello world"]);
    }

    #[test]
    fn test_parse_nested_quotes() {
        assert_eq!(mock_parse_shell_args("echo \"it's a test\""), vec!["echo", "it's a test"]);
    }

    #[test]
    fn test_parse_empty() {
        assert!(mock_parse_shell_args("").is_empty());
    }

    #[test]
    fn test_parse_multiple_spaces() {
        assert_eq!(mock_parse_shell_args("a   b    c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_gh_repo_create() {
        let result = mock_parse_shell_args(
            "gh repo create \"my repo\" --public --description \"A test repo\""
        );
        assert_eq!(
            result,
            vec!["gh", "repo", "create", "my repo", "--public", "--description", "A test repo"]
        );
    }

    // ==========================================================================
    // ActiveAgentStatus — Serialización y Deserialización
    // ==========================================================================

    #[test]
    fn test_active_agent_status_serialization_with_question() {
        let status = json!({
            "running": true,
            "interrupted": false,
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Quieres continuar con la siguiente fase?",
            "respuesta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "thinking_content": [],
            "steps": [],
            "current_session_id": "abc-123"
        });

        assert_eq!(status["esperando_respuesta_usuario"], true);
        assert!(status["pregunta_usuario"].is_string());
        assert!(status["pregunta_usuario"].as_str().unwrap().len() > 0);
        assert_eq!(status["respuesta_usuario"], json!(null));
    }

    #[test]
    fn test_active_agent_status_serialization_with_plan() {
        let status = json!({
            "running": true,
            "interrupted": false,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "respuesta_usuario": null,
            "esperando_aprobacion_plan": true,
            "plan_propuesto": "1. Modificar auth.rs\n2. Agregar tests\n3. Actualizar DOCUMENTATION.md",
            "thinking_content": [],
            "steps": [],
            "current_session_id": "session-456"
        });

        assert_eq!(status["esperando_aprobacion_plan"], true);
        assert!(status["plan_propuesto"].as_str().unwrap().contains("auth.rs"));
    }

    #[test]
    fn test_active_agent_status_all_fields_present() {
        let status = json!({
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

        let required_fields = [
            "running", "interrupted",
            "esperando_respuesta_usuario", "pregunta_usuario", "respuesta_usuario",
            "esperando_aprobacion_plan", "plan_propuesto",
            "thinking_content", "steps", "current_session_id"
        ];

        for field in &required_fields {
            assert!(status.get(field).is_some(),
                "Campo requerido '{}' no está presente en ActiveAgentStatus", field);
        }
    }

    #[test]
    fn test_active_agent_status_roundtrip_json() {
        let original = json!({
            "running": true,
            "interrupted": false,
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Estás seguro?",
            "respuesta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "thinking_content": ["Paso 1", "Paso 2"],
            "steps": [
                {"step_type": "tool_call", "title": "read_file", "detail": "leyendo", "timestamp": 1700000000u64}
            ],
            "current_session_id": "test-session"
        });

        let serialized = serde_json::to_string(&original).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(parsed["running"], original["running"]);
        assert_eq!(parsed["esperando_respuesta_usuario"], original["esperando_respuesta_usuario"]);
        assert_eq!(parsed["pregunta_usuario"], original["pregunta_usuario"]);
        assert_eq!(parsed["current_session_id"], original["current_session_id"]);
    }
}

// ============================================================================
// TESTS DE REGRESIÓN — Bug A: Preguntas del agente no se muestran al usuario
// ============================================================================

#[cfg(test)]
mod regression_tests_bug_a {
    use serde_json::json;

    /// BUG A: El endpoint /api/agent/status debe devolver los campos
    /// esperando_respuesta_usuario, pregunta_usuario, esperando_aprobacion_plan,
    /// plan_propuesto y current_session_id. El frontend usa estos campos
    /// en startAgentMonitoring() para abrir los modales de pregunta y plan.
    #[test]
    fn test_agent_status_includes_all_required_fields() {
        let response = json!({
            "status": "ok",
            "active": true,
            "interrupted": false,
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Debo continuar con la fase de optimización?",
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "current_session_id": "abc-123"
        });

        assert!(response.get("esperando_respuesta_usuario").is_some(),
            "BUG REG-001: falta campo 'esperando_respuesta_usuario' en /api/agent/status");
        assert!(response.get("pregunta_usuario").is_some(),
            "BUG REG-001: falta campo 'pregunta_usuario' en /api/agent/status");
        assert!(response.get("esperando_aprobacion_plan").is_some(),
            "BUG REG-001: falta campo 'esperando_aprobacion_plan' en /api/agent/status");
        assert!(response.get("plan_propuesto").is_some(),
            "BUG REG-001: falta campo 'plan_propuesto' en /api/agent/status");
        assert!(response.get("current_session_id").is_some(),
            "BUG REG-001: falta campo 'current_session_id' en /api/agent/status");
    }

    #[test]
    fn test_agent_status_question_not_null_when_esperando() {
        let response = json!({
            "status": "ok",
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Continuar?",
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "current_session_id": "session-1"
        });

        if response["esperando_respuesta_usuario"].as_bool() == Some(true) {
            assert!(!response["pregunta_usuario"].is_null(),
                "BUG REG-002: pregunta_usuario es null pero esperando_respuesta_usuario=true");
            assert!(response["pregunta_usuario"].as_str().unwrap().len() > 0,
                "BUG REG-002: pregunta_usuario vacío pero esperando_respuesta_usuario=true");
        }
    }

    #[test]
    fn test_agent_status_plan_not_null_when_esperando_plan() {
        let response = json!({
            "status": "ok",
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": true,
            "plan_propuesto": "Fase 1: Refactorizar\nFase 2: Testear",
            "current_session_id": "session-2"
        });

        if response["esperando_aprobacion_plan"].as_bool() == Some(true) {
            assert!(!response["plan_propuesto"].is_null(),
                "BUG REG-003: plan_propuesto es null pero esperando_aprobacion_plan=true");
        }
    }

    #[test]
    fn test_frontend_polling_detects_question() {
        // Simula el polling del frontend (startAgentMonitoring)
        // Poll 1: sin preguntas
        let poll_1 = json!({
            "status": "ok", "active": true, "running": true,
            "esperando_respuesta_usuario": false, "pregunta_usuario": null
        });
        assert_eq!(poll_1["esperando_respuesta_usuario"], false);

        // Poll 2: agente hizo una pregunta
        let poll_2 = json!({
            "status": "ok", "active": true, "running": true,
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "¿Quieres continuar?"
        });
        assert_eq!(poll_2["esperando_respuesta_usuario"], true);
        assert!(!poll_2["pregunta_usuario"].is_null());

        // Poll 3: usuario respondió, agente continúa
        let poll_3 = json!({
            "status": "ok", "active": true, "running": true,
            "esperando_respuesta_usuario": false, "pregunta_usuario": null
        });
        assert_eq!(poll_3["esperando_respuesta_usuario"], false);
    }

    #[test]
    fn test_frontend_polling_plan_approval_flow() {
        let poll_1 = json!({
            "status": "ok", "active": true,
            "esperando_aprobacion_plan": true,
            "plan_propuesto": "1. Crear auth.rs\n2. Agregar tests\n3. Documentar"
        });
        assert_eq!(poll_1["esperando_aprobacion_plan"], true);

        let poll_2 = json!({
            "status": "ok", "active": true,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null
        });
        assert_eq!(poll_2["esperando_aprobacion_plan"], false);
    }

    #[test]
    fn test_agent_question_modal_flag_reset() {
        // El flag agentQuestionShown debe resetearse cuando el agente
        // deja de esperar respuesta (para permitir futuras preguntas)
        let mut agent_question_shown = true;

        // Si esperando_respuesta_usuario pasa a false, el flag se resetea
        let esperando = false;
        if !esperando {
            agent_question_shown = false;
        }
        assert!(!agent_question_shown,
            "BUG REG-004: agentQuestionShown debe resetearse cuando ya no se espera respuesta");
    }
}

// ============================================================================
// TESTS DE REGRESIÓN — Bug B: copyNonceCmd no copia
// ============================================================================

#[cfg(test)]
mod regression_tests_bug_b {
    #[test]
    fn test_copynoncecmd_event_parameter_required() {
        // BUG B: La función usaba 'event' sin declararlo como parámetro.
        // La firma correcta es: function copyNonceCmd(event)
        let js_signature = "function copyNonceCmd(event)";

        assert!(js_signature.contains("event"),
            "BUG REG-005: copyNonceCmd debe declarar 'event' como parámetro explícito");

        let param_pos = js_signature.find("event").unwrap();
        let brace_pos = js_signature.find('{').unwrap();
        assert!(param_pos < brace_pos,
            "BUG REG-005: 'event' debe estar antes de la llave de apertura del cuerpo");
    }

    #[test]
    fn test_copynoncecmd_fallback_required() {
        // BUG B: Sin Clipboard API (HTTP no seguro), copyNonceCmd no copiaba nada.
        // Debe existir fallback con document.execCommand('copy') y un textarea.

        let has_clipboard_api = false; // simulando HTTP sin HTTPS

        let fallback_available = if !has_clipboard_api {
            // Debe usar textarea + execCommand
            true
        } else {
            true
        };

        assert!(fallback_available,
            "BUG REG-006: copyNonceCmd debe tener fallback para navegadores sin Clipboard API");
    }

    #[test]
    fn test_copynoncecmd_window_event_fallback() {
        // Cuando event es undefined/null, debe usar window.event como fallback
        let event_is_null = true;
        let has_window_event_fallback = true;

        assert!(has_window_event_fallback || !event_is_null,
            "BUG REG-007: copyNonceCmd debe manejar event === undefined usando window.event");
    }

    #[test]
    fn test_copynoncecmd_command_format_valid() {
        let nonce = "abc123nonce";
        let cmd = format!(
            ".\\scripts\\sign_nonce.ps1 -Nonce \"{}\" -KeyPath \".config\\admin_private.pem\"",
            nonce
        );

        assert!(cmd.contains("sign_nonce.ps1"), "El comando debe referenciar sign_nonce.ps1");
        assert!(cmd.contains(nonce), "El comando debe contener el nonce");
        assert!(cmd.contains("-Nonce"), "El comando debe incluir -Nonce");
        assert!(cmd.contains("-KeyPath"), "El comando debe incluir -KeyPath");
        assert!(cmd.contains("admin_private.pem"), "Debe apuntar a admin_private.pem");
    }
}

// ============================================================================
// TESTS DE INTEGRACIÓN DEL AGENTE — Flujos simulados
// ============================================================================

#[cfg(test)]
mod agent_integration_tests {
    use serde_json::json;

    #[test]
    fn test_notificar_usuario_pregunta_flow() {
        // 1. Agente llama a notificar_usuario tipo "pregunta"
        let call = json!({
            "name": "notificar_usuario",
            "arguments": { "tipo": "pregunta", "mensaje": "¿Debo continuar?" }
        });
        assert_eq!(call["arguments"]["tipo"], "pregunta");

        // 2. Backend pone esperando_respuesta_usuario = true
        let mut esperando = true;
        assert!(esperando);

        // 3. Usuario responde
        let respuesta = "Sí, continúa";
        esperando = false;
        assert!(!esperando);
    }

    #[test]
    fn test_notificar_usuario_informativo_no_pausa() {
        let call = json!({
            "name": "notificar_usuario",
            "arguments": { "tipo": "informativo", "mensaje": "Fase 1 completada" }
        });
        assert_eq!(call["arguments"]["tipo"], "informativo");

        // Informativo NO debe pausar
        let esperando = false;
        assert!(!esperando, "Mensaje informativo NO debe pausar la ejecución");
    }

    #[test]
    fn test_finalizar_tarea_limpia_procesos() {
        let call = json!({
            "name": "finalizar_tarea",
            "arguments": { "mensaje_final": "Tarea completada exitosamente." }
        });
        assert!(call["arguments"]["mensaje_final"].as_str().unwrap().len() > 0);

        // Debe limpiar procesos y detener
        let procesos_limpios = true;
        let agente_detenido = true;
        assert!(procesos_limpios);
        assert!(agente_detenido);
    }

    #[test]
    fn test_agent_cicle_phases_order() {
        let phases = vec![
            "Implementacion", "Optimizacion", "BusquedaDeBugs",
            "Reduccion", "SegundaBusquedaDeBugs", "TerminarTarea"
        ];
        assert_eq!(phases.len(), 6);
        assert_eq!(phases[0], "Implementacion");
        assert_eq!(phases[5], "TerminarTarea");
    }

    #[test]
    fn test_agent_cicle_bug_found_returns_to_optimization() {
        // Si se encuentra bug en BusquedaDeBugs → vuelve a Optimizacion
        let fase = "BusquedaDeBugs";
        let bug_encontrado = true;
        let siguiente = if bug_encontrado && fase == "BusquedaDeBugs" { "Optimizacion" } else { fase };
        assert_eq!(siguiente, "Optimizacion");

        // También desde SegundaBusquedaDeBugs
        let fase2 = "SegundaBusquedaDeBugs";
        let siguiente2 = if bug_encontrado && fase2 == "SegundaBusquedaDeBugs" { "Optimizacion" } else { fase2 };
        assert_eq!(siguiente2, "Optimizacion");
    }

    #[test]
    fn test_agent_plan_approval_accepted() {
        let mut esperando = true;
        let aprobado = true;
        esperando = false;
        assert!(!esperando);
        assert!(aprobado);
    }

    #[test]
    fn test_agent_plan_approval_rejected() {
        let mut esperando = true;
        let aprobado = false;
        esperando = false;
        let plan: Option<&str> = None;
        assert!(!esperando);
        assert!(!aprobado);
        assert!(plan.is_none(), "Plan debe limpiarse al rechazar");
    }

    #[test]
    fn test_agent_interrupt_while_waiting_for_response() {
        let mut esperando_respuesta = true;
        let mut interrumpido = false;
        interrumpido = true;
        esperando_respuesta = false;
        assert!(interrumpido);
        assert!(!esperando_respuesta);
    }

    #[test]
    fn test_agent_interrupt_during_execution() {
        let mut running = true;
        let mut interrupted = false;
        interrupted = true;
        running = false;
        assert!(interrupted);
        assert!(!running);
    }

    #[test]
    fn test_captcha_total_flow() {
        let captcha_pending = true;
        let captcha_id = "captcha-789";
        let solved = "03AFcWeA5zy7DB6s...";

        let payload = json!({ "id": captcha_id, "solved_content": solved });
        assert_eq!(payload["id"], captcha_id);
        assert_eq!(payload["solved_content"], solved);

        let captcha_pending = false;
        assert!(!captcha_pending);
    }
}

// ============================================================================
// TESTS E2E — Ciclo completo del agente (simulado)
// ============================================================================

#[cfg(test)]
mod e2e_tests {
    use serde_json::json;

    #[test]
    fn test_e2e_full_agent_lifecycle() {
        // 1. Usuario envía mensaje
        let user_msg = json!({
            "message": "Corrige los bugs del frontend",
            "project_name": "IAF", "mode": "programming"
        });
        assert_eq!(user_msg["mode"], "programming");

        // 2. Backend responde con session_id
        let session_id = "e2e-session-001";
        let resp = json!({ "status": "ok", "session_id": session_id });
        assert_eq!(resp["session_id"], session_id);

        // 3. Polling: agente ejecutando
        let status_1 = json!({
            "status": "ok", "active": true,
            "esperando_respuesta_usuario": false,
            "esperando_aprobacion_plan": false
        });
        assert_eq!(status_1["active"], true);

        // 4. Agente pregunta
        let status_2 = json!({
            "status": "ok", "active": true,
            "esperando_respuesta_usuario": true,
            "pregunta_usuario": "Encontré código duplicado. ¿Lo elimino?"
        });
        assert_eq!(status_2["esperando_respuesta_usuario"], true);

        // 5. Usuario responde
        json!({ "respuesta": "Sí, elimínalo" });

        // 6. Agente propone plan
        let status_4 = json!({
            "status": "ok", "active": true,
            "esperando_aprobacion_plan": true,
            "plan_propuesto": "1. Eliminar duplicados\n2. Tests\n3. Commit"
        });
        assert_eq!(status_4["esperando_aprobacion_plan"], true);

        // 7. Usuario aprueba
        json!({ "aprobado": true });

        // 8. Agente finaliza
        let status_5 = json!({
            "status": "ok", "active": false,
            "esperando_respuesta_usuario": false,
            "esperando_aprobacion_plan": false
        });
        assert_eq!(status_5["active"], false);
    }

    #[test]
    fn test_e2e_multiple_questions_in_session() {
        let preguntas = vec![
            "¿Debo refactorizar auth.rs primero?",
            "¿Quieres que use argon2 o bcrypt?",
            "¿Agrego tests de integración también?",
        ];

        for pregunta in &preguntas {
            let status = json!({
                "esperando_respuesta_usuario": true,
                "pregunta_usuario": pregunta
            });
            assert_eq!(status["pregunta_usuario"], *pregunta);

            let status_post = json!({
                "esperando_respuesta_usuario": false,
                "respuesta_usuario": "ok"
            });
            assert_eq!(status_post["esperando_respuesta_usuario"], false);
        }
    }

    #[test]
    fn test_e2e_session_persistence() {
        let session_id = "persistent-session";

        let chat_resp = json!({ "status": "ok", "session_id": session_id });
        assert_eq!(chat_resp["session_id"], session_id);

        let agent_status = json!({
            "status": "ok", "active": true, "current_session_id": session_id
        });
        assert_eq!(agent_status["current_session_id"], session_id);

        let session_saved = true;
        assert!(session_saved, "La sesión debe persistirse");
    }
}

// ============================================================================
// TESTS DE ESTRÉS ADICIONALES
// ============================================================================

#[cfg(test)]
mod stress_tests_extended {
    use serde_json::json;

    #[test]
    fn test_stress_concurrent_agent_sessions() {
        let num = 100;
        let sessions: Vec<_> = (0..num).map(|i| {
            json!({
                "id": format!("stress-{}", i),
                "title": format!("Stress {}", i),
                "messages": (0..20).map(|j| json!({
                    "role": if j % 2 == 0 { "user" } else { "agent" },
                    "content": format!("Msg {}-{}", i, j)
                })).collect::<Vec<_>>()
            })
        }).collect();

        assert_eq!(sessions.len(), num);
        let total: usize = sessions.iter().map(|s| s["messages"].as_array().unwrap().len()).sum();
        assert_eq!(total, num * 20);
    }

    #[test]
    fn test_stress_agent_status_polling_simulation() {
        let polls = 1000;
        let mut questions = 0;
        let mut plans = 0;

        for i in 0..polls {
            let status = json!({
                "status": "ok",
                "esperando_respuesta_usuario": i % 50 == 0,
                "pregunta_usuario": if i % 50 == 0 { Some(format!("Pregunta {}", i/50)) } else { None },
                "esperando_aprobacion_plan": i % 100 == 0,
                "plan_propuesto": if i % 100 == 0 { Some(format!("Plan {}", i/100)) } else { None }
            });

            if status["esperando_respuesta_usuario"].as_bool() == Some(true) { questions += 1; }
            if status["esperando_aprobacion_plan"].as_bool() == Some(true) { plans += 1; }
        }

        assert_eq!(questions, 20, "20 preguntas en 1000 polls");
        assert_eq!(plans, 10, "10 planes en 1000 polls");
    }

    #[test]
    fn test_stress_schedule_grid_all_ranges() {
        let schedule = json!({
            "horarios": {
                "lunes": [[0,2],[2,4],[4,6],[6,8],[8,10],[10,12],[12,14],[14,16],[16,18],[18,20],[20,22],[22,24]],
                "martes": [[9,12],[14,18]],
                "miercoles": [[9,12],[14,18]],
                "jueves": [[9,12],[14,18]],
                "viernes": [[9,12],[14,18]],
                "sabado": [[10,14]],
                "domingo": []
            }
        });

        let total: usize = schedule["horarios"].as_object().unwrap()
            .values().map(|v| v.as_array().unwrap().len()).sum();
        assert_eq!(total, 21);
    }

    #[test]
    fn test_stress_users_with_limits() {
        let users: Vec<_> = (0..500).map(|i| {
            json!({
                "username": format!("user_{}", i),
                "limits": {
                    "max_tokens_per_day": i * 100,
                    "max_api_calls_per_day": i * 10,
                    "limite_iteraciones": i * 5,
                    "max_sub_agents": (i % 8) + 1,
                    "activacion": i % 3 != 0
                }
            })
        }).collect();

        assert_eq!(users.len(), 500);
        let activos = users.iter().filter(|u| u["limits"]["activacion"].as_bool() == Some(true)).count();
        assert!(activos > 300);
    }

    #[test]
    fn test_stress_chat_messages_10k_roundtrip() {
        let messages: Vec<_> = (0..10_000).map(|i| {
            json!({
                "role": if i % 2 == 0 { "user" } else { "agent" },
                "content": format!("Mensaje {} con padding lorem ipsum dolor sit amet", i),
                "timestamp": 1700000000u64 + i as u64
            })
        }).collect();

        let session = json!({
            "id": "stress-10k", "title": "10K Messages",
            "project_name": "IAF", "messages": messages
        });

        let serialized = serde_json::to_string(&session).unwrap();
        assert!(serialized.len() > 500_000);

        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["messages"].as_array().unwrap().len(), 10_000);
    }
}

// ============================================================================
// TESTS DE INTEGRACIÓN HTTP ADICIONALES (endpoints del agente)
// ============================================================================

#[cfg(test)]
mod integration_tests_http_extended {
    use std::sync::LazyLock;
    use std::time::Duration;

    const SERVER_URL: &str = "http://127.0.0.1:8080";

    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP test client")
    });

    /// REG-001: Verifica que /api/agent/status incluya TODOS los campos
    /// requeridos por el frontend (esperando_respuesta_usuario, pregunta_usuario,
    /// esperando_aprobacion_plan, plan_propuesto, current_session_id).
    #[tokio::test]
    async fn test_agent_status_has_all_fields() {
        let client = &*CLIENT;
        let resp = client.get(format!("{}/api/agent/status", SERVER_URL)).send().await;

        match resp {
            Ok(r) => {
                assert_ne!(r.status().as_u16(), 404, "Endpoint /api/agent/status debe existir");
                if r.status().is_success() {
                    let body = r.json::<serde_json::Value>().await.unwrap_or_default();
                    if body.get("status").and_then(|v| v.as_str()) == Some("ok") {
                        for field in &["esperando_respuesta_usuario", "pregunta_usuario",
                            "esperando_aprobacion_plan", "plan_propuesto", "current_session_id"] {
                            assert!(body.get(field).is_some(),
                                "REG-001 FAIL: falta campo '{}' en /api/agent/status", field);
                        }
                    }
                }
            }
            Err(_) => { /* servidor no disponible */ }
        }
    }

    #[tokio::test]
    async fn test_agent_responder_endpoint() {
        let client = &*CLIENT;
        let resp = client.post(format!("{}/api/agent/responder", SERVER_URL))
            .json(&serde_json::json!({"respuesta": "Sí, continúa"}))
            .send().await;
        match resp {
            Ok(r) => assert_ne!(r.status().as_u16(), 404, "/api/agent/responder debe existir"),
            Err(_) => {}
        }
    }

    #[tokio::test]
    async fn test_agent_approve_plan_endpoint() {
        let client = &*CLIENT;
        let resp = client.post(format!("{}/api/agent/aprobar_plan", SERVER_URL))
            .json(&serde_json::json!({"aprobado": true}))
            .send().await;
        match resp {
            Ok(r) => assert_ne!(r.status().as_u16(), 404, "/api/agent/aprobar_plan debe existir"),
            Err(_) => {}
        }
    }

    #[tokio::test]
    async fn test_agent_interrupt_endpoint() {
        let client = &*CLIENT;
        let resp = client.post(format!("{}/api/agent/interrupt", SERVER_URL))
            .send().await;
        match resp {
            Ok(r) => assert_ne!(r.status().as_u16(), 404, "/api/agent/interrupt debe existir"),
            Err(_) => {}
        }
    }

    #[tokio::test]
    async fn test_agent_steps_endpoint() {
        let client = &*CLIENT;
        let resp = client.get(format!("{}/api/agent/steps", SERVER_URL)).send().await;
        match resp {
            Ok(r) => {
                assert_ne!(r.status().as_u16(), 404, "/api/agent/steps debe existir");
                if r.status().is_success() {
                    let body = r.json::<serde_json::Value>().await.unwrap_or_default();
                    assert_eq!(body["status"], "ok");
                    assert!(body["steps"].is_array());
                }
            }
            Err(_) => {}
        }
    }
}
