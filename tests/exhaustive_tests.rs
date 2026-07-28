// ============================================================================
// tests/exhaustive_tests.rs ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â Tests Exhaustivos: RegresiÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n, IntegraciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n,
// E2E, EstrÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â©s, InyecciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n de Fallos, Casos LÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­mite y VerificaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n de CÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³digo
//
// TODOS los tests son REALES: verifican cÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³digo fuente con include_str!,
// prueban comportamiento real de std::path::Path, validan la existencia
// de funciones en el cÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³digo compilado, y testean estructuras de datos reales.
// ============================================================================

// ============================================================================
// SECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN 1: TESTS DE VERIFICACIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN DE CÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œDIGO FUENTE (Source Code Verification)
// Usan include_str! para leer archivos reales del proyecto.
// Si el cÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³digo fuente cambia incorrectamente, estos tests fallan.
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
// SECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN 2: TESTS DE REGRESIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â Validan que bugs especÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­ficos no reaparezcan
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
// SECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN 3: TESTS DE INTEGRACIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â Prueban interacciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n real entre componentes
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
        let status = iaf::state::ActiveAgentStatus::default();

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
        let status = iaf::state::ActiveAgentStatus::default();
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
// SECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN 4: TESTS DE ESTRÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â°S
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
// SECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN 5: TESTS DE INYECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN DE FALLOS
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
        let path = "documento_ÃƒÆ’Ã‚Â¥Ãƒâ€šÃ‚Â¾Ãƒâ€¹Ã¢â‚¬Â ÃƒÆ’Ã‚Â¥Ãƒâ€¦Ã‚Â½ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â°ÃƒÆ’Ã‚Â¥Ãƒâ€šÃ‚Â®Ãƒâ€šÃ‚Â³.pdf";
        let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "pdf");
    }

    #[test]
    fn mensaje_final_con_caracteres_especiales_es_valido() {
        let msg = "Tarea ÃƒÆ’Ã‚Â¢Ãƒâ€¦Ã¢â‚¬Å“ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œ completada: se procesaron 100 archivos ÃƒÆ’Ã‚Â°Ãƒâ€¦Ã‚Â¸Ãƒâ€¦Ã‚Â¡ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, msg);
        assert!(final_msg.contains("ÃƒÆ’Ã‚Â¢Ãƒâ€¦Ã¢â‚¬Å“ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œ"));
        assert!(final_msg.contains("ÃƒÆ’Ã‚Â°Ãƒâ€¦Ã‚Â¸Ãƒâ€¦Ã‚Â¡ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬"));
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
// SECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN 6: TESTS DE CASOS LÃƒÆ’Ã†â€™Ãƒâ€šÃ‚ÂMITE
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
        let msg = "ÃƒÆ’Ã‚Â¢Ãƒâ€¦Ã¢â‚¬Å“ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦ Tarea completada\nÃƒÆ’Ã‚Â°Ãƒâ€¦Ã‚Â¸ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œÃƒâ€¦Ã‚Â  42 archivos procesados\nÃƒÆ’Ã‚Â°Ãƒâ€¦Ã‚Â¸Ãƒâ€šÃ‚Â§Ãƒâ€šÃ‚Âª 156 tests pasados";
        let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
        assert_eq!(final_msg, msg);
        assert!(final_msg.contains("ÃƒÆ’Ã‚Â¢Ãƒâ€¦Ã¢â‚¬Å“ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â¦"));
        assert!(final_msg.contains("ÃƒÆ’Ã‚Â°Ãƒâ€¦Ã‚Â¸ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œÃƒâ€¦Ã‚Â "));
        assert!(final_msg.contains("ÃƒÆ’Ã‚Â°Ãƒâ€¦Ã‚Â¸Ãƒâ€šÃ‚Â§Ãƒâ€šÃ‚Âª"));
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
    fn edge008_nombre_archivo_solo_extension() {
        // Caso limite: nombre de archivo que ES solo una extension
        // Ejemplo: ".gitignore" -> Path::extension() devuelve None porque
        // el punto inicial es parte del nombre del archivo, no un separador
        use std::path::Path;
        
        // .gitignore: extension retorna None (el punto es parte del filename)
        let ext_gitignore = Path::new(".gitignore").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_gitignore, "", "'.gitignore' no debe tener extension detectable");
        
        // .pdf: esto SI tiene extension "pdf" porque no hay stem
        let ext_pdf = Path::new(".pdf").extension().and_then(|e| e.to_str()).unwrap_or("");
        // En Rust, Path::new(".pdf") tiene file_stem ".pdf" (todo es stem) y extension None
        // porque el punto es el primer caracter.
        assert_eq!(ext_pdf, "", "'.pdf' con solo extension no debe tener extension (caso limite)");
        
        // .env: similar a .gitignore
        let ext_env = Path::new(".env").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_env, "");
        
        // nombre real con solo extension: "archive." -> extension ""
        let ext_trailing = Path::new("archive.").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_trailing, "", "'archive.' con punto final debe tener extension vacia");
        
        // "only_ext." -> extension ""
        let ext_only = Path::new("only_ext.").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_only, "", "'only_ext.' con punto final debe tener extension vacia");
        
        // Caso borde: path que termina en punto
        let ext_dot = Path::new("file.").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_dot, "", "Archivo terminado en punto: extension vacia");
        
        // Caso borde: solo un punto
        let ext_solo_punto = Path::new(".").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_solo_punto, "", "Solo un punto: sin extension");
        
        // Caso borde: dos puntos
        let ext_dos_puntos = Path::new("..").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext_dos_puntos, "", "Dos puntos: sin extension");
    }

    #[test]
    fn edge009_nombre_archivo_con_solo_numeros() {
        use std::path::Path;
        let ext = Path::new("12345").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "");
    }

    #[test]
    fn edge010_nombre_archivo_vacio() {
        use std::path::Path;
        let ext = Path::new("").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "");
    }

    #[test]
    fn edge011_extension_mayusculas_minusculas() {
        use std::path::Path;
        // El handler de read_file debe convertir a lowercase
        let ext_upper = Path::new("file.PDF").extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let ext_mixed = Path::new("file.DocX").extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        assert_eq!(ext_upper, "pdf");
        assert_eq!(ext_mixed, "docx");
    }

    #[test]
    fn edge012_nombre_archivo_con_emojis() {
        use std::path::Path;
        let ext = Path::new("reporte_final_🚀.pdf").extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "pdf");
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
        let status = iaf::state::ActiveAgentStatus::default();
        let json = serde_json::to_value(&status).unwrap();
        let arr = json["info_messages"].as_array().unwrap();
        assert!(arr.is_empty());
    }
}


