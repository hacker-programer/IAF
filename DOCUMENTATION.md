# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v3.1

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos,
> cliente Electron (desktop) + Capacitor (Android) con ShellExecutor nativo, túnel Cloudflare para acceso remoto seguro.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~2300 | Servidor HTTP doble puerto, endpoints REST, CAPTCHA, migración de chats, `clean_old_chat_files()`, deduplicación, `sanitize_filename()`, `migrate_chats()`, `client_check()` v3.1 |
| `src/agent.rs` | ~2418 | Bucle principal del agente, 26 herramientas, extract_text_from_docx(), soporte PDF/DOCX |
| `src/auth.rs` | ~947 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~580 | AppState (con `connected_clients` HashMap), ActiveAgentStatus, CicleState/CiclePhase, CaptchaRequest, ToolResultStore, SubAgentManager |
| `src/study.rs` | ~973 | Motor de estudio: UserLearningProfile, UserKnowledgeBase, StudyEngine, `build_study_system_prompt()` |
| `src/sync.rs` | ~280 | Sincronización de proyectos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor (ClientAction, ClientRequest, ClientResponse, ConnectedClient) |
| `src/validator.rs` | ~508 | Validación post-escritura (líneas duplicadas, delimitadores, errores Rust) |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos (máx 8, permisos por Patrón Composite) |
| `src/desktop.rs` | ~165 | Control de mouse/teclado (rdev) |
| `src/lib.rs` | ~12 | Librería pública + `pub const STUDY_SYSTEM_PROMPT` |
| `src/utils.rs` | ~72 | sanitize_filename() |
| `electron/main.js` | ~350 | Cliente Electron: BrowserWindow + protocolo cliente (connect→poll→execute→respond), ejecuta PowerShell/git/cargo/archivos con Node.js |
| `electron/preload.js` | ~30 | Puente IPC contextBridge: executeLocal, setCredentials, getStatus |
| `electron/package.json` | ~30 | Dependencias: electron, node-fetch, electron-builder |
| `electron/README.md` | ~100 | Documentación del cliente Electron (arquitectura, build, uso) |
| `capacitor/capacitor.config.ts` | ~75 | Config Capacitor Android: webDir=../public, plugins Filesystem/Browser/Network/ShellExecutor |
| `capacitor/package.json` | ~20 | Dependencias: @capacitor/core, @capacitor/android, @capacitor/filesystem |
| `capacitor/setup_capacitor.ps1` | ~140 | **[v3.1]** Script de inicialización + instalación del plugin ShellExecutor |
| `capacitor/src/plugins/shell-executor.ts` | ~70 | **[v3.1]** Interfaz TypeScript del plugin ShellExecutor (execute, which, info) |
| `capacitor/android-plugins/src/main/java/com/iaf/plugins/ShellExecutorPlugin.java` | ~220 | **[v3.1]** Implementación nativa Android: ejecuta comandos shell con Runtime.exec(), timeout 60s, buffer 512KB |
| `scripts/generate_keys.ps1` | ~105 | Genera par de claves Ed25519 |
| `scripts/sign_nonce.ps1` | ~110 | Firma nonce con clave privada |
| `scripts/cloudflare_tunnel.ps1` | ~180 | Túnel Cloudflare (quick + permanent) |
| `public/index.html` | ~300 | Frontend web con hamburger menu mobile, login dual, admin panel, perfil de estudio |
| `public/app.js` | ~1260 | **[v3.1]** Lógica frontend: checkClient() actualizado con detección por plataforma, deduplicación client-side, initMobileNav() |
| `public/style.css` | ~900 | Estilos + media queries responsive (tablets ≤1024px, móviles ≤768px, small ≤400px, landscape) |
| `tests/exhaustive_tests.rs` | ~1835 | Tests exhaustivos |
| `tests/integration_tests.rs` | ~1197 | Tests de integración |
| `tests/frontend_regression_tests.js` | ~230 | Tests de regresión del frontend |

### ❌ Eliminado en v3.0
| Archivo | Motivo |
|---------|--------|
| `client/Cargo.toml` | Reemplazado por `electron/` |
| `client/src/main.rs` | Reemplazado por `electron/main.js` |
| `client/target/` | Build artifacts del cliente Rust |

---

## 🔧 Cambios v3.1 — Fix "pide cliente" + ShellExecutor Android

### 🐛 Fix: client_check() ya no busca el viejo cliente Rust

