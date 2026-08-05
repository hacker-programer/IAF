// ============================================================================
// google_auth.rs — Autenticación OAuth2 para Google APIs
// ============================================================================
// Maneja el flujo OAuth2 completo:
//   1. Generar URL de autorización
//   2. Intercambiar código por tokens
//   3. Refrescar tokens expirados
//   4. Almacenar tokens por usuario
//
// Soportado:
//   - Google Drive (scope: https://www.googleapis.com/auth/drive)
//   - Gmail (scope: https://www.googleapis.com/auth/gmail.modify)
//   - Google Docs (scope: https://www.googleapis.com/auth/documents)
//   - Google Sheets (scope: https://www.googleapis.com/auth/spreadsheets)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ============================================================================
// Configuración de Google OAuth2
// ============================================================================

/// Credenciales de la aplicación Google Cloud
#[derive(Clone, Serialize, Deserialize)]
pub struct GoogleCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl Default for GoogleCredentials {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: "http://localhost:8080/api/google/callback".to_string(),
        }
    }
}

/// Token OAuth2 con refresh token para renovación
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GoogleToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64, // Unix timestamp
    pub token_type: String,
    pub scope: String,
}

/// Respuesta del endpoint de token de Google
#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

/// Estado OAuth2 pendiente (para verificar al recibir el callback)
#[derive(Clone, Serialize, Deserialize)]
pub struct OAuthState {
    pub state: String,
    pub username: String,
    pub created_at: u64,
}

// ============================================================================
// Google Auth Store
// ============================================================================

#[derive(Clone)]
pub struct GoogleAuthStore {
    pub credentials: Arc<Mutex<GoogleCredentials>>,
    pub tokens: Arc<Mutex<HashMap<String, GoogleToken>>>, // username -> token
    pub pending_states: Arc<Mutex<HashMap<String, OAuthState>>>, // state -> OAuthState
    pub base_path: PathBuf,
}

impl GoogleAuthStore {
    pub fn new(base_path: PathBuf) -> Self {
        let store = Self {
            credentials: Arc::new(Mutex::new(GoogleCredentials::default())),
            tokens: Arc::new(Mutex::new(HashMap::new())),
            pending_states: Arc::new(Mutex::new(HashMap::new())),
            base_path,
        };
        store.load_from_disk();
        store
    }

    /// Carga credenciales y tokens desde disco
    fn load_from_disk(&self) {
        let creds_path = self.base_path.join("google_credentials.json");
        if creds_path.exists() {
            if let Ok(data) = fs::read_to_string(&creds_path) {
                if let Ok(creds) = serde_json::from_str::<GoogleCredentials>(&data) {
                    *self.credentials.lock().unwrap() = creds;
                }
            }
        }

        let tokens_path = self.base_path.join("google_tokens.json");
        if tokens_path.exists() {
            if let Ok(data) = fs::read_to_string(&tokens_path) {
                if let Ok(tokens) = serde_json::from_str::<HashMap<String, GoogleToken>>(&data) {
                    *self.tokens.lock().unwrap() = tokens;
                }
            }
        }
    }

    /// Guarda credenciales y tokens a disco
    pub fn save_to_disk(&self) -> Result<(), String> {
        let _ = fs::create_dir_all(&self.base_path);

        let creds = self.credentials.lock().unwrap();
        let creds_path = self.base_path.join("google_credentials.json");
        fs::write(&creds_path, serde_json::to_string_pretty(&*creds).unwrap())
            .map_err(|e| format!("Error guardando credenciales Google: {}", e))?;

        let tokens = self.tokens.lock().unwrap();
        let tokens_path = self.base_path.join("google_tokens.json");
        fs::write(&tokens_path, serde_json::to_string_pretty(&*tokens).unwrap())
            .map_err(|e| format!("Error guardando tokens Google: {}", e))?;

        Ok(())
    }

    /// Configura las credenciales de Google Cloud
    pub fn set_credentials(&self, client_id: &str, client_secret: &str, redirect_uri: &str) -> Result<(), String> {
        let mut creds = self.credentials.lock().unwrap();
        creds.client_id = client_id.to_string();
        creds.client_secret = client_secret.to_string();
        creds.redirect_uri = redirect_uri.to_string();
        drop(creds);
        self.save_to_disk()
    }

    /// Verifica si hay credenciales configuradas
    pub fn has_credentials(&self) -> bool {
        let creds = self.credentials.lock().unwrap();
        !creds.client_id.is_empty() && !creds.client_secret.is_empty()
    }

    /// Genera la URL de autorización OAuth2
    pub fn generate_auth_url(&self, username: &str, scopes: &[&str]) -> Result<(String, String), String> {
        let creds = self.credentials.lock().unwrap();
        if creds.client_id.is_empty() {
            return Err("Credenciales Google no configuradas. Configuralas en el panel de administración.".into());
        }

        let state = format!("oauth_{}_{}", username, uuid::Uuid::new_v4());
        let scope_str = scopes.join(" ");

        let url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
             client_id={}&\
             redirect_uri={}&\
             response_type=code&\
             scope={}&\
             access_type=offline&\
             prompt=consent&\
             state={}",
            urlencoding(&creds.client_id),
            urlencoding(&creds.redirect_uri),
            urlencoding(&scope_str),
            urlencoding(&state)
        );

        let oauth_state = OAuthState {
            state: state.clone(),
            username: username.to_string(),
            created_at: now_secs(),
        };

        self.pending_states.lock().unwrap().insert(state.clone(), oauth_state);

        // Limpiar estados viejos (>10 min)
        self.clean_old_states();

        Ok((url, state))
    }

    /// Intercambia el código de autorización por tokens
    pub async fn exchange_code(&self, code: &str, state: &str) -> Result<GoogleToken, String> {
        // Verificar state
        let oauth_state = {
            let mut states = self.pending_states.lock().unwrap();
            states.remove(state).ok_or_else(|| "Estado OAuth inválido o expirado.".to_string())?
        };

        let creds = self.credentials.lock().unwrap().clone();

        let client = reqwest::Client::new();
        let resp = client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", creds.client_id.as_str()),
                ("client_secret", creds.client_secret.as_str()),
                ("code", code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", creds.redirect_uri.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("Error de red al intercambiar código: {}", e))?;

        let token_resp: GoogleTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando respuesta de token: {}", e))?;

        if let Some(err) = token_resp.error {
            return Err(format!(
                "Error de Google: {} - {}",
                err,
                token_resp.error_description.unwrap_or_default()
            ));
        }

        let expires_in = token_resp.expires_in.unwrap_or(3600);
        let token = GoogleToken {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token,
            expires_at: now_secs() + expires_in,
            token_type: token_resp.token_type.unwrap_or_else(|| "Bearer".to_string()),
            scope: token_resp.scope.unwrap_or_default(),
        };

        // Guardar token
        self.tokens.lock().unwrap().insert(oauth_state.username.clone(), token.clone());
        let _ = self.save_to_disk();

        Ok(token)
    }

    /// Obtiene un token de acceso válido para un usuario, refrescándolo si es necesario
    pub async fn get_valid_token(&self, username: &str) -> Result<String, String> {
        let token = {
            let tokens = self.tokens.lock().unwrap();
            tokens.get(username).cloned()
        };

        match token {
            Some(t) => {
                // Si aún es válido, devolverlo
                if t.expires_at > now_secs() + 60 {
                    return Ok(t.access_token);
                }
                // Si no, refrescar
                self.refresh_token(username).await
            }
            None => Err(format!(
                "El usuario '{}' no tiene vinculada su cuenta de Google. Usá /api/google/auth-url para vincularla.",
                username
            )),
        }
    }

    /// Refresca un token expirado
    async fn refresh_token(&self, username: &str) -> Result<String, String> {
        let token = {
            let tokens = self.tokens.lock().unwrap();
            tokens.get(username).cloned()
        };

        let token = token.ok_or_else(|| "No hay token para refrescar.".to_string())?;
        let refresh = token
            .refresh_token
            .clone()
            .ok_or_else(|| "No hay refresh token. Reautorizá la aplicación.".to_string())?;

        let creds = self.credentials.lock().unwrap().clone();

        let client = reqwest::Client::new();
        let resp = client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", creds.client_id.as_str()),
                ("client_secret", creds.client_secret.as_str()),
                ("refresh_token", refresh.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| format!("Error refrescando token: {}", e))?;

        let token_resp: GoogleTokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("Error parseando refresh: {}", e))?;

        if let Some(err) = token_resp.error {
            // Si falla el refresh, eliminar el token para forzar reautorización
            self.tokens.lock().unwrap().remove(username);
            let _ = self.save_to_disk();
            return Err(format!("Token revocado. Reautorizá: {} - {}", err, token_resp.error_description.unwrap_or_default()));
        }

        let expires_in = token_resp.expires_in.unwrap_or(3600);
        let new_token = GoogleToken {
            access_token: token_resp.access_token,
            refresh_token: Some(refresh), // Mantener el refresh token
            expires_at: now_secs() + expires_in,
            token_type: token_resp.token_type.unwrap_or_else(|| "Bearer".to_string()),
            scope: token_resp.scope.unwrap_or(token.scope),
        };

        self.tokens.lock().unwrap().insert(username.to_string(), new_token.clone());
        let _ = self.save_to_disk();

        Ok(new_token.access_token)
    }

    /// Revoca el token de un usuario
    pub fn revoke_token(&self, username: &str) {
        self.tokens.lock().unwrap().remove(username);
        let _ = self.save_to_disk();
    }

    /// Verifica si un usuario tiene token
    pub fn has_token(&self, username: &str) -> bool {
        self.tokens.lock().unwrap().contains_key(username)
    }

    fn clean_old_states(&self) {
        let now = now_secs();
        self.pending_states.lock().unwrap().retain(|_, s| now - s.created_at < 600);
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn urlencoding(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

// ============================================================================
// Scopes comunes
// ============================================================================

pub const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";
pub const DRIVE_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";
pub const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.modify";
pub const DOCS_SCOPE: &str = "https://www.googleapis.com/auth/documents";
pub const SHEETS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";

/// Todos los scopes combinados para acceso completo
pub const ALL_SCOPES: &[&str] = &[
    DRIVE_SCOPE,
    GMAIL_SCOPE,
    DOCS_SCOPE,
    SHEETS_SCOPE,
];
