// ============================================================================
// google_gmail.rs — API de Gmail v1
// ============================================================================
// Soporta:
//   - Listar emails del inbox
//   - Leer email completo (headers + cuerpo)
//   - Enviar emails
//   - Buscar emails por query
//   - Mover a papelera/archivar

use crate::google_auth::GoogleAuthStore;
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};

// ============================================================================
// Tipos de datos
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GmailMessage {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub snippet: String,
    pub body_text: String,
    pub is_unread: bool,
    pub labels: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GmailResult {
    pub success: bool,
    pub message: String,
    pub messages: Option<Vec<GmailMessage>>,
    pub total_results: Option<usize>,
}

// ============================================================================
// Gmail API Client
// ============================================================================

pub struct GmailClient {
    auth: GoogleAuthStore,
}

impl GmailClient {
    pub fn new(auth: GoogleAuthStore) -> Self {
        Self { auth }
    }

    /// Lista emails del inbox
    pub async fn list_emails(
        &self,
        username: &str,
        query: Option<&str>,
        max_results: Option<usize>,
    ) -> Result<GmailResult, String> {
        let token = self.auth.get_valid_token(username).await?;
        let max = max_results.unwrap_or(20).min(500);

        let q = query.unwrap_or("in:inbox");
        let q_encoded = urlencoding(q);

        let url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages?\
             maxResults={}&q={}",
            max, q_encoded
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error listando emails: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando: {}", e))?;

        let message_list = body["messages"].as_array().cloned().unwrap_or_else(|| vec![]);
        let total = body["resultSizeEstimate"].as_u64().unwrap_or(0) as usize;

        let mut messages = Vec::new();
        for msg_ref in message_list.iter().take(max) {
            if let Some(msg_id) = msg_ref["id"].as_str() {
                match self.get_email_detail(username, msg_id).await {
                    Ok(msg) => messages.push(msg),
                    Err(e) => {
                        // Si falla uno, seguir con los demás
                        eprintln!("[Gmail] Error leyendo email {}: {}", msg_id, e);
                    }
                }
            }
        }

        Ok(GmailResult {
            success: true,
            message: format!("{} emails encontrados.", messages.len()),
            messages: Some(messages),
            total_results: Some(total),
        })
    }

    /// Obtiene el detalle completo de un email
    async fn get_email_detail(
        &self,
        username: &str,
        message_id: &str,
    ) -> Result<GmailMessage, String> {
        let token = self.auth.get_valid_token(username).await?;

        let url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}?\
             format=full&fields=id,threadId,labelIds,payload,snippet,internalDate",
            message_id
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error leyendo email: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando email: {}", e))?;

        let headers = body["payload"]["headers"].as_array().cloned().unwrap_or_else(|| vec![]);
        let mut subject = String::new();
        let mut from = String::new();
        let mut to = String::new();
        let mut date_str = String::new();

        for h in headers {
            let name = h["name"].as_str().unwrap_or("").to_lowercase();
            let value = h["value"].as_str().unwrap_or("");
            match name.as_str() {
                "subject" => subject = value.to_string(),
                "from" => from = value.to_string(),
                "to" => to = value.to_string(),
                "date" => date_str = value.to_string(),
                _ => {}
            }
        }

        // Extraer cuerpo del email
        let body_text = extract_email_body(&body["payload"]);

        let labels: Vec<String> = body["labelIds"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let is_unread = labels.contains(&"UNREAD".to_string());

        Ok(GmailMessage {
            id: message_id.to_string(),
            thread_id: body["threadId"].as_str().unwrap_or("").to_string(),
            subject,
            from,
            to,
            date: date_str,
            snippet: body["snippet"].as_str().unwrap_or("").to_string(),
            body_text,
            is_unread,
            labels,
        })
    }

    /// Envía un email
    pub async fn send_email(
        &self,
        username: &str,
        to: &str,
        subject: &str,
        body_text: &str,
    ) -> Result<GmailResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        // Construir email en formato RFC 2822 + base64url
        let email_raw = format!(
            "To: {}\r\nSubject: {}\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n{}",
            to, subject, body_text
        );

        let encoded = general_purpose::URL_SAFE.encode(email_raw.as_bytes());

        let payload = serde_json::json!({
            "raw": encoded,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post("https://gmail.googleapis.com/gmail/v1/users/me/messages/send")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Error enviando email: {}", e))?;

        let res_body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando respuesta: {}", e))?;

        if let Some(err) = res_body["error"].as_object() {
            return Err(format!(
                "Error de Gmail: {}",
                err.get("message").and_then(|m| m.as_str()).unwrap_or("Error desconocido")
            ));
        }

        Ok(GmailResult {
            success: true,
            message: format!("Email enviado a '{}'.", to),
            messages: None,
            total_results: None,
        })
    }

    /// Archiva un email (quita de inbox)
    pub async fn archive_email(
        &self,
        username: &str,
        message_id: &str,
    ) -> Result<GmailResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let payload = serde_json::json!({
            "removeLabelIds": ["INBOX"],
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&format!(
                "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/modify",
                message_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Error archivando: {}", e))?;

        if resp.status().is_success() {
            Ok(GmailResult {
                success: true,
                message: "Email archivado.".to_string(),
                messages: None,
                total_results: None,
            })
        } else {
            Err("Error archivando email.".to_string())
        }
    }

    /// Mueve un email a la papelera
    pub async fn trash_email(
        &self,
        username: &str,
        message_id: &str,
    ) -> Result<GmailResult, String> {
        let token = self.auth.get_valid_token(username).await?;

        let client = reqwest::Client::new();
        let resp = client
            .post(&format!(
                "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}/trash",
                message_id
            ))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Error moviendo a papelera: {}", e))?;

        if resp.status().is_success() {
            Ok(GmailResult {
                success: true,
                message: "Email movido a la papelera.".to_string(),
                messages: None,
                total_results: None,
            })
        } else {
            Err("Error moviendo email a papelera.".to_string())
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Extrae el cuerpo de texto de la estructura de payload de Gmail
fn extract_email_body(payload: &serde_json::Value) -> String {
    // Caso 1: body.data disponible directamente
    if let Some(data) = payload["body"]["data"].as_str() {
        if !data.is_empty() {
            return decode_base64url(data);
        }
    }

    // Caso 2: Buscar en parts
    if let Some(parts) = payload["parts"].as_array() {
        // Buscar text/plain primero
        for part in parts {
            if part["mimeType"].as_str() == Some("text/plain") {
                if let Some(data) = part["body"]["data"].as_str() {
                    if !data.is_empty() {
                        return decode_base64url(data);
                    }
                }
            }
        }
        // Si no hay text/plain, buscar text/html
        for part in parts {
            if part["mimeType"].as_str() == Some("text/html") {
                if let Some(data) = part["body"]["data"].as_str() {
                    if !data.is_empty() {
                        return format!("[HTML] {}", decode_base64url(data));
                    }
                }
            }
        }
        // Recursivo en parts anidadas
        for part in parts {
            let nested = extract_email_body(part);
            if !nested.is_empty() {
                return nested;
            }
        }
    }

    String::new()
}

fn decode_base64url(encoded: &str) -> String {
    // Reemplazar caracteres URL-safe por estándar
    let cleaned = encoded.replace('-', "+").replace('_', "/");
    general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .unwrap_or_else(|_| "[Error decodificando base64]".to_string())
}

fn urlencoding(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}
