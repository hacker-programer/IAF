# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo en Rust + Axum + DeepSeek API.
> Servidor HTTP que orquesta un agente de desarrollo de software con herramientas,
> sub-agentes paralelos y almacenamiento de resultados con IDs.

---

## 📁 Estructura de Archivos Fuente

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | 976 | Servidor HTTP (Axum), endpoints REST, inicialización |
| `src/agent.rs` | 2088 | Bucle principal del agente, herramientas, loop de ejecución |
| `src/state.rs` | 647 | Estructuras de datos compartidas (AppState, ToolResultStore, SubAgentManager) |
| `src/validator.rs` | 508 | Validación post-escritura (duplicados, delimitadores, contexto impl) |
| `src/scraper.rs` | 170 | Búsqueda web vía DuckDuckGo Lite + fallback Google |
| `src/sub_agent.rs` | 520 | Ejecución paralela de sub-agentes con restricciones de path |
| `src/desktop.rs` | 165 | Control de mouse/teclado (rdev), lanzamiento de ejecutables |
| `prompts/default_system_prompt.txt` | 517 | System prompt global del agente (reglas, técnicas de optimización) |

---

## 🧩 Estructuras de Datos Principales (`src/state.rs`)

| Estructura | Línea aprox. | Descripción |
|-----------|-------------|-------------|
| `Project` | ~13 | `name: String, path: String, is_local: bool` — Proyecto registrado |
| `PromptConfig` | ~19 | `global_default, global_current: String, projects: HashMap<String, String>` |
| `ChatMessage` | ~33 | `role: String, content: String, timestamp: u64` |
| `ChatSession` | ~39 | `id, title, messages: Vec<ChatMessage>, project_name, steps` |
| `AuditStep` | ~49 | `step_type, title, detail, timestamp` — Paso de auditoría |
| `ActiveAgentStatus` | ~56 | `running, interrupted, esperando_respuesta_usuario, steps, current_session_id` |
| `ContextEntry` | ~72 | `id, entry_type, summary, full_content, created_at` |
| `CaptchaRequest` | ~80 | `id, sitekey, url, solved_content` |
| **`ToolResultStore`** | ~97 | **Almacena resultados completos de herramientas con IDs. Reemplaza el truncado arbitrario.** |
| `StoredToolResult` | ~103 | `call_id, tool_name, full_content, stored_at` — Una entrada del store |
| **`SubAgentHandle`** | ~192 | Handle para gestionar un sub-agente (id, task, status, abort_handle) |
| **`SubAgentStatus`** | ~184 | Enum: `Running, Completed, Failed(String), Cancelled` |
| **`SubAgentManager`** | ~218 | Gestor de sub-agentes paralelos con límite dinámico según hardware |
| `ProcessRegistry` | ~465 | `spawned: Arc<Mutex<HashSet<u32>>>, server_pid: u32` — Registro seguro de PIDs |
| `AppState` | ~624 | Estado global con todos los campos anteriores |

### ToolResultStore (NUEVO — 2026-07-08)

| Método | Línea | Descripción |
|--------|-------|-------------|
| `store(call_id, tool_name, full_content) -> String` | ~119 | Guarda resultado completo. Si >3000 chars, devuelve resumen + ID + instrucciones de paginación |
| `fetch_page(call_id, page, page_size) -> Option<String>` | ~162 | Recupera una página del resultado por ID |
| `release(call_id) -> bool` | ~199 | Libera un resultado de memoria por ID |
| `reap_old(max_age_secs) -> usize` | ~205 | Limpia resultados más antiguos que N segundos |
| `len() -> usize` | ~217 | Cantidad de resultados almacenados |

### SubAgentManager (NUEVO — 2026-07-08)