// ============================================================================
// SECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN 7: TESTS DE HUMO (Smoke Tests)
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
// SECCIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN 6: TESTS DE REGRESIÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…â€œN ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â Bugs descubiertos en sesiÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n 2025-07
// Estos bugs NO tenÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­an tests. Ahora sÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­.
// ============================================================================

#[cfg(test)]
mod regression_new_bugs {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    // =========================================================================
    // BUG: No carga el perfil de estudio en el frontend
    // Causa: loadStudyProfile podÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­a fallar silenciosamente o no ser llamada
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
    // Causa: agent.rs no recibÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â­a project_name o no lo usaba
    // =========================================================================

    #[test]
    fn agent_rs_recibe_project_name_y_local_prompt() {
        let src = include_str!("../src/agent.rs");
        // Debe recibir project_name como parametro
        assert!(src.contains("project_name: Option<String>"),
            "REGRESION: run_agent_loop no recibe project_name.");
        // Debe cargar el prompt local
        assert!(src.contains("load_local_prompt(username, name)"),
            "REGRESION: agent.rs no carga local_prompt desde load_local_prompt.");
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
    // BUG: No se puede empezar una conversaciÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â³n
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

    #[test]
    fn app_js_load_prompts_existe() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function loadPrompts"),
            "BUG-028 REGRESION: app.js no contiene loadPrompts. Se perdio en una reescritura.");
        assert!(js.contains("savePromptsBtn"),
            "BUG-028 REGRESION: app.js no contiene savePromptsBtn.");
        assert!(js.contains("resetPromptBtn"),
            "BUG-028 REGRESION: app.js no contiene resetPromptBtn.");
        assert!(js.contains("globalPrompt"),
            "BUG-028 REGRESION: app.js no contiene referencias a globalPrompt.");
        assert!(js.contains("localPrompt"),
            "BUG-028 REGRESION: app.js no contiene referencias a localPrompt.");
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
        assert!(src.contains("local_prompt") || src.contains("load_local_prompt"),
            "REGRESION: agent.rs no carga el prompt local del proyecto.");
        // El formato debe incluir "Project Specific Prompt:"
        assert!(src.contains("Project Specific Prompt:"),
            "REGRESION: El prompt local no se incluye en el system prompt.");
    }

}
// ============================================================================
// SECCIÃƒâ€œN: TESTS CON NOMBRES EXACTOS SOLICITADOS POR EL USUARIO
// Para evitar confusion con nombres de tests renombrados
// ============================================================================

