// ============================================================================
// file_editor.rs — Editor de archivos con 3 modos de edición
// ============================================================================
// Soporta:
//   - Adición (Adicion): Inserta líneas en una posición específica
//   - Reemplazo (Reemplazo): Reemplaza líneas en un rango [start, end]
//   - Eliminación (Eliminacion): Elimina líneas en un rango [start, end]
//
// Usado tanto para archivos locales como para archivos de Google Drive
// (descargados, editados y re-subidos).

use std::fs;
use std::path::Path;

/// Los tres modos de edición de archivos
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EditMode {
    /// Añade líneas después de `position` (0 = al principio, usize::MAX = al final)
    Adicion,
    /// Reemplaza las líneas en el rango [start_line, end_line] (inclusivo, indexado desde 1)
    Reemplazo,
    /// Elimina las líneas en el rango [start_line, end_line] (inclusivo, indexado desde 1)
    Eliminacion,
}

impl EditMode {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "adicion" | "add" | "insert" | "insercion" => Ok(EditMode::Adicion),
            "reemplazo" | "replace" | "remplazo" | "sustitucion" => Ok(EditMode::Reemplazo),
            "eliminacion" | "delete" | "remove" | "borrado" => Ok(EditMode::Eliminacion),
            other => Err(format!(
                "Modo de edición no reconocido: '{}'. Usá: adicion, reemplazo, eliminacion",
                other
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EditMode::Adicion => "adicion",
            EditMode::Reemplazo => "reemplazo",
            EditMode::Eliminacion => "eliminacion",
        }
    }
}

/// Resultado de una operación de edición
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EditResult {
    pub success: bool,
    pub message: String,
    /// Número de líneas antes de la edición
    pub lines_before: usize,
    /// Número de líneas después de la edición
    pub lines_after: usize,
    /// Vista previa del resultado (primeras 20 líneas)
    pub preview: String,
}

