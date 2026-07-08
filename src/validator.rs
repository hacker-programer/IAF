//! Módulo de validación post-escritura.
//!
//! Este módulo proporciona funciones para validar archivos después de ser modificados
//! por el agente, detectando problemas comunes como:
//! - Líneas duplicadas consecutivas (copy-paste accidental)
//! - Delimitadores no balanceados (llaves, paréntesis, corchetes)
//! - Razonamiento del modelo inyectado sin comentarios
//! - Definiciones duplicadas reales (misma fn/struct en el mismo scope)
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

    // === VALIDACIÓN 1.5: Detectar definiciones duplicadas (con scope-awareness) ===
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
/// Ignora líneas estructurales y líneas que parecen argumentos de macro.
fn detect_duplicate_lines(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 2 {
        return warnings;
    }

    // Primero, construir un mapa de "profundidad de macro" para cada línea.
    // Las líneas dentro de format!(...) u otras macros con muchos argumentos
    // no deberían ser penalizadas por tener texto repetido (como "call_id,").
    let macro_depth = compute_macro_depth(&lines);

    let mut i = 0;
    let mut duplicate_blocks = 0usize;
    while i < lines.len() - 1 {
        let current = lines[i].trim();
        let next = lines[i + 1].trim();

        // Solo reportar duplicados si la línea no está vacía y es idéntica
        if !current.is_empty() && current == next {
            // Evitar falsos positivos en líneas estructurales
            let is_structural = current == "}" || current == ")" || current == "]"
                || current.starts_with("//") || current == "};" || current == "});";

            // Evitar falsos positivos dentro de invocaciones de macro profundas
            // (ej. argumentos de format!, json!, vec! con texto repetitivo)
            let is_macro_arg = macro_depth[i] >= 2;

            if !is_structural && !is_macro_arg {
                warnings.push(format!(
                    "Línea duplicada detectada (línea {}): \"{}\"",
                    i + 1,
                    truncate_for_display(current, 60)
                ));

                // Saltar todas las repeticiones consecutivas
                while i < lines.len() - 1 && lines[i].trim() == lines[i + 1].trim() {
                    i += 1;
                }
                duplicate_blocks += 1;
            }
        }
        i += 1;
    }

    if !warnings.is_empty() {
        warnings.push(format!(
            "Se encontraron {} bloques de líneas duplicadas. Esto suele ser resultado de copy-paste accidental del agente.",
            duplicate_blocks
        ));
    }

    warnings
}

