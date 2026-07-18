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
    /// true = tiene acceso administrativo total (implica el resto de permisos)
    pub is_admin: bool,
    /// Permite gestionar al resto de usuarios desde la interfaz web
    #[serde(default)]
    pub admin: bool,
    /// Permite acceder al modo programador
    #[serde(default)]
    pub modo_programador: bool,
    /// Permite acceder al modo estudio
    #[serde(default)]
    pub modo_estudio: bool,
    /// Permite editar el system prompt global
    #[serde(default)]
    pub editar_system_prompt_global: bool,
    /// Permite editar los system prompts locales de cada proyecto
    #[serde(default)]
    pub editar_system_prompt_local: bool,
    /// Lista de permisos específicos (herramientas, etc.)
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Límites de uso y restricciones
    #[serde(default)]
    pub limits: UserLimits,
    /// Timestamp de creación (epoch seconds)
    #[serde(default)]
    pub created_at: u64,
    /// Timestamp de último cambio de clave/contraseña (epoch seconds)
    #[serde(default)]
    pub key_updated_at: u64,
}

impl UserAccount {
    /// Verifica si el usuario tiene un permiso específico. Admin siempre tiene todo.
    pub fn has_permission(&self, perm: &str) -> bool {
        if self.is_admin || self.admin {
            return true;
        }
        self.permissions.iter().any(|p| p == perm || p == "*")
    }

    /// Verifica si tiene acceso al modo programador
    pub fn has_programming_access(&self) -> bool {
        self.is_admin || self.admin || self.modo_programador
    }

    /// Verifica si tiene acceso al modo estudio
    pub fn has_study_access(&self) -> bool {
        self.is_admin || self.admin || self.modo_estudio
    }

    /// Verifica si puede editar el system prompt global
    pub fn can_edit_global_prompt(&self) -> bool {
        self.is_admin || self.admin || self.editar_system_prompt_global
    }

    /// Verifica si puede editar system prompts locales
    pub fn can_edit_local_prompt(&self) -> bool {
        self.is_admin || self.admin || self.editar_system_prompt_local
    }
}

/// Horario semanal para activación de límites
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct WeeklySchedule {
    /// Días de la semana y sus franjas horarias activas
    /// Ejemplo: "lunes" -> [(9, 10), (16, 18)] = activo de 9-10 AM y 4-6 PM
    #[serde(default)]
    pub horarios: HashMap<String, Vec<(u32, u32)>>,
}

impl WeeklySchedule {
    /// Verifica si el usuario está activo en este momento según el horario
    pub fn is_active_now(&self) -> bool {
        if self.horarios.is_empty() {
            return true; // Sin horarios = siempre activo
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Convertir a fecha/hora local (aproximación UTC para simplificar)
        let secs_since_midnight = now % 86400;
        let current_hour = (secs_since_midnight / 3600) as u32;

        // Día de la semana (0 = domingo en UTC, mapeamos a nombres)
        let days_since_epoch = now / 86400;
        let day_of_week = (days_since_epoch + 4) % 7; // Ajuste: 0 = lunes
        let day_name = match day_of_week {
            0 => "lunes",
            1 => "martes",
            2 => "miercoles",
            3 => "jueves",
            4 => "viernes",
            5 => "sabado",
            6 => "domingo",
            _ => return true,
        };

        if let Some(ranges) = self.horarios.get(day_name) {
            for &(start, end) in ranges {
                if current_hour >= start && current_hour < end {
                    return true;
                }
            }
            false
        } else {
            // Si no hay horario para este día, no está activo
            false
        }
    }
}

/// Límites configurables por usuario
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserLimits {
    /// Si los límites están activados. false = sin restricciones (pero sin acceso si no está activado)
    #[serde(default = "default_activacion")]
    pub activacion: bool,
    /// Límite de peticiones por minuto (0 = ilimitado)
    #[serde(default)]
    pub peticiones_por_minuto: u32,
    /// Límite de peticiones por hora (0 = ilimitado)
    #[serde(default)]
    pub peticiones_por_hora: u32,
    /// Límite de iteraciones del agente (0 = ilimitado)
    #[serde(default)]
    pub limite_iteraciones: u32,
    /// Límite de tokens de entrada (0 = ilimitado)
    #[serde(default)]
    pub limite_tokens_entrada: u64,
    /// Límite de tokens de salida (0 = ilimitado)
    #[serde(default)]
    pub limite_tokens_salida: u64,
    /// Horarios de activación (días y horas)
    #[serde(default)]
    pub horarios: WeeklySchedule,
    /// Herramientas permitidas (vacío = todas si admin, ninguna si no)
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Máximo de sub-agentes paralelos
    #[serde(default = "default_max_sub_agents")]
    pub max_sub_agents: usize,
    /// Máximo de proyectos
    #[serde(default = "default_max_projects")]
    pub max_projects: usize,
    /// Puede forkear repositorios
    #[serde(default)]
    pub can_fork_repos: bool,
    /// Puede ejecutar PowerShell
    #[serde(default)]
    pub can_execute_powershell: bool,
    /// Puede escribir archivos
    #[serde(default)]
    pub can_write_files: bool,
}

fn default_activacion() -> bool { true }
fn default_max_sub_agents() -> usize { 1 }
fn default_max_projects() -> usize { 2 }

impl Default for UserLimits {
    fn default() -> Self {
        Self {
            activacion: true,
            peticiones_por_minuto: 10,
            peticiones_por_hora: 100,
            limite_iteraciones: 50,
            limite_tokens_entrada: 500_000,
            limite_tokens_salida: 200_000,
            horarios: WeeklySchedule::default(),
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
            activacion: true,
            peticiones_por_minuto: 0,
            peticiones_por_hora: 0,
            limite_iteraciones: 0,
            limite_tokens_entrada: 0,
            limite_tokens_salida: 0,
            horarios: WeeklySchedule::default(),
            allowed_tools: vec!["*".into()],
            max_sub_agents: 8,
            max_projects: usize::MAX,
            can_fork_repos: true,
            can_execute_powershell: true,
            can_write_files: true,
        }
    }

