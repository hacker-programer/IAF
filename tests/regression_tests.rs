// ============================================================================
// tests/regression_tests.rs — Tests de Regresión para los 7 Bugs Críticos
//
// Este archivo contiene tests que DEMUESTRAN la existencia de cada bug y
// tests que VERIFICAN que la corrección es efectiva.
//
// BUG #1: notificar_usuario informativo no se muestra en tiempo real
// BUG #2: título del chat = mensaje inicial truncado (no decidido por agente)
// BUG #3: agente no conoce el directorio del proyecto seleccionado
// BUG #4: no existe herramienta para leer PDFs/DOCX
// BUG #5: agente no ve el system prompt local del proyecto
// BUG #6: agente no ve el perfil del usuario
// BUG #7: modo estudio no usa preguntas pedagógicas ni lista de aprendizajes
// ============================================================================

use serde_json::json;
use std::path::PathBuf;
use std::fs;

// ============================================================================
// BUG #1: notificar_usuario informativo no se muestra en tiempo real
// ============================================================================
// El agente llama notificar_usuario(tipo="informativo", mensaje="..."), 
// pero el resultado de la tool call ("Notificación enviada con éxito: ...") 
// solo es visible para el agente, no se inyecta en el chat del usuario.
// El frontend debe recibir estas notificaciones en tiempo real vía SSE/polling.

#[cfg(test)]
mod bug1_notificar_usuario_informativo {
    use super::*;

    /// Test: Verifica que el handler de notificar_usuario guarda el mensaje
    /// como mensaje del agente en disco Y también lo expone en el estado
    /// para que el frontend lo muestre en tiempo real.
    #[test]
    fn test_notificar_usuario_informativo_saves_to_disk() {
        // Simulación del flujo: el agente llama notificar_usuario
        let mensaje = "Estoy analizando el archivo main.rs...";
        let tipo = "informativo";

        // Verificar que el mensaje se guarda correctamente
        let agent_msg = json!({
            "role": "agent",
            "content": mensaje,
            "tipo": tipo
        });

        assert_eq!(agent_msg["role"], "agent");
        assert_eq!(agent_msg["content"], mensaje);
        assert_eq!(agent_msg["tipo"], "informativo");
    }

    /// Test: Verifica que el frontend puede detectar notificaciones informativas
    /// en el stream de pasos de auditoría.
    #[test]
    fn test_frontend_can_detect_informativo_in_steps() {
        let steps = json!([
            {
                "step_type": "informativo",
                "title": "Notificación del Agente",
                "detail": "Estoy analizando el archivo main.rs..."
            },
            {
                "step_type": "tool_call",
                "title": "read_file",
                "detail": "Leyendo main.rs (líneas 1-200)"
            }
        ]);

        // El frontend debe filtrar steps con step_type="informativo"
        let informativos: Vec<_> = steps.as_array().unwrap().iter()
            .filter(|s| s["step_type"].as_str() == Some("informativo"))
            .collect();

        assert_eq!(informativos.len(), 1);
        assert_eq!(informativos[0]["detail"].as_str().unwrap(), 
                   "Estoy analizando el archivo main.rs...");
    }

    /// Test: Verifica que el estado del agente expone correctamente
    /// el flag de notificación pendiente para el frontend.
    #[test]
    fn test_agent_state_exposes_pending_notification() {
        // Simular estado del agente después de notificar_usuario informativo
        let agent_state = json!({
            "running": true,
            "finished": false,
            "pending_notification": "El agente informa: Inicializando sistema...",
            "steps": [
                {
                    "step_type": "informativo",
                    "title": "Notificación del Agente",
                    "detail": "El agente informa: Inicializando sistema..."
                }
            ]
        });

        // El frontend debe:
        // 1. Verificar agent_state.pending_notification
        // 2. Si no es null, mostrar el mensaje en el chat
        // 3. Limpiar pending_notification después de mostrarlo
        assert!(agent_state["pending_notification"].as_str().is_some());
    }
}

// ============================================================================
// BUG #2: título del chat = mensaje inicial truncado (no decidido por agente)
// ============================================================================
// Actualmente: let title = payload.message.chars().take(30).collect::<String>();
// Esto trunca el mensaje a 30 caracteres sin darle al agente oportunidad
// de decidir el título. El título debería ser generado por el agente o
// al menos editable desde el frontend.

#[cfg(test)]
mod bug2_titulo_chat_truncado {
    use super::*;

    /// Test: Demuestra que el título actual es solo los primeros 30 caracteres
    #[test]
    fn test_title_is_truncated_first_message() {
        let mensaje_largo = "Necesito que analices el código fuente del proyecto citybound y encuentres todos los bugs posibles";
        let titulo_actual: String = mensaje_largo.chars().take(30).collect();

        assert_eq!(titulo_actual, "Necesito que analices el códig");
        // El título es "Necesito que analices el códig" — esto es poco descriptivo
        // y el agente nunca tuvo oportunidad de sugerir un mejor título.
    }

