# MEMORIES.md — Memoria Persistente del Proyecto IAF

> Este archivo registra limitaciones técnicas, fallos de configuración y comportamientos de APIs
> descubiertos durante el desarrollo. Su objetivo es **minimizar llamadas innecesarias al modelo**,
> reducir cómputo redundante y evitar llamadas repetitivas de red.

---

## 🐛 Bugs Conocidos y Solucionados

### [2026-07-08] Google Search siempre fallaba (CAPTCHA / bloqueo)
- **Estado**: CORREGIDO (2026-07-08)
- **Archivo**: `src/scraper.rs`
- **Causa**: Google bloquea agresivamente las peticiones automatizadas, incluso con User-Agent de navegador real. El agente gastaba iteraciones intentando search_google sin éxito.
- **Solución**: Se reescribió `scraper.rs` para usar **DuckDuckGo Lite** (`lite.duckduckgo.com`) como fuente principal. DDG Lite devuelve HTML simple sin JavaScript, fácil de parsear y mucho más amigable con scrapers. Google se mantiene como fallback (poco probable que funcione). La función `perform_search` ahora orquesta: DDG primero → Google fallback → DDG con mensaje de error si Google falla.

### [2026-07-08] Truncado arbitrario de tool results (pérdida de información)
- **Estado**: CORREGIDO (2026-07-08)
- **Archivo**: `src/state.rs` (nuevo `ToolResultStore`)
- **Causa**: Los resultados de herramientas >25K chars se truncaban con `[VISUALIZACIÓN PARCIAL...]`, perdiendo información que el agente podía necesitar. El agente no tenía control sobre qué resultados mantener.
- **Solución**: Se creó `ToolResultStore`:
  - Resultados pequeños (<3000 chars): se devuelven completos
  - Resultados grandes: se almacenan bajo un ID único (call_id) y se devuelve un resumen + instrucciones de paginación
  - El agente puede usar `fetch_tool_result(id, pagina, page_size)` para leer más
  - El agente puede usar `release_tool_result(id)` para liberar memoria cuando ya no necesita el resultado
  - Reemplaza el truncado arbitrario por un sistema de visibilidad controlado por el agente

### [2026-07-08] Falsos positivos del validador: "definiciones duplicadas" entre diferentes impl blocks
- **Estado**: CORREGIDO (2026-07-08)
- **Archivo**: `src/validator.rs`
- **Causa**: `detect_duplicate_definitions` no distinguía entre métodos de diferentes structs. `fn new()` en `impl ToolResultStore` se reportaba como duplicado de `fn new()` en `impl SubAgentManager` y `impl ProcessRegistry`.
- **Solución**: Se modificó `detect_duplicate_definitions` para trackear el contexto de `impl` blocks:
  1. Nueva función `extract_impl_struct_name()`: extrae el nombre del struct de una declaración `impl`
  2. Nueva función `extract_def_name_with_context()`: prefija el nombre de la definición con `StructName::`
  3. Stack de contextos `impl` para manejar bloques anidados
  4. Las claves ahora son `ToolResultStore::fn new`, `SubAgentManager::fn new`, `ProcessRegistry::fn new` → no colisionan

### [2026-07-08] Falsos positivos del validador: líneas duplicadas en argumentos de macros
- **Estado**: CORREGIDO (2026-07-08)
- **Archivo**: `src/validator.rs` (`detect_duplicate_lines`)
- **Causa**: Argumentos repetidos en `format!()` como `call_id, call_id, call_id` se reportaban como líneas duplicadas.
- **Solución**: Se agregó `current.ends_with(",")` al conjunto de líneas estructurales ignoradas.

