// ============================================================================
// tests/regression_tests.rs — Tests de Regresión para Bugs Críticos
// ============================================================================
//
// BUG-032: Encoding corrupto en agent.rs (16,589 ocurrencias de mojibake)
// BUG-033: Mismatch execute_power_shell entre servidor Rust y cliente Electron
// BUG-read_file: Fallback a lectura local cuando cliente falla
//
// Estos tests verifican que las correcciones permanezcan en su lugar.
// ============================================================================

#[cfg(test)]
mod bug_033_client_protocol_mismatch {
    #![allow(unused_imports)]
    use std::path::Path;

    // =========================================================================
    // BUG-033: El servidor Rust serializa ExecutePowerShell como
    // "execute_power_shell" (snake_case con underscore), pero el cliente
    // Electron esperaba "execute_powershell" (sin underscore).
    // =========================================================================

    #[test]
    fn client_protocol_rs_usa_rename_all_snake_case() {
        // Verificar que el enum ClientAction usa snake_case
        let src = include_str!("../src/client_protocol.rs");
        assert!(src.contains("rename_all = \"snake_case\""),
            "BUG-033 REGRESION: client_protocol.rs NO usa #[serde(rename_all = \"snake_case\")]");
    }

    #[test]
    fn client_protocol_rs_contiene_execute_power_shell() {
        // Verificar que ExecutePowerShell existe en el enum
        let src = include_str!("../src/client_protocol.rs");
        assert!(src.contains("ExecutePowerShell"),
            "BUG-033 REGRESION: ClientAction NO contiene ExecutePowerShell");
    }

    #[test]
    fn electron_main_js_espera_execute_power_shell_con_underscore() {
        // Verificar que el cliente Electron acepta el nombre correcto
        // con underscore entre "power" y "shell" (NO todo junto)
        let src = include_str!("../electron/main.js");
        assert!(src.contains("'execute_power_shell'"),
            "BUG-033 REGRESION: electron/main.js NO contiene 'execute_power_shell' (con underscore). Debe coincidir con la serializacion snake_case de ExecutePowerShell.");
        assert!(!src.contains("'execute_powershell'"),
            "BUG-033 REGRESION: electron/main.js contiene 'execute_powershell' (sin underscore). Debe usar 'execute_power_shell' para coincidir con el servidor Rust.");
    }

    #[test]
    fn electron_main_js_tiene_case_para_todas_las_acciones() {
        // Verificar que el cliente tiene handlers para todas las acciones del protocolo
        let src = include_str!("../electron/main.js");
        let acciones_requeridas = [
            "read_file",
            "write_file",
            "execute_power_shell", // BUG-033: con underscore
            "list_directory",
            "file_exists",
            "file_metadata",
            "git_operation",
            "cargo_operation",
            "search_code",
        ];
        for accion in &acciones_requeridas {
            let pattern = format!("case '{}':", accion);
            assert!(src.contains(&pattern),
                "BUG-033 REGRESION: electron/main.js NO tiene case para '{}'", accion);
        }
    }

    #[test]
    fn nombres_accion_coinciden_rust_y_js() {
        // Verificar consistencia bidireccional: todos los ClientAction de Rust
        // deben tener un case correspondiente en JS
        let rust_src = include_str!("../src/client_protocol.rs");
        let js_src = include_str!("../electron/main.js");

        // Extraer los nombres de las variantes del enum ClientAction
        // Las variantes son: ReadFile, WriteFile, ExecutePowerShell, etc.
        let variantes = ["ReadFile", "WriteFile", "ExecutePowerShell",
                         "ListDirectory", "FileExists", "FileMetadata",
                         "GitOperation", "CargoOperation", "SearchCode"];

        for variante in &variantes {
            assert!(rust_src.contains(variante),
                "BUG-033 REGRESION: ClientAction ya no contiene la variante {}", variante);
        }

        // Convertir a snake_case manualmente (lo que hace serde)
        // ReadFile -> read_file, ExecutePowerShell -> execute_power_shell
        let snake_names = [
            ("ReadFile", "read_file"),
            ("WriteFile", "write_file"),
            ("ExecutePowerShell", "execute_power_shell"),
            ("ListDirectory", "list_directory"),
            ("FileExists", "file_exists"),
            ("FileMetadata", "file_metadata"),
            ("GitOperation", "git_operation"),
            ("CargoOperation", "cargo_operation"),
            ("SearchCode", "search_code"),
        ];

        for (_variant, snake_name) in &snake_names {
            let case_pattern = format!("case '{}':", snake_name);
            assert!(js_src.contains(&case_pattern),
                "BUG-033 REGRESION: electron/main.js no tiene 'case '{}':'. La serializacion snake_case de Rust produce '{}'.",
                snake_name, snake_name);
        }
    }
}

