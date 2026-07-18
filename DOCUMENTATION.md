# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v2.1

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos y cliente de ejecución remota.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~1350 | Servidor HTTP doble puerto, endpoints REST, system prompts, ciclos |
| `src/auth.rs` | ~750 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos, horarios |
| `src/state.rs` | ~550 | AppState, CicleState, rutas de guardado de prompts/ciclos |
| `src/study.rs` | ~570 | Motor de estudio: perfiles, knowledge base, hipótesis, engagement |
| `src/sync.rs` | ~280 | Sincronización de proyectos entre amigos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor para ejecución remota |
| `src/agent.rs` | ~2300 | Bucle principal del agente, 26 herramientas (incluye no_sync, reportar_fallo) |
| `src/validator.rs` | ~508 | Validación post-escritura |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos |
| `src/desktop.rs` | ~165 | Control de mouse/teclado |
| `client/Cargo.toml` | 15 | Cliente binario independiente |
| `client/src/main.rs` | ~350 | Ejecutor local (files, PowerShell, git, cargo) |
| `prompts/study_system_prompt.txt` | 60 | System prompt del modo estudio |
| `tests/integration_tests.rs` | ~250 | Tests de integración y aceptación |

---

## 🔐 Autenticación Dual y Permisos

| Método | Usuarios | Endpoint |
|--------|----------|----------|
| **Username + Password (argon2id)** | Usuarios normales | `POST /api/auth/login` |
| **Ed25519 Challenge-Response** | Solo admins | `POST /api/auth/challenge` → `POST /api/auth/verify` |

### Estructura UserAccount (`src/auth.rs` línea ~40)

Campos principales:
- `username`, `public_key?`, `password_hash?`, `is_admin`
- **Permisos booleanos**: `admin`, `modo_programador`, `modo_estudio`, `editar_system_prompt_global`, `editar_system_prompt_local`
- `permissions[]` — lista de herramientas permitidas
- `limits` — UserLimits con activación, horarios y límites detallados

### UserLimits (`src/auth.rs` línea ~170)

```
activacion: bool,
peticiones_por_minuto: u32 (0 = ilimitado),
peticiones_por_hora: u32,
limite_iteraciones: u32,
limite_tokens_entrada: u64,
limite_tokens_salida: u64,
horarios: WeeklySchedule,
allowed_tools[], max_sub_agents, max_projects,
can_fork_repos, can_execute_powershell, can_write_files
```

### WeeklySchedule (`src/auth.rs` línea ~100)

HashMap días → Vec<(hora_inicio, hora_fin)>. Ejemplo: `{"lunes": [(9,10), (16,18)], "martes": [(9,12)]}`

---

## 🌐 Doble Puerto

| Puerto | Auth | Acceso |
|--------|------|--------|
| **80** | Ninguno (auto-admin) | Total. Ejecuta localmente. |
| **8080** | Requiere login | Según permisos. |

---

## 🔄 Ciclos del Modo Programación (`src/state.rs` CiclePhase)

| Ciclo | Fase | Descripción |
|-------|------|-------------|
| 1 | `Implementacion` | Implementar la tarea completa |
| 2 | `Optimizacion` | Optimización exhaustiva extrema |
| 3 | `BusquedaBugs` | Búsqueda exhaustiva de bugs |
| 4 | `Reduccion` | Eliminar archivos y código redundante |
| 5 | `SegundaBusquedaBugs` | Segunda búsqueda de bugs |
| 6 | `Terminar` | Tarea finalizada |

Estado guardado en `.config/data/<username>/<project>/cicle.json`

---

## 🤖 Herramientas del Agente (`src/agent.rs`)

26 herramientas totales:

| Herramienta | Descripción |
|-------------|-------------|
| `search_google` | Búsqueda web (DuckDuckGo Lite) |
| `read_file` | Leer archivo con rango opcional |
| `write_file_with_commit` | Escribir archivo + commit GitHub |
| `execute_powershell` | Ejecutar comandos PowerShell |
| `search_code` | Búsqueda local por palabras clave |
| `fork_and_clone_repo` | Forkear/clonar repos |
| `read_url` | Extraer texto de URL |
| `check_github_cli` | Comandos GitHub CLI |
| `notificar_usuario` | Comunicación con usuario |
| `finalizar_tarea` | Terminar ejecución |
| `image_fetch` / `image_view` / `image_release` | Manejo de imágenes |
| `git_resolve_divergence` | Resolver conflictos git |
| **`analyze_images`** | Análisis multimodal con **MiniMax M3** (OpenRouter, providers: DeepInfra) |
| `kill_process` | Matar procesos por PID |
| `fetch_tool_result` / `release_tool_result` | Paginación de resultados |
| `spawn_sub_agent` / `check_sub_agent` / `kill_sub_agent` | Sub-agentes (máx 8) |
| **`no_sync`** | Configurar sincronización selectiva (Patrón Composite) |
| **`reportar_fallo`** | Reportar fallos internos de IAF |

---

## 📡 API Endpoints (completos)

### Auth
| Método | Ruta | Descripción |
|--------|------|-------------|
| `POST` | `/api/auth/login` | Login con contraseña |
| `POST` | `/api/auth/challenge` | Obtener nonce para admin |
| `POST` | `/api/auth/verify` | Verificar firma Ed25519 |
| `GET` | `/api/auth/keygen` | Generar par de claves |
| `POST` | `/api/auth/logout` | Cerrar sesión |
| `POST` | `/api/auth/sign` | Firmar nonce (helper) |
| `GET` | `/api/client/check` | Verificar cliente instalado |

### Admin
| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/admin/users` | Listar usuarios |
| `POST` | `/api/admin/users` | Crear usuario |
| `PUT` | `/api/admin/users/:username/limits` | Actualizar límites |
| `PUT` | `/api/admin/users/:username/access` | Actualizar accesos (modo_estudio, modo_programador, etc.) |
| `PUT` | `/api/admin/users/:username/schedule` | Actualizar horarios |
| `PUT` | `/api/admin/users/:username/password` | Cambiar contraseña |
| `DELETE` | `/api/admin/users/:username` | Eliminar usuario |

### System Prompts
| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/prompts/global` | Obtener system prompt global del usuario |
| `POST` | `/api/prompts/global` | Guardar system prompt global |
| `POST` | `/api/prompts/global/reset` | Restaurar al default |
| `GET` | `/api/prompts/local?project=X` | Obtener system prompt local |
| `POST` | `/api/prompts/local` | Guardar system prompt local |

### Ciclos
| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/cicle/:project_name` | Obtener estado del ciclo |
| `POST` | `/api/cicle/:project_name/advance` | Avanzar al siguiente ciclo |

### Study, Sync, Client, Chats — sin cambios

---

## 📂 Rutas de Guardado

```
.config/
├── chats/<username>/<titulo>-<uuid>.json
├── data/<username>/
│   ├── profile.json
│   ├── learnings.json
│   ├── teachingMethod.json
│   ├── globalPrompt.json
│   └── <project>/
│       ├── localPrompt.json
│       └── cicle.json
└── users.json
```

---

## 🧪 Tests

```bash
# Unit tests
cargo test --lib

# Integration tests (requieren servidor)
cargo test --test integration_tests -- --ignored

# Con target dir alternativo
$env:CARGO_TARGET_DIR = "C:\Users\Fa\AppData\Local\Temp\cargo-target"
cargo check
```