**Problema (v3.0)**: `client_check()` verificaba la existencia de `iaf-client.exe` (cliente Rust eliminado en v3.0). Como nunca lo encontraba, siempre mostraba "Cliente no detectado" con instrucciones obsoletas.

**Solución (v3.1)**:
- `client_check()` ahora recibe `State<AppState>` y consulta el mapa `connected_clients` (línea 483 de state.rs)
- Verifica clientes activos (heartbeat < 120s)
- Verifica si Electron está instalado (`electron/package.json` + `electron/main.js`)
- Devuelve instrucciones actualizadas según el estado real

**Respuesta del endpoint**:
```json
{
  "status": "ok",
  "client_installed": true|false,
  "active_clients": [{ "client_id": "...", "username": "...", "host_info": "..." }],
  "active_client_count": N,
  "total_connected": N,
  "electron_installed": true|false,
  "v3_client": true,
  "instructions": "..."
}
```

### ✅ Frontend checkClient() actualizado

- **Electron**: Verifica estado de conexión vía `window.iafClient.getStatus()`
- **Capacitor/Android**: Informa sobre comandos shell disponibles (ls, cat, grep, find, curl)
- **Browser**: Consulta `/api/client/check` y muestra mensajes contextuales con colores

### 📱 ShellExecutor Plugin para Android

**Plugin nativo Capacitor** que permite a la app Android ejecutar comandos shell localmente.

**Archivos**:
- `capacitor/src/plugins/shell-executor.ts` — Interfaz TypeScript (ShellExecuteOptions, ShellExecuteResult)
- `capacitor/android-plugins/.../ShellExecutorPlugin.java` — Implementación nativa

**Métodos**:
| Método | Descripción |
|--------|-------------|
| `execute({ command, timeout?, workdir?, env? })` | Ejecuta comando shell con `/system/bin/sh -c` |
| `which({ command })` | Verifica si un comando está en el PATH |
| `info()` | Devuelve shell, home, PATH y comandos disponibles |

**Seguridad**:
- Timeout máximo 60 segundos
- Buffer limitado a 512 KB para stdout/stderr
- Registro de auditoría vía Android Log

**Comandos disponibles en Android sin Termux**:
`ls`, `cat`, `echo`, `mkdir`, `rm`, `cp`, `mv`, `pwd`, `chmod`, `ps`, `df`, `du`, `grep`, `find`, `head`, `tail`, `wc`, `sort`, `uniq`, `cut`, `tr`, `sed`, `awk`, `curl`, `wget`, `tar`, `gzip`, `date`, `whoami`, `id`, `uname`

**Comandos que requieren Termux**: `cargo`, `git`, `rustc`, `python`, `node`, `npm`

---

## 🔧 Cambios v3.0 — Electron + Capacitor + Chat Dedup + Android UX

### 🖥️ Cliente Electron (reemplaza al cliente Rust)

```
┌──────────────────────────────────────────────────┐
│              IAF Electron Client                  │
│                                                   │
│  ┌─────────────────────┐  ┌────────────────────┐ │
│  │   Renderer (UI)     │  │   Main Process     │ │
│  │   Carga UI desde    │  │   • connect/poll   │ │
│  │   servidor IAF      │  │   • execute local  │ │
│  │   (HTTP/HTTPS)      │  │   • heartbeat      │ │
│  └─────────────────────┘  └────────────────────┘ │
│               ↕ IPC (contextBridge)               │
└──────────────────────────────────────────────────┘
```

- **`electron/main.js`**: Implementa el mismo protocolo que el cliente Rust (connect→poll→execute→respond) usando Node.js built-ins (`fs`, `child_process`, `crypto`)
- **`electron/preload.js`**: Puente seguro entre renderer (UI web) y main process vía `contextBridge`
- El renderer NO tiene acceso directo a Node.js — solo 3 funciones expuestas: `executeLocal`, `setCredentials`, `getStatus`
- Comandos soportados: PowerShell, git, cargo, archivos (read/write/exists/metadata), directorios, búsqueda de código

### 📱 Capacitor Android

- **`capacitor/capacitor.config.ts`**: Configuración para envolver `public/` en una WebView Android
- **Plugins**: `@capacitor/filesystem` (operaciones básicas de archivos), `@capacitor/network` (conectividad), `@capacitor/browser` (enlaces externos), **`ShellExecutor`** (comandos shell nativos, nuevo en v3.1)
- **Ejecución de comandos en Android (v3.1)**:
  - **ShellExecutor nativo**: Comandos shell básicos disponibles directamente (ls, cat, grep, find, curl...)
  - Usuario admin → el servidor ejecuta directamente
  - Para desarrollo completo (cargo, git) → instalar Termux y usarlo como entorno

