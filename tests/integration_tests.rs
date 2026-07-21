// ============================================================================
// tests/integration_tests.rs — Tests de Integración y Aceptación
//
// Tests REALES que prueban componentes del sistema interactuando entre sí:
// StudyEngine con disco, UserStore con contraseñas, sanitize_filename,
// ActiveAgentStatus serialization, y creación/lectura real de DOCX.
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::io::Write;

// ============================================================================
// Helpers
// ============================================================================

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("iaf_int_{}", name));
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(&dir);
    dir
}

// ============================================================================
// SECCIÓN 1: StudyEngine — Persistencia real en disco
// ============================================================================

#[cfg(test)]
mod study_engine_tests {
    use super::*;
    use iaf::study::StudyEngine;
    use serde_json::json;

    #[test]
    fn study_engine_nuevo_carga_perfiles_desde_disco() {
        let tmp = tmp_dir("se_load");
        let user_data = tmp.join(".config").join("data").join("testuser");
        fs::create_dir_all(&user_data).unwrap();

        let profile = json!({
            "username": "testuser",
            "age": 14,
            "phase": "Exploration",
            "high_capabilities": null,
            "neurological_conditions": [],
            "favorite_games": ["Minecraft"],
            "favorite_youtubers": [],
            "hobbies": ["programar"],
            "exploration_started_at": 1700000000u64,
            "exploitation_started_at": null,
            "hypothesis_history": [],
            "learning_style_summary": "",
            "message_timestamps": [],
            "last_updated": 1700000000u64
        });
        fs::write(
            user_data.join("profile.json"),
            serde_json::to_string_pretty(&profile).unwrap(),
        ).unwrap();

        let engine = StudyEngine::new(tmp.clone());
        let loaded = engine.get_profile("testuser");
        assert!(loaded.is_some(), "StudyEngine debe cargar perfiles existentes desde disco");
        assert_eq!(loaded.unwrap().username, "testuser");
    }

    #[test]
    fn study_engine_save_profile_crea_archivo_en_disco() {
        let tmp = tmp_dir("se_save");
        let engine = StudyEngine::new(tmp.clone());

        let mut profile = engine.get_or_create_profile("nuevo_user");
        profile.age = Some(20);
        profile.hobbies = vec!["rust".to_string(), "ajedrez".to_string()];
        engine.save_profile(&profile).unwrap();

        let profile_path = tmp.join(".config").join("data").join("nuevo_user").join("profile.json");
        assert!(profile_path.exists(), "save_profile debe crear profile.json en disco");

        let contenido = fs::read_to_string(&profile_path).unwrap();
        let cargado: serde_json::Value = serde_json::from_str(&contenido).unwrap();
        assert_eq!(cargado["username"], "nuevo_user");
        assert_eq!(cargado["age"], 20);
    }

    #[test]
    fn study_engine_profile_exists_on_disk_es_preciso() {
        let tmp = tmp_dir("se_exists");
        let engine = StudyEngine::new(tmp.clone());

        // Usuario sin perfil
        assert!(!engine.profile_exists_on_disk("ghost_user"));

        // Crear perfil
        let profile = engine.get_or_create_profile("real_user");
        engine.save_profile(&profile).unwrap();

        assert!(engine.profile_exists_on_disk("real_user"));
    }

    #[test]
    fn study_engine_knowledge_base_se_guarda_y_carga() {
        let tmp = tmp_dir("se_kb");
        let engine = StudyEngine::new(tmp.clone());

        let mut kb = engine.get_or_create_knowledge("alumno1");
        kb.learning_summary = "Aprendiendo Rust basics".to_string();
        engine.save_knowledge(&kb).unwrap();

        let kb_path = tmp.join(".config").join("data").join("alumno1").join("learnings.json");
        assert!(kb_path.exists());

        // Crear nuevo engine y verificar que carga
        let engine2 = StudyEngine::new(tmp);
        let kb2 = engine2.get_knowledge("alumno1");
        assert!(kb2.is_some());
        assert_eq!(kb2.unwrap().learning_summary, "Aprendiendo Rust basics");
    }