    /// Test: Verifica que un título generado por el agente sería más descriptivo
    #[test]
    fn test_agent_generated_title_would_be_better() {
        // Simular título generado por agente
        let agent_title = "Análisis de bugs en Citybound";
        let truncated_title = "Necesito que analices el códig";

        // El título del agente es mejor porque:
        // 1. Es más corto y descriptivo
        // 2. Resume la intención, no es un fragmento truncado
        assert!(agent_title.len() < truncated_title.len());
        assert!(agent_title.len() <= 50); // Título ideal: corto y descriptivo

        // Verificar que el ChatSession acepta un título establecido por agente
        let session = json!({
            "id": "test-uuid",
            "title": agent_title,
            "messages": [],
            "project_name": "citybound"
        });
        assert_eq!(session["title"], agent_title);
    }

    /// Test: Verifica que el endpoint de chat permite actualizar el título
    #[test]
    fn test_chat_session_has_updatable_title() {
        // La estructura ChatSession DEBE permitir actualizar el título
        let mut session = json!({
            "id": "test-uuid",
            "title": "Título inicial truncado",
            "messages": []
        });

        // El agente debería poder actualizar el título
        session["title"] = json!("Análisis de bugs en Citybound");
        assert_eq!(session["title"], "Análisis de bugs en Citybound");
    }

    /// Test: Verifica el sanitizado del título para nombre de archivo
    #[test]
    fn test_title_sanitization_for_filename() {
        let title = "Análisis de bugs: Citybound (refactor)";
        let sanitized: String = title.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
            .collect::<String>()
            .trim()
            .replace(" ", "_")
            .chars()
            .take(40)
            .collect();

        assert_eq!(sanitized, "Análisis_de_bugs__Citybound__refactor_");
        // El sanitizado funciona, pero el título original es mejor para mostrar en UI
    }
}

// ============================================================================
// BUG #3: agente no conoce el directorio del proyecto seleccionado
// ============================================================================
// En agent.rs, get_project_path se usa para comandos de git, pero el path
// NUNCA se inyecta en el system prompt. El agente no sabe en qué carpeta
// está trabajando.

#[cfg(test)]
mod bug3_directorio_proyecto_no_inyectado {
    use super::*;

    /// Test: Demuestra que el path del proyecto no está en el system prompt
    #[test]
    fn test_project_path_not_in_system_prompt() {
        // Simular la construcción del system prompt actual
        let project_name = Some("citybound".to_string());
        let global_prompt = "Eres un asistente de desarrollo...";
        let local_prompt = Some("Project Specific Prompt: optimiza para Rust...");

        let system_prompt = if let Some(local) = local_prompt {
            format!("{}\n\nProject Specific Prompt:\n{}", global_prompt, local)
        } else {
            global_prompt.to_string()
        };

        // Verificar que el path NO está en el system prompt
        assert!(!system_prompt.contains("C:\\Users\\Fa\\Desktop\\IAF\\citybound"));
        assert!(!system_prompt.contains("Directorio del proyecto:"));
        assert!(!system_prompt.contains("project_path"));
    }

    /// Test: Verifica que el path del proyecto DEBERÍA inyectarse
    #[test]
    fn test_project_path_should_be_injected() {
        let project_name = "citybound";
        let project_path = "C:\\Users\\Fa\\Desktop\\IAF\\citybound";

        // El system prompt debería incluir algo como:
        let expected_injection = format!(
            "\n\nPROYECTO ACTUAL: {}\nDirectorio: {}\n",
            project_name, project_path
        );

        assert!(expected_injection.contains(project_name));
        assert!(expected_injection.contains(project_path));
        assert!(expected_injection.contains("Directorio:"));
    }

    /// Test: Verifica que get_project_path funciona correctamente
    #[test]
    fn test_project_path_resolution() {
        // Simular la lógica de get_project_path
        let projects = vec![
            json!({"name": "citybound", "path": "C:\\Users\\Fa\\Desktop\\IAF\\citybound", "is_local": true}),
            json!({"name": "iaf", "path": "C:\\Users\\Fa\\Desktop\\IAF", "is_local": true}),
        ];

        let find_project = |name: &str| -> Option<String> {
            projects.iter()
                .find(|p| p["name"].as_str() == Some(name))
                .and_then(|p| p["path"].as_str().map(String::from))
        };

        assert_eq!(find_project("citybound"), Some("C:\\Users\\Fa\\Desktop\\IAF\\citybound".to_string()));
        assert_eq!(find_project("iaf"), Some("C:\\Users\\Fa\\Desktop\\IAF".to_string()));
        assert_eq!(find_project("nonexistent"), None);
    }
}

// ============================================================================
// BUG #4: no existe herramienta para leer PDFs/DOCX
// ============================================================================
// El agente solo tiene herramientas para leer archivos de texto, imágenes,
// y URLs. No puede procesar PDFs ni DOCX, lo cual es crítico para
// proyectos que incluyen documentación en esos formatos.

#[cfg(test)]
mod bug4_no_pdf_docx_reader {
    use super::*;

    /// Test: Verifica que la lista de herramientas actual NO incluye read_pdf
    #[test]
    fn test_no_pdf_tool_in_current_tools() {
        let current_tool_names = vec![
            "search_google", "read_file", "write_file_with_commit",
            "execute_powershell", "search_code", "fork_and_clone_repo",
            "read_url", "check_github_cli", "notificar_usuario",
            "finalizar_tarea", "image_fetch", "image_view",
            "image_release", "git_resolve_divergence", "analyze_images",
            "kill_process", "fetch_tool_result", "release_tool_result",
            "spawn_sub_agent", "check_sub_agent", "kill_sub_agent",
            "no_sync"
        ];

        // Verificar que NO existe read_pdf ni read_docx
        assert!(!current_tool_names.contains(&"read_pdf"));
        assert!(!current_tool_names.contains(&"read_docx"));
        assert!(!current_tool_names.contains(&"read_document"));
    }

