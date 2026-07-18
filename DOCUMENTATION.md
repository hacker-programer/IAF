# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v2.2

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos y cliente de ejecución remota.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~1350 | Servidor HTTP doble puerto, endpoints REST, system prompts, ciclos |
| `src/auth.rs` | ~750 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos booleanos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~560 | AppState, CicleState/CiclePhase, rutas de guardado de prompts/ciclos, AppState::load_global_prompt() |
| `src/study.rs` | ~570 | Motor de estudio: perfiles, knowledge base, hipótesis, engagement |
| `src/sync.rs` | ~280 | Sincronización de proyectos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor para ejecución remota |
| `src/agent.rs` | ~2300 | Bucle principal del agente, 26 herramientas (incluye no_sync, reportar_fallo, MiniMax M3) |
| `src/validator.rs` | ~508 | Validación post-escritura (líneas duplicadas, delimitadores, errores comunes Rust) |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite (Google bloquea scrapers) |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos (máx 8, permisos por Patrón Composite) |
| `src/desktop.rs` | ~165 | Control de mouse/teclado (rdev) |
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
- **Permisos booleanos explícitos**:
  - `admin` — Implica todos los demás. Permite gestionar usuarios.
  - `modo_programador` — Acceso al modo programación.
  - `modo_estudio` — Acceso al modo estudio.
  - `editar_system_prompt_global` — Editar system prompt global.
  - `editar_system_prompt_local` — Editar system prompts locales.
- `permissions[]` — lista de herramientas permitidas (strings)
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

`is_active_now()` determina si el usuario está en una franja horaria activa.

---

## 🌐 Doble Puerto

| Puerto | Auth | Acceso |
|--------|------|--------|
| **80** | Ninguno (auto-admin) | Total. `state.port_80 = true`. Ejecuta localmente. |
| **8080** | Requiere login | Según permisos del usuario. `state.port_80 = false`. |

---

## 🔄 Ciclos del Modo Programación (`src/state.rs` CiclePhase, línea ~54)

| Ciclo | Fase | Descripción |
|-------|------|-------------|
| 1 | `Implementacion` | Implementar la tarea completa (no finaliza hasta que esté TODO) |
| 2 | `Optimizacion` | Optimización exhaustiva extrema |
| 3 | `BusquedaBugs` | Búsqueda exhaustiva de bugs. Si hay, corrige y vuelve al ciclo 2 |
| 4 | `Reduccion` | Eliminar archivos y código redundante |
| 5 | `SegundaBusquedaBugs` | Segunda búsqueda de bugs. Si hay, corrige y vuelve al ciclo 2 |
| 6 | `Terminar` | Tarea finalizada |

### CicleState (`src/state.rs` línea ~88)
- `project_name`, `current_phase: CiclePhase`, `iteration_count`, `started_at`, `last_updated`
- Guardado en `.config/data/<username>/<project>/cicle.json`

---

## 🤖 Herramientas del Agente (`src/agent.rs`)

26 herramientas totales, definidas en el array `tools` (línea ~121) con handlers en el match `func_name` (línea ~609):

| Herramienta | Descripción |
|-------------|-------------|
| `search_google` | Búsqueda web (DuckDuckGo Lite como fallback de Google) |
| `read_file` | Leer archivo con rango opcional (start_line, end_line) |
| `write_file_with_commit` | Escribir archivo + commit automático en GitHub |
| `execute_powershell` | Ejecutar comandos PowerShell (con timer opcional) |
| `search_code` | Búsqueda local por palabras clave (no embeddings) |
| `fork_and_clone_repo` | Forkear/clonar repos via GitHub CLI |
| `read_url` | Extraer texto de URL pública |
| `check_github_cli` | Comandos GitHub CLI (gh) |
| `notificar_usuario` | Comunicación: tipo "informativo" o "pregunta" (pausa ejecución) |
| `finalizar_tarea` | Terminar ejecución con resumen |
| `image_fetch` | Descargar imagen, devuelve UUID |
| `image_view` | Mostrar imagen en contexto (Base64) |
| `image_release` | Liberar imagen del contexto |
| `git_resolve_divergence` | Resolver conflictos: keep_local/keep_remote/merge_both |
| **`analyze_images`** | Análisis multimodal con **MiniMax M3** via OpenRouter (providers: DeepInfra). Soporta imágenes, audio y video. |
| `kill_process` | Matar proceso por PID (solo procesos registrados) |
| `fetch_tool_result` | Paginación de resultados grandes |
| `release_tool_result` | Liberar resultado de herramienta |
| `spawn_sub_agent` | Crear sub-agente paralelo (máx 8) |
| `check_sub_agent` | Verificar estado de sub-agente |
| `kill_sub_agent` | Cancelar sub-agente |
| **`no_sync`** | Configurar sincronización selectiva (Patrón Composite: include/exclude patterns) |
| **`reportar_fallo`** | Reportar fallos internos de IAF (guardado en fallos_reportados.json) |

---

## 📡 API Endpoints (completos)

