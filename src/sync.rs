// ============================================================================
// sync.rs — Sincronización de Proyectos de Estudio entre Amigos
// ============================================================================
//
// Permite que múltiples estudiantes trabajen en el mismo proyecto de estudio
// con enseñanza personalizada para cada uno. Los cambios se sincronizan a través
// del servidor central.
//
// Protocolo:
//   1. El owner crea un proyecto de estudio (POST /api/study/projects)
//   2. Invita a miembros (POST /api/study/projects/:id/members)
//   3. Cada miembro tiene su propio perfil de aprendizaje
//   4. Los archivos de estudio se sincronizan via pull/push
//   5. El servidor mantiene el historial de versiones

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use sha2::{Sha256, Digest};

// ============================================================================
// Estructuras de Sincronización
// ============================================================================

/// Una versión de un archivo de estudio
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FileVersion {
    /// Hash SHA256 del contenido
    pub content_hash: String,
    /// Quién hizo el cambio
    pub author: String,
    /// Timestamp del cambio
    pub timestamp: u64,
    /// Mensaje descriptivo del cambio
    pub message: String,
    /// Contenido codificado en base64 (solo en el servidor)
    #[serde(default)]
    pub content_base64: Option<String>,
}

/// Manifiesto de sincronización: lo que el cliente envía/recibe
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SyncManifest {
    /// Archivos que el cliente tiene (path → hash local)
    pub client_files: HashMap<String, String>,
    /// Timestamp de la última sync
    pub last_sync: u64,
}