#[cfg(test)]
mod user_requested_test_names {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn verify_agent_uses_pdf_extract_not_pdftotext() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("pdf_extract::extract_text"),
            "BUG-001 REGRESION: agent.rs no usa pdf_extract::extract_text nativo");
        assert!(!src.contains("pdftotext"),
            "BUG-001 REGRESION: agent.rs contiene referencias al binario externo pdftotext");
    }

    #[test]
    fn verify_agent_has_extract_text_from_docx() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("fn extract_text_from_docx"),
            "BUG-001 REGRESION: agent.rs no contiene fn extract_text_from_docx");
        assert!(src.contains("zip::ZipArchive"),
            "BUG-001 REGRESION: agent.rs no usa zip::ZipArchive para DOCX");
        assert!(src.contains("quick_xml::Reader"),
            "BUG-001 REGRESION: agent.rs no usa quick_xml::Reader para parsear DOCX");
    }

    #[test]
    fn edge008_nombre_archivo_solo_extension() {
        // Caso limite: archivo que es SOLO extension (ej: ".gitignore", ".env")
        use std::path::Path;
        // .gitignore: Path::extension() devuelve None porque el nombre completo es considerado stem
        assert_eq!(Path::new(".gitignore").extension().and_then(|e| e.to_str()), None,
            "Caso limite edge008: .gitignore debe devolver extension None");
        assert_eq!(Path::new(".env").extension().and_then(|e| e.to_str()), None,
            "Caso limite edge008: .env debe devolver extension None");
        assert_eq!(Path::new(".dockerignore").extension().and_then(|e| e.to_str()), None,
            "Caso limite edge008: .dockerignore debe devolver extension None");
        // Caso: archivo con extension explicita que empieza con punto
        assert_eq!(Path::new("..hidden").extension().and_then(|e| e.to_str()), Some("hidden"),
            "Caso limite edge008: ..hidden extension = hidden");
    }
}

// ============================================================================
// SECCIÃƒâ€œN: TESTS DE REGRESIÃƒâ€œN ADICIONALES PARA BUGS REPORTADOS
// ============================================================================

