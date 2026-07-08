//! Módulo de validación post-escritura.
//!
//! Este módulo proporciona funciones para validar archivos después de ser modificados
//! por el agente, detectando problemas comunes como:
//! - Líneas duplicadas consecutivas (copy-paste accidental)
//! - Delimitadores no balanceados (llaves, paréntesis, corchetes)
//! - Razonamiento del modelo inyectado sin comentarios
//! - Errores de sintaxis en archivos .rs y .js
//!
//! **CORRECCIÓN CRÍTICA (2026-07-08):** El validador ahora es consciente de la sintaxis
//! de Rust: ignora delimitadores y definiciones dentro de strings literales,
//! strings raw (r#"..."#), comentarios de línea (//) y comentarios de bloque (/* */).
//! Esto evita falsos positivos en código de test que contiene fragmentos de Rust
//! dentro de strings.

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
pub fn validate_file_after_write(file_path: &str, _content: &str) -> ValidationResult {
    let mut result = ValidationResult {
        path: file_path.to_string(),
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    let path = Path::new(file_path);
    if !path.exists() {
        return result;
    }

    let disk_content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            result.errors.push(format!("No se pudo leer el archivo para validación: {}", e));
            return result;
        }
    };

    // Pre-calcular máscara de caracteres "activos" (no dentro de strings/comentarios)
    let is_active = compute_active_mask(&disk_content);

    let dup_warnings = detect_duplicate_lines(&disk_content, &is_active);
    result.warnings.extend(dup_warnings);

    let def_warnings = detect_duplicate_definitions(&disk_content, &is_active);
    result.warnings.extend(def_warnings);

    let reasoning_warnings = detect_reasoning_injection(&disk_content);
    result.warnings.extend(reasoning_warnings);

    let delim_errors = check_balanced_delimiters(&disk_content, &is_active);
    result.errors.extend(delim_errors);

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

// ============================================================================
// MÁSCARA DE CARACTERES ACTIVOS
// ============================================================================

/// Tipos de contextos que "desactivan" caracteres para el análisis de delimitadores.
#[derive(Clone, Copy, PartialEq)]
enum CharContext {
    /// Carácter normal de código (activo para análisis)
    Active,
    /// Dentro de un string literal: "..."
    InString,
    /// Dentro de un string raw: r"..." o r#"..."#
    InRawString,
    /// Dentro de un comentario de línea: // ...
    InLineComment,
    /// Dentro de un comentario de bloque: /* ... */
    InBlockComment,
    /// Carácter de escape (\) dentro de un string — el siguiente char se ignora
    Escaped,
}

