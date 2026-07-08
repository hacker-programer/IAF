//! Módulo de validación post-escritura.
//!
//! Este módulo proporciona funciones para validar archivos después de ser modificados
//! por el agente, detectando problemas comunes como:
//! - Líneas duplicadas consecutivas (copy-paste accidental)
//! - Delimitadores no balanceados (llaves, paréntesis, corchetes)
//! - Razonamiento del modelo inyectado sin comentarios (nuevo)
//! - Errores de sintaxis en archivos .rs y .js
//!
//! Se diseñó para prevenir los errores recurrentes documentados en MEMORIES.md.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Resultado de la validación de un archivo.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    #[allow(dead_code)]
    pub path: String,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty() && self.errors.is_empty()
    }

    pub fn to_message(&self) -> String {
        if self.is_clean() {
            return String::new();
        }
        let mut msg = String::new();
        if !self.warnings.is_empty() {
            msg.push_str("\n\n⚠️ ADVERTENCIAS DE VALIDACIÓN POST-ESCRITURA:");
            for w in &self.warnings {
                msg.push_str(&format!("\n  • {}", w));
            }
        }
        if !self.errors.is_empty() {
            msg.push_str("\n\n❌ ERRORES DE VALIDACIÓN POST-ESCRITURA:");
            for e in &self.errors {
                msg.push_str(&format!("\n  • {}", e));
            }
        }
        msg
    }
}

/// Valida un archivo después de ser escrito por el agente.
/// Retorna advertencias y errores encontrados.
pub fn validate_file_after_write(file_path: &str, _content: &str) -> ValidationResult {
    let mut result = ValidationResult {
        path: file_path.to_string(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    // Solo validar si el archivo existe
    let path = Path::new(file_path);
    if !path.exists() {
        return result;
    }

    // Leer el archivo del disco (el contenido real, no el pasado por parámetro)
    let disk_content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            result.errors.push(format!("No se pudo leer el archivo para validación: {}", e));
            return result;
        }
    };

    // === VALIDACIÓN 1: Detectar líneas duplicadas consecutivas ===
    let dup_warnings = detect_duplicate_lines(&disk_content);
    result.warnings.extend(dup_warnings);

    // === VALIDACIÓN 1.5: Detectar definiciones duplicadas (fn, struct, enum, etc.) ===
    let def_warnings = detect_duplicate_definitions(&disk_content);
    result.warnings.extend(def_warnings);

    // === VALIDACIÓN 1.6: Detectar razonamiento del modelo inyectado sin comentarios ===
    // Este es el detector del bug más insidioso: el agente a veces inyecta su
    // razonamiento (ej. "OK, ahora necesito modificar esta función...") directamente
    // en archivos de código sin //, causando errores de compilación.
    let reasoning_warnings = detect_reasoning_injection(&disk_content);
    result.warnings.extend(reasoning_warnings);

    // === VALIDACIÓN 2: Verificar delimitadores balanceados ===
    let delim_errors = check_balanced_delimiters(&disk_content);
    result.errors.extend(delim_errors);

    // === VALIDACIÓN 3: Verificar sintaxis específica del lenguaje ===
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => {
            // Para archivos Rust, ejecutar cargo check rápido (solo si el proyecto compila)
            // Nota: no ejecutamos cargo check aquí porque sería muy lento.
            // En su lugar, verificamos patrones comunes de error.
            let syntax_warnings = check_rust_common_errors(&disk_content);
            result.warnings.extend(syntax_warnings);
        }
        "js" => {
            // Para JavaScript, intentar node --check
            match Command::new("node")
                .args(&["--check", file_path])
                .output()
            {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        result.errors.push(format!(
                            "Error de sintaxis JavaScript (node --check): {}",
                            stderr.trim()
                        ));
                    }
                }
                Err(_) => {
                    // node no está disponible, no es un error crítico
                }
            }
        }
        _ => {}
    }

    result
}