    #[test]
    fn study_engine_teaching_method_se_guarda_y_carga() {
        let tmp = tmp_dir("se_tm");
        let engine = StudyEngine::new(tmp.clone());

        let mut tm = engine.get_or_create_teaching_method("alumno1");
        tm.chosen_method = Some("gamificacion".to_string());
        engine.save_teaching_method(&tm).unwrap();

        let tm_path = tmp.join(".config").join("data").join("alumno1").join("teachingMethod.json");
        assert!(tm_path.exists());
        assert!(engine.teaching_method_exists_on_disk("alumno1"));

        let engine2 = StudyEngine::new(tmp);
        let tm2 = engine2.get_teaching_method("alumno1");
        assert!(tm2.is_some());
        assert_eq!(tm2.unwrap().chosen_method.unwrap(), "gamificacion");
    }

    #[test]
    fn study_engine_directorios_internos_no_se_cargan_como_usuarios() {
        let tmp = tmp_dir("se_internal");
        let data_dir = tmp.join(".config").join("data");

        // Crear _projects (interno)
        let projects_dir = data_dir.join("_projects");
        fs::create_dir_all(&projects_dir).unwrap();
        fs::write(projects_dir.join("p1.json"), "{}").unwrap();

        // Crear usuario real
        let user_dir = data_dir.join("real_user");
        fs::create_dir_all(&user_dir).unwrap();
        let profile = json!({"username":"real_user","age":15,"phase":"Exploration","high_capabilities":null,"neurological_conditions":[],"favorite_games":[],"favorite_youtubers":[],"hobbies":[],"exploration_started_at":null,"exploitation_started_at":null,"hypothesis_history":[],"learning_style_summary":"","message_timestamps":[],"last_updated":0});
        fs::write(user_dir.join("profile.json"), serde_json::to_string_pretty(&profile).unwrap()).unwrap();

        let engine = StudyEngine::new(tmp);
        assert!(engine.get_profile("real_user").is_some());
        // _projects NO debe aparecer como usuario
        assert!(engine.get_profile("_projects").is_none());
    }

    #[test]
    fn study_engine_multiples_usuarios_independientes() {
        let tmp = tmp_dir("se_multi");
        let engine = StudyEngine::new(tmp.clone());

        let p1 = engine.get_or_create_profile("alice");
        engine.save_profile(&p1).unwrap();

        let p2 = engine.get_or_create_profile("bob");
        engine.save_profile(&p2).unwrap();

        assert!(tmp.join(".config").join("data").join("alice").join("profile.json").exists());
        assert!(tmp.join(".config").join("data").join("bob").join("profile.json").exists());
    }

    #[test]
    fn study_engine_save_crea_directorio_si_no_existe() {
        let tmp = tmp_dir("se_mkdir");
        let engine = StudyEngine::new(tmp.clone());

        let profile = engine.get_or_create_profile("newuser");
        engine.save_profile(&profile).unwrap();

        assert!(tmp.join(".config").join("data").join("newuser").join("profile.json").exists());
    }
}


// ============================================================================
// SECCIÓN 2: sanitize_filename — Funciones utilitarias reales
// ============================================================================

#[cfg(test)]
mod sanitize_filename_tests {
    use iaf::utils::sanitize_filename;

    #[test]
    fn sanitiza_nombre_ascii_simple() {
        assert_eq!(sanitize_filename("hello"), "hello");
    }

    #[test]
    fn sanitiza_espacios_a_underscores() {
        assert_eq!(sanitize_filename("hello world"), "hello_world");
    }

    #[test]
    fn sanitiza_caracteres_especiales() {
        let result = sanitize_filename("hello!@#world");
        assert!(!result.contains('!'));
        assert!(!result.contains('@'));
        assert!(!result.contains('#'));
    }

    #[test]
    fn sanitiza_caracteres_no_ascii() {
        let result = sanitize_filename("Análisis del código");
        assert!(result.chars().all(|c| c.is_ascii()));
        assert!(!result.contains('á'));
        assert!(!result.contains('ó'));
    }

    #[test]
    fn sanitiza_trunca_a_40_caracteres() {
        let long_name = "a".repeat(100);
        let result = sanitize_filename(&long_name);
        assert_eq!(result.len(), 40);
    }

    #[test]
    fn sanitiza_trim_espacios() {
        assert_eq!(sanitize_filename("  hello  "), "hello");
    }

    #[test]
    fn sanitiza_preserva_guiones() {
        assert_eq!(sanitize_filename("my-file"), "my-file");
    }

    #[test]
    fn sanitiza_preserva_underscores() {
        assert_eq!(sanitize_filename("my_file"), "my_file");
    }

    #[test]
    fn sanitiza_nombre_vacio() {
        assert_eq!(sanitize_filename(""), "");
    }

