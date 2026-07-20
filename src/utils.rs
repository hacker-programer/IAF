/// Funciones utilitarias compartidas entre el binario y los tests de integración.

/// Sanitiza un string para usarlo como nombre de archivo seguro.
/// - Reemplaza caracteres no-ASCII y no alfanuméricos por `_`
/// - Trunca a 40 caracteres máximo
/// - Convierte espacios a `_`
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect::<String>()
        .trim()
        .replace(" ", "_")
        .chars()
        .take(40)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_ascii_plain() {
        assert_eq!(sanitize_filename("hello"), "hello");
    }

    #[test]
    fn test_sanitize_filename_spaces_to_underscores() {
        assert_eq!(sanitize_filename("hello world"), "hello_world");
    }

    #[test]
    fn test_sanitize_filename_special_chars() {
        assert_eq!(sanitize_filename("hello!@#world"), "hello___world");
    }

    #[test]
    fn test_sanitize_filename_non_ascii_replaced() {
        // Caracteres no-ASCII (acentos, ñ, emojis) deben reemplazarse por _
        let result = sanitize_filename("Análisis ♥ del código: ¿bug o feature?");
        // Todos los caracteres deben ser ASCII
        assert!(result.chars().all(|c| c.is_ascii()));
        // No debe contener caracteres Unicode
        assert!(!result.contains('á'));
        assert!(!result.contains('♥'));
        assert!(!result.contains('¿'));
        assert!(!result.contains('ó'));
    }

    #[test]
    fn test_sanitize_filename_truncate_40() {
        let long_name = "a".repeat(100);
        let result = sanitize_filename(&long_name);
        assert_eq!(result.len(), 40);
    }

    #[test]
    fn test_sanitize_filename_trim_spaces() {
        assert_eq!(sanitize_filename("  hello  "), "hello");
    }

    #[test]
    fn test_sanitize_filename_keep_hyphens() {
        assert_eq!(sanitize_filename("my-file"), "my-file");
    }

    #[test]
    fn test_sanitize_filename_keep_underscores() {
        assert_eq!(sanitize_filename("my_file"), "my_file");
    }

    #[test]
    fn test_sanitize_filename_empty() {
        assert_eq!(sanitize_filename(""), "");
    }

    #[test]
    fn test_sanitize_filename_only_special_chars() {
        assert_eq!(sanitize_filename("!!!@@@"), "______");
    }
}
