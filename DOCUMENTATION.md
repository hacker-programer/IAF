# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v2.6

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos y cliente de ejecución remota.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~2166 | Servidor HTTP doble puerto, endpoints REST, CAPTCHA, legacy routes, migración, scripts, system prompts, ciclos |
| `src/agent.rs` | ~2442 | Bucle principal del agente, 26 herramientas, extract_text_from_docx(), soporte PDF/DOCX nativo |
| `src/auth.rs` | ~947 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos booleanos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~575 | AppState, ActiveAgentStatus (con info_messages), CicleState/CiclePhase, CaptchaRequest, ToolResultStore, SubAgentManager, ProcessRegistry |
| `src/study.rs` | ~570 | Motor de estudio: perfiles, knowledge base, hipótesis, engagement, persistencia en .config/data/ |
| `src/sync.rs` | ~280 | Sincronización de proyectos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor para ejecución remota |
| `src/validator.rs` | ~508 | Validación post-escritura (líneas duplicadas, delimitadores, errores comunes Rust) |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite (Google bloquea scrapers) |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos (máx 8, permisos por Patrón Composite) |
| `src/desktop.rs` | ~165 | Control de mouse/teclado (rdev) |
| `src/lib.rs` | ~8 | Librería pública: expone utils, state, auth, study, desktop, sync para tests de integración |
| `src/utils.rs` | ~72 | sanitize_filename() — sanitización de nombres de archivo |
| `scripts/generate_keys.ps1` | ~105 | Genera par de claves Ed25519 via API y las guarda como .pem |
| `scripts/sign_nonce.ps1` | ~110 | Firma un nonce con clave privada para autenticación admin |
| `public/index.html` | ~298 | Frontend web con login dual, admin panel, gestión de usuarios |
| `public/app.js` | ~1034 | Lógica del frontend: auth, admin, scripts, keygen, .pem upload, startAgentMonitoring con info_messages |
| `public/style.css` | ~520 | Estilos completos: toasts, modales, consola, login, admin panel |
| `client/Cargo.toml` | 15 | Cliente binario independiente |
| `client/src/main.rs` | ~350 | Ejecutor local (files, PowerShell, git, cargo) |
| `tests/exhaustive_tests.rs` | ~510 | Tests exhaustivos: verificación código fuente (include_str!), regresión, integración, estrés, inyección fallos, casos límite, smoke tests |
| `tests/integration_tests.rs` | ~470 | Tests reales: StudyEngine, UserStore, sanitize_filename, ActiveAgentStatus, DOCX real, CiclePhase, ChatSession, contrato API |
| `tests/frontend_regression_tests.js` | ~230 | Tests de regresión del frontend (Node.js): copyNonceCmd, startAgentMonitoring, info_messages |
| `prompts/study_system_prompt.txt` | ~80 | System prompt para modo estudio (anti-resúmenes reforzado) |

---

## 🔧 Cambios v2.6 (Tests Reales y Cero Warnings)

### Tests completamente reescritos
- **`exhaustive_tests.rs`**: 50+ tests REALES usando `include_str!` para verificar código fuente, `std::path::Path` para extensiones, serialización real con `serde_json`.
- **`integration_tests.rs`**: Tests de integración REALES con `StudyEngine` (disco real), `UserStore` (contraseñas reales), `ActiveAgentStatus` (serialización), creación de DOCX real con `zip` + `quick-xml`.
- **`frontend_regression_tests.js`**: Mantenido para tests de lógica JS del frontend.
- **Eliminados**: `regression_tests.rs`, `reg_stu_tests.rs` (redundantes, reemplazados por los nuevos).

### Módulos expuestos en lib.rs
- `pub mod utils` — `sanitize_filename()`
- `pub mod state` — `ActiveAgentStatus`, `CiclePhase`, `CicleState`, `ChatSession`, `ChatMessage`
- `pub mod auth` — `UserStore`, `UserLimits`
- `pub mod study` — `StudyEngine`
- `pub mod desktop` — `DesktopController`
- `pub mod sync` — `SyncStore`

### Principio: CERO tests simulados
Todos los tests ahora prueban comportamiento REAL:
- `include_str!` verifica que el código fuente contiene las funciones requeridas
- `std::path::Path::extension()` prueba el comportamiento real de detección de extensiones
- `StudyEngine::new()` crea y lee archivos reales en disco
- `UserStore` crea usuarios con contraseñas hasheadas reales (argon2)
- `zip::ZipWriter` + `quick_xml::Reader` crean y parsean DOCX reales

---

## 📊 ActiveAgentStatus (state.rs)

