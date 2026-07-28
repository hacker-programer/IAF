# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v2.8

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos y cliente de ejecución remota.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~2239 | Servidor HTTP doble puerto, endpoints REST, CAPTCHA, legacy routes, migración, scripts, system prompts, ciclos, `pub const STUDY_SYSTEM_PROMPT` (L51) |
| `src/agent.rs` | ~2418 | Bucle principal del agente, 26 herramientas, extract_text_from_docx(), soporte PDF/DOCX nativo, BUG-026: `if mode == "study"` carga prompt de estudio (L116-121) |
| `src/auth.rs` | ~947 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos booleanos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~575 | AppState, ActiveAgentStatus (con info_messages), CicleState/CiclePhase, CaptchaRequest, ToolResultStore, SubAgentManager, ProcessRegistry |
| `src/study.rs` | ~973 | Motor de estudio: UserLearningProfile, UserKnowledgeBase, StudyEngine, persistencia en .config/data/, `build_study_system_prompt()` (L515) |
| `src/sync.rs` | ~280 | Sincronización de proyectos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor para ejecución remota |
| `src/validator.rs` | ~508 | Validación post-escritura (líneas duplicadas, delimitadores, errores comunes Rust) |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite (Google bloquea scrapers) |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos (máx 8, permisos por Patrón Composite) |
| `src/desktop.rs` | ~165 | Control de mouse/teclado (rdev) |
| `src/lib.rs` | ~12 | Librería pública: expone utils, state, auth, study, desktop, sync + `pub const STUDY_SYSTEM_PROMPT` |
| `src/utils.rs` | ~72 | sanitize_filename() — sanitización de nombres de archivo |
| `scripts/generate_keys.ps1` | ~105 | Genera par de claves Ed25519 via API y las guarda como .pem |
| `scripts/sign_nonce.ps1` | ~110 | Firma un nonce con clave privada para autenticación admin |
| `public/index.html` | ~298 | Frontend web con login dual, admin panel, gestión de usuarios, perfil de estudio con IDs correctos |
| `public/app.js` | ~1066 | Lógica del frontend: auth, admin, perfil de estudio, mensajes en tiempo real, BUG-027: Ctrl+Enter para enviar respuesta |
| `public/style.css` | ~893 | Estilos completos: .info-msg, .final-msg, .input-container, toasts, modales, consola |
| `client/Cargo.toml` | 15 | Cliente binario independiente |
| `client/src/main.rs` | ~350 | Ejecutor local (files, PowerShell, git, cargo) |
| `tests/exhaustive_tests.rs` | ~1835 | Tests exhaustivos: verificación código fuente (include_str!), regresión, integración, estrés |
| `tests/integration_tests.rs` | ~1197 | Tests reales: StudyEngine, UserStore, sanitize_filename, ActiveAgentStatus, DOCX real |
| `tests/frontend_regression_tests.js` | ~230 | Tests de regresión del frontend (Node.js) |
| `prompts/study_system_prompt.txt` | ~80 | System prompt para modo estudio (4 reglas de oro, anti-resúmenes, transparencia) |

---

## 🔧 Cambios v2.8 (BUG-026 + BUG-027)

### BUG-026: System prompt de estudio no se cargaba — el agente usaba prompt de programación
- **`run_agent_loop()`** (agent.rs L55-135): Recibía `mode: &str` pero NUNCA lo usaba. Siempre cargaba `global_prompt` (modo programación).
- **Fix**: Bloque `if mode == "study"` en L116-121 que SOBREESCRIBE completamente `system_prompt` con `state.study_engine.build_study_system_prompt(username, crate::STUDY_SYSTEM_PROMPT)`.
- **`STUDY_SYSTEM_PROMPT`** (main.rs L51): Cambiado de `const` a `pub const` para que `agent.rs` pueda acceder vía `crate::`.
- **`lib.rs`**: Agregado `pub const STUDY_SYSTEM_PROMPT` también en la librería.
- **Flujo completo**: Usuario selecciona modo estudio → frontend envía `mode: "study"` → `chat_endpoint` pasa `mode_bg = "study"` → `run_agent_loop` detecta `mode == "study"` → carga prompt pedagógico + perfil del estudiante.

### BUG-027: Enter en respuesta a pregunta del agente no permitía saltos de línea
- **`#agentQuestionResponse` keydown** (app.js L924-929): Antes enviaba con cualquier Enter. Ahora requiere Ctrl+Enter (o Cmd+Enter en Mac) para enviar; Enter simple inserta nueva línea.
- **Código**: `if (e.key === 'Enter' && (e.ctrlKey || e.metaKey))` con `e.preventDefault()`.

---

## 🔧 Cambios v2.7 (Perfil de Estudio + Mensajes en Tiempo Real)

### BUG-021: Mensajes informativos en el chat (tiempo real)
- **`addMessage(role, text, extraClass)`** (app.js ~951): Ahora acepta tercer parámetro `extraClass` para inyectar clase CSS adicional.
- **`startAgentMonitoring()`** (app.js ~785): Los `info_messages` del backend se muestran con `addMessage('agent', 'ℹ️ ' + msg, 'info-msg')`, apareciendo en el chat con estilo azul animado. El mensaje final se muestra con `addMessage('agent', '✅ ' + msg, 'final-msg')`.
- **Polling reducido**: De 1500ms a 1000ms para respuesta en tiempo real.
- **CSS `.info-msg`** (style.css ~377): Fondo azul semitransparente, animación `infoMsgFadeIn`.
- **CSS `.final-msg`** (style.css ~396): Fondo verde semitransparente, animación `finalMsgPulse`.

### BUG-022: Perfil de estudio no cargaba en el frontend
- **`loadStudyProfile()`** (app.js ~647): Ahora usa IDs correctos del HTML.
- **`switchMode('study')`** (app.js ~279): Muestra `#studyProfileSection` y llama a `loadStudyProfile()`.

### BUG-023: CSS roto — regla `.input-container` huérfana
- Se eliminó un comentario JavaScript `//` que quedó en medio del CSS.

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
| `POST` | `/api/chat` | `chat_endpoint` | Enviar mensaje al agente. Recibe `mode: "study"|"programming"`. Spawnea agente en background. |
| `GET` | `/api/agent/status` | `get_agent_status` | Estado del agente (incluye info_messages, finished, final_message) |
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
| `GET` | `/api/study/profile` | `study_get_profile` | Obtener perfil (UserLearningProfile) + knowledge + engagement |
| `POST` | `/api/study/profile` | `study_save_profile` | Guardar campos del perfil (age, favorite_games, hobbies, neurological_conditions) |
| `GET` | `/api/study/knowledge` | `study_get_knowledge` | Obtener UserKnowledgeBase |
| `POST` | `/api/study/projects` | `study_create_project` | Crear proyecto de estudio |
| `GET` | `/api/study/projects` | `study_get_projects` | Listar proyectos de estudio del usuario |
| `POST` | `/api/study/projects/:id/members` | `study_add_member` | Agregar miembro a proyecto |
| `POST` | `/api/study/build-prompt` | `study_build_prompt` | Construir system prompt personalizado con perfil |

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