#[cfg(test)]
mod bug_032_encoding_corrupto {
    #![allow(unused_imports)]

    // =========================================================================
    // BUG-032: agent.rs tiene 16,589 ocurrencias de texto corrupto (mojibake)
    // por doble codificación UTF-8. Estos tests verifican que no hay secuencias
    // de mojibake en strings clave que afectan al system prompt del agente.
    // =========================================================================

    #[test]
    fn agent_rs_no_contiene_secuencia_mojibake_a_con_tilde() {
        // La secuencia "Ãƒ" (C3 83 C6 92) es la firma del doble encoding.
        // "ó" en UTF-8 es C3 B3. Mal decodificado como Windows-1252 produce "Ã³".
        // Eso recodificado como UTF-8 produce C3 83 C2 B3 ("ÃƒÂ³").
        let src = include_str!("../src/agent.rs");
        let mojibake_count = src.matches("Ãƒ").count();
        // Si hay más de 100 ocurrencias, el archivo sigue corrupto
        assert!(mojibake_count < 100,
            "BUG-032 REGRESION: agent.rs contiene {} ocurrencias de 'Ãƒ' (firma de doble encoding UTF-8). El archivo sigue corrupto.",
            mojibake_count);
    }

    #[test]
    fn agent_rs_contiene_texto_espanol_correcto_en_system_prompt() {
        // Verificar que el system prompt inyectado contiene texto legible
        let src = include_str!("../src/agent.rs");
        // Estas frases deben existir en español correcto
        let frases_requeridas = [
            "OBLIGACIÓN",
            "DOCUMENTACIÓN",
            "INMEDIATAMENTE",
            "técnico",
            "sesión",
            "acción",
        ];
        for frase in &frases_requeridas {
            assert!(src.contains(frase),
                "BUG-032 REGRESION: agent.rs NO contiene '{}' (posiblemente corrupto). Texto esperado en el system prompt.", frase);
        }
    }

    #[test]
    fn agent_rs_descripcion_herramientas_sin_mojibake() {
        // Verificar que las descripciones de herramientas están en español correcto
        let src = include_str!("../src/agent.rs");
        // Patrones que indican corrupción
        let patrones_corruptos = [
            "ÃƒÆ'Ã†â€™",  // Triple encoding signature
            "Ã¢â€šÂ¬",     // Double encoding signature  
            "Ã‚Â¢",       // Common mojibake
        ];
        for patron in &patrones_corruptos {
            let count = src.matches(patron).count();
            assert!(count < 50,
                "BUG-032 REGRESION: agent.rs contiene {} ocurrencias de '{}' (mojibake). Las descripciones de herramientas pueden estar corruptas.",
                count, patron);
        }
    }

    #[test]
    fn prompts_default_system_prompt_txt_utf8_limpio() {
        // El archivo prompts/default_system_prompt.txt debe estar en UTF-8 correcto
        let src = include_str!("../prompts/default_system_prompt.txt");
        assert!(src.contains("autónomo"),
            "BUG-032 REGRESION: prompts/default_system_prompt.txt NO contiene 'autónomo' (UTF-8 corrupto)");
        assert!(src.contains("código"),
            "BUG-032 REGRESION: prompts/default_system_prompt.txt NO contiene 'código' (UTF-8 corrupto)");
        assert!(!src.contains("Ã³"),
            "BUG-032 REGRESION: prompts/default_system_prompt.txt contiene 'Ã³' (mojibake)");
    }