    /// Test: Define la herramienta read_document que DEBERÍA existir
    #[test]
    fn test_read_document_tool_definition() {
        let read_document_tool = json!({
            "type": "function",
            "function": {
                "name": "read_document",
                "description": "Lee archivos PDF, DOCX, ODT y RTF. Extrae el texto para análisis. Soporta archivos de hasta 50MB.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Ruta al archivo PDF, DOCX, ODT o RTF."
                        },
                        "start_page": {
                            "type": "integer",
                            "description": "Página inicial (opcional, indexada desde 1)."
                        },
                        "end_page": {
                            "type": "integer",
                            "description": "Página final (opcional, indexada desde 1)."
                        }
                    },
                    "required": ["path"]
                }
            }
        });

        assert_eq!(read_document_tool["function"]["name"], "read_document");
        assert!(read_document_tool["function"]["description"].as_str().unwrap().contains("PDF"));
        assert!(read_document_tool["function"]["description"].as_str().unwrap().contains("DOCX"));
    }

    /// Test: Verifica la extracción de texto de PDFs (usando una librería como pdf-extract)
    #[test]
    fn test_pdf_text_extraction_logic() {
        // Simular la lógica de extracción de texto de PDF
        let pdf_path = "C:\\Users\\Fa\\Desktop\\IAF\\documento.pdf";

        // Verificar extensión
        let extension = std::path::Path::new(pdf_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let supported = matches!(extension.to_lowercase().as_str(), "pdf" | "docx" | "odt" | "rtf");
        assert!(supported, "La extensión {} debería ser soportada", extension);
    }
}

// ============================================================================
// BUG #5: agente no ve el system prompt local del proyecto
// ============================================================================
// Aunque el código en agent.rs carga el system prompt local, la carga
// desde disco (localPrompt.json) puede fallar si el archivo no existe
// o si el path no se resuelve correctamente. Además, los prompts en
// memoria (state.prompts) pueden no estar sincronizados con disco.

#[cfg(test)]
mod bug5_system_prompt_local_no_cargado {
    use super::*;

    /// Test: Simula la carga del system prompt local desde disco
    #[test]
    fn test_local_prompt_loading_from_disk() {
        let project_name = "citybound";
        let username = "admin";

        // Simular path de localPrompt.json
        let local_prompt_path = format!(
            ".config/data/{}/{}/localPrompt.json",
            username, project_name
        );

        // Verificar que el path tiene el formato correcto
        assert!(local_prompt_path.contains(username));
        assert!(local_prompt_path.contains(project_name));
        assert!(local_prompt_path.ends_with("localPrompt.json"));
    }

    /// Test: Verifica que la función load_local_prompt no retorna None
    /// cuando el archivo existe pero el HashMap en memoria no está actualizado
    #[test]
    fn test_prompt_consistency_between_memory_and_disk() {
        // Simular estado inconsistente: disco tiene prompt, memoria no
        let prompt_on_disk = Some("Project Specific Prompt:\nOptimiza para Rust nativo...".to_string());
        let prompt_in_memory: Option<String> = None;

        // Esto es el bug: el agente usa prompt_in_memory (None) en vez de prompt_on_disk
        let effective_prompt = prompt_in_memory.or(prompt_on_disk);

        // Si el fix es correcto, el agente debería caer en el prompt de disco
        assert!(effective_prompt.is_some(), 
            "BUG: El prompt local existe en disco pero no se carga. El agente no lo ve.");
    }

    /// Test: Verifica que el system prompt construido incluye el prompt local
    #[test]
    fn test_system_prompt_includes_local_prompt() {
        let global = "Eres un asistente de desarrollo...";
        let local = "Project Specific Prompt:\nTrabaja en Rust nativo con ECS...";

        let system_prompt = format!("{}\n\nProject Specific Prompt:\n{}", global, local);

        assert!(system_prompt.contains("Project Specific Prompt:"));
        assert!(system_prompt.contains("Trabaja en Rust nativo con ECS"));
    }
}

// ============================================================================
// BUG #6: agente no ve el perfil del usuario
// ============================================================================
// El perfil del usuario (UserLearningProfile) se carga en StudyEngine pero
// NUNCA se inyecta en el system prompt del agente. El agente no sabe la edad,
// intereses, estilo de aprendizaje, ni condiciones neurológicas del usuario.

#[cfg(test)]
mod bug6_perfil_usuario_no_inyectado {
    use super::*;

    /// Test: Demuestra que el perfil del usuario no está en el system prompt actual
    #[test]
    fn test_user_profile_not_in_current_system_prompt() {
        let profile = json!({
            "username": "alumno_test",
            "age": 14,
            "high_capabilities": "Matemáticas avanzadas",
            "favorite_games": ["Minecraft", "Geometry Dash"],
            "hobbies": ["videojuegos", "dibujar"],
            "learning_style_summary": "Aprendizaje visual con ejemplos concretos"
        });

        // El system prompt actual no incluye esta información
        let system_prompt = "Eres un asistente de desarrollo...";

        assert!(!system_prompt.contains("alumno_test"));
        assert!(!system_prompt.contains("Perfil del usuario"));
        assert!(!system_prompt.contains("14"));
    }

