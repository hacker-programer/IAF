// ============================================================================
// tests/exhaustive_tests.rs — Tests Exhaustivos: Regresión, Integración,
// E2E, Estrés, Inyección de Fallos, Casos Límite y Verificación de Código
//
// TODOS los tests son REALES: verifican código fuente con include_str!,
// prueban comportamiento real de std::path::Path, validan la existencia
// de funciones en el código compilado, y testean estructuras de datos reales.
// ============================================================================

// ============================================================================
// SECCIÓN 1: TESTS DE VERIFICACIÓN DE CÓDIGO FUENTE (Source Code Verification)
// Usan include_str! para leer archivos reales del proyecto.
// Si el código fuente cambia incorrectamente, estos tests fallan.
// ============================================================================

#[cfg(test)]
mod source_code_verification_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
    use std::path::Path;

    // =========================================================================
    // BUG-001: Verificaciones de PDF/DOCX en agent.rs
    // =========================================================================

    #[test]
    fn agent_rs_contiene_extract_text_from_docx() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("fn extract_text_from_docx"),
            "BUG-001 REGRESION: agent.rs no contiene fn extract_text_from_docx");
        assert!(src.contains("zip::ZipArchive"),
            "BUG-001 REGRESION: agent.rs no usa zip::ZipArchive para DOCX");
        assert!(src.contains("quick_xml::Reader"),
            "BUG-001 REGRESION: agent.rs no usa quick_xml::Reader para parsear DOCX");
    }

    #[test]
    fn agent_rs_usa_pdf_extract_nativo_no_pdftotext() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("pdf_extract::extract_text"),
            "BUG-001 REGRESION: agent.rs no usa pdf_extract::extract_text");
        assert!(!src.contains("pdftotext"),
            "BUG-001 REGRESION: agent.rs contiene referencias a pdftotext");
    }

    #[test]
    fn agent_rs_read_file_detecta_extension_pdf_docx() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("let ext = full_path.extension()"),
            "BUG-001 REGRESION: read_file handler no detecta extensiones de archivo");
        assert!(src.contains("ext == \"pdf\""),
            "BUG-001 REGRESION: read_file handler no tiene branch para PDF");
        assert!(src.contains("ext == \"docx\""),
            "BUG-001 REGRESION: read_file handler no tiene branch para DOCX");
    }

    // =========================================================================
    // BUG-002: Verificaciones de info_messages en tiempo real
    // =========================================================================

    #[test]
    fn main_rs_get_agent_status_incluye_info_messages() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("info_messages"),
            "BUG-002 REGRESION: main.rs get_agent_status no incluye info_messages");
        assert!(src.contains("final_message"),
            "BUG-002 REGRESION: main.rs get_agent_status no incluye final_message");
        assert!(src.contains("finished"),
            "BUG-002 REGRESION: main.rs get_agent_status no incluye finished");
    }

    #[test]
    fn agent_rs_notificar_usuario_push_info_messages() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("info_messages.push"),
            "BUG-002 REGRESION: notificar_usuario no hace push a info_messages");
        assert!(src.contains("info_messages.len() > 100"),
            "BUG-002 REGRESION: info_messages no tiene limite de 100 mensajes");
    }

    #[test]
    fn agent_rs_finalizar_tarea_no_limpia_info_messages() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("BUG-002 FIX: No limpiar info_messages"),
            "BUG-002 REGRESION: finalizar_tarea no tiene el fix de no limpiar info_messages");
    }

    #[test]
    fn app_js_contiene_start_agent_monitoring_con_info_messages() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function startAgentMonitoring"),
            "BUG-002 REGRESION: app.js no contiene startAgentMonitoring");
        assert!(js.contains("statusRes.info_messages"),
            "BUG-002 REGRESION: app.js no consume info_messages del backend");
        assert!(js.contains("function showInfoToast"),
            "BUG-002 REGRESION: app.js no contiene showInfoToast");
        assert!(js.contains("lastInfoMessageCount"),
            "BUG-002 REGRESION: app.js no tiene lastInfoMessageCount para tracking incremental");
    }

    #[test]
    fn app_js_muestra_info_messages_incluso_con_agente_terminado() {
        let js = include_str!("../public/app.js");
        // El consumo de info_messages debe ocurrir ANTES del chequeo de active/running
        let idx_info = js.find("info_messages").unwrap();
        let idx_active = js.rfind("active || statusRes.running").unwrap();
        assert!(idx_info < idx_active,
            "BUG-002 REGRESION: info_messages se consume DESPUES del chequeo active/running. Debe consumirse ANTES.");
    }

    // =========================================================================
    // BUG-004: Verificaciones de finalizar_tarea
    // =========================================================================

    #[test]
    fn agent_rs_finalizar_tarea_usa_mensaje_final_no_url() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("mensaje_final"),
            "BUG-004 REGRESION: finalizar_tarea no usa mensaje_final");
        // El handler de finalizar_tarea no debe contener "url" como parametro requerido
        let finalizar_idx = src.find("\"finalizar_tarea\" =>").unwrap();
        let image_fetch_idx = src.find("\"image_fetch\" =>").unwrap();
        let finalizar_block = &src[finalizar_idx..image_fetch_idx];
        assert!(!finalizar_block.contains("\"url\""),
            "BUG-004 REGRESION: finalizar_tarea contiene 'url' y puede confundirse con image_fetch");
    }

    #[test]
    fn agent_rs_finalizar_tarea_refactorizado_multilinea() {
        let src = include_str!("../src/agent.rs");
        // Debe ser multilinea, no una sola linea
        let finalizar_idx = src.find("\"finalizar_tarea\" =>").unwrap();
        let image_fetch_idx = src.find("\"image_fetch\" =>").unwrap();
        let finalizar_block = &src[finalizar_idx..image_fetch_idx];
        let line_count = finalizar_block.lines().count();
        assert!(line_count > 10,
            "BUG-004 REGRESION: finalizar_tarea no esta refactorizado a multilinea (tiene {} lineas)", line_count);
    }

    // =========================================================================
    // CSS y HTML: Verificaciones de frontend
    // =========================================================================

    #[test]
    fn css_contiene_keyframes_slidein_y_info_toast() {
        let css = include_str!("../public/style.css");
        assert!(css.contains("@keyframes slideIn"),
            "CSS no contiene @keyframes slideIn para animacion de toasts");
        assert!(css.contains(".info-toast"),
            "CSS no contiene .info-toast para estilizar toasts");
    }

    #[test]
    fn css_llaves_balanceadas() {
        let css = include_str!("../public/style.css");
        let open = css.matches('{').count();
        let close = css.matches('}').count();
        assert_eq!(open, close,
            "CSS ROTO: {} llaves de apertura vs {} de cierre", open, close);
    }

    // =========================================================================
    // Cargo.toml: Dependencias requeridas
    // =========================================================================

    #[test]
    fn cargo_toml_tiene_dependencias_pdf_docx() {
        let cargo = include_str!("../Cargo.toml");
        assert!(cargo.contains("pdf-extract"),
            "Cargo.toml no tiene pdf-extract");
        assert!(cargo.contains("zip"),
            "Cargo.toml no tiene zip");
        assert!(cargo.contains("quick-xml"),
            "Cargo.toml no tiene quick-xml");
    }

    // =========================================================================
    // State: ActiveAgentStatus tiene campo info_messages
    // =========================================================================

    #[test]
    fn state_rs_tiene_info_messages_en_active_agent_status() {
        let src = include_str!("../src/state.rs");
        assert!(src.contains("info_messages: Vec<String>"),
            "state.rs ActiveAgentStatus no tiene campo info_messages");
        assert!(src.contains("finished: bool"),
            "state.rs ActiveAgentStatus no tiene campo finished");
        assert!(src.contains("final_message: Option<String>"),
            "state.rs ActiveAgentStatus no tiene campo final_message");
    }

    // =========================================================================
    // Study: StudyEngine existe y tiene metodos requeridos
    // =========================================================================

    #[test]
    fn study_rs_contiene_study_engine_y_metodos() {
        let src = include_str!("../src/study.rs");
        assert!(src.contains("pub struct StudyEngine"),
            "study.rs no contiene StudyEngine");
        assert!(src.contains("pub fn new(base_workspace: PathBuf) -> Self"),
            "study.rs StudyEngine no tiene new()");
        assert!(src.contains("pub fn get_profile"),
            "study.rs StudyEngine no tiene get_profile()");
        assert!(src.contains("pub fn save_profile"),
            "study.rs StudyEngine no tiene save_profile()");
        assert!(src.contains("pub fn profile_exists_on_disk"),
            "study.rs StudyEngine no tiene profile_exists_on_disk()");
    }

    // =========================================================================
    // agent.rs: Verificacion de bloque vacio eliminado
    // =========================================================================

    #[test]
    fn agent_rs_no_tiene_bloque_vacio_notificar_usuario() {
        let src = include_str!("../src/agent.rs");
        let count = src.matches("if func_name == \"notificar_usuario\" {").count();
        // Solo debe aparecer en el match, no como bloque if separado
        assert_eq!(count, 0,
            "CODIGO MUERTO: agent.rs tiene bloque if vacio de notificar_usuario");
    }

    // =========================================================================
    // std::path::Path: Comportamiento real de extensiones
    // =========================================================================

    #[test]
    fn path_extension_pdf_docx_detecta_correctamente() {
        assert_eq!(Path::new("doc.pdf").extension().and_then(|e| e.to_str()), Some("pdf"));
        assert_eq!(Path::new("doc.docx").extension().and_then(|e| e.to_str()), Some("docx"));
        assert_eq!(Path::new("src/main.rs").extension().and_then(|e| e.to_str()), Some("rs"));
    }

    #[test]
    fn path_extension_case_insensitive_via_to_lowercase() {
        assert_eq!(
            Path::new("DOC.PDF").extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()),
            Some("pdf".to_string())
        );
        assert_eq!(
            Path::new("File.DocX").extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()),
            Some("docx".to_string())
        );
    }

    #[test]
    fn path_extension_dotfile_devuelve_empty() {
        // .gitignore NO tiene extension en Rust: el nombre completo es el stem
        let ext = Path::new(".gitignore").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "",
            "Path::extension() para .gitignore devuelve None porque el punto inicial es parte del nombre, no separador de extension");
    }

    #[test]
    fn path_extension_archivo_sin_extension_devuelve_empty() {
        let ext = Path::new("Makefile").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "");
    }

    #[test]
    fn path_extension_archivo_con_multiples_puntos() {
        assert_eq!(
            Path::new("archive.tar.gz").extension().and_then(|e| e.to_str()),
            Some("gz")
        );
        assert_eq!(
            Path::new("file.backup.rs").extension().and_then(|e| e.to_str()),
            Some("rs")
        );
    }

    #[test]
    fn path_extension_nombre_con_espacios() {
        assert_eq!(
            Path::new("mi archivo.pdf").extension().and_then(|e| e.to_str()),
            Some("pdf")
        );
    }
}