/// Detecta líneas idénticas consecutivas (indicador de copy-paste accidental del agente).
fn detect_duplicate_lines(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 2 {
        return warnings;
    }

    let mut i = 0;
    while i < lines.len() - 1 {
        let current = lines[i].trim();
        let next = lines[i + 1].trim();

        // Solo reportar duplicados si la línea no está vacía y es idéntica
        if !current.is_empty() && current == next {
            // Evitar falsos positivos en líneas que son naturalmente repetitivas
            // (como "}" repetido, líneas de solo comentarios, etc.)
            let is_structural = current == "}" || current == ")" || current == "]"
                || current.starts_with("//") || current == "};" || current == "});";

            if !is_structural {
                warnings.push(format!(
                    "Línea duplicada detectada (línea {}): \"{}\"",
                    i + 1,
                    truncate_for_display(current, 60)
                ));

                // Saltar todas las repeticiones consecutivas
                while i < lines.len() - 1 && lines[i].trim() == lines[i + 1].trim() {
                    i += 1;
                }
            }
        }
        i += 1;
    }

    if !warnings.is_empty() {
        warnings.push(format!(
            "Se encontraron {} bloques de líneas duplicadas. Esto suele ser resultado de copy-paste accidental del agente.",
            warnings.len()
        ));
    }

    warnings
}