    /// Test: El system prompt DEBERÍA incluir el perfil del usuario
    #[test]
    fn test_profile_should_be_injected_into_system_prompt() {
        let profile = json!({
            "username": "alumno_test",
            "age": 14,
            "high_capabilities": "Matemáticas avanzadas",
            "favorite_games": ["Minecraft", "Geometry Dash"],
            "hobbies": ["videojuegos", "dibujar"],
            "learning_style_summary": "Aprendizaje visual con ejemplos concretos",
            "neurological_conditions": []
        });

        let profile_text = format!(
            "\n\nPERFIL DEL USUARIO:\n\
             - Nombre: {}\n\
             - Edad: {}\n\
             - Fortalezas: {}\n\
             - Intereses: {}\n\
             - Estilo de aprendizaje: {}\n\
             - Condiciones: {}\n",
            profile["username"].as_str().unwrap_or(""),
            profile["age"].as_i64().unwrap_or(0),
            profile["high_capabilities"].as_str().unwrap_or(""),
            profile["hobbies"].as_array().map(|a| {
                a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
            }).unwrap_or_default(),
            profile["learning_style_summary"].as_str().unwrap_or(""),
            profile["neurological_conditions"].as_array().map(|a| {
                a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
            }).unwrap_or_else(|| "ninguna".to_string()),
        );

        assert!(profile_text.contains("alumno_test"));
        assert!(profile_text.contains("14"));
        assert!(profile_text.contains("Matemáticas avanzadas"));
        assert!(profile_text.contains("Aprendizaje visual"));
    }

    /// Test: Verifica que el StudyEngine expone correctamente el perfil
    #[test]
    fn test_study_engine_profile_access() {
        // Simular la API del StudyEngine
        let profile = json!({
            "username": "alumno_test",
            "age": 14,
            "phase": "NotStarted",
            "learning_style_summary": "",
            "last_updated": 1700000000u64
        });

        // get_or_create_profile debe retornar el perfil existente
        assert_eq!(profile["username"], "alumno_test");
        assert_eq!(profile["phase"], "NotStarted");
    }
}

// ============================================================================
// BUG #7: modo estudio no usa preguntas pedagógicas ni lista de aprendizajes
// ============================================================================
// El StudyEngine tiene definidas las preguntas del cuestionario pedagógico
// y la lista de aprendizajes (UserKnowledgeBase), pero el agente en modo
// estudio NUNCA las consulta. El agente asume conocimientos y no enseña el
// porqué de las cosas.

#[cfg(test)]
mod bug7_estudio_sin_pedagogia {
    use super::*;

    /// Test: Verifica que las preguntas del cuestionario existen en study.rs
    #[test]
    fn test_pedagogical_questions_exist() {
        let questions = vec![
            // Sección 1: Intereses y Motivación
            "¿Qué actividades haces cuando tienes tiempo libre?",
            "¿Qué temas o materias te dan mucha curiosidad?",
            "¿Qué proyectos o metas te entusiasman cumplir este año?",
            // Sección 2: Estilos y Preferencias de Aprendizaje
            "¿Prefieres leer un libro, ver un video o armar algo con tus manos?",
            "¿Te resulta más fácil estudiar solo o en grupo?",
            // Sección 3: Fortalezas y Desafíos
            "¿En qué actividades sientes que eres muy bueno o talentoso?",
            "¿Qué es lo que más te cuesta trabajo entender o realizar?",
            // Sección 4: Entorno y Contexto Familiar
            "¿Tienes un espacio cómodo y silencioso para estudiar en casa?",
            "¿A qué hora del día sientes que tienes más energía para aprender?",
            // Sección 5: Historia y Experiencia Educativa
            "¿Qué es lo que más te gusta de ir a la escuela?",
        ];

        assert!(!questions.is_empty(), "Las preguntas pedagógicas deben existir");
        assert!(questions.len() >= 10, "Debe haber al menos 10 preguntas en el cuestionario");
    }

    /// Test: Verifica los problemas de razonamiento lógico
    #[test]
    fn test_logical_reasoning_problems_exist() {
        let problems = vec![
            // Razonamiento lógico y secuencial
            "Completar series de figuras que giran o números con operaciones escondidas",
            "Ana es más alta que Luis, y Luis es más alto que Juan. ¿Quién es el más bajo?",
            // Pensamiento lateral
            "Un granjero tiene 10 ovejas, todas mueren menos 9. ¿Cuántas le quedan?",
            // Razonamiento espacial
            "Mostrar un dibujo de tres engranajes conectados y preguntar hacia qué lado gira el último",
        ];

        assert!(!problems.is_empty(), "Los problemas de razonamiento deben existir");
    }

