# DOCUMENTATION.md - Proyecto IAF (Intelligent Agent Framework)

> **Última actualización**: Julio 2026 — Post-correcciones mayores
> **Versión**: 2.0

---

## Arquitectura General
IAF es un agente autónomo de desarrollo escrito en Rust (edición 2021) que usa la API de DeepSeek V4 como backend LLM. El agente tiene acceso a herramientas como búsqueda web, ejecución de comandos PowerShell, manipulación de archivos con GitHub, y gestión de imágenes.

---

## Archivos Fuente Principales

### `src/main.rs` (~950 líneas) ⬅️ Reducido de 1436
Punto de entrada del servidor web (Axum). Maneja rutas HTTP, sesiones de chat, y sirve la UI.

- `deepseek_key()` (línea ~30): API key desde variable de entorno `DEEPSEEK_API_KEY`
- `openrouter_key()` (línea ~40): API key desde `OPENROUTER_API_KEY`
- `DEFAULT_GLOBAL_SYSTEM_PROMPT` (línea ~47): Cargado vía `include_str!("../prompts/default_system_prompt.txt")` ⬅️ **EXTERNALIZADO**
- `main()` (línea ~700): Configura rutas Axum, carga prompts, inicia servidor en `0.0.0.0:8080`
- Llamadas a DeepSeek Flash para generación de títulos (líneas ~599, ~671)
- Manejo de sesiones de chat desde/hacia `.config/chats/*.json`
- `AppState` se inicializa con `ProcessRegistry` para gestión segura de procesos
- **Endpoint `/api/captcha/solve`** (línea ~600): Resuelve CAPTCHAs ⬅️ **AGREGADO**
- `mod validator` (línea ~22): Módulo de validación post-escritura ⬅️ **AGREGADO**
- `mod embeddings` ⬅️ **ELIMINADO**

### `src/agent.rs` (~1939 líneas)
Núcleo del agente. Contiene el bucle principal de ejecución y todas las herramientas.

- `DEEPSEEK_API_URL` (línea 12): `https://api.deepseek.com/v1/chat/completions`
- `run_agent_loop()` (línea 14): Bucle principal del agente
- `write_file_with_commit` (línea ~609): Herramienta para modificar archivos con commit automático
  - **PASO 0**: Verifica `git remote get-url origin`. Si no existe, intenta `gh repo create`. Si falla, aborta.
  - **ELIMINADO**: `git clean -fd` del flujo de autocuración
  - **AGREGADO (Julio 2026)**: Validación post-escritura vía `validator::validate_file_after_write()`
  - Usa labeled block `'write_handler` para evitar que errores terminen la sesión
- `execute_powershell` (línea ~824): Ejecuta comandos PowerShell con sanitización de seguridad
- `check_github_cli` (línea ~1043): Ejecuta comandos `gh` — **CORREGIDO**: ahora usa `.current_dir(&working_dir)` ⬅️
- `sanitize_messages_for_api()` (línea ~1717): Convierte mensajes `tool` huérfanos a `user`
- `compress_active_messages_if_needed()` (línea ~1553): Comprime contexto cuando excede 500K caracteres
- `truncate_old_tool_responses()` (línea ~1530): Trunca respuestas de assistant tras 3+ iteraciones
- `safe_truncate()` (línea ~1518): Trunca strings en límites de caracteres UTF-8 seguros
- `semantic_code_search()` (línea ~1428): Búsqueda de código LOCAL por palabras clave
- `save_chat_steps_to_disk()` (línea ~1385): Persiste pasos de auditoría
- `get_project_path()` (línea ~1399): Resuelve ruta de proyecto por nombre
- `discover_projects()` (línea ~1504): Escanea `base_workspace` — **CORREGIDO**: usa `retain(|p| p.is_local)` ⬅️
- `play_error_beep()` (línea ~1773): Emite beep de error del sistema
- `notificar_usuario` (línea ~1030): Handler para preguntas e informativos
- `finalizar_tarea` (línea ~1098): Limpia procesos hijo y termina ejecución

### `src/state.rs`
Estructuras de estado compartido:

- `Project` (línea ~9): Nombre, ruta, flag `is_local`
- `PromptConfig` (línea ~16): Prompts global y por proyecto
- `ChatMessage` (línea ~23): Rol, contenido, timestamp
- `ChatSession` (línea ~30): ID, título, mensajes, proyecto, pasos de auditoría
- `AuditStep` (línea ~39): Tipo, título, detalle, timestamp
- `ActiveAgentStatus` (línea ~47): Estado completo del agente
- `ContextEntry` (línea ~61): Entrada de contexto para RAG
- `CaptchaRequest` (línea ~69): ID, sitekey, URL, contenido resuelto
- `ProcessRegistry` (línea ~84): Registro seguro de PIDs
  - `register()`: Registra un PID spawnado por el agente
  - `safe_kill()`: Verifica parent PID antes de matar — **CORREGIDO**: usa `Get-CimInstance` en vez de `wmic` ⬅️
  - `kill_all()`: Mata todos los procesos registrados
  - `reap()`: Limpia procesos ya terminados