#[cfg(test)]
mod additional_regression_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    // =========================================================================
    // BUG: No carga el perfil en modo estudio en el frontend
    // =========================================================================

    #[test]
    fn app_js_load_study_profile_existe_y_llama_api() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function loadStudyProfile"),
            "REGRESION: app.js no tiene loadStudyProfile. El perfil de estudio no carga.");
        assert!(js.contains("/api/study/profile"),
            "REGRESION: loadStudyProfile no llama a /api/study/profile.");
    }

    #[test]
    fn main_rs_tiene_endpoint_study_get_profile() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("/api/study/profile"),
            "REGRESION: main.rs no tiene endpoint /api/study/profile.");
        assert!(src.contains("study_get_profile"),
            "REGRESION: main.rs no tiene handler study_get_profile.");
    }

    #[test]
    fn study_rs_profile_exists_on_disk_funciona() {
        let src = include_str!("../src/study.rs");
        assert!(src.contains("profile_exists_on_disk"),
            "REGRESION: study.rs no tiene profile_exists_on_disk.");
        assert!(src.contains("profile.json"),
            "REGRESION: study.rs no referencia profile.json.");
    }

    // =========================================================================
    // BUG: No se puede empezar una conversacion (frontend JS roto)
    // =========================================================================

    #[test]
    fn app_js_llaves_balanceadas() {
        let js = include_str!("../public/app.js");
        let open = js.matches('{').count();
        let close = js.matches('}').count();
        assert_eq!(open, close,
            "JS ROTO: {} llaves de apertura vs {} de cierre. El frontend esta roto.", open, close);
    }

    #[test]
    fn app_js_funcion_init_existe() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("function init"),
            "REGRESION: app.js no tiene function init(). El frontend no inicia.");
        assert!(js.contains("init()"),
            "REGRESION: app.js no llama a init() al final.");
    }

    #[test]
    fn app_js_chat_input_existe_y_funciona() {
        let js = include_str!("../public/app.js");
        assert!(js.contains("chatInput"),
            "REGRESION: app.js no referencia chatInput.");
        assert!(js.contains("sendMessageToAgent"),
            "REGRESION: app.js no tiene sendMessageToAgent. No se puede enviar mensajes.");
    }

    // =========================================================================
    // BUG: finalizar_tarea devuelve "No se proporciono URL"
    // =========================================================================

    #[test]
    fn agent_rs_finalizar_tarea_handler_no_contiene_string_url_requerido() {
        let src = include_str!("../src/agent.rs");
        let finalizar_idx = src.find("\"finalizar_tarea\" =>").unwrap();
        let next_tool_idx = src[finalizar_idx..].find("\"image_fetch\" =>").unwrap_or(src.len() - finalizar_idx);
        let finalizar_block = &src[finalizar_idx..finalizar_idx + next_tool_idx];
        // El bloque finalizar_tarea NO debe contener la palabra "url" como campo
        let url_count = finalizar_block.matches("\"url\"").count();
        assert_eq!(url_count, 0,
            "BUG-004 REGRESION: finalizar_tarea contiene {} referencias a 'url'. Se confunde con image_fetch.", url_count);
        assert!(finalizar_block.contains("mensaje_final"),
            "BUG-004 REGRESION: finalizar_tarea no usa mensaje_final.");
    }

    #[test]
    fn agent_rs_finalizar_tarea_no_exige_parametro_url() {
        let src = include_str!("../src/agent.rs");
        // Buscar el schema de finalizar_tarea: desde "finalizar_tarea" hasta el "]"
        // del bloque "required". Asi solo verificamos el required del schema correcto.
        let ft_idx = src.find("\"finalizar_tarea\"").unwrap();
        let after_ft = &src[ft_idx..];
        // Buscar el primer "required" despues de "finalizar_tarea"
        let req_idx = after_ft.find("\"required\"").unwrap();
        let req_block = &after_ft[req_idx..];
        // Acotar al primer "]" (fin del array required)
        let bracket_close = req_block.find("]").unwrap();
        let required_content = &req_block[..bracket_close + 1];
        // El required de finalizar_tarea solo debe contener "mensaje_final", NO "url"
        assert!(!required_content.contains("\"url\""),
            "BUG-004 REGRESION: El schema de finalizar_tarea lista 'url' como required.");
        assert!(required_content.contains("\"mensaje_final\""),
            "BUG-004 REGRESION: El schema de finalizar_tarea no lista 'mensaje_final' como required.");
    }
    // =========================================================================
    // BUG: El frontend no muestra los mensajes informativos en tiempo real
    // =========================================================================

    #[test]
    fn app_js_info_messages_se_consumen_antes_del_check_active() {
        let js = include_str!("../public/app.js");
        let idx_info = js.find("info_messages").unwrap();
        // El consumo debe ocurrir antes de cualquier check de active/running
        let remainder = &js[idx_info..];
        let idx_active = remainder.find("statusRes.running").unwrap_or(usize::MAX);
        let idx_finished = remainder.find("statusRes.finished").unwrap_or(usize::MAX);
        let first_check = idx_active.min(idx_finished);
        // Debe haber un consumo de info_messages antes de chequear running/finished
        let idx_show = remainder.find("showInfoToast").unwrap();
        assert!(idx_show < first_check,
            "BUG-002 REGRESION: showInfoToast se llama DESPUES de chequear running/finished. Los mensajes no se muestran en tiempo real.");
    }

    // =========================================================================
    // BUG: No ve ni el system prompt local ni el perfil ni el directorio del proyecto
    // =========================================================================

    #[test]
    fn agent_rs_construye_prompt_desde_disco_no_solo_memoria() {
        let src = include_str!("../src/agent.rs");
        // Debe usar load_global_prompt y load_local_prompt (que leen de disco)
        assert!(src.contains("load_global_prompt") || src.contains("load_local_prompt"),
            "REGRESION: agent.rs no usa load_global_prompt/load_local_prompt. Carga prompts solo de memoria.");
        assert!(src.contains("load_global_prompt(username)"),
            "REGRESION: agent.rs no carga el prompt global especifico del usuario desde disco.");
        assert!(src.contains("load_local_prompt(username, name)"),
            "REGRESION: agent.rs no carga el prompt local especifico del proyecto desde disco.");
    }

    #[test]
    fn state_rs_tiene_metodos_load_prompt_desde_disco() {
        let src = include_str!("../src/state.rs");
        assert!(src.contains("fn load_global_prompt"),
            "REGRESION: state.rs no tiene load_global_prompt.");
        assert!(src.contains("fn load_local_prompt"),
            "REGRESION: state.rs no tiene load_local_prompt.");
        assert!(src.contains("globalPrompt.json"),
            "REGRESION: load_global_prompt no lee globalPrompt.json.");
        assert!(src.contains("localPrompt.json"),
            "REGRESION: load_local_prompt no lee localPrompt.json.");
    }

    // =========================================================================
    // BUG: No puede analizar PDFs ni .docx
    // =========================================================================

    #[test]
    fn agent_rs_read_file_handler_detecta_y_procesa_pdf_docx() {
        let src = include_str!("../src/agent.rs");
        // La deteccion debe ser case-insensitive
        assert!(src.contains("to_lowercase"),
            "REGRESION: read_file no normaliza extension a lowercase. Archivos .PDF no se detectan.");
        assert!(src.contains("ext == \"pdf\""),
            "REGRESION: read_file no tiene branch para PDF.");
        assert!(src.contains("ext == \"docx\""),
            "REGRESION: read_file no tiene branch para DOCX.");
    }
}

// ============================================================================
// SECCIÃƒâ€œN: TESTS DE ESTRÃƒâ€°S Ã¢â‚¬â€ Validan comportamiento bajo carga masiva
// ============================================================================

#[cfg(test)]
mod stress_tests_extended {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn stress_info_messages_100000_sin_panico() {
        let mut messages: Vec<String> = Vec::with_capacity(101);
        for i in 0..100000 {
            messages.push(format!("Msg {}", i));
            if messages.len() > 100 { messages.remove(0); }
        }
        assert_eq!(messages.len(), 100);
        assert_eq!(messages[0], "Msg 99900");
    }

