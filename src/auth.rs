// ============================================================================
// auth.rs — Sistema de Autenticación Dual
// ============================================================================
//
// ADMIN:    Autenticación por firma Ed25519 (challenge-response).
//           POST /api/auth/challenge → firma nonce → POST /api/auth/verify
//           El admin NO tiene contraseña. Solo nonce criptográfico.
//
// USUARIOS: Username + contraseña (argon2id, memory-hard).
//           POST /api/auth/login { "username": "...", "password": "..." }
//           La contraseña se hashea con argon2id (OWASP recommended).
//
// Solo el admin puede: crear/eliminar usuarios, cambiar límites, permisos,
// claves públicas, y restablecer contraseñas.
//
// Las cuentas solo las crea el admin. Nadie más puede.

use ed25519_dalek::{VerifyingKey, SigningKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use argon2::{
    password_hash::{rand_core::OsRng as ArgonOsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use parking_lot::Mutex;

// ============================================================================
// Estructuras de Datos
// ============================================================================

/// Cuenta de usuario almacenada en users.json
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserAccount {
    /// Nombre de usuario único
    pub username: String,
    /// Clave pública Ed25519 en formato hex (64 caracteres). Solo para admin.
    #[serde(default)]
    pub public_key: Option<String>,
    /// Hash argon2id de la contraseña. Solo para usuarios normales.
    #[serde(default)]
    pub password_hash: Option<String>,
    /// true = tiene acceso administrativo total
    pub is_admin: bool,
    /// Lista de permisos específicos
    pub permissions: Vec<String>,
    /// Límites de uso diario y restricciones
    pub limits: UserLimits,
    /// ¿Tiene acceso al modo estudio?
    pub has_study_access: bool,
    /// ¿Tiene acceso al modo programar?
    pub has_programming_access: bool,
    /// Timestamp de creación (epoch seconds)
    pub created_at: u64,
    /// Timestamp de último cambio de clave/contraseña (epoch seconds)
    pub key_updated_at: u64,
}

/// Límites configurables por usuario
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserLimits {
    pub max_tokens_per_day: Option<u64>,
    pub max_api_calls_per_day: Option<u64>,
    pub allowed_tools: Vec<String>,
    pub max_sub_agents: usize,
    pub max_projects: usize,
    pub can_fork_repos: bool,
    pub can_execute_powershell: bool,
    pub can_write_files: bool,
}

impl Default for UserLimits {
    fn default() -> Self {
        Self {
            max_tokens_per_day: Some(100_000),
            max_api_calls_per_day: Some(500),
            allowed_tools: vec![
                "read_file".into(),
                "search_code".into(),
                "search_google".into(),
            ],
            max_sub_agents: 1,
            max_projects: 2,
            can_fork_repos: false,
            can_execute_powershell: false,
            can_write_files: false,
        }
    }
}

impl UserLimits {
    pub fn admin() -> Self {
        Self {
            max_tokens_per_day: None,
            max_api_calls_per_day: None,
            allowed_tools: vec!["*".into()],
            max_sub_agents: 8,
            max_projects: usize::MAX,
            can_fork_repos: true,
            can_execute_powershell: true,
            can_write_files: true,
        }
    }
}

/// Contenedor para users.json
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct UsersFile {
    pub users: Vec<UserAccount>,
}

// ============================================================================
// UserStore — Gestión persistente de usuarios
// ============================================================================

#[derive(Clone)]
pub struct UserStore {
    file_path: Arc<Mutex<PathBuf>>,
    users: Arc<Mutex<UsersFile>>,
}

impl UserStore {
    pub fn load(config_dir: &PathBuf) -> Self {
        let file_path = config_dir.join("users.json");
        let users = if file_path.exists() {
            match fs::read_to_string(&file_path) {
                Ok(content) => {
                    serde_json::from_str::<UsersFile>(&content)
                        .unwrap_or_else(|e| {
                            eprintln!("[auth] Error al parsear users.json: {}. Creando archivo vacío.", e);
                            UsersFile::default()
                        })
                }
                Err(e) => {
                    eprintln!("[auth] Error al leer users.json: {}. Creando archivo vacío.", e);
                    UsersFile::default()
                }
            }
        } else {
            UsersFile::default()
        };

        Self {
            file_path: Arc::new(Mutex::new(file_path)),
            users: Arc::new(Mutex::new(users)),
        }
    }

    fn save(&self) -> Result<(), String> {
        let path = self.file_path.lock().clone();
        let users = self.users.lock().clone();
        let json = serde_json::to_string_pretty(&users)
            .map_err(|e| format!("Error serializando: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Error escribiendo archivo: {}", e))?;
        Ok(())
    }

    pub fn find_user(&self, username: &str) -> Option<UserAccount> {
        let users = self.users.lock();
        users.users.iter().find(|u| u.username == username).cloned()
    }

    pub fn list_users(&self) -> Vec<UserAccount> {
        self.users.lock().users.clone()
    }

    /// Crea un usuario normal (con contraseña). SOLO admin.
    pub fn create_user_with_password(
        &self,
        username: &str,
        password: &str,
        is_admin: bool,
        permissions: Vec<String>,
        limits: UserLimits,
        has_study: bool,
        has_programming: bool,
    ) -> Result<UserAccount, String> {
        if password.len() < 8 {
            return Err("La contraseña debe tener al menos 8 caracteres.".into());
        }

        let hash = hash_password(password)?;

        let mut users = self.users.lock();
        if users.users.iter().any(|u| u.username == username) {
            return Err(format!("El usuario '{}' ya existe.", username));
        }

        let now = epoch_now();
        let account = UserAccount {
            username: username.to_string(),
            public_key: None,
            password_hash: Some(hash),
            is_admin,
            permissions,
            limits,
            has_study_access: has_study,
            has_programming_access: has_programming,
            created_at: now,
            key_updated_at: now,
        };

        users.users.push(account.clone());
        drop(users);
        self.save()?;
        Ok(account)
    }

    /// Crea un admin (con clave pública Ed25519). SOLO admin.
    pub fn create_admin(
        &self,
        username: &str,
        public_key: &str,
        permissions: Vec<String>,
        limits: UserLimits,
    ) -> Result<UserAccount, String> {
        Self::validate_public_key_hex(public_key)?;

        let mut users = self.users.lock();
        if users.users.iter().any(|u| u.username == username) {
            return Err(format!("El usuario '{}' ya existe.", username));
        }

        let now = epoch_now();
        let account = UserAccount {
            username: username.to_string(),
            public_key: Some(public_key.to_string()),
            password_hash: None,
            is_admin: true,
            permissions,
            limits,
            has_study_access: true,
            has_programming_access: true,
            created_at: now,
            key_updated_at: now,
        };

        users.users.push(account.clone());
        drop(users);
        self.save()?;
        Ok(account)
    }

    /// Verifica credenciales de usuario normal (username + password).
    /// Retorna Some(UserAccount) si es válido, None si no.
    pub fn verify_password(&self, username: &str, password: &str) -> Result<Option<UserAccount>, String> {
        let user = match self.find_user(username) {
            Some(u) => u,
            None => return Ok(None),
        };

        let hash_str = match &user.password_hash {
            Some(h) => h.clone(),
            None => return Err("Este usuario no tiene contraseña configurada (usa nonce).".into()),
        };

        let parsed_hash = PasswordHash::new(&hash_str)
            .map_err(|e| format!("Error interno: hash mal formado: {}", e))?;

        let valid = Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok();

        if valid {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Cambia la contraseña de un usuario. SOLO admin.
    pub fn change_password(&self, username: &str, new_password: &str) -> Result<(), String> {
        if new_password.len() < 8 {
            return Err("La contraseña debe tener al menos 8 caracteres.".into());
        }

        let hash = hash_password(new_password)?;
        let mut users = self.users.lock();
        let user = users.users.iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;

        user.password_hash = Some(hash);
        user.key_updated_at = epoch_now();
        drop(users);
        self.save()
    }

    /// Actualiza la clave pública Ed25519 de un admin. SOLO admin.
    pub fn update_public_key(&self, username: &str, new_public_key: &str) -> Result<(), String> {
        Self::validate_public_key_hex(new_public_key)?;

        let mut users = self.users.lock();
        let user = users.users.iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;

        user.public_key = Some(new_public_key.to_string());
        user.key_updated_at = epoch_now();
        drop(users);
        self.save()
    }

    pub fn update_limits(&self, username: &str, limits: UserLimits) -> Result<(), String> {
        let mut users = self.users.lock();
        let user = users.users.iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;
        user.limits = limits;
        drop(users);
        self.save()
    }

    pub fn update_permissions(&self, username: &str, permissions: Vec<String>) -> Result<(), String> {
        let mut users = self.users.lock();
        let user = users.users.iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;
        user.permissions = permissions;
        drop(users);
        self.save()
    }

    pub fn update_access(&self, username: &str, study: bool, programming: bool) -> Result<(), String> {
        let mut users = self.users.lock();
        let user = users.users.iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;
        user.has_study_access = study;
        user.has_programming_access = programming;
        drop(users);
        self.save()
    }

    pub fn delete_user(&self, username: &str) -> Result<(), String> {
        let mut users = self.users.lock();
        let idx = users.users.iter()
            .position(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;
        users.users.remove(idx);
        drop(users);
        self.save()
    }

    fn validate_public_key_hex(hex_key: &str) -> Result<(), String> {
        if hex_key.len() != 64 {
            return Err(format!(
                "Clave pública inválida: debe ser 64 caracteres hex (32 bytes). Tiene {}.",
                hex_key.len()
            ));
        }
        let bytes = hex::decode(hex_key)
            .map_err(|e| format!("Clave pública no es hex válido: {}", e))?;
        VerifyingKey::try_from(&bytes[..])
            .map_err(|e| format!("Clave pública no es una clave Ed25519 válida: {}", e))?;
        Ok(())
    }

    pub fn is_admin(&self, username: &str) -> bool {
        self.find_user(username).map(|u| u.is_admin).unwrap_or(false)
    }
}

// ============================================================================
// ChallengeStore — Nonces efímeros (solo admin)
// ============================================================================

#[derive(Clone)]
pub struct ChallengeStore {
    challenges: Arc<Mutex<HashMap<String, ChallengeEntry>>>,
    ttl_secs: u64,
}

struct ChallengeEntry {
    nonce_bytes: [u8; 32],
    created_at: u64,
}

impl ChallengeStore {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            challenges: Arc::new(Mutex::new(HashMap::new())),
            ttl_secs,
        }
    }

    pub fn generate_challenge(&self, username: &str) -> String {
        let mut nonce_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut nonce_bytes);

        let now = epoch_now();
        let mut challenges = self.challenges.lock();
        challenges.retain(|_, entry| now - entry.created_at < self.ttl_secs);

        challenges.insert(
            username.to_string(),
            ChallengeEntry { nonce_bytes, created_at: now },
        );

        BASE64.encode(nonce_bytes)
    }

    pub fn verify_challenge(
        &self,
        username: &str,
        nonce_b64: &str,
        signature_b64: &str,
        public_key_hex: &str,
    ) -> Result<bool, String> {
        let pk_bytes = hex::decode(public_key_hex)
            .map_err(|e| format!("Error decodificando clave pública: {}", e))?;
        let verifying_key = VerifyingKey::try_from(&pk_bytes[..])
            .map_err(|e| format!("Clave pública inválida: {}", e))?;

        let nonce_bytes = BASE64.decode(nonce_b64)
            .map_err(|e| format!("Error decodificando nonce: {}", e))?;

        if nonce_bytes.len() != 32 {
            return Err("Nonce debe ser 32 bytes.".into());
        }

        let now = epoch_now();
        {
            let mut challenges = self.challenges.lock();
            challenges.retain(|_, entry| now - entry.created_at < self.ttl_secs);

            match challenges.get(username) {
                Some(entry) => {
                    if entry.nonce_bytes.as_slice() != nonce_bytes.as_slice() {
                        return Ok(false);
                    }
                    if now - entry.created_at >= self.ttl_secs {
                        challenges.remove(username);
                        return Err("Challenge expirado. Solicita uno nuevo.".into());
                    }
                }
                None => {
                    return Err("No hay challenge activo para este usuario. Solicita uno primero.".into());
                }
            }
        }

        let sig_bytes = BASE64.decode(signature_b64)
            .map_err(|e| format!("Error decodificando firma: {}", e))?;
        let signature = Signature::try_from(&sig_bytes[..])
            .map_err(|e| format!("Firma inválida: {}", e))?;

        let is_valid = verifying_key.verify(&nonce_bytes, &signature).is_ok();

        {
            let mut challenges = self.challenges.lock();
            challenges.remove(username);
        }

        Ok(is_valid)
    }

    pub fn reap(&self) -> usize {
        let now = epoch_now();
        let mut challenges = self.challenges.lock();
        let before = challenges.len();
        challenges.retain(|_, entry| now - entry.created_at < self.ttl_secs);
        before - challenges.len()
    }
}

// ============================================================================
// Session Token Store
// ============================================================================

#[derive(Clone, Default)]
pub struct SessionStore {
    sessions: Arc<Mutex<HashMap<String, SessionEntry>>>,
}

pub struct SessionEntry {
    pub username: String,
    pub created_at: u64,
}

impl SessionStore {
    pub fn new() -> Self {
        Self { sessions: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn create_session(&self, username: &str) -> String {
        let token = format!("iaf_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let now = epoch_now();
        let mut sessions = self.sessions.lock();
        sessions.retain(|_, entry| now - entry.created_at < 86400);
        sessions.insert(token.clone(), SessionEntry {
            username: username.to_string(),
            created_at: now,
        });
        token
    }

    pub fn validate_token(&self, token: &str) -> Option<String> {
        let now = epoch_now();
        let sessions = self.sessions.lock();
        sessions.get(token).and_then(|entry| {
            if now - entry.created_at < 86400 {
                Some(entry.username.clone())
            } else {
                None
            }
        })
    }

    pub fn revoke_token(&self, token: &str) -> bool {
        self.sessions.lock().remove(token).is_some()
    }

    pub fn reap(&self) -> usize {
        let now = epoch_now();
        let mut sessions = self.sessions.lock();
        let before = sessions.len();
        sessions.retain(|_, entry| now - entry.created_at < 86400);
        before - sessions.len()
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn epoch_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

/// Hashea una contraseña con argon2id (memory-hard, OWASP recommended).
fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut ArgonOsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Error al hashear contraseña: {}", e))?;
    Ok(hash.to_string())
}

pub fn generate_keypair() -> (String, String) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    (hex::encode(signing_key.to_bytes()), hex::encode(verifying_key.to_bytes()))
}

pub fn sign_message(private_key_hex: &str, message: &[u8]) -> Result<String, String> {
    let sk_bytes = hex::decode(private_key_hex)
        .map_err(|e| format!("Error decodificando clave privada: {}", e))?;
    let signing_key = SigningKey::try_from(&sk_bytes[..])
        .map_err(|e| format!("Clave privada inválida: {}", e))?;
    let signature = signing_key.sign(message);
    Ok(BASE64.encode(signature.to_bytes()))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let hash = hash_password("mi_super_contraseña_123").unwrap();
        assert!(hash.starts_with("$argon2"));

        // Verify
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(Argon2::default().verify_password("mi_super_contraseña_123".as_bytes(), &parsed).is_ok());
        assert!(Argon2::default().verify_password("contraseña_mala".as_bytes(), &parsed).is_err());
    }

    #[test]
    fn test_create_user_with_password() {
        let tmp = std::env::temp_dir().join("iaf_test_pw");
        let _ = std::fs::create_dir_all(&tmp);
        let config_dir = tmp.join(".config");
        let _ = std::fs::create_dir_all(&config_dir);

        let store = UserStore::load(&config_dir);
        let user = store.create_user_with_password(
            "alumno1", "contraseña_segura_123", false,
            vec!["read_file".into()], UserLimits::default(),
            true, false,
        ).unwrap();

        assert_eq!(user.username, "alumno1");
        assert!(user.password_hash.is_some());
        assert!(user.public_key.is_none());
        assert!(user.has_study_access);
        assert!(!user.has_programming_access);

        // Verify login
        let verified = store.verify_password("alumno1", "contraseña_segura_123").unwrap();
        assert!(verified.is_some());

        // Bad password
        let bad = store.verify_password("alumno1", "mala").unwrap();
        assert!(bad.is_none());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_admin_nonce_flow() {
        let (private_hex, public_hex) = generate_keypair();

        let tmp = std::env::temp_dir().join("iaf_test_admin");
        let _ = std::fs::create_dir_all(&tmp);
        let config_dir = tmp.join(".config");
        let _ = std::fs::create_dir_all(&config_dir);

        let store = UserStore::load(&config_dir);
        store.create_admin("admin_test", &public_hex, vec!["*".into()], UserLimits::admin()).unwrap();

        let challenge_store = ChallengeStore::new(300);
        let nonce_b64 = challenge_store.generate_challenge("admin_test");
        let nonce_bytes = BASE64.decode(&nonce_b64).unwrap();
        let sig = sign_message(&private_hex, &nonce_bytes).unwrap();

        let result = challenge_store.verify_challenge("admin_test", &nonce_b64, &sig, &public_hex).unwrap();
        assert!(result);

        // Replay: should fail
        let replay = challenge_store.verify_challenge("admin_test", &nonce_b64, &sig, &public_hex);
        assert!(replay.is_err() || !replay.unwrap());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_session_token() {
        let store = SessionStore::new();
        let token = store.create_session("testuser");
        assert!(token.starts_with("iaf_"));

        let username = store.validate_token(&token);
        assert_eq!(username, Some("testuser".to_string()));

        store.revoke_token(&token);
        assert_eq!(store.validate_token(&token), None);
    }

    #[test]
    fn test_password_too_short() {
        let tmp = std::env::temp_dir().join("iaf_test_short");
        let _ = std::fs::create_dir_all(&tmp);
        let config_dir = tmp.join(".config");
        let _ = std::fs::create_dir_all(&config_dir);

        let store = UserStore::load(&config_dir);
        let result = store.create_user_with_password(
            "test", "corta", false, vec![], UserLimits::default(), false, false
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("8 caracteres"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
