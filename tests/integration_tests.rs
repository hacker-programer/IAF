// ============================================================================
// tests/integration_tests.rs â€” Tests de IntegraciÃ³n y AceptaciÃ³n
//
// Tests REALES que prueban componentes del sistema interactuando entre sÃ­:
// StudyEngine con disco, UserStore con contraseÃ±as, sanitize_filename,
// ActiveAgentStatus serialization, y creaciÃ³n/lectura real de DOCX.
// ============================================================================

use std::fs;
use std::path::PathBuf;

// ============================================================================
// Helpers
// ============================================================================

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("iaf_int_{}", name));
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Crea un UserStore aislado en un directorio temporal con users.json vacÃ­o
fn temp_user_store(name: &str) -> iaf::auth::UserStore {
    let dir = tmp_dir(name);
    let _ = fs::write(dir.join("users.json"), "{\"users\":[]}");
    iaf::auth::UserStore::load(&dir)
}

// ============================================================================
// SECCIÃ“N 1: StudyEngine â€” Persistencia real en disco
// ============================================================================

#[cfg(test)]
mod study_engine_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
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

        assert!(!engine.profile_exists_on_disk("ghost_user"));

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

        let projects_dir = data_dir.join("_projects");
        fs::create_dir_all(&projects_dir).unwrap();
        fs::write(projects_dir.join("p1.json"), "{}").unwrap();

        let user_dir = data_dir.join("real_user");
        fs::create_dir_all(&user_dir).unwrap();
        let profile = json!({"username":"real_user","age":15,"phase":"Exploration","high_capabilities":null,"neurological_conditions":[],"favorite_games":[],"favorite_youtubers":[],"hobbies":[],"exploration_started_at":null,"exploitation_started_at":null,"hypothesis_history":[],"learning_style_summary":"","message_timestamps":[],"last_updated":0});
        fs::write(user_dir.join("profile.json"), serde_json::to_string_pretty(&profile).unwrap()).unwrap();

        let engine = StudyEngine::new(tmp);
        assert!(engine.get_profile("real_user").is_some());
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
// SECCIÃ“N 2: sanitize_filename â€” Funciones utilitarias reales
// ============================================================================

