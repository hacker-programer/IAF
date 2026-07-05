# DOCUMENTATION.md - Proyecto IAF (Intelligent Agent Framework)

## Arquitectura General
IAF es un agente autónomo de desarrollo escrito en Rust (edición 2021) que usa la API de DeepSeek V4 como backend LLM. El agente tiene acceso a herramientas como búsqueda web, ejecución de comandos PowerShell, manipulación de archivos con GitHub, y gestión de imágenes.

## Archivos Fuente Principales

### `src/main.rs` (~1200 líneas)
Punto de entrada del servidor web (Axum). Maneja rutas HTTP, sesiones de chat, y sirve la UI.

- `DEEPSEEK_KEY` (línea 28): API key hardcodeada
- `DEFAULT_GLOBAL_SYSTEM_PROMPT` (línea 31): System prompt del agente
- Llamadas a DeepSeek Flash para generación de títulos (líneas 599, 671)
- Manejo de sesiones de chat desde/hacia `.config/chats/*.json`

### `src/agent.rs` (~1620 líneas)
Núcleo del agente. Contiene el bucle principal de ejecución y todas las herramientas.

- `DEEPSEEK_API_URL` (línea 12): `https://api.deepseek.com/v1/chat/completions`
- `run_agent_loop()` (línea 14): Bucle principal del agente
- `write_file_with_commit` (línea ~520): Herramienta para modificar archivos con commit automático. **CORREGIDA con PASO 0 de seguridad (ver abajo)**
- `sanitize_messages_for_api()` (línea 1477): **CORREGIDA** - Convierte mensajes multimodales (`image_url`) a texto plano y sana mensajes tool huérfanos
- `compress_active_messages_if_needed()` (línea 1312): Comprime contexto cuando excede 500K caracteres
- `truncate_old_tool_responses()` (línea 1289): **CREADA** - Trunca respuestas de assistant tras 3+ iteraciones
- `safe_truncate()` (línea 1277): Trunca strings en límites de caracteres UTF-8
- `semantic_code_search()` (línea 1187): Búsqueda de código con VoyageAI
- `image_view` / `image_fetch` / `image_release`: Herramientas de manipulación de imágenes (inyectan mensajes multimodales en líneas ~1035-1054)
- `play_error_beep()` (línea 1552): Emite un beep de error

### `src/state.rs`
Estructuras de estado: `AppState`, `ChatSession`, `AuditStep`, `AgentStatus`.

### `src/scraper.rs`
Funciones de scraping web: `perform_search()`, `scraper_clean_tags()`.

### `union.py`
Script Python para unir archivos. Usa clipboard.

## Bugs Corregidos

### Bug #1: Borrado Masivo de Código en Proyectos sin Remote (CORREGIDO - 2025)
**Severidad**: Crítica (pérdida total de código fuente)

**Síntoma**: Al editar una carpeta local sin repositorio remoto en GitHub mediante `write_file_with_commit`, el sistema borraba TODO el código fuente local en un intento fallido de sincronización.

**Causa raíz**: El flujo original en `agent.rs` (función `write_file_with_commit`) seguía esta secuencia:
1. Intentaba `git pull --rebase --autostash` de `origin/master`
2. Si fallaba (porque no existía remote), ejecutaba "autocuración":
   - `git fetch origin`
   - `git reset --hard origin/master` ← descarta todos los cambios locales
   - `git clean -fd` ← **ELIMINA PERMANENTEMENTE todos los archivos no rastreados**

Al no existir `origin`, `git clean -fd` borraba irreversiblemente todo el proyecto.

**Solución aplicada** (líneas ~543-700 de `agent.rs`):
1. **PASO 0 (NUEVO)**: Verificar que `git remote get-url origin` existe
2. Si NO existe: intentar crearlo automáticamente con `gh repo create --source=. --push --remote=origin`
3. Si `gh` no está disponible o falla: **ABORTAR con error claro**, sin ejecutar ninguna operación destructiva. El código local queda intacto.
4. Si el remote existe pero `origin/master` no es accesible (`git ls-remote` falla): intentar push inicial (`git push -u origin master`)
5. Si todo falla: abortar sin tocar archivos locales
6. **ELIMINADO**: `git clean -fd` del flujo de autocuración por ser demasiado destructivo
7. El `git reset --hard` ahora solo se ejecuta si `origin/master` fue verificado como accesible

## Flujo de Corrección del Bug `image_url`

1. `image_view` inyecta mensaje con `content: [{type: "text", ...}, {type: "image_url", ...}]` (línea ~1035)
2. Antes de enviar a DeepSeek, `sanitize_messages_for_api()` (línea 350) convierte `content` array → string plano
3. DeepSeek recibe solo texto, sin `image_url`

## Problemas Conocidos

- **Compilación**: `num-traits v0.2.19` falla en el build script por permisos de escritura (probablemente antivirus). No relacionado con el código.
- **API Key**: Hardcodeada en `main.rs:28` (riesgo de seguridad).