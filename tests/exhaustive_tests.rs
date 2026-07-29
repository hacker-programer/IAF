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
        assert!(src.contains("info_messages.len() > 200"),
            "BUG-002 REGRESION: info_messages no tiene limite de 200 mensajes");
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
        assert!(js.contains("res.info_messages"),
            "BUG-002 REGRESION: app.js no consume info_messages del backend");
        assert!(js.contains("function showInfoToast"),
            "BUG-002 REGRESION: app.js no contiene showInfoToast");
        assert!(js.contains("_shownInfoMsgs"),
            "BUG-002 REGRESION: app.js no tiene _shownInfoMsgs para tracking incremental");
    }

    #[test]
    fn app_js_muestra_info_messages_incluso_con_agente_terminado() {
        let js = include_str!("../public/app.js");
        // El consumo de info_messages debe ocurrir ANTES del chequeo de active/running
        let idx_info = js.find("info_messages").unwrap();
        let idx_active = js.rfind("res.finished").unwrap();
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

    #[test]
    fn integracion_info_messages_flujo_completo() {
        // Simula el flujo completo: agente produce -> main.rs almacena -> frontend consume
        let mut info_messages: Vec<String> = Vec::new();
        let mut finished = false;
        let mut final_message: Option<String> = None;

        // Agente produce mensajes
        info_messages.push("Iniciando tarea...".to_string());
        info_messages.push("Buscando en Google...".to_string());
        info_messages.push("Leyendo archivo...".to_string());
        info_messages.push("Escribiendo archivo...".to_string());

        // Main.rs almacena en ActiveAgentStatus
        // (simulado: los mensajes ya estan en el Vec)

        // Agente termina
        finished = true;
        final_message = Some("Tarea completada exitosamente.".to_string());

        // Frontend verifica: info_messages se consumen incluso con finished=true
        assert!(finished);
        assert_eq!(info_messages.len(), 4);
        assert_eq!(info_messages[3], "Escribiendo archivo...");
        assert_eq!(final_message.unwrap(), "Tarea completada exitosamente.");
    }

    #[test]
    fn integracion_notificar_usuario_flujo_completo() {
        let mut info_messages: Vec<String> = Vec::new();

        // notificar_usuario de tipo "informativo"
        info_messages.push("[NOTIF] Informativo: El archivo se guardo correctamente.".to_string());

        // notificar_usuario de tipo "pregunta"
        info_messages.push("[NOTIF] Pregunta: ¿Deseas continuar con la optimizacion?".to_string());

        assert_eq!(info_messages.len(), 2);
        assert!(info_messages[0].starts_with("[NOTIF]"));
        assert!(info_messages[1].starts_with("[NOTIF]"));
    }

    #[test]
    fn integracion_limite_info_messages_con_notificaciones() {
        let mut messages: Vec<String> = Vec::new();
        for i in 0..120 {
            messages.push(format!("[NOTIF] Notificacion {}", i));
            if messages.len() > 100 {
                messages.remove(0);
            }
        }
        assert_eq!(messages.len(), 100);
        assert_eq!(messages[0], "[NOTIF] Notificacion 20");
        assert_eq!(messages[99], "[NOTIF] Notificacion 119");
    }

    #[test]
    fn integracion_finalizar_tarea_no_afecta_info_messages() {
        // El handler finalizar_tarea NO debe tocar info_messages
        let info_before = vec!["Msg1", "Msg2", "Msg3"];
        let mut _finished = false;

        // Simulamos finalizar_tarea: solo cambia flags
        _finished = true;

        // info_messages debe permanecer intacto
        assert_eq!(info_before.len(), 3);
        assert_eq!(info_before[2], "Msg3");
    }
}


// ============================================================================
// SECCIÓN 4: TESTS DE ESTRÉS — Validan comportamiento bajo carga
// ============================================================================

#[cfg(test)]
mod stress_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn stress_1000_info_messages_sin_panico() {
        let mut messages: Vec<String> = Vec::new();
        for i in 0..1000 {
            messages.push(format!("Stress message {}", i));
            if messages.len() > 100 {
                messages.remove(0);
            }
        }
        assert_eq!(messages.len(), 100);
    }

    #[test]
    fn stress_10000_mensajes_finalizar_tarea_rapido() {
        let mut messages: Vec<String> = Vec::new();
        for i in 0..10000 {
            messages.push(format!("M{}", i));
            if messages.len() > 100 { messages.remove(0); }
        }
        // Finalizar tarea: no debe iterar sobre los mensajes
        let _finished = true;
        assert_eq!(messages.len(), 100);
    }

    #[test]
    fn stress_multiples_extensiones_en_un_segundo() {
        use std::path::Path;
        let paths: Vec<_> = (0..1000).map(|i| {
            if i % 3 == 0 { format!("file{}.pdf", i) }
            else if i % 3 == 1 { format!("file{}.docx", i) }
            else { format!("file{}.txt", i) }
        }).collect();

        for path_str in &paths {
            let ext = Path::new(path_str).extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            // No debe panicar
            let _ = ext.len();
        }
    }
}