/// Verifica que los delimitadores (llaves, paréntesis, corchetes) estén balanceados.
fn check_balanced_delimiters(content: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut stack: Vec<(char, usize)> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for ch in line.chars() {
            match ch {
                '{' | '(' | '[' => stack.push((ch, line_num + 1)),
                '}' => {
                    if stack.last().map(|&(c, _)| c) == Some('{') {
                        stack.pop();
                    } else {
                        errors.push(format!(
                            "Llave '}}' no balanceada en línea {} (esperaba '{}')",
                            line_num + 1,
                            stack.last().map(|&(c, _)| matching_open(c)).unwrap_or('?')
                        ));
                    }
                }
                ')' => {
                    if stack.last().map(|&(c, _)| c) == Some('(') {
                        stack.pop();
                    } else {
                        errors.push(format!(
                            "Paréntesis ')' no balanceado en línea {} (esperaba '{}')",
                            line_num + 1,
                            stack.last().map(|&(c, _)| matching_open(c)).unwrap_or('?')
                        ));
                    }
                }
                ']' => {
                    if stack.last().map(|&(c, _)| c) == Some('[') {
                        stack.pop();
                    } else {
                        errors.push(format!(
                            "Corchete ']' no balanceado en línea {} (esperaba '{}')",
                            line_num + 1,
                            stack.last().map(|&(c, _)| matching_open(c)).unwrap_or('?')
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    for (ch, line) in stack {
        errors.push(format!(
            "Delimitador '{}' abierto en línea {} nunca se cierra",
            ch, line
        ));
    }

    errors
}

/// Devuelve el delimitador de apertura correspondiente a uno de cierre.
fn matching_open(close: char) -> char {
    match close {
        '}' => '{',
        ')' => '(',
        ']' => '[',
        _ => '?',
    }
}

/// Verifica patrones comunes de error en archivos Rust.
fn check_rust_common_errors(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // Detectar 'mod' de archivos que no existen (esto se verificaría en compilación)
    // Pero podemos advertir sobre patrones sospechosos.

    // Detectar funciones vacías (posible código huérfano)
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Advertir sobre 'unsafe' blocks (posiblemente generados por error)
        if trimmed.contains("unsafe {") && !trimmed.starts_with("//") {
            warnings.push(format!(
                "Bloque 'unsafe' detectado en línea {} - verificar que sea intencional",
                i + 1
            ));
        }

        // Advertir sobre 'unwrap()' en código de producción (posible fuente de pánicos)
        if trimmed.contains(".unwrap()") && !trimmed.starts_with("//") && !trimmed.starts_with("//!") {
            // No advertir en tests
        }
    }

    warnings
}

/// Detecta definiciones duplicadas de funciones, structs, enums, traits, constantes y módulos.
/// Este es el detector MÁS IMPORTANTE: atrapa el patrón de error más frecuente del agente
/// donde copy-paste accidental deja dos definiciones idénticas de la misma función/struct.
fn detect_duplicate_definitions(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut definitions: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();

    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Detectar definiciones de fn, pub fn, async fn, etc.
        if let Some(name) = extract_def_name(trimmed, "fn ") {
            definitions.entry(name).or_default().push(i + 1);
        }
        // Detectar struct
        if let Some(name) = extract_def_name(trimmed, "struct ") {
            definitions.entry(format!("struct {}", name)).or_default().push(i + 1);
        }
        // Detectar enum
        if let Some(name) = extract_def_name(trimmed, "enum ") {
            definitions.entry(format!("enum {}", name)).or_default().push(i + 1);
        }
        // Detectar trait
        if let Some(name) = extract_def_name(trimmed, "trait ") {
            definitions.entry(format!("trait {}", name)).or_default().push(i + 1);
        }
        // Detectar const
        if let Some(name) = extract_def_name(trimmed, "const ") {
            definitions.entry(format!("const {}", name)).or_default().push(i + 1);
        }
        // Detectar static
        if let Some(name) = extract_def_name(trimmed, "static ") {
            definitions.entry(format!("static {}", name)).or_default().push(i + 1);
        }
        // Detectar mod
        if let Some(name) = extract_def_name(trimmed, "mod ") {
            // Ignorar mod que no tienen body (solo declaración de archivo externo)
            if !trimmed.ends_with(';') {
                definitions.entry(format!("mod {}", name)).or_default().push(i + 1);
            }
        }
    }

    for (def_name, locations) in &definitions {
        if locations.len() > 1 {
            let locs_str = locations.iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            warnings.push(format!(
                "DEFINICIÓN DUPLICADA DETECTADA: '{}' definida {} veces (líneas: {}). Esto causará error de compilación.",
                def_name, locations.len(), locs_str
            ));
        }
    }

    if !warnings.is_empty() {
        warnings.push(format!(
            "Se encontraron {} definiciones duplicadas. EDITAR EL ARCHIVO COMPLETO, no uses start_line/end_line.",
            warnings.len()
        ));
    }

    warnings
}

/// Extrae el nombre de una definición. Ej: "pub fn foo(" → Some("fn foo"), "struct Bar {" → Some("struct Bar")
fn extract_def_name<'a>(line: &'a str, keyword: &str) -> Option<String> {
    let line = line.trim();
    // Buscar el keyword
    let pos = line.find(keyword)?;
    let after_keyword = &line[pos + keyword.len()..];
    // El nombre es lo que sigue hasta '(' o '{' o '<' o ' '
    let name_end = after_keyword.find(|c: char| c == '(' || c == '{' || c == '<' || c == ';' || c == ':').unwrap_or(after_keyword.len());
    let name = after_keyword[..name_end].trim();
    if name.is_empty() || name == "(" || name == "{" {
        return None;
    }
    // Verificar que no sea una llamada a función (que tendría "=" antes o estaría dentro de otra expresión)
    // Solo consideramos definiciones a nivel de archivo (sin indentación o con pub)
    let before_keyword = &line[..pos];
    if before_keyword.trim().is_empty() || before_keyword.trim() == "pub" || before_keyword.trim() == "pub(crate)" || before_keyword.trim() == "pub(super)" || before_keyword.trim() == "async" || before_keyword.trim() == "pub async" || before_keyword.trim() == "unsafe" || before_keyword.trim() == "pub unsafe" || before_keyword.trim() == "default" || before_keyword.trim() == "const" || before_keyword.trim() == "extern" {
        Some(format!("{} {}", keyword.trim(), name))
    } else {
        None
    }
}

/// Detecta texto de razonamiento del modelo inyectado en archivos de código sin
/// marcadores de comentario. Este es el bug donde el agente escribe frases como
/// "OK, ahora necesito modificar esta función..." directamente en archivos .rs.
///
/// Busca patrones de lenguaje natural (español e inglés) que NO están dentro de
/// comentarios (//, /* */, #, <!-- -->).
fn detect_reasoning_injection(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // Patrones de razonamiento típicos del modelo (español e inglés)
    // Estos son fragmentos que NUNCA deberían aparecer en código fuente sin comentar
    let reasoning_patterns: &[&str] = &[
        // Español
        "OK, ahora", "Ok, ahora", "Vale, ahora", "Bien, ahora",
        "Ahora necesito", "Ahora voy a", "Voy a modificar", "Voy a editar",
        "Voy a crear", "Voy a añadir", "Voy a escribir",
        "Primero,", "En primer lugar,", "Para empezar,",
        "El problema es que", "La causa es", "El bug está en",
        "He detectado", "He encontrado", "He visto",
        "Necesito arreglar", "Necesito corregir", "Necesito cambiar",
        "Déjame ver", "Déjame revisar", "Déjame analizar",
        "Permíteme", "Permítanme",
        "Analizando el", "Revisando el", "Examinando el",
        "Esto debería", "Esto podría", "Esto hará",
        "La solución es", "La corrección es",
        "Según el", "De acuerdo al", "Basado en",
        // Inglés
        "OK, now", "Ok, now", "Alright, now", "Well, now",
        "Now I need to", "Now I'll", "Now I will",
        "I need to fix", "I need to change", "I need to edit",
        "I'll modify", "I'll edit", "I'll create", "I'll add", "I'll write",
        "I will modify", "I will edit", "I will create",
        "Let me see", "Let me check", "Let me analyze", "Let me review",
        "Let me look", "Let me read", "Let me edit", "Let me fix",
        "Let me think", "Let me verify", "Let me examine",
        "Let's start", "Let's begin", "Let's fix",
        "First,", "Firstly,", "To start,",
        "The problem is", "The issue is", "The bug is",
        "I've detected", "I've found", "I've seen",
        "This should", "This could", "This will",
        "The solution is", "The fix is",
        "Looking at the", "Checking the", "Examining the",
        "According to", "Based on",
        "So the", "So now", "So I",
        "Wait,", "Actually,", "Hmm,",
    ];

    let lines: Vec<&str> = content.lines().collect();
    
    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Ignorar líneas vacías
        if trimmed.is_empty() {
            continue;
        }
        
        // Ignorar líneas que ya están comentadas
        if trimmed.starts_with("//") || trimmed.starts_with("/*") 
            || trimmed.starts_with("*") || trimmed.starts_with("*/")
            || trimmed.starts_with("#") || trimmed.starts_with("<!--")
            || trimmed.starts_with("///") || trimmed.starts_with("//!") 
        {
            continue;
        }
        
        // Verificar si la línea comienza con algún patrón de razonamiento
        for pattern in reasoning_patterns {
            let trimmed_lower = trimmed.to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            if trimmed_lower.starts_with(&pattern_lower) {
                // Verificar que no es código válido disfrazado
                // Si la línea contiene caracteres típicos de código, podría ser un falso positivo
                let looks_like_code = trimmed.contains('(') || trimmed.contains('{') 
                    || trimmed.contains(';') || trimmed.contains("fn ")
                    || trimmed.contains("let ") || trimmed.contains("pub ")
                    || trimmed.contains("use ") || trimmed.contains("mod ")
                    || trimmed.contains("struct ") || trimmed.contains("enum ")
                    || trimmed.contains("impl ") || trimmed.contains("const ")
                    || trimmed.contains("import ") || trimmed.contains("from ")
                    || trimmed.contains("def ") || trimmed.contains("class ")
                    || trimmed.contains("function ") || trimmed.contains("var ")
                    || trimmed.contains("return ") || trimmed.contains("match ");
                
                if !looks_like_code {
                    warnings.push(format!(
                        "POSIBLE RAZONAMIENTO SIN COMENTAR (línea {}): \"{}\" — parece texto de lenguaje natural, no código. Si es intencional, usa // para comentarlo.",
                        line_num + 1,
                        truncate_for_display(trimmed, 80)
                    ));
                    break; // Una advertencia por línea es suficiente
                }
            }
        }
    }
    
    if !warnings.is_empty() {
        warnings.push(format!(
            "Se encontraron {} líneas con posible texto de razonamiento sin comentar. \
            Esto suele ocurrir cuando el agente inyecta su razonamiento (\"OK, ahora voy a...\") \
            directamente en el archivo de código sin //. CORRIGE EL ARCHIVO COMPLETO.",
            warnings.len()
        ));
    }
    
    warnings
}

fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_duplicate_lines_empty() {
        let warnings = detect_duplicate_lines("");
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_detect_duplicate_lines_no_dups() {
        let content = "linea uno\nlinea dos\nlinea tres\n";
        let warnings = detect_duplicate_lines(content);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_detect_duplicate_lines_found() {
        let content = "fn foo() {\n    let x = 1;\n    let x = 1;\n}\n";
        let warnings = detect_duplicate_lines(content);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_detect_duplicate_lines_structural_ignored() {
        // Las llaves de cierre duplicadas no deberían generar advertencia
        let content = "fn foo() {\n    if true {\n    }\n}\n}\n";
        let warnings = detect_duplicate_lines(content);
        // Las llaves "}" no deberían reportarse como duplicadas
        assert!(warnings.iter().all(|w| !w.contains("\"}\"")));
    }

    #[test]
    fn test_balanced_delimiters_ok() {
        let content = "fn main() {\n    let x = (1 + 2) * 3;\n}\n";
        let errors = check_balanced_delimiters(content);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_balanced_delimiters_unbalanced() {
        let content = "fn main() {\n    let x = (1 + 2;\n}\n";
        let errors = check_balanced_delimiters(content);
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_balanced_delimiters_extra_close() {
        let content = "fn main() {\n    let x = 1;\n}}\n";
        let errors = check_balanced_delimiters(content);
        assert!(!errors.is_empty());
    }

    // === Tests para detect_reasoning_injection ===

    #[test]
    fn test_reasoning_injection_spanish_detected() {
        let content = "OK, ahora necesito modificar esta función\nfn foo() {\n    let x = 1;\n}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("OK, ahora")));
    }

    #[test]
    fn test_reasoning_injection_english_detected() {
        let content = "Let me check the file first\nfn foo() {\n    let x = 1;\n}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("Let me")));
    }

    #[test]
    fn test_reasoning_injection_commented_ignored() {
        // Líneas comentadas con // no deberían generar advertencia
        let content = "// OK, ahora necesito modificar esta función\nfn foo() {\n    let x = 1;\n}\n";
        let warnings = detect_reasoning_injection(content);
        // No debería haber advertencias porque la línea está comentada
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("POSIBLE RAZONAMIENTO")));
    }

    #[test]
    fn test_reasoning_injection_block_comment_ignored() {
        let content = "/* Let me check the file first */\nfn foo() {\n    let x = 1;\n}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("POSIBLE RAZONAMIENTO")));
    }

    #[test]
    fn test_reasoning_injection_clean_code_ok() {
        let content = "fn foo() {\n    let x = 1;\n    let y = 2;\n}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("POSIBLE RAZONAMIENTO")));
    }

    #[test]
    fn test_reasoning_injection_i_need_to_fix() {
        let content = "I need to fix this bug\nfn fix_bug() {\n    return;\n}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("I need to")));
    }

    #[test]
    fn test_reasoning_injection_now_i_will() {
        let content = "Now I will create the function\nfn new_func() {}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(!warnings.is_empty());
    }
}