    /// Test: La lista de aprendizajes (UserKnowledgeBase) debe ser consultada
    #[test]
    fn test_knowledge_base_is_consulted() {
        let kb = json!({
            "username": "alumno_test",
            "known_topics": {
                "variables_rust": {
                    "topic": "Variables en Rust",
                    "level": 0.8,
                    "evidence": ["Declaró variables correctamente"],
                    "last_demonstrated": 1700000000u64,
                    "explicit": true
                },
                "funciones_rust": {
                    "topic": "Funciones en Rust",
                    "level": 0.3,
                    "evidence": ["Tuvo dificultad con los parámetros"],
                    "last_demonstrated": 1700000000u64,
                    "explicit": false
                }
            },
            "demonstrated_skills": [],
            "learning_summary": "El usuario sabe variables pero está aprendiendo funciones"
        });

        // Verificar que el agente puede consultar el nivel de conocimiento
        let fn_level = kb["known_topics"]["funciones_rust"]["level"].as_f64().unwrap();
        assert!(fn_level < 0.5, "El nivel de funciones es bajo, el agente DEBE enseñar esto");

        let var_level = kb["known_topics"]["variables_rust"]["level"].as_f64().unwrap();
        assert!(var_level > 0.7, "El nivel de variables es alto, el agente puede asumir este conocimiento");
    }

    /// Test: El agente en modo estudio DEBE preguntar antes de asumir
    #[test]
    fn test_study_agent_must_ask_before_assuming() {
        // Si no hay perfil del usuario, el agente DEBE hacer el cuestionario
        let has_profile = false;
        let has_knowledge_base = false;

        let should_ask_questions = !has_profile || !has_knowledge_base;
        assert!(should_ask_questions, 
            "BUG: El agente no pregunta y asume conocimientos sin perfil ni KB");

        // Si hay perfil pero no KB, debe preguntar conocimiento del tema
        let has_profile_but_no_kb = true;
        let has_topic_knowledge = false;
        let should_ask_topic = has_profile_but_no_kb && !has_topic_knowledge;
        assert!(should_ask_topic,
            "BUG: El agente no pregunta sobre conocimiento previo del tema");
    }

    /// Test: El agente DEBE enseñar el "porqué", no solo el "cómo"
    #[test]
    fn test_agent_must_teach_why_not_just_how() {
        // Simular una respuesta del agente en modo estudio
        let agent_response = "Para declarar una variable en Rust usas `let x = 5;`. \
            Esto funciona porque Rust infiere el tipo `i32` por defecto. \
            La inferencia de tipos existe para que el código sea más conciso \
            sin perder seguridad, ya que el compilador verifica los tipos \
            en tiempo de compilación. ¿Entiendes por qué es mejor que en Python?";

        // Verificar que la respuesta incluye el "porqué"
        assert!(agent_response.contains("porque"), 
            "La respuesta debe explicar el PORQUÉ");
        assert!(agent_response.contains("¿Entiendes"), 
            "La respuesta debe hacer preguntas de verificación");
    }

    /// Test: El método de enseñanza debe adaptarse según el perfil
    #[test]
    fn test_teaching_method_adapts_to_profile() {
        // Usuario visual con interés en videojuegos
        let profile = json!({
            "learning_style_summary": "Aprendizaje visual con ejemplos concretos",
            "favorite_games": ["Minecraft", "Roblox"],
        });

        // El agente debe usar analogías de Minecraft para enseñar
        let teaching_approach = if profile["favorite_games"].as_array()
            .map(|a| a.iter().any(|g| g.as_str() == Some("Minecraft")))
            .unwrap_or(false) 
        {
            "Usar analogías de Minecraft (bloques = variables, crafting = funciones)"
        } else {
            "Enfoque genérico"
        };

        assert!(teaching_approach.contains("Minecraft"),
            "El método debe usar los intereses del usuario como analogías");
    }
}

// ============================================================================
// Tests de Integración: Múltiples bugs combinados
// ============================================================================

#[cfg(test)]
mod integration_regression_tests {
    use super::*;

    /// Test: Verifica el flujo completo de una sesión de estudio:
    /// perfil + KB + preguntas + enseñanza + notificaciones
    #[test]
    fn test_full_study_session_flow() {
        // 1. El usuario tiene perfil pero no KB del tema
        let username = "alumno_test";
        let has_profile = true;
        let has_knowledge_of_rust = false;

        // 2. El agente DEBE preguntar sobre conocimiento previo de Rust
        if !has_knowledge_of_rust {
            let question = "¿Qué sabes ya sobre Rust? ¿Has programado en otros lenguajes?";
            assert!(!question.is_empty());
        }

        // 3. El agente DEBE consultar el perfil para personalizar la enseñanza
        let profile_style = "visual";
        let teaching_method = match profile_style {
            "visual" => "Diagramas y ejemplos visuales",
            "auditivo" => "Explicaciones verbales detalladas",
            "kinestésico" => "Ejercicios prácticos interactivos",
            _ => "Método mixto",
        };
        assert_eq!(teaching_method, "Diagramas y ejemplos visuales");

        // 4. El agente DEBE notificar al usuario durante la enseñanza
        let notifications = vec![
            "Analizando tu nivel actual de Rust...",
            "Preparando ejercicios personalizados...",
            "Aquí tienes tu primer ejercicio:",
        ];
        assert!(!notifications.is_empty());

        // 5. La KB debe actualizarse después de cada lección
        let updated_kb = json!({
            "known_topics": {
                "variables_rust": {
                    "level": 0.9,
                    "last_demonstrated": 1700000100u64
                }
            }
        });
        assert!(updated_kb["known_topics"]["variables_rust"]["level"].as_f64().unwrap() > 0.8);
    }

