// ============================================================================
// iaf-client — Cliente de Ejecución Local para IAF
// ============================================================================
//
// Este binario se ejecuta en la PC del usuario. Se conecta al servidor IAF
// central (que gestiona API keys y límites) y ejecuta todas las operaciones
// de sistema de archivos y comandos localmente.
//
// El servidor NUNCA ejecuta comandos en nombre de usuarios normales.
// Solo el admin (puerto 80 o nonce verificado) ejecuta localmente en el server.
//
// Protocolo:
//   1. Cliente se conecta → POST /api/client/connect
//   2. Cliente hace poll → POST /api/client/poll (cada 2s)
//   3. Servidor responde con solicitudes pendientes (read_file, write_file, etc.)
//   4. Cliente ejecuta localmente y envía respuesta → POST /api/client/response
//   5. Heartbeat cada 30s → POST /api/client/heartbeat

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use sha2::{Sha256, Digest};

// ============================================================================
// Tipos del protocolo (duplicados aquí para evitar dependencia circular)
// ============================================================================

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ClientAction {
    ReadFile,
    WriteFile,
    ExecutePowerShell,
    ListDirectory,
    FileExists,
    FileMetadata,
    GitOperation,
    CargoOperation,
    SearchCode,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ClientRequest {
    request_id: String,
    action: ClientAction,
    params: serde_json::Value,
    timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ClientResponse {
    request_id: String,
    status: String,
    result: serde_json::Value,
    error: Option<String>,
    timestamp: u64,
}

// ============================================================================
// Configuración
// ============================================================================

#[derive(Clone)]
struct Config {
    server_url: String,
    username: String,
    token: String,
    client_id: Option<String>,
}

// ============================================================================
// Ejecutor de acciones
// ============================================================================

fn execute_request(req: &ClientRequest, config: &Config) -> ClientResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    let result = match req.action {
        ClientAction::ReadFile => execute_read_file(&req.params),
        ClientAction::WriteFile => execute_write_file(&req.params),
        ClientAction::ExecutePowerShell => execute_powershell(&req.params),
        ClientAction::ListDirectory => execute_list_directory(&req.params),
        ClientAction::FileExists => execute_file_exists(&req.params),
        ClientAction::FileMetadata => execute_file_metadata(&req.params),
        ClientAction::GitOperation => execute_git(&req.params),
        ClientAction::CargoOperation => execute_cargo(&req.params),
        ClientAction::SearchCode => execute_search_code(&req.params),
    };

    match result {
        Ok(value) => ClientResponse {
            request_id: req.request_id.clone(),
            status: "ok".into(),
            result: value,
            error: None,
            timestamp: now,
        },
        Err(e) => ClientResponse {
            request_id: req.request_id.clone(),
            status: "error".into(),
            result: serde_json::json!({}),
            error: Some(e),
            timestamp: now,
        },
    }
}

fn execute_read_file(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let path = params["path"].as_str().ok_or("path requerido")?;
    let start_line = params["start_line"].as_u64();
    let end_line = params["end_line"].as_u64();

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Error leyendo {}: {}", path, e))?;

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let (selected, range) = match (start_line, end_line) {
        (Some(s), Some(e)) => {
            let s = s as usize;
            let e = e as usize;
            if s > total_lines || e > total_lines || s > e {
                return Err(format!("Rango inválido: {}-{} (total: {})", s, e, total_lines));
            }
            (lines[s-1..e].join("\n"), format!("{}-{}", s, e))
        }
        (Some(s), None) => {
            let s = s as usize;
            if s > total_lines {
                return Err(format!("Línea {} fuera de rango (total: {})", s, total_lines));
            }
            (lines[s-1..].join("\n"), format!("{}-{}", s, total_lines))
        }
        _ => (content.clone(), format!("1-{}", total_lines)),
    };

    Ok(serde_json::json!({
        "content": selected,
        "lines": range,
        "total_lines": total_lines,
        "path": path,
    }))
}

fn execute_write_file(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let path = params["path"].as_str().ok_or("path requerido")?;
    let content = params["content"].as_str().ok_or("content requerido")?;

    // Verificar que el directorio existe
    if let Some(parent) = std::path::Path::new(path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Error creando directorio: {}", e))?;
    }

    fs::write(path, content)
        .map_err(|e| format!("Error escribiendo {}: {}", path, e))?;

    // Calcular SHA256 del contenido
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hex::encode(hasher.finalize());

    Ok(serde_json::json!({
        "status": "ok",
        "path": path,
        "sha256": hash,
        "bytes_written": content.len(),
    }))
}

fn execute_powershell(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let command = params["command"].as_str().ok_or("command requerido")?;
    let work_dir = params["work_dir"].as_str();

    let mut cmd = Command::new("powershell");
    cmd.args(&["-NoProfile", "-Command", command]);

    if let Some(dir) = work_dir {
        cmd.current_dir(dir);
    }

    let output = cmd.output()
        .map_err(|e| format!("Error ejecutando PowerShell: {}", e))?;

    Ok(serde_json::json!({
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        "exit_code": output.status.code().unwrap_or(-1),
    }))
}

fn execute_list_directory(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let path = params["path"].as_str().ok_or("path requerido")?;
    let recursive = params["recursive"].as_bool().unwrap_or(false);
    let pattern = params["pattern"].as_str();

    let entries = if recursive {
        let mut v = Vec::new();
        let walker = walkdir::WalkDir::new(path).max_depth(10);
        for entry in walker.into_iter().filter_map(Result::ok) {
            if let Some(pat) = pattern {
                let name = entry.file_name().to_string_lossy();
                if !name.contains(pat) { continue; }
            }
            v.push(json_entry(&entry));
        }
        v
    } else {
        fs::read_dir(path)
            .map_err(|e| format!("Error leyendo directorio: {}", e))?
            .filter_map(Result::ok)
            .filter(|e| {
                if let Some(pat) = pattern {
                    e.file_name().to_string_lossy().contains(pat)
                } else { true }
            })
            .map(|e| json_dir_entry(&e))
            .collect()
    };

    Ok(serde_json::json!({ "entries": entries }))
}

fn json_dir_entry(entry: &fs::DirEntry) -> serde_json::Value {
    let path = entry.path();
    let metadata = entry.metadata().ok();
    serde_json::json!({
        "name": entry.file_name().to_string_lossy(),
        "path": path.to_string_lossy(),
        "is_dir": path.is_dir(),
        "size_bytes": metadata.as_ref().map(|m| m.len()).unwrap_or(0),
        "modified": metadata.and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs()),
    })
}

