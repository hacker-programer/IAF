# 🔧 DOCUMENTACIÓN INTERNA — IAF (Intelligent Agent Framework)

> **Audiencia**: Desarrolladores que van a mantener, extender o depurar el sistema.
> **Última actualización**: Julio 2026

---

## 🏗️ Arquitectura General

IAF es un servidor HTTP (Axum + Tokio) que expone una API REST y sirve un frontend SPA.
El componente central es un **agente LLM** que itera sobre un bucle de razonamiento-acción
usando la API de DeepSeek.

```
┌──────────────┐     HTTP/WS     ┌──────────────┐     API Calls    ┌─────────────┐
│   Frontend   │ ◄──────────────► │   Backend    │ ◄──────────────► │  DeepSeek   │
│  (HTML/JS)   │                 │  (Rust/Axum) │                 │   API V4    │
└──────────────┘                 └──────┬───────┘                 └─────────────┘
                                       │
                                       ├──► PowerShell (comandos del sistema)
                                       ├──► GitHub CLI (gh)
                                       ├──► Google (scraping)
                                       └──► OpenRouter (análisis multimodal)
```

### Flujo del agente (agent loop)

1. Recibe un mensaje del usuario desde el frontend
2. Construye el historial de chat con tool definitions
3. Llama a DeepSeek API con el historial
4. Si DeepSeek responde con tool_calls → ejecuta las herramientas → vuelve al paso 2
5. Si DeepSeek responde con texto → lo muestra al usuario
6. Si DeepSeek llama a `finalizar_tarea` → termina el loop

---

## 📁 Estructura de Archivos

```
C:\Users\Fa\Desktop\IAF\
├── src/
│   ├── main.rs          # Punto de entrada, rutas HTTP, inicialización
│   ├── agent.rs         # Lógica del agente, tool handlers, agent loop
│   ├── state.rs         # Estructuras de estado compartido (AppState)
│   ├── scraper.rs       # Búsqueda web (Google scraping)
│   ├── desktop.rs       # Control del escritorio (simulación de teclado/mouse)
│   └── validator.rs     # Validación post-escritura de archivos
├── public/
│   ├── index.html       # Frontend SPA
│   ├── app.js           # Lógica del frontend
│   └── style.css        # Estilos
├── prompts/
│   └── default_system_prompt.txt  # System prompt (extraído de main.rs)
├── .config/
│   ├── chats/           # Historiales de conversación (JSON)
│   └── logs/            # Logs de errores y panics
├── Cargo.toml           # Dependencias Rust
├── .env                 # Claves API
├── DOCUMENTATION.md     # Mapa técnico del proyecto
├── DOCUMENTACION_CLIENTE.md
├── DOCUMENTACION_INTERNA.md  # Este archivo
└── MEMORIES.md          # Lecciones aprendidas y limitaciones
```

---

## 🔑 Componentes Principales

### `main.rs` — Servidor HTTP y rutas

**Responsabilidades**:
- Inicializar `AppState` con todas las dependencias
- Cargar system prompt desde `prompts/default_system_prompt.txt` vía `include_str!()`
- Configurar rutas HTTP
- Servir archivos estáticos desde `public/`
- Descubrir proyectos en el workspace

**Rutas principales**:

| Método | Ruta | Handler | Descripción |
|--------|------|---------|-------------|
| GET | `/api/projects` | `list_projects` | Lista proyectos |
| POST | `/api/projects/local` | `add_local_project` | Agrega proyecto local |
| POST | `/api/projects/fork` | `fork_project` | Forkea repositorio |
| POST | `/api/chat` | `chat_handler` | Envía mensaje al agente |
| POST | `/api/chat/approve` | `approve_agent_plan` | Aprueba plan de acción |
| POST | `/api/chat/interrupt` | `interrupt_agent` | Interrumpe al agente |
| GET | `/api/agent/status` | `agent_status` | Estado del agente |
| GET | `/api/agent/steps` | `agent_steps` | Pasos de auditoría |
| GET | `/api/agent/summary` | `agent_summary` | Resumen de pasos |
| POST | `/api/captcha/solve` | `captcha_solve` | Resuelve CAPTCHA |
| GET | `/api/captcha/status` | `captcha_status` | Estado del CAPTCHA |
| GET/POST | `/api/prompts` | `prompts_handler` | Gestión de prompts |
| GET | `/api/chats` | `list_chats` | Lista historiales |
| POST | `/api/chats/new` | `new_chat` | Crea nuevo chat |
| GET | `/api/chats/:id` | `get_chat` | Obtiene chat por ID |

---

### `agent.rs` — Lógica del agente

**Tool Definitions** (herramientas que el agente puede usar):

| Herramienta | Descripción | Handler |
|-------------|-------------|---------|
| `search_google` | Búsqueda web | Llama a `perform_search` en scraper.rs |
| `read_file` | Lee archivo | `std::fs::read_to_string` |
| `write_file_with_commit` | Escribe archivo + commit | Escribe, git add, commit, push + validación post-escritura |
| `execute_powershell` | Ejecuta comandos | `Command::new("powershell")` |
| `search_code` | Búsqueda local | Coincidencia de palabras clave |
| `fork_and_clone_repo` | Fork GitHub | GitHub CLI |
| `check_github_cli` | Comandos gh | GitHub CLI con directorio del proyecto |
| `read_url` | Lee URL | reqwest GET |
| `image_fetch` | Descarga imagen | reqwest GET + guardar a disco |
| `image_view` | Muestra imagen | Codifica en base64 |
| `image_release` | Libera imagen | Elimina del contexto |
| `analyze_images` | Análisis multimodal | OpenRouter API |
| `notificar_usuario` | Notifica al usuario | Callback al frontend |
| `finalizar_tarea` | Termina ejecución | Finaliza el agent loop |
| `kill_process` | Mata proceso | Mata por PID |
| `git_resolve_divergence` | Resuelve divergencia Git | force push / reset / rebase |

