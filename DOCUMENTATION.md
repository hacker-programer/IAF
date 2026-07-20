# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v2.5

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos y cliente de ejecución remota.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~2172 | Servidor HTTP doble puerto, endpoints REST, CAPTCHA, legacy routes, migración, scripts, system prompts, ciclos |
| `src/auth.rs` | ~947 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos booleanos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~575 | AppState, ActiveAgentStatus (con info_messages), CicleState/CiclePhase, CaptchaRequest |
| `src/study.rs` | ~570 | Motor de estudio: perfiles, knowledge base, hipótesis, engagement |
| `src/sync.rs` | ~280 | Sincronización de proyectos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor para ejecución remota |
| `src/agent.rs` | ~2440 | Bucle principal del agente, 26 herramientas (PDF/DOCX en read_file, finalizar_tarea refactorizado, extract_text_from_docx_xml) |
| `src/validator.rs` | ~508 | Validación post-escritura (líneas duplicadas, delimitadores, errores comunes Rust) |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite (Google bloquea scrapers) |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos (máx 8, permisos por Patrón Composite) |
| `src/desktop.rs` | ~165 | Control de mouse/teclado (rdev) |
| `scripts/generate_keys.ps1` | ~105 | Genera par de claves Ed25519 via API y las guarda como .pem |
| `scripts/sign_nonce.ps1` | ~110 | Firma un nonce con clave privada para autenticación admin |
| `public/index.html` | ~298 | Frontend web con login dual, admin panel, gestión de usuarios |
| `public/app.js` | ~1034 | Lógica del frontend: auth, admin, scripts, keygen, .pem upload, showInfoToast para notificaciones en tiempo real |
| `client/Cargo.toml` | 15 | Cliente binario independiente |
| `client/src/main.rs` | ~350 | Ejecutor local (files, PowerShell, git, cargo) |
| `tests/integration_tests.rs` | ~1136 | Tests de integración, aceptación y regresión (42+ tests) |
| `tests/exhaustive_tests.rs` | ~650 | Tests exhaustivos: regresión BUG-001 a BUG-004, integración, E2E, estrés, inyección de fallos, casos límite, contrato API |
| `prompts/study_system_prompt.txt` | ~80 | System prompt para modo estudio (v2.5: anti-resúmenes reforzado) |

---

## 🔧 Cambios v2.5 (Arreglo de BUG-001 a BUG-004)

### BUG-001: Soporte PDF/DOCX en read_file
- **Herramienta**: `read_file` en `agent.rs` ahora detecta extensiones `.pdf` y `.docx`
- **PDF**: usa `pdf-extract = "0.7"` para extraer texto
- **DOCX**: usa `zip = "1.1"` para abrir el ZIP y `extract_text_from_docx_xml()` para parsear `word/document.xml`
- **Función auxiliar**: `extract_text_from_docx_xml(xml: &str) -> String` — parsea tags `<w:t>` y `<w:p>` del XML

### BUG-002: Mensajes informativos en tiempo real
- **`ActiveAgentStatus`** (state.rs línea 152): nuevo campo `info_messages: Vec<String>` (máx 100)
- **`get_agent_status`** (main.rs línea 1556): incluye `info_messages` en la respuesta JSON
- **`notificar_usuario` tipo "informativo"** (agent.rs línea 1290): agrega a `info_messages`
- **`startAgentMonitoring`** (app.js línea 855): detecta nuevos `info_messages` y los muestra como toast + en chat
- **`showInfoToast(message)`** (app.js línea 920): crea notificación flotante con auto-dismiss (8s)

### BUG-003: Modo estudio anti-resúmenes
- **`study_system_prompt.txt`**: regla de oro "PROHIBIDO DAR RESÚMENES O TEMARIOS"
- Formato de respuesta obligatorio de 5 pasos: concepto → explicación → pregunta → mini-ejercicio → esperar
- Máximo un concepto nuevo por mensaje

### BUG-004: finalizar_tarea refactorizado
- **`finalizar_tarea`** (agent.rs línea 1306): código refactorizado de 1 línea a ~30 líneas legibles
- Validación de `mensaje_final` vacío
- Limpieza de flags: `esperando_respuesta_usuario`, `esperando_aprobacion_plan`, `info_messages`
- Prevención de race conditions al finalizar

---

## 📊 ActiveAgentStatus (state.rs líneas 140-155)

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

### Creación de Admin (v2.4)

- Los admins se crean con **clave pública Ed25519**, NO contraseña.
- El frontend permite subir archivo `.pem` o generar claves desde la UI.
- `POST /api/admin/users` acepta `public_key` (64 chars hex) para admins.
- `GET /api/auth/keygen` genera un par de claves nuevo (se muestra una sola vez).

---

## 🌐 Endpoints REST