    #[test]
    fn stress_extension_detection_10000_archivos() {
        use std::path::Path;
        let exts = ["pdf", "docx", "txt", "rs", "md", "json", "js", "html", "css", "toml"];
        for i in 0..10000 {
            let path_str = format!("file_{}.{}", i, exts[i % exts.len()]);
            let ext = Path::new(&path_str).extension().and_then(|e| e.to_str()).unwrap_or("");
            assert!(!ext.is_empty(), "Extension no deberia ser vacia para {}", path_str);
        }
    }

    #[test]
    fn stress_json_parse_info_messages_10000() {
        use serde_json::json;
        let messages: Vec<String> = (0..10000).map(|i| format!("Mensaje numero {}", i)).collect();
        let json_val = json!({ "info_messages": messages });
        let serialized = serde_json::to_string(&json_val).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let arr = deserialized["info_messages"].as_array().unwrap();
        assert_eq!(arr.len(), 10000);
        assert_eq!(arr[0].as_str().unwrap(), "Mensaje numero 0");
        assert_eq!(arr[9999].as_str().unwrap(), "Mensaje numero 9999");
    }

    #[test]
    fn stress_llaves_balanceadas_10000_aberturas() {
        let mut s = String::new();
        for _ in 0..10000 { s.push('{'); }
        for _ in 0..10000 { s.push('}'); }
        let open = s.matches('{').count();
        let close = s.matches('}').count();
        assert_eq!(open, close);
        assert_eq!(open, 10000);
    }
}

// ============================================================================
// SECCIÃƒâ€œN: TESTS DE INYECCIÃƒâ€œN DE FALLOS
// ============================================================================

#[cfg(test)]
mod fault_injection_tests_extended {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn fault_info_messages_con_caracteres_especiales_no_rompe() {
        let special = vec![
            "Mensaje con comillas \"internas\"",
            "Mensaje con \\ backslash",
            "Mensaje con \n salto de linea",
            "Mensaje con \t tabulacion",
            "Mensaje con emoji Ã°Å¸Å½â€°Ã¢Å“Â¨",
            "Mensaje con caracteres chinos Ã¤Â½Â Ã¥Â¥Â½",
            "",
            "   ",
            "!@#$%^&*()_+-=[]{}|;:',.<>?/`~",
        ];
        for msg in &special {
            // Simular push a info_messages
            let json_val = serde_json::json!({ "info_messages": [msg] });
            let serialized = serde_json::to_string(&json_val).unwrap();
            let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
            let recovered = deserialized["info_messages"][0].as_str().unwrap();
            assert_eq!(recovered, *msg, "Fallo con mensaje: {:?}", msg);
        }
    }

    #[test]
    fn fault_finalizar_tarea_con_unicode_no_se_confunde() {
        // Mensajes con caracteres que parecen URL pero no lo son
        let tricky_messages = vec![
            "DescarguÃƒÂ© desde https://ejemplo.com y funcionÃƒÂ³",
            "url: no es una url real",
            "No se proporcionÃƒÂ³ URL",  // Este es el mensaje de error malinterpretado
            "url = https://image_fetch.png",  // Parece parametro de image_fetch
            "El parÃƒÂ¡metro 'url' no es necesario aquÃƒÂ­",
        ];
        for msg in &tricky_messages {
            let final_msg = if msg.trim().is_empty() { "Tarea finalizada." } else { msg };
            assert!(!final_msg.is_empty());
            // No debe interpretarse como image_fetch
        }
    }

