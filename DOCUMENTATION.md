# DOCUMENTATION.md - Proyecto IAF (Intelligent Agent Framework)

## Arquitectura General
IAF es un agente autónomo de desarrollo escrito en Rust (edición 2021) que usa la API de DeepSeek V4 como backend LLM. El agente tiene acceso a herramientas como búsqueda web, ejecución de comandos PowerShell, manipulación de archivos con GitHub, y gestión de imágenes.

## Archivos Fuente Principales

### `src/main.rs` (~1436 líneas)
Punto de entrada del servidor web (Axum). Maneja rutas HTTP, sesiones de chat, y sirve la UI.

- `deepseek_key()` (línea ~30): API key desde variable de entorno `DEEPSEEK_API_KEY`
- `voyage_key()` (línea ~35): API key desde `VOYAGE_API_KEY`
- `openrouter_key()` (línea ~40): API key desde `OPENROUTER_API_KEY`
- `DEFAULT_GLOBAL_SYSTEM_PROMPT` (línea ~47): System prompt del agente (~500 líneas de reglas de optimización)
- `main()` (línea ~1200): Configura rutas Axum, carga prompts, inicia servidor en `0.0.0.0:8080`
- Llamadas a DeepSeek Flash para generación de títulos (líneas ~599, ~671)
- Manejo de sesiones de chat desde/hacia `.config/chats/*.json`
- `AppState` se inicializa con `ProcessRegistry` para gestión segura de procesos

### `src/agent.rs` (~1847 líneas)
Núcleo del agente. Contiene el bucle principal de ejecución y todas las herramientas.

- `DEEPSEEK_API_URL` (línea 12): `https://api.deepseek.com/v1/chat/completions`
- `run_agent_loop()` (línea 14): Bucle principal del agente — construye system prompt, carga historial, ejecuta loop de herramientas
- `write_file_with_commit` (línea ~580): Herramienta para modificar archivos con commit automático
  - **PASO 0 (AGREGADO 2026-07-04)**: Verifica `git remote get-url origin`. Si no existe, intenta `gh repo create`. Si falla, aborta sin tocar archivos.
  - **ELIMINADO (2026-07-04)**: `git clean -fd` del flujo de autocuración
  - Usa labeled block `'write_handler` para evitar que errores de git terminen la sesión del agente
- `execute_powershell` (línea ~824): Ejecuta comandos PowerShell con:
  - Sanitización de seguridad (bloquea taskkill, Stop-Process, etc.)
  - Soporte para comandos de larga duración (spawn sin bloquear)
  - Registro de PIDs en `ProcessRegistry`
  - Timeout de 30s para comandos cortos
- `sanitize_messages_for_api()` (línea ~1717): Convierte mensajes `tool` huérfanos a `user` (evita error 400 de DeepSeek)
- `compress_active_messages_if_needed()` (línea ~1553): Comprime contexto cuando excede 500K caracteres usando DeepSeek Flash
- `truncate_old_tool_responses()` (línea ~1530): Trunca respuestas de assistant tras 3+ iteraciones
- `safe_truncate()` (línea ~1518): Trunca strings en límites de caracteres UTF-8 seguros
- `semantic_code_search()` (línea ~1428): Búsqueda de código local por palabras clave (NO USA VoyageAI embeddings)
- `save_chat_steps_to_disk()` (línea ~1385): Persiste pasos de auditoría en el archivo JSON de sesión
- `get_project_path()` (línea ~1399): Resuelve la ruta de un proyecto por nombre
- `discover_projects()` (línea ~1407): Escanea `base_workspace` en busca de proyectos
- `play_error_beep()` (línea ~1773): Emite un beep de error del sistema
- `notificar_usuario` (línea ~1030): Handler para preguntas (bloquea hasta respuesta) e informativos
- `finalizar_tarea` (línea ~1098): Limpia procesos hijo y termina la ejecución

### `src/state.rs`
Estructuras de estado compartido:

- `Project` (línea ~9): Nombre, ruta, flag is_local
- `PromptConfig` (línea ~16): Prompts global y por proyecto
- `ChatMessage` (línea ~23): Rol, contenido, timestamp
- `ChatSession` (línea ~30): ID, título, mensajes, proyecto, pasos de auditoría
- `AuditStep` (línea ~39): Tipo, título, detalle, timestamp
- `ActiveAgentStatus` (línea ~47): Estado completo del agente (running, interrupted, pregunta, plan, steps, etc.)
- `ContextEntry` (línea ~61): Entrada de contexto para RAG
- `CaptchaRequest` (línea ~69): ID, sitekey, URL, contenido resuelto
- `ProcessRegistry` (línea ~84): **Registro seguro de PIDs** con validación de parent PID antes de matar procesos
  - `register()`: Registra un PID spawnado por el agente
  - `safe_kill()`: Verifica que el PID esté registrado y sea hijo del servidor antes de matar
  - `kill_all()`: Mata todos los procesos registrados al finalizar sesión
  - `reap()`: Limpia procesos ya terminados
- `AppState` (línea ~212): Estado global con paths, prompts, proyectos, captcha, imágenes, ProcessRegistry

### `src/scraper.rs`
- `perform_search()` (línea 7): Busca en Google, extrae títulos con regex, maneja CAPTCHAs
- `scraper_clean_tags()` (línea 62): Elimina etiquetas HTML con regex

### `src/embeddings.rs`
- `get_voyage_embeddings()` (línea 6): Obtiene embeddings de VoyageAI (NO USADO actualmente - `#[allow(dead_code)]`)
- `cosine_similarity()` (línea 32): Calcula similitud coseno entre vectores (NO USADO actualmente - `#[allow(dead_code)]`)

### `src/desktop.rs`
- `DesktopController` (línea 6): Control de mouse, teclado, ejecución de apps
  - `move_mouse()`: Mueve el cursor a coordenadas absolutas
  - `click()`: Click izquierdo/derecho/medio
  - `type_text()`: Escribe texto (SOLO espacios implementados actualmente)
  - `launch_executable()`: Lanza un ejecutable
  - `open_image()`: Abre archivo con app predeterminada

### `public/index.html` (~171 líneas)
UI del panel de control con sidebar (proyectos, historial, prompts) y área principal (chat + consola de auditoría).

### `public/app.js` (~502 líneas)
Lógica del frontend: API calls, gestión de proyectos, chat en tiempo real, monitoreo del agente (polling 1s), modales de CAPTCHA/preguntas/planes.

### `public/style.css`
Estilos con tema oscuro, efectos de vidrio, neón.

## Flujo de Herramientas

### write_file_with_commit (CORREGIDO 2026-07-04)
1. **PASO 0**: `git remote get-url origin` → si no existe, `gh repo create`
2. **PASO 1**: `git pull --rebase --autostash origin master`
3. Si falla: autocuración SEGURA (abort rebase, reset --hard HEAD, limpiar lock files, reset --hard origin/master)
4. Si sigue fallando: error como tool result (no termina sesión)
5. Escribir archivo (completo o por rango de líneas)
6. `git add` → `git commit` → `git push`

### execute_powershell
1. Sanitización: bloquea comandos peligrosos (taskkill, Stop-Process, etc.)
2. Si es comando largo (`cargo run`, `npm start`): spawn sin bloquear, registrar PID
3. Si es comando corto: ejecutar con timeout 30s
4. Si tiene timer explícito: spawn con timer background

### search_code
- Búsqueda LOCAL por palabras clave en archivos (.rs, .js, .ts, .py, .json, .md, .html, .css, .toml)
- NO usa VoyageAI embeddings (a pesar de que el nombre de la herramienta lo sugiere)
- Score basado en coincidencia exacta (10pts) + palabras clave (1pt c/u)
- Limitado a 8 resultados

## Problemas Conocidos

- **Compilación**: `num-traits v0.2.19` puede fallar en el build script por permisos de escritura (antivirus)
- **API Key**: Se cargan desde variables de entorno (DEEPSEEK_API_KEY, VOYAGE_API_KEY, OPENROUTER_API_KEY)
- **I/O**: `save_chat_steps_to_disk` y `debug_messages.json` escriben en cada iteración (ineficiente)
- **type_text**: Solo implementa espacios, ignora el resto de caracteres
- **check_github_cli**: `split_whitespace()` no maneja argumentos con comillas
- **Bucle infinito**: No hay límite máximo de iteraciones en el loop principal
- **Explosión de pasos**: Los pasos de auditoría crecen sin límite (2177 pasos en una sesión)