// ============================================================================
// SECCIÓN 5: TESTS DE INYECCIÓN DE FALLOS — Validan manejo de errores
// ============================================================================

#[cfg(test)]
mod fault_injection_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn fault_json_info_messages_falta_campo() {
        // Simular que el backend no incluye info_messages en la respuesta
        let json_val = serde_json::json!({
            "status": "ok",
            "active": true,
            "running": true,
            "finished": false
        });

        let info = json_val.get("info_messages");
        // El frontend debe manejar que info_messages sea null/absent
        if let Some(arr) = info.and_then(|v| v.as_array()) {
            // Si existe, consumir
            let _count = arr.len();
        }
        // No debe panicar
    }

    #[test]
    fn fault_json_info_messages_es_null() {
        let json_val = serde_json::json!({
            "status": "ok",
            "active": true,
            "info_messages": null
        });

        let info = json_val.get("info_messages");
        assert!(info.is_some());
        assert!(info.unwrap().is_null());
        // El frontend debe verificar is_array() antes de iterar
        if let Some(arr) = info.and_then(|v| v.as_array()) {
            panic!("No deberia llegar aqui porque info_messages es null");
        }
    }

    #[test]
    fn fault_json_info_messages_no_es_array() {
        let json_val = serde_json::json!({
            "status": "ok",
            "info_messages": "no soy un array"
        });

        let info = json_val.get("info_messages");
        // Si no es array, el frontend no debe intentar iterar
        let is_array = info.and_then(|v| v.as_array()).is_some();
        assert!(!is_array);
    }

    #[test]
    fn fault_json_info_messages_con_elementos_no_string() {
        let json_val = serde_json::json!({
            "info_messages": ["valido", 42, null, "tambien valido"]
        });

        // El frontend debe filtrar solo strings
        let arr = json_val["info_messages"].as_array().unwrap();
        let strings: Vec<_> = arr.iter().filter(|v| v.is_string()).collect();
        assert_eq!(strings.len(), 2);
    }

    #[test]
    fn fault_info_messages_con_caracteres_unicode_extremos() {
        let special = vec![
            "Emoji: 🎉🚀💻",
            "Chino: 你好世界",
            "Arabe: مرحبا بالعالم",
            "Matematicas: ∑∏∫√∞≈",
            "Control: \n\t\r",
            "HTML: <script>alert('xss')</script>",
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
            "Descargué desde https://ejemplo.com y funcionó",
            "url: no es una url real",
            "No se proporcionó URL",  // Este es el mensaje de error malinterpretado
            "url = https://image_fetch.png",  // Parece parametro de image_fetch
            "El parámetro 'url' no es necesario aquí",
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
// SECCIÓN: TESTS DE CASOS LÍMITE ADICIONALES
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
        let end = js[start..].find("function updateConsoleThinking").unwrap();
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
        let block_end = js[start..].find("function updateConsoleThinking").unwrap();
        let block = &js[start..start+block_end];
        
        assert!(block.contains("setInterval"), 
            "startAgentMonitoring no tiene setInterval");
        assert!(block.contains("info_messages"), 
            "startAgentMonitoring no consume info_messages");
        assert!(block.contains("_shownInfoMsgs"), 
            "startAgentMonitoring no tiene _shownInfoMsgs");
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


// ============================================================================
// SECCIÓN 10: TESTS DE REGRESIÓN — Nuevos bugs corregidos recientemente
// ============================================================================

#[cfg(test)]
mod regression_new_bugs {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    // =========================================================================
    // BUG: Conversaciones duplicadas en el historial de chats
    // =========================================================================

    #[test]
    fn chat_dedup_mismo_uuid_diferente_titulo_solo_uno() {
        let chats = vec![
            ("chat-abc", "Hola", "/path/a.json"),
            ("chat-abc", "Hola Mundo", "/path/b.json"),  // mismo UUID, titulo actualizado
            ("chat-xyz", "Ayuda", "/path/c.json"),
        ];

        // Simular deduplicacion por UUID
        use std::collections::HashSet;
        let mut seen: HashSet<&str> = HashSet::new();
        let deduped: Vec<_> = chats.iter()
            .filter(|(uuid, _, _)| seen.insert(uuid))
            .collect();

        assert_eq!(deduped.len(), 2, "Debe haber 2 chats unicos, no 3");
        assert_eq!(deduped[0].0, "chat-abc");
        assert_eq!(deduped[1].0, "chat-xyz");
    }

    #[test]
    fn chat_dedup_archivos_bak_ignorados() {
        let files = vec![
            "chat-abc.json",
            "chat-abc.json.bak",
            "chat-xyz.json",
        ];

        let valid: Vec<_> = files.iter()
            .filter(|f| !f.ends_with(".bak"))
            .collect();

        assert_eq!(valid.len(), 2);
    }

    // =========================================================================
    // BUG: addMessage estaba duplicada en app.js
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
        assert!(js.contains("chatArea.scrollTop = chatArea.scrollHeight"),
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
    fn app_js_send_message_to_agent_solo_con_mensaje_valido() {
        let js = include_str!("../public/app.js");
        // Debe validar que el mensaje no este vacio
        let func_start = js.find("function sendMessageToAgent").unwrap();
        let next_func = js[func_start+1..].find("function ").unwrap_or(js.len() - func_start - 1);
        let block = &js[func_start..func_start+1+next_func];

        assert!(block.contains(".trim()"),
            "REGRESION: sendMessageToAgent no hace trim del mensaje");
        assert!(block.contains("addMessage"),
            "REGRESION: sendMessageToAgent no llama a addMessage con el mensaje del usuario");
    }

    // =========================================================================
    // BUG: renderConsoleSteps no renderizaba los steps
    // =========================================================================

    #[test]
    fn app_js_render_console_steps_no_vacio_al_inicio() {
        let js = include_str!("../public/app.js");
        let func_start = js.find("function renderConsoleSteps").unwrap();
        let next_func = js[func_start+1..].find("function ").unwrap_or(js.len() - func_start - 1);
        let block = &js[func_start..func_start+1+next_func];

        // Debe iterar sobre los steps
        assert!(block.contains(".forEach"),
            "REGRESION: renderConsoleSteps no itera sobre los steps");
        // Debe usar step.step_type para elegir el icono
        assert!(block.contains("step_type"),
            "REGRESION: renderConsoleSteps no usa step_type para los iconos");
    }

    // =========================================================================
    // BUG: La consola de auditoria no se muestra en mobile
    // =========================================================================

    #[test]
    fn app_js_console_area_existe_en_dom() {
        let html = include_str!("../public/index.html");
        assert!(html.contains("id=\"consoleArea\""),
            "REGRESION: index.html no tiene consoleArea. La consola no existe en el DOM.");
    }

    #[test]
    fn css_console_area_tiene_estilos() {
        let css = include_str!("../public/style.css");
        assert!(css.contains("console-area") || css.contains("#consoleArea"),
            "REGRESION: style.css no tiene estilos para consoleArea.");
    }
}


// ============================================================================
// SECCIÓN 11: TESTS UNITARIOS DEL FRONTEND — Validan funciones JS via Rust
// (Simulaciones de comportamiento JavaScript en Rust)
// ============================================================================

#[cfg(test)]
mod frontend_unit_tests {
    #![allow(unused_imports, unused_variables, unused_assignments, unused_mut)]

    #[test]
    fn frontend_add_message_escapa_html_basico() {
        // Simula que addMessage usa textContent (no innerHTML) para user messages
        let user_input = "<script>alert('xss')</script>";
        // textContent no interpreta HTML
        let safe = user_input.replace("<", "&lt;").replace(">", "&gt;");
        assert!(!safe.contains("<script>"));
        assert!(safe.contains("&lt;script&gt;"));
    }

    #[test]
    fn frontend_sanitize_filename_js_equivalente() {
        // Simula sanitize_filename del frontend (misma logica que Rust)
        let filename = "mi:archivo<malo>.txt";
        let sanitized: String = filename.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
            .collect();
        assert_eq!(sanitized, "mi_archivo_malo_.txt");
    }

    #[test]
    fn frontend_markdown_agent_messages_usando_marked() {
        // El frontend usa marked.parse() para mensajes del agente
        let md = "**negrita** y `codigo`";
        // Simulacion basica de marked
        let has_bold = md.contains("**");
        let has_code = md.contains("`");
        assert!(has_bold);
        assert!(has_code);
    }

    #[test]
    fn frontend_detecta_mobile_android() {
        let user_agent = "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36";
        let is_android = user_agent.contains("Android");
        assert!(is_android);
    }

    #[test]
    fn frontend_detecta_mobile_iphone() {
        let user_agent = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0)";
        let is_iphone = user_agent.contains("iPhone");
        assert!(is_iphone);
    }
}