    #[test]
    fn global_prompt_json_sin_mojibake() {
        // Verificar que el globalPrompt.json del admin no tiene mojibake
        let path = std::path::Path::new("../.config/data/admin/globalPrompt.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                let mojibake = content.matches("Ãƒ").count();
                assert!(mojibake < 100,
                    "BUG-032 REGRESION: globalPrompt.json contiene {} ocurrencias de 'Ãƒ' (mojibake). El system prompt está corrupto.", mojibake);
            }
        }
    }

    #[test]
    fn agent_rs_tamano_razonable() {
        // agent.rs no debería exceder ~300KB. Actualmente pesa 569KB por la corrupción.
        let src = include_str!("../src/agent.rs");
        let size_kb = src.len() / 1024;
        // Este test fallará hasta que se repare el encoding. Es un recordatorio.
        assert!(size_kb < 350,
            "BUG-032 REGRESION: agent.rs pesa {} KB (debería ser < 350 KB). El encoding corrupto infla el archivo. 16,589 ocurrencias de mojibake ocupan ~300KB extra.", size_kb);
    }
}

#[cfg(test)]
mod bug_read_file_archivos_preexistentes {
    #![allow(unused_imports)]
    use std::path::Path;

    // =========================================================================
    // BUG-read_file: El agente solo lee archivos que él creó, no los
    // preexistentes. Causas:
    //   1. BUG-033: execute_powershell fallaba (ya corregido)
    //   2. Fallback local después de error de cliente
    //   3. Resolución de rutas con caracteres especiales
    // =========================================================================

    #[test]
    fn agent_rs_tiene_try_delegate_to_client() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("fn try_delegate_to_client"),
            "REGRESION: agent.rs NO contiene fn try_delegate_to_client");
        assert!(src.contains("return None;"),
            "REGRESION: try_delegate_to_client no tiene return None para fallback local");
    }

    #[test]
    fn agent_rs_read_file_tiene_fallback_local() {
        // Verificar que el handler de read_file intenta fallback local
        // después de intentar delegar al cliente
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("if let Some(delegated) = try_delegate_to_client"),
            "REGRESION: read_file no intenta delegar al cliente");
        // Debe tener un else o ejecución local después de la delegación
        assert!(src.contains("let rel_path = args[\"path\"].as_str()"),
            "REGRESION: read_file no extrae rel_path para fallback local");
        assert!(src.contains("get_project_path"),
            "REGRESION: read_file no usa get_project_path para resolver rutas");
    }

    #[test]
    fn agent_rs_tiene_mensaje_error_archivo_no_encontrado() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("Error leyendo archivo"),
            "REGRESION: read_file no tiene mensaje de error 'Error leyendo archivo'");
    }

    #[test]
    fn path_join_con_ruta_absoluta_funciona() {
        // Verificar que Path::join con ruta absoluta devuelve la ruta absoluta
        // (comportamiento estándar de Rust)
        let base = Path::new("C:\\Users\\Fa\\Desktop\\Roblox");
        let absoluta = Path::new("C:\\Users\\Fa\\Desktop\\Roblox\\Documentacion\\README.md");
        let result = base.join(absoluta);
        assert_eq!(result, absoluta,
            "REGRESION: Path::join con ruta absoluta debería devolver la ruta absoluta");
    }

    #[test]
    fn path_join_con_ruta_relativa_funciona() {
        let base = Path::new("C:\\Users\\Fa\\Desktop\\Roblox");
        let relativa = "Documentacion\\README.md";
        let result = base.join(relativa);
        assert_eq!(result, Path::new("C:\\Users\\Fa\\Desktop\\Roblox\\Documentacion\\README.md"),
            "REGRESION: Path::join con ruta relativa no funciona correctamente");
    }

    #[test]
    fn path_join_con_forward_slashes_funciona() {
        // El agente a veces usa forward slashes en lugar de backslashes
        let base = Path::new("C:/Users/Fa/Desktop/Roblox");
        let relativa = "Documentacion/README.md";
        let result = base.join(relativa);
        assert!(result.to_string_lossy().contains("Documentacion"),
            "REGRESION: Path::join con forward slashes no funciona. Resultado: {}",
            result.to_string_lossy());
    }

    #[test]
    fn path_join_con_caracteres_especiales() {
        // Verificar que rutas con ñ y espacios funcionan
        let base = Path::new("C:\\Users\\Fa\\Desktop\\Roblox");
        let relativa = "Documentacion\\Rayfield general\\índice.md";
        let result = base.join(relativa);
        assert!(result.to_string_lossy().contains("Rayfield general"),
            "REGRESION: Path::join con caracteres especiales (ñ, espacios) no funciona");
        assert!(result.to_string_lossy().contains("índice"),
            "REGRESION: Path::join con tildes no funciona");
    }

    #[test]
    fn get_project_path_existe_en_agent_rs() {
        let src = include_str!("../src/agent.rs");
        assert!(src.contains("fn get_project_path"),
            "REGRESION: agent.rs NO contiene fn get_project_path");
        assert!(src.contains("state.projects.lock()"),
            "REGRESION: get_project_path no accede a state.projects");
    }
}

