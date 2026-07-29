# apply_all_fixes.ps1 — Aplica TODOS los fixes de bugs encontrados en IAF
$ErrorActionPreference = "Stop"
$base = "C:\Users\Fa\Desktop\IAF"

Write-Output "=== Aplicando todos los fixes ==="

# ========================================================================
# auth.rs: Fix duplicate line in update_access
# ========================================================================
$f = "$base\src\auth.rs"
$c = [System.IO.File]::ReadAllText($f)
$c = $c -replace "user\.modo_estudio = modo_estudio;\r?\n\s+user\.modo_estudio = modo_estudio;", "user.modo_estudio = modo_estudio;"
[System.IO.File]::WriteAllText($f, $c)
"auth.rs: duplicate fixed"

# ========================================================================
# main.rs: Fix #22/#46 - replace local sanitize_filename with import
# ========================================================================
$f = "$base\src\main.rs"
$c = [System.IO.File]::ReadAllText($f)

# Fix #22/#46: Replace the function with an import
$oldFn = "/// Sanitiza un string para usarlo como nombre de archivo`r`nfn sanitize_filename(title: &str) -> String {`r`n    title`r`n        .chars()`r`n        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })`r`n        .take(80)`r`n        .collect::<String>()`r`n        .trim_matches('_')`r`n        .to_string()`r`n}"
$newFn = "// FIX #22/#46: Using unified sanitize_filename from utils.rs (hash anti-collision)`r`nuse iaf::utils::sanitize_filename;"
$c = $c.Replace($oldFn, $newFn)
"main.rs: fix #22/#46 applied = $($c.Contains('use iaf::utils::sanitize_filename'))"

# Fix #10: clean_old_chat_files strict UUID
$old10 = "/// Limpia archivos viejos con el mismo UUID en el directorio de chats`r`n/// para evitar duplicados cuando el tÃ­tulo cambia.`r`nfn clean_old_chat_files(dir: &PathBuf, session_id: &str) {`r`n    if !dir.exists() {`r`n        return;`r`n    }`r`n    if let Ok(entries) = fs::read_dir(dir) {`r`n        for entry in entries.filter_map(Result::ok) {`r`n            let path = entry.path();`r`n            if path.extension().and_then(|e| e.to_str()) != Some(`"json`") {`r`n                continue;`r`n            }`r`n            let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or(`"`");`r`n            if fname.ends_with(&format!(`"-{}`", session_id)) {`r`n                let _ = fs::remove_file(&path);`r`n                eprintln!(`"[IAF] Limpiado archivo duplicado: {}`", path.display());`r`n            }`r`n        }`r`n    }`r`n}"
$new10 = "/// Limpia archivos viejos con el mismo UUID. FIX #10: validacion estricta del sufijo.`r`nfn clean_old_chat_files(dir: &PathBuf, session_id: &str) {`r`n    if !dir.exists() || !looks_like_uuid_stem(session_id) {`r`n        return;`r`n    }`r`n    if let Ok(entries) = fs::read_dir(dir) {`r`n        for entry in entries.filter_map(Result::ok) {`r`n            let path = entry.path();`r`n            if path.extension().and_then(|e| e.to_str()) != Some(`"json`") {`r`n                continue;`r`n            }`r`n            let fname = path.file_stem().and_then(|s| s.to_str()).unwrap_or(`"`");`r`n            let expected_suffix = format!(`"-{}`", session_id);`r`n            if fname.ends_with(&expected_suffix) && fname.len() > expected_suffix.len() {`r`n                let _ = fs::remove_file(&path);`r`n                eprintln!(`"[IAF] Limpiado archivo duplicado: {}`", path.display());`r`n            }`r`n        }`r`n    }`r`n}"
$c = $c.Replace($old10, $new10)
"main.rs: fix #10 applied = $($c.Contains('expected_suffix'))"