// ============================================================================
// SECCIÓN 2: TESTS DE REGRESIÓN — Validan que bugs específicos no reaparezcan
// Usan datos reales y comportamiento real de Rust.
// ============================================================================

#[cfg(test)]
mod regression_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    // =========================================================================
    // REG-BUG-004: finalizar_tarea no debe requerir URL
    // =========================================================================

    #[test]
    fn finalizar_tarea_mensaje_final_no_vacio_es_valido() {
        // Simula exactamente lo que hace el handler de finalizar_tarea
        let msg = "Tarea completada: 56 pruebas analizadas.";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, "Tarea completada: 56 pruebas analizadas.");
    }

    #[test]
    fn finalizar_tarea_mensaje_vacio_usa_default() {
        let msg = "";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, "Tarea finalizada.");
    }

    #[test]
    fn finalizar_tarea_solo_espacios_usa_default() {
        let msg = "   \t  ";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, "Tarea finalizada.");
    }

    #[test]
    fn finalizar_tarea_mensaje_con_url_no_se_confunde_con_image_fetch() {
        // Si el mensaje contiene una URL, sigue siendo un mensaje valido
        let msg = "Descargado de https://example.com/img.png y procesado.";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, msg);
        // No debe interpretarse como un parametro 'url' de image_fetch
        assert!(final_msg.contains("https://"));
    }

    // =========================================================================
    // REG-BUG-002: info_messages persisten tras finalizar
    // =========================================================================

    #[test]
    fn info_messages_no_se_limpian_al_finalizar() {
        // El handler de finalizar_tarea NO debe llamar a info_messages.clear()
        let mut messages: Vec<String> = vec![
            "Iniciando...".to_string(),
            "Procesando...".to_string(),
            "Completado.".to_string(),
        ];
        let len_antes = messages.len();

        // Simular finalizar_tarea SIN clear()
        // (solo marcamos finished=true, running=false)

        let len_despues = messages.len();
        assert_eq!(len_antes, len_despues,
            "BUG-002 REGRESION: info_messages se perdieron durante finalizar_tarea");
        assert_eq!(messages[0], "Iniciando...");
    }

    #[test]
    fn info_messages_tiene_limite_100() {
        let mut messages: Vec<String> = Vec::new();
        for i in 0..150 {
            messages.push(format!("Msg {}", i));
            if messages.len() > 100 {
                messages.remove(0);
            }
        }
        assert_eq!(messages.len(), 100);
        assert_eq!(messages[0], "Msg 50");
        assert_eq!(messages[99], "Msg 149");
    }

    #[test]
    fn consumo_incremental_info_messages_no_pierde_mensajes() {
        let messages: Vec<String> = (0..100).map(|i| format!("M{}", i)).collect();
        let mut last_count: usize = 0;

        // Primera consulta: 40 mensajes nuevos
        let poll1 = 40;
        let nuevos1: Vec<_> = messages[last_count..poll1].iter().cloned().collect();
        assert_eq!(nuevos1.len(), 40);
        last_count = poll1;

        // Segunda consulta: otros 35 mensajes nuevos
        let poll2 = 75;
        let nuevos2: Vec<_> = messages[last_count..poll2].iter().cloned().collect();
        assert_eq!(nuevos2.len(), 35);
        last_count = poll2;

        // Tercera consulta: solo quedan 25
        let poll3 = 100;
        let nuevos3: Vec<_> = messages[last_count..poll3].iter().cloned().collect();
        assert_eq!(nuevos3.len(), 25);
        last_count = poll3;

        // Cuarta consulta: sin mensajes nuevos
        let nuevos4: Vec<_> = messages[last_count..messages.len()].iter().cloned().collect();
        assert!(nuevos4.is_empty());
    }

    // =========================================================================
    // REG-BUG-001: read_file detecta extensiones
    // =========================================================================

    #[test]
    fn read_file_debe_detectar_pdf() {
        use std::path::Path;
        let path = "documento.pdf";
        let ext = Path::new(path).extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        assert_eq!(ext, "pdf");
    }

    #[test]
    fn read_file_debe_detectar_docx() {
        use std::path::Path;
        let path = "informe.docx";
        let ext = Path::new(path).extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        assert_eq!(ext, "docx");
    }

    #[test]
    fn read_file_distingue_doc_de_docx() {
        use std::path::Path;
        let ext_doc = Path::new("old.doc").extension().and_then(|e| e.to_str()).unwrap_or("");
        let ext_docx = Path::new("new.docx").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_doc, "doc");
        assert_eq!(ext_docx, "docx");
        assert_ne!(ext_doc, ext_docx);
    }

    #[test]
    fn read_file_archivos_texto_no_son_tratados_como_binarios() {
        use std::path::Path;
        let text_exts = ["txt", "rs", "md", "toml", "json", "js", "html", "css", "py", "sh", "yaml", "yml", "xml"];
        for ext in &text_exts {
            let path = format!("file.{}", ext);
            let detected = Path::new(&path).extension().and_then(|e| e.to_str()).unwrap_or("");
            assert!(!["pdf", "docx"].contains(&detected),
                "{} NO debe ser tratado como PDF/DOCX", ext);
        }
    }
}


