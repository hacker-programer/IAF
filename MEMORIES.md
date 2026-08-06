# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Cambios estructurales v3.0 — Electron + Capacitor + Chat Dedup + Android UX

- **Cliente Rust eliminado**: `client/` reemplazado por `electron/` (Electron) y `capacitor/` (Android)
- **Electron** (`electron/main.js`): Implementa protocolo cliente (connect→poll→execute→respond) con Node.js
- **Capacitor** (`capacitor/capacitor.config.ts`): WebView Android wrappeando `public/`
- **Chat dedup**: `clean_old_chat_files()`, `HashSet<String>` en `get_chats`, dedup client-side
- **Admin username labels**: `get_chats` incluye `username`, frontend muestra `(@username)`
- **Android responsive**: Hamburger menu `☰`, sidebar drawer, bottom sheet, media queries
- **Regla de seguridad**: Servidor NUNCA ejecuta comandos para no-admin. Electron ejecuta localmente.

### BUG-028 (SOLUCIONADO v3.0): Conversaciones duplicadas con mismo UUID
### BUG-030 (SOLUCIONADO v3.3): API DeepSeek retornaba 400 Bad Request — "messages[0]: missing field 'role'"

- **Síntoma**: Al iniciar el agente, la API retornaba 400 Bad Request con el error `messages[0]: missing field 'role'`. El servidor mostraba "Advertencia: La API retornó status 400 Bad Request" y reintentaba 3 veces antes de fallar.
- **Causa raíz**: Las 6 herramientas de Google Drive (`google_drive_list`, `google_drive_download`, `google_drive_read_content`, `google_drive_metadata`, `google_drive_create_folder`, `google_drive_upload`) se estaban definiendo como elementos del array `messages` en lugar de estar en el array `tools`. La API DeepSeek espera que cada elemento en `messages` tenga los campos `role` y `content`, pero las tool definitions solo tienen `type` y `function`.
- **Fix**:
  - `agent.rs::run_agent_loop()`: Se movieron las 6 Google Drive tools desde `let mut messages = vec![...]` (líneas 216-305) hacia `let tools = vec![...]` (después de línea 276), donde pertenecen.
  - `messages` ahora solo contiene `{ "role": "system", "content": system_prompt }` como único elemento inicial.
  - `tools` ahora contiene las 32 herramientas (6 Google Drive + 26 regulares).
- **Archivos modificados**: `src/agent.rs`
- **Lección**: Las tool definitions van en el campo `tools` del request body, NO en `messages`. La API DeepSeek/OpenRouter sigue el estándar OpenAI: `messages` requiere `role` + `content`, `tools` requiere `type` + `function`.
- **Verificación**: `cargo check` pasa limpiamente. El servidor ya no muestra el error 400.
  - `main.rs::get_chats()`: `HashSet<String>` para deduplicar por `id`, salta `.bak`
  - `app.js::loadChatHistory()`: Deduplicación client-side como defensa en profundidad
  - `app.js::selectChatSession()`: Eliminada línea `chatArea.innerHTML = ''` duplicada
- **Archivos modificados**: `src/main.rs`, `public/app.js`

### BUG-029 (SOLUCIONADO): Túnel Cloudflare mostraba interfaz rota
- **Fix**: `isPort80` solo detecta puerto 80 literal, no puerto vacío (HTTPS).

---

### BUG-028 (HISTÓRICO): System prompts no se cargaban en la UI — loadPrompts() perdido
- **Causa real**: `loadPrompts()` se perdió al reescribir `app.js` completo.
- **Lección**: **NUNCA reescribir un archivo completo para un fix pequeño.**

### BUG-026: System prompt de estudio no se cargaba
- **Fix**: `pub const STUDY_SYSTEM_PROMPT` + bloque `if mode == "study"` en agent.rs

### BUG-018: Study mode — obsesión con escribir archivos .md
- **Fix**: `study_system_prompt.txt` con 4 reglas de oro

---

## APIs y comportamiento verificado

- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD
- `window.location.port` es `''` tanto para HTTP:80 como HTTPS:443
- El validador JS marca falsos positivos en variables duplicadas entre funciones distintas
- **NUNCA reescribir un archivo completo para un fix pequeño** — usar ediciones parciales
- `cargo check` pasa limpiamente después de todas las correcciones v3.0

## Archivos de tests
- `tests/exhaustive_tests.rs` (1835 líneas) — 15 módulos, 123 tests
- `tests/integration_tests.rs` (1197 líneas) — 10 módulos
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend
