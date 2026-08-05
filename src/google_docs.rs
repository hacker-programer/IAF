// ============================================================================
// google_docs.rs — API de Google Docs v1 + Google Sheets v4
// ============================================================================
// Soporta:
//   Google Docs:
//     - Leer documento completo
//     - Crear documento nuevo
//     - Insertar/Reemplazar/Eliminar texto con los 3 modos de edición
//   Google Sheets:
//     - Leer hoja completa
//     - Leer rango específico
//     - Escribir en rango
//     - Crear hoja nueva

use crate::google_auth::GoogleAuthStore;
use crate::file_editor::{EditMode, EditResult};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ============================================================================
// Google Docs
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoogleDocContent {
    pub document_id: String,
    pub title: String,
    pub full_text: String,
    /// Líneas del documento (split por \n)
    pub lines: Vec<String>,
    pub total_lines: usize,
}

pub struct GoogleDocsClient {
    auth: GoogleAuthStore,
}

impl GoogleDocsClient {
    pub fn new(auth: GoogleAuthStore) -> Self {
        Self { auth }
    }

    /// Lee el contenido completo de un Google Doc
    pub async fn read_document(
        &self,
        username: &str,
        document_id: &str,
    ) -> Result<GoogleDocContent, String> {
        let token = self.auth.get_valid_token(username).await?;

        let url = format!(
            "https://docs.googleapis.com/v1/documents/{}",
            document_id
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error leyendo documento: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando: {}", e))?;

        if let Some(err) = body["error"].as_object() {
            return Err(format!(
                "Error de Google Docs: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let title = body["title"].as_str().unwrap_or("Sin título").to_string();
        let full_text = extract_doc_text(&body);
        let lines: Vec<String> = full_text.lines().map(|s| s.to_string()).collect();
        let total_lines = lines.len();

        Ok(GoogleDocContent {
            document_id: document_id.to_string(),
            title,
            full_text,
            lines,
            total_lines,
        })
    }

    /// Crea un nuevo Google Doc
    pub async fn create_document(
        &self,
        username: &str,
        title: &str,
    ) -> Result<String, String> {
        let token = self.auth.get_valid_token(username).await?;

        let payload = json!({
            "title": title,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post("https://docs.googleapis.com/v1/documents")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Error creando documento: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando: {}", e))?;

        if let Some(err) = body["error"].as_object() {
            return Err(format!(
                "Error creando documento: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let doc_id = body["documentId"].as_str().unwrap_or("").to_string();
        Ok(doc_id)
    }

    /// Edita un Google Doc usando los 3 modos de edición
    /// - Adicion: inserta texto en una posición
    /// - Reemplazo: reemplaza texto en un rango de índices
    /// - Eliminacion: elimina texto en un rango de índices
    ///
    /// Nota: Google Docs usa índices de caracteres, no números de línea.
    /// Para simplificar, operamos sobre el contenido textual completo.
    pub async fn edit_document(
        &self,
        username: &str,
        document_id: &str,
        mode: EditMode,
        content: &str,
        start_line: usize,
        end_line: usize,
    ) -> Result<EditResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        // Leer documento actual
        let doc = self.read_document(username, document_id).await?;

        // Aplicar edición sobre las líneas
        let mut lines = doc.lines.clone();
        let total_lines = lines.len();
        let lines_before = total_lines;

        match mode {
            EditMode::Adicion => {
                let insert_pos = if start_line == 0 {
                    0
                } else if start_line >= total_lines {
                    total_lines
                } else {
                    start_line
                };

                let new_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                let mut result = Vec::with_capacity(total_lines + new_lines.len());

                for i in 0..insert_pos {
                    result.push(lines[i].clone());
                }
                result.extend(new_lines);
                for i in insert_pos..total_lines {
                    result.push(lines[i].clone());
                }
                lines = result;
            }
            EditMode::Reemplazo => {
                let start_idx = start_line.saturating_sub(1);
                let end_idx = end_line.min(total_lines);

                if start_idx > total_lines {
                    return Err(format!("Línea de inicio {} fuera de rango.", start_line));
                }

                let new_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                let mut result = Vec::with_capacity(
                    total_lines.saturating_sub(end_idx.saturating_sub(start_idx)) + new_lines.len(),
                );

                for i in 0..start_idx {
                    result.push(lines[i].clone());
                }
                result.extend(new_lines);
                for i in end_idx..total_lines {
                    result.push(lines[i].clone());
                }
                lines = result;
            }
            EditMode::Eliminacion => {
                if total_lines == 0 {
                    return Ok(EditResult {
                        success: true,
                        message: "Documento vacío.".to_string(),
                        lines_before: 0,
                        lines_after: 0,
                        preview: String::new(),
                    });
                }

                let start_idx = start_line.saturating_sub(1);
                let end_idx = end_line.min(total_lines);

                if start_idx > total_lines {
                    return Err(format!("Línea de inicio {} fuera de rango.", start_line));
                }

                let mut result = Vec::new();
                for i in 0..start_idx {
                    result.push(lines[i].clone());
                }
                for i in end_idx..total_lines {
                    result.push(lines[i].clone());
                }
                lines = result;
            }
        }

        let new_text = lines.join("\n");
        let lines_after = lines.len();
        let preview = lines.iter().take(20).cloned().collect::<Vec<_>>().join("\n");

        // Aplicar cambios al documento usando batchUpdate
        // Estrategia: eliminar todo el contenido y reinsertar
        let requests = build_replace_all_requests(&doc.full_text, &new_text);

        let payload = json!({
            "requests": requests,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&format!(
                "https://docs.googleapis.com/v1/documents/{}:batchUpdate",
                document_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Error aplicando edición: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando respuesta: {}", e))?;

        if let Some(err) = body["error"].as_object() {
            return Err(format!(
                "Error de Google Docs: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let message = match mode {
            EditMode::Adicion => format!(
                "✅ Adición en Google Doc: {} líneas insertadas. {} → {} líneas.",
                content.lines().count(), lines_before, lines_after
            ),
            EditMode::Reemplazo => format!(
                "✅ Reemplazo en Google Doc: líneas {}-{} reemplazadas. {} → {} líneas.",
                start_line, end_line, lines_before, lines_after
            ),
            EditMode::Eliminacion => format!(
                "✅ Eliminación en Google Doc: líneas {}-{} eliminadas. {} → {} líneas.",
                start_line, end_line, lines_before, lines_after
            ),
        };

        Ok(EditResult {
            success: true,
            message,
            lines_before,
            lines_after,
            preview,
        })
    }
}

// ============================================================================
// Google Sheets
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoogleSheetData {
    pub spreadsheet_id: String,
    pub title: String,
    pub sheets: Vec<SheetInfo>,
    pub values: Vec<Vec<String>>, // Valores de la hoja activa
    pub range: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SheetInfo {
    pub sheet_id: u64,
    pub title: String,
    pub row_count: usize,
    pub column_count: usize,
}

pub struct GoogleSheetsClient {
    auth: GoogleAuthStore,
}

impl GoogleSheetsClient {
    pub fn new(auth: GoogleAuthStore) -> Self {
        Self { auth }
    }

    /// Lee valores de una hoja de cálculo
    pub async fn read_sheet(
        &self,
        username: &str,
        spreadsheet_id: &str,
        range: Option<&str>,
    ) -> Result<GoogleSheetData, String> {
        let token = self.auth.get_valid_token(username).await?;
        let range_str = range.unwrap_or("A1:Z1000");

        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}?\
             ranges={}&includeGridData=false",
            spreadsheet_id, range_str
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error leyendo hoja: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando: {}", e))?;

        if let Some(err) = body["error"].as_object() {
            return Err(format!(
                "Error de Google Sheets: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let title = body["properties"]["title"].as_str().unwrap_or("Sin título").to_string();

        let sheets: Vec<SheetInfo> = body["sheets"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|s| SheetInfo {
                sheet_id: s["properties"]["sheetId"].as_u64().unwrap_or(0),
                title: s["properties"]["title"].as_str().unwrap_or("").to_string(),
                row_count: s["properties"]["gridProperties"]["rowCount"].as_u64().unwrap_or(0) as usize,
                column_count: s["properties"]["gridProperties"]["columnCount"].as_u64().unwrap_or(0) as usize,
            })
            .collect();

        // También obtener valores
        let values_url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            spreadsheet_id, range_str
        );

        let values_resp = client
            .get(&values_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error leyendo valores: {}", e))?;

        let values_body: serde_json::Value = values_resp
            .json()
            .await
            .map_err(|e| format!("Error parseando valores: {}", e))?;

        let values: Vec<Vec<String>> = values_body["values"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|cell| cell.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .collect();

        Ok(GoogleSheetData {
            spreadsheet_id: spreadsheet_id.to_string(),
            title,
            sheets,
            values,
            range: range_str.to_string(),
        })
    }

    /// Escribe valores en un rango de la hoja
    pub async fn write_sheet_range(
        &self,
        username: &str,
        spreadsheet_id: &str,
        range: &str,
        values: &[Vec<String>],
    ) -> Result<String, String> {
        let token = self.auth.get_valid_token(username).await?;

        let payload = json!({
            "range": range,
            "majorDimension": "ROWS",
            "values": values,
        });

        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?\
             valueInputOption=USER_ENTERED",
            spreadsheet_id, range
        );

        let client = reqwest::Client::new();
        let resp = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Error escribiendo en hoja: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando: {}", e))?;

        if let Some(err) = body["error"].as_object() {
            return Err(format!(
                "Error de Google Sheets: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let updated = body["updatedCells"].as_u64().unwrap_or(0);
        Ok(format!("{} celdas actualizadas en el rango '{}'.", updated, range))
    }

    /// Crea una nueva hoja de cálculo
    pub async fn create_spreadsheet(
        &self,
        username: &str,
        title: &str,
    ) -> Result<String, String> {
        let token = self.auth.get_valid_token(username).await?;

        let payload = json!({
            "properties": {
                "title": title,
            },
        });

        let client = reqwest::Client::new();
        let resp = client
            .post("https://sheets.googleapis.com/v4/spreadsheets")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Error creando hoja: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando: {}", e))?;

        if let Some(err) = body["error"].as_object() {
            return Err(format!(
                "Error creando hoja: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let id = body["spreadsheetId"].as_str().unwrap_or("").to_string();
        Ok(id)
    }

    /// Aplica los 3 modos de edición a una hoja (sobre filas)
    pub async fn edit_sheet_lines(
        &self,
        username: &str,
        spreadsheet_id: &str,
        sheet_name: &str,
        mode: EditMode,
        content: &str,
        start_line: usize,
        end_line: usize,
    ) -> Result<EditResult, String> {
        // Leer hoja actual
        let sheet_data = self.read_sheet(
            username,
            spreadsheet_id,
            Some(&format!("{}!A1:ZZ10000", sheet_name)),
        )
        .await?;

        let mut rows: Vec<Vec<String>> = sheet_data.values.clone();
        let total_rows = rows.len();
        let lines_before = total_rows;

        match mode {
            EditMode::Adicion => {
                let insert_pos = if start_line == 0 {
                    0
                } else if start_line >= total_rows {
                    total_rows
                } else {
                    start_line
                };

                let new_rows: Vec<Vec<String>> = content
                    .lines()
                    .map(|line| vec![line.to_string()])
                    .collect();

                let mut result = Vec::with_capacity(total_rows + new_rows.len());
                for i in 0..insert_pos {
                    result.push(rows[i].clone());
                }
                result.extend(new_rows);
                for i in insert_pos..total_rows {
                    result.push(rows[i].clone());
                }
                rows = result;
            }
            EditMode::Reemplazo => {
                let start_idx = start_line.saturating_sub(1);
                let end_idx = end_line.min(total_rows);

                let new_rows: Vec<Vec<String>> = content
                    .lines()
                    .map(|line| vec![line.to_string()])
                    .collect();

                let mut result = Vec::new();
                for i in 0..start_idx {
                    result.push(rows[i].clone());
                }
                result.extend(new_rows);
                for i in end_idx..total_rows {
                    result.push(rows[i].clone());
                }
                rows = result;
            }
            EditMode::Eliminacion => {
                if total_rows == 0 {
                    return Ok(EditResult {
                        success: true,
                        message: "Hoja vacía, nada que eliminar.".to_string(),
                        lines_before: 0,
                        lines_after: 0,
                        preview: String::new(),
                    });
                }

                let start_idx = start_line.saturating_sub(1);
                let end_idx = end_line.min(total_rows);

                let mut result = Vec::new();
                for i in 0..start_idx {
                    result.push(rows[i].clone());
                }
                for i in end_idx..total_rows {
                    result.push(rows[i].clone());
                }
                rows = result;
            }
        }

        let lines_after = rows.len();

        // Escribir de vuelta a la hoja
        let range = format!("{}!A1:ZZ{}", sheet_name, rows.len().max(1));
        self.write_sheet_range(username, spreadsheet_id, &range, &rows).await?;

        let preview = rows.iter().take(10)
            .map(|r| r.first().cloned().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(EditResult {
            success: true,
            message: format!("✅ Hoja '{}' editada (modo {:?}). {} → {} filas.", sheet_name, mode, lines_before, lines_after),
            lines_before,
            lines_after,
            preview,
        })
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Extrae todo el texto de un Google Doc (respuesta JSON de la API)
fn extract_doc_text(body: &serde_json::Value) -> String {
    let mut text = String::new();
    if let Some(content) = body["body"]["content"].as_array() {
        for element in content {
            if let Some(paragraph) = element.get("paragraph") {
                if let Some(elements) = paragraph["elements"].as_array() {
                    for el in elements {
                        if let Some(t) = el["textRun"]["content"].as_str() {
                            text.push_str(t);
                        }
                    }
                }
            }
            // También extraer de tablas
            if let Some(table) = element.get("table") {
                if let Some(rows) = table["tableRows"].as_array() {
                    for row in rows {
                        if let Some(cells) = row["tableCells"].as_array() {
                            for cell in cells {
                                if let Some(cell_content) = cell["content"].as_array() {
                                    for cc in cell_content {
                                        if let Some(paragraph) = cc.get("paragraph") {
                                            if let Some(elements) = paragraph["elements"].as_array() {
                                                for el in elements {
                                                    if let Some(t) = el["textRun"]["content"].as_str() {
                                                        text.push_str(t);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        text.push('\n');
                    }
                }
            }
        }
    }
    text
}

/// Construye requests para reemplazar todo el contenido de un Google Doc
fn build_replace_all_requests(old_text: &str, new_text: &str) -> Vec<serde_json::Value> {
    let end_index = old_text.chars().count() as u64 + 1;

    vec![
        // Insertar nuevo texto al final
        json!({
            "insertText": {
                "location": {
                    "index": end_index,
                },
                "text": new_text,
            }
        }),
        // Eliminar todo el texto antiguo
        json!({
            "deleteContentRange": {
                "range": {
                    "startIndex": 1,
                    "endIndex": end_index - 1,
                }
            }
        }),
    ]
}