### Auth
| Método | Ruta | Descripción |
|--------|------|-------------|
| `POST` | `/api/auth/login` | Login con contraseña (argon2id) |
| `POST` | `/api/auth/challenge` | Obtener nonce para admin |
| `POST` | `/api/auth/verify` | Verificar firma Ed25519 |
| `GET` | `/api/auth/keygen` | Generar par de claves Ed25519 |
| `POST` | `/api/auth/logout` | Cerrar sesión |
| `POST` | `/api/auth/sign` | Firmar nonce (helper para scripts .ps1) |
| `GET` | `/api/client/check` | Verificar si el cliente está instalado |

### Admin
| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/admin/users` | Listar todos los usuarios |
| `POST` | `/api/admin/users` | Crear usuario (password o public_key) |
| `PUT` | `/api/admin/users/:username/limits` | Actualizar límites (UserLimits completo) |
| `PUT` | `/api/admin/users/:username/access` | Actualizar accesos (modo_estudio, modo_programador, editar_system_prompt_global, editar_system_prompt_local) |
| `PUT` | `/api/admin/users/:username/schedule` | Actualizar horarios semanales |
| `PUT` | `/api/admin/users/:username/password` | Cambiar contraseña |
| `DELETE` | `/api/admin/users/:username` | Eliminar usuario |

### System Prompts
| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/prompts/global` | Obtener system prompt global del usuario |
| `POST` | `/api/prompts/global` | Guardar system prompt global |
| `POST` | `/api/prompts/global/reset` | Restaurar global al default |
| `GET` | `/api/prompts/local/:project_name` | Obtener system prompt local de un proyecto |
| `POST` | `/api/prompts/local` | Guardar system prompt local |

### Ciclos
| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/cicles/:project_name` | Obtener estado actual del ciclo |
| `PUT` | `/api/cicles/:project_name` | Actualizar/avanzar ciclo |

### Chat
| Método | Ruta | Descripción |
|--------|------|-------------|
| `POST` | `/api/chat` | Enviar mensaje al agente |
| `GET` | `/api/chats` | Listar chats del usuario |
| `GET` | `/api/chats/:id` | Obtener sesión de chat específica |

### Study
| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/study/profile` | Obtener perfil de aprendizaje |
| `POST` | `/api/study/profile` | Guardar perfil |
| `GET` | `/api/study/knowledge` | Obtener knowledge base |
| `POST` | `/api/study/projects` | Crear proyecto de estudio |
| `GET` | `/api/study/projects` | Listar proyectos |
| `POST` | `/api/study/projects/:id/members` | Agregar miembro |
| `POST` | `/api/study/build-prompt` | Construir system prompt personalizado |

### Sync
| Método | Ruta | Descripción |
|--------|------|-------------|
| `POST` | `/api/sync/process` | Procesar manifiesto de sync |
| `POST` | `/api/sync/push` | Subir nueva versión de archivo |
| `GET` | `/api/sync/history/:project_id/*path` | Historial de versiones |

### Cliente
| Método | Ruta | Descripción |
|--------|------|-------------|
| `POST` | `/api/client/connect` | Registrar cliente |
| `POST` | `/api/client/heartbeat` | Heartbeat cada 30s |
| `POST` | `/api/client/poll` | Polling de trabajo pendiente |
| `POST` | `/api/client/response` | Enviar resultado de ejecución |

---

## 📂 Rutas de Guardado

```
.config/
├── chats/
│   ├── <titulo>-<uuid>.json              (admin / puerto 80)
│   └── <username>/<titulo>-<uuid>.json   (usuarios normales)
├── data/<username>/
│   ├── profile.json                      (UserLearningProfile)
│   ├── learnings.json                    (UserKnowledgeBase)
│   ├── teachingMethod.json               (fase y métodos de enseñanza)
│   ├── globalPrompt.json                 (system prompt global del usuario)
│   └── <project>/
│       ├── localPrompt.json              (system prompt local del proyecto)
│       └── cicle.json                    (CicleState)
├── users.json                            (UserStore)
└── prompts.json                          (PromptConfig global)
```

---

## 🔧 Detalles Técnicos Importantes

### MiniMax M3 (`src/agent.rs`)
- Reemplazó a Qwen2.5-VL para análisis multimodal.
- Llamadas via OpenRouter con providers: `{"order": ["DeepInfra"], "allow_fallbacks": true}`
- Usado en `analyze_images` (imágenes/video/audio) y en `image_view`.

### no_sync (`src/agent.rs` línea ~1623)
- Recibe `include_patterns` y `exclude_patterns` (arrays de strings).
- Devuelve un reporte de configuración.
- Diseñado para Patrón Composite de sincronización.

### reportar_fallo (`src/agent.rs` línea ~1641)
- Recibe `informe` (string) y `severidad` (baja/media/alta/critica).
- Guarda en `.config/fallos_reportados.json`.

### Ediciones parciales en write_file_with_commit
- **PROHIBIDO usar start_line/end_line**. Causan corrupción de archivos (delimitadores desbalanceados, líneas duplicadas).
- Siempre escribir el archivo COMPLETO.
- Verificar que no haya definiciones duplicadas antes de escribir.

---

## 🧪 Tests

```bash
# Compilar (target dir alternativo por permisos)
$env:CARGO_TARGET_DIR = "C:\Users\Fa\AppData\Local\Temp\cargo-target"
cargo check

# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration_tests -- --ignored
```