    /// Verifica si el usuario está activo ahora (según horarios y activación)
    pub fn is_active_now(&self) -> bool {
        if !self.activacion {
            return false;
        }
        self.horarios.is_active_now()
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
        modo_estudio: bool,
        modo_programador: bool,
        editar_global: bool,
        editar_local: bool,
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
            admin: is_admin,
            modo_programador: is_admin || modo_programador,
            modo_estudio: is_admin || modo_estudio,
            editar_system_prompt_global: is_admin || editar_global,
            editar_system_prompt_local: is_admin || editar_local,
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
            admin: true,
            modo_programador: true,
            modo_estudio: true,
            editar_system_prompt_global: true,
            editar_system_prompt_local: true,
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
            // Verificar límites de horario
            if !user.limits.is_active_now() && !user.is_admin {
                return Err("Tu cuenta no está activa en este horario.".into());
            }
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

    /// Actualiza accesos del usuario (modo programador, modo estudio, editar prompts)
    pub fn update_access(
        &self,
        username: &str,
        modo_estudio: bool,
        modo_programador: bool,
        editar_global: bool,
        editar_local: bool,
    ) -> Result<(), String> {
        let mut users = self.users.lock();
        let user = users.users.iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;
        if !user.is_admin {
            user.modo_estudio = modo_estudio;
            user.modo_programador = modo_programador;
            user.editar_system_prompt_global = editar_global;
            user.editar_system_prompt_local = editar_local;
        }
        drop(users);
        self.save()
    }

    /// Actualiza horarios de un usuario
    pub fn update_schedule(&self, username: &str, schedule: WeeklySchedule) -> Result<(), String> {
        let mut users = self.users.lock();
        let user = users.users.iter_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| format!("Usuario '{}' no encontrado.", username))?;
        user.limits.horarios = schedule;
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
        self.find_user(username).map(|u| u.is_admin || u.admin).unwrap_or(false)
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
            true, false, false, false,
        ).unwrap();

        assert_eq!(user.username, "alumno1");
        assert!(user.password_hash.is_some());
        assert!(user.public_key.is_none());
        assert!(user.has_study_access());
        assert!(!user.has_programming_access());
        assert!(!user.is_admin);

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
            "test", "corta", false, vec![], UserLimits::default(), false, false, false, false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("8 caracteres"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_permissions_model() {
        // Admin tiene todos los permisos
        let admin = UserAccount {
            username: "admin".into(),
            is_admin: true,
            admin: true,
            modo_programador: true,
            modo_estudio: true,
            editar_system_prompt_global: true,
            editar_system_prompt_local: true,
            ..Default::default()
        };
        assert!(admin.has_study_access());
        assert!(admin.has_programming_access());
        assert!(admin.can_edit_global_prompt());
        assert!(admin.can_edit_local_prompt());

        // Usuario normal con modo estudio solamente
        let user = UserAccount {
            username: "estudiante".into(),
            modo_estudio: true,
            modo_programador: false,
            editar_system_prompt_global: false,
            editar_system_prompt_local: false,
            ..Default::default()
        };
        assert!(user.has_study_access());
        assert!(!user.has_programming_access());
        assert!(!user.can_edit_global_prompt());
        assert!(!user.can_edit_local_prompt());
    }

    #[test]
    fn test_user_limits_defaults() {
        let limits = UserLimits::default();
        assert!(limits.activacion);
        assert_eq!(limits.peticiones_por_minuto, 10);
        assert_eq!(limits.peticiones_por_hora, 100);
        assert_eq!(limits.limite_iteraciones, 50);
        assert!(limits.is_active_now()); // Sin horarios = siempre activo

        let admin_limits = UserLimits::admin();
        assert!(admin_limits.activacion);
        assert_eq!(admin_limits.peticiones_por_minuto, 0); // ilimitado
        assert_eq!(admin_limits.max_sub_agents, 8);
    }

    #[test]
    fn test_weekly_schedule() {
        let mut schedule = WeeklySchedule::default();
        // Sin horarios = siempre activo
        assert!(schedule.is_active_now());

        // Agregar un horario que cubre todas las horas
        let mut horarios = HashMap::new();
        horarios.insert("lunes".to_string(), vec![(0, 24)]);
        horarios.insert("martes".to_string(), vec![(0, 24)]);
        horarios.insert("miercoles".to_string(), vec![(0, 24)]);
        horarios.insert("jueves".to_string(), vec![(0, 24)]);
        horarios.insert("viernes".to_string(), vec![(0, 24)]);
        horarios.insert("sabado".to_string(), vec![(0, 24)]);
        horarios.insert("domingo".to_string(), vec![(0, 24)]);
        schedule.horarios = horarios;
        assert!(schedule.is_active_now());
    }

    #[test]
    fn test_user_limits_inactive_when_disabled() {
        let mut limits = UserLimits::default();
        limits.activacion = false;
        assert!(!limits.is_active_now());
    }

    impl Default for UserAccount {
        fn default() -> Self {
            Self {
                username: String::new(),
                public_key: None,
                password_hash: None,
                is_admin: false,
                admin: false,
                modo_programador: false,
                modo_estudio: false,
                editar_system_prompt_global: false,
                editar_system_prompt_local: false,
                permissions: vec![],
                limits: UserLimits::default(),
                created_at: 0,
                key_updated_at: 0,
            }
        }
    }
}