#[cfg(test)]
mod bug_regresion_general {
    #![allow(unused_imports)]

    // =========================================================================
    // Tests de regresión general: verifican que correcciones anteriores
    // no se deshacen accidentalmente.
    // =========================================================================

    #[test]
    fn agent_rs_no_contiene_timeout_30s() {
        // BUG-031: El timeout de delegación debe ser 10s, no 30s
        let src = include_str!("../src/agent.rs");
        assert!(!src.contains("from_secs(30)"),
            "BUG-031 REGRESION: agent.rs contiene timeout de 30s. Debe ser 10s.");
        assert!(src.contains("from_secs(10)"),
            "BUG-031 REGRESION: agent.rs no contiene timeout de 10s.");
    }

    #[test]
    fn tools_no_estan_en_messages() {
        // BUG-030: Las tool definitions deben estar en el array 'tools', no en 'messages'
        let src = include_str!("../src/agent.rs");
        // Verificar que hay un array 'tools' separado
        assert!(src.contains("let tools = vec!"),
            "BUG-030 REGRESION: agent.rs no tiene 'let tools = vec!'. Las herramientas deben estar en array separado.");
    }

    #[test]
    fn clean_old_chat_files_existe() {
        // Verificar que la función de limpieza de chats duplicados existe
        let src = include_str!("../src/main.rs");
        assert!(src.contains("fn clean_old_chat_files"),
            "REGRESION: main.rs no contiene fn clean_old_chat_files");
    }

    #[test]
    fn get_chats_usa_hashset_dedup() {
        let src = include_str!("../src/main.rs");
        assert!(src.contains("HashSet") && src.contains("get_chats"),
            "REGRESION: get_chats no usa HashSet para deduplicación");
    }

    #[test]
    fn study_system_prompt_const_existe() {
        let src = include_str!("../src/lib.rs");
        assert!(src.contains("STUDY_SYSTEM_PROMPT"),
            "BUG-026 REGRESION: lib.rs no exporta STUDY_SYSTEM_PROMPT");
    }

    #[test]
    fn file_editor_tiene_3_modos() {
        let src = include_str!("../src/file_editor.rs");
        assert!(src.contains("Adicion"),
            "REGRESION: file_editor.rs no contiene modo Adicion");
        assert!(src.contains("Reemplazo"),
            "REGRESION: file_editor.rs no contiene modo Reemplazo");
        assert!(src.contains("Eliminacion"),
            "REGRESION: file_editor.rs no contiene modo Eliminacion");
    }

    #[test]
    fn google_drive_client_derives_clone() {
        let src = include_str!("../src/google_drive.rs");
        assert!(src.contains("struct GoogleDriveClient") && src.contains("Clone"),
            "REGRESION: GoogleDriveClient no deriva Clone");
    }

    #[test]
    fn validator_existe_y_contiene_funciones_clave() {
        let src = include_str!("../src/validator.rs");
        assert!(src.contains("fn validate_file_after_write"),
            "REGRESION: validator.rs no contiene validate_file_after_write");
        assert!(src.contains("duplicad"),
            "REGRESION: validator.rs no detecta líneas duplicadas");
    }
}