/// Calcula la "profundidad de macro" para cada línea.
/// Cuando estamos dentro de `format!(`, `json!(`, `vec![`, etc., 
/// cada línea dentro cuenta como profundidad >= 1.
/// Las líneas con profundidad >= 2 son probablemente argumentos de macro
/// y no deberían generar falsos positivos de duplicación.
fn compute_macro_depth(lines: &[&str]) -> Vec<usize> {
    let mut depths = vec![0usize; lines.len()];
    let mut current_depth = 0usize;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detectar inicio de macro: nombre!( o nombre![
        // donde el nombre es un identificador seguido de !( o ![
        let open_parens = trimmed.matches('(').count();
        let open_brackets = trimmed.matches('[').count();
        let close_parens = trimmed.matches(')').count();
        let close_brackets = trimmed.matches(']').count();

        // Si la línea contiene !( o ![, es probablemente inicio de macro
        if trimmed.contains("!(") || trimmed.contains("![") {
            current_depth += 1;
        }

        depths[i] = current_depth;

        // Ajustar profundidad según balance de paréntesis/corchetes en esta línea
        let net_open = (open_parens + open_brackets) as i32 - (close_parens + close_brackets) as i32;
        if net_open > 0 {
            current_depth = current_depth.saturating_add(net_open as usize);
        } else if net_open < 0 {
            current_depth = current_depth.saturating_sub((-net_open) as usize);
        }
    }

    depths
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

fn matching_open(close: char) -> char {
    match close {
        '}' => '{',
        ')' => '(',
        ']' => '[',
        _ => '?',
    }
}

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

/// Detecta definiciones duplicadas de funciones, structs, enums, traits, constantes y módulos.
///
/// **CORREGIDO**: Ahora rastrea el scope actual (impl blocks, funciones) para evitar
/// falsos positivos como `fn new()` en diferentes `impl` blocks, o `static KEY` en
/// diferentes funciones. Las definiciones se califican con su scope padre.
fn detect_duplicate_definitions(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    // Map de nombre_calificado → vec de números de línea
    let mut definitions: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();

    let lines: Vec<&str> = content.lines().collect();
    
    // Rastrear el scope actual para calificar nombres
    let mut current_impl: Option<String> = None;   // ej: "impl ToolResultStore"
    let mut current_fn: Option<String> = None;      // ej: "fn new"
    let mut brace_depth: i32 = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Rastrear profundidad de llaves
        let opens = trimmed.matches('{').count() as i32;
        let closes = trimmed.matches('}').count() as i32;
        brace_depth += opens - closes;

        // Detectar inicio de impl block
        if let Some(name) = extract_impl_target(trimmed) {
            current_impl = Some(name);
        }

        // Detectar inicio de fn (a nivel superior o dentro de impl)
        if let Some(fn_name) = extract_def_name(trimmed, "fn ") {
            if brace_depth <= 1 || current_impl.is_some() {
                current_fn = Some(fn_name.clone());
                
                // Calificar el nombre con el scope
                let qualified = if let Some(ref impl_name) = current_impl {
                    format!("{}::{}", impl_name, fn_name)
                } else {
                    fn_name.clone()
                };
                definitions.entry(qualified).or_default().push(i + 1);
            }
        }

        // Detectar struct (solo a nivel superior)
        if brace_depth <= 0 {
            if let Some(name) = extract_def_name(trimmed, "struct ") {
                definitions.entry(format!("struct {}", name)).or_default().push(i + 1);
            }
            if let Some(name) = extract_def_name(trimmed, "enum ") {
                definitions.entry(format!("enum {}", name)).or_default().push(i + 1);
            }
            if let Some(name) = extract_def_name(trimmed, "trait ") {
                definitions.entry(format!("trait {}", name)).or_default().push(i + 1);
            }
            if let Some(name) = extract_def_name(trimmed, "mod ") {
                if !trimmed.ends_with(';') {
                    definitions.entry(format!("mod {}", name)).or_default().push(i + 1);
                }
            }
        }

        // Detectar const/static — calificar con la función padre si estamos dentro de una
        if let Some(name) = extract_def_name(trimmed, "const ") {
            let qualified = if let Some(ref fn_name) = current_fn {
                format!("{}::const {}", fn_name, name)
            } else {
                format!("const {}", name)
            };
            definitions.entry(qualified).or_default().push(i + 1);
        }
        if let Some(name) = extract_def_name(trimmed, "static ") {
            let qualified = if let Some(ref fn_name) = current_fn {
                format!("{}::static {}", fn_name, name)
            } else {
                format!("static {}", name)
            };
            definitions.entry(qualified).or_default().push(i + 1);
        }

        // Resetear scopes cuando salimos
        if brace_depth <= 0 {
            current_impl = None;
            current_fn = None;
        } else if brace_depth <= 1 && current_impl.is_some() {
            // Seguimos dentro del impl pero fuera de cualquier fn
            current_fn = None;
        }
    }

    // Reportar solo las que tienen > 1 ubicación
    let mut duplicate_count = 0usize;
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
            duplicate_count += 1;
        }
    }

    if !warnings.is_empty() {
        warnings.push(format!(
            "Se encontraron {} definiciones duplicadas. EDITAR EL ARCHIVO COMPLETO, no uses start_line/end_line.",
            duplicate_count
        ));
    }

    warnings
}

/// Extrae el nombre del tipo en `impl TypeName` o `impl Trait for TypeName`.
fn extract_impl_target(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("impl ") && !line.starts_with("impl<") {
        return None;
    }
    // Remover "impl " o "impl<...> "
    let after_impl = if let Some(pos) = line.find("impl ") {
        &line[pos + 5..]
    } else {
        return None;
    };
    // Si tiene "for", extraer lo que está después del "for"
    let target = if let Some(for_pos) = after_impl.find(" for ") {
        &after_impl[for_pos + 5..]
    } else {
        after_impl
    };
    // Tomar hasta '{' o el final
    let name_end = target.find(|c: char| c == '{' || c == '<').unwrap_or(target.len());
    let name = target[..name_end].trim();
    if name.is_empty() {
        None
    } else {
        Some(format!("impl {}", name))
    }
}

