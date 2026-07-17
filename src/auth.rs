// ============================================================================
// auth.rs — Sistema de Autenticación por Firma Ed25519 (Challenge-Response)
// ============================================================================
//
// Flujo de autenticación:
//   1. Cliente → POST /api/auth/challenge { "username": "Fa" }
//   2. Servidor → { "nonce": "<base64_32bytes>" } (guarda nonce en memoria, TTL 5 min)
//   3. Cliente firma el nonce con su clave privada Ed25519
//   4. Cliente → POST /api/auth/verify { "username": "Fa", "nonce": "...", "signature": "<base64_64bytes>" }
//   5. Servidor verifica firma con la clave pública almacenada → { "token": "<session_token>" }
//
// Solo el admin puede:
//   - Crear/eliminar usuarios
//   - Cambiar límites, permisos y clave pública de cualquier usuario
//
// Las cuentas solo las crea el admin. Nadie más puede.
// No hay contraseñas: solo firma criptográfica Ed25519.

use ed25519_dalek::{VerifyingKey, SigningKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

// ============================================================================
// Estructuras de Datos
// ============================================================================

/// Cuenta de usuario almacenada en users.json
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserAccount {
    /// Nombre de usuario único
    pub username: String,
    /// Clave pública Ed25519 en formato hex (64 caracteres)
    pub public_key: String,
    /// true = tiene acceso administrativo total
    pub is_admin: bool,
    /// Lista de permisos específicos (ej: ["read_code", "write_code", "manage_users"])
    pub permissions: Vec<String>,
    /// Límites de uso diario y restricciones
    pub limits: UserLimits,
    /// Timestamp de creación (epoch seconds)
    pub created_at: u64,
    /// Timestamp de último cambio de clave (epoch seconds)
    pub key_updated_at: u64,
}

/// Límites configurables por usuario
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserLimits {
    /// Máximo de tokens por día (None = ilimitado, solo admin)
    pub max_tokens_per_day: Option<u64>,
    /// Máximo de llamadas a API por día
    pub max_api_calls_per_day: Option<u64>,
    /// Herramientas permitidas (["*"] = todas)
    pub allowed_tools: Vec<String>,
    /// Máximo de sub-agentes concurrentes
    pub max_sub_agents: usize,
    /// Máximo de proyectos simultáneos
    pub max_projects: usize,
    /// ¿Puede forkear repositorios?
    pub can_fork_repos: bool,
    /// ¿Puede ejecutar comandos de PowerShell?
    pub can_execute_powershell: bool,
    /// ¿Puede modificar archivos (write_file)?
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
    /// Límites para admin: acceso total sin restricciones
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
    /// Ruta al archivo users.json
    file_path: Arc<Mutex<PathBuf>>,
    /// Caché en memoria de los usuarios
    users: Arc<Mutex<UsersFile>>,
}

impl UserStore {
    /// Carga el archivo users.json o crea uno vacío.
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

    /// Persiste el estado actual a users.json
    fn save(&self) -> Result<(), String> {
        let path = self.file_path.lock().map_err(|e| format!("Lock error: {}", e))?;
        let users = self.users.lock().map_err(|e| format!("Lock error: {}", e))?;
        let json = serde_json::to_string_pretty(&*users)
            .map_err(|e| format!("Error serializando: {}", e))?;
        fs::write(&*path, json).map_err(|e| format!("Error escribiendo archivo: {}", e))?;
        Ok(())
    }

    /// Busca un usuario por nombre
    pub fn find_user(&self, username: &str) -> Option<UserAccount> {
        let users = self.users.lock().ok()?;
        users.users.iter().find(|u| u.username == username).cloned()
    }

    /// Lista todos los usuarios (solo admin debe poder)
    pub fn list_users(&self) -> Vec<UserAccount> {
        self.users
            .lock()
            .map(|u| u.users.clone())
            .unwrap_or_default()
    }

    /// Crea un nuevo usuario. Retorna error si ya existe.
    /// SOLO el admin puede llamar a esto.
    pub fn create_user(
        &self,
        username: &str,
        public_key: &str,
        is_admin: bool,
        permissions: Vec<String>,
        limits: UserLimits,
    ) -> Result<UserAccount, String> {
        // Validar clave pública
        Self::validate_public_key_hex(public_key)?;

        let mut users = self.users.lock().map_err(|e| format!("Lock error: {}", e))?;

        if users.users.iter().any(|u| u.username == username) {
            return Err(format!("El usuario '{}' ya existe.", username));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let account = UserAccount {
            username: username.to_string(),
            public_key: public_key.to_string(),
            is_admin,
            permissions,
            limits,
            created_at: now,
            key_updated_at: now,
        };

        users.users.push(account.clone());
        drop(users);
        self.save()?;

        Ok(account)
    }

    /// Actualiza la clave pública de un usuario. SOLO admin.
    pub fn update_public_key(&self, username: &str, new_public_key: &str) -> Result<(), String> {
        Self::validate_public_key_hex(new_public_key)?;

        let mut users = self.users.lock().map_err(|e| format!("Lock error: {}", e))?;
        let user = users
            .users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;

        user.public_key = new_public_key.to_string();
        user.key_updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        drop(users);
        self.save()
    }

    /// Actualiza los límites de un usuario. SOLO admin.
    pub fn update_limits(&self, username: &str, limits: UserLimits) -> Result<(), String> {
        let mut users = self.users.lock().map_err(|e| format!("Lock error: {}", e))?;
        let user = users
            .users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;

        user.limits = limits;
        drop(users);
        self.save()
    }

    /// Actualiza los permisos de un usuario. SOLO admin.
    pub fn update_permissions(&self, username: &str, permissions: Vec<String>) -> Result<(), String> {
        let mut users = self.users.lock().map_err(|e| format!("Lock error: {}", e))?;
        let user = users
            .users
            .iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;

        user.permissions = permissions;
        drop(users);
        self.save()
    }

    /// Elimina un usuario. SOLO admin.
    pub fn delete_user(&self, username: &str) -> Result<(), String> {
        let mut users = self.users.lock().map_err(|e| format!("Lock error: {}", e))?;
        let idx = users
            .users
            .iter()
            .position(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;

        users.users.remove(idx);
        drop(users);
        self.save()
    }

    /// Verifica que un string sea una clave pública Ed25519 válida en hex.
    fn validate_public_key_hex(hex_key: &str) -> Result<(), String> {
        // Ed25519 public keys are 32 bytes → 64 hex chars
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

    /// Retorna true si el username tiene rol admin
    pub fn is_admin(&self, username: &str) -> bool {
        self.find_user(username)
            .map(|u| u.is_admin)
            .unwrap_or(false)
    }
}

// ============================================================================
// ChallengeStore — Nonces efímeros para challenge-response
// ============================================================================

/// Almacén de nonces temporales en memoria (NO persiste a disco).
/// Cada nonce expira automáticamente tras `ttl_secs`.
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

    /// Genera un nonce aleatorio para un username, lo guarda y lo retorna en base64.
    /// Si ya existía un challenge para ese usuario, lo reemplaza.
    pub fn generate_challenge(&self, username: &str) -> String {
        let mut nonce_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut nonce_bytes);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut challenges = self.challenges.lock().unwrap();
        // Reap expired entries while we're here
        challenges.retain(|_, entry| now - entry.created_at < self.ttl_secs);

        challenges.insert(
            username.to_string(),
            ChallengeEntry {
                nonce_bytes,
                created_at: now,
            },
        );

        BASE64.encode(nonce_bytes)
    }

    /// Verifica la firma de un nonce. Retorna true si es válida.
    /// Consume el nonce (lo elimina) tras la verificación para evitar replay attacks.
    pub fn verify_challenge(
        &self,
        username: &str,
        nonce_b64: &str,
        signature_b64: &str,
        public_key_hex: &str,
    ) -> Result<bool, String> {
        // 1. Decodificar clave pública
        let pk_bytes = hex::decode(public_key_hex)
            .map_err(|e| format!("Error decodificando clave pública: {}", e))?;
        let verifying_key = VerifyingKey::try_from(&pk_bytes[..])
            .map_err(|e| format!("Clave pública inválida: {}", e))?;

        // 2. Decodificar nonce
        let nonce_bytes = BASE64
            .decode(nonce_b64)
            .map_err(|e| format!("Error decodificando nonce: {}", e))?;

        if nonce_bytes.len() != 32 {
            return Err("Nonce debe ser 32 bytes.".into());
        }

        // 3. Verificar que el nonce coincide con el almacenado
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        {
            let mut challenges = self.challenges.lock().unwrap();
            // Reap expired
            challenges.retain(|_, entry| now - entry.created_at < self.ttl_secs);

            match challenges.get(username) {
                Some(entry) => {
                    if entry.nonce_bytes.as_slice() != nonce_bytes.as_slice() {
                        return Ok(false); // Nonce mismatch
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

        // 4. Decodificar firma
        let sig_bytes = BASE64
            .decode(signature_b64)
            .map_err(|e| format!("Error decodificando firma: {}", e))?;

        let signature = Signature::try_from(&sig_bytes[..])
            .map_err(|e| format!("Firma inválida: {}", e))?;

        // 5. Verificar firma contra el nonce
        let is_valid = verifying_key.verify(&nonce_bytes, &signature).is_ok();

        // 6. Consumir el nonce (eliminarlo) para prevenir replay
        {
            let mut challenges = self.challenges.lock().unwrap();
            challenges.remove(username);
        }

        Ok(is_valid)
    }

    /// Limpia challenges expirados
    pub fn reap(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut challenges = self.challenges.lock().unwrap();
        let before = challenges.len();
        challenges.retain(|_, entry| now - entry.created_at < self.ttl_secs);
        before - challenges.len()
    }
}

// ============================================================================
// Session Token Store — Tokens de sesión post-autenticación
// ============================================================================

/// Almacén de tokens de sesión (simples tokens aleatorios).
/// Tras autenticación exitosa, se genera un token que el cliente envía
/// en headers subsecuentes.
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
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Crea una sesión para un usuario y retorna el token.
    pub fn create_session(&self, username: &str) -> String {
        let token = format!("iaf_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut sessions = self.sessions.lock().unwrap();
        // Reap expired (24h TTL)
        sessions.retain(|_, entry| now - entry.created_at < 86400);

        sessions.insert(
            token.clone(),
            SessionEntry {
                username: username.to_string(),
                created_at: now,
            },
        );

        token
    }

    /// Valida un token y retorna el username asociado.
    pub fn validate_token(&self, token: &str) -> Option<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let sessions = self.sessions.lock().unwrap();
        sessions.get(token).and_then(|entry| {
            if now - entry.created_at < 86400 {
                Some(entry.username.clone())
            } else {
                None
            }
        })
    }

    /// Invalida un token (logout).
    pub fn revoke_token(&self, token: &str) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(token).is_some()
    }

    /// Limpia sesiones expiradas
    pub fn reap(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut sessions = self.sessions.lock().unwrap();
        let before = sessions.len();
        sessions.retain(|_, entry| now - entry.created_at < 86400);
        before - sessions.len()
    }
}

// ============================================================================
// Helpers para generar claves (CLI / setup inicial)
// ============================================================================

/// Genera un nuevo par de claves Ed25519.
/// Retorna (private_key_hex, public_key_hex).
pub fn generate_keypair() -> (String, String) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    (private_hex, public_hex)
}

/// Firma un mensaje (bytes) con una clave privada Ed25519 en hex.
/// Retorna la firma en base64.
pub fn sign_message(private_key_hex: &str, message: &[u8]) -> Result<String, String> {
    let sk_bytes = hex::decode(private_key_hex)
        .map_err(|e| format!("Error decodificando clave privada: {}", e))?;
    let signing_key = SigningKey::try_from(&sk_bytes[..])
        .map_err(|e| format!("Clave privada inválida: {}", e))?;

    let signature = signing_key.sign(message);
    Ok(BASE64.encode(signature.to_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair_and_sign_verify() {
        let (private_hex, public_hex) = generate_keypair();
        assert_eq!(private_hex.len(), 64); // 32 bytes seed in hex
        assert_eq!(public_hex.len(), 64);  // 32 bytes public key in hex

        // Sign a message
        let message = b"test challenge nonce";
        let signature_b64 = sign_message(&private_hex, message).unwrap();

        // Verify
        let pk_bytes = hex::decode(&public_hex).unwrap();
        let verifying_key = VerifyingKey::try_from(&pk_bytes[..]).unwrap();
        let sig_bytes = BASE64.decode(&signature_b64).unwrap();
        let signature = Signature::try_from(&sig_bytes[..]).unwrap();

        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_challenge_flow() {
        let (private_hex, public_hex) = generate_keypair();

        // Create a temp users file
        let tmp = std::env::temp_dir().join("iaf_test_auth");
        let _ = std::fs::create_dir_all(&tmp);
        let config_dir = tmp.join(".config");
        let _ = std::fs::create_dir_all(&config_dir);

        let store = UserStore::load(&config_dir);
        store.create_user("testuser", &public_hex, false, vec![], UserLimits::default()).unwrap();

        let challenge_store = ChallengeStore::new(300);

        // Step 1: Generate challenge
        let nonce_b64 = challenge_store.generate_challenge("testuser");
        assert_eq!(nonce_b64.len(), 44); // base64 of 32 bytes

        // Step 2: Sign the nonce
        let nonce_bytes = BASE64.decode(&nonce_b64).unwrap();
        let signature_b64 = sign_message(&private_hex, &nonce_bytes).unwrap();

        // Step 3: Verify
        let result = challenge_store.verify_challenge("testuser", &nonce_b64, &signature_b64, &public_hex).unwrap();
        assert!(result);

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_replay_attack_prevented() {
        let (private_hex, public_hex) = generate_keypair();

        let tmp = std::env::temp_dir().join("iaf_test_auth2");
        let _ = std::fs::create_dir_all(&tmp);
        let config_dir = tmp.join(".config");
        let _ = std::fs::create_dir_all(&config_dir);

        let store = UserStore::load(&config_dir);
        store.create_user("testuser2", &public_hex, false, vec![], UserLimits::default()).unwrap();

        let challenge_store = ChallengeStore::new(300);
        let nonce_b64 = challenge_store.generate_challenge("testuser2");

        let nonce_bytes = BASE64.decode(&nonce_b64).unwrap();
        let signature_b64 = sign_message(&private_hex, &nonce_bytes).unwrap();

        // First verification should succeed
        let result1 = challenge_store.verify_challenge("testuser2", &nonce_b64, &signature_b64, &public_hex).unwrap();
        assert!(result1);

        // Second verification with same nonce should fail (nonce consumed)
        let result2 = challenge_store.verify_challenge("testuser2", &nonce_b64, &signature_b64, &public_hex);
        assert!(result2.is_err() || !result2.unwrap());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
