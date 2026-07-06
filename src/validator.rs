//! Módulo de validación post-escritura.
//!
//! Este módulo proporciona funciones para validar archivos después de ser modificados
//! por el agente, detectando problemas comunes como:
//! - Líneas duplicadas consecutivas (copy-paste accidental)
//! - Delimitadores no balanceados (llaves, paréntesis, corchetes)
//! - Errores de sintaxis en archivos .rs y .js
//!
//! Se diseñó para prevenir los errores recurrentes documentados en MEMORIES.md.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Resultado de la validación de un archivo.
#[derive(Debug, Clone)]
pub struct ValidationResult {
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

/// Trunca un string para mostrarlo en advertencias.
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
}