| Método | Línea | Descripción |
|--------|-------|-------------|
| `new() -> Self` | ~231 | Detecta hardware para escalar dinámicamente paralelismo (1 en 2 cores, hasta 8 en 16+) |
| `register(id, task, project, paths, context, handle)` | ~248 | Registra un sub-agente como Running |
| `update_status(id, status, result)` | ~266 | Actualiza estado de un sub-agente |
| `cancel(id) -> bool` | ~275 | Cancela un sub-agente por ID |
| `cancel_all()` | ~291 | Cancela todos los sub-agentes activos |
| `running_count() -> usize` | ~306 | Sub-agentes actualmente en ejecución |
| `can_spawn() -> bool` | ~313 | true si hay capacidad para más sub-agentes |
| `status_summary() -> String` | ~318 | Reporte formateado del estado de todos los sub-agentes |
| `reap_old(max_age_secs) -> usize` | ~427 | Limpia sub-agentes terminados antiguos |

---

## 🧠 Bucle del Agente (`src/agent.rs`)

### Constantes
- `DEEPSEEK_API_URL`: `"https://api.deepseek.com/v1/chat/completions"` (línea ~15)

### Función principal: `run_agent_loop()` (línea ~18)
Flujo:
1. Carga system prompt global + local del proyecto
2. Añade directiva DOCUMENTATION.md y nota de contexto
3. Construye array `messages` con historial de chat + memoria de ejecución reciente
4. Define `tools` (16 herramientas en JSON Schema + nuevas de sub-agentes)
5. Loop principal con compresión de contexto, sanitización y rate-limiting

### Herramientas implementadas:

| Herramienta | Descripción |
|------------|-------------|
| `search_google` | Búsqueda web vía **DuckDuckGo Lite** (primario) + Google (fallback) |
| `read_file` | Lee archivo con/sin rango de líneas |
| `write_file_with_commit` | Escribe archivo + git add/commit/push + validación post-escritura |
| `execute_powershell` | Ejecuta comandos PowerShell con sanitización de seguridad |
| `search_code` | Búsqueda local de palabras clave en código |
| `kill_process` | Mata procesos vía ProcessRegistry::safe_kill() |
| `fork_and_clone_repo` | `gh repo fork --clone` |
| `read_url` | HTTP GET + scraper_clean_tags() |
| `check_github_cli` | `gh` CLI genérico |
| `notificar_usuario` | Pausa el agente para pregunta o envía notificación |
| `finalizar_tarea` | Asigna final_message, mata procesos hijo |
| `image_fetch` / `image_view` / `image_release` | Manejo de imágenes |
| `analyze_images` | Análisis multimodal con Qwen2.5-VL |
| `git_resolve_divergence` | Resuelve divergencias git |

### Funciones exportadas:

| Función | Línea | Descripción |
|---------|-------|-------------|
| `run_agent_loop()` | ~18 | Bucle principal del agente |
| `discover_projects()` | ~1518 | Escanea base_workspace en busca de proyectos |
| **`search_code_in_project()`** | ~1612 | **Búsqueda local (ahora `pub` para sub_agent.rs)** |
| `semantic_code_search()` | ~1623 | Búsqueda por palabras clave con scoring |
| `save_chat_steps_to_disk()` | ~1498 | Persiste pasos de auditoría |

---

## 🔍 Sub-Agent System (`src/sub_agent.rs`)

| Función | Línea | Descripción |
|---------|-------|-------------|
| `spawn_sub_agent()` | ~38 | Spawnea un sub-agente con límite de paralelismo |
| `is_path_allowed()` | ~98 | Verifica restricciones de path |
| `run_sub_agent()` | ~135 | Ejecuta el loop del sub-agente (máx 15 iteraciones) |
| `build_sub_agent_tools()` | ~302 | Subconjunto restringido de herramientas |
| `execute_sub_agent_tool()` | ~385 | Ejecuta herramientas con verificación de path |

---

## 🔍 Scraper (`src/scraper.rs`)

| Función | Línea | Descripción |
|---------|-------|-------------|
| **`perform_search()`** | ~15 | **Usa DuckDuckGo Lite como fuente principal, Google como fallback** |
| `search_duckduckgo()` | ~42 | Búsqueda en lite.duckduckgo.com (HTML simple, sin CAPTCHA) |
| `search_google()` | ~112 | Fallback: Google (probablemente falle por bloqueo) |
| `scraper_clean_tags()` | ~168 | Limpia tags HTML con regex precomputada en OnceLock |