- `AppState` (línea ~212): Estado global

### `src/validator.rs` ⬅️ **NUEVO (Julio 2026)**
Módulo de validación post-escritura para detectar errores del agente:

- `ValidationResult` (línea ~18): Estructura de resultado con warnings y errors
- `validate_file_after_write(path)` (línea ~40): Función principal — ejecuta todas las validaciones
- `check_duplicate_lines(content)` (línea ~70): Detecta líneas duplicadas consecutivas
- `check_balanced_delimiters(content)` (línea ~95): Verifica `{}`, `()`, `[]` balanceados
- `check_rust_syntax(path)` (línea ~130): Ejecuta `rustfmt --check`
- `check_js_syntax(path)` (línea ~155): Ejecuta `node --check`

### `src/scraper.rs`
- `perform_search()` (línea 10): Busca en Google, extrae títulos con regex, maneja CAPTCHAs
- `scraper_clean_tags()` (línea 62): Elimina etiquetas HTML — **OPTIMIZADO**: usa `OnceLock<Regex>` ⬅️

### `src/desktop.rs`
- `DesktopController` (línea 8): Control de mouse, teclado, ejecución de apps
  - `char_to_key_map()` (línea 14): Mapa estático `HashMap<char, (Key, bool)>` precomputado ⬅️ **REFACTORIZADO**
  - `type_text()` (línea ~80): Escribe texto usando el mapa — **REDUCIDO** de ~100 brazos match a lookup O(1)

### ~~`src/embeddings.rs`~~ ⬅️ **ELIMINADO (Julio 2026)**
Funciones `get_voyage_embeddings` y `cosine_similarity` estaban marcadas `#[allow(dead_code)]` y no se usaban.

### `prompts/default_system_prompt.txt` ⬅️ **NUEVO (Julio 2026)**
System prompt de ~36KB externalizado de `main.rs`. Se carga con `include_str!()`.

### `public/index.html` (~171 líneas)
UI del panel de control.

### `public/app.js` (~509 líneas) ⬅️ **REESCRITO (Julio 2026)**
- **CORREGIDO**: Doble definición de `sendBtn.onclick` eliminada
- **CORREGIDO**: Código huérfano de `reRefinePromptBtn` eliminado
- Lógica del frontend: API calls, gestión de proyectos, chat en tiempo real, monitoreo del agente

### `public/style.css`
Estilos con tema oscuro, efectos de vidrio, neón.

---

## Flujo de Herramientas

### write_file_with_commit (CORREGIDO Julio 2026)
1. **PASO 0**: `git remote get-url origin` → si no existe, `gh repo create`
2. **PASO 1**: `git pull --rebase --autostash origin master`
3. Si falla: autocuración SEGURA (abort rebase, reset --hard HEAD, reset --hard origin/master)
4. Escribir archivo (completo, ya no se recomienda edición por rango)
5. **PASO 2 (NUEVO)**: `validator::validate_file_after_write()` → detecta duplicados y errores
6. `git add` → `git commit` → `git push`

### execute_powershell
1. Sanitización: bloquea comandos peligrosos
2. Si es comando largo: spawn sin bloquear, registrar PID
3. Si es comando corto: ejecutar con timeout 30s
4. Si tiene `timer`: spawn con timer background

### search_code
- Búsqueda LOCAL por palabras clave en archivos del proyecto
- NO usa VoyageAI embeddings
- Score basado en coincidencias de palabras clave
- Limitado a 8 resultados

---

## Problemas Conocidos

- **Compilación**: `num-traits v0.2.19` puede fallar en el build script por permisos (antivirus)
- **API Key**: Se cargan desde variables de entorno
- **I/O**: `save_chat_steps_to_disk` y `debug_messages.json` escriben en cada iteración (ineficiente)
- **check_github_cli**: `split_whitespace()` no maneja argumentos con comillas
- **Bucle infinito**: No hay límite máximo de iteraciones en el loop principal
- **Explosión de pasos**: Los pasos de auditoría crecen sin límite (2177 pasos en una sesión)
- **Google scraping**: Frágil, Google cambia markup frecuentemente