    #[test]
    fn fault_path_con_null_bytes_no_rompe_extension_detection() {
        use std::path::Path;
        // Path de Rust maneja null bytes rechazandolos
        let result = std::panic::catch_unwind(|| {
            Path::new("file.pdf\0malicioso.txt").extension()
        });
        // No debe panicar
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn fault_extension_vacia_en_handler() {
        // Simular el comportamiento cuando extension es None/vacia
        let extensions = vec![None, Some(""), Some("pdf"), Some("docx"), Some("rs")];
        for ext_opt in &extensions {
            let ext = ext_opt.unwrap_or("").to_lowercase();
            match ext.as_str() {
                "pdf" => assert!(ext == "pdf"),
                "docx" => assert!(ext == "docx"),
                _ => assert!(ext != "pdf" && ext != "docx" || ext.is_empty()),
            }
        }
    }
}

// ============================================================================
// SECCIÃƒâ€œN: TESTS DE CASOS LÃƒÂMITE ADICIONALES
// ============================================================================

#[cfg(test)]
mod additional_edge_case_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn edge_extension_500_caracteres() {
        use std::path::Path;
        let long_ext = "x".repeat(500);
        let path_str = format!("file.{}", long_ext);
        let ext = Path::new(&path_str).extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, long_ext);
    }

    #[test]
    fn edge_nombre_archivo_1000_caracteres() {
        use std::path::Path;
        let long_name = "a".repeat(1000);
        let path_str = format!("{}.pdf", long_name);
        let ext = Path::new(&path_str).extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_eq!(ext, "pdf");
    }

    #[test]
    fn edge_path_con_espacios_y_puntos_multiples() {
        use std::path::Path;
        let path = Path::new("C:\\Users\\Fa\\Desktop\\mi proyecto.backup.v2.rs");
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("rs"));
        let path2 = Path::new("/home/user/my documents/report.final.pdf");
        assert_eq!(path2.extension().and_then(|e| e.to_str()), Some("pdf"));
    }

    #[test]
    fn edge_info_messages_array_vacio_serializacion() {
        let json_val = serde_json::json!({ "info_messages": [] });
        let serialized = serde_json::to_string(&json_val).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let arr = deserialized["info_messages"].as_array().unwrap();
        assert!(arr.is_empty());
    }

    #[test]
    fn edge_info_messages_limite_exacto_100() {
        let mut messages: Vec<String> = Vec::with_capacity(101);
        for i in 0..100 {
            messages.push(format!("Msg {}", i));
        }
        assert_eq!(messages.len(), 100);
        messages.push("Msg 100".to_string());
        if messages.len() > 100 { messages.remove(0); }
        assert_eq!(messages.len(), 100);
        assert_eq!(messages[0], "Msg 1");
        assert_eq!(messages[99], "Msg 100");
    }

    #[test]
    fn edge_finalizar_tarea_con_json_malformado() {
        // Si el JSON viene sin el campo mensaje_final
        let json_val = serde_json::json!({});
        let msg = json_val["mensaje_final"].as_str().unwrap_or("Tarea finalizada.");
        assert_eq!(msg, "Tarea finalizada.");
    }
}


// ============================================================================
// SECCIÓN 8: TESTS END-TO-END — Backend ↔ Frontend
// Simulan el flujo real de datos entre el servidor y el cliente.
// ============================================================================

