/// Funciones utilitarias compartidas entre el binario y los tests de integración.
use std::hash::{Hash, Hasher};

/// Sanitiza un string para usarlo como nombre de archivo seguro.
/// - Reemplaza caracteres no-ASCII ni alfanuméricos por `_`
/// - Trunca a 70 caracteres + 8-char hash para evitar colisiones
/// - Convierte espacios a `_`
pub fn sanitize_filename(name: &str) -> String {
    let base: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect::<String>()
        .trim()
        .replace(" ", "_")
        .chars()
        .take(70)
        .collect();
    let base = base.trim_matches('_').to_string();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = format!("{:08x}", hasher.finish());
    if base.is_empty() { hash } else { format!("{}_{}", base, hash) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_ascii_plain() {
        let result = sanitize_filename("hello");
        assert!(result.starts_with("hello_"));
    }

    #[test]
    fn test_sanitize_filename_spaces_to_underscores() {
        let result = sanitize_filename("hello world");
        assert!(result.starts_with("hello_world_"));
    }

    #[test]
    fn test_sanitize_filename_special_chars() {
        let result = sanitize_filename("hello!@#world");
        assert!(result.starts_with("hello___world_"));
    }

    #[test]
    fn test_sanitize_filename_non_ascii_replaced() {
        let result = sanitize_filename("Análisis ♥ del código: ¿bug o feature?");
        assert!(result.chars().all(|c| c.is_ascii()));
        assert!(!result.contains('á'));
        assert!(!result.contains('♥'));
    }

    #[test]
    fn test_sanitize_filename_max_length() {
        let long_name = "a".repeat(200);
        let result = sanitize_filename(&long_name);
        assert!(result.len() <= 79); // 70 chars + '_' + 8 hex = 79 max
    }

    #[test]
    fn test_sanitize_filename_trim_spaces() {
        let result = sanitize_filename("  hello  ");
        assert!(result.starts_with("hello_"));
    }

    #[test]
    fn test_sanitize_filename_keep_hyphens() {
        let result = sanitize_filename("my-file");
        assert!(result.starts_with("my-file_"));
    }

    #[test]
    fn test_sanitize_filename_hash_present() {
        let result = sanitize_filename("test");
        // Debe terminar con _XXXXXXXX (8 hex)
        let parts: Vec<&str> = result.split('_').collect();
        assert!(parts.len() >= 2);
        let hash_part = parts.last().unwrap();
        assert_eq!(hash_part.len(), 8);
        assert!(hash_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sanitize_filename_collision_resistant() {
        // Dos strings que difieren después del char 70 deben producir nombres distintos
        let a = format!("{}_suffixA", "x".repeat(70));
        let b = format!("{}_suffixB", "x".repeat(70));
        let ra = sanitize_filename(&a);
        let rb = sanitize_filename(&b);
        assert_ne!(ra, rb);
    }
}