---

## ✅ Validador (`src/validator.rs`)

| Función | Línea | Descripción |
|---------|-------|-------------|
| `validate_file_after_write()` | ~55 | Orquesta todas las validaciones |
| `detect_duplicate_lines()` | ~112 | Detecta líneas consecutivas idénticas (ignora argumentos repetidos) |
| `check_balanced_delimiters()` | ~155 | Verifica balanceo de `{}`, `()`, `[]` |
| `check_rust_common_errors()` | ~198 | Detecta bloques `unsafe` |
| **`detect_duplicate_definitions()`** | ~218 | **Detecta definiciones duplicadas CON contexto impl (arreglado 2026-07-08)** |
| `extract_impl_struct_name()` | ~287 | Extrae nombre de struct de `impl` block |
| `extract_def_name_with_context()` | ~303 | Extrae nombre de definición con prefijo de impl |
| `detect_reasoning_injection()` | ~323 | Detecta texto de razonamiento sin comentar |

**Corrección de falsos positivos (2026-07-08):** `detect_duplicate_definitions` ahora trackea bloques `impl` para distinguir métodos de diferentes structs. `fn new()` en `impl ToolResultStore` no se confunde con `fn new()` en `impl SubAgentManager`.

---

## 🌐 Servidor HTTP (`src/main.rs`)

### Endpoints REST (principales):

| Ruta | Método | Handler | Descripción |
|------|--------|---------|-------------|
| `/api/projects` | GET | `get_projects` | Lista proyectos |
| `/api/projects/fork` | POST | `fork_project` | Fork + clone |
| `/api/projects/local` | POST | `add_local_project` | Añade proyecto local |
| `/api/prompts` | GET/POST | `get_prompts`/`save_prompts` | CRUD de prompts |
| `/api/prompts/reset` | POST | `reset_global_prompt` | Restaura prompt default |
| `/api/chat` | POST | `chat_endpoint` | Principal: inicia/continúa chat del agente |
| `/api/chats` | GET | `get_chats` | Lista sesiones de chat |
| `/api/chats/:id` | GET | `get_chat_session` | Obtiene sesión específica |
| `/api/chats/:id/summarize_steps` | POST | `summarize_chat_steps` | Resume pasos de auditoría |
| `/api/agent/status` | GET | `get_agent_status` | Estado del agente |
| `/api/agent/interrupt` | POST | `interrupt_agent` | Interrumpe ejecución |
| `/api/agent/responder` | POST | `respond_to_agent` | Responde pregunta del agente |
| `/api/agent/aprobar_plan` | POST | `approve_agent_plan` | Aprueba/rechaza plan |

---

## 🐛 Bugs Conocidos y Solucionados (MEMORIES.md)

Ver `MEMORIES.md` para el registro completo. Cambios recientes:

### [2026-07-08] Falsos positivos en validator.rs: definiciones duplicadas entre impl blocks
- **Estado**: CORREGIDO
- **Causa**: `detect_duplicate_definitions` no distinguía entre `impl ToolResultStore::fn new` y `impl SubAgentManager::fn new`
- **Solución**: Se agregó trackeo de contexto `impl` con `extract_impl_struct_name()` y `extract_def_name_with_context()`

### [2026-07-08] Google Search siempre fallaba
- **Estado**: CORREGIDO
- **Causa**: Google bloquea agresivamente scrapers
- **Solución**: `scraper.rs` ahora usa DuckDuckGo Lite como fuente principal, con Google como fallback

### [2026-07-08] Truncado arbitrario de tool results
- **Estado**: CORREGIDO
- **Causa**: Resultados grandes se truncaban a 25K chars, perdiendo información
- **Solución**: `ToolResultStore` con sistema de IDs y paginación. El agente decide cuándo liberar.

### [2026-07-08] Sin capacidad de paralelismo
- **Estado**: CORREGIDO
- **Causa**: Solo un agente secuencial
- **Solución**: `SubAgentManager` + `sub_agent.rs`. Sub-agentes con restricciones de path y contexto heredado.