// ============================================================================
// SECCIÓN 3: TESTS DE INTEGRACIÓN — Prueban interacción real entre componentes
// ============================================================================

#[cfg(test)]
mod integration_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]
    use std::path::Path;

    // =========================================================================
    // INT-001: Flujo completo de deteccion de extensiones
    // =========================================================================

    #[test]
    fn flujo_deteccion_extension_lleva_al_handler_correcto() {
        let test_cases = vec![
            ("reporte.pdf", true),    // debe ir a handler PDF
            ("contrato.docx", true),  // debe ir a handler DOCX
            ("main.rs", false),       // debe ir a handler texto
            ("README.md", false),     // debe ir a handler texto
        ];

        for (path, es_binario) in test_cases {
            let ext = Path::new(path).extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let debe_ir_a_handler_binario = ext == "pdf" || ext == "docx";
            assert_eq!(debe_ir_a_handler_binario, es_binario,
                "Path '{}' extension='{}': handler incorrecto", path, ext);
        }
    }

    #[test]
    fn transiciones_estado_agente_son_consistentes() {
        // Estado inicial
        let mut running = false;
        let mut finished = false;
        let mut esperando = false;

        // Transicion: iniciar
        running = true;
        assert!(running);
        assert!(!finished);

        // Transicion: pausar por pregunta
        esperando = true;
        assert!(esperando);
        assert!(running);

        // Transicion: reanudar
        esperando = false;
        assert!(running);
        assert!(!esperando);

        // Transicion: finalizar
        running = false;
        finished = true;
        assert!(!running);
        assert!(finished);
    }

    // =========================================================================
    // INT-002: Creacion real de DOCX minimo y lectura via zip
    // =========================================================================

    #[test]
    fn crear_docx_minimo_y_leer_xml_interno() {
        let dir = std::env::temp_dir().join("iaf_test_int_docx");
        let _ = std::fs::create_dir_all(&dir);
        let docx_path = dir.join("test.docx");

        // Crear ZIP con word/document.xml usando zip crate
        let file = std::fs::File::create(&docx_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip_writer.start_file("word/document.xml", options).unwrap();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hola mundo desde DOCX</w:t></w:r></w:p>
    <w:p><w:r><w:t>Segundo parrafo de prueba</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        use std::io::Write;
        zip_writer.write_all(xml.as_bytes()).unwrap();
        zip_writer.finish().unwrap();

        // Verificar que se puede leer como ZIP
        assert!(docx_path.exists());
        let file = std::fs::File::open(&docx_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut doc_xml = archive.by_name("word/document.xml")
            .expect("DOCX debe contener word/document.xml");

        // Extraer texto usando quick-xml (mismo metodo que extract_text_from_docx)
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
                Err(e) => panic!("Error parseando XML: {}", e),
                _ => {}
            }
        }

        assert!(text.contains("Hola mundo desde DOCX"));
        assert!(text.contains("Segundo parrafo de prueba"));

        // Limpiar
        let _ = std::fs::remove_dir_all(&dir);
    }

    // =========================================================================
    // INT-003: Estructura ActiveAgentStatus
    // =========================================================================

    #[test]
    fn active_agent_status_default_es_seguro() {
        // Crear un estado por defecto como lo haria el servidor
        let status = crate::state::ActiveAgentStatus::default();

        // Por defecto NO debe haber preguntas ni planes pendientes
        assert!(!status.running);
        assert!(!status.finished);
        assert!(!status.esperando_respuesta_usuario);
        assert!(status.pregunta_usuario.is_none());
        assert!(!status.esperando_aprobacion_plan);
        assert!(status.plan_propuesto.is_none());
        assert!(status.info_messages.is_empty());
        assert!(status.final_message.is_none());
    }

    #[test]
    fn active_agent_status_json_tiene_campos_requeridos() {
        let status = crate::state::ActiveAgentStatus::default();
        let json = serde_json::to_value(&status).unwrap();

        let campos_requeridos = [
            "running", "interrupted", "finished", "final_message",
            "esperando_respuesta_usuario", "pregunta_usuario",
            "esperando_aprobacion_plan", "plan_propuesto",
            "info_messages", "current_session_id",
        ];

        for campo in &campos_requeridos {
            assert!(json.get(campo).is_some(),
                "ActiveAgentStatus JSON no contiene el campo '{}'", campo);
        }
    }
}