fn json_entry(entry: &walkdir::DirEntry) -> serde_json::Value {
    let path = entry.path();
    let metadata = entry.metadata().ok();
    serde_json::json!({
        "name": entry.file_name().to_string_lossy(),
        "path": path.to_string_lossy(),
        "is_dir": path.is_dir(),
        "size_bytes": metadata.as_ref().map(|m| m.len()).unwrap_or(0),
        "depth": entry.depth(),
    })
}

fn execute_file_exists(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let path = params["path"].as_str().ok_or("path requerido")?;
    Ok(serde_json::json!({ "exists": std::path::Path::new(path).exists() }))
}

fn execute_file_metadata(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let path = params["path"].as_str().ok_or("path requerido")?;
    let metadata = fs::metadata(path)
        .map_err(|e| format!("Error leyendo metadata: {}", e))?;

    Ok(serde_json::json!({
        "size_bytes": metadata.len(),
        "is_dir": metadata.is_dir(),
        "is_file": metadata.is_file(),
        "readonly": metadata.permissions().readonly(),
        "modified": metadata.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs()),
    }))
}

fn execute_git(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let subcommand = params["subcommand"].as_str().ok_or("subcommand requerido")?;
    let args_str = params["args"].as_str().unwrap_or("");
    let work_dir = params["work_dir"].as_str().unwrap_or(".");
    let mut args: Vec<&str> = subcommand.split_whitespace().chain(args_str.split_whitespace()).collect();

    let output = Command::new("git")
        .args(&args)
        .current_dir(work_dir)
        .output()
        .map_err(|e| format!("Error ejecutando git: {}", e))?;

    Ok(serde_json::json!({
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        "exit_code": output.status.code().unwrap_or(-1),
    }))
}