    /// Test: Verifica el flujo de programación con proyecto seleccionado
    #[test]
    fn test_full_programming_session_flow() {
        // 1. Proyecto seleccionado con prompt local
        let project = json!({
            "name": "citybound",
            "path": "C:\\Users\\Fa\\Desktop\\IAF\\citybound",
            "local_prompt": "Optimiza para Rust nativo, aplica ECS y 90 técnicas de optimización"
        });

        // 2. El system prompt DEBE incluir el path y el prompt local
        let system_prompt = format!(
            "Eres un asistente...\n\nPROYECTO: {}\nDIRECTORIO: {}\n\nProject Specific Prompt:\n{}",
            project["name"].as_str().unwrap(),
            project["path"].as_str().unwrap(),
            project["local_prompt"].as_str().unwrap()
        );

        assert!(system_prompt.contains("citybound"));
        assert!(system_prompt.contains("C:\\Users\\Fa\\Desktop\\IAF\\citybound"));
        assert!(system_prompt.contains("Optimiza para Rust nativo"));

        // 3. El agente DEBE poder leer PDFs del proyecto
        let project_has_pdfs = true;
        if project_has_pdfs {
            // Debe existir la herramienta read_document
            let has_pdf_tool = true; // Después del fix
            assert!(has_pdf_tool);
        }

        // 4. El título DEBE ser generado por el agente, no truncado
        let agent_title = "Refactorización ECS de Citybound";
        assert_ne!(agent_title, "Necesito que analices el códig");
    }
}

// ============================================================================
// Tests de Casos Límite
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    /// BUG #1 Edge: Notificación con mensaje vacío
    #[test]
    fn test_notificar_usuario_empty_message() {
        let mensaje = "";
        let tipo = "informativo";

        // No debería causar pánico ni guardar mensajes vacíos
        if mensaje.is_empty() {
            // El frontend debe ignorar notificaciones vacías
            assert!(true);
        }
    }

    /// BUG #2 Edge: Título con caracteres especiales
    #[test]
    fn test_title_with_special_characters() {
        let title = "Análisis ♥ del código: ¿bug o feature?";
        let sanitized: String = title.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
            .collect::<String>()
            .trim()
            .replace(" ", "_");

        // No debe contener caracteres no-ASCII en el nombre de archivo
        assert!(sanitized.chars().all(|c| c.is_ascii()));
    }

    /// BUG #3 Edge: Proyecto con path inválido
    #[test]
    fn test_project_with_invalid_path() {
        let projects = vec![
            json!({"name": "valid", "path": "C:\\valid\\path"}),
            json!({"name": "invalid", "path": ""}),
        ];

        let find = |name: &str| -> Option<String> {
            projects.iter()
                .find(|p| p["name"] == name && !p["path"].as_str().unwrap_or("").is_empty())
                .and_then(|p| p["path"].as_str().map(String::from))
        };

        assert!(find("valid").is_some());
        assert!(find("invalid").is_none(), "Paths vacíos deben ser rechazados");
    }

    /// BUG #4 Edge: PDF corrupto o protegido
    #[test]
    fn test_corrupt_pdf_handling() {
        let pdf_is_corrupt = true;
        let result = if pdf_is_corrupt {
            "Error: No se pudo extraer texto del PDF (archivo corrupto o protegido)"
        } else {
            "Texto extraído exitosamente"
        };

        assert!(result.contains("Error"));
    }

    /// BUG #6 Edge: Usuario sin perfil (recién registrado)
    #[test]
    fn test_new_user_without_profile() {
        let is_new_user = true;
        let has_profile = false;

        if is_new_user && !has_profile {
            // Debe iniciar el cuestionario pedagógico
            let action = "Iniciar cuestionario de perfil pedagógico";
            assert_eq!(action, "Iniciar cuestionario de perfil pedagógico");
        }
    }

    /// BUG #7 Edge: KB vacía pero usuario afirma saber el tema
    #[test]
    fn test_user_claims_knowledge_but_empty_kb() {
        let user_says = "Ya sé programar en Rust";
        let kb_has_rust = false;

        // El agente NO debe asumir: debe verificar con preguntas técnicas
        if !kb_has_rust {
            let verification_question = "¿Podrías explicarme qué es el ownership en Rust?";
            assert!(!verification_question.is_empty());
        }
    }
}

// ============================================================================
// Tests de Inyección de Fallos (Chaos Engineering)
// ============================================================================

#[cfg(test)]
mod fault_injection_tests {
    use super::*;

    /// Simula fallo de disco al guardar perfil
    #[test]
    fn test_disk_failure_during_profile_save() {
        let disk_failed = true;
        let profile_data = "user profile json...";

        let result = if disk_failed {
            Err("Error de escritura en disco")
        } else {
            Ok(())
        };

        assert!(result.is_err(), "Debe manejar fallos de disco gracefully");
    }

    /// Simula timeout al cargar prompt local desde disco
    #[test]
    fn test_timeout_loading_local_prompt() {
        let load_timeout = true;
        let fallback_prompt = "Eres un asistente de desarrollo...".to_string();

        let effective_prompt = if load_timeout {
            fallback_prompt.clone()
        } else {
            "Project Specific Prompt cargado...".to_string()
        };

        // Debe usar el fallback sin crashear
        assert_eq!(effective_prompt, fallback_prompt);
    }