/// Edita un archivo local aplicando el modo especificado.
///
/// # Argumentos
/// - `path`: Ruta absoluta al archivo
/// - `mode`: Modo de edición (Adicion, Reemplazo, Eliminacion)
/// - `content`: Contenido nuevo (ignorado en modo Eliminacion)
/// - `start_line`: Línea de inicio (indexado desde 1). En Adicion, es la posición después de la cual insertar.
/// - `end_line`: Línea de fin (indexado desde 1, inclusiva). Solo usado en Reemplazo y Eliminacion.
pub fn edit_file(
    path: &str,
    mode: EditMode,
    content: &str,
    start_line: usize,
    end_line: usize,
) -> Result<EditResult, String> {
    let file_path = Path::new(path);

    // Leer el archivo existente
    let existing = if file_path.exists() {
        fs::read_to_string(file_path)
            .map_err(|e| format!("Error leyendo archivo '{}': {}", path, e))?
    } else {
        // Si no existe, crear uno vacío
        if mode == EditMode::Adicion {
            String::new()
        } else {
            return Err(format!(
                "El archivo '{}' no existe. No se puede aplicar el modo '{:?}'.",
                path, mode
            ));
        }
    };

    let lines: Vec<&str> = if existing.is_empty() {
        Vec::new()
    } else {
        existing.lines().collect()
    };

    let total_lines = lines.len();
    let lines_before = total_lines;

    let new_lines: Vec<String> = match mode {
        EditMode::Adicion => {
            // Insertar después de start_line (0 = principio)
            let insert_pos = if start_line == 0 {
                0 // al principio
            } else if start_line >= total_lines {
                total_lines // al final
            } else {
                start_line // después de la línea start_line
            };

            let mut result: Vec<String> = Vec::with_capacity(total_lines + content.lines().count() + 1);

            // Copiar líneas antes de la inserción
            for i in 0..insert_pos {
                result.push(lines[i].to_string());
            }

            // Insertar nuevo contenido
            for line in content.lines() {
                result.push(line.to_string());
            }

            // Copiar líneas después de la inserción
            for i in insert_pos..total_lines {
                result.push(lines[i].to_string());
            }

            result
        }
        EditMode::Reemplazo => {
            // Reemplazar líneas en el rango [start_line, end_line] (indexado desde 1)
            let start_idx = start_line.saturating_sub(1);
            let end_idx = end_line.min(total_lines);

            if start_idx > total_lines {
                return Err(format!(
                    "La línea de inicio {} está fuera de rango (archivo tiene {} líneas).",
                    start_line, total_lines
                ));
            }

            let mut result: Vec<String> = Vec::with_capacity(
                total_lines.saturating_sub(end_idx.saturating_sub(start_idx)) + content.lines().count() + 1,
            );

            // Copiar líneas antes del rango
            for i in 0..start_idx {
                result.push(lines[i].to_string());
            }

            // Insertar contenido nuevo
            for line in content.lines() {
                result.push(line.to_string());
            }

            // Copiar líneas después del rango
            for i in end_idx..total_lines {
                result.push(lines[i].to_string());
            }

            result
        }
        EditMode::Eliminacion => {
            // Eliminar líneas en el rango [start_line, end_line] (indexado desde 1)
            if total_lines == 0 {
                return Ok(EditResult {
                    success: true,
                    message: "Archivo vacío, nada que eliminar.".to_string(),
                    lines_before: 0,
                    lines_after: 0,
                    preview: String::new(),
                });
            }

            let start_idx = start_line.saturating_sub(1);
            let end_idx = end_line.min(total_lines);

            if start_idx > total_lines {
                return Err(format!(
                    "La línea de inicio {} está fuera de rango (archivo tiene {} líneas).",
                    start_line, total_lines
                ));
            }

            let mut result: Vec<String> = Vec::with_capacity(
                total_lines.saturating_sub(end_idx.saturating_sub(start_idx) + 1),
            );

            // Copiar líneas antes del rango
            for i in 0..start_idx {
                result.push(lines[i].to_string());
            }

            // Saltar el rango (no copiar)
            // Copiar líneas después del rango
            for i in end_idx..total_lines {
                result.push(lines[i].to_string());
            }

            result
        }
    };

    let new_content = new_lines.join("\n");
    let lines_after = new_lines.len();
    let preview = new_lines.iter()
        .take(20)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");

    // Escribir el archivo
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Error creando directorio padre: {}", e))?;
    }

    fs::write(file_path, &new_content)
        .map_err(|e| format!("Error escribiendo archivo '{}': {}", path, e))?;

    let message = match mode {
        EditMode::Adicion => format!(
            "✅ Adición exitosa: {} líneas insertadas después de la línea {}. {} → {} líneas totales.",
            content.lines().count(),
            if start_line == 0 { 0 } else { start_line.min(total_lines) },
            lines_before,
            lines_after
        ),
        EditMode::Reemplazo => format!(
            "✅ Reemplazo exitoso: líneas {}-{} reemplazadas con {} líneas nuevas. {} → {} líneas totales.",
            start_line,
            end_line.min(total_lines.max(1)),
            content.lines().count(),
            lines_before,
            lines_after
        ),
        EditMode::Eliminacion => {
            let deleted = if total_lines == 0 { 0 } else {
                end_idx_minus_start_idx_plus_one(start_line, end_line, total_lines)
            };
            format!(
                "✅ Eliminación exitosa: {} líneas eliminadas (rango {}-{}). {} → {} líneas totales.",
                deleted, start_line, end_line.min(total_lines.max(1)), lines_before, lines_after
            )
        }
    };

    Ok(EditResult {
        success: true,
        message,
        lines_before,
        lines_after,
        preview,
    })
}

/// Helper: calcula cuántas líneas se eliminaron
fn end_idx_minus_start_idx_plus_one(start_line: usize, end_line: usize, total_lines: usize) -> usize {
    let start_idx = start_line.saturating_sub(1);
    let end_idx = end_line.min(total_lines);
    if end_idx > start_idx {
        end_idx - start_idx
    } else {
        0
    }
}

/// Lee el contenido completo de un archivo (para preview antes de editar)
pub fn read_file_full(path: &str) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|e| format!("Error leyendo archivo '{}': {}", path, e))
}

/// Lee un rango de líneas de un archivo
pub fn read_file_range(
    path: &str,
    start_line: usize,
    end_line: usize,
) -> Result<String, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Error leyendo archivo '{}': {}", path, e))?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    let start_idx = start_line.saturating_sub(1);
    let end_idx = end_line.min(total);

    if start_idx > total {
        return Err(format!(
            "Línea de inicio {} fuera de rango (archivo tiene {} líneas).",
            start_line, total
        ));
    }

    Ok(lines[start_idx..end_idx].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn setup_test_file(dir: &std::path::Path, content: &str) -> String {
        let path = dir.join("test.txt");
        fs::write(&path, content).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_adicion_al_principio() {
        let dir = tempdir().unwrap();
        let path = setup_test_file(dir.path(), "linea1\nlinea2\nlinea3");
        let result = edit_file(&path, EditMode::Adicion, "NUEVA1\nNUEVA2", 0, 0).unwrap();
        assert!(result.success);
        assert_eq!(result.lines_before, 3);
        assert_eq!(result.lines_after, 5);
        let full = read_file_full(&path).unwrap();
        assert!(full.starts_with("NUEVA1\nNUEVA2\nlinea1"));
    }

    #[test]
    fn test_adicion_al_final() {
        let dir = tempdir().unwrap();
        let path = setup_test_file(dir.path(), "linea1\nlinea2\nlinea3");
        let result = edit_file(&path, EditMode::Adicion, "FINAL1\nFINAL2", 999, 0).unwrap();
        assert!(result.success);
        let full = read_file_full(&path).unwrap();
        assert!(full.ends_with("FINAL2"));
    }

    #[test]
    fn test_adicion_en_medio() {
        let dir = tempdir().unwrap();
        let path = setup_test_file(dir.path(), "A\nB\nC\nD\nE");
        let result = edit_file(&path, EditMode::Adicion, "X\nY", 2, 0).unwrap();
        assert!(result.success);
        let full = read_file_full(&path).unwrap();
        assert_eq!(full, "A\nB\nX\nY\nC\nD\nE");
    }

    #[test]
    fn test_reemplazo_rango() {
        let dir = tempdir().unwrap();
        let path = setup_test_file(dir.path(), "A\nB\nC\nD\nE");
        let result = edit_file(&path, EditMode::Reemplazo, "X\nY\nZ", 2, 4).unwrap();
        assert!(result.success);
        let full = read_file_full(&path).unwrap();
        assert_eq!(full, "A\nX\nY\nZ\nE");
    }

    #[test]
    fn test_eliminacion_rango() {
        let dir = tempdir().unwrap();
        let path = setup_test_file(dir.path(), "A\nB\nC\nD\nE");
        let result = edit_file(&path, EditMode::Eliminacion, "", 2, 4).unwrap();
        assert!(result.success);
        let full = read_file_full(&path).unwrap();
        assert_eq!(full, "A\nE");
    }

    #[test]
    fn test_eliminacion_todo() {
        let dir = tempdir().unwrap();
        let path = setup_test_file(dir.path(), "A\nB\nC");
        let result = edit_file(&path, EditMode::Eliminacion, "", 1, 3).unwrap();
        assert!(result.success);
        let full = read_file_full(&path).unwrap();
        assert!(full.is_empty());
    }

    #[test]
    fn test_archivo_nuevo_adicion() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nuevo.txt").to_string_lossy().to_string();
        let result = edit_file(&path, EditMode::Adicion, "Hola\nMundo", 0, 0).unwrap();
        assert!(result.success);
        let full = read_file_full(&path).unwrap();
        assert_eq!(full, "Hola\nMundo");
    }

    #[test]
    fn test_archivo_nuevo_no_adicion_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("no_existe.txt").to_string_lossy().to_string();
        let result = edit_file(&path, EditMode::Reemplazo, "x", 1, 1);
        assert!(result.is_err());
    }
}