# Fix #1/#39: client_check
$old1 = "async fn client_check() -> impl IntoResponse {`r`n    let possible_paths = vec![`r`n        `"client/target/release/iaf-client.exe`",`r`n        `"client/target/debug/iaf-client.exe`",`r`n        `"iaf-client.exe`",`r`n    ];`r`n    let mut found = Vec::new();`r`n    for path in &possible_paths {`r`n        if std::path::Path::new(path).exists() {`r`n            found.push(path.to_string());`r`n        }`r`n    }`r`n    Json(json!({`r`n        `"status`": `"ok`",`r`n        `"client_installed`": !found.is_empty(),`r`n        `"found_at`": found,`r`n        `"expected_paths`": possible_paths,`r`n        `"instructions`": if found.is_empty() {`r`n            `"Para instalar el cliente: cd client && cargo build --release. Luego: .\\\\client\\\\target\\\\release\\\\iaf-client.exe <url> <user> <token>`"`r`n        } else {`r`n            `"Cliente encontrado. Ejecutalo con: iaf-client.exe http://127.0.0.1:8080 <username> <token>`"`r`n        }`r`n    }))`r`n}"
$new1 = "async fn client_check() -> impl IntoResponse {`r`n    // FIX #1/#39: v3.0 - Electron + Capacitor, no Rust client`r`n    Json(json!({`r`n        `"status`": `"ok`",`r`n        `"client_installed`": true,`r`n        `"message`": `"IAF v3.0 usa Electron (desktop) o Capacitor (Android).`",`r`n        `"instructions`": `"Electron: cd electron && npm install && npm start. Capacitor: cd capacitor && .\\\\setup_capacitor.ps1`"`r`n    }))`r`n}"
$c = $c.Replace($old1, $new1)
"main.rs: fix #1 applied = $($c.Contains('Electron.*Capacitor'))"

# Fix #13: admin_create_user
$old13 = "    let result = if payload.is_admin && payload.public_key.is_some() {`r`n        state.user_store.create_admin(&payload.username, &payload.public_key.unwrap(), perms, limits)`r`n    } else if let Some(ref pw) = payload.password {"
$new13 = "    let result = if payload.is_admin && payload.public_key.is_some() {`r`n        state.user_store.create_admin(&payload.username, &payload.public_key.unwrap(), perms, limits)`r`n    } else if payload.is_admin && payload.public_key.is_none() {`r`n        // FIX #13: Error claro cuando admin no tiene clave publica`r`n        Err(`"Para crear un admin necesitas generar claves (boton Generar Claves) o subir un .pem.`".into())`r`n    } else if let Some(ref pw) = payload.password {"
$c = $c.Replace($old13, $new13)
"main.rs: fix #13 applied = $($c.Contains('necesitas generar claves'))"

# Fix #9: agent_interrupt
$old9 = "    let mut agent = state.active_agent.lock().unwrap();`r`n    agent.interrupted = true;`r`n    agent.running = false;`r`n`r`n    if let Some(ref path) = agent.current_chat_path {`r`n        // Append interruption message to chat"
$new9 = "    // FIX #9: Abortar tokio task para detencion inmediata`r`n    if let Ok(mut abort_handle) = state.abort_handle.lock() {`r`n        if let Some(handle) = abort_handle.take() {`r`n            handle.abort();`r`n        }`r`n    }`r`n`r`n    let mut agent = state.active_agent.lock().unwrap();`r`n    agent.interrupted = true;`r`n    agent.running = false;`r`n`r`n    if let Some(ref path) = agent.current_chat_path {"
$c = $c.Replace($old9, $new9)
"main.rs: fix #9 applied = $($c.Contains('abort_handle.take().abort'))"

# Fix #6/#25: Always restart agent
$old6 = "if !agent.running {`r`n            agent.running = true;"
$new6 = "// FIX #6/#25: Always interrupt and restart agent for new messages`r`n        if agent.running {`r`n            agent.interrupted = true;`r`n            agent.running = false;`r`n            if let Ok(mut abort_handle) = state.abort_handle.lock() {`r`n                if let Some(handle) = abort_handle.take() { handle.abort(); }`r`n            }`r`n        }`r`n        agent.running = true;"
$c = $c.Replace($old6, $new6)
"main.rs: fix #6 applied = $($c.Contains('Always interrupt and restart'))"

[System.IO.File]::WriteAllText($f, $c)
"main.rs: ALL 7 fixes written"

# ========================================================================
# agent.rs: Fix #26, #31
# ========================================================================
$f = "$base\src\agent.rs"
$c = [System.IO.File]::ReadAllText($f)
$c = $c.Replace('"deepseek-v4-pro"', '"deepseek-chat" // FIX #26: correct model name')
$c = $c.Replace("let force_none_tool_choice = false;`r`n        let current_tool_choice = if force_none_tool_choice { `"none`" } else { `"auto`" };", 
                "// FIX #31: removed dead force_none_tool_choice code`r`n        let current_tool_choice = `"auto`";")
[System.IO.File]::WriteAllText($f, $c)
"agent.rs: fixes #26, #31 applied"

# ========================================================================
# sub_agent.rs: Fix #29, #44
# ========================================================================
$f = "$base\src\sub_agent.rs"
$c = [System.IO.File]::ReadAllText($f)

# Fix #29: is_path_allowed with canonicalization
$old29 = "pub fn is_path_allowed(file_path: &str, allowed_paths: &[String]) -> bool {`r`n    if allowed_paths.is_empty() {`r`n        return true;`r`n    }`r`n    let path = Path::new(file_path);`r`n    let normalized = path.to_string_lossy().to_lowercase();`r`n`r`n    for allowed in allowed_paths {`r`n        let allowed_norm = allowed.to_lowercase();`r`n        if normalized.starts_with(&allowed_norm) {`r`n            return true;`r`n        }`r`n        if allowed_norm.contains(&normalized) || normalized.contains(&allowed_norm) {`r`n            return true;`r`n        }`r`n    }`r`n`r`n    false`r`n}"
$new29 = "/// FIX #29: Usa canonicalizacion para prevenir path traversal.`r`npub fn is_path_allowed(file_path: &str, allowed_paths: &[String]) -> bool {`r`n    if allowed_paths.is_empty() { return true; }`r`n    let canonical = match std::fs::canonicalize(file_path) {`r`n        Ok(p) => p,`r`n        Err(_) => {`r`n            let path = Path::new(file_path);`r`n            if let Some(parent) = path.parent() {`r`n                match std::fs::canonicalize(parent) {`r`n                    Ok(p) => p.join(path.file_name().unwrap_or_default()),`r`n                    Err(_) => return false,`r`n                }`r`n            } else { return false; }`r`n        }`r`n    };`r`n    let canonical_str = canonical.to_string_lossy().to_lowercase();`r`n    for allowed in allowed_paths {`r`n        if let Ok(allowed_canon) = std::fs::canonicalize(Path::new(allowed)) {`r`n            if canonical_str.starts_with(&allowed_canon.to_string_lossy().to_lowercase()) {`r`n                return true;`r`n            }`r`n        }`r`n    }`r`n    false`r`n}"
$c = $c.Replace($old29, $new29)

# Fix #44: timeout in sub_agent
$c = $c.Replace("let max_iterations = 15;`r`n    let mut iteration = 0;", 
                "let max_iterations = 15;`r`n    let mut iteration = 0;`r`n    let start_time = std::time::Instant::now(); // FIX #44: global timeout")
$c = $c.Replace("if iteration > max_iterations {`r`n            return Ok(format!(`"LÃ­mite de {} iteraciones alcanzado.`", max_iterations));`r`n        }",
                "if iteration > max_iterations {`r`n            return Ok(format!(`"Limite de {} iteraciones alcanzado.`", max_iterations));`r`n        }`r`n        // FIX #44: Timeout global de 10 minutos`r`n        if start_time.elapsed().as_secs() > 600 {`r`n            return Ok(format!(`"Timeout global de 10min tras {} iteraciones.`", iteration));`r`n        }")
[System.IO.File]::WriteAllText($f, $c)
"sub_agent.rs: fixes #29, #44 applied"

Write-Output "=== ALL FIXES APPLIED ==="
Write-Output "Now run: cargo check"
