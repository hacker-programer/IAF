// ============================================================================
// client_protocol.rs — Protocolo Cliente-Servidor
// ============================================================================
//
// El servidor central (este binario) gestiona:
//   - API keys (DeepSeek, Voyage, OpenRouter)
//   - Límites y permisos de usuarios
//   - Autenticación
//   - Almacenamiento de chats y proyectos
//
// El cliente (binario separado) ejecuta:
//   - Lectura/escritura de archivos
//   - Ejecución de comandos (PowerShell, cargo, git)
//   - Operaciones de sistema de archivos
//
// Protocolo:
//   Cliente ──POST /api/client/request──> Servidor
//     { "action": "read_file", "params": {...}, "token": "..." }
//   Servidor ──{ "status": "ok", "action": "read_file", "params": {...} }──> Cliente
//   Cliente ejecuta localmente
//   Cliente ──POST /api/client/response──> Servidor
//     { "request_id": "...", "result": {...} }
//
// El servidor NUNCA ejecuta comandos en nombre de usuarios normales.
// Solo el admin (puerto 80 o nonce verificado) ejecuta localmente.

use serde::{Deserialize, Serialize};

// ============================================================================
// Mensajes del Protocolo
// ============================================================================

/// Solicitud del servidor al cliente para ejecutar una acción
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClientRequest {
    /// ID único de la solicitud
    pub request_id: String,
    /// Acción a ejecutar
    pub action: ClientAction,
    /// Parámetros de la acción
    pub params: serde_json::Value,
    /// Timestamp
    pub timestamp: u64,
}

/// Respuesta del cliente al servidor con el resultado
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClientResponse {
    /// ID de la solicitud original
    pub request_id: String,
    /// Estado: "ok", "error", "timeout"
    pub status: String,
    /// Resultado (depende de la acción)
    pub result: serde_json::Value,
    /// Mensaje de error si status != "ok"
    pub error: Option<String>,
    /// Timestamp
    pub timestamp: u64,
}

/// Acciones que el cliente puede ejecutar
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClientAction {
    /// Leer un archivo del sistema de archivos local
    ReadFile,
    /// Escribir un archivo en el sistema de archivos local
    WriteFile,
    /// Ejecutar un comando de PowerShell
    ExecutePowerShell,
    /// Listar directorio
    ListDirectory,
    /// Verificar si un archivo existe
    FileExists,
    /// Obtener metadata de un archivo
    FileMetadata,
    /// Ejecutar git (add, commit, push)
    GitOperation,
    /// Ejecutar cargo (check, build, test)
    CargoOperation,
    /// Búsqueda de código local
    SearchCode,
}

// ============================================================================
// Payloads específicos por acción
// ============================================================================

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ReadFileParams {
    pub path: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WriteFileParams {
    pub path: String,
    pub content: String,
    pub commit_message: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExecutePowerShellParams {
    pub command: String,
    pub work_dir: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ListDirectoryParams {
    pub path: String,
    pub pattern: Option<String>,
    pub recursive: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SearchCodeParams {
    pub query: String,
    pub path: Option<String>,
    pub file_pattern: Option<String>,
}

// ============================================================================
// Registro de Clientes Conectados
// ============================================================================

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConnectedClient {
    /// ID único del cliente
    pub client_id: String,
    /// Username asociado
    pub username: String,
    /// Timestamp de conexión
    pub connected_at: u64,
    /// Último heartbeat
    pub last_heartbeat: u64,
    /// Hostname o IP
    pub host_info: String,
}

// ============================================================================
// Endpoints del protocolo
// ============================================================================

/// POST /api/client/connect — El cliente se registra
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConnectRequest {
    pub username: String,
    pub token: String,
    pub host_info: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConnectResponse {
    pub client_id: String,
    pub status: String,
    pub pending_requests: Vec<ClientRequest>,
}

/// POST /api/client/heartbeat — El cliente envía heartbeat
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct HeartbeatRequest {
    pub client_id: String,
    pub token: String,
}

/// POST /api/client/response — El cliente envía resultado
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ClientResponseWrapper {
    pub client_id: String,
    pub token: String,
    pub response: ClientResponse,
}

/// POST /api/client/poll — El cliente pregunta si hay trabajo pendiente
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PollRequest {
    pub client_id: String,
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PollResponse {
    pub pending_requests: Vec<ClientRequest>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_request_serialization() {
        let req = ClientRequest {
            request_id: "req_001".into(),
            action: ClientAction::ReadFile,
            params: serde_json::json!({"path": "/tmp/test.txt"}),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ClientRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.action, ClientAction::ReadFile);
        assert_eq!(deserialized.request_id, "req_001");
    }

    #[test]
    fn test_client_response_serialization() {
        let resp = ClientResponse {
            request_id: "req_001".into(),
            status: "ok".into(),
            result: serde_json::json!({"content": "hello world", "lines": 1}),
            error: None,
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: ClientResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.status, "ok");
        assert_eq!(deserialized.result["content"], "hello world");
    }
}