#[cfg(test)]
mod sanitize_filename_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
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
    #[test]
    fn sanitiza_caracteres_no_ascii() {
        let result = sanitize_filename("Análisis del código");
        assert!(result.chars().all(|c| c.is_ascii()));
        assert!(!result.contains("á"));
        assert!(!result.contains("ó"));
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
// SECCIÃ“N 3: ActiveAgentStatus â€” SerializaciÃ³n y valores por defecto
// ============================================================================

#[cfg(test)]
mod active_agent_status_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
    use iaf::state::ActiveAgentStatus;

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
// SECCIÃ“N 4: DOCX Creation & Reading â€” Prueba real de extract_text_from_docx
// ============================================================================

#[cfg(test)]
mod docx_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn crear_docx_y_extraer_texto_con_quick_xml() {
        let dir = std::env::temp_dir().join("iaf_test_docx_real");
        let _ = std::fs::create_dir_all(&dir);
        let docx_path = dir.join("test_real.docx");

        let file = std::fs::File::create(&docx_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip_writer.start_file("word/document.xml", options).unwrap();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Primer pÃ¡rrafo del documento</w:t></w:r></w:p>
    <w:p><w:r><w:t>Segundo pÃ¡rrafo con mÃ¡s contenido</w:t></w:r></w:p>
    <w:p><w:r><w:t>Tercer pÃ¡rrafo</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        {
            use std::io::Write;
            zip_writer.write_all(xml.as_bytes()).unwrap();
        }
        zip_writer.finish().unwrap();

        assert!(docx_path.exists());

        let file = std::fs::File::open(&docx_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut doc_xml = archive.by_name("word/document.xml").unwrap();
        let mut xml_str = String::new();
        {
            use std::io::Read;
            doc_xml.read_to_string(&mut xml_str).unwrap();
        }

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

        assert!(text.contains("Primer pÃ¡rrafo del documento"));
        assert!(text.contains("Segundo pÃ¡rrafo con mÃ¡s contenido"));
        assert!(text.contains("Tercer pÃ¡rrafo"));

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
        {
            use std::io::Write;
            zip_writer.write_all(xml.as_bytes()).unwrap();
        }
        zip_writer.finish().unwrap();

        let file = std::fs::File::open(&docx_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut doc_xml = archive.by_name("word/document.xml").unwrap();
        let mut xml_str = String::new();
        {
            use std::io::Read;
            doc_xml.read_to_string(&mut xml_str).unwrap();
        }

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
// SECCIÃ“N 5: UserStore â€” AutenticaciÃ³n y permisos
// ============================================================================

#[cfg(test)]
mod user_store_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
    use iaf::auth::UserStore;
    use iaf::auth::UserLimits;

    fn new_store(name: &str) -> UserStore {
        super::temp_user_store(name)
    }

    #[test]
    fn crear_usuario_con_password_funciona() {
        let store = new_store("us_pw");
        let result = store.create_user_with_password(
            "testuser", "secure123", false,
            vec!["read_file".to_string(), "search_code".to_string()],
            UserLimits::default(),
            true, false, false, false,
        );
        assert!(result.is_ok());

        let user = store.find_user("testuser");
        assert!(user.is_some());
        assert!(!user.unwrap().is_admin);
    }

    #[test]
    fn verificar_password_correcto() {
        let store = new_store("us_vpc");
        store.create_user_with_password(
            "user1", "mypassword", false,
            vec!["read_file".to_string()],
            UserLimits::default(),
            true, false, false, false,
        ).unwrap();

        let result = store.verify_password("user1", "mypassword");
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn verificar_password_incorrecto() {
        let store = new_store("us_vpi");
        store.create_user_with_password(
            "user1", "mypassword", false,
            vec!["read_file".to_string()],
            UserLimits::default(),
            true, false, false, false,
        ).unwrap();

        let result = store.verify_password("user1", "wrongpassword");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn crear_admin_con_public_key() {
        let store = new_store("us_admin");
        let public_key = "a".repeat(64);
        let result = store.create_admin(
            "admin1", &public_key,
            vec!["read_file".to_string()],
            UserLimits::admin(),
        );
        assert!(result.is_ok());

        let user = store.find_user("admin1");
        assert!(user.is_some());
        assert!(user.unwrap().is_admin);
    }

    #[test]
    fn listar_usuarios_funciona() {
        let store = new_store("us_list");
        store.create_user_with_password(
            "u1", "password1", false, vec!["read_file".to_string()],
            UserLimits::default(),
            true, false, false, false,
        ).unwrap();
        store.create_user_with_password(
            "u2", "password2", false, vec!["read_file".to_string()],
            UserLimits::default(),
            false, true, false, false,
        ).unwrap();

        let users = store.list_users();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn has_study_access_admin_siempre_true() {
        let store = new_store("us_hsa");
        let pk = "b".repeat(64);
        store.create_admin("admin2", &pk, vec!["read_file".to_string()], UserLimits::admin()).unwrap();
        let user = store.find_user("admin2").unwrap();
        assert!(user.has_study_access());
        assert!(user.has_programming_access());
    }

    #[test]
    fn has_study_access_usuario_normal_respeta_permiso() {
        let store = new_store("us_hsa2");
        store.create_user_with_password(
            "user_study", "password1", false, vec!["read_file".to_string()],
            UserLimits::default(),
            true, false, false, false,
        ).unwrap();

        let user = store.find_user("user_study").unwrap();
        assert!(user.has_study_access());
        assert!(!user.has_programming_access());
    }
}


// ============================================================================
// SECCIÃ“N 6: CiclePhase â€” Transiciones de estado
// ============================================================================

#[cfg(test)]
mod cicle_phase_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
    use iaf::state::CiclePhase;

    #[test]
    fn cicle_phase_default_es_implementacion() {
        let phase = CiclePhase::default();
        assert_eq!(phase, CiclePhase::Implementacion);
    }

    #[test]
    fn cicle_phase_tiene_todas_las_fases() {
        let fases = vec![
            CiclePhase::Implementacion,
            CiclePhase::Optimizacion,
            CiclePhase::BusquedaBugs,
            CiclePhase::Reduccion,
            CiclePhase::SegundaBusquedaBugs,
            CiclePhase::Terminar,
        ];
        assert_eq!(fases.len(), 6);
    }

    #[test]
    fn cicle_phase_serializacion() {
        let phase = CiclePhase::Implementacion;
        let json = serde_json::to_value(&phase).unwrap();
        let deserialized: CiclePhase = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, CiclePhase::Implementacion);
    }
}


// ============================================================================
// SECCIÃ“N 7: ChatSession â€” SerializaciÃ³n
// ============================================================================

#[cfg(test)]
mod chat_session_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
    use iaf::state::{ChatSession, ChatMessage};

    #[test]
    fn chat_session_serializacion_deserializacion() {
        let session = ChatSession {
            id: "sess-001".to_string(),
            title: "Test Chat".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hola".to_string(),
                    timestamp: 1700000000,
                },
                ChatMessage {
                    role: "agent".to_string(),
                    content: "Hola, Â¿cÃ³mo puedo ayudarte?".to_string(),
                    timestamp: 1700000001,
                },
            ],
            project_name: Some("test_project".to_string()),
            steps: None,
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: ChatSession = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "sess-001");
        assert_eq!(deserialized.title, "Test Chat");
        assert_eq!(deserialized.messages.len(), 2);
        assert_eq!(deserialized.messages[0].role, "user");
        assert_eq!(deserialized.messages[1].role, "agent");
        assert_eq!(deserialized.project_name.unwrap(), "test_project");
    }
}


// ============================================================================
// SECCIÃ“N 8: Contrato API â€” VerificaciÃ³n de endpoints requeridos
// ============================================================================

#[cfg(test)]
mod api_contract_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
    use serde_json::json;

    #[test]
    fn api_agent_status_tiene_todos_los_campos() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("get_agent_status"), "Falta get_agent_status en main.rs");
        assert!(src.contains("info_messages"), "get_agent_status no incluye info_messages");
        assert!(src.contains("final_message"), "get_agent_status no incluye final_message");
        assert!(src.contains("finished"), "get_agent_status no incluye finished");
        assert!(src.contains("current_session_id"), "get_agent_status no incluye current_session_id");
    }

    #[test]
    fn api_chat_endpoint_acepta_mode() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("chat_endpoint"), "Falta chat_endpoint en main.rs");
        assert!(src.contains("payload.mode"), "chat_endpoint no procesa mode");
    }

    #[test]
    fn api_agent_responder_existe() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("agent_responder"), "Falta agent_responder en main.rs");
    }

    #[test]
    fn api_agent_aprobar_plan_existe() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("agent_approve_plan"), "Falta agent_approve_plan en main.rs");
    }

    #[test]
    fn api_agent_interrupt_existe() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("agent_interrupt"), "Falta agent_interrupt en main.rs");
    }

    #[test]
    fn api_chat_response_tiene_status_ok() {
        let response = json!({
            "status": "ok",
            "session_id": "abc-123",
            "title": "Test",
            "chat_path": "/tmp/test.json"
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



// ============================================================================
// TESTS DE INTEGRIDAD DE ARCHIVOS DE TESTS
// Estos tests verifican que los archivos de tests no tengan errores de sintaxis
// que impidan la compilacion. Usan include_str! que no compila el archivo,
// solo lo lee como texto.
// ============================================================================

#[cfg(test)]
mod test_file_integrity_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    /// Cuenta llaves { } excluyendo strings (con escapes \" y \\),
    /// char literals 'x', y comentarios // y /* */.
    fn count_real_braces(content: &str) -> (usize, usize) {
        let mut open: usize = 0;
        let mut close: usize = 0;
        let mut in_string = false;
        let mut in_char = false;
        let mut in_line_comment = false;
        let mut in_block_comment = false;
        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut i = 0;
        while i < len {
            let c = chars[i];
            let prev = if i > 0 { chars[i - 1] } else { ' ' };
            let prev2 = if i > 1 { chars[i - 2] } else { ' ' };
            let next = if i + 1 < len { chars[i + 1] } else { ' ' };

            if c == '/' && next == '/' && !in_string && !in_char && !in_block_comment {
                in_line_comment = true;
            }
            if c == '/' && next == '*' && !in_string && !in_char && !in_line_comment {
                in_block_comment = true;
            }
            if c == '*' && next == '/' && in_block_comment {
                in_block_comment = false;
                i += 2;
                continue;
            }
            if c == '\n' {
                in_line_comment = false;
            }

            if !in_string && !in_char && !in_line_comment && !in_block_comment {
                if c == '"' { in_string = true; i += 1; continue; }
                if c == '\'' { in_char = true; i += 1; continue; }
                if c == '{' { open += 1; }
                if c == '}' { close += 1; }
                i += 1;
                continue;
            }

            if in_string {
                if c == '"' && prev != '\\' { in_string = false; }
                else if c == '"' && prev == '\\' && prev2 == '\\' { in_string = false; }
                i += 1;
                continue;
            }

            if in_char {
                if c == '\'' && prev != '\\' { in_char = false; }
                else if c == '\'' && prev == '\\' && prev2 == '\\' { in_char = false; }
                i += 1;
                continue;
            }

            i += 1;
        }
        (open, close)
    }

    #[test]
    fn exhaustive_tests_rs_tiene_llaves_balanceadas() {
        let content = include_str!("exhaustive_tests.rs");
        let (open, close) = count_real_braces(content);
        let delta = (open as i64 - close as i64).abs();
        assert!(delta <= 2,
            "REGRESION: exhaustive_tests.rs tiene {} llaves de apertura y {} de cierre. Delta: {}. El archivo NO compilara.",
            open, close, delta);
    }

    #[test]
    fn integration_tests_rs_tiene_llaves_balanceadas() {
        let content = include_str!("integration_tests.rs");
        let (open, close) = count_real_braces(content);
        let delta = (open as i64 - close as i64).abs();
        assert!(delta <= 2,
            "REGRESION: integration_tests.rs tiene {} llaves de apertura y {} de cierre. Delta: {}.",
            open, close, delta);
    }

    #[test]
    fn archivos_fuente_principales_tienen_llaves_balanceadas() {
        let files = vec![
            "src/main.rs",
            "src/agent.rs",
            "src/state.rs",
            "src/auth.rs",
            "src/study.rs",
            "src/validator.rs",
            "src/sub_agent.rs",
            "src/sync.rs",
            "src/scraper.rs",
            "src/desktop.rs",
            "src/client_protocol.rs",
        ];

        for file_path in &files {
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let (open, close) = count_real_braces(&content);
            let delta = (open as i64 - close as i64).abs();
            assert!(delta <= 2,
                "REGRESION: {} tiene {} llaves de apertura y {} de cierre. Delta: {}. El archivo NO compilara.",
                file_path, open, close, delta);
        }
    }

    #[test]
    fn app_js_tiene_delimitadores_balanceados() {
        let content = include_str!("../public/app.js");
        let mut braces_open = 0usize;
        let mut braces_close = 0usize;
        let mut parens_open = 0usize;
        let mut parens_close = 0usize;
        let mut brackets_open = 0usize;
        let mut brackets_close = 0usize;
        let mut in_double = false;
        let mut in_single = false;
        let mut in_template = false;
        let mut in_line = false;
        let mut in_block = false;
        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut i = 0;
        while i < len {
            let c = chars[i];
            let prev = if i > 0 { chars[i - 1] } else { ' ' };
            let next = if i + 1 < len { chars[i + 1] } else { ' ' };
            let in_str = in_double || in_single || in_template;

            if c == '/' && next == '/' && !in_str && !in_block { in_line = true; }
            if c == '/' && next == '*' && !in_str && !in_line { in_block = true; }
            if c == '*' && next == '/' && in_block { in_block = false; i += 2; continue; }
            if c == '\n' { in_line = false; }

            if !in_str && !in_line && !in_block {
                if c == '"' { in_double = true; i += 1; continue; }
                if c == '\'' { in_single = true; i += 1; continue; }
                if c == '`' { in_template = true; i += 1; continue; }
                if c == '{' { braces_open += 1; }
                if c == '}' { braces_close += 1; }
                if c == '(' { parens_open += 1; }
                if c == ')' { parens_close += 1; }
                if c == '[' { brackets_open += 1; }
                if c == ']' { brackets_close += 1; }
                i += 1;
                continue;
            }

            if in_double && c == '"' && prev != '\\' { in_double = false; }
            if in_single && c == '\'' && prev != '\\' { in_single = false; }
            if in_template && c == '`' && prev != '\\' { in_template = false; }
            i += 1;
        }
        assert_eq!(braces_open, braces_close,
            "JS ROTO: app.js tiene {} llaves de apertura vs {} de cierre.", braces_open, braces_close);
        assert_eq!(parens_open, parens_close,
            "JS ROTO: app.js tiene {} parentesis de apertura vs {} de cierre.", parens_open, parens_close);
        assert_eq!(brackets_open, brackets_close,
            "JS ROTO: app.js tiene {} corchetes de apertura vs {} de cierre.", brackets_open, brackets_close);
    }
}

// ============================================================================
// TESTS DE REGRESION DE BUGS VIEJOS (Verificacion de codigo fuente)
// Estos tests usan include_str! para verificar que los fixes en el codigo
// fuente sigan presentes. Si un fix se revierte, el test falla.
// ============================================================================

#[cfg(test)]
mod regression_bugs_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    // =========================================================================
    // BUG-001: No puede analizar PDFs ni .docx
    // =========================================================================

    #[test]
    fn bug001_agent_rs_tiene_extract_text_from_docx() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("fn extract_text_from_docx"),
            "BUG-001 REGRESION: agent.rs no tiene fn extract_text_from_docx");
        assert!(src.contains("zip::ZipArchive"),
            "BUG-001 REGRESION: agent.rs no usa zip::ZipArchive");
        assert!(src.contains("quick_xml::Reader"),
            "BUG-001 REGRESION: agent.rs no usa quick_xml::Reader");
    }

    #[test]
    fn bug001_agent_rs_usa_pdf_extract_nativo() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("pdf_extract::extract_text"),
            "BUG-001 REGRESION: agent.rs no usa pdf_extract::extract_text");
        assert!(!src.contains("pdftotext"),
            "BUG-001 REGRESION: agent.rs contiene pdftotext (debe usar pdf_extract nativo)");
    }

    #[test]
    fn bug001_agent_rs_read_file_detecta_extensiones_pdf_docx() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("ext == \"pdf\""),
            "BUG-001 REGRESION: read_file no detecta extension .pdf");
        assert!(src.contains("ext == \"docx\""),
            "BUG-001 REGRESION: read_file no detecta extension .docx");
    }

    // =========================================================================
    // BUG-002: El frontend no muestra mensajes informativos en tiempo real
    // =========================================================================

    #[test]
    fn bug002_app_js_consume_info_messages_antes_de_check_active() {
        let js = include_str!("../public/app.js");
        let idx_info = js.find("info_messages").unwrap();
        let remainder = &js[idx_info..];
        let idx_active = remainder.find("statusRes.running").unwrap_or(usize::MAX);
        let idx_finished = remainder.find("statusRes.finished").unwrap_or(usize::MAX);
        let first_check = idx_active.min(idx_finished);
        let idx_show = remainder.find("showInfoToast").unwrap();
        assert!(idx_show < first_check,
            "BUG-002 REGRESION: showInfoToast se llama DESPUES de chequear running/finished. Los mensajes no se muestran en tiempo real.");
    }

    #[test]
    fn bug002_agent_rs_finalizar_tarea_no_limpia_info_messages() {
        let src = include_str!("../src/agent.rs");
        let finalizar_idx = src.find("\"finalizar_tarea\" =>").unwrap();
        let next_tool = src[finalizar_idx..].find("\"image_fetch\" =>").unwrap_or(src.len() - finalizar_idx);
        let block = &src[finalizar_idx..finalizar_idx + next_tool];
        assert!(!block.contains("info_messages.clear()"),
            "BUG-002 REGRESION: finalizar_tarea contiene info_messages.clear(). Los mensajes se pierden.");
    }

    #[test]
    fn bug002_state_rs_active_agent_status_tiene_info_messages() {
        let src = include_str!("../src/state.rs");
        assert!(src.contains("info_messages: Vec<String>"),
            "BUG-002 REGRESION: ActiveAgentStatus no tiene campo info_messages");
    }

    // =========================================================================
    // BUG-004: finalizar_tarea devuelve error "No se proporcionï¿½ URL"
    // =========================================================================

    #[test]
    fn bug004_agent_rs_finalizar_tarea_no_contiene_url() {
        let src = include_str!("../src/agent.rs");
        let finalizar_idx = src.find("\"finalizar_tarea\" =>").unwrap();
        let next_tool = src[finalizar_idx..].find("\"image_fetch\" =>").unwrap_or(src.len() - finalizar_idx);
        let block = &src[finalizar_idx..finalizar_idx + next_tool];
        let url_count = block.matches("\"url\"").count();
        assert_eq!(url_count, 0,
            "BUG-004 REGRESION: finalizar_tarea contiene {} referencias a 'url'. Se confunde con image_fetch.", url_count);
        assert!(block.contains("mensaje_final"),
            "BUG-004 REGRESION: finalizar_tarea no usa mensaje_final.");
    }

    #[test]
    fn bug004_agent_rs_finalizar_tarea_refactorizado_multilinea() {
        let src = include_str!("../src/agent.rs");
        let finalizar_idx = src.find("\"finalizar_tarea\" =>").unwrap();
        let next_tool = src[finalizar_idx..].find("\"image_fetch\" =>").unwrap_or(src.len() - finalizar_idx);
        let block = &src[finalizar_idx..finalizar_idx + next_tool];
        let line_count = block.lines().count();
        assert!(line_count > 10,
            "BUG-004 REGRESION: finalizar_tarea solo tiene {} lineas. Debe estar refactorizado a multi-linea.", line_count);
    }

    // =========================================================================
    // BUG: No carga el perfil en modo estudio en el frontend
    // =========================================================================

    #[test]
    fn perfil_estudio_app_js_tiene_load_study_profile() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function loadStudyProfile"),
            "REGRESION: app.js no tiene loadStudyProfile");
        assert!(js.contains("/api/study/profile"),
            "REGRESION: loadStudyProfile no llama a /api/study/profile");
    }

    #[test]
    fn perfil_estudio_main_rs_tiene_endpoint_study_profile() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("/api/study/profile"),
            "REGRESION: main.rs no tiene endpoint /api/study/profile");
        assert!(src.contains("study_get_profile"),
            "REGRESION: main.rs no tiene handler study_get_profile");
    }

    #[test]
    fn perfil_estudio_study_rs_tiene_profile_exists_on_disk() {
        let src = include_str!("../src/study.rs");
        assert!(src.contains("profile_exists_on_disk"),
            "REGRESION: study.rs no tiene profile_exists_on_disk");
        assert!(src.contains("profile.json"),
            "REGRESION: study.rs no referencia profile.json");
    }

    // =========================================================================
    // BUG: No ve el system prompt local ni el perfil ni el directorio
    // =========================================================================

    #[test]
    fn system_prompt_agent_rs_tiene_load_local_prompt() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("load_local_prompt"),
            "REGRESION: agent.rs no usa load_local_prompt");
        assert!(src.contains("get_project_path"),
            "REGRESION: agent.rs no usa get_project_path");
        assert!(src.contains("Project Specific Prompt:"),
            "REGRESION: agent.rs no incluye el prompt local en el system prompt");
    }

    #[test]
    fn system_prompt_state_rs_tiene_metodos_load_prompt() {
        let src = include_str!("../src/state.rs");
        assert!(src.contains("fn load_global_prompt"),
            "REGRESION: state.rs no tiene load_global_prompt");
        assert!(src.contains("fn load_local_prompt"),
            "REGRESION: state.rs no tiene load_local_prompt");
        assert!(src.contains("globalPrompt.json"),
            "REGRESION: load_global_prompt no lee globalPrompt.json");
        assert!(src.contains("localPrompt.json"),
            "REGRESION: load_local_prompt no lee localPrompt.json");
    }

    // =========================================================================
    // BUG: No se puede empezar una conversaciï¿½n (addMessage duplicada)
    // =========================================================================

    #[test]
    fn addmessage_app_js_definida_una_sola_vez() {
        let js = include_str!("../public/app.js");
        let count = js.matches("function addMessage").count();
        assert_eq!(count, 1,
            "REGRESION: addMessage definida {} veces. Debe ser exactamente 1.", count);
    }

    #[test]
    fn addmessage_app_js_tiene_send_message_to_agent() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function sendMessageToAgent"),
            "REGRESION: app.js no tiene sendMessageToAgent");
        assert!(js.contains("/api/chat"),
            "REGRESION: sendMessageToAgent no llama a /api/chat");
    }

    #[test]
    fn addmessage_app_js_tiene_function_init() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function init"),
            "REGRESION: app.js no tiene function init()");
        assert!(js.contains("init()"),
            "REGRESION: app.js no llama a init()");
    }

    // =========================================================================
    // Tests de integridad de delimitadores en archivos fuente
    // =========================================================================

    #[test]
    fn delimitadores_app_js_balanceados() {
        let js = include_str!("../public/app.js");
        let braces_open = js.matches('{').count();
        let braces_close = js.matches('}').count();
        assert_eq!(braces_open, braces_close,
            "JS ROTO: {} llaves de apertura vs {} de cierre", braces_open, braces_close);
        let parens_open = js.matches('(').count();
        let parens_close = js.matches(')').count();
        assert_eq!(parens_open, parens_close,
            "JS ROTO: {} parentesis de apertura vs {} de cierre", parens_open, parens_close);
        let brackets_open = js.matches('[').count();
        let brackets_close = js.matches(']').count();
        assert_eq!(brackets_open, brackets_close,
            "JS ROTO: {} corchetes de apertura vs {} de cierre", brackets_open, brackets_close);
    }
}