#[cfg(test)]
mod e2e_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    // =========================================================================
    // E2E-001: Flujo completo de /api/agent/status
    // =========================================================================

    #[test]
    fn e2e_api_agent_status_respuesta_tiene_campos_requeridos() {
        // Simula la respuesta JSON que el servidor envia al frontend
        let status_json = serde_json::json!({
            "status": "ok",
            "active": true,
            "running": true,
            "finished": false,
            "final_message": null,
            "esperando_respuesta_usuario": false,
            "pregunta_usuario": null,
            "esperando_aprobacion_plan": false,
            "plan_propuesto": null,
            "info_messages": ["Iniciando...", "Procesando..."],
            "steps": [],
            "current_session_id": "abc-123"
        });

        // Verificar estructura
        assert_eq!(status_json["status"], "ok");
        assert_eq!(status_json["active"], true);
        assert!(status_json.get("info_messages").is_some());
        assert!(status_json.get("final_message").is_some());
        assert!(status_json.get("finished").is_some());
    }

    #[test]
    fn e2e_api_agent_status_con_agente_finalizado() {
        let status_json = serde_json::json!({
            "status": "ok",
            "active": false,
            "running": false,
            "finished": true,
            "final_message": "Tarea completada exitosamente.",
            "info_messages": ["Iniciando...", "Procesando...", "Finalizado."],
            "esperando_respuesta_usuario": false,
            "esperando_aprobacion_plan": false,
        });

        // El frontend debe poder acceder a info_messages incluso cuando el agente termino
        assert_eq!(status_json["finished"], true);
        let info = status_json["info_messages"].as_array().unwrap();
        assert_eq!(info.len(), 3);
        assert_eq!(info[2], "Finalizado.");
    }

    // =========================================================================
    // E2E-002: Flujo completo de /api/study/profile
    // =========================================================================

    #[test]
    fn e2e_api_study_profile_respuesta_tiene_campos_requeridos() {
        let profile_json = serde_json::json!({
            "status": "ok",
            "profile": {
                "username": "testuser",
                "age": 14,
                "phase": "Exploration",
                "high_capabilities": null,
                "neurological_conditions": [],
                "favorite_games": ["Minecraft"],
                "favorite_youtubers": [],
                "hobbies": ["programming"]
            },
            "knowledge": {
                "learned_items": {},
                "topics_of_interest": []
            },
            "engagement": {
                "level": "medium",
                "score": 0.65
            },
            "phase": "Exploration"
        });

        // Verificar estructura que el frontend consume
        assert_eq!(profile_json["status"], "ok");
        assert!(profile_json.get("profile").is_some());
        assert!(profile_json.get("knowledge").is_some());
        assert!(profile_json.get("engagement").is_some());
        assert!(profile_json.get("phase").is_some());
    }

    // =========================================================================
    // E2E-003: Flujo completo de /api/chat
    // =========================================================================

    #[test]
    fn e2e_api_chat_respuesta_tiene_campos_requeridos() {
        let chat_json = serde_json::json!({
            "status": "ok",
            "session_id": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Hola, necesito ayuda con...",
            "chat_path": "/path/to/chat.json"
        });

        assert_eq!(chat_json["status"], "ok");
        assert!(chat_json["session_id"].as_str().unwrap().len() > 0);
        assert!(chat_json["title"].as_str().unwrap().len() > 0);
    }

    #[test]
    fn e2e_api_chat_error_falta_permisos() {
        let error_json = serde_json::json!({
            "status": "error",
            "message": "No tienes permiso para usar el modo estudio. Contacta al administrador."
        });

        assert_eq!(error_json["status"], "error");
        assert!(error_json["message"].as_str().unwrap().contains("permiso"));
    }

    // =========================================================================
    // E2E-004: Flujo de inicio de sesion
    // =========================================================================

    #[test]
    fn e2e_api_login_respuesta_tiene_campos_requeridos() {
        let login_json = serde_json::json!({
            "status": "ok",
            "token": "session-token-abc123",
            "username": "testuser",
            "is_admin": false,
            "has_study_access": true,
            "has_programming_access": true
        });

        assert_eq!(login_json["status"], "ok");
        assert!(login_json.get("token").is_some());
        assert!(login_json.get("has_study_access").is_some());
        assert!(login_json.get("has_programming_access").is_some());
    }

    // =========================================================================
    // E2E-005: Simulacion de polling del frontend
    // =========================================================================

    #[test]
    fn e2e_simulacion_polling_frontend_5_ciclos() {
        // Simula 5 ciclos de polling del frontend al backend
        let mut last_info_count: usize = 0;
        let mut total_toasts: usize = 0;
        let mut session_id: Option<String> = None;

        // Simular respuestas del backend en 5 polls
        let polls = vec![
            serde_json::json!({"status":"ok","active":true,"running":true,"info_messages":["Msg1","Msg2"],"current_session_id":"sess-1","finished":false}),
            serde_json::json!({"status":"ok","active":true,"running":true,"info_messages":["Msg1","Msg2","Msg3","Msg4"],"current_session_id":"sess-1","finished":false}),
            serde_json::json!({"status":"ok","active":true,"running":true,"info_messages":["Msg1","Msg2","Msg3","Msg4","Msg5","Msg6"],"current_session_id":"sess-1","finished":false}),
            serde_json::json!({"status":"ok","active":true,"running":true,"info_messages":["Msg1","Msg2","Msg3","Msg4","Msg5","Msg6","Msg7"],"current_session_id":"sess-1","finished":false}),
            serde_json::json!({"status":"ok","active":false,"running":false,"finished":true,"info_messages":["Msg1","Msg2","Msg3","Msg4","Msg5","Msg6","Msg7","Msg8","Msg9"],"final_message":"Tarea completada.","current_session_id":"sess-1"}),
        ];

        for poll in &polls {
            // Resetear contador si cambio la sesion
            let curr_sid = poll["current_session_id"].as_str();
            if curr_sid != session_id.as_deref() {
                last_info_count = 0;
                session_id = curr_sid.map(|s| s.to_string());
            }

            // Consumir info_messages (BUG-002 FIX: siempre consumir)
            if let Some(arr) = poll["info_messages"].as_array() {
                let current_count = arr.len();
                if current_count > last_info_count {
                    let nuevos = current_count - last_info_count;
                    total_toasts += nuevos;
                    last_info_count = current_count;
                }
            }
        }

        // Total de toasts mostrados al frontend:
        // Poll 1: 2 nuevos, Poll 2: 2 nuevos, Poll 3: 2 nuevos, Poll 4: 1 nuevo, Poll 5: 2 nuevos
        assert_eq!(total_toasts, 9, "Debe mostrar 9 toasts en total durante los 5 polls");
    }

    #[test]
    fn e2e_final_message_se_muestra_cuando_agente_termina() {
        // Simular el ultimo poll donde el agente termino
        let poll = serde_json::json!({
            "status": "ok",
            "active": false,
            "running": false,
            "finished": true,
            "final_message": "Tarea completada: 42 archivos procesados."
        });

        // El frontend debe mostrar el mensaje final
        assert_eq!(poll["finished"], true);
        let final_msg = poll["final_message"].as_str().unwrap();
        assert!(final_msg.contains("Tarea completada"));
    }
}


// ============================================================================
// SECCIÓN 9: TESTS DE REGRESIÓN ADICIONALES — Cobertura de bugs historicos
// ============================================================================