### 🐛 Chat Deduplication (BUG-028)

**Síntoma**: Dos archivos `.json` con mismo UUID pero diferente título aparecían como conversaciones duplicadas.

**Causa**: Al cambiar el título de una conversación, se creaba un nuevo archivo sin eliminar el viejo.

**Fixes**:
- **Backend** (`clean_old_chat_files()`): Elimina archivos viejos con el mismo UUID antes de guardar
- **Backend** (`get_chats`): `HashSet<String>` para deduplicar por `id`, salta `.bak`
- **Frontend** (`loadChatHistory`): Deduplicación client-side como defensa en profundidad
- **Frontend** (`selectChatSession`): Corregida línea repetida `chatArea.innerHTML = ''`

### 👑 Admin ve username en chats

- **`get_chats`**: Respuesta incluye campo `username` con el dueño de cada chat
- **`loadChatHistory`**: Muestra `(@username)` para admins viendo chats de otros usuarios

### 📱 Android / Mobile Responsive

- **CSS media queries**:
  - ≤1024px: sidebar 320px, console 280px
  - ≤768px: sidebar → drawer deslizable, console → bottom sheet, botones ≥44px touch-friendly, inputs 16px para evitar zoom iOS
  - ≤400px: ajustes adicionales de espaciado
  - ≤500px height (landscape): layout horizontal compacto
- **Hamburger menu**: botón ☰ con toggle, overlay semitransparente, cierre al hacer click fuera
- **JS** (`initMobileNav()`): Manejo completo del drawer en mobile

---

## 🔧 Cambios v2.9 — Cloudflare Tunnel (Solo Puerto 8080)

### Arquitectura de doble puerto con túnel

```
                    ┌──────────────────────────────────────┐
                    │         IAF Server (Rust/Axum)        │
                    │                                      │
                    │  ┌─────────────┐  ┌───────────────┐  │
 Firewall local ◄───┼──│ Puerto 80   │  │ Puerto 8080   │──┼──► cloudflared
 (solo confianza)   │  │ 0.0.0.0:80  │  │ 127.0.0.1:8080│  │    (túnel TLS)
                    │  │ port_80:true│  │ port_80:false │  │    │
                    │  │ SIN auth    │  │ CON auth      │  │    ▼
                    │  └─────────────┘  └───────────────┘  │  Internet
                    └──────────────────────────────────────┘  (https://...)
```

---

## 🔀 Arquitectura Completa v3.1

```
                          ┌──────────────────────────┐
                          │     IAF Server (Rust)     │
                          │   Puerto 80 + Puerto 8080 │
                          └─────┬──────────┬─────────┘
                                │          │
              ┌─────────────────┘          └──────────────┐
              ▼                                           ▼
   ┌─────────────────────┐                   ┌─────────────────────┐
   │  Electron (PC)      │                   │  Capacitor (Android)│
   │  • UI web embebida  │                   │  • WebView nativo   │
   │  • Ejecuta comandos │                   │  • UI web empaquetada│
   │    - PowerShell     │                   │  • ShellExecutor 🔥  │
   │    - Git            │                   │    (ls,cat,grep,etc) │
   │    - Cargo          │                   │  • Filesystem plugin │
   │  • Poll + Heartbeat │                   │  • Comandos vía      │
   └─────────────────────┘                   │    servidor (admin)  │
                                             │    o PC cliente      │
                                             └─────────────────────┘
```

**Regla de oro**: El servidor NUNCA ejecuta comandos para usuarios no-admin. Solo admin (puerto 80 o nonce verificado) ejecuta en el servidor. Para usuarios normales:
- **Windows/Linux/Mac**: cliente Electron ejecuta comandos localmente
- **Android**: ShellExecutor ejecuta comandos shell nativos localmente

---

## 📊 AppState (state.rs líneas 470-495)

