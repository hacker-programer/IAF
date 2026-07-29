# 🔧 DOCUMENTACIÓN INTERNA — IAF (Intelligent Agent Framework) v3.0

> **Audiencia**: Desarrolladores que van a mantener, extender o depurar el sistema.
> **Última actualización**: Julio 2026

---

## 🏗️ Arquitectura General v3.0

```
┌──────────────────────┐     HTTP/WS     ┌──────────────────────┐     API Calls    ┌─────────────┐
│   Frontend (SPA)     │ ◄──────────────► │   Backend (Rust)     │ ◄──────────────► │  DeepSeek   │
│  HTML/CSS/JS         │                 │   Axum + Tokio       │                 │   API V4    │
│                      │                 │   Puerto 80 + 8080   │                 └─────────────┘
└──────────────────────┘                 └──────────┬───────────┘
        │                                          │
        │ Cargado por:                             ├──► Cliente Electron (PC)
        ├── Navegador (http://...)                 │    • connect/poll/execute/respond
        ├── Electron (BrowserWindow)               │    • PowerShell, git, cargo
        └── Capacitor (WebView Android)            │
                                                   └──► Cliente Capacitor (Android)
                                                        • WebView nativo
                                                        • Filesystem plugin
                                                        • Comandos vía PC o server (admin)
```

### Clientes

| Cliente | Plataforma | Ejecuta comandos | Cómo |
|---------|-----------|-----------------|------|
| **Navegador** | Cualquiera | ❌ (solo admin) | Server ejecuta si admin |
| **Electron** | Windows/Linux/Mac | ✅ | Node.js child_process |
| **Capacitor** | Android | ✅ (vía PC) | Server reenvía a Electron del usuario |

---

## 🌐 Arquitectura de Doble Puerto

```
                    ┌──────────────────────────────────────┐
                    │         IAF Server (Rust/Axum)        │
                    │                                      │
                    │  ┌─────────────┐  ┌───────────────┐  │
 Firewall local ◄───┼──│ Puerto 80   │  │ Puerto 8080   │──┼──► Túnel Cloudflare
 (solo confianza)   │  │ 0.0.0.0:80  │  │ 127.0.0.1:8080│  │    (Internet)
                    │  │ port_80:true│  │ port_80:false │  │
                    │  │ SIN auth    │  │ CON auth      │  │
                    │  └─────────────┘  └───────────────┘  │
                    └──────────────────────────────────────┘
```

---

## 📁 Estructura de Archivos v3.0

```
C:\Users\Fa\Desktop\IAF\
├── src/
│   ├── main.rs          # Servidor HTTP, doble puerto, clean_old_chat_files(), migrate_chats()
│   ├── agent.rs         # Agent loop, 26 herramientas
│   ├── state.rs         # AppState, ActiveAgentStatus, CicleState
│   ├── auth.rs          # Auth dual (password + Ed25519)
│   ├── study.rs         # Motor de estudio
│   ├── scraper.rs       # Búsqueda web
│   ├── desktop.rs       # Control mouse/teclado
│   ├── validator.rs     # Validación post-escritura
│   ├── sync.rs          # Sincronización proyectos
│   ├── client_protocol.rs # Protocolo cliente-servidor (ClientAction, ClientRequest, etc.)
│   ├── sub_agent.rs     # Sub-agentes (máx 8)
│   ├── lib.rs           # Re-exportaciones
│   └── utils.rs         # sanitize_filename()
├── electron/            # [v3.0] Cliente Electron
│   ├── main.js          # Main process: BrowserWindow + client protocol
│   ├── preload.js       # contextBridge (IPC seguro)
│   ├── package.json     # Dependencias (electron, node-fetch)
│   └── README.md        # Documentación del cliente
├── capacitor/           # [v3.0] App Android
│   ├── capacitor.config.ts  # Config (webDir=../public, plugins)
│   ├── package.json     # Dependencias (@capacitor/core, android, filesystem)
│   └── setup_capacitor.ps1  # Script de inicialización
├── scripts/
│   ├── generate_keys.ps1
│   ├── sign_nonce.ps1
│   ├── cloudflare_tunnel.ps1
│   └── cloudflared_config.yml
├── public/
│   ├── index.html       # [v3.0] + hamburger menu mobile
│   ├── app.js           # [v3.0] + dedup client-side, initMobileNav(), username labels
│   └── style.css        # [v3.0] + media queries responsive
├── prompts/
│   ├── default_system_prompt.txt
│   └── study_system_prompt.txt
├── .config/
├── Cargo.toml
├── DOCUMENTATION.md
├── DOCUMENTACION_CLIENTE.md
├── DOCUMENTACION_INTERNA.md  # Este archivo
└── MEMORIES.md
```

### ❌ Eliminado en v3.0
- `client/Cargo.toml`, `client/src/main.rs`, `client/target/` — reemplazados por `electron/`

---

## 🔑 Componentes Principales

### `main.rs` — Servidor HTTP (v3.0)

**Nuevas funciones v3.0**:

| Función | Línea | Propósito |
|---------|-------|-----------|
| `sanitize_filename()` | ~100 | Sanitiza títulos para nombres de archivo |
| `clean_old_chat_files()` | ~125 | Elimina archivos duplicados con mismo UUID al guardar chat |
| `migrate_chats_in_dir()` | ~150 | Migración recursiva de `<uuid>.json` → `<title>-<uuid>.json` |
| `migrate_chats()` | ~196 | Migración completa: chats, prompts.json, local_projects.json |
| `looks_like_uuid_stem()` | ~140 | Detecta si un nombre de archivo parece UUID |

**Cambios en `get_chats()`**:
- `HashSet<String>` para deduplicar por `id`
- Salta archivos `.bak`
- Incluye campo `username` en la respuesta para que el admin vea de quién es cada chat

**Cambios en `chat_endpoint()`**:
- Llama a `clean_old_chat_files()` antes de guardar para eliminar duplicados

### `client_protocol.rs` — Protocolo Cliente-Servidor

Define los mensajes que el servidor y los clientes (Electron, Capacitor) intercambian:

```rust
pub enum ClientAction {
    ReadFile, WriteFile, ExecutePowerShell, ListDirectory,
    FileExists, FileMetadata, GitOperation, CargoOperation, SearchCode,
}

pub struct ClientRequest { pub request_id, pub action, pub params, pub timestamp }
pub struct ClientResponse { pub request_id, pub status, pub result, pub error }
pub struct ConnectedClient { pub client_id, pub username, pub connected_at, pub last_heartbeat, pub host_info }
```

### `electron/main.js` — Cliente Electron (v3.0)

Implementa el mismo protocolo que el viejo `client/src/main.rs` pero en Node.js:

```javascript
// Loop del cliente
async function startPolling() {
    setInterval(async () => {
        const resp = await apiCall('/api/client/poll', 'POST', { client_id, token });
        for (const req of resp.pending_requests) {
            const response = executeRequest(req);  // ejecuta localmente
            await apiCall('/api/client/response', 'POST', { client_id, token, response });
        }
    }, 2000);
}
```

**Ejecutores**:

| Acción | Implementación Node.js |
|--------|----------------------|
| `read_file` | `fs.readFileSync(path, 'utf-8')` + slicing por líneas |
| `write_file` | `fs.writeFileSync(path, content)` + SHA256 |
| `execute_powershell` | `child_process.execSync('powershell ...')` |
| `list_directory` | `fs.readdirSync` recursivo con profundidad máxima 10 |
| `git_operation` | `child_process.execSync('git ...')` |
| `cargo_operation` | `child_process.execSync('cargo ...')` con timeout 5min |
| `search_code` | Búsqueda textual con `fs.readFileSync` en archivos matching pattern |

### `capacitor/capacitor.config.ts` — App Android (v3.0)

```typescript
const config: CapacitorConfig = {
  appId: 'com.iaf.app',
  appName: 'IAF',
  webDir: '../public',         // Usa la UI web existente
  server: {
    androidScheme: 'https',
    cleartext: true,           // Permite HTTP en desarrollo
  },
  plugins: {
    Filesystem: {},            // Operaciones de archivos en Android
    Browser: {},               // Chrome Custom Tabs
    Network: {},               // Monitoreo de conectividad
  },
};
```

---

## 🔄 Flujo de Datos

### Cliente Electron: Conexión y ejecución

```
Electron inicia → main.js::createWindow()
  → BrowserWindow carga http://127.0.0.1:8080
  → Usuario hace login en la UI
  → UI llama a window.iafClient.setCredentials({ username, token })
  → preload.js reenvía vía IPC a main.js
  → main.js::connectToServer()
    → POST /api/client/connect { username, token, host_info }
    → Servidor registra ConnectedClient
  → main.js::startPolling()
    → Cada 2s: POST /api/client/poll { client_id, token }
    → Servidor retorna pending_requests[]
    → main.js::executeRequest(req) ejecuta localmente
    → POST /api/client/response { client_id, token, response }
```

### Cliente Capacitor: Comandos vía PC

```
Android app abre → WebView carga UI desde public/ o servidor
  → Usuario envía mensaje al agente
  → Agente quiere ejecutar PowerShell
  → Servidor verifica: ¿usuario admin?
    → SÍ: servidor ejecuta directamente
    → NO: servidor encola ClientRequest para el client_id del Electron
  → Electron (en la PC del usuario) hace poll, ejecuta, responde
  → Servidor retorna resultado al agente
  → Agente continúa
```

---

## 🧪 Cómo Extender el Sistema

### Agregar un nuevo comando al cliente Electron

1. Agregar el handler en `electron/main.js` en el `switch` de `executeRequest()`
2. Agregar la variante en `ClientAction` en `src/client_protocol.rs`
3. Agregar la tool definition en `src/agent.rs`

### Agregar un nuevo plugin de Capacitor

1. `cd capacitor && npm install @capacitor/nuevo-plugin`
2. Agregar la config en `capacitor.config.ts`
3. Usar el plugin desde `public/app.js`

---

## ⚠️ Problemas Conocidos

Ver `MEMORIES.md` para la lista completa.

- BUG-028 (SOLUCIONADO v3.0): Conversaciones duplicadas con mismo UUID y diferente título
- El validador de JS confunde variables locales duplicadas entre funciones distintas (falsos positivos)
- Google scraping es frágil (cambia markup frecuentemente)