```rust
pub struct ActiveAgentStatus {
    pub running: bool,
    pub interrupted: bool,
    pub finished: bool,                    // true cuando llama a finalizar_tarea
    pub final_message: Option<String>,     // resumen final del agente
    pub esperando_respuesta_usuario: bool, // pregunta pendiente
    pub pregunta_usuario: Option<String>,
    pub respuesta_usuario: Option<String>,
    pub esperando_aprobacion_plan: bool,   // plan pendiente
    pub plan_propuesto: Option<String>,
    pub info_messages: Vec<String>,        // [v2.5] notificaciones informativas en tiempo real (máx 100)
    pub thinking_content: Vec<String>,
    pub steps: Vec<AuditStep>,             // pasos de auditoría
    pub current_session_id: Option<String>,
}
```

---

## 🔐 Autenticación Dual y Permisos

| Método | Usuarios | Endpoint |
|--------|----------|----------|
| **Username + Password (argon2id)** | Usuarios normales | `POST /api/auth/login` |
| **Ed25519 Challenge-Response** | Solo admins | `POST /api/auth/challenge` → `POST /api/auth/verify` |

---

## 🌐 Endpoints REST

### Agente y Chat
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `POST` | `/api/chat` | `chat_endpoint` | Enviar mensaje al agente (spawnea agente en background) |
| `GET` | `/api/agent/status` | `get_agent_status` | Estado del agente (incluye info_messages, finished, final_message) |
| `GET` | `/api/agent/steps` | `agent_steps` | Pasos de auditoría |
| `GET` | `/api/agent/summary` | `agent_summary` | Resumen textual del progreso |
| `POST` | `/api/agent/responder` | `agent_responder` | Responder a pregunta del agente |
| `POST` | `/api/agent/aprobar_plan` | `agent_approve_plan` | Aprobar/rechazar plan |
| `POST` | `/api/agent/interrupt` | `interrupt_agent` | Interrumpir agente |
| `GET` | `/api/chats` | `get_chats` | Listar historial de chats |
| `GET` | `/api/chats/:id` | `get_chat_session` | Obtener chat por ID |
| `POST` | `/api/reportar-fallo` | `reportar_fallo` | Reportar bug/fallo al sistema |

---

## 🧪 Tests (v2.6)

### exhaustive_tests.rs (50+ tests)
1. **Source Code Verification**: `include_str!` contra agent.rs, main.rs, app.js, style.css, Cargo.toml, state.rs, study.rs
2. **Regresión**: finalizar_tarea (BUG-004), info_messages (BUG-002), read_file PDF/DOCX (BUG-001)
3. **Integración**: flujo detección extensiones, transiciones estado agente, ActiveAgentStatus JSON
4. **Estrés**: 10K info_messages, 5K consumo incremental, 1K extensiones
5. **Inyección de Fallos**: archivos inexistentes, extensiones vacías, Unicode, null bytes, path traversal
6. **Casos Límite**: mensajes vacíos, unicode multilínea, dotfiles (.gitignore), múltiples puntos, números en extensión
7. **Smoke Tests**: todas las herramientas definidas en agent.rs

### integration_tests.rs (40+ tests)
1. **StudyEngine**: carga/save perfiles, knowledge base, teaching method, múltiples usuarios, directorios internos
2. **sanitize_filename**: ASCII, espacios, especiales, no-ASCII, truncado, trim, guiones, underscores, vacío
3. **ActiveAgentStatus**: default, serialización JSON, deserialización, info_messages vacío
4. **DOCX**: creación real + quick-xml, documento vacío
5. **UserStore**: crear usuario, verificar password, admin con public key, listar, has_study_access
6. **CiclePhase**: transiciones completas, default
7. **ChatSession**: serialización/deserialización
8. **Contrato API**: estructura /api/agent/status, /api/chat, login, errores

---

## 📦 Dependencias (Cargo.toml)

| Dependencia | Versión | Uso | Agregada en |
|-------------|---------|-----|-------------|
| `pdf-extract` | 0.7 | Extraer texto de PDFs | v2.5 (BUG-001) |
| `zip` | 0.6 | Leer DOCX (formato ZIP) | v2.5 (BUG-001) |
| `quick-xml` | 0.31 | Parsear XML dentro de DOCX | v2.5 (BUG-001) |
| `tokio` | 1 (full) | Runtime async | v1.0 |
| `axum` | 0.7 | Framework HTTP | v1.0 |
| `ed25519-dalek` | 2 | Firmas Ed25519 | v1.0 |
| `argon2` | 0.5 | Hashing de contraseñas | v1.0 |