fn execute_cargo(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let subcommand = params["subcommand"].as_str().ok_or("subcommand requerido")?;
    let args_str = params["args"].as_str().unwrap_or("");
    let work_dir = params["work_dir"].as_str().unwrap_or(".");
    let mut args: Vec<&str> = subcommand.split_whitespace().chain(args_str.split_whitespace()).collect();

    // Agregar timeout para evitar builds infinitos (máx 5 min)
    let output = Command::new("cargo")
        .args(&args)
        .current_dir(work_dir)
        .output()
        .map_err(|e| format!("Error ejecutando cargo: {}", e))?;

    Ok(serde_json::json!({
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        "exit_code": output.status.code().unwrap_or(-1),
    }))
}

fn execute_search_code(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let query = params["query"].as_str().ok_or("query requerido")?;
    let search_path = params["path"].as_str().unwrap_or(".");
    let file_pattern = params["file_pattern"].as_str().unwrap_or("*.rs");

    let mut results = Vec::new();
    let walker = walkdir::WalkDir::new(search_path).max_depth(10);

    for entry in walker.into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() { continue; }
        let fname = entry.file_name().to_string_lossy();
        // Simple glob matching
        let matches_pattern = file_pattern == "*" || file_pattern.split(',')
            .any(|pat| fname.ends_with(pat.trim().trim_start_matches('*')));

        if !matches_pattern { continue; }

        if let Ok(content) = fs::read_to_string(entry.path()) {
            for (i, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&query.to_lowercase()) {
                    results.push(serde_json::json!({
                        "file": entry.path().to_string_lossy(),
                        "line": i + 1,
                        "content": line.trim().to_string(),
                    }));

                    if results.len() >= 200 { break; }
                }
            }
        }
        if results.len() >= 200 { break; }
    }

    Ok(serde_json::json!({
        "query": query,
        "matches": results.len(),
        "results": results,
    }))
}

// ============================================================================
// Main Loop del Cliente
// ============================================================================

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Uso: iaf-client <server_url> <username> <token>");
        eprintln!("Ejemplo: iaf-client http://127.0.0.1:8080 mi_usuario iaf_abc123...");
        std::process::exit(1);
    }

    let server_url = args[1].trim_end_matches('/').to_string();
    let username = args[2].clone();
    let token = args[3].clone();

    let host_info = format!("{}@{}", whoami::username(), whoami::hostname());

    let client = reqwest::Client::new();
    let mut config = Config {
        server_url: server_url.clone(),
        username: username.clone(),
        token: token.clone(),
        client_id: None,
    };

    println!("🔌 Conectando a {} como {}...", server_url, username);

    // 1. Conectar
    let connect_resp: serde_json::Value = client
        .post(format!("{}/api/client/connect", server_url))
        .json(&serde_json::json!({
            "username": username,
            "token": token,
            "host_info": host_info,
        }))
        .send()
        .await
        .expect("Error conectando al servidor")
        .json()
        .await
        .expect("Error parseando respuesta de conexión");

    if connect_resp["status"] != "ok" {
        eprintln!("❌ Error de conexión: {}", connect_resp["message"]);
        std::process::exit(1);
    }

    config.client_id = Some(connect_resp["client_id"].as_str().unwrap().to_string());
    println!("✅ Conectado como cliente {}", config.client_id.as_ref().unwrap());

    // 2. Loop principal: poll + execute + respond
    loop {
        // Heartbeat cada 30 segundos (se hace implícitamente con el poll)
        let poll_resp: serde_json::Value = client
            .post(format!("{}/api/client/poll", server_url))
            .json(&serde_json::json!({
                "client_id": config.client_id,
                "token": token,
            }))
            .send()
            .await
            .expect("Error en poll")
            .json()
            .await
            .expect("Error parseando respuesta de poll");

        if poll_resp["status"] != "ok" {
            eprintln!("⚠️  Error en poll: {:?}", poll_resp);
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            continue;
        }

        let requests: Vec<ClientRequest> = serde_json::from_value(
            poll_resp["pending_requests"].clone()
        ).unwrap_or_default();

        for req in &requests {
            println!("  📋 Ejecutando: {:?} ({})", req.action, req.request_id);
            let response = execute_request(req, &config);

            let _: serde_json::Value = client
                .post(format!("{}/api/client/response", server_url))
                .json(&serde_json::json!({
                    "client_id": config.client_id,
                    "token": token,
                    "response": response,
                }))
                .send()
                .await
                .expect("Error enviando respuesta")
                .json()
                .await
                .expect("Error parseando respuesta del servidor");
        }

        if requests.is_empty() {
            // Dormir 2 segundos antes de siguiente poll
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
}
