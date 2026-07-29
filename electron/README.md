# IAF Electron Client

Cliente desktop para IAF construido con Electron. Reemplaza al cliente Rust (`client/`).

## Arquitectura

```
┌──────────────────────────────────────────────────┐
│              IAF Electron Client                  │
│                                                   │
│  ┌─────────────────────┐  ┌────────────────────┐ │
│  │   Renderer (UI)     │  │   Main Process     │ │
│  │                     │  │                    │ │
│  │  Carga la UI web    │  │  • Conecta al      │ │
│  │  desde el servidor  │  │    servidor IAF    │ │
│  │  IAF (HTTP/HTTPS)   │  │  • Poll cada 2s    │ │
│  │                     │  │  • Ejecuta:        │ │
│  │  Igual que el       │  │    - PowerShell    │ │
│  │  navegador pero     │  │    - Git           │ │
│  │  con API del        │  │    - Cargo         │ │
│  │  cliente expuesta   │  │    - Archivos      │ │
│  │  vía preload.js     │  │  • Heartbeat 30s   │ │
│  └─────────────────────┘  └────────────────────┘ │
│               ↕ IPC (contextBridge)               │
└──────────────────────────────────────────────────┘
                        ↕ HTTP
┌──────────────────────────────────────────────────┐
│              IAF Server (Rust/Axum)               │
│  Puerto 80 (admin local) + Puerto 8080 (auth)     │
└──────────────────────────────────────────────────┘
```

## Instalación

```powershell
cd electron
npm install
```

## Uso

```powershell
# Iniciar en modo desarrollo
npm start

# Iniciar con DevTools abiertas
npm start -- --dev
```

El cliente Electron:
1. Abre una ventana cargando la UI web desde `http://127.0.0.1:8080`
2. Al hacer login en la UI, las credenciales se pasan al main process vía IPC
3. El main process se conecta al servidor (`POST /api/client/connect`)
4. Hace poll cada 2 segundos (`POST /api/client/poll`)
5. Ejecuta comandos localmente y envía resultados (`POST /api/client/response`)

## Build (empaquetar como .exe)

```powershell
# Build para Windows
npm run build

# Solo empaquetar (sin instalador)
npm run pack
```

El instalador se genera en `electron/dist/`.

## Integración con Android

El cliente Electron en la PC es quien ejecuta comandos solicitados desde la app Android. Cuando un usuario en Android envía un mensaje al agente, el servidor IAF encola las solicitudes de comandos para el `client_id` del Electron conectado a esa cuenta.

## Credenciales

Las credenciales se guardan en `%APPDATA%/iaf-electron/config.json`:
```json
{
    "serverUrl": "http://127.0.0.1:8080",
    "username": "mi_usuario",
    "token": "iaf_abc123..."
}
```

## Comandos soportados

| Acción | Implementación |
|--------|---------------|
| `read_file` | `fs.readFileSync` |
| `write_file` | `fs.writeFileSync` + SHA256 |
| `execute_powershell` | `child_process.execSync('powershell ...')` |
| `list_directory` | `fs.readdirSync` recursivo |
| `file_exists` | `fs.existsSync` |
| `file_metadata` | `fs.statSync` |
| `git_operation` | `child_process.execSync('git ...')` |
| `cargo_operation` | `child_process.execSync('cargo ...')` |
| `search_code` | `fs.readFileSync` + búsqueda textual |

## Seguridad

- El renderer (UI web) NO tiene acceso directo a Node.js
- Toda comunicación es vía `contextBridge` (preload.js)
- Solo se exponen 3 funciones: `executeLocal`, `setCredentials`, `getStatus`
- Los comandos se ejecutan con los permisos del usuario del sistema operativo
