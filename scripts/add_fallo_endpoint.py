import sys

with open('src/main.rs', 'r', encoding='utf-8-sig') as f:
    content = f.read()

# 1. Agregar struct y handler antes de build_app
struct_and_handler = '''
// ============================================================================
// Endpoint de Reporte de Fallos (Usuarios)
// ============================================================================

#[derive(Deserialize)]
struct ReportarFalloRequest {
    informe: String,
    severidad: Option<String>,
}

async fn reportar_fallo_usuario(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReportarFalloRequest>,
) -> impl IntoResponse {
    let _username = match require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(e) => return (e.0, Json(json!({ "status": "error", "message": e.1 }))).into_response(),
    };

    if payload.informe.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "status": "error",
            "message": "El campo 'informe' es obligatorio y no puede estar vacio."
        }))).into_response();
    }

    let severidad = payload.severidad.unwrap_or_else(|| "media".to_string());
    let severidad_validada = match severidad.as_str() {
        "baja" | "media" | "alta" | "critica" => severidad,
        _ => "media".to_string(),
    };

    let report_path = state.base_workspace.join(".config").join("fallos_reportados.json");
    let mut fallos: Vec<serde_json::Value> = if report_path.exists() {
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap_or_default()).unwrap_or_default()
    } else {
        Vec::new()
    };

    fallos.push(json!({
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        "severidad": severidad_validada,
        "informe": payload.informe,
        "reportado_por": _username,
    }));

    let _ = fs::create_dir_all(report_path.parent().unwrap());
    let _ = fs::write(&report_path, serde_json::to_string_pretty(&fallos).unwrap_or_default());

    Json(json!({
        "status": "ok",
        "message": format!("Fallo reportado con severidad \\"{}\\". Los ingenieros lo revisaran.", severidad_validada)
    })).into_response()
}

'''

# Insertar antes de 'fn build_app'
insert_pos = content.find('fn build_app(state: AppState) -> Router {')
if insert_pos >= 0:
    content = content[:insert_pos] + struct_and_handler + content[insert_pos:]
    print('Struct y handler agregados antes de build_app')
else:
    print('ERROR: No se encontro build_app')

# 2. Agregar ruta antes de .layer(cors)
old_route = '        .route("/api/client/response", post(client_response))\n        .layer(cors)'
new_route = '        .route("/api/client/response", post(client_response))\n        // Reporte de fallos por usuarios\n        .route("/api/reportar-fallo", post(reportar_fallo_usuario))\n        .layer(cors)'

if old_route in content:
    content = content.replace(old_route, new_route)
    print('Ruta agregada')
else:
    print('ERROR: No se encontro la ruta .layer(cors)')

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print('main.rs guardado')
