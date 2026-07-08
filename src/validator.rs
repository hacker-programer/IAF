//! Módulo de validación post-escritura.
//!
//! Este módulo proporciona funciones para validar archivos después de ser modificados
//! por el agente, detectando problemas comunes como:
//! - Líneas duplicadas consecutivas (copy-paste accidental)
//! - Delimitadores no balanceados (llaves, paréntesis, corchetes)
//! - Razonamiento del modelo inyectado sin comentarios
//! - Definiciones duplicadas (con contexto de impl blocks)
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

    // === VALIDACIÓN 1: Detectar líneas duplicadas consecutivas ===
    let dup_warnings = detect_duplicate_lines(&disk_content);
    result.warnings.extend(dup_warnings);

    // === VALIDACIÓN 2: Detectar definiciones duplicadas con contexto impl ===
    let def_warnings = detect_duplicate_definitions(&disk_content);
    result.warnings.extend(def_warnings);

    // === VALIDACIÓN 3: Detectar razonamiento del modelo inyectado sin comentarios ===
    let reasoning_warnings = detect_reasoning_injection(&disk_content);
    result.warnings.extend(reasoning_warnings);

    // === VALIDACIÓN 4: Verificar delimitadores balanceados ===
    let delim_errors = check_balanced_delimiters(&disk_content);
    result.errors.extend(delim_errors);

    // === VALIDACIÓN 5: Verificar sintaxis específica del lenguaje ===
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

        if !current.is_empty() && current == next {
            let is_structural = current == "}" || current == ")" || current == "]"
                || current.starts_with("//") || current == "};" || current == "});"
                || current == ","  // argumentos repetidos en macros/funciones (ej: format!)
                || current.ends_with(",") && current.chars().filter(|&c| c == '(' || c == '{').count() == 0;
                // ↑ ignora líneas como "call_id," que son argumentos de función repetidos intencionalmente

            if !is_structural {
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

/// Verifica patrones comunes de error en archivos Rust.
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

/// Detecta definiciones duplicadas de funciones, structs, enums, traits, constantes y módulos,
/// con conciencia de contexto `impl` para evitar falsos positivos.
///
/// **Corrección de falsos positivos (2026-07-08):**
/// Ahora trackea bloques `impl` para prefijar métodos con el nombre del struct,
/// evitando que `fn new()` en `impl ToolResultStore` se confunda con
/// `fn new()` en `impl SubAgentManager`. La clave ahora es `StructName::fn new`
/// en lugar de solo `fn new`.
fn detect_duplicate_definitions(content: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut definitions: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();

    let lines: Vec<&str> = content.lines().collect();

    // Stack de contextos impl: (nombre_del_struct, profundidad_de_llaves_al_entrar)
    #[derive(Clone)]
    struct ImplContext {
        struct_name: String,
        brace_depth_on_entry: usize,
    }

    let mut impl_stack: Vec<ImplContext> = Vec::new();
    let mut brace_depth: usize = 0;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        // ── Actualizar profundidad de llaves ──
        // Contamos { y } en esta línea
        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                        // Si salimos del nivel donde entramos a un impl, hacer pop
                        while let Some(ctx) = impl_stack.last() {
                            if brace_depth < ctx.brace_depth_on_entry {
                                impl_stack.pop();
                            } else {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // ── Detectar entrada a un impl block ──
        // Patrones: "impl StructName", "impl TraitName for StructName", "impl<T> StructName"
        if trimmed.starts_with("impl ") || trimmed.starts_with("pub impl ") {
            let after_impl = if trimmed.starts_with("pub impl ") {
                &trimmed[9..]
            } else {
                &trimmed[5..]
            };

            // Extraer el nombre del struct (lo que sigue a impl, antes de {, for, <, o espacio)
            let struct_name = extract_impl_struct_name(after_impl);
            if !struct_name.is_empty() {
                impl_stack.push(ImplContext {
                    struct_name,
                    brace_depth_on_entry: brace_depth,
                });
            }
        }

        // ── Detectar definiciones ──
        let current_impl = impl_stack.last().map(|ctx| ctx.struct_name.clone());

        // fn / pub fn / async fn
        if let Some(name) = extract_def_name_with_context(trimmed, "fn ", &current_impl) {
            definitions.entry(name).or_default().push(line_num);
        }
        // struct
        if let Some(name) = extract_def_name_with_context(trimmed, "struct ", &current_impl) {
            definitions.entry(name).or_default().push(line_num);
        }
        // enum
        if let Some(name) = extract_def_name_with_context(trimmed, "enum ", &current_impl) {
            definitions.entry(name).or_default().push(line_num);
        }
        // trait
        if let Some(name) = extract_def_name_with_context(trimmed, "trait ", &current_impl) {
            definitions.entry(name).or_default().push(line_num);
        }
        // const
        if let Some(name) = extract_def_name_with_context(trimmed, "const ", &current_impl) {
            definitions.entry(name).or_default().push(line_num);
        }
        // static
        if let Some(name) = extract_def_name_with_context(trimmed, "static ", &current_impl) {
            definitions.entry(name).or_default().push(line_num);
        }
        // mod (solo si tiene cuerpo, no declaración externa)
        if let Some(name) = extract_def_name_with_context(trimmed, "mod ", &current_impl) {
            if !trimmed.ends_with(';') {
                definitions.entry(name).or_default().push(line_num);
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

/// Extrae el nombre del struct de una declaración `impl`.
/// Ejemplos:
///   "ToolResultStore {" → "ToolResultStore"
///   "SubAgentManager {" → "SubAgentManager"
///   "ProcessRegistry {" → "ProcessRegistry"
///   "Display for MyType {" → "MyType"  (impl Trait for Type)
///   "MyType<T> where T: Clone {" → "MyType"
fn extract_impl_struct_name(after_impl: &str) -> String {
    let trimmed = after_impl.trim();

    // Si es "Trait for Struct", extraer el Struct
    if let Some(for_pos) = trimmed.find(" for ") {
        let after_for = &trimmed[for_pos + 5..];
        let name_end = after_for.find(|c: char| c == '{' || c == '<' || c == ' ' || c == '\n')
            .unwrap_or(after_for.len());
        return after_for[..name_end].trim().to_string();
    }

    // Si no, es "impl StructName" o "impl StructName<T>"
    let name_end = trimmed.find(|c: char| c == '{' || c == '<' || c == ' ' || c == '\n' || c == '\t')
        .unwrap_or(trimmed.len());
    trimmed[..name_end].trim().to_string()
}

/// Extrae el nombre de una definición con contexto de impl.
/// Si estamos dentro de un impl block, prefija el nombre con "StructName::".
/// Ejemplo: dentro de `impl ToolResultStore`, `fn new` → `ToolResultStore::fn new`
fn extract_def_name_with_context(line: &str, keyword: &str, current_impl: &Option<String>) -> Option<String> {
    let trimmed = line.trim();
    let pos = trimmed.find(keyword)?;
    let after_keyword = &trimmed[pos + keyword.len()..];

    let name_end = after_keyword.find(|c: char| c == '(' || c == '{' || c == '<' || c == ';' || c == ':')
        .unwrap_or(after_keyword.len());
    let name = after_keyword[..name_end].trim();

    if name.is_empty() || name == "(" || name == "{" {
        return None;
    }

    // Verificar que sea una definición (no una llamada)
    let before_keyword = &trimmed[..pos];
    let before_trimmed = before_keyword.trim();

    // Permitir: nada, pub, pub(crate), pub(super), async, pub async, unsafe, pub unsafe, default, const, extern
    let is_top_level = before_trimmed.is_empty()
        || before_trimmed == "pub"
        || before_trimmed == "pub(crate)"
        || before_trimmed == "pub(super)"
        || before_trimmed == "async"
        || before_trimmed == "pub async"
        || before_trimmed == "unsafe"
        || before_trimmed == "pub unsafe"
        || before_trimmed == "default"
        || before_trimmed == "const"
        || before_trimmed == "extern";

    // También permitir definiciones dentro de impl blocks (tienen indentación)
    let is_inside_impl = current_impl.is_some() && before_trimmed.is_empty();

    if !is_top_level && !is_inside_impl {
        return None;
    }

    // Construir el nombre completo con contexto
    let base_name = format!("{} {}", keyword.trim(), name);
    if let Some(ref impl_name) = current_impl {
        Some(format!("{}::{}", impl_name, base_name))
    } else {
        Some(base_name)
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

    // ── detect_duplicate_lines ──

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
    fn test_detect_duplicate_lines_arguments_ignored() {
        // Argumentos repetidos en macros (ej: call_id, call_id,) no deberían reportarse
        let content = "format!(\"{}{}\",\n            call_id,\n            call_id,\n        );";
        let warnings = detect_duplicate_lines(content);
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("call_id")));
    }

    // ── check_balanced_delimiters ──

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

    // ── detect_duplicate_definitions (con contexto impl) ──

    #[test]
    fn test_duplicate_defs_different_impls_not_duplicate() {
        // Dos structs con fn new() NO deben reportarse como duplicados
        let content = "\
pub struct Foo;

impl Foo {
    pub fn new() -> Self { Self }
}

pub struct Bar;

impl Bar {
    pub fn new() -> Self { Self }
}
";
        let warnings = detect_duplicate_definitions(content);
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("fn new")));
    }

    #[test]
    fn test_duplicate_defs_free_functions_are_duplicates() {
        // Dos funciones libres con el mismo nombre SÍ deben reportarse
        let content = "\
fn hello() { println!(\"a\"); }

fn hello() { println!(\"b\"); }
";
        let warnings = detect_duplicate_definitions(content);
        assert!(warnings.iter().any(|w| w.contains("fn hello")));
    }

    #[test]
    fn test_duplicate_defs_same_impl_duplicate() {
        // Dos métodos con el mismo nombre en el MISMO impl
        let content = "\
pub struct Foo;

impl Foo {
    pub fn bar() { }
    pub fn bar() { }
}
";
        let warnings = detect_duplicate_definitions(content);
        assert!(warnings.iter().any(|w| w.contains("Foo::fn bar")));
    }

    // ── detect_reasoning_injection ──

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

    // ── extract_impl_struct_name ──

    #[test]
    fn test_extract_impl_simple() {
        assert_eq!(extract_impl_struct_name("ToolResultStore {"), "ToolResultStore");
        assert_eq!(extract_impl_struct_name("SubAgentManager {"), "SubAgentManager");
        assert_eq!(extract_impl_struct_name("ProcessRegistry {"), "ProcessRegistry");
    }

    #[test]
    fn test_extract_impl_with_generics() {
        assert_eq!(extract_impl_struct_name("MyType<T> where T: Clone {"), "MyType");
        assert_eq!(extract_impl_struct_name("HashMap<K, V> {"), "HashMap");
    }

    #[test]
    fn test_extract_impl_trait_for_type() {
        assert_eq!(extract_impl_struct_name("Display for MyType {"), "MyType");
        assert_eq!(extract_impl_struct_name("Clone for MyStruct {"), "MyStruct");
    }
}