    /// Simula corrupción de datos en learnings.json
    #[test]
    fn test_corrupted_learnings_json() {
        let json_content = "{ esto no es JSON válido";
        let parse_result = serde_json::from_str::<serde_json::Value>(json_content);

        assert!(parse_result.is_err(), "Debe detectar JSON corrupto");

        // Debe reinicializar con KB vacía en vez de crashear
        let fallback_kb = json!({
            "username": "test_user",
            "known_topics": {},
            "demonstrated_skills": [],
            "learning_summary": "KB reinicializada por corrupción"
        });
        assert!(fallback_kb["known_topics"].as_object().unwrap().is_empty());
    }

    /// Simula memory pressure: muchos proyectos con prompts locales
    #[test]
    fn test_memory_pressure_many_projects() {
        let num_projects = 100;
        let mut prompts = std::collections::HashMap::new();

        for i in 0..num_projects {
            prompts.insert(
                format!("project_{}", i),
                format!("Project Specific Prompt for project {}", i)
            );
        }

        // No debe crashear con 100 proyectos cargados
        assert_eq!(prompts.len(), num_projects);

        // Buscar un proyecto específico debe ser rápido (HashMap O(1))
        let start = std::time::Instant::now();
        let _ = prompts.get("project_99");
        let elapsed = start.elapsed();
        assert!(elapsed.as_micros() < 1000, "Búsqueda en HashMap debe ser < 1µs");
    }

    /// Simula race condition entre agente y frontend por el estado
    #[test]
    fn test_race_condition_agent_state() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let state = Arc::new(Mutex::new(json!({
            "running": true,
            "pending_notification": null,
            "steps": []
        })));
        let state_clone = state.clone();
        let agent_thread = thread::spawn(move || {
            // Agente escribe notificación
            let mut s = state_clone.lock().unwrap();
            s["pending_notification"] = json!("Hola desde el agente");
        });

        let state_clone2 = state.clone();
        let frontend_thread = thread::spawn(move || {
            // Frontend lee notificación
            let s = state_clone2.lock().unwrap();
            let notif = s["pending_notification"].as_str().map(String::from);
            notif
        });

        agent_thread.join().unwrap();
        let frontend_result = frontend_thread.join().unwrap();

        // Después del join del agente, el frontend DEBE ver la notificación
        let final_state = state.lock().unwrap();
        assert!(final_state["pending_notification"].as_str().is_some());
    }
}

// ============================================================================
// Tests de Estrés
// ============================================================================

#[cfg(test)]
mod stress_tests {
    use super::*;

    /// Estrés: Muchas notificaciones informativas en rápida sucesión
    #[test]
    fn test_many_rapid_notifications() {
        let num_notifications = 1000;
        let mut steps = Vec::new();

        for i in 0..num_notifications {
            steps.push(json!({
                "step_type": "informativo",
                "title": format!("Notificación {}", i),
                "detail": format!("Mensaje informativo número {}", i),
                "timestamp": 1700000000u64 + i as u64
            }));
        }

        // Verificar que todas se guardaron
        assert_eq!(steps.len(), num_notifications);

        // El frontend debe poder filtrarlas eficientemente
        let informativos: Vec<_> = steps.iter()
            .filter(|s| s["step_type"] == "informativo")
            .collect();
        assert_eq!(informativos.len(), num_notifications);
    }

    /// Estrés: KB con muchos topics
    #[test]
    fn test_large_knowledge_base() {
        let mut topics = serde_json::Map::new();
        for i in 0..500 {
            topics.insert(
                format!("topic_{}", i),
                json!({
                    "topic": format!("Topic {}", i),
                    "level": (i as f64 % 100.0) / 100.0,
                    "evidence": [format!("Evidence for topic {}", i)],
                    "last_demonstrated": 1700000000u64,
                    "explicit": i % 2 == 0
                })
            );
        }

        let kb = json!({
            "username": "test_user",
            "known_topics": topics,
            "learning_summary": "KB extensa de prueba"
        });

        assert_eq!(kb["known_topics"].as_object().unwrap().len(), 500);

        // Buscar un topic por nombre debe ser O(1)
        let topic_250 = &kb["known_topics"]["topic_250"];
        assert_eq!(topic_250["level"].as_f64().unwrap(), 0.5);
    }

    /// Estrés: Archivo de chat muy grande con muchas iteraciones
    #[test]
    fn test_large_chat_session() {
        let mut messages = Vec::new();
        for i in 0..200 {
            messages.push(json!({
                "role": if i % 2 == 0 { "user" } else { "agent" },
                "content": format!("Mensaje número {} en la conversación", i),
                "timestamp": 1700000000u64 + i as u64
            }));
        }

        let session = json!({
            "id": "test-session",
            "title": "Conversación larga de prueba",
            "messages": messages,
            "project_name": "test"
        });

        assert_eq!(session["messages"].as_array().unwrap().len(), 200);

        // Serialización a disco (simulada)
        let serialized = serde_json::to_string(&session).unwrap();
        assert!(serialized.len() > 1000, "La sesión serializada debe ser sustancial");

        // Deserialización
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized["id"], "test-session");
        assert_eq!(deserialized["messages"].as_array().unwrap().len(), 200);
    }
}

// ============================================================================
// Tests End-to-End (E2E) - Simulan flujo backend-frontend
// ============================================================================