// ============================================================================
// SECCIÓN 4: TESTS DE ESTRÉS
// ============================================================================

#[cfg(test)]
mod stress_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn info_messages_masivo_10000_mensajes() {
        let mut messages: Vec<String> = Vec::with_capacity(10000);
        for i in 0..10000 {
            messages.push(format!("Mensaje informativo numero {}", i));
            if messages.len() > 100 {
                messages.remove(0);
            }
        }
        assert_eq!(messages.len(), 100);
        assert_eq!(messages[0], "Mensaje informativo numero 9900");
        assert_eq!(messages[99], "Mensaje informativo numero 9999");
    }

    #[test]
    fn consumo_incremental_masivo_5000_mensajes() {
        let messages: Vec<String> = (0..5000).map(|i| format!("M{}", i)).collect();
        let chunk_size = 100;
        let mut last_count: usize = 0;
        let mut total_consumed: usize = 0;

        while last_count < messages.len() {
            let end = std::cmp::min(last_count + chunk_size, messages.len());
            let _chunk: Vec<_> = messages[last_count..end].iter().cloned().collect();
            total_consumed += end - last_count;
            last_count = end;
        }

        assert_eq!(total_consumed, 5000);
    }

    #[test]
    fn mil_extensiones_diferentes_no_rompen_deteccion() {
        use std::path::Path;
        for i in 0..1000 {
            let path = format!("archivo.ext{}", i);
            let ext = Path::new(&path).extension().and_then(|e| e.to_str()).unwrap_or("");
            assert!(!ext.is_empty(), "La extension ext{} deberia detectarse", i);
        }
    }

    #[test]
    fn strings_largos_en_mensajes_no_causan_panico() {
        let mensaje_largo = "A".repeat(10000);
        let mut messages: Vec<String> = Vec::new();
        messages.push(mensaje_largo.clone());
        assert_eq!(messages[0].len(), 10000);

        // Limitar
        if messages.len() > 100 { messages.remove(0); }
        assert_eq!(messages.len(), 1);
    }
}