/// Respuesta de sincronización del servidor
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SyncResponse {
    /// Archivos que el cliente necesita descargar (cambiaron en el servidor)
    pub files_to_download: HashMap<String, FileVersion>,
    /// Archivos que el cliente necesita subir (cambiaron localmente)
    pub files_to_upload: Vec<String>,
    /// Archivos en conflicto (cambiaron en ambos lados)
    pub conflicts: Vec<SyncConflict>,
    /// Nuevo timestamp de sync
    pub sync_timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SyncConflict {
    pub path: String,
    pub server_version: FileVersion,
    pub client_hash: String,
}

// ============================================================================
// SyncStore — Almacén central de sincronización
// ============================================================================

#[derive(Clone)]
pub struct SyncStore {
    /// Ruta base para archivos de sync
    pub sync_dir: Arc<Mutex<PathBuf>>,
    /// Versiones de archivos: project_id → (path → versiones)
    pub versions: Arc<Mutex<HashMap<String, HashMap<String, Vec<FileVersion>>>>>,
}

impl SyncStore {
    pub fn new(base_dir: &PathBuf) -> Self {
        let sync_dir = base_dir.join("study_sync");
        let _ = fs::create_dir_all(&sync_dir);

        Self {
            sync_dir: Arc::new(Mutex::new(sync_dir)),
            versions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registra una nueva versión de un archivo.
    pub fn push_version(
        &self,
        project_id: &str,
        path: &str,
        content_base64: &str,
        author: &str,
        message: &str,
    ) -> Result<FileVersion, String> {
        let mut hasher = Sha256::new();
        hasher.update(content_base64.as_bytes());
        let content_hash = hex::encode(hasher.finalize());

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let version = FileVersion {
            content_hash: content_hash.clone(),
            author: author.to_string(),
            timestamp: now,
            message: message.to_string(),
            content_base64: Some(content_base64.to_string()),
        };

        let mut versions = self.versions.lock();
        let project_versions = versions
            .entry(project_id.to_string())
            .or_insert_with(HashMap::new);

        let file_versions = project_versions
            .entry(path.to_string())
            .or_insert_with(Vec::new);

        file_versions.push(version.clone());

        // Limitar a últimas 20 versiones por archivo
        if file_versions.len() > 20 {
            *file_versions = file_versions.split_off(file_versions.len() - 20);
        }

        Ok(version)
    }

    /// Obtiene la última versión de un archivo.
    pub fn get_latest_version(&self, project_id: &str, path: &str) -> Option<FileVersion> {
        let versions = self.versions.lock();
        versions
            .get(project_id)
            .and_then(|pv| pv.get(path))
            .and_then(|vs| vs.last())
            .cloned()
    }

    /// Procesa un manifiesto de sync y devuelve la respuesta.
    pub fn process_sync(
        &self,
        project_id: &str,
        manifest: &SyncManifest,
    ) -> Result<SyncResponse, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let versions = self.versions.lock();
        let project_versions = versions.get(project_id);

        let mut files_to_download = HashMap::new();
        let mut files_to_upload = Vec::new();
        let mut conflicts = Vec::new();

        // Si no hay versiones en el servidor, el cliente sube todo
        if project_versions.is_none() || project_versions.unwrap().is_empty() {
            return Ok(SyncResponse {
                files_to_download: HashMap::new(),
                files_to_upload: manifest.client_files.keys().cloned().collect(),
                conflicts: Vec::new(),
                sync_timestamp: now,
            });
        }

        let project_versions = project_versions.unwrap();

        // Archivos que el servidor tiene
        for (path, server_versions) in project_versions {
            let latest = match server_versions.last() {
                Some(v) => v,
                None => continue,
            };

            match manifest.client_files.get(path) {
                Some(client_hash) => {
                    if client_hash == &latest.content_hash {
                        // Ambos iguales, nada que hacer
                        continue;
                    }
                    // ¿Cambió en el servidor después de la última sync?
                    if latest.timestamp > manifest.last_sync {
                        conflicts.push(SyncConflict {
                            path: path.clone(),
                            server_version: latest.clone(),
                            client_hash: client_hash.clone(),
                        });
                    }
                }
                None => {
                    // El cliente no tiene este archivo
                    files_to_download.insert(path.clone(), latest.clone());
                }
            }
        }

        // Archivos que el cliente tiene pero el servidor no
        for (path, _hash) in &manifest.client_files {
            if !project_versions.contains_key(path) {
                files_to_upload.push(path.clone());
            }
        }

        Ok(SyncResponse {
            files_to_download,
            files_to_upload,
            conflicts,
            sync_timestamp: now,
        })
    }

    /// Resuelve un conflicto a favor del servidor.
    pub fn resolve_conflict_use_server(
        &self,
        project_id: &str,
        path: &str,
    ) -> Result<FileVersion, String> {
        self.get_latest_version(project_id, path)
            .ok_or_else(|| "Archivo no encontrado.".to_string())
    }

    /// Resuelve un conflicto a favor del cliente (sube la versión del cliente).
    pub fn resolve_conflict_use_client(
        &self,
        project_id: &str,
        path: &str,
        content_base64: &str,
        author: &str,
    ) -> Result<FileVersion, String> {
        self.push_version(project_id, path, content_base64, author, "Resolución de conflicto (cliente)")
    }

    /// Obtiene el historial completo de un archivo.
    pub fn get_file_history(&self, project_id: &str, path: &str) -> Vec<FileVersion> {
        self.versions
            .lock()
            .get(project_id)
            .and_then(|pv| pv.get(path))
            .cloned()
            .unwrap_or_default()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> SyncStore {
        let tmp = std::env::temp_dir().join("iaf_test_sync");
        let _ = std::fs::create_dir_all(&tmp);
        SyncStore::new(&tmp)
    }

    #[test]
    fn test_push_and_get_version() {
        let store = test_store();
        let v1 = store.push_version(
            "proj1", "main.py", "cHJpbnQoJ2hlbGxvJyk=", "alumno1", "Initial commit",
        ).unwrap();

        assert!(!v1.content_hash.is_empty());

        let latest = store.get_latest_version("proj1", "main.py").unwrap();
        assert_eq!(latest.content_hash, v1.content_hash);
    }

    #[test]
    fn test_sync_manifest_new_files() {
        let store = test_store();

        // Server has a file
        store.push_version("proj1", "readme.md", "I0hlbGxv", "alumno1", "Add readme").unwrap();

        // Client has different files
        let manifest = SyncManifest {
            client_files: vec![("main.py".to_string(), "abc123".to_string())]
                .into_iter().collect(),
            last_sync: 0,
        };

        let response = store.process_sync("proj1", &manifest).unwrap();
        assert!(response.files_to_download.contains_key("readme.md"));
        assert!(response.files_to_upload.contains(&"main.py".to_string()));
    }

    #[test]
    fn test_sync_no_changes() {
        let store = test_store();
        let v = store.push_version("proj1", "main.py", "cHJpbnQoJ2hlbGxvJyk=", "alumno1", "v1").unwrap();

        let manifest = SyncManifest {
            client_files: vec![("main.py".to_string(), v.content_hash.clone())]
                .into_iter().collect(),
            last_sync: v.timestamp + 1,
        };

        let response = store.process_sync("proj1", &manifest).unwrap();
        assert!(response.files_to_download.is_empty());
        assert!(response.files_to_upload.is_empty());
        assert!(response.conflicts.is_empty());
    }

    #[test]
    fn test_file_history() {
        let store = test_store();
        store.push_version("proj1", "main.py", "djE=", "user1", "v1").unwrap();
        store.push_version("proj1", "main.py", "djI=", "user2", "v2").unwrap();
        store.push_version("proj1", "main.py", "djM=", "user1", "v3").unwrap();

        let history = store.get_file_history("proj1", "main.py");
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].message, "v1");
        assert_eq!(history[2].message, "v3");
    }
}