```rust
pub struct AppState {
    pub config_path: PathBuf,
    pub prompts: Arc<Mutex<PromptConfig>>,
    pub projects: Arc<Mutex<Vec<Project>>>,
    pub base_workspace: PathBuf,
    pub pending_captcha: Arc<Mutex<Option<CaptchaRequest>>>,
    pub active_agent: Arc<Mutex<ActiveAgentStatus>>,
    pub abort_handle: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
    pub desktop: Arc<Mutex<DesktopController>>,
    pub image_store: Arc<Mutex<HashMap<String, String>>>,
    pub context_store: Arc<Mutex<HashMap<String, ContextEntry>>>,
    pub process_registry: ProcessRegistry,
    pub tool_results: ToolResultStore,
    pub sub_agents: SubAgentManager,
    pub user_store: UserStore,
    pub challenge_store: ChallengeStore,
    pub session_store: SessionStore,
    pub study_engine: StudyEngine,
    pub sync_store: SyncStore,
    pub connected_clients: Arc<Mutex<HashMap<String, ConnectedClient>>>,    // ← v3.1 client_check usa esto
    pub client_pending_requests: Arc<Mutex<HashMap<String, Vec<ClientRequest>>>>,
    pub client_responses: Arc<Mutex<HashMap<String, ClientResponse>>>,
    pub port_80: bool,
}
```

## 📊 ActiveAgentStatus (state.rs)

```rust
pub struct ActiveAgentStatus {
    pub running: bool,
    pub interrupted: bool,
    pub finished: bool,
    pub final_message: Option<String>,
    pub esperando_respuesta_usuario: bool,
    pub pregunta_usuario: Option<String>,
    pub respuesta_usuario: Option<String>,
    pub esperando_aprobacion_plan: bool,
    pub plan_propuesto: Option<String>,
    pub info_messages: Vec<String>,
    pub thinking_content: Vec<String>,
    pub steps: Vec<AuditStep>,
    pub current_session_id: Option<String>,
}
```

## 🔐 Autenticación Dual

| Método | Usuarios | Endpoint |
|--------|----------|----------|
| **Username + Password (argon2id)** | Usuarios normales | `POST /api/auth/login` |
| **Ed25519 Challenge-Response** | Solo admins | `POST /api/auth/challenge` → `POST /api/auth/verify` |

---

## 🌐 Endpoints REST

### Agente y Chat
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `POST` | `/api/chat` | `chat_endpoint` | Enviar mensaje al agente |
| `GET` | `/api/agent/status` | `get_agent_status` | Estado del agente |
| `GET` | `/api/agent/steps` | `agent_steps` | Pasos de auditoría |
| `POST` | `/api/agent/responder` | `agent_responder` | Responder a pregunta |
| `POST` | `/api/agent/aprobar_plan` | `agent_approve_plan` | Aprobar/rechazar plan |
| `POST` | `/api/agent/interrupt` | `interrupt_agent` | Interrumpir agente |
| `GET` | `/api/chats` | `get_chats` | Listar chats (admin ve todos con username) |
| `GET` | `/api/chats/:id` | `get_chat_session` | Obtener chat por ID |

### Cliente (Electron / Capacitor)
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `POST` | `/api/client/connect` | `client_connect` | Registrar cliente |
| `POST` | `/api/client/poll` | `client_poll` | Polling de solicitudes |
| `POST` | `/api/client/response` | `client_response` | Enviar resultado |
| `POST` | `/api/client/heartbeat` | `client_heartbeat` | Heartbeat (30s) |
| `GET` | `/api/client/check` | `client_check` | **[v3.1]** Verificar clientes conectados (Electron/Capacitor) + Electron instalado |

---

## 📦 Dependencias Clave

| Dependencia | Uso |
|-------------|-----|
| `axum` 0.7 | Framework HTTP |
| `ed25519-dalek` 2 | Firmas Ed25519 |
| `argon2` 0.5 | Hashing de contraseñas |
| `electron` 28 | Cliente desktop (Node.js) |
| `@capacitor/core` 6 | App Android híbrida |
| `@capacitor/filesystem` 6 | Operaciones de archivos en Android |
| `ShellExecutor` (custom) | **[v3.1]** Plugin nativo Android para comandos shell |

---

## 🏷️ Términos de Búsqueda

Para encontrar componentes rápidamente con `search_code`:
- `client_check` → Endpoint de verificación de cliente (main.rs línea ~438)
- `connected_clients` → Mapa de clientes conectados (state.rs línea ~483)
- `ShellExecutor` → Plugin Android para comandos shell
- `checkClient` → Frontend checkClient() en app.js
- `ActiveAgentStatus` → Estado del agente activo
- `CiclePhase` → Fases del ciclo de programación
- `ClientRequest` / `ClientResponse` → Protocolo cliente-servidor
- `UserLimits` / `WeeklySchedule` → Límites y horarios de usuarios
- `migrate_chats` → Migración de formato de chats
- `clean_old_chat_files` → Deduplicación de archivos de chat
