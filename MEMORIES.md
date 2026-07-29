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
- **Síntoma**: Dos archivos `.json` en `chats/` con mismo UUID pero títulos diferentes. Al hacer clic en uno, se seleccionaban ambos.
- **Causa**: Al cambiar el título de una conversación, se creaba un nuevo archivo `<nuevo-titulo>-<uuid>.json` sin eliminar el viejo.
- **Fix**:
  - `main.rs::clean_old_chat_files()`: Elimina archivos viejos con mismo UUID antes de guardar
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