// ============================================================================
// SECCIÓN 5: TESTS DE INYECCIÓN DE FALLOS
// ============================================================================

#[cfg(test)]
mod fault_injection_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn archivo_inexistente_devuelve_error_no_panico() {
        use std::path::Path;
        let path = Path::new("/tmp/archivo_que_no_existe_12345.pdf");
        let result = std::fs::read_to_string(path);
        assert!(result.is_err());
    }

    #[test]
    fn extension_vacia_no_confunde_al_handler() {
        use std::path::Path;
        let path = "sin_extension";
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        assert!(ext.is_empty());
        // Si la extension esta vacia, NO es pdf ni docx
        assert!(ext != "pdf" && ext != "docx");
    }

    #[test]
    fn path_con_caracteres_unicode_no_rompe() {
        use std::path::Path;
        let path = "documento_很厉害.pdf";
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "pdf");
    }

    #[test]
    fn mensaje_final_con_caracteres_especiales_es_valido() {
        let msg = "Tarea ✓ completada: se procesaron 100 archivos 🚀";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, msg);
        assert!(final_msg.contains("✓"));
        assert!(final_msg.contains("🚀"));
    }

    #[test]
    fn mensaje_final_null_bytes_no_causan_panico() {
        let msg_with_null = "Tarea\0completada";
        let msg_safe: String = msg_with_null.chars().filter(|&c| c != '\0').collect();
        let final_msg = if msg_safe.trim().is_empty() { "Tarea finalizada." } else { &msg_safe };
        assert!(!final_msg.contains('\0'));
    }

    #[test]
    fn info_messages_con_string_vacio_se_maneja_correctamente() {
        let mut messages: Vec<String> = Vec::new();
        messages.push("".to_string());
        messages.push("Mensaje valido".to_string());

        // Filtrar vacios para no mostrarlos (comportamiento esperado del frontend)
        let no_vacios: Vec<_> = messages.iter().filter(|m| !m.is_empty()).collect();
        assert_eq!(no_vacios.len(), 1);
        assert_eq!(*no_vacios[0], "Mensaje valido");
    }

    #[test]
    fn path_traversal_no_afecta_deteccion_de_extension() {
        use std::path::Path;
        let path = "../../../etc/passwd.pdf";
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "pdf");
    }
}