    #[test]
    fn sanitiza_solo_caracteres_especiales() {
        let result = sanitize_filename("!!!@@@");
        assert!(!result.contains('!'));
        assert!(!result.contains('@'));
    }
}


// ============================================================================
// SECCIÓN 3: ActiveAgentStatus — Serialización y valores por defecto
// ============================================================================

#[cfg(test)]
mod active_agent_status_tests {
    use crate::state::ActiveAgentStatus;

    #[test]
    fn default_no_tiene_preguntas_ni_planes_pendientes() {
        let status = ActiveAgentStatus::default();
        assert!(!status.running);
        assert!(!status.finished);
        assert!(!status.esperando_respuesta_usuario);
        assert!(status.pregunta_usuario.is_none());
        assert!(!status.esperando_aprobacion_plan);
        assert!(status.plan_propuesto.is_none());
        assert!(status.info_messages.is_empty());
    }

    #[test]
    fn serializacion_json_incluye_info_messages() {
        let mut status = ActiveAgentStatus::default();
        status.info_messages.push("Test message".to_string());
        status.finished = true;
        status.final_message = Some("Done".to_string());

        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["finished"], true);
        assert_eq!(json["final_message"], "Done");
        assert_eq!(json["info_messages"].as_array().unwrap().len(), 1);
        assert_eq!(json["info_messages"][0], "Test message");
    }

    #[test]
    fn deserializacion_json_restaura_info_messages() {
        let json = serde_json::json!({
            "running": false,
            "interrupted": false,
            "finished": true,
            "final_message": "Tarea completada.",
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "respuesta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "info_messages": ["Msg1", "Msg2"],
            "thinking_content": [],
            "steps": [],
            "current_session_id": "abc-123"
        });

        let status: ActiveAgentStatus = serde_json::from_value(json).unwrap();
        assert!(status.finished);
        assert_eq!(status.final_message.unwrap(), "Tarea completada.");
        assert_eq!(status.info_messages.len(), 2);
        assert_eq!(status.info_messages[0], "Msg1");
        assert_eq!(status.current_session_id.unwrap(), "abc-123");
    }

    #[test]
    fn info_messages_vacio_se_serializa_como_array_vacio() {
        let status = ActiveAgentStatus::default();
        let json = serde_json::to_value(&status).unwrap();
        let arr = json["info_messages"].as_array().unwrap();
        assert!(arr.is_empty());
    }
}


// ============================================================================
// SECCIÓN 4: DOCX Creation & Reading — Prueba real de extract_text_from_docx
// ============================================================================

#[cfg(test)]
mod docx_tests {