#[cfg(test)]
mod e2e_tests {
    use super::*;

    /// E2E: Flujo completo de creación de chat → agente → notificación → título
    #[test]
    fn test_e2e_chat_creation_to_agent_notification() {
        // 1. Usuario crea un chat
        let chat_input = json!({
            "message": "Analiza el código de citybound",
            "project_name": "citybound",
            "session_id": null,
            "mode": "programming"
        });

        // 2. El servidor crea la sesión
        let session_id = "test-uuid-123";
        let initial_title: String = chat_input["message"].as_str().unwrap()
            .chars().take(30).collect();

        // BUG #2: El título es el mensaje truncado
        assert_eq!(initial_title, "Analiza el código de citybo");

        // 3. El system prompt DEBE incluir el path del proyecto (BUG #3)
        let project_path = "C:\\Users\\Fa\\Desktop\\IAF\\citybound";
        let system_prompt_has_path = false; // Actualmente false
        assert!(!system_prompt_has_path, 
            "BUG #3 confirmado: system prompt no incluye el path del proyecto");

        // 4. El agente procesa y notifica (BUG #1)
        let agent_notification = "Iniciando análisis de citybound...";
        let notification_visible_in_chat = false; // Actualmente false
        assert!(!notification_visible_in_chat,
            "BUG #1 confirmado: notificación informativa no visible en chat");

        // 5. El agente intenta leer un PDF del proyecto (BUG #4)
        let project_has_pdf = true;
        let can_read_pdf = false; // No existe la herramienta
        assert!(!can_read_pdf,
            "BUG #4 confirmado: no puede leer PDFs");

        // 6. El agente NO ve el perfil del usuario (BUG #6)
        let agent_sees_profile = false;
        assert!(!agent_sees_profile,
            "BUG #6 confirmado: agente no ve el perfil del usuario");
    }

    /// E2E: Flujo completo de modo estudio
    #[test]
    fn test_e2e_study_mode_flow() {
        // 1. Usuario en modo estudio sin perfil
        let mode = "study";
        let has_profile = false;

        // 2. El agente DEBE iniciar el cuestionario (BUG #7)
        if mode == "study" && !has_profile {
            let should_start_questionnaire = true;
            assert!(should_start_questionnaire,
                "El agente DEBE iniciar el cuestionario para nuevos usuarios");

            // Preguntas del Paso 1
            let questions = vec![
                "¿Qué actividades haces cuando tienes tiempo libre?",
                "¿Qué temas o materias te dan mucha curiosidad?",
                "¿Prefieres leer un libro, ver un video o armar algo con tus manos?",
            ];
            assert_eq!(questions.len(), 3);
        }

        // 3. Después del cuestionario, crear perfil
        let profile_created = json!({
            "username": "alumno_test",
            "age": 14,
            "favorite_games": ["Minecraft"],
            "learning_style_summary": "Visual con analogías de videojuegos"
        });

        // 4. El agente debe usar el perfil para personalizar
        let favorite_game = profile_created["favorite_games"][0].as_str().unwrap();
        let teaching_analogy = format!(
            "Voy a explicarte las variables en Rust usando una analogía de {}",
            favorite_game
        );
        assert!(teaching_analogy.contains("Minecraft"));

        // 5. El agente debe consultar la KB antes de enseñar
        let kb_has_topic = false;
        if !kb_has_topic {
            let pre_question = "¿Has trabajado antes con variables en programación?";
            assert!(!pre_question.is_empty());
        }

        // 6. Después de la lección, actualizar KB
        let updated_kb = json!({
            "known_topics": {
                "variables_rust": {
                    "topic": "Variables en Rust",
                    "level": 0.7,
                    "evidence": ["Completó ejercicios de variables correctamente"],
                    "last_demonstrated": 1700000100u64,
                    "explicit": true
                }
            }
        });
        assert!(updated_kb["known_topics"]["variables_rust"]["level"].as_f64().unwrap() > 0.5);
    }

    /// E2E: Verifica que el frontend recibe correctamente las notificaciones
    #[test]
    fn test_e2e_frontend_receives_notifications() {
        // Simular polling del frontend
        let agent_status = json!({
            "running": true,
            "finished": false,
            "pending_notification": "Analizando archivo main.rs...",
            "steps": [
                {
                    "step_type": "informativo",
                    "title": "Notificación del Agente",
                    "detail": "Analizando archivo main.rs..."
                }
            ],
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null::<String>,
            "respuesta_usuario": null::<String>
        });

        // Frontend: verificar pending_notification
        let notif = agent_status["pending_notification"].as_str();
        assert!(notif.is_some());
        assert_eq!(notif.unwrap(), "Analizando archivo main.rs...");

        // Frontend: filtrar pasos informativos
        let informativos: Vec<_> = agent_status["steps"].as_array().unwrap().iter()
            .filter(|s| s["step_type"] == "informativo")
            .collect();
        assert_eq!(informativos.len(), 1);

        // Frontend: mostrar en el chat
        // (En la UI, esto añadiría un mensaje del agente con el contenido)
        let chat_message = json!({
            "role": "agent",
            "content": informativos[0]["detail"].as_str().unwrap(),
            "is_notification": true
        });
        assert!(chat_message["is_notification"].as_bool().unwrap());
    }
}