/// Extrae el nombre de una definición. Ej: "pub fn foo(" → "fn foo", "struct Bar {" → "struct Bar"
fn extract_def_name<'a>(line: &'a str, keyword: &str) -> Option<String> {
    let line = line.trim();
    let pos = line.find(keyword)?;
    let after_keyword = &line[pos + keyword.len()..];
    let name_end = after_keyword.find(|c: char| c == '(' || c == '{' || c == '<' || c == ';' || c == ':').unwrap_or(after_keyword.len());
    let name = after_keyword[..name_end].trim();
    if name.is_empty() || name == "(" || name == "{" {
        return None;
    }
    let before_keyword = &line[..pos];
    if before_keyword.trim().is_empty() || before_keyword.trim() == "pub" || before_keyword.trim() == "pub(crate)" || before_keyword.trim() == "pub(super)" || before_keyword.trim() == "async" || before_keyword.trim() == "pub async" || before_keyword.trim() == "unsafe" || before_keyword.trim() == "pub unsafe" || before_keyword.trim() == "default" || before_keyword.trim() == "const" || before_keyword.trim() == "extern" {
        Some(format!("{} {}", keyword.trim(), name))
    } else {
        None
    }
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
        
        if trimmed.starts_with("//") || trimmed.starts_with("/*") 
            || trimmed.starts_with("*") || trimmed.starts_with("*/")
            || trimmed.starts_with("#") || trimmed.starts_with("<!--")
            || trimmed.starts_with("///") || trimmed.starts_with("//!") 
        {
            continue;
        }
        
        for pattern in reasoning_patterns {
            let trimmed_lower = trimmed.to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            if trimmed_lower.starts_with(&pattern_lower) {
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

    #[test]
    fn test_detect_duplicate_lines_macro_args_ignored() {
        // Líneas dentro de format!() no deberían generar falsos positivos
        let content = "let msg = format!(\n    \"{}\",\n    call_id,\n    call_id,\n    call_id\n);\n";
        let warnings = detect_duplicate_lines(content);
        // No debería reportar "call_id," como duplicado porque está dentro de macro
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("call_id")));
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

    // === Tests para detect_duplicate_definitions (scope-aware) ===

    #[test]
    fn test_duplicate_defs_different_impl_blocks_ok() {
        // fn new() en diferentes impl blocks NO es un error
        let content = "\
impl ToolResultStore {
    pub fn new() -> Self { Self {} }
}
impl SubAgentManager {
    pub fn new() -> Self { Self {} }
}
impl ProcessRegistry {
    pub fn new() -> Self { Self {} }
}";
        let warnings = detect_duplicate_definitions(content);
        // No debería haber advertencias porque cada fn new() está en un impl diferente
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    #[test]
    fn test_duplicate_defs_same_scope_detected() {
        // Dos fn new() en el mismo scope SÍ es un error
        let content = "\
impl Foo {
    pub fn new() -> Self { Self {} }
    pub fn new() -> Self { Self {} }
}";
        let warnings = detect_duplicate_definitions(content);
        assert!(warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    #[test]
    fn test_duplicate_defs_static_in_different_fns_ok() {
        // static KEY en diferentes funciones es válido
        let content = "\
fn deepseek_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var(\"DEEPSEEK\").unwrap())
}
fn voyage_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var(\"VOYAGE\").unwrap())
}";
        let warnings = detect_duplicate_definitions(content);
        // No debería reportar static KEY como duplicada (están en diferentes funciones)
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("DUPLICADA")));
    }

    // === Tests para detect_reasoning_injection ===

    #[test]
    fn test_reasoning_injection_spanish_detected() {
        let content = "OK, ahora necesito modificar esta función\nfn foo() {\n    let x = 1;\n}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_reasoning_injection_commented_ignored() {
        let content = "// OK, ahora necesito modificar esta función\nfn foo() {\n    let x = 1;\n}\n";
        let warnings = detect_reasoning_injection(content);
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("POSIBLE RAZONAMIENTO")));
    }
}