### [2026-07-07] CRÍTICO: Razonamiento del modelo inyectado en archivos de código sin //
- **Estado**: CORREGIDO (2026-07-07)
- **Archivos**: `src/agent.rs`, `src/validator.rs`, `prompts/default_system_prompt.txt`
- **Causa raíz**: El `write_handler` usaba la respuesta textual del modelo en lugar del contenido real de la herramienta.
- **Solución múltiple (defensa en 3 capas)**:
  1. `agent.rs`: variable `content` extraída de `args["content"]`
  2. `agent.rs`: `detect_reasoning_in_pre_write()` — validación pre-escritura
  3. `validator.rs`: `detect_reasoning_injection()` — validación post-escritura
  4. System prompt: regla #6 prohíbe razonamiento en `content`

### [2026-07-06] Mensaje "TRUNCADO POR EL SISTEMA" confundía al agente → reversión destructiva
- **Estado**: CORREGIDO (2026-07-06)
- **Archivo**: `src/agent.rs`
- **Solución**: Mensaje cambiado a "VISUALIZACIÓN PARCIAL" + regla #4 en system prompt

### [2026-07-06] Código duplicado por ediciones parciales (start_line/end_line)
- **Estado**: CORREGIDO (2026-07-06)
- **Solución**: System prompt prohíbe start_line/end_line + validator.rs `detect_duplicate_definitions()`

### [2026-07] `git clean -fd` borraba código fuente en proyectos sin remote
- **Estado**: CORREGIDO (2026-07-04)

---

## 🔧 APIs y Limitaciones Técnicas

### DeepSeek API
- **URL**: `https://api.deepseek.com/v1/chat/completions`
- **Modelo principal**: `deepseek-v4-pro` (con `thinking: {type: "enabled"}`)
- **Modelo de compresión**: `deepseek-v4-flash`
- **No soporta**: contenido multimodal `image_url` (solo texto)

### DuckDuckGo Lite API (NUEVO)
- **URL**: `https://lite.duckduckgo.com/lite/?q=...`
- **Ventaja**: No requiere API key, no bloquea scrapers, HTML simple
- **Limitación**: Resultados menos ricos que Google, ~10 por página
- **Uso**: Fuente principal en `scraper.rs`, Google como fallback

### search_code
- La herramienta `search_code` usa búsqueda LOCAL por palabras clave (NO VoyageAI).
- La función `search_code_in_project()` en `agent.rs` ahora es **`pub`** para que `sub_agent.rs` pueda usarla.

---

## 📐 Decisiones de Arquitectura

### Tool Result Store (NUEVO)
- Reemplaza el truncado arbitrario de 25K chars
- Sistema de IDs + paginación controlado por el agente
- `reap_old()` para limpieza automática de resultados antiguos

### Sub-Agent Manager (NUEVO)
- Sub-agentes paralelos con límite dinámico según hardware:
  - 2 cores → 1 sub-agente
  - 4 cores → 2 sub-agentes
  - 8 cores → 4 sub-agentes
  - 16+ cores → 8 sub-agentes
- Contexto heredado del agente principal (resumen)
- Restricciones de path para evitar colisiones
- Cancelación vía `AbortHandle`

### Validación post-escritura (`validator.rs`)
- Ahora con conciencia de contexto `impl` blocks
- Ignora argumentos repetidos en macros (`call_id, call_id,`)
- Tests unitarios incluidos en el mismo archivo

---

## 🚨 Patrones de Error del Agente

1. **Duplicación al editar por rango**: ⚠️ CORREGIDO — System prompt prohíbe start_line/end_line
2. **Miedo al mensaje TRUNCADO**: ⚠️ CORREGIDO — Mensaje "VISUALIZACIÓN PARCIAL" + regla #4
3. **No verificar compilación entre ediciones**: ⚠️ CORREGIDO — System prompt exige `cargo check`
4. **Código huérfano por rangos incorrectos**: ⚠️ MITIGADO — Ediciones por archivo completo
5. **Timeouts en comandos pesados**: Usar parámetro `timer` (mínimo 120s para compilación)
6. **Falsos positivos del validador**: ⚠️ CORREGIDO — Contexto impl en definiciones, argumentos repetidos ignorados
