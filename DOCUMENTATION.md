# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v3.0

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos,
> cliente Electron (desktop) + Capacitor (Android), túnel Cloudflare para acceso remoto seguro.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~2000 | Servidor HTTP doble puerto, endpoints REST, CAPTCHA, migración de chats, `clean_old_chat_files()`, deduplicación, `sanitize_filename()`, `migrate_chats()` |
| `src/agent.rs` | ~2418 | Bucle principal del agente, 26 herramientas, extract_text_from_docx(), soporte PDF/DOCX |
| `src/auth.rs` | ~947 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~575 | AppState, ActiveAgentStatus, CicleState/CiclePhase, CaptchaRequest, ToolResultStore, SubAgentManager |
| `src/study.rs` | ~973 | Motor de estudio: UserLearningProfile, UserKnowledgeBase, StudyEngine, `build_study_system_prompt()` |
| `src/sync.rs` | ~280 | Sincronización de proyectos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor (ClientAction, ClientRequest, ClientResponse, ConnectedClient) |
| `src/validator.rs` | ~508 | Validación post-escritura (líneas duplicadas, delimitadores, errores Rust) |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos (máx 8, permisos por Patrón Composite) |
| `src/desktop.rs` | ~165 | Control de mouse/teclado (rdev) |
| `src/lib.rs` | ~12 | Librería pública + `pub const STUDY_SYSTEM_PROMPT` |
| `src/utils.rs` | ~72 | sanitize_filename() |
| `electron/main.js` | ~350 | **[v3.0]** Cliente Electron: BrowserWindow + protocolo cliente (connect→poll→execute→respond), ejecuta PowerShell/git/cargo/archivos con Node.js |
| `electron/preload.js` | ~30 | **[v3.0]** Puente IPC contextBridge: executeLocal, setCredentials, getStatus |
| `electron/package.json` | ~30 | **[v3.0]** Dependencias: electron, node-fetch, electron-builder |
| `electron/README.md` | ~100 | **[v3.0]** Documentación del cliente Electron (arquitectura, build, uso) |
| `capacitor/capacitor.config.ts` | ~75 | **[v3.0]** Config Capacitor Android: webDir=../public, plugins Filesystem/Browser/Network |
| `capacitor/package.json` | ~20 | **[v3.0]** Dependencias: @capacitor/core, @capacitor/android, @capacitor/filesystem |
| `capacitor/setup_capacitor.ps1` | ~90 | **[v3.0]** Script de inicialización de plataforma Android |
| `scripts/generate_keys.ps1` | ~105 | Genera par de claves Ed25519 |
| `scripts/sign_nonce.ps1` | ~110 | Firma nonce con clave privada |
| `scripts/cloudflare_tunnel.ps1` | ~180 | Túnel Cloudflare (quick + permanent) |
| `public/index.html` | ~300 | Frontend web con hamburger menu mobile, login dual, admin panel, perfil de estudio |
| `public/app.js` | ~1165 | Lógica frontend: deduplicación client-side, initMobileNav(), username labels para admin, Ctrl+Enter |
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
- **Plugins**: `@capacitor/filesystem` (operaciones básicas de archivos), `@capacitor/network` (conectividad), `@capacitor/browser` (enlaces externos)
- **Ejecución de comandos en Android**: 
  - Usuario admin → el servidor ejecuta directamente (regla: "el servidor NUNCA ejecuta comandos, EXCEPTO SI ES ADMIN")
  - Usuario normal → el servidor reenvía las solicitudes al cliente Electron del usuario en su PC
  - Operaciones básicas de archivos → plugin `@capacitor/filesystem` ejecuta localmente en Android

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

## 🔀 Arquitectura Completa v3.0

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
   │    - PowerShell     │                   │  • Filesystem plugin │
   │    - Git            │                   │  • Comandos vía      │
   │    - Cargo          │                   │    servidor (admin)  │
   │  • Poll + Heartbeat │                   │    o PC cliente      │
   └─────────────────────┘                   └─────────────────────┘
```

**Regla de oro**: El servidor NUNCA ejecuta comandos para usuarios no-admin. Solo admin (puerto 80 o nonce verificado) ejecuta en el servidor. Para usuarios normales, el cliente Electron en su PC ejecuta los comandos.

---

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
| `GET` | `/api/client/check` | `client_check` | Verificar si hay cliente |

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