/// Computa una máscara que indica, para cada byte del contenido, si está en
/// un contexto "activo" (código real) o "inactivo" (string, comentario, etc.).
///
/// Retorna un Vec<bool> donde true = el carácter en esa posición debe ser
/// considerado para análisis de delimitadores y definiciones.
fn compute_active_mask(content: &str) -> Vec<bool> {
    let chars: Vec<char> = content.chars().collect();
    let mut mask = vec![true; chars.len()];
    let mut context = CharContext::Active;
    let mut raw_hash_count: usize = 0; // Para r#"..."# — cuenta los # después de r

    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];

        match context {
            CharContext::Active => {
                mask[i] = true;

                // Detectar inicio de string raw: r" o r#" ... "#
                if ch == 'r' && i + 1 < chars.len() && (chars[i + 1] == '"' || chars[i + 1] == '#') {
                    // Verificar que es realmente r" o r#"
                    if chars[i + 1] == '"' {
                        context = CharContext::InRawString;
                        raw_hash_count = 0;
                        mask[i] = false; // 'r' no es código activo
                        i += 1;
                        mask[i] = false; // '"' tampoco
                    } else if chars[i + 1] == '#' {
                        // Contar cuántos # hay
                        let mut j = i + 1;
                        while j < chars.len() && chars[j] == '#' {
                            j += 1;
                        }
                        if j < chars.len() && chars[j] == '"' {
                            raw_hash_count = j - (i + 1);
                            context = CharContext::InRawString;
                            // Marcar todos estos caracteres como inactivos
                            for k in i..=j {
                                mask[k] = false;
                            }
                            i = j;
                        }
                    }
                }
                // Detectar inicio de string normal: "
                else if ch == '"' {
                    context = CharContext::InString;
                    mask[i] = false;
                }
                // Detectar comentario de línea: //
                else if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    context = CharContext::InLineComment;
                    mask[i] = false;
                    i += 1;
                    mask[i] = false;
                }
                // Detectar comentario de bloque: /*
                else if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                    context = CharContext::InBlockComment;
                    mask[i] = false;
                    i += 1;
                    mask[i] = false;
                }
            }

            CharContext::InString => {
                mask[i] = false;
                if ch == '\\' {
                    context = CharContext::Escaped;
                } else if ch == '"' {
                    context = CharContext::Active;
                }
            }

            CharContext::Escaped => {
                mask[i] = false;
                context = CharContext::InString; // Volver al string después del escape
            }

            CharContext::InRawString => {
                mask[i] = false;
                // Buscar el cierre: "# (con el número correcto de #)
                if ch == '"' {
                    // Verificar si después hay exactamente raw_hash_count #
                    let mut j = i + 1;
                    let mut found_hashes = 0;
                    while j < chars.len() && chars[j] == '#' && found_hashes < raw_hash_count {
                        found_hashes += 1;
                        j += 1;
                    }
                    if found_hashes == raw_hash_count {
                        // Es el cierre
                        for k in i..j {
                            mask[k] = false;
                        }
                        i = j - 1; // -1 porque el bucle hace i += 1
                        context = CharContext::Active;
                    }
                }
            }

            CharContext::InLineComment => {
                mask[i] = false;
                if ch == '\n' {
                    context = CharContext::Active;
                    mask[i] = true; // El newline sí es activo (no queremos unir líneas)
                }
            }

            CharContext::InBlockComment => {
                mask[i] = false;
                if ch == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    mask[i] = false;
                    i += 1;
                    mask[i] = false;
                    context = CharContext::Active;
                }
            }
        }

        i += 1;
    }

    mask
}

// ============================================================================
// DETECCIÓN DE LÍNEAS DUPLICADAS
// ============================================================================

