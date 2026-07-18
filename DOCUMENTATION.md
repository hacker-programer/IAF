# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v2.4

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos y cliente de ejecución remota.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~1950 | Servidor HTTP doble puerto, endpoints REST, CAPTCHA, legacy routes, migración, scripts, system prompts, ciclos |
| `src/auth.rs` | ~947 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos booleanos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~571 | AppState, CicleState/CiclePhase, CaptchaRequest, rutas de guardado de prompts/ciclos, ToolResultStore, SubAgentManager |
| `src/study.rs` | ~570 | Motor de estudio: perfiles, knowledge base, hipótesis, engagement |
| `src/sync.rs` | ~280 | Sincronización de proyectos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor para ejecución remota |
| `src/agent.rs` | ~2300 | Bucle principal del agente, 26 herramientas (incluye no_sync, reportar_fallo, MiniMax M3) |
| `src/validator.rs` | ~508 | Validación post-escritura (líneas duplicadas, delimitadores, errores comunes Rust) |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite (Google bloquea scrapers) |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos (máx 8, permisos por Patrón Composite) |
| `src/desktop.rs` | ~165 | Control de mouse/teclado (rdev) |
| `scripts/generate_keys.ps1` | ~105 | Genera par de claves Ed25519 via API y las guarda como .pem |
| `scripts/sign_nonce.ps1` | ~110 | Firma un nonce con clave privada para autenticación admin |
| `public/index.html` | ~258 | Frontend web con login dual, admin panel, gestión de usuarios |
| `public/app.js` | ~778 | Lógica del frontend: auth, admin, scripts, keygen, .pem upload |
| `client/Cargo.toml` | 15 | Cliente binario independiente |
| `client/src/main.rs` | ~350 | Ejecutor local (files, PowerShell, git, cargo) |
| `tests/integration_tests.rs` | ~500 | Tests de integración, aceptación y regresión (42 tests) |

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

### Scripts PowerShell para Autenticación Admin

| Script | Función | Descarga |
|--------|---------|----------|
| `generate_keys.ps1` | Genera par de claves y guarda .pem | `GET /api/scripts/generate_keys` |
| `sign_nonce.ps1` | Firma un nonce con clave privada | `GET /api/scripts/sign_nonce` |

### Flujo de Autenticación Admin (Nonce)
1. Admin ingresa username → `POST /api/auth/challenge` → recibe nonce base64
2. Admin firma el nonce: `.\scripts\sign_nonce.ps1 -Nonce "<nonce>"`
3. Admin envía firma → `POST /api/auth/verify` → recibe token de sesión

---

## 📂 Persistencia de Proyectos (v2.4)

### Archivo: `.config/local_projects.json`
- Se carga al iniciar el servidor.
- Se persiste automáticamente al agregar proyectos via `fork_project` o `add_local_project`.
- Se crea backup `.bak` (copy, no rename) en la primera migración.
- **Recovery automático**: si `.json` no existe pero `.bak` sí, se restaura desde backup.
- **Recovery desde memoria**: si no existe nada, los proyectos en memoria se guardan a disco.

---

## 🌐 Endpoints REST

### Auth
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `POST` | `/api/auth/login` | `login` | Login con username + password |
| `POST` | `/api/auth/challenge` | `challenge` | Genera nonce para admin |
| `POST` | `/api/auth/verify` | `verify` | Verifica firma Ed25519 |
| `GET` | `/api/auth/keygen` | `keygen` | Genera par de claves Ed25519 |
| `POST` | `/api/auth/logout` | `logout` | Cierra sesión |
| `POST` | `/api/auth/sign` | `sign_nonce` | Firma nonce (para scripts) |

### Scripts
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `GET` | `/api/scripts/:name` | `serve_script` | Descarga script .ps1 |

### Proyectos
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `GET` | `/api/projects` | `get_projects` | Listar proyectos |
| `POST` | `/api/projects/fork` | `fork_project` | Clonar repo via `gh` |
| `POST` | `/api/projects/local` | `add_local_project` | Agregar proyecto local |

### Admin (gestión de usuarios)
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `GET` | `/api/admin/users` | `admin_list_users` | Listar usuarios |
| `POST` | `/api/admin/users` | `admin_create_user` | Crear usuario (password o public_key) |
| `PUT` | `/api/admin/users/:username/limits` | `admin_update_limits` | Actualizar límites |
| `PUT` | `/api/admin/users/:username/access` | `admin_update_access` | Actualizar accesos |
| `PUT` | `/api/admin/users/:username/schedule` | `admin_update_schedule` | Actualizar horarios |
| `PUT` | `/api/admin/users/:username/password` | `admin_change_password` | Cambiar contraseña |
| `DELETE` | `/api/admin/users/:username` | `admin_delete_user` | Eliminar usuario |

### Chat
| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| `POST` | `/api/chat` | `chat_endpoint` | Enviar mensaje al agente |
| `GET` | `/api/chats` | `get_chats` | Listar historial de chats |
| `GET` | `/api/chats/:id` | `get_chat_session` | Obtener chat por ID |

---

## 🔧 Funcionamiento del Frontend (v2.4)

### Admin Panel
- **Gestionar Usuarios**: modal con tabla de usuarios + formulario de creación.
- **Crear Admin**: checkbox "Admin" activa modo public_key (oculta contraseña, muestra input de clave pública).
- **Subir .pem**: botón para cargar archivo .pem y extraer automáticamente la clave pública.
- **Generar Claves**: llama a `/api/auth/keygen`, muestra modal con claves y permite descargar .pem.
- **Descargar Scripts**: botones para bajar `generate_keys.ps1` y `sign_nonce.ps1`.

### Autenticación
- Puerto 80: acceso directo como `admin_local` (sin login).
- Puerto 8080: login obligatorio con pestañas "Usuario" (password) y "Admin Nonce" (firma Ed25519).
- El login por nonce guía al admin a usar `sign_nonce.ps1` para firmar el challenge.

---

## 📝 Notas de Versión v2.4

- **Fix**: `local_projects.json` ya no se renombra (se usa `copy` en vez de `rename`), preservando los proyectos entre reinicios.
- **Feat**: Recovery de proyectos desde `.bak` si el archivo principal no existe.
- **Feat**: Endpoint `GET /api/scripts/:name` para descargar scripts PowerShell.
- **Feat**: Frontend permite crear admins con clave pública (upload .pem o generar claves).
- **Feat**: Modal de generación de claves con descarga de .pem (privada y pública).
- **Feat**: Botones de descarga de scripts en el admin panel.