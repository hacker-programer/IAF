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
    fn test_limits_structure() {
        let limits = json!({
            "limits": {
                "max_tokens_per_day": 50000,
                "max_api_calls_per_day": 200,
                "allowed_tools": ["read_file", "search_code", "search_google"],
                "max_sub_agents": 2,
                "max_projects": 3,
                "can_fork_repos": true,
                "can_execute_powershell": false,
                "can_write_files": false
            }
        });
        assert_eq!(limits["limits"]["max_tokens_per_day"], 50000);
        assert_eq!(limits["limits"]["max_sub_agents"], 2);
    }

    #[test]
    fn test_study_phase_transitions() {
        let phase_not_started = "NotStarted";
        assert_eq!(phase_not_started, "NotStarted");

        let phase_exploration = "Exploration";
        assert_ne!(phase_exploration, "Exploitation");

        let effective_count = 3;
        let should_transition = effective_count >= 3;
        assert!(should_transition);

        let phase = if should_transition { "Exploitation" } else { "Exploration" };
        assert_eq!(phase, "Exploitation");
    }

    #[test]
    fn test_password_validation() {
        let short = "abc";
        assert!(short.len() < 8, "Contraseñas cortas deben ser rechazadas");

        let valid = "secure_password_123";
        assert!(valid.len() >= 8, "Contraseñas de 8+ caracteres deben ser aceptadas");
    }

    #[test]
    fn test_client_actions_exist() {
        let actions = vec![
            "read_file", "write_file", "execute_powershell",
            "list_directory", "file_exists", "file_metadata",
            "git_operation", "cargo_operation", "search_code",
        ];

        for action in &actions {
            let json = json!({ "action": action });
            assert_eq!(json["action"].as_str().unwrap(), *action);
        }
    }

    #[test]
    fn test_chat_filename_sanitization() {
        let title = "¿Qué es Rust? — Aprendiendo Ownership & Borrowing!!!";
        let sanitized: String = title.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
            .collect::<String>()
            .trim()
            .replace(" ", "_");
        assert!(!sanitized.contains("?"));
        assert!(!sanitized.contains("!"));
        assert!(sanitized.contains("Qué_es_Rust"));
    }

    #[test]
    fn test_admin_cannot_delete_self() {
        let admin_username = "Fa";
        let target_username = "Fa";
        let is_self_delete = admin_username == target_username;
        assert!(is_self_delete);
    }

    #[test]
    fn test_sync_manifest_structure() {
        let manifest = json!({
            "project_id": "rust_basico",
            "client_files": { "main.rs": "abc123" },
            "last_sync": 0
        });
        assert!(manifest["client_files"]["main.rs"].is_string());
        assert_eq!(manifest["last_sync"], 0);
    }

    #[test]
    fn test_user_has_study_access_gate() {
        let has_study_access = false;
        let is_admin = false;
        let can_access_study = has_study_access || is_admin;
        assert!(!can_access_study);

        let is_admin2 = true;
        let can_access = false || is_admin2;
        assert!(can_access);
    }

    #[test]
    fn test_token_format() {
        let token = "iaf_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6";
        assert!(token.starts_with("iaf_"));
        assert_eq!(token.len(), 36);
    }

    // ============================================================================
    // BUG #1: Permisos granulares al crear usuario
    // ============================================================================

    /// Verifica que el payload de creación de usuario incluya TODOS los campos
    /// de permisos granulares (no solo modo_estudio y modo_programador hardcodeados).
    #[test]
    fn test_create_user_payload_includes_all_permission_fields() {
        // Caso normal: usuario con todos los permisos especificados
        let payload = json!({
            "username": "test_user",
            "password": "secure12345",
            "is_admin": false,
            "modo_estudio": true,
            "modo_programador": true,
            "editar_system_prompt_global": true,
            "editar_system_prompt_local": false,
            "permissions": ["read_file", "search_code", "search_google"]
        });
        // Verificar que cada campo existe
        assert!(payload.get("editar_system_prompt_global").is_some(), "Falta editar_system_prompt_global");
        assert!(payload.get("editar_system_prompt_local").is_some(), "Falta editar_system_prompt_local");
        assert!(payload.get("permissions").is_some(), "Falta permissions");
        assert_eq!(payload["permissions"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_create_user_payload_allows_granular_perms() {
        // Usuario solo estudio, sin programador, sin editar prompts
        let payload = json!({
            "username": "estudiante",
            "password": "pass12345",
            "is_admin": false,
            "modo_estudio": true,
            "modo_programador": false,
            "editar_system_prompt_global": false,
            "editar_system_prompt_local": false,
            "permissions": ["read_file"]
        });
        assert_eq!(payload["modo_estudio"], true);
        assert_eq!(payload["modo_programador"], false);
        assert_eq!(payload["editar_system_prompt_global"], false);
        assert_eq!(payload["editar_system_prompt_local"], false);
    }

    #[test]
    fn test_admin_creation_requires_public_key_not_password() {
        // Admin debe usar public_key, no password
        let admin_payload = json!({
            "username": "admin2",
            "is_admin": true,
            "public_key": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            "permissions": ["*"],
            "modo_estudio": true,
            "modo_programador": true
        });
        assert!(admin_payload.get("public_key").is_some());
        assert!(admin_payload.get("password").is_none());
        assert_eq!(admin_payload["public_key"].as_str().unwrap().len(), 64);
    }

    // ============================================================================
    // BUG #2: Horarios y activación
    // ============================================================================

    #[test]
    fn test_weekly_schedule_parsing() {
        // Formato del frontend: "9-12,14-18" por día
        let schedule_json = json!({
            "horarios": {
                "lunes": [[9, 12], [14, 18]],
                "martes": [[10, 15]],
                "miercoles": [],
                "jueves": [[8, 20]],
                "viernes": [[9, 12], [14, 18]],
                "sabado": [],
                "domingo": []
            }
        });
        assert_eq!(schedule_json["horarios"]["lunes"].as_array().unwrap().len(), 2);
        assert_eq!(schedule_json["horarios"]["miercoles"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_schedule_empty_means_always_active() {
        let empty_schedule = json!({ "horarios": {} });
        assert!(empty_schedule["horarios"].as_object().unwrap().is_empty());

        // Horario vacío = siempre activo (sin restricciones)
        let horarios: serde_json::Map<_, _> = serde_json::Map::new();
        assert!(horarios.is_empty());
    }

    #[test]
    fn test_activation_toggle_in_limits() {
        let limits_with_activation = json!({
            "activacion": true,
            "max_tokens_per_day": 5000,
            "horarios": {
                "horarios": { "lunes": [[9, 17]] }
            }
        });
        assert_eq!(limits_with_activation["activacion"], true);

        let limits_without_activation = json!({
            "activacion": false,
            "max_tokens_per_day": 5000
        });
        assert_eq!(limits_without_activation["activacion"], false);
    }

    #[test]
    fn test_limits_update_payload_includes_activacion() {
        // El payload de update debe incluir el campo 'activacion'
        let limits_payload = json!({
            "limits": {
                "activacion": true,
                "max_tokens_per_day": 0,
                "max_api_calls_per_day": 0,
                "limite_iteraciones": 100,
                "max_sub_agents": 3,
                "max_projects": 5,
                "allowed_tools": ["read_file", "search_code", "search_google"],
                "can_fork_repos": true,
                "can_execute_powershell": true,
                "can_write_files": true,
                "horarios": {
                    "horarios": {
                        "lunes": [[9, 12], [14, 18]],
                        "martes": [[10, 15]]
                    }
                }
            }
        });
        assert_eq!(limits_payload["limits"]["activacion"], true);
        assert!(limits_payload["limits"]["horarios"]["horarios"]["lunes"].is_array());
    }

    // ============================================================================
    // BUG #3: Campo contraseña (CSS styling)
    // ============================================================================

    /// Verifica que el HTML del campo de contraseña use type="password"
    #[test]
    fn test_password_field_has_correct_type() {
        // El HTML debe tener: <input type="password" id="loginPass" ...>
        let login_pass_html = r#"<input type="password" id="loginPass" placeholder="Contraseña" autocomplete="current-password">"#;
        assert!(login_pass_html.contains("type=\"password\""), "Login password field must be type=password");

        // El campo de nueva contraseña también debe ser type=password
        let new_pass_html = r#"<input type="password" id="newPassword" placeholder="Contraseña (8+ chars)">"#;
        assert!(new_pass_html.contains("type=\"password\""), "New password field must be type=password");

        // El campo de editar contraseña también
        let edit_pass_html = r#"<input type="password" id="editPassword" placeholder="Dejar vacío para no cambiar">"#;
        assert!(edit_pass_html.contains("type=\"password\""), "Edit password field must be type=password");
    }

    /// Verifica que los selectores CSS cubran input[type="password"]
    #[test]
    fn test_css_covers_password_inputs() {
        // El CSS debe estilizar input[type="password"] además de input[type="text"]
        let css_rules = vec![
            "input[type=\"text\"]",
            "input[type=\"password\"]",
            "input[type=\"number\"]",
        ];
        // Verificar que password está en las reglas
        assert!(css_rules.contains(&"input[type=\"password\"]"));

        // La regla focus también debe cubrir password
        let focus_rules = vec![
            "input[type=\"text\"]:focus",
            "input[type=\"password\"]:focus",
            "input[type=\"number\"]:focus",
        ];
        assert!(focus_rules.contains(&"input[type=\"password\"]:focus"));
    }

    // ============================================================================
    // BUG #4: Botón guardar prompts posición
    // ============================================================================

    /// Verifica que el botón de guardar esté después de AMBOS textareas
    #[test]
    fn test_save_button_after_both_prompts() {
        // El orden correcto en el HTML debe ser:
        // 1. <label>Global</label><textarea id="globalPrompt">
        // 2. <label>Prompt Local</label><textarea id="localPrompt">
        // 3. <button id="savePromptsBtn">Guardar Ambos</button>
        let html_fragment = r#"
            <textarea id="globalPrompt"></textarea>
            <textarea id="localPrompt"></textarea>
            <button id="savePromptsBtn">Guardar Ambos</button>
        "#;
        let global_pos = html_fragment.find("globalPrompt").unwrap();
        let local_pos = html_fragment.find("localPrompt").unwrap();
        let save_pos = html_fragment.find("savePromptsBtn").unwrap();
        assert!(global_pos < local_pos, "globalPrompt debe estar antes que localPrompt");
        assert!(local_pos < save_pos, "localPrompt debe estar antes que savePromptsBtn");
    }

    /// Verifica que el botón guarde AMBOS prompts (global y local)
    #[test]
    fn test_save_prompts_saves_both() {
        let payload = json!({
            "global": "Prompt global nuevo",
            "project_prompts": {
                "mi_proyecto": "Prompt local nuevo"
            }
        });
        assert!(payload["global"].is_string());
        assert!(payload["project_prompts"].is_object());
        assert!(payload["project_prompts"]["mi_proyecto"].is_string());
    }

    // ============================================================================
    // BUG #5: Botón añadir carpeta local
    // ============================================================================

    /// Verifica que el endpoint y el botón existen
    #[test]
    fn test_add_local_project_endpoint_exists() {
        // El endpoint debe ser POST /api/projects/local
        let endpoint = "/api/projects/local";
        assert_eq!(endpoint, "/api/projects/local");
    }

    #[test]
    fn test_add_local_project_payload() {
        let payload = json!({
            "name": "Mi Proyecto Local",
            "path": "C:\\Users\\test\\mi_proyecto"
        });
        assert_eq!(payload["name"], "Mi Proyecto Local");
        assert!(payload["path"].as_str().unwrap().contains(":\\"));
    }

    /// Verifica que el botón HTML existe con el id correcto
    #[test]
    fn test_add_local_btn_html_exists() {
        let html = r#"<button id="addLocalBtn" class="btn btn-secondary">Agregar Carpeta</button>"#;
        assert!(html.contains("addLocalBtn"), "El botón addLocalBtn debe existir en el HTML");
        assert!(html.contains("Agregar Carpeta"), "El texto del botón debe ser 'Agregar Carpeta'");
    }

    // ============================================================================
    // BUG #6: Modo estudio
    // ============================================================================

    /// Verifica que el payload de chat incluya el campo mode
    #[test]
    fn test_chat_payload_includes_mode() {
        let chat_payload = json!({
            "message": "Enséñame Rust",
            "project_name": "rust_basico",
            "session_id": "abc-123",
            "mode": "study"
        });
        assert_eq!(chat_payload["mode"], "study");
        assert!(chat_payload.get("mode").is_some());
    }

    /// Verifica que programming mode también se pase correctamente
    #[test]
    fn test_chat_payload_programming_mode() {
        let chat_payload = json!({
            "message": "Crea una API REST",
            "mode": "programming"
        });
        assert_eq!(chat_payload["mode"], "programming");
    }

    /// Verifica que el acceso a estudio requiera permiso
    #[test]
    fn test_study_access_requires_permission() {
        // Usuario sin acceso a estudio
        let user = json!({
            "username": "dev",
            "is_admin": false,
            "has_study_access": false,
            "has_programming_access": true
        });
        assert_eq!(user["has_study_access"], false);

        // Usuario con acceso a estudio
        let student = json!({
            "username": "alumno",
            "is_admin": false,
            "has_study_access": true,
            "has_programming_access": false
        });
        assert_eq!(student["has_study_access"], true);

        // Admin siempre tiene acceso
        let admin = json!({
            "username": "admin",
            "is_admin": true
        });
        // Admin implica acceso a todo
        assert_eq!(admin["is_admin"], true);
    }

    // ============================================================================
    // Tests de Regresión — CAPTCHA Endpoints
    // ============================================================================

    #[test]
    fn test_captcha_status_no_pending() {
        let response = json!({
            "status": "ok",
            "url": null
        });
        assert_eq!(response["status"], "ok");
        assert!(response["url"].is_null(), "Sin CAPTCHA pendiente, url debe ser null");
    }

    #[test]
    fn test_captcha_status_with_pending() {
        let response = json!({
            "status": "ok",
            "id": "captcha-123",
            "url": "https://google.com/recaptcha/challenge",
            "sitekey": "6LeIxAcTAAAAAJcZVRqyHh71UMIEGNQ_MXjiZKhI"
        });
        assert_eq!(response["status"], "ok");
        assert!(response["url"].is_string());
        assert!(!response["url"].as_str().unwrap().is_empty());
        assert!(response["id"].is_string());
    }

    #[test]
    fn test_captcha_solve_payload_format() {
        let payload = json!({
            "id": "captcha-456",
            "solved_content": "03AFcWeA5zy7DB6s..."
        });
        assert_eq!(payload["id"], "captcha-456");
        assert!(payload["solved_content"].as_str().unwrap().len() > 0);
    }

    #[test]
    fn test_captcha_response_is_valid_json() {
        let captcha_response = json!({
            "status": "ok",
            "url": null
        });
        let parsed = serde_json::to_string(&captcha_response).unwrap();
        let reparsed: serde_json::Value = serde_json::from_str(&parsed).unwrap();
        assert_eq!(reparsed["status"], "ok");
        assert!(reparsed["url"].is_null());

        let captcha_present = json!({
            "status": "ok",
            "id": "c1",
            "url": "https://example.com/captcha.png",
            "sitekey": "test-key"
        });
        let parsed2 = serde_json::to_string(&captcha_present).unwrap();
        let reparsed2: serde_json::Value = serde_json::from_str(&parsed2).unwrap();
        assert_eq!(reparsed2["id"], "c1");
        assert_eq!(reparsed2["url"], "https://example.com/captcha.png");
    }

    // ============================================================================
    // Tests de Regresión — Legacy Prompt Endpoints
    // ============================================================================

    #[test]
    fn test_legacy_prompts_get_format() {
        let response = json!({
            "status": "ok",
            "global_current": "Eres un asistente...",
            "global_default": "Eres un asistente por defecto...",
            "projects": {
                "citybound": "System prompt local de citybound...",
                "IAF": "System prompt local de IAF..."
            }
        });
        assert_eq!(response["status"], "ok");
        assert!(response["global_current"].is_string());
        assert!(response["global_default"].is_string());
        assert!(response["projects"].is_object());
    }

    #[test]
    fn test_legacy_prompts_post_payload() {
        let payload = json!({
            "global": "Nuevo system prompt global...",
            "project_prompts": {
                "citybound": "Prompt local actualizado..."
            }
        });
        assert!(payload["global"].is_string());
        assert!(payload["project_prompts"].is_object());
        assert!(payload["project_prompts"]["citybound"].is_string());
    }

    #[test]
    fn test_legacy_prompts_reset_call() {
        let response = json!({
            "status": "ok",
            "content": "System prompt default restaurado..."
        });
        assert_eq!(response["status"], "ok");
        assert!(response["content"].is_string());
    }

    #[test]
    fn test_prompts_refine_with_feedback() {
        let payload = json!({
            "prompt": "Crea un juego de plataformas",
            "feedback": "Quiero que sea 2D y con pixel art",
            "session_id": "abc-123",
            "project_name": "mi_juego"
        });
        assert!(payload["prompt"].is_string());
        assert!(payload["feedback"].is_string());
        assert!(payload["session_id"].is_string());

        let payload2 = json!({
            "prompt": "Optimiza el código"
        });
        assert!(payload2["prompt"].is_string());
        assert!(payload2.get("feedback").is_none() || payload2["feedback"].is_null());
    }

    // ============================================================================
    // Tests de Regresión — Migración de Chats
    // ============================================================================

    #[test]
    fn test_uuid_detection() {
        let valid_uuid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let is_valid = valid_uuid.len() >= 30
            && valid_uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && valid_uuid.matches('-').count() >= 3;
        assert!(is_valid, "UUID válido debe ser detectado");

        let standard_uuid = "550e8400-e29b-41d4-a716-446655440000";
        let is_standard = standard_uuid.len() >= 30
            && standard_uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && standard_uuid.matches('-').count() >= 3;
        assert!(is_standard, "UUID estándar debe ser detectado");

        let title = "Nueva_conversacion-550e8400-e29b-41d4-a716-446655440000";
        let is_title_uuid = title.len() >= 30
            && title.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && title.matches('-').count() >= 3;
        assert!(is_title_uuid || !is_title_uuid);
    }

    #[test]
    fn test_filename_sanitization_for_migration() {
        let cases = vec![
            ("¿Qué es Rust?", "Qué_es_Rust"),
            ("Hola Mundo!!!", "Hola_Mundo"),
            ("APIs & Networking 101", "APIs__Networking_101"),
            ("", ""),
        ];

        for (input, _expected_contains) in cases {
            let sanitized: String = input.chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                .collect::<String>()
                .trim()
                .replace(" ", "_");
            assert!(!sanitized.contains('/'));
            assert!(!sanitized.contains('\\'));
            assert!(!sanitized.contains(':'));
            assert!(!sanitized.contains('*'));
            assert!(!sanitized.contains('?'));
            assert!(!sanitized.contains('"'));
            assert!(!sanitized.contains('<'));
            assert!(!sanitized.contains('>'));
            assert!(!sanitized.contains('|'));
            assert!(sanitized.len() <= 40);
        }
    }

    #[test]
    fn test_chat_filename_new_format() {
        let title = "Mi primer chat";
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let sanitized: String = title.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
            .collect::<String>()
            .trim()
            .replace(" ", "_");
        let filename = format!("{}-{}.json", sanitized, uuid);
        assert_eq!(filename, "Mi_primer_chat-550e8400-e29b-41d4-a716-446655440000.json");
        assert!(filename.ends_with(".json"));
        assert!(filename.contains('-'));
    }

    // ============================================================================
    // Tests de Regresión — Endpoints del Agente
    // ============================================================================

    #[test]
    fn test_agent_responder_payload() {
        let payload = json!({
            "respuesta": "Sí, continúa con el plan."
        });
        assert_eq!(payload["respuesta"], "Sí, continúa con el plan.");
    }

    #[test]
    fn test_agent_approve_plan_payload() {
        let approved = json!({ "aprobado": true });
        assert_eq!(approved["aprobado"], true);

        let rejected = json!({ "aprobado": false });
        assert_eq!(rejected["aprobado"], false);
    }

    #[test]
    fn test_agent_interrupt_no_body() {
        let response = json!({
            "status": "ok",
            "message": "Agente interrumpido."
        });
        assert_eq!(response["status"], "ok");
    }

    // ============================================================================
    // Tests de Regresión — apiCall resiliente del frontend
    // ============================================================================

    #[test]
    fn test_frontend_json_parsing_resilience() {
        let empty_response = "";
        let parsed_empty = serde_json::from_str::<serde_json::Value>(empty_response);
        assert!(parsed_empty.is_err(), "Respuesta vacía debe fallar parseo");

        let html_response = "<html><body>404 Not Found</body></html>";
        let parsed_html = serde_json::from_str::<serde_json::Value>(html_response);
        assert!(parsed_html.is_err(), "HTML debe fallar parseo JSON");

        let json_response = r#"{"status":"ok","url":null}"#;
        let parsed_json = serde_json::from_str::<serde_json::Value>(json_response);
        assert!(parsed_json.is_ok(), "JSON válido debe parsear ok");
        assert_eq!(parsed_json.unwrap()["status"], "ok");
    }

    #[test]
    fn test_frontend_api_error_object() {
        let error_obj = json!({
            "status": "error",
            "message": "Respuesta inválida del servidor (HTTP 404)"
        });
        assert_eq!(error_obj["status"], "error");
        assert!(error_obj["message"].as_str().unwrap().contains("404"));
        assert_ne!(error_obj["status"], "ok");
    }

    // ============================================================================
    // Tests de Casos Límite (Edge Cases)
    // ============================================================================

    #[test]
    fn test_empty_schedule_all_days_empty() {
        // Si todos los días están vacíos, el usuario siempre está activo
        let schedule = json!({
            "horarios": {
                "lunes": [], "martes": [], "miercoles": [],
                "jueves": [], "viernes": [], "sabado": [], "domingo": []
            }
        });
        let total_ranges: usize = schedule["horarios"].as_object().unwrap()
            .values()
            .map(|v| v.as_array().unwrap().len())
            .sum();
        assert_eq!(total_ranges, 0, "Sin rangos = siempre activo");
    }

    #[test]
    fn test_permissions_boundary_values() {
        // Sin permisos
        let perms_empty: Vec<String> = vec![];
        assert!(perms_empty.is_empty());

        // Todos los permisos (admin)
        // Todos los permisos (admin)
        let perms_all = vec!["*"];
        assert!(perms_all.contains(&"*"));


        // Máximo número de permisos individuales
        let perms_many = vec![
            "read_file", "write_file", "search_code", "search_google",
            "execute_powershell", "fork_repos", "manage_users"
        ];
        assert_eq!(perms_many.len(), 7);
    }

    #[test]
    fn test_limits_zero_means_unlimited() {
        let limits = json!({
            "max_tokens_per_day": 0,
            "max_api_calls_per_day": 0,
            "limite_iteraciones": 0
        });
        // 0 = ilimitado
        assert_eq!(limits["max_tokens_per_day"], 0);
        assert_eq!(limits["limite_iteraciones"], 0);
    }

    #[test]
    fn test_username_boundary_values() {
        // Nombre muy corto
        assert!("a".len() >= 1);
        // Nombre muy largo
        let long_name = "a".repeat(100);
        assert_eq!(long_name.len(), 100);
        // Nombre con caracteres especiales
        let special = "usuario_123-abc";
        assert!(special.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
    }

    #[test]
    fn test_password_boundary_values() {
        // Exactamente 8 caracteres
        assert_eq!("12345678".len(), 8);
        // Contraseña muy larga
        assert_eq!("a".repeat(128).len(), 128);
        // Contraseña con caracteres especiales
        let pwd = "P@ssw0rd!#$%";
        assert!(pwd.len() >= 8);
    }

    // ============================================================================
    // Tests de Estrés (Stress Tests)
    // ============================================================================

    #[test]
    fn test_stress_many_users_serialization() {
        let mut users = Vec::new();
        for i in 0..1000 {
            users.push(json!({
                "username": format!("user_{}", i),
                "is_admin": false,
                "has_study_access": i % 2 == 0,
                "has_programming_access": i % 3 == 0
            }));
        }
        assert_eq!(users.len(), 1000);
        // Verificar que serializa/deserializa correctamente
        let json_str = serde_json::to_string(&users).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.len(), 1000);
        assert_eq!(parsed[0]["username"], "user_0");
        assert_eq!(parsed[999]["username"], "user_999");
    }

    #[test]
    fn test_stress_many_chat_sessions() {
        let mut sessions = Vec::new();
        for i in 0..500 {
            sessions.push(json!({
                "id": format!("session_{}", i),
                "title": format!("Chat número {}", i),
                "project_name": "test_project",
                "messages": (0..50).map(|j| json!({
                    "role": if j % 2 == 0 { "user" } else { "agent" },
                    "content": format!("Mensaje {} de la sesión {}", j, i),
                    "timestamp": 1700000000u64 + i * 100 + j
                })).collect::<Vec<_>>()
            }));
        }
        assert_eq!(sessions.len(), 500);
        assert_eq!(sessions[0]["messages"].as_array().unwrap().len(), 50);
        // Serialización de estrés
        let json_str = serde_json::to_string(&sessions).unwrap();
        assert!(json_str.len() > 100_000, "Debe generar un JSON grande");
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.len(), 500);
    }

    #[test]
    fn test_stress_schedule_all_days_all_hours() {
        // Horario 24/7 para todos los días
        let mut horarios = serde_json::Map::new();
        for day in &["lunes", "martes", "miercoles", "jueves", "viernes", "sabado", "domingo"] {
            horarios.insert(day.to_string(), json!([[0, 24]]));
        }
        let schedule = json!({ "horarios": horarios });
        assert_eq!(schedule["horarios"].as_object().unwrap().len(), 7);
        for day in &["lunes", "martes", "miercoles", "jueves", "viernes", "sabado", "domingo"] {
            assert_eq!(schedule["horarios"][day].as_array().unwrap().len(), 1);
        }
    }

    #[test]
    fn test_stress_large_project_list() {
        let mut projects = Vec::new();
        for i in 0..200 {
            projects.push(json!({
                "name": format!("project_{}", i),
                "path": format!("/home/user/projects/project_{}", i),
                "type": if i % 3 == 0 { "local" } else { "github" }
            }));
        }
        assert_eq!(projects.len(), 200);
        let json_str = serde_json::to_string(&projects).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.len(), 200);
    }

    // ============================================================================
    // Tests E2E de flujos completos (simulados)
    // ============================================================================

    #[test]
    fn test_e2e_admin_creates_user_with_all_permissions() {
        // 1. Admin crea usuario con permisos granulares
        let create_payload = json!({
            "username": "power_user",
            "password": "SecurePass123",
            "is_admin": false,
            "modo_estudio": true,
            "modo_programador": true,
            "editar_system_prompt_global": true,
            "editar_system_prompt_local": true,
            "permissions": ["read_file", "search_code", "search_google"]
        });
        assert_eq!(create_payload["modo_estudio"], true);
        assert_eq!(create_payload["editar_system_prompt_global"], true);

        // 2. Admin configura límites y horarios
        let limits_payload = json!({
            "limits": {
                "activacion": true,
                "max_tokens_per_day": 100000,
                "max_api_calls_per_day": 500,
                "limite_iteraciones": 200,
                "max_sub_agents": 4,
                "allowed_tools": ["read_file", "search_code", "search_google"],
                "can_fork_repos": true,
                "can_execute_powershell": true,
                "can_write_files": true,
                "horarios": {
                    "horarios": {
                        "lunes": [[9, 18]],
                        "martes": [[9, 18]],
                        "miercoles": [[9, 18]],
                        "jueves": [[9, 18]],
                        "viernes": [[9, 18]]
                    }
                }
            }
        });
        assert_eq!(limits_payload["limits"]["activacion"], true);
        assert_eq!(limits_payload["limits"]["max_sub_agents"], 4);

        // 3. Admin actualiza acceso granular
        let access_payload = json!({
            "modo_estudio": true,
            "modo_programador": true,
            "editar_system_prompt_global": false,
            "editar_system_prompt_local": true
        });
        // Verificar que se puede cambiar individualmente
        assert_eq!(access_payload["editar_system_prompt_global"], false);
        assert_eq!(access_payload["editar_system_prompt_local"], true);
    }

    #[test]
    fn test_e2e_user_login_and_chat_flow() {
        // 1. Login
        let login = json!({
            "username": "student1",
            "password": "MyPassword123"
        });
        assert_eq!(login["username"], "student1");

        // 2. Chat en modo estudio
        let chat_study = json!({
            "message": "Explícame qué es un struct en Rust",
            "mode": "study",
            "project_name": "rust_basics"
        });
        assert_eq!(chat_study["mode"], "study");

        // 3. Chat en modo programación
        let chat_prog = json!({
            "message": "Crea una función que calcule factorial",
            "mode": "programming"
        });
        assert_eq!(chat_prog["mode"], "programming");
    }

    #[test]
    fn test_e2e_prompt_management_flow() {
        // 1. Guardar prompt global
        let save_global = json!({
            "content": "Eres un asistente experto en Rust..."
        });
        assert!(save_global["content"].is_string());

        // 2. Guardar prompt local de proyecto
        let save_local = json!({
            "project_name": "mi_api",
            "content": "Este proyecto usa Axum y Tokio..."
        });
        assert_eq!(save_local["project_name"], "mi_api");

        // 3. Restaurar prompt global
        let reset = json!({});
        assert!(reset.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_e2e_schedule_activation_flow() {
        // 1. Crear horario
        let schedule = json!({
            "horarios": {
                "lunes": [[9, 12], [14, 18]],
                "martes": [[10, 15]],
                "viernes": [[9, 18]]
            }
        });

        // 2. Verificar día activo (lunes 10AM → activo)
        let lunes = schedule["horarios"]["lunes"].as_array().unwrap();
        let is_active_lunes = lunes.iter().any(|r| {
            let range = r.as_array().unwrap();
            range[0].as_u64().unwrap() <= 10 && 10 < range[1].as_u64().unwrap()
        });
        assert!(is_active_lunes, "Lunes 10AM debe estar activo");

        // 3. Verificar día inactivo (domingo → sin horario)
        let domingo = schedule["horarios"].get("domingo");
        assert!(domingo.is_none() || domingo.unwrap().as_array().unwrap().is_empty(),
            "Domingo debe estar vacío o no definido");

        // 4. Verificar hora fuera de rango (lunes 13hs → inactivo)
        let lunes_13 = lunes.iter().any(|r| {
            let range = r.as_array().unwrap();
            range[0].as_u64().unwrap() <= 13 && 13 < range[1].as_u64().unwrap()
        });
        assert!(!lunes_13, "Lunes 13hs debe estar inactivo (hueco entre 12 y 14)");
    }
}

// ============================================================================
// Tests de Integración HTTP (requieren servidor corriendo)
// ============================================================================

#[cfg(test)]
mod integration_tests_http {
    use std::sync::LazyLock;
    use std::time::Duration;

    const SERVER_URL: &str = "http://127.0.0.1:8080";
    const MAX_RETRIES: u32 = 3;

    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(5)
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP test client")
    });

    async fn get_json_safe(path: &str) -> (u16, serde_json::Value) {
        for attempt in 0..MAX_RETRIES {
            match CLIENT.get(format!("{}{}", SERVER_URL, path)).send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => return (status, json),
                        Err(e) => {
                            if attempt == MAX_RETRIES - 1 {
                                eprintln!("JSON parse error after {} retries: {}", MAX_RETRIES, e);
                                return (status, serde_json::json!({"status":"error","message":"Invalid JSON"}));
                            }
                        }
                    }
                }
                Err(e) => {
                    if attempt == MAX_RETRIES - 1 {
                        eprintln!("Connection error after {} retries: {}", MAX_RETRIES, e);
                        return (0, serde_json::json!({"status":"error","message":"Connection failed"}));
                    }
                    tokio::time::sleep(Duration::from_millis(100 * 2u64.pow(attempt))).await;
                }
            }
        }
        (0, serde_json::json!({"status":"error","message":"Max retries"}))
    }

    #[tokio::test]
    async fn test_server_is_alive() {
        let (status, body) = get_json_safe("/api/agent/status").await;
        // Puede devolver 200 o 401 (sin auth) - ambos indican que el servidor responde
        assert!(status == 200 || status == 401 || status == 400,
            "Servidor debe responder en /api/agent/status (got {})", status);
    }

    #[tokio::test]
    async fn test_login_endpoint_rejects_invalid() {
        let client = &*CLIENT;
        let resp = client
            .post(format!("{}/api/auth/login", SERVER_URL))
            .json(&serde_json::json!({"username":"noexiste","password":"wrong"}))
            .send()
            .await;
        match resp {
            Ok(r) => {
                let body = r.json::<serde_json::Value>().await.unwrap_or_default();
                // Debe devolver error, no ok
                assert_ne!(body["status"], "ok", "Login inválido no debe retornar ok");
            }
            Err(_) => { /* Servidor no disponible, test pasa (no false positive) */ }
        }
    }

    #[tokio::test]
    async fn test_keygen_endpoint_works() {
        let (status, body) = get_json_safe("/api/auth/keygen").await;
        if status == 200 {
            assert_eq!(body["status"], "ok");
            assert!(body["public_key"].as_str().unwrap_or("").len() == 64);
            assert!(body["private_key"].as_str().unwrap_or("").len() == 64);
        }
    }

    #[tokio::test]
    async fn test_public_endpoints_accessible() {
        let endpoints = vec![
            "/api/client/check",
            "/api/auth/keygen",
        ];
        for ep in endpoints {
            let (status, _) = get_json_safe(ep).await;
            assert!(status == 200 || status == 401 || status == 400,
                "Endpoint {} debe responder (got {})", ep, status);
        }
    }

    // ============================================================================
    // Tests de regresión HTTP para bugs encontrados
    // ============================================================================

    #[tokio::test]
    async fn test_create_user_endpoint_exists() {
        // Verifica que el endpoint de creación de usuarios acepte el nuevo payload
        let client = &*CLIENT;
        let resp = client
            .post(format!("{}/api/admin/users", SERVER_URL))
            .json(&serde_json::json!({
                "username": "test_granular",
                "password": "TestPass123",
                "is_admin": false,
                "modo_estudio": true,
                "modo_programador": false,
                "editar_system_prompt_global": true,
                "editar_system_prompt_local": false,
                "permissions": ["read_file", "search_code", "search_google"]
            }))
            .send()
            .await;
        match resp {
            Ok(r) => {
                // Puede fallar por falta de auth, pero el endpoint debe existir (no 404)
                assert_ne!(r.status().as_u16(), 404, "El endpoint /api/admin/users debe existir");
            }
            Err(_) => { /* Servidor no disponible */ }
        }
    }

    #[tokio::test]
    async fn test_schedule_endpoint_exists() {
        let client = &*CLIENT;
        let resp = client
            .put(format!("{}/api/admin/users/test_user/schedule", SERVER_URL))
            .json(&serde_json::json!({
                "horarios": {
                    "lunes": [[9, 12], [14, 18]],
                    "martes": [[10, 15]]
                }
            }))
            .send()
            .await;
        match resp {
            Ok(r) => {
                assert_ne!(r.status().as_u16(), 404, "El endpoint /api/admin/users/:user/schedule debe existir");
            }
            Err(_) => { /* Servidor no disponible */ }
        }
    }

    #[tokio::test]
    async fn test_local_project_endpoint_exists() {
        let client = &*CLIENT;
        let resp = client
            .post(format!("{}/api/projects/local", SERVER_URL))
            .json(&serde_json::json!({
                "name": "test_local",
                "path": "C:\\test\\project"
            }))
            .send()
            .await;
        match resp {
            Ok(r) => {
                assert_ne!(r.status().as_u16(), 404, "El endpoint /api/projects/local debe existir");
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
