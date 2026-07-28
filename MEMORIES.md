# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

### BUG-028: System prompts no se cargaban en la UI — loadPrompts() perdido
- **Síntoma**: Los textareas `globalPrompt` y `localPrompt` aparecían vacíos en la interfaz gráfica. El system prompt global y local no se cargaban al iniciar sesión ni al cambiar de proyecto.
- **Causa real**: La función `loadPrompts()` y los handlers `savePromptsBtn`/`resetPromptBtn` se **perdieron completamente** al reescribir `app.js` en la sesión anterior (para aplicar BUG-027). El archivo pasó de 1164 líneas a 1066 líneas, y en esa reescritura se eliminaron ~100 líneas que incluían estas funciones críticas. `loadPrompts()` seguía siendo llamada desde `showApp()` (L237) y `selectProject()` (L623), pero como no existía, simplemente fallaba silenciosamente.
- **Fix aplicado**: Restaurar `public/app.js` desde el commit `8558c2a` (el último commit donde el archivo estaba íntegro), que ya contenía:
  - `loadPrompts()` (L601): Carga `GET /api/prompts` y rellena `#globalPrompt` y `#localPrompt`
  - `savePromptsBtn.onclick`: POST a `/api/prompts/global` y `/api/prompts/local`
  - `resetPromptBtn.onclick`: POST a `/api/prompts/global/reset`
  - BUG-027 fix: Shift+Enter permite saltos de línea en respuesta del agente
- **Lección**: **NUNCA reescribir un archivo completo para aplicar un fix pequeño.** Usar ediciones parciales quirúrgicas. Si es inevitable reescribir, verificar con `diff` que no se haya perdido ninguna función. También: tener tests de regresión que verifiquen la presencia de cada función clave (`loadPrompts`, `loadProjects`, `startAgentMonitoring`, etc.) mediante `include_str!`.

### BUG-027: Enter en respuesta a pregunta del agente no permitía saltos de línea
- **Síntoma**: Al responder una pregunta del agente en el textarea del banner, presionar Enter enviaba inmediatamente la respuesta sin permitir escribir múltiples líneas.
- **Fix**: `public/app.js` L1024-1030 — Shift+Enter inserta nueva línea, Enter solo envía. Código: `if (e.key === 'Enter' && !e.shiftKey)`.

### BUG-026: System prompt de estudio no se cargaba — el agente actuaba como modo programación
- **Síntoma**: Al usar el modo estudio, el agente ignoraba completamente el prompt de estudio (`study_system_prompt.txt`) y usaba el prompt global de programación.
- **Causa real**: `run_agent_loop()` en `agent.rs` recibía el parámetro `mode: &str` pero **jamás lo usaba**.
- **Fix aplicado** (3 archivos):
  1. `src/main.rs` L51: `const STUDY_SYSTEM_PROMPT` → `pub const STUDY_SYSTEM_PROMPT`
  2. `src/lib.rs`: Agregado `pub const STUDY_SYSTEM_PROMPT`
  3. `src/agent.rs` L116-121: Bloque `if mode == "study"` que reemplaza `system_prompt` con `study_engine.build_study_system_prompt()`

### BUG-024: System prompt no se cargaba en la interfaz gráfica (TextArea vacío)
- **Síntoma**: El textarea `globalPrompt` siempre aparecía vacío en la UI, aunque el backend sí devolvía el prompt correctamente.
- **Causa real**: Mismatch de nombres de campo: backend `legacy_prompts_get` devuelve `"global_current"`, frontend leía `prompts.global` (inexistente).
- **Fix**: `app.js:606`: Cambiado `prompts.global` → `prompts.global_current`.

### BUG-018: Study mode — obsesión con escribir archivos .md en vez de enseñar
- **Fix**: `study_system_prompt.txt` reescrito con 4 reglas de oro, anti-.md, transparencia, testing de métodos.

### BUG-017: Study mode — ignoraba preguntas sobre formato de razonamiento y métodos
- **Fix**: Agregada REGLA DE ORO #4 (transparencia) y sección sobre fases de aprendizaje.

### BUG-016: Respuesta a pregunta del agente no se guardaba en el chat
- **Fix**: Frontend (`addMessage`) + backend (`agent_responder` persiste en sesión JSON).

### BUG-015/014: Monitoreo/auditoría no funcionaba al recargar página
- **Fix**: `selectChatSession()` ahora carga `session.steps` y llama a `startAgentMonitoring()`.

### BUG-013: Métodos `get_user_projects` y `build_study_system_prompt` perdidos de study.rs
- **Causa**: `write_file_with_commit` truncó `study.rs` en memoria.
- **Fix**: Restaurados desde git.

### BUG-012: Race condition en test_engine()
- **Fix**: `AtomicU32` para directorios únicos por test.

### Cadena BUG-005→011 (9 bugs en tests)
- **Fix**: Correcciones en `tests/exhaustive_tests.rs`.

## Por qué estos bugs no fueron detectados por tests

### BUG-028 (loadPrompts perdido)
- No había test que verificara la presencia de `loadPrompts` en `app.js`.
- **Solución**: Agregar test en `exhaustive_tests.rs` que use `include_str!("../public/app.js")` y verifique que `function loadPrompts` existe.

### BUG-026 (system prompt estudio)
- `run_agent_loop` recibía `mode: &str` pero ningún test verificaba que el prompt cambiara según el modo.

### BUG-027 (Enter en textarea)
- No había test de frontend que simulase eventos de teclado.

## APIs y comportamiento verificado
- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD
- `cargo test` ejecuta tests en paralelo por defecto
- `write_file_with_commit` con archivos grandes puede truncar el contenido
- Verificar métodos cross-file después de cada edición
- `node --check` valida sintaxis JS sin ejecutar
- Un parámetro que se recibe pero no se usa es una bandera roja de bug
- **NUNCA reescribir un archivo completo para un fix pequeño** — usar ediciones parciales

## Cambios estructurales (v3.7 — BUG-028)
- `public/app.js`: Restaurado desde commit `8558c2a` (~1165 líneas). Contiene `loadPrompts()`, `savePromptsBtn`, `resetPromptBtn`, BUG-027 fix.
- `src/main.rs`: `pub const STUDY_SYSTEM_PROMPT` (L51)
- `src/lib.rs`: `pub const STUDY_SYSTEM_PROMPT`
- `src/agent.rs`: ~2418 líneas. Bloque `if mode == "study"` en L116-121.

## Archivos de tests
- `tests/exhaustive_tests.rs` (1835 líneas) — 15 módulos, 123 tests
- `tests/integration_tests.rs` (1197 líneas) — 10 módulos
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend