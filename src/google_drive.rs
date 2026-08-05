// ============================================================================
// google_drive.rs — API de Google Drive v3
// ============================================================================
// Soporta:
//   - Listar archivos/carpetas
//   - Crear archivos/carpetas
//   - Renombrar archivos
//   - Descargar archivos (exportar Google Docs/Sheets a formatos editables)
//   - Subir archivos (crear/actualizar)
//   - Eliminar archivos (mover a papelera)
//   - Ver metadatos
//   - Búsqueda por nombre
//
// Draw.io: Los archivos .drawio se almacenan como archivos normales de Drive.
// Se pueden descargar, editar y re-subir como cualquier otro archivo.

use crate::google_auth::GoogleAuthStore;
use serde::{Deserialize, Serialize};
use serde_json::json;

// ============================================================================
// Tipos de datos
// ============================================================================

/// Representa un archivo/carpeta de Google Drive
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
    pub size: Option<String>,
    pub web_view_link: Option<String>,
    pub parents: Option<Vec<String>>,
    pub is_folder: bool,
    pub is_google_doc: bool,
    pub is_google_sheet: bool,
    pub is_drawio: bool,
}

/// Resultado de una operación en Drive
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriveResult {
    pub success: bool,
    pub message: String,
    pub file: Option<DriveFile>,
    pub files: Option<Vec<DriveFile>>,
    pub content: Option<String>,    // Para descargas
    pub download_path: Option<String>, // Ruta local donde se guardó
}

// ============================================================================
// Google Drive API Client
// ============================================================================

pub struct GoogleDriveClient {
    auth: GoogleAuthStore,
}

impl GoogleDriveClient {
    pub fn new(auth: GoogleAuthStore) -> Self {
        Self { auth }
    }

    /// Lista archivos en Google Drive
    /// - `query`: Búsqueda opcional (ej: "name contains 'proyecto'")
    /// - `parent_id`: Carpeta padre (None = raíz)
    /// - `max_results`: Máximo de resultados (default 100)
    pub async fn list_files(
        &self,
        username: &str,
        query: Option<&str>,
        parent_id: Option<&str>,
        max_results: Option<usize>,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;
        let max = max_results.unwrap_or(100).min(1000);

        let mut q_parts: Vec<String> = Vec::new();
        q_parts.push("trashed = false".to_string());

        if let Some(q) = query {
            q_parts.push(format!("({})", q));
        }

        if let Some(pid) = parent_id {
            q_parts.push(format!("'{}' in parents", pid));
        }

        let q_str = q_parts.join(" and ");

        let url = format!(
            "https://www.googleapis.com/drive/v3/files?\
             pageSize={}&\
             q={}&\
             fields=files(id,name,mimeType,createdTime,modifiedTime,size,webViewLink,parents)",
            max,
            urlencoding(&q_str)
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error listando archivos: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando respuesta: {}", e))?;

        let files: Vec<DriveFile> = body["files"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|f| parse_drive_file(f))
            .collect();

        Ok(DriveResult {
            success: true,
            message: format!("{} archivos encontrados.", files.len()),
            file: None,
            files: Some(files),
            content: None,
            download_path: None,
        })
    }

    /// Obtiene metadatos de un archivo específico
    pub async fn get_file_metadata(
        &self,
        username: &str,
        file_id: &str,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?\
             fields=id,name,mimeType,createdTime,modifiedTime,size,webViewLink,parents",
            file_id
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error obteniendo metadatos: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando respuesta: {}", e))?;

        let file = parse_drive_file(&body);

        Ok(DriveResult {
            success: true,
            message: format!("Metadatos de '{}'.", file.name),
            file: Some(file),
            files: None,
            content: None,
            download_path: None,
        })
    }

    /// Descarga un archivo de Google Drive
    /// Para Google Docs/Sheets, exporta a formatos editables:
    ///   - Google Docs → .docx o .txt
    ///   - Google Sheets → .xlsx o .csv
    ///   - Google Slides → .pptx
    ///   - Draw.io → .drawio (archivo XML, se descarga tal cual)
    pub async fn download_file(
        &self,
        username: &str,
        file_id: &str,
        save_path: &str,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        // Obtener metadatos primero para saber el tipo
        let meta = self.get_file_metadata(username, file_id).await?;
        let file = meta.file.as_ref().ok_or("No se pudieron obtener metadatos.")?;

        let (download_url, is_export) = match file.mime_type.as_str() {
            "application/vnd.google-apps.document" => {
                // Google Doc → exportar como DOCX
                ("https://www.googleapis.com/drive/v3/files/{}/export?mimeType=application/vnd.openxmlformats-officedocument.wordprocessingml.document", true)
            }
            "application/vnd.google-apps.spreadsheet" => {
                // Google Sheet → exportar como XLSX
                ("https://www.googleapis.com/drive/v3/files/{}/export?mimeType=application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", true)
            }
            "application/vnd.google-apps.presentation" => {
                // Google Slides → exportar como PPTX
                ("https://www.googleapis.com/drive/v3/files/{}/export?mimeType=application/vnd.openxmlformats-officedocument.presentationml.presentation", true)
            }
            _ => {
                // Archivo normal (incluyendo .drawio) → descargar directamente
                ("https://www.googleapis.com/drive/v3/files/{}?alt=media", false)
            }
        };

        let url = download_url.replace("{}", file_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error descargando: {}", e))?;

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Error leyendo respuesta: {}", e))?;

        // Determinar extensión
        let save_path = if is_export {
            if file.mime_type.contains("document") && !save_path.ends_with(".docx") {
                format!("{}.docx", save_path)
            } else if file.mime_type.contains("spreadsheet") && !save_path.ends_with(".xlsx") {
                format!("{}.xlsx", save_path)
            } else if file.mime_type.contains("presentation") && !save_path.ends_with(".pptx") {
                format!("{}.pptx", save_path)
            } else {
                save_path.to_string()
            }
        } else {
            // Para draw.io y otros, usar la extensión original
            save_path.to_string()
        };

        // Guardar en disco
        if let Some(parent) = std::path::Path::new(&save_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Error creando directorio: {}", e))?;
        }
        std::fs::write(&save_path, &bytes)
            .map_err(|e| format!("Error guardando archivo: {}", e))?;

        Ok(DriveResult {
            success: true,
            message: format!("Archivo descargado como '{}'.", save_path),
            file: Some(file.clone()),
            files: None,
            content: None,
            download_path: Some(save_path),
        })
    }

    /// Sube/Crea un archivo en Google Drive
    pub async fn upload_file(
        &self,
        username: &str,
        local_path: &str,
        drive_name: Option<&str>,
        parent_id: Option<&str>,
        mime_type: Option<&str>,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let path = std::path::Path::new(local_path);
        if !path.exists() {
            return Err(format!("El archivo local '{}' no existe.", local_path));
        }

        let name = drive_name.unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("archivo_sin_nombre")
        });

        let mime = mime_type.unwrap_or_else(|| {
            mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string()
                .as_str()
                .to_string()
        });

        let content = std::fs::read(path)
            .map_err(|e| format!("Error leyendo archivo local: {}", e))?;

        // Usar multipart upload para archivos pequeños (< 5MB)
        // Para archivos grandes usar resumable upload
        let boundary = format!("boundary_{}", uuid::Uuid::new_v4());

        let mut body = Vec::new();

        // Metadata part
        let mut metadata = json!({
            "name": name,
        });
        if let Some(pid) = parent_id {
            metadata["parents"] = json!([pid]);
        }

        let metadata_str = serde_json::to_string(&metadata).unwrap();
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
        body.extend_from_slice(metadata_str.as_bytes());
        body.extend_from_slice(b"\r\n");

        // File content part
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", mime).as_bytes());
        body.extend_from_slice(&content);
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let client = reqwest::Client::new();
        let resp = client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,name,mimeType,createdTime,modifiedTime,size,webViewLink,parents")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", format!("multipart/related; boundary={}", boundary))
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Error subiendo archivo: {}", e))?;

        let body_val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando respuesta: {}", e))?;

        // Verificar errores
        if let Some(err) = body_val["error"].as_object() {
            return Err(format!(
                "Error de Google Drive: {}",
                err.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Error desconocido")
            ));
        }

        let file = parse_drive_file(&body_val);

        Ok(DriveResult {
            success: true,
            message: format!("Archivo '{}' subido exitosamente (ID: {}).", file.name, file.id),
            file: Some(file),
            files: None,
            content: None,
            download_path: None,
        })
    }

    /// Crea una carpeta en Google Drive
    pub async fn create_folder(
        &self,
        username: &str,
        folder_name: &str,
        parent_id: Option<&str>,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let mut metadata = json!({
            "name": folder_name,
            "mimeType": "application/vnd.google-apps.folder",
        });
        if let Some(pid) = parent_id {
            metadata["parents"] = json!([pid]);
        }

        let client = reqwest::Client::new();
        let resp = client
            .post("https://www.googleapis.com/drive/v3/files?fields=id,name,mimeType,createdTime,webViewLink")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&metadata)
            .send()
            .await
            .map_err(|e| format!("Error creando carpeta: {}", e))?;

        let body_val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando respuesta: {}", e))?;

        if let Some(err) = body_val["error"].as_object() {
            return Err(format!(
                "Error creando carpeta: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let file = parse_drive_file(&body_val);

        Ok(DriveResult {
            success: true,
            message: format!("Carpeta '{}' creada (ID: {}).", file.name, file.id),
            file: Some(file),
            files: None,
            content: None,
            download_path: None,
        })
    }

    /// Renombra un archivo/carpeta
    pub async fn rename_file(
        &self,
        username: &str,
        file_id: &str,
        new_name: &str,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let metadata = json!({
            "name": new_name,
        });

        let client = reqwest::Client::new();
        let resp = client
            .patch(&format!(
                "https://www.googleapis.com/drive/v3/files/{}?fields=id,name,mimeType,modifiedTime",
                file_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&metadata)
            .send()
            .await
            .map_err(|e| format!("Error renombrando: {}", e))?;

        let body_val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando respuesta: {}", e))?;

        if let Some(err) = body_val["error"].as_object() {
            return Err(format!(
                "Error renombrando: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let file = parse_drive_file(&body_val);

        Ok(DriveResult {
            success: true,
            message: format!("Renombrado a '{}'.", file.name),
            file: Some(file),
            files: None,
            content: None,
            download_path: None,
        })
    }

    /// Mueve un archivo a la papelera (soft delete)
    pub async fn trash_file(
        &self,
        username: &str,
        file_id: &str,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let metadata = json!({ "trashed": true });

        let client = reqwest::Client::new();
        let resp = client
            .patch(&format!(
                "https://www.googleapis.com/drive/v3/files/{}?fields=id,name",
                file_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&metadata)
            .send()
            .await
            .map_err(|e| format!("Error eliminando: {}", e))?;

        let body_val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando: {}", e))?;

        if let Some(err) = body_val["error"].as_object() {
            return Err(format!(
                "Error eliminando: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        Ok(DriveResult {
            success: true,
            message: format!("Archivo '{}' movido a la papelera.", file_id),
            file: None,
            files: None,
            content: None,
            download_path: None,
        })
    }

    /// Elimina permanentemente un archivo
    pub async fn delete_permanently(
        &self,
        username: &str,
        file_id: &str,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let client = reqwest::Client::new();
        let resp = client
            .delete(&format!("https://www.googleapis.com/drive/v3/files/{}", file_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error eliminando permanentemente: {}", e))?;

        if resp.status().is_success() || resp.status().as_u16() == 204 {
            Ok(DriveResult {
                success: true,
                message: format!("Archivo '{}' eliminado permanentemente.", file_id),
                file: None,
                files: None,
                content: None,
                download_path: None,
            })
        } else {
            let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
            Err(format!(
                "Error eliminando: {}",
                body["error"]["message"].as_str().unwrap_or("Error desconocido")
            ))
        }
    }

    /// Lee el contenido textual de un archivo (para edición)
    /// Para Google Docs, exporta como texto plano
    /// Para Google Sheets, exporta como CSV
    /// Para archivos .drawio, lee el XML
    pub async fn read_file_content(
        &self,
        username: &str,
        file_id: &str,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let meta = self.get_file_metadata(username, file_id).await?;
        let file = meta.file.as_ref().ok_or("Metadatos no disponibles.")?;

        let (url, _) = match file.mime_type.as_str() {
            "application/vnd.google-apps.document" => {
                ("https://www.googleapis.com/drive/v3/files/{}/export?mimeType=text/plain", false)
            }
            "application/vnd.google-apps.spreadsheet" => {
                ("https://www.googleapis.com/drive/v3/files/{}/export?mimeType=text/csv", false)
            }
            _ => {
                ("https://www.googleapis.com/drive/v3/files/{}?alt=media", false)
            }
        };

        let url = url.replace("{}", file_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error leyendo contenido: {}", e))?;

        let content = resp
            .text()
            .await
            .map_err(|e| format!("Error leyendo respuesta: {}", e))?;

        Ok(DriveResult {
            success: true,
            message: format!("Contenido de '{}' leído ({} caracteres).", file.name, content.len()),
            file: Some(file.clone()),
            files: None,
            content: Some(content),
            download_path: None,
        })
    }

    /// Actualiza el contenido de un archivo existente (para edición)
    pub async fn update_file_content(
        &self,
        username: &str,
        file_id: &str,
        new_content: &str,
        mime_type: Option<&str>,
    ) -> Result<DriveResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let mime = mime_type.unwrap_or("text/plain");

        let client = reqwest::Client::new();
        let resp = client
            .patch(&format!(
                "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media&fields=id,name,mimeType,modifiedTime",
                file_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", mime)
            .body(new_content.to_string())
            .send()
            .await
            .map_err(|e| format!("Error actualizando contenido: {}", e))?;

        let body_val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando: {}", e))?;

        if let Some(err) = body_val["error"].as_object() {
            return Err(format!(
                "Error actualizando: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        let file = parse_drive_file(&body_val);

        Ok(DriveResult {
            success: true,
            message: format!("Archivo '{}' actualizado.", file.name),
            file: Some(file),
            files: None,
            content: None,
            download_path: None,
        })
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn parse_drive_file(f: &serde_json::Value) -> DriveFile {
    let mime = f["mimeType"].as_str().unwrap_or("unknown").to_string();
    let is_folder = mime == "application/vnd.google-apps.folder";
    let is_google_doc = mime == "application/vnd.google-apps.document";
    let is_google_sheet = mime == "application/vnd.google-apps.spreadsheet";
    let is_drawio = f["name"]
        .as_str()
        .map(|n| n.ends_with(".drawio") || n.ends_with(".xml"))
        .unwrap_or(false);

    DriveFile {
        id: f["id"].as_str().unwrap_or("").to_string(),
        name: f["name"].as_str().unwrap_or("sin_nombre").to_string(),
        mime_type: mime,
        created_time: f["createdTime"].as_str().map(|s| s.to_string()),
        modified_time: f["modifiedTime"].as_str().map(|s| s.to_string()),
        size: f["size"].as_str().map(|s| s.to_string()),
        web_view_link: f["webViewLink"].as_str().map(|s| s.to_string()),
        parents: f["parents"].as_array().map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        }),
        is_folder,
        is_google_doc,
        is_google_sheet,
        is_drawio,
    }
}

fn urlencoding(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}