**Agent Loop** (`run_agent_loop`, línea ~60-550):
1. Construye el array de mensajes (system prompt + historial + tools)
2. Llama a `call_deepseek_api`
3. Parsea la respuesta: texto → mostrar al usuario, tool_calls → ejecutar
4. La respuesta de la herramienta se añade al historial
5. Repite hasta que `finalizar_tarea` es llamado

**Validación post-escritura** (línea ~843):
Después de escribir un archivo, se llama a `validate_file_after_write` de `validator.rs`.
Si se detectan duplicados, delimitadores no balanceados o errores de sintaxis,
se añade una advertencia al mensaje de respuesta que se envía al modelo.

---

### `state.rs` — Estado compartido

**Estructuras principales**:

- `AppState`: Estado global del servidor (proyectos, sesiones, agente activo, etc.)
- `Project`: Representa un proyecto (nombre, ruta, si es local)
- `ActiveAgentStatus`: Estado del agente (corriendo, interrumpido, esperando respuesta)
- `ChatSession`: Historial de conversación
- `ChatMessage`: Mensaje individual en el chat
- `AuditStep`: Paso de auditoría para monitoreo
- `ProcessRegistry`: Registro de procesos spawnados (para kill seguro)
- `CaptchaRequest`: Petición de CAPTCHA pendiente
- `PromptConfig`: Configuración de prompts

---

### `validator.rs` — Validación post-escritura

**Propósito**: Detectar errores comunes después de que el agente modifica archivos.

**Funciones**:
- `validate_file_after_write(path)`: Función principal. Ejecuta todas las validaciones.
- `check_duplicate_lines(content)`: Detecta líneas duplicadas consecutivas.
- `check_balanced_delimiters(content)`: Verifica llaves, paréntesis y corchetes balanceados.
- `check_rust_syntax(path)`: Ejecuta `rustfmt --check` y `cargo check` en archivos .rs.
- `check_js_syntax(path)`: Ejecuta `node --check` en archivos .js.

**Integración**: Se llama desde `agent.rs` en el handler de `write_file_with_commit`.

---

### `scraper.rs` — Búsqueda web

- `perform_search(query)`: Busca en Google y retorna resultados HTML crudos.
- `scraper_clean_tags(html)`: Limpia tags HTML usando regex precompilada (`OnceLock<Regex>`).
- Maneja detección de CAPTCHA y bloqueos de Google.

---

### `desktop.rs` — Control de escritorio

- `DesktopController`: Estructura principal.
- `type_text(text)`: Escribe texto simulando teclado. Usa `HashMap<char, (Key, bool)>` precomputado.
- `mouse_move(x, y)`: Mueve el mouse.
- `mouse_click(button)`: Hace clic.
- `launch_app(command)`: Abre una aplicación.
- `get_screen_size()`: Obtiene resolución de pantalla.

---

## 🔄 Flujo de Datos

### Envío de mensaje del usuario

```
Usuario escribe en el chat (app.js)
  → POST /api/chat { message, project_name, session_id }
    → main.rs::chat_handler
      → Spawnea tokio::spawn(run_agent_loop)
        → agent.rs::run_agent_loop
          → Construye historial con system prompt
          → Loop:
            → call_deepseek_api(historial)
            → Si tool_calls → ejecutar herramienta → añadir resultado al historial
            → Si texto → enviar SSE al frontend
            → Si finalizar_tarea → salir del loop
```

### Escritura de archivo

```
Agente llama a write_file_with_commit
  → agent.rs::handler
    → std::fs::write(path, content)
    → validator::validate_file_after_write(path)
    → Si hay warnings → añadir al mensaje de respuesta
    → git add → git commit → git push
    → Retornar resultado al modelo
```

---

## 🧪 Cómo Extender el Sistema

### Agregar una nueva herramienta

1. Añadir la definición en `agent.rs` en el array de `tools` (línea ~130-370)
2. Añadir el handler en el match de tool_calls (línea ~600-1500)
3. Si requiere nueva dependencia, agregarla a `Cargo.toml`

### Agregar una nueva validación

1. Añadir la función en `validator.rs`
2. Llamarla desde `validate_file_after_write`
3. Agregar los resultados al `ValidationResult`

### Modificar el system prompt

Editar `prompts/default_system_prompt.txt`. Se carga automáticamente con `include_str!()`.

---

## ⚠️ Problemas Conocidos

Ver `MEMORIES.md` para la lista completa de limitaciones y comportamientos de APIs.

- El agente tiende a duplicar código al editar por rango de líneas → **solucionado parcialmente** con `validator.rs`
- Google scraping es frágil (cambia markup frecuentemente)
- `safe_truncate` causaba pánico con UTF-8 multi-byte → **solucionado**
- `wmic` está deprecated → **reemplazado** por `Get-CimInstance`

---

## 📦 Dependencias Clave

| Crate | Versión | Uso |
|-------|---------|-----|
| `axum` | 0.7 | Servidor HTTP |
| `tokio` | 1.x | Runtime async |
| `serde` / `serde_json` | 1.x | Serialización |
| `reqwest` | 0.12 | Cliente HTTP |
| `regex` | 1.x | Expresiones regulares |
| `rdev` | 0.5 | Simulación de entrada |
| `uuid` | 1.x | Generación de IDs |
| `base64` | 0.22 | Codificación de imágenes |
| `urlencoding` | 2.x | Encoding de URLs |
| `open` | 5.x | Abrir navegador |
| `tower-http` | 0.5 | CORS, serving estático |
