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
    let reasoning_warnings = detect_reasoning_injection(&disk_content);
    result.warnings.extend(reasoning_warnings);

    // === VALIDACIÓN 2: Verificar delimitadores balanceados ===
    let delim_errors = check_balanced_delimiters(&disk_content);
    result.errors.extend(delim_errors);

    // === VALIDACIÓN 3: Verificar sintaxis específica del lenguaje ===
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => {
            let syntax_warnings = check_rust_common_errors(&disk_content);
            result.warnings.extend(syntax_warnings);
        }
        "js" => {
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
                Err(_) => {}
            }
        }
        _ => {}
    }

    result
}

/// Detecta líneas idénticas consecutivas (indicador de copy-paste accidental del agente).
///
/// **CORRECCIÓN DE FALSOS POSITIVOS (2026-07-08):**
/// Ahora trackea la profundidad de paréntesis para detectar argumentos de macros
/// (como `format!(...)`) y no reporta líneas duplicadas dentro de ellas.
/// Por ejemplo, `call_id,` repetido 3 veces dentro de `format!("...", call_id, call_id, call_id)`
/// es perfectamente normal y no debe generar advertencia.
fn detect_duplicate_lines(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 2 {
        return warnings;
    }

    // Pre-calcular la profundidad de paréntesis al inicio de cada línea.
    // Esto nos permite saber si una línea está dentro de una invocación de macro.
    let mut paren_depth: Vec<i32> = Vec::with_capacity(lines.len());
    let mut depth: i32 = 0;
    for line in &lines {
        paren_depth.push(depth);
        for ch in line.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
    }

    let mut i = 0;
    while i < lines.len() - 1 {
        let current = lines[i].trim();
        let next = lines[i + 1].trim();

        // Solo reportar duplicados si la línea no está vacía y es idéntica
        if !current.is_empty() && current == next {
            // Evitar falsos positivos en líneas que son naturalmente repetitivas
            let is_structural = current == "}" || current == ")" || current == "]"
                || current.starts_with("//") || current == "};" || current == "});";

            // NUEVO: Evitar falsos positivos dentro de invocaciones de macro.
            // Si la profundidad de paréntesis > 0 en esta línea, estamos dentro de
            // algo como format!(...), vec![...], etc., donde los argumentos repetidos
            // son normales.
            let inside_macro = paren_depth[i] > 0;

            // Si es un identificador simple seguido de coma (ej: "call_id,"),
            // es muy probable que sea un argumento de macro, no código duplicado real.
            let looks_like_macro_arg = current.ends_with(',')
                && !current.contains(' ')
                && current.len() < 40;

            if !is_structural && !inside_macro && !looks_like_macro_arg {
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
    }

    warnings
}

/// Detecta definiciones duplicadas de funciones, structs, enums, traits, constantes
/// y módulos. SOLO reporta duplicados si dos definiciones con el mismo nombre están
/// en el MISMO ámbito (mismo bloque `impl` o ámbito de archivo).
///
/// **CORRECCIÓN DE FALSOS POSITIVOS (2026-07-08):**
/// Ahora trackea en qué bloque `impl` o `trait` estamos, de modo que `fn new()` en
/// `impl ToolResultStore` no se confunda con `fn new()` en `impl SubAgentManager`.
/// También ignora definiciones dentro de módulos `#[cfg(test)]` si están en impl
/// blocks de test.
fn detect_duplicate_definitions(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // Para cada nombre de definición, guardamos (línea, ámbito).
    // El ámbito es el nombre del impl/trait block, o "" para ámbito de archivo.
    let mut definitions: std::collections::HashMap<String, Vec<(usize, String)>> =
        std::collections::HashMap::new();

    let lines: Vec<&str> = content.lines().collect();

    // Trackear en qué impl/trait block estamos.
    // Ej: "impl ToolResultStore" → current_scope = "ToolResultStore"
    let mut current_scope: String = String::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detectar entrada en un bloque impl
        if let Some(rest) = trimmed.strip_prefix("impl ") {
            // Extraer el nombre del tipo (antes de '{', 'for', o 'where')
            let type_name = rest
                .split(|c: char| c == '{' || c == ' ' || c == '<' || c == '\t')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !type_name.is_empty() && type_name != "for" {
                current_scope = type_name;
            }
        }
        // Detectar entrada en un bloque trait
        if let Some(rest) = trimmed.strip_prefix("trait ") {
            let trait_name = rest
                .split(|c: char| c == '{' || c == ' ' || c == '<')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !trait_name.is_empty() {
                current_scope = format!("trait {}", trait_name);
            }
        }

        // Detectar cierre de bloque (línea que es solo "}" o empieza con "}")
        // Cuando encontramos un cierre, volvemos al scope anterior.
        // Simplificación: si la línea es solo "}", asumimos que cierra el bloque actual.
        if trimmed == "}" {
            current_scope = String::new();
        }

        // Detectar definiciones de fn
        if let Some(name) = extract_def_name_with_scope(trimmed, "fn ", &current_scope) {
            definitions
                .entry(name.clone())
                .or_default()
                .push((i + 1, current_scope.clone()));
        }
        // Detectar struct
        if let Some(name) = extract_def_name_with_scope(trimmed, "struct ", &current_scope) {
            definitions
                .entry(name.clone())
                .or_default()
                .push((i + 1, current_scope.clone()));
        }
        // Detectar enum
        if let Some(name) = extract_def_name_with_scope(trimmed, "enum ", &current_scope) {
            definitions
                .entry(name.clone())
                .or_default()
                .push((i + 1, current_scope.clone()));
        }
        // Detectar trait
        if let Some(name) = extract_def_name_with_scope(trimmed, "trait ", &current_scope) {
            definitions
                .entry(name.clone())
                .or_default()
                .push((i + 1, current_scope.clone()));
        }
        // Detectar const
        if let Some(name) = extract_def_name_with_scope(trimmed, "const ", &current_scope) {
            definitions
                .entry(name.clone())
                .or_default()
                .push((i + 1, current_scope.clone()));
        }
        // Detectar static
        if let Some(name) = extract_def_name_with_scope(trimmed, "static ", &current_scope) {
            definitions
                .entry(name.clone())
                .or_default()
                .push((i + 1, current_scope.clone()));
        }
        // Detectar mod (solo los que tienen body, no declaraciones externas)
        if let Some(name) = extract_def_name_with_scope(trimmed, "mod ", &current_scope) {
            if !trimmed.ends_with(';') {
                definitions
                    .entry(name.clone())
                    .or_default()
                    .push((i + 1, current_scope.clone()));
            }
        }
    }

    // Agrupar definiciones duplicadas por (nombre, ámbito)
    let mut dup_by_scope: std::collections::HashMap<(String, String), Vec<usize>> =
        std::collections::HashMap::new();

    for (def_name, occurrences) in &definitions {
        if occurrences.len() <= 1 {
            continue;
        }
        // Agrupar por ámbito
        for (line, scope) in occurrences {
            dup_by_scope
                .entry((def_name.clone(), scope.clone()))
                .or_default()
                .push(*line);
        }
    }

    for ((def_name, scope), locations) in &dup_by_scope {
        if locations.len() > 1 {
            let locs_str = locations
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let scope_info = if scope.is_empty() {
                "ámbito de archivo".to_string()
            } else {
                format!("impl/trait '{}'", scope)
            };
            warnings.push(format!(
                "DEFINICIÓN DUPLICADA DETECTADA: '{}' definida {} veces en {} (líneas: {}). Esto causará error de compilación.",
                def_name,
                locations.len(),
                scope_info,
                locs_str
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

/// Extrae el nombre de una definición e incluye el ámbito en el nombre.
/// Ej: "pub fn new()" en impl "ToolResultStore" → "fn new (ToolResultStore)"
fn extract_def_name_with_scope<'a>(
    line: &'a str,
    keyword: &str,
    scope: &str,
) -> Option<String> {
    let line = line.trim();

    // Buscar el keyword
    let pos = line.find(keyword)?;
    let after_keyword = &line[pos + keyword.len()..];

    // El nombre es lo que sigue hasta '(' o '{' o '<' o ' ' o ';' o ':'
    let name_end = after_keyword
        .find(|c: char| c == '(' || c == '{' || c == '<' || c == ';' || c == ':')
        .unwrap_or(after_keyword.len());
    let name = after_keyword[..name_end].trim();

    if name.is_empty() || name == "(" || name == "{" {
        return None;
    }

    // Solo consideramos definiciones a nivel de archivo (sin indentación o con
    // modificadores de visibilidad)
    let before_keyword = &line[..pos];
    let is_definition = before_keyword.trim().is_empty()
        || before_keyword.trim() == "pub"
        || before_keyword.trim() == "pub(crate)"
        || before_keyword.trim() == "pub(super)"
        || before_keyword.trim() == "async"
        || before_keyword.trim() == "pub async"
        || before_keyword.trim() == "unsafe"
        || before_keyword.trim() == "pub unsafe"
        || before_keyword.trim() == "default"
        || before_keyword.trim() == "const"
        || before_keyword.trim() == "extern"
        || before_keyword.trim() == "pub const"
        || before_keyword.trim() == "pub static";

    if !is_definition {
        return None;
    }

    // Incluir el scope en el nombre para distinguir definiciones en diferentes impl blocks
    let full_name = if scope.is_empty() {
        format!("{} {}", keyword.trim(), name)
    } else {
        format!("{} {} ({})", keyword.trim(), name, scope)
    };

    Some(full_name)
}

/// Detecta texto de razonamiento del modelo inyectado en archivos de código sin
/// marcadores de comentario.
fn detect_reasoning_injection(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();

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

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with("*")
            || trimmed.starts_with("*/")
            || trimmed.starts_with("#")
            || trimmed.starts_with("<!--")
            || trimmed.starts_with("///")
            || trimmed.starts_with("//!")
        {
            continue;
        }

        for pattern in reasoning_patterns {
            let trimmed_lower = trimmed.to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            if trimmed_lower.starts_with(&pattern_lower) {
                let looks_like_code = trimmed.contains('(')
                    || trimmed.contains('{')
                    || trimmed.contains(';')
                    || trimmed.contains("fn ")
                    || trimmed.contains("let ")
                    || trimmed.contains("pub ")
                    || trimmed.contains("use ")
                    || trimmed.contains("mod ")
                    || trimmed.contains("struct ")
                    || trimmed.contains("enum ")
                    || trimmed.contains("impl ")
                    || trimmed.contains("const ")
                    || trimmed.contains("import ")
                    || trimmed.contains("from ")
                    || trimmed.contains("def ")
                    || trimmed.contains("class ")
                    || trimmed.contains("function ")
                    || trimmed.contains("var ")
                    || trimmed.contains("return ")
                    || trimmed.contains("match ");

                if !looks_like_code {
                    warnings.push(format!(
                        "POSIBLE RAZONAMIENTO SIN COMENTAR (línea {}): \"{}\" — parece texto de lenguaje natural, no código. Si es intencional, usa // para comentarlo.",
                        line_num + 1,
                        truncate_for_display(trimmed, 80)
                    ));
                    break;
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

    // =========================================================================
    // Tests para detect_duplicate_lines
    // =========================================================================

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
        let content = "fn foo() {\n    if true {\n    }\n}\n}\n";
        let warnings = detect_duplicate_lines(content);
        assert!(warnings.iter().all(|w| !w.contains("\"}\"")));
    }

    // NUEVO: Verificar que los argumentos de macro no se reportan como duplicados
    #[test]
    fn test_macro_args_not_flagged_as_duplicates() {
        // Simula el caso de format!("...", call_id, call_id, call_id) que es perfectamente normal
        let content = "fn foo() {\n    format!(\n        \"{}\",\n        call_id,\n        call_id,\n        call_id\n    );\n}\n";
        let warnings = detect_duplicate_lines(content);
        // No debería reportar los "call_id," como duplicados
        assert!(!warnings.iter().any(|w| w.contains("call_id")));
    }

    #[test]
    fn test_macro_args_inside_format_not_duplicated() {
        let content = "format!(\n    \"{}\\n{}\\n{}\",\n    x,\n    x,\n    x\n);\n";
        let warnings = detect_duplicate_lines(content);
        assert!(!warnings.iter().any(|w| w.contains("\"x,\"")));
    }

    // =========================================================================
    // Tests para detect_duplicate_definitions (con scope tracking)
    // =========================================================================

    #[test]
    fn test_diff_impl_blocks_not_duplicates() {
        // fn new() en diferentes impl blocks NO debe ser reportado
        let content = "\
impl Foo {
    pub fn new() -> Self { Self }
}

impl Bar {
    pub fn new() -> Self { Self }
}

impl Baz {
    pub fn new() -> Self { Self }
}
";
        let warnings = detect_duplicate_definitions(content);
        // No debe haber advertencias de duplicados porque están en diferentes impl blocks
        assert!(!warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    #[test]
    fn test_same_impl_block_duplicates_detected() {
        // fn new() repetido en el MISMO impl block SÍ debe ser reportado
        let content = "\
impl Foo {
    pub fn new() -> Self { Self }
    pub fn new() -> Self { Self }
}
";
        let warnings = detect_duplicate_definitions(content);
        assert!(warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    #[test]
    fn test_file_scope_duplicates_detected() {
        // Dos fn main() a nivel de archivo SÍ deben ser reportadas
        let content = "\
fn main() {
    println!(\"hello\");
}

fn main() {
    println!(\"world\");
}
";
        let warnings = detect_duplicate_definitions(content);
        assert!(warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    #[test]
    fn test_reap_old_on_different_types_not_duplicates() {
        // Simula el caso real: reap_old en ToolResultStore y SubAgentManager
        let content = "\
impl ToolResultStore {
    pub fn reap_old(&self, max_age_secs: u64) -> usize { 0 }
}

impl SubAgentManager {
    pub fn reap_old(&self, max_age_secs: u64) -> usize { 0 }
}
";
        let warnings = detect_duplicate_definitions(content);
        assert!(!warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    // =========================================================================
    // Tests para balanced_delimiters
    // =========================================================================

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

    // =========================================================================
    // Tests para detect_reasoning_injection
    // =========================================================================

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
        let content = "// OK, ahora necesito modificar esta función\nfn foo() {\n    let x = 1;\n}\n";
        let warnings = detect_reasoning_injection(content);
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
