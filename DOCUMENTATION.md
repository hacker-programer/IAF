# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo en Rust + Axum + DeepSeek API.
> Servidor HTTP que orquesta un agente de desarrollo de software con herramientas.

---

## 📁 Estructura de Archivos Fuente

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | 951 | Servidor HTTP (Axum), endpoints REST, inicialización |
| `src/agent.rs` | 1940 | Bucle principal del agente, herramientas, loop de ejecución |
| `src/state.rs` | 254 | Estructuras de datos compartidas (AppState, ChatSession, etc.) |
| `src/validator.rs` | 230 | Validación post-escritura (detección de duplicados, delimitadores) |
| `src/scraper.rs` | 80 | Búsqueda en Google vía scraping + limpieza HTML |
| `src/desktop.rs` | 165 | Control de mouse/teclado (rdev), lanzamiento de ejecutables |
| `prompts/default_system_prompt.txt` | 505 | System prompt global del agente (reglas, técnicas de optimización) |

---

## 🧩 Estructuras de Datos Principales (`src/state.rs`)

| Estructura | Línea | Descripción |
|-----------|-------|-------------|
| `Project` | ~8 | `name: String, path: String, is_local: bool` — Proyecto registrado |
| `PromptConfig` | ~14 | `global_default, global_current: String, projects: HashMap<String, String>` |
| `ChatMessage` | ~21 | `role: String, content: String, timestamp: u64` |
| `ChatSession` | ~27 | `id, title, messages: Vec<ChatMessage>, project_name, steps` |
| `AuditStep` | ~35 | `step_type, title, detail, timestamp` — Paso de auditoría |
| `ActiveAgentStatus` | ~42 | `running, interrupted, esperando_respuesta_usuario, steps, current_session_id` |
| `ContextEntry` | ~55 | `id, entry_type, summary, full_content, created_at` |
| `CaptchaRequest` | ~62 | `id, sitekey, url, solved_content` |
| `ProcessRegistry` | ~73 | `spawned: Arc<Mutex<HashSet<u32>>>, server_pid: u32` — Registro seguro de PIDs |
| `AppState` | ~230 | Estado global: `config_path, prompts, projects, base_workspace, active_agent, process_registry, image_store, context_store` |

### Funciones clave en `state.rs`:

| Función | Línea | Descripción |
|---------|-------|-------------|
| `ProcessRegistry::new()` | ~95 | Crea registro con PID del servidor cacheado |
| `ProcessRegistry::register(pid)` | ~101 | Registra un PID como spawnado por el agente |
| `ProcessRegistry::safe_kill(pid)` | ~107 | Mata un proceso con 3 pasos de validación: (1) en registro, (2) parent PID == server PID, (3) taskkill |
| `ProcessRegistry::kill_all()` | ~176 | Mata todos los procesos registrados que sigan siendo hijos |
| `ProcessRegistry::reap()` | ~193 | Limpia procesos ya terminados del registro |
| `get_parent_pid(pid)` | ~210 | Obtiene ParentProcessId vía PowerShell `Get-Process` |

---

## 🧠 Bucle del Agente (`src/agent.rs`)

### Constantes
- `DEEPSEEK_API_URL`: `"https://api.deepseek.com/v1/chat/completions"` (línea ~11)

### Función principal: `run_agent_loop()` (línea ~13)
Flujo:
1. Carga system prompt global + local del proyecto
2. Añade directiva DOCUMENTATION.md y nota de contexto
3. Construye array `messages` con historial de chat + memoria de ejecución reciente
4. Define `tools` (16 herramientas en JSON Schema)
5. Loop principal:
   - Verifica interrupción
   - `compress_active_messages_if_needed()` — comprime contexto si >500K chars
   - `sanitize_messages_for_api()` — sana tool messages huérfanos
   - Llama a DeepSeek API con reintentos (hasta 3)
   - Procesa tool_calls: ejecuta cada herramienta, trunca resultados >25K chars
   - Si `final_message` es Some, retorna Ok