### Agente y Chat
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `POST` | `/api/chat` | `chat_endpoint` | Enviar mensaje al agente (spawnea agente en background) |
| `GET` | `/api/agent/status` | `get_agent_status` | Estado del agente (incluye info_messages desde v2.5) |
| `GET` | `/api/agent/steps` | `agent_steps` | Pasos de auditoría |
| `GET` | `/api/agent/summary` | `agent_summary` | Resumen textual del progreso |
| `POST` | `/api/agent/responder` | `agent_responder` | Responder a pregunta del agente |
| `POST` | `/api/agent/aprobar_plan` | `agent_approve_plan` | Aprobar/rechazar plan |
| `POST` | `/api/agent/interrupt` | `interrupt_agent` | Interrumpir agente |
| `GET` | `/api/chats` | `get_chats` | Listar historial de chats |
| `GET` | `/api/chats/:id` | `get_chat_session` | Obtener chat por ID |
| `POST` | `/api/reportar-fallo` | `reportar_fallo` | Reportar bug/fallo al sistema |

### Estudio
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `GET` | `/api/study/profile` | `get_study_profile` | Obtener perfil de aprendizaje |
| `POST` | `/api/study/profile` | `save_study_profile` | Guardar perfil |
| `GET` | `/api/study/knowledge` | `get_study_knowledge` | Obtener knowledge base |

---

## 🧪 Tests Exhaustivos (tests/exhaustive_tests.rs)

### Secciones
1. **Regresión (BUG-001 a BUG-004)**: 17 tests que validan que los bugs no reaparezcan
2. **Integración Backend↔Frontend**: 10 tests de contrato API
3. **End to End**: 4 tests de flujos completos (estudio, programación, reporte de fallo, CAPTCHA)
4. **Estrés**: 6 tests de carga (muchos mensajes, muchos pasos, muchos proyectos)
5. **Inyección de Fallos**: 8 tests de manejo de errores (path traversal, JSON malformado, parámetros faltantes)
6. **Casos Límite**: 11 tests de edge cases (mensajes vacíos, UUIDs inválidos, passwords cortas)
7. **Contrato API**: 2 tests de validación de endpoints y respuestas de error

---

## 🔧 Herramientas del Agente (agent.rs)

| # | Herramienta | Descripción | Cambios v2.5 |
|---|------------|-------------|--------------|
| 1 | `search_google` | Búsqueda web | - |
| 2 | `read_file` | Leer archivo | Soporte PDF/DOCX (BUG-001) |
| 3 | `write_file_with_commit` | Escribir archivo + commit git | - |
| 4 | `execute_powershell` | Ejecutar comando PowerShell | - |
| 5 | `search_code` | Búsqueda de texto en código | - |
| 6 | `fork_and_clone_repo` | Forkear y clonar repo | - |
| 7 | `read_url` | Leer URL pública | - |
| 8 | `check_github_cli` | Ejecutar gh CLI | - |
| 9 | `notificar_usuario` | Comunicarse con el usuario | Escribe en info_messages (BUG-002) |
| 10 | `finalizar_tarea` | Finalizar la tarea | Refactorizado + limpieza de estado (BUG-004) |
| 11 | `image_fetch` | Descargar imagen | - |
| 12 | `image_view` | Ver imagen en contexto | - |
| 13 | `image_release` | Liberar imagen | - |
| 14 | `analyze_images` | Analizar imagen con Qwen2.5-VL | - |
| 15 | `git_resolve_divergence` | Resolver divergencia git | - |
| 16 | `kill_process` | Matar proceso hijo | - |
| ... | ... | ... | ... |

---

## 📦 Dependencias (Cargo.toml)

| Dependencia | Versión | Uso | Agregada en |
|-------------|---------|-----|-------------|
| `pdf-extract` | 0.7 | Extraer texto de PDFs | v2.5 (BUG-001) |
| `zip` | 1.1 | Leer DOCX (formato ZIP) | v2.5 (BUG-001) |
| `tokio` | 1 (full) | Runtime async | v1.0 |
| `axum` | 0.7 | Framework HTTP | v1.0 |
| `ed25519-dalek` | 2 | Firmas Ed25519 | v1.0 |
| `argon2` | 0.5 | Hashing de contraseñas | v1.0 |

---

## 📝 Frontend — Funciones Clave (app.js v2.5)

| Función | Línea | Descripción |
|---------|-------|-------------|
| `startAgentMonitoring()` | ~837 | Polling cada 1.5s del estado del agente: preguntas, planes, CAPTCHA, info_messages |
| `showInfoToast(message)` | ~920 | [v2.5] Notificación toast flotante para mensajes informativos en tiempo real |
| `renderConsoleSteps(steps)` | ~954 | Renderiza pasos de auditoría en la consola lateral |
| `sendMessageToAgent(text, mode)` | ~811 | Envía mensaje al agente con modo (study/programming) |
| `init()` | ~1 | Inicialización de la UI, detección de puerto 80 vs 8080 |