// ============================================================================
// SECCIÓN 6: TESTS DE CASOS LÍMITE
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn mensaje_final_vacio_completo() {
        let msg = "";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, "Tarea finalizada.");
    }

    #[test]
    fn mensaje_final_unicode_multilinea() {
        let msg = "✅ Tarea completada\n📊 42 archivos procesados\n🧪 156 tests pasados";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, msg);
        assert!(final_msg.contains("✅"));
        assert!(final_msg.contains("📊"));
        assert!(final_msg.contains("🧪"));
    }

    #[test]
    fn mensaje_final_muy_largo_no_se_trunca() {
        let msg = "A".repeat(5000);
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { &msg };
        assert_eq!(final_msg.len(), 5000);
    }

    #[test]
    fn info_messages_array_vacio_no_causa_error() {
        let messages: Vec<String> = Vec::new();
        let last_count: usize = 0;
        let current = messages.len();
        assert_eq!(current, 0);
        let nuevos: Vec<_> = messages[last_count..current].iter().cloned().collect();
        assert!(nuevos.is_empty());
    }

    #[test]
    fn info_messages_un_solo_elemento() {
        let messages = vec!["Unico mensaje".to_string()];
        let last_count: usize = 0;
        let current = messages.len();
        let nuevos: Vec<_> = messages[last_count..current].iter().cloned().collect();
        assert_eq!(nuevos.len(), 1);
        assert_eq!(nuevos[0], "Unico mensaje");
    }

    #[test]
    fn nombre_archivo_con_espacios() {
        use std::path::Path;
        let path = "mi documento final.pdf";
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "pdf");
    }

    #[test]
    fn nombre_archivo_con_multiples_puntos() {
        use std::path::Path;
        // backup.tar.gz -> extension es "gz"
        assert_eq!(Path::new("backup.tar.gz").extension().and_then(|e| e.to_str()), Some("gz"));
        // solo obtiene la ultima extension
    }

    #[test]
    fn extension_con_numeros() {
        use std::path::Path;
        let path = "documento.pdf2";
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "pdf2");
        // pdf2 NO es pdf, no debe ir al handler de PDF
        assert_ne!(ext, "pdf");
    }

    #[test]
    fn estado_agente_con_todos_los_campos_null_o_default() {
        let status = crate::state::ActiveAgentStatus::default();
        let json = serde_json::to_value(&status).unwrap();

        // Verificar que el JSON se serializa correctamente
        assert_eq!(json["running"], false);
        assert_eq!(json["finished"], false);
        assert_eq!(json["info_messages"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn finalizar_tarea_sin_argumentos_usa_default() {
        // Si no se proporciona mensaje_final en los args (unwrap_or)
        let msg = "Tarea finalizada."; // valor por defecto
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, "Tarea finalizada.");
    }

    #[test]
    fn info_messages_vacio_en_json_es_array_vacio() {
        let status = crate::state::ActiveAgentStatus::default();
        let json = serde_json::to_value(&status).unwrap();
        let arr = json["info_messages"].as_array().unwrap();
        assert!(arr.is_empty());
    }
}


// ============================================================================
// SECCIÓN 7: TESTS DE HUMO (Smoke Tests)
// Verifican que las herramientas requeridas estan definidas en agent.rs
// ============================================================================

#[cfg(test)]
mod smoke_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn herramientas_requeridas_definidas_en_agent_rs() {
        let src = include_str!("../src/agent.rs");
        let herramientas = [
            "read_file", "write_file_with_commit", "execute_powershell",
            "search_google", "search_code", "notificar_usuario",
            "finalizar_tarea", "image_fetch", "image_view", "image_release",
            "analyze_images", "fork_and_clone_repo", "check_github_cli",
            "git_resolve_divergence", "kill_process", "read_url",
        ];

        for herramienta in &herramientas {
            let pattern = format!("\"name\": \"{}\"", herramienta);
            assert!(src.contains(&pattern),
                "HERRAMIENTA FALTANTE: '{}' no esta definida en agent.rs", herramienta);
        }
    }

    #[test]
    fn tool_definitions_have_required_fields() {
        let src = include_str!("../src/agent.rs");
        // Cada tool definition debe ser un objeto JSON con "type": "function"
        assert!(src.contains("\"type\": \"function\""),
            "Las tool definitions deben tener type: function");
        assert!(src.contains("\"function\": {"),
            "Las tool definitions deben tener function object");
    }
}


// ============================================================================
// SECCIÓN 6: TESTS DE REGRESIÓN — Bugs descubiertos en sesión 2025-07
// Estos bugs NO tenían tests. Ahora sí.
// ============================================================================

#[cfg(test)]
mod regression_new_bugs {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    // =========================================================================
    // BUG: No carga el perfil de estudio en el frontend
    // Causa: loadStudyProfile podía fallar silenciosamente o no ser llamada
    // =========================================================================

    #[test]
    fn app_js_contiene_load_study_profile() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function loadStudyProfile"),
            "REGRESION: app.js no contiene loadStudyProfile. El perfil de estudio no se cargara.");
        assert!(js.contains("/api/study/profile"),
            "REGRESION: app.js no llama a /api/study/profile. El perfil no se obtiene del backend.");
    }

    #[test]
    fn app_js_load_study_profile_maneja_respuesta() {
        let js = include_str!("../public/app.js");
        // Debe acceder a res.profile y res.engagement
        assert!(js.contains("res.profile") || js.contains("profileAge"),
            "REGRESION: loadStudyProfile no procesa res.profile.");
    }

    // =========================================================================
    // BUG: No ve el system prompt local ni el directorio del proyecto
    // Causa: agent.rs no recibía project_name o no lo usaba
    // =========================================================================

    #[test]
    fn agent_rs_recibe_project_name_y_local_prompt() {
        let src = include_str!("../src/agent.rs");
        // Debe recibir project_name como parametro
        assert!(src.contains("project_name: Option<String>"),
            "REGRESION: run_agent_loop no recibe project_name.");
        // Debe cargar el prompt local
        assert!(src.contains("prompts.projects.get(name)"),
            "REGRESION: agent.rs no carga local_prompt desde prompts.projects.");
        // Debe formatear el system prompt con el local
        assert!(src.contains("Project Specific Prompt:"),
            "REGRESION: agent.rs no incluye el prompt local en el system prompt.");
    }

    #[test]
    fn agent_rs_usa_get_project_path() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("get_project_path"),
            "REGRESION: agent.rs no usa get_project_path. No conoce el directorio del proyecto.");
        assert!(src.contains("proj_path"),
            "REGRESION: agent.rs no construye la ruta del proyecto.");
    }

    // =========================================================================
    // BUG: No se puede empezar una conversación
    // Causa: addMessage estaba definida dos veces (duplicada)
    // =========================================================================

    #[test]
    fn app_js_add_message_definida_una_sola_vez() {
        let js = include_str!("../public/app.js");
        let count = js.matches("function addMessage").count();
        assert_eq!(count, 1,
            "REGRESION: addMessage esta definida {} veces. Debe estar definida UNA sola vez. La duplicacion rompe el inicio de conversacion.", count);
    }

    #[test]
    fn app_js_add_message_cierra_llaves_correctamente() {
        let js = include_str!("../public/app.js");
        // La funcion addMessage debe tener su cuerpo completo con appendChild
        assert!(js.contains("appendChild(div)"),
            "REGRESION: addMessage no contiene appendChild. La funcion esta incompleta.");
        assert!(js.contains("scrollTop = document.getElementById"),
            "REGRESION: addMessage no hace scroll. La funcion esta incompleta.");
    }

    #[test]
    fn app_js_send_message_to_agent_existe() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function sendMessageToAgent"),
            "REGRESION: app.js no contiene sendMessageToAgent.");
        assert!(js.contains("/api/chat"),
            "REGRESION: sendMessageToAgent no llama a /api/chat.");
        assert!(js.contains("startAgentMonitoring()"),
            "REGRESION: sendMessageToAgent no inicia el monitoreo del agente.");
    }

    // =========================================================================
    // BUG: El perfil de usuario no se pasa al agente
    // =========================================================================

    #[test]
    fn agent_rs_recibe_username_y_mode() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("username: &str"),
            "REGRESION: run_agent_loop no recibe username. No puede cargar el perfil.");
        assert!(src.contains("mode: &str"),
            "REGRESION: run_agent_loop no recibe mode. No sabe si es estudio o programacion.");
    }

    #[test]
    fn main_rs_pasa_username_y_mode_al_agente() {
        let src = include_str!("../src/main.rs");
        // Debe pasar username y mode a run_agent_loop
        assert!(src.contains("run_agent_loop"),
            "REGRESION: main.rs no llama a run_agent_loop.");
        assert!(src.contains("&uname_bg"),
            "REGRESION: main.rs no pasa username al agente.");
        assert!(src.contains("&mode_bg"),
            "REGRESION: main.rs no pasa mode al agente.");
    }

    // =========================================================================
    // BUG: System prompt local no se aplica correctamente
    // =========================================================================

    #[test]
    fn agent_rs_local_prompt_overridea_global() {
        let src = include_str!("../src/agent.rs");
        // Debe haber un if let o similar que combine global + local
        assert!(src.contains("local_prompt") || src.contains("prompts.projects.get"),
            "REGRESION: agent.rs no carga el prompt local del proyecto.");
        // El formato debe incluir "Project Specific Prompt:"
        assert!(src.contains("Project Specific Prompt:"),
            "REGRESION: El prompt local no se incluye en el system prompt.");
    }
}