### Herramientas implementadas (handlers en el match de `tool_result`):

| Herramienta | Línea aprox. | Descripción |
|------------|-------------|-------------|
| `search_google` | ~570 | Llama a `perform_search()`, maneja CAPTCHA |
| `read_file` | ~590 | Lee archivo con/sin rango de líneas |
| `write_file_with_commit` | ~620 | Bloque `'write_handler`: sincroniza git (pull), escribe, add, commit, push. Con autocuración git. |
| `execute_powershell` | ~730 | Ejecuta comandos PowerShell con sanitización de seguridad (bloquea taskkill/Stop-Process). Spawnea procesos largos. |
| `search_code` | ~810 | `search_code_in_project()` → `semantic_code_search()` |
| `kill_process` | ~820 | `ProcessRegistry::safe_kill()` |
| `fork_and_clone_repo` | ~830 | `gh repo fork --clone` |
| `read_url` | ~850 | HTTP GET + `scraper_clean_tags()` |
| `check_github_cli` | ~870 | `gh` CLI genérico |
| `notificar_usuario` | ~890 | Pausa el agente para pregunta o envía notificación |
| `finalizar_tarea` | ~960 | Asigna `final_message`, mata procesos hijo |
| `image_fetch` | ~970 | Descarga imagen, guarda en `assets/images/` |
| `image_view` | ~1020 | Codifica en Base64, analiza con Qwen2.5-VL vía OpenRouter |
| `image_release` | ~1100 | Elimina marcadores de imagen del contexto |
| `git_resolve_divergence` | ~1140 | `keep_local` (push --force), `keep_remote` (reset --hard), `merge_both` |
| `analyze_images` | ~1180 | Análisis multimodal con Qwen2.5-VL vía OpenRouter |

### Funciones auxiliares:

| Función | Línea | Descripción |
|---------|-------|-------------|
| `save_chat_steps_to_disk()` | 1487 | Persiste pasos de auditoría en archivo JSON del chat |
| `get_project_path()` | 1501 | Resuelve ruta de proyecto por nombre |
| `discover_projects()` | 1509 | Escanea `base_workspace` en busca de proyectos |
| `search_code_in_project()` | 1533 | Wrapper que llama a `semantic_code_search()` |
| `semantic_code_search()` | 1537 | Búsqueda local de texto (NO VoyageAI). Puntúa por coincidencia exacta + keywords. |
| `safe_truncate()` | ~1580 | Trunca string en boundary UTF-8 seguro |
| `truncate_old_tool_responses()` | ~1590 | Trunca tool responses antiguos (>3 iteraciones) a 2000 chars |
| `compress_active_messages_if_needed()` | 1662 | Si contexto >500K chars, comprime con DeepSeek V4 Flash |
| `parse_shell_args()` | 1847 | Tokenizador de shell respetando comillas |
| `play_error_beep()` | 1872 | Beep del sistema (Windows: `[System.Console]::Beep`) |
| `sanitize_messages_for_api()` | 1886 | Corrige tool messages huérfanos (sin tool_call_id padre) |

---

## 🌐 Servidor HTTP (`src/main.rs`)

### Endpoints REST:

| Ruta | Método | Handler | Línea | Descripción |
|------|--------|---------|-------|-------------|
| `/api/projects` | GET | `get_projects` | ~870 | Lista proyectos |
| `/api/projects/fork` | POST | `fork_project` | ~880 | Fork + clone |
| `/api/projects/local` | POST | `add_local_project` | ~57 | Añade proyecto local |
| `/api/prompts` | GET/POST | `get_prompts`/`save_prompts` | ~900 | CRUD de prompts |
| `/api/prompts/reset` | POST | `reset_global_prompt` | ~920 | Restaura prompt default |
| `/api/chat` | POST | `chat_endpoint` | ~360 | **Principal**: inicia/continúa chat del agente |
| `/api/chats` | GET | `get_chats` | ~85 | Lista sesiones de chat |
| `/api/chats/:id` | GET | `get_chat_session` | ~100 | Obtiene sesión específica |
| `/api/chats/:id/summarize_steps` | POST | `summarize_chat_steps` | ~930 | Resume pasos de auditoría |
| `/api/agent/status` | GET | `get_agent_status` | ~110 | Estado del agente |
| `/api/agent/interrupt` | POST | `interrupt_agent` | ~120 | Interrumpe ejecución |
| `/api/agent/responder` | POST | `respond_to_agent` | ~140 | Responde pregunta del agente |
| `/api/agent/aprobar_plan` | POST | `approve_agent_plan` | ~170 | Aprueba/rechaza plan |
| `/api/prompts/refine` | POST | `refine_prompt_endpoint` | ~200 | Refina prompt con IA |
| `/api/captcha/status` | GET | `captcha_status` | ~750 | Estado de CAPTCHA |
| `/api/captcha/solve` | POST | `captcha_solve` | ~760 | Resuelve CAPTCHA |
| `/api/desktop/move` | POST | `move_mouse_handler` | ~780 | Mueve mouse |
| `/api/desktop/click` | POST | `click_handler` | ~790 | Click mouse |
| `/api/desktop/type` | POST | `type_text_handler` | ~800 | Escribe texto |
| `/api/desktop/launch` | POST | `launch_handler` | ~810 | Lanza ejecutable |

### Flujo de `chat_endpoint` (línea ~360):
1. Determina/genera session_id
2. Carga/crea ChatSession, genera título con DeepSeek V4 Flash
3. Guarda mensaje del usuario
4. Cancela agente anterior si existe
5. Configura ActiveAgentStatus (running=true, inyecta steps previos)
6. Spawnea `run_agent_loop()` en tarea Tokio con manejo de pánico
7. Al finalizar, guarda respuesta + auditoría en ChatSession

---

## ✅ Validador (`src/validator.rs`)

### Función principal: `validate_file_after_write()` (línea ~46)
- **Detección de líneas duplicadas consecutivas** (`detect_duplicate_lines`, línea ~100)
  - Ignora líneas estructurales: `}`, `)`, `]`, `//`, `};`, `});`
- **Verificación de delimitadores balanceados** (`check_balanced_delimiters`, línea ~140)
  - Stack-based: `{}`, `()`, `[]`
- **Verificación de sintaxis por lenguaje**:
  - `.rs`: `check_rust_common_errors()` — detecta bloques `unsafe`
  - `.js`: `node --check`

---

## 🔍 Scraper (`src/scraper.rs`)

- `perform_search()`: Busca en Google, detecta CAPTCHA, extrae títulos `<h3>`
- `scraper_clean_tags()`: Limpia tags HTML con regex precompilada (`OnceLock<Regex>`)

---

## 🖥️ Desktop Controller (`src/desktop.rs`)

- `DesktopController`: `children: Mutex<Vec<Child>>`
- `move_mouse(x, y)`: `rdev::simulate(MouseMove)`
- `click(button)`: `rdev::simulate(ButtonPress/Release)`
- `type_text(text)`: LUT `char_to_key_map()` precomputada con `OnceLock`
- `launch_executable(path)`: `Command::new(path).spawn()`

---

## 🐛 Bugs Conocidos (ver MEMORIES.md para detalles)

1. **CRÍTICO**: Mensaje "TRUNCADO POR EL SISTEMA" confunde al agente → reversión destructiva
2. **ALTO**: Código duplicado por ediciones parciales con start_line/end_line
3. **ALTO**: validator.rs no ejecuta `cargo check` post-escritura
4. **MEDIO**: Doble `play_error_beep()` en write_handler
5. **MEDIO**: Doble `discover_projects()` en main.rs
6. **MEDIO**: Comentario "SANITIZACIÓN DE SEGURIDAD" duplicado
7. **BAJO**: Mojibake UTF-8 en strings literales del código fuente