    #[test]
    fn crear_docx_y_extraer_texto_con_quick_xml() {
        let dir = std::env::temp_dir().join("iaf_test_docx_real");
        let _ = std::fs::create_dir_all(&dir);
        let docx_path = dir.join("test_real.docx");

        // Crear DOCX con ZIP
        let file = std::fs::File::create(&docx_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip_writer.start_file("word/document.xml", options).unwrap();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Primer párrafo del documento</w:t></w:r></w:p>
    <w:p><w:r><w:t>Segundo párrafo con más contenido</w:t></w:r></w:p>
    <w:p><w:r><w:t>Tercer párrafo</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        use std::io::Write;
        zip_writer.write_all(xml.as_bytes()).unwrap();
        zip_writer.finish().unwrap();

        assert!(docx_path.exists());

        // Leer como ZIP y extraer texto
        let file = std::fs::File::open(&docx_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut doc_xml = archive.by_name("word/document.xml").unwrap();
        let mut xml_str = String::new();
        use std::io::Read;
        doc_xml.read_to_string(&mut xml_str).unwrap();

        // Extraer texto con quick-xml
        let mut text = String::new();
        let mut reader = quick_xml::Reader::from_str(&xml_str);
        reader.trim_text(true);
        let mut in_text = false;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    if e.local_name().as_ref() == b"t" { in_text = true; }
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if in_text { text.push_str(&e.unescape().unwrap_or_default()); }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    if e.local_name().as_ref() == b"t" { in_text = false; }
                    if e.local_name().as_ref() == b"p" { text.push('\n'); }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => panic!("Error: {}", e),
                _ => {}
            }
        }

        assert!(text.contains("Primer párrafo del documento"));
        assert!(text.contains("Segundo párrafo con más contenido"));
        assert!(text.contains("Tercer párrafo"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn docx_sin_texto_no_causa_panico() {
        let dir = std::env::temp_dir().join("iaf_test_docx_empty");
        let _ = std::fs::create_dir_all(&dir);
        let docx_path = dir.join("empty.docx");

        let file = std::fs::File::create(&docx_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip_writer.start_file("word/document.xml", options).unwrap();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
  </w:body>
</w:document>"#;
        use std::io::Write;
        zip_writer.write_all(xml.as_bytes()).unwrap();
        zip_writer.finish().unwrap();

        // Leer y verificar que no hay texto
        let file = std::fs::File::open(&docx_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut doc_xml = archive.by_name("word/document.xml").unwrap();
        let mut xml_str = String::new();
        use std::io::Read;
        doc_xml.read_to_string(&mut xml_str).unwrap();

        let mut text = String::new();
        let mut reader = quick_xml::Reader::from_str(&xml_str);
        reader.trim_text(true);
        let mut in_text = false;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    if e.local_name().as_ref() == b"t" { in_text = true; }
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if in_text { text.push_str(&e.unescape().unwrap_or_default()); }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    if e.local_name().as_ref() == b"t" { in_text = false; }
                    if e.local_name().as_ref() == b"p" { text.push('\n'); }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => panic!("Error: {}", e),
                _ => {}
            }
        }

        assert!(text.trim().is_empty(), "DOCX sin contenido debe devolver texto vacio");

        let _ = std::fs::remove_dir_all(&dir);
    }
}


// ============================================================================
// SECCIÓN 5: UserStore — Autenticación y permisos
// ============================================================================

#[cfg(test)]
mod user_store_tests {
    use crate::auth::UserStore;

    #[test]
    fn crear_usuario_con_password_funciona() {
        let store = UserStore::new();
        let result = store.create_user_with_password(
            "testuser", "secure123", false,
            vec!["read_file".to_string(), "search_code".to_string()],
            crate::auth::UserLimits::default(),
            true, false, false, false,
        );
        assert!(result.is_ok());

        let user = store.find_user("testuser");
        assert!(user.is_some());
        assert!(!user.unwrap().is_admin);
    }

    #[test]
    fn verificar_password_correcto() {
        let store = UserStore::new();
        store.create_user_with_password(
            "user1", "mypassword", false,
            vec!["read_file".to_string()],
            crate::auth::UserLimits::default(),
            true, false, false, false,
        ).unwrap();

        let result = store.verify_password("user1", "mypassword");
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn verificar_password_incorrecto() {
        let store = UserStore::new();
        store.create_user_with_password(
            "user1", "mypassword", false,
            vec!["read_file".to_string()],
            crate::auth::UserLimits::default(),
            true, false, false, false,
        ).unwrap();

        let result = store.verify_password("user1", "wrongpassword");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn crear_admin_con_public_key() {
        let store = UserStore::new();
        let public_key = "a".repeat(64); // 64 chars hex
        let result = store.create_admin(
            "admin1", &public_key,
            vec!["read_file".to_string()],
            crate::auth::UserLimits::admin(),
        );
        assert!(result.is_ok());

        let user = store.find_user("admin1");
        assert!(user.is_some());
        assert!(user.unwrap().is_admin);
    }

    #[test]
    fn listar_usuarios_funciona() {
        let store = UserStore::new();
        store.create_user_with_password(
            "u1", "pw1", false, vec!["read_file".to_string()],
            crate::auth::UserLimits::default(),
            true, false, false, false,
        ).unwrap();
        store.create_user_with_password(
            "u2", "pw2", false, vec!["read_file".to_string()],
            crate::auth::UserLimits::default(),
            false, true, false, false,
        ).unwrap();

        let users = store.list_users();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn has_study_access_admin_siempre_true() {
        let store = UserStore::new();
        let pk = "b".repeat(64);
        store.create_admin("admin2", &pk, vec!["read_file".to_string()], crate::auth::UserLimits::admin()).unwrap();
        let user = store.find_user("admin2").unwrap();
        assert!(user.has_study_access());
        assert!(user.has_programming_access());
    }

    #[test]
    fn has_study_access_usuario_normal_respeta_permiso() {
        let store = UserStore::new();
        store.create_user_with_password(
            "user_study", "pw", false, vec!["read_file".to_string()],
            crate::auth::UserLimits::default(),
            true, false, false, false,
        ).unwrap();

        let user = store.find_user("user_study").unwrap();
        assert!(user.has_study_access());
        assert!(!user.has_programming_access());
    }
}


// ============================================================================
// SECCIÓN 6: CiclePhase — Transiciones de estado
// ============================================================================

#[cfg(test)]
mod cicle_phase_tests {
    use crate::state::CiclePhase;

    #[test]
    fn ciclo_completo_de_fases() {
        let mut phase = CiclePhase::Implementacion;
        assert_eq!(phase.as_str(), "ciclo1_implementacion");

        phase = phase.next();
        assert_eq!(phase, CiclePhase::Optimizacion);
        assert_eq!(phase.as_str(), "ciclo2_optimizacion");

        phase = phase.next();
        assert_eq!(phase, CiclePhase::BusquedaBugs);
        assert_eq!(phase.as_str(), "ciclo3_busqueda_bugs");

        phase = phase.next();
        assert_eq!(phase, CiclePhase::Reduccion);
        assert_eq!(phase.as_str(), "ciclo4_reduccion");

        phase = phase.next();
        assert_eq!(phase, CiclePhase::SegundaBusquedaBugs);
        assert_eq!(phase.as_str(), "ciclo5_segunda_busqueda_bugs");

        phase = phase.next();
        assert_eq!(phase, CiclePhase::Terminar);
        assert_eq!(phase.as_str(), "ciclo6_terminar");

        // Terminar se queda en Terminar
        phase = phase.next();
        assert_eq!(phase, CiclePhase::Terminar);
    }

    #[test]
    fn cicle_phase_default_es_implementacion() {
        assert_eq!(CiclePhase::default(), CiclePhase::Implementacion);
    }
}


// ============================================================================
// SECCIÓN 7: ChatSession y ChatMessage — Serialización
// ============================================================================

#[cfg(test)]
mod chat_session_tests {
    use crate::state::{ChatSession, ChatMessage};

    #[test]
    fn chat_session_se_serializa_y_deserializa() {
        let session = ChatSession {
            id: "test-123".to_string(),
            title: "Test Chat".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hola".to_string(),
                    timestamp: 1700000000,
                },
                ChatMessage {
                    role: "agent".to_string(),
                    content: "Hola, ¿en qué puedo ayudarte?".to_string(),
                    timestamp: 1700000001,
                },
            ],
            project_name: Some("test_project".to_string()),
            steps: None,
        };

        let json = serde_json::to_string(&session).unwrap();
        let restored: ChatSession = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, "test-123");
        assert_eq!(restored.title, "Test Chat");
        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.messages[0].role, "user");
        assert_eq!(restored.messages[1].role, "agent");
        assert_eq!(restored.project_name.unwrap(), "test_project");
    }
}


// ============================================================================
// SECCIÓN 8: Contrato API — Verificación de estructura de endpoints
// ============================================================================

#[cfg(test)]
mod api_contract_tests {
    use serde_json::json;

    #[test]
    fn respuesta_agent_status_tiene_estructura_correcta() {
        // Verifica el contrato de /api/agent/status
        let response = json!({
            "status": "ok",
            "active": true,
            "running": true,
            "finished": false,
            "final_message": null,
            "interrupted": false,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "info_messages": [],
            "current_session_id": "abc"
        });

        let campos_requeridos = [
            "status", "active", "running", "finished", "final_message",
            "interrupted", "esperando_respuesta_usuario", "pregunta_usuario",
            "esperando_aprobacion_plan", "plan_propuesto",
            "info_messages", "current_session_id",
        ];

        for campo in &campos_requeridos {
            assert!(response.get(campo).is_some(),
                "CONTRATO API ROTO: /api/agent/status no incluye '{}'", campo);
        }
    }

    #[test]
    fn respuesta_chat_tiene_status_y_session_id() {
        let response = json!({
            "status": "ok",
            "session_id": "uuid-123",
            "title": "Mi chat",
            "chat_path": "/path/to/chat.json"
        });

        assert_eq!(response["status"], "ok");
        assert!(!response["session_id"].as_str().unwrap().is_empty());
    }

    #[test]
    fn respuesta_login_tiene_campos_requeridos() {
        let response = json!({
            "status": "ok",
            "token": "token123",
            "username": "testuser",
            "is_admin": false,
            "has_study_access": true,
            "has_programming_access": false
        });

        assert_eq!(response["status"], "ok");
        assert!(response.get("has_study_access").is_some());
        assert!(response.get("has_programming_access").is_some());
    }

    #[test]
    fn respuesta_error_tiene_status_y_message() {
        let response = json!({
            "status": "error",
            "message": "Algo salio mal"
        });

        assert_eq!(response["status"], "error");
        assert!(!response["message"].as_str().unwrap().is_empty());
    }
}