fn detect_duplicate_lines(content: &str, is_active: &[bool]) -> Vec<String> {
    let mut warnings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 2 {
        return warnings;
    }

    // Calcular la profundidad de paréntesis activa (ignorando strings/comentarios)
    let chars: Vec<char> = content.chars().collect();
    let mut paren_depth: Vec<i32> = Vec::with_capacity(lines.len());
    let mut depth: i32 = 0;
    let mut char_idx = 0;

    for line in &lines {
        paren_depth.push(depth);
        for ch in line.chars() {
            if char_idx < is_active.len() && is_active[char_idx] {
                match ch {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
            }
            char_idx += 1;
        }
        // El newline no está en line.chars(), avanzamos manualmente
        char_idx += 1; // para el '\n'
    }

    let mut i = 0;
    while i < lines.len() - 1 {
        let current = lines[i].trim();
        let next = lines[i + 1].trim();

        if !current.is_empty() && current == next {
            let is_structural = current == "}" || current == ")" || current == "]"
                || current.starts_with("//") || current == "};" || current == "});";

            let inside_macro = paren_depth[i] > 0;

            let looks_like_macro_arg = current.ends_with(',')
                && !current.contains(' ')
                && current.len() < 40;

            if !is_structural && !inside_macro && !looks_like_macro_arg {
                warnings.push(format!(
                    "Línea duplicada detectada (línea {}): \"{}\"",
                    i + 1,
                    truncate_for_display(current, 60)
                ));

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

// ============================================================================
// DELIMITADORES BALANCEADOS
// ============================================================================

fn check_balanced_delimiters(content: &str, is_active: &[bool]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut stack: Vec<(char, usize)> = Vec::new();

    let chars: Vec<char> = content.chars().collect();
    let mut line_num = 1;

    for (idx, &ch) in chars.iter().enumerate() {
        if ch == '\n' {
            line_num += 1;
            continue;
        }

        // Solo considerar caracteres "activos" (no en strings/comentarios)
        if idx >= is_active.len() || !is_active[idx] {
            continue;
        }

        match ch {
            '{' | '(' | '[' => stack.push((ch, line_num)),
            '}' => {
                if stack.last().map(|&(c, _)| c) == Some('{') {
                    stack.pop();
                } else {
                    errors.push(format!(
                        "Llave '}}' no balanceada en línea {} (esperaba '{}')",
                        line_num,
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
                        line_num,
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
                        line_num,
                        stack.last().map(|&(c, _)| matching_open(c)).unwrap_or('?')
                    ));
                }
            }
            _ => {}
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

fn matching_open(close: char) -> char {
    match close {
        '}' => '{',
        ')' => '(',
        ']' => '[',
        _ => '?',
    }
}

// ============================================================================
// ERRORES COMUNES DE RUST
// ============================================================================

fn check_rust_common_errors(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("unsafe {") && !trimmed.starts_with("//") {
            warnings.push(format!(
                "Bloque 'unsafe' detectado en línea {} - verificar que sea intencional",
                i + 1
            ));
        }
    }
    warnings
}

// ============================================================================
// DEFINICIONES DUPLICADAS (con scope tracking y máscara de strings)
// ============================================================================

fn detect_duplicate_definitions(content: &str, is_active: &[bool]) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut definitions: std::collections::HashMap<String, Vec<(usize, String)>> =
        std::collections::HashMap::new();

    let chars: Vec<char> = content.chars().collect();
    let lines: Vec<&str> = content.lines().collect();
    let mut current_scope: String = String::new();
    let mut line_start_idx = 0;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        // Verificar si esta línea está completamente en un contexto inactivo
        // (string literal, comentario). Si es así, ignorarla completamente.
        let line_chars: Vec<char> = line.chars().collect();
        let all_inactive = line_chars.iter().enumerate().all(|(offset, &ch)| {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                return true; // whitespace es "neutral"
            }
            let abs_idx = line_start_idx + offset;
            abs_idx >= is_active.len() || !is_active[abs_idx]
        });

        // Si toda la línea está dentro de un string/comentario, ignorarla
        if all_inactive && !trimmed.is_empty() {
            line_start_idx += line.len() + 1;
            continue;
        }

        // Detectar entrada en un bloque impl
        if let Some(rest) = trimmed.strip_prefix("impl ") {
            // Verificar que "impl" está en contexto activo
            let impl_pos = line.as_bytes().iter().position(|&b| b == b'i').unwrap_or(0);
            let abs_pos = line_start_idx + impl_pos;
            if abs_pos < is_active.len() && is_active[abs_pos] {
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
        }

        // Detectar entrada en un bloque trait
        if let Some(rest) = trimmed.strip_prefix("trait ") {
            let trait_pos = line.as_bytes().iter().position(|&b| b == b't').unwrap_or(0);
            let abs_pos = line_start_idx + trait_pos;
            if abs_pos < is_active.len() && is_active[abs_pos] {
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
        }

        // Detectar cierre de bloque: "}" en contexto activo
        if trimmed == "}" {
            // Buscar el '}' en la línea
            if let Some(brace_pos) = line.find('}') {
                let abs_pos = line_start_idx + brace_pos;
                if abs_pos < is_active.len() && is_active[abs_pos] {
                    current_scope = String::new();
                }
            }
        }

        // Solo extraer definiciones si el keyword está en contexto activo
        for keyword in &["fn ", "struct ", "enum ", "trait ", "const ", "static ", "mod "] {
            if let Some(pos) = line.find(keyword) {
                let abs_pos = line_start_idx + pos;
                if abs_pos < is_active.len() && is_active[abs_pos] {
                    if let Some(name) = extract_def_name_with_scope(trimmed, keyword, &current_scope) {
                        // Para "mod", ignorar declaraciones externas (con ;)
                        if *keyword == "mod " && trimmed.ends_with(';') {
                            continue;
                        }
                        definitions
                            .entry(name.clone())
                            .or_default()
                            .push((line_num, current_scope.clone()));
                        break; // Una sola definición por línea
                    }
                }
            }
        }

        line_start_idx += line.len() + 1; // +1 para el '\n'
    }

    // Agrupar definiciones duplicadas por (nombre, ámbito)
    let mut dup_by_scope: std::collections::HashMap<(String, String), Vec<usize>> =
        std::collections::HashMap::new();

    for (def_name, occurrences) in &definitions {
        if occurrences.len() <= 1 {
            continue;
        }
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

fn extract_def_name_with_scope<'a>(
    line: &'a str,
    keyword: &str,
    scope: &str,
) -> Option<String> {
    let line = line.trim();
    let pos = line.find(keyword)?;
    let after_keyword = &line[pos + keyword.len()..];

    let name_end = after_keyword
        .find(|c: char| c == '(' || c == '{' || c == '<' || c == ';' || c == ':')
        .unwrap_or(after_keyword.len());
    let name = after_keyword[..name_end].trim();

    if name.is_empty() || name == "(" || name == "{" {
        return None;
    }

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

    let full_name = if scope.is_empty() {
        format!("{} {}", keyword.trim(), name)
    } else {
        format!("{} {} ({})", keyword.trim(), name, scope)
    };

    Some(full_name)
}

// ============================================================================
// DETECCIÓN DE RAZONAMIENTO INYECTADO
// ============================================================================

fn detect_reasoning_injection(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    let reasoning_patterns: &[&str] = &[
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
        if trimmed.is_empty() { continue; }
        if trimmed.starts_with("//") || trimmed.starts_with("/*")
            || trimmed.starts_with("*") || trimmed.starts_with("*/")
            || trimmed.starts_with("#") || trimmed.starts_with("<!--")
            || trimmed.starts_with("///") || trimmed.starts_with("//!")
        { continue; }

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

// ============================================================================
// UTILIDADES
// ============================================================================

fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len).collect::<String>())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Tests para compute_active_mask
    // =========================================================================

    #[test]
    fn test_active_mask_simple_code() {
        let mask = compute_active_mask("fn main() { let x = 1; }");
        // Todo debe ser activo
        assert!(mask.iter().all(|&a| a));
    }

    #[test]
    fn test_active_mask_string_literal() {
        let content = "let s = \"hello {world}\";";
        let mask = compute_active_mask(content);
        let chars: Vec<char> = content.chars().collect();
        // Encontrar las posiciones de '{' y '}' dentro del string
        let brace_open_pos = chars.iter().position(|&c| c == '{').unwrap();
        let brace_close_pos = chars.iter().position(|&c| c == '}').unwrap();
        // Deben estar marcadas como inactivas
        assert!(!mask[brace_open_pos], "{{ inside string should be inactive");
        assert!(!mask[brace_close_pos], "}} inside string should be inactive");
    }

    #[test]
    fn test_active_mask_raw_string() {
        let content = "let s = r#\"raw {text}\"#;";
        let mask = compute_active_mask(content);
        let chars: Vec<char> = content.chars().collect();
        let brace_pos = chars.iter().position(|&c| c == '{').unwrap();
        assert!(!mask[brace_pos], "{{ inside raw string should be inactive");
    }

    #[test]
    fn test_active_mask_line_comment() {
        let content = "let x = 1; // this is a { comment";
        let mask = compute_active_mask(content);
        let chars: Vec<char> = content.chars().collect();
        let brace_pos = chars.iter().position(|&c| c == '{').unwrap();
        assert!(!mask[brace_pos], "{{ inside line comment should be inactive");
    }

    #[test]
    fn test_active_mask_block_comment() {
        let content = "fn foo() /* inline { block } */ { }";
        let mask = compute_active_mask(content);
        let chars: Vec<char> = content.chars().collect();
        // Las llaves dentro del block comment no deben contar
        let active_braces: Vec<usize> = chars.iter().enumerate()
            .filter(|(_, &c)| c == '{' || c == '}')
            .filter(|(i, _)| mask[*i])
            .map(|(i, _)| i)
            .collect();
        // Solo las llaves reales de fn foo() { } deben ser activas
        assert_eq!(active_braces.len(), 2, "Only 2 active braces expected (fn body)");
    }

    #[test]
    fn test_active_mask_escaped_quote() {
        let content = "let s = \"escaped \\\"quote\\\" here\"; let x = 1;";
        let mask = compute_active_mask(content);
        // "let x = 1;" debe estar activo
        let semi_pos = content.find("let x = 1;").unwrap();
        assert!(mask[semi_pos], "Code after escaped string should be active");
    }

    // =========================================================================
    // Tests para check_balanced_delimiters (con máscara)
    // =========================================================================

    #[test]
    fn test_balanced_delimiters_with_string() {
        let content = "fn main() {\n    let s = \"hello (world)\";\n}\n";
        let mask = compute_active_mask(content);
        let errors = check_balanced_delimiters(content, &mask);
        assert!(errors.is_empty(), "Delimiters inside strings should be ignored");
    }

    #[test]
    fn test_balanced_delimiters_with_raw_string() {
        let content = "fn main() {\n    let s = r#\"raw {[(text)]}\"#;\n}\n";
        let mask = compute_active_mask(content);
        let errors = check_balanced_delimiters(content, &mask);
        assert!(errors.is_empty(), "Delimiters inside raw strings should be ignored");
    }

    #[test]
    fn test_balanced_delimiters_with_comments() {
        let content = "fn main() { // { [ (\n    let x = 1;\n} // ) ] }\n";
        let mask = compute_active_mask(content);
        let errors = check_balanced_delimiters(content, &mask);
        assert!(errors.is_empty(), "Delimiters in comments should be ignored");
    }

    #[test]
    fn test_balanced_delimiters_real_unbalanced() {
        let content = "fn main() {\n    let x = (1 + 2;\n}\n";
        let mask = compute_active_mask(content);
        let errors = check_balanced_delimiters(content, &mask);
        assert!(!errors.is_empty(), "Real unbalanced parens should be detected");
    }

    // =========================================================================
    // Tests para detect_duplicate_definitions (con scope y máscara)
    // =========================================================================

    #[test]
    fn test_diff_impl_blocks_not_duplicates() {
        let content = "\
impl Foo {
    pub fn new() -> Self { Self }
}

impl Bar {
    pub fn new() -> Self { Self }
}
";
        let mask = compute_active_mask(content);
        let warnings = detect_duplicate_definitions(content, &mask);
        assert!(!warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    #[test]
    fn test_same_impl_block_duplicates_detected() {
        let content = "\
impl Foo {
    pub fn new() -> Self { Self }
    pub fn new() -> Self { Self }
}
";
        let mask = compute_active_mask(content);
        let warnings = detect_duplicate_definitions(content, &mask);
        assert!(warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    #[test]
    fn test_definitions_in_strings_ignored() {
        // Las definiciones "fn foo" dentro de un string NO deben contar
        let content = "let code = \"fn foo() { } fn foo() { }\";\nfn real_func() { }\n";
        let mask = compute_active_mask(content);
        let warnings = detect_duplicate_definitions(content, &mask);
        assert!(!warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    #[test]
    fn test_reap_old_on_different_types_not_duplicates() {
        let content = "\
impl ToolResultStore {
    pub fn reap_old(&self, max_age_secs: u64) -> usize { 0 }
}

impl SubAgentManager {
    pub fn reap_old(&self, max_age_secs: u64) -> usize { 0 }
}
";
        let mask = compute_active_mask(content);
        let warnings = detect_duplicate_definitions(content, &mask);
        assert!(!warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    // =========================================================================
    // Tests para detect_duplicate_lines
    // =========================================================================

    #[test]
    fn test_macro_args_not_flagged_as_duplicates() {
        let content = "fn foo() {\n    format!(\n        \"{}\",\n        call_id,\n        call_id,\n        call_id\n    );\n}\n";
        let mask = compute_active_mask(content);
        let warnings = detect_duplicate_lines(content, &mask);
        assert!(!warnings.iter().any(|w| w.contains("call_id")));
    }

    // =========================================================================
    // Tests para detect_reasoning_injection
    // =========================================================================

    #[test]
    fn test_reasoning_injection_spanish_detected() {
        let content = "OK, ahora necesito modificar esta función\nfn foo() { let x = 1; }\n";
        let warnings = detect_reasoning_injection(content);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("OK, ahora")));
    }

    #[test]
    fn test_reasoning_injection_commented_ignored() {
        let content = "// OK, ahora necesito modificar esta función\nfn foo() { let x = 1; }\n";
        let warnings = detect_reasoning_injection(content);
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("POSIBLE RAZONAMIENTO")));
    }

    #[test]
    fn test_reasoning_injection_clean_code_ok() {
        let content = "fn foo() {\n    let x = 1;\n    let y = 2;\n}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("POSIBLE RAZONAMIENTO")));
    }
}