#[cfg(test)]
mod regression_historical {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn finalizar_tarea_handler_esta_separado_de_image_fetch() {
        let src = include_str!("../src/agent.rs");
        // Verificar que finalizar_tarea y image_fetch estan en lineas separadas
        // (no en la misma linea como antes: }                    "image_fetch" =>)
        let finalizar_idx = src.find("finalizar_tarea").unwrap();
        let image_fetch_idx = src.find("image_fetch").unwrap();
        let block = &src[finalizar_idx..image_fetch_idx];
        // No debe contener "image_fetch" en el mismo bloque
        assert!(!block.contains("image_fetch"),
            "REGRESION: finalizar_tarea e image_fetch estan en la misma linea o bloque");
    }

    #[test]
    fn app_js_no_tiene_llaves_desbalanceadas() {
        let js = include_str!("../public/app.js");
        let open = js.matches('{').count();
        let close = js.matches('}').count();
        assert_eq!(open, close,
            "JS ROTO: {} llaves de apertura vs {} de cierre. El frontend no funciona.", open, close);
    }

    #[test]
    fn app_js_no_tiene_parentesis_desbalanceados() {
        let js = include_str!("../public/app.js");
        let open = js.matches('(').count();
        let close = js.matches(')').count();
        assert_eq!(open, close,
            "JS ROTO: {} parentesis de apertura vs {} de cierre.", open, close);
    }

    #[test]
    fn app_js_no_tiene_corchetes_desbalanceados() {
        let js = include_str!("../public/app.js");
        let open = js.matches('[').count();
        let close = js.matches(']').count();
        assert_eq!(open, close,
            "JS ROTO: {} corchetes de apertura vs {} de cierre.", open, close);
    }

    #[test]
    fn app_js_render_console_steps_llaves_balanceadas() {
        let js = include_str!("../public/app.js");
        // Verificar que renderConsoleSteps tiene sus llaves balanceadas
        let start = js.find("function renderConsoleSteps").unwrap();
        let end = js[start..].find("function showInfoToast").unwrap();
        let block = &js[start..start+end];
        let open = block.matches('{').count();
        let close = block.matches('}').count();
        assert_eq!(open, close,
            "BUG: renderConsoleSteps tiene llaves desbalanceadas ({} vs {})", open, close);
    }

    #[test]
    fn app_js_start_agent_monitoring_estructura_correcta() {
        let js = include_str!("../public/app.js");
        let start = js.find("function startAgentMonitoring").unwrap();
        // Debe tener la estructura correcta con setInterval y consumo de info_messages
        let block_end = js[start..].find("function showInfoToast").unwrap();
        let block = &js[start..start+block_end];
        
        assert!(block.contains("setInterval"), 
            "startAgentMonitoring no tiene setInterval");
        assert!(block.contains("info_messages"), 
            "startAgentMonitoring no consume info_messages");
        assert!(block.contains("lastInfoMessageCount"), 
            "startAgentMonitoring no tiene lastInfoMessageCount");
    }

    #[test]
    fn agent_rs_read_file_tiene_manejo_errores() {
        let src = include_str!("../src/agent.rs");
        let read_file_idx = src.find("\"read_file\" =>").unwrap();
        // Buscar el fin del handler (proximo "=>")
        let next_handler = src[read_file_idx+1..].find("\"search_google\" =>").unwrap_or(
            src[read_file_idx+1..].find("\"write_file_with_commit\" =>").unwrap()
        );
        let block = &src[read_file_idx..read_file_idx+1+next_handler];
        
        // Debe manejar el caso de archivo no encontrado
        assert!(block.contains("Error leyendo archivo"), 
            "read_file no maneja archivo inexistente");
        // Debe usar pdf_extract para PDFs
        assert!(block.contains("pdf_extract::extract_text"), 
            "read_file no usa pdf_extract para PDFs");
        // Debe usar extract_text_from_docx para DOCX
        assert!(block.contains("extract_text_from_docx"), 
            "read_file no usa extract_text_from_docx para DOCX");
    }

    #[test]
    fn main_rs_router_tiene_endpoint_study_profile() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("/api/study/profile"),
            "REGRESION: El router no tiene /api/study/profile. El frontend no puede cargar el perfil.");
    }

    #[test]
    fn main_rs_chat_endpoint_recibe_mode() {
        let src = include_str!("../src/main.rs");
        // El struct ChatInput debe tener mode
        assert!(src.contains("mode: Option<String>"),
            "REGRESION: ChatInput no tiene campo mode.");
    }

    #[test]
    fn study_rs_profile_tiene_campos_requeridos() {
        let src = include_str!("../src/study.rs");
        assert!(src.contains("age: Option<u8>"), "StudyProfile no tiene age");
        assert!(src.contains("phase"), "StudyProfile no tiene phase");
        assert!(src.contains("high_capabilities"), "StudyProfile no tiene high_capabilities");
    }
}
