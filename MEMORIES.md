# MEMORIES.md — Lecciones Aprendidas y Limitaciones del Proyecto IAF

> Archivo de memoria persistente para minimizar llamadas innecesarias al modelo,
> cómputo redundante y llamadas repetitivas de red.

---

## 🔴 Limitaciones y Fallos Conocidos

### Google Scraping
- **Fecha**: 2025-01
- **Problema**: La búsqueda en Google (scraper.rs) no usa la API oficial; hace scraping del HTML de resultados.
- **Síntoma**: Puede disparar CAPTCHAs y dejar de funcionar sin previo aviso.
- **Mitigación**: Se usa DuckDuckGo Lite como fallback principal.

### API de Voyage
- **Fecha**: 2025-01
- **Problema**: La API de Voyage no soporta búsquedas híbridas.
- **Mitigación**: Se reemplazó search_code con búsqueda local por palabras clave (sin embeddings).

### Validación de Escritura
- **Fecha**: 2025-01
- **Problema**: El validador post-escritura en main.rs marcaba falsos positivos al detectar `const` en JS como "definiciones duplicadas".
- **Mitigación**: Ignorar warnings de validación en archivos JS. El validador está diseñado para Rust.

---

## 🟡 Bugs Corregidos (v2.3)

### CAPTCHA Endpoints 404 (2025-02)
- **Síntoma**: El frontend hacía polling cada 3s a `/api/captcha/status` y `/api/captcha/solve` que no existían en el router `build_app`.
- **Causa raíz**: `CaptchaRequest` existía en `state.rs` como estructura de datos pero nunca se implementaron los handlers HTTP ni las rutas.
- **Solución**: Se agregaron `captcha_status` (GET) y `captcha_solve` (POST) en `main.rs` y se registraron en `build_app`. Ahora siempre devuelven JSON válido (`{"status":"ok","url":null}`) en vez de 404.
- **Por qué los tests no lo cubrieron**: Los tests HTTP existentes estaban todos con `#[ignore]` (requieren servidor). No había tests unitarios para verificar que las rutas estuvieran registradas.

### apiCall se rompía con respuestas no-JSON (2025-02)
- **Síntoma**: `Uncaught (in promise) SyntaxError: Failed to execute 'json' on 'Response': Unexpected end of JSON input`
- **Causa raíz**: La función `apiCall` en app.js hacía `return res.json()` directamente. Si cualquier endpoint devolvía 404 con HTML (como `/api/prompts` o `/api/captcha/status`), `res.json()` lanzaba excepción no capturada.
- **Solución**: Se modificó `apiCall` para leer `res.text()` primero, intentar `JSON.parse()`, y devolver un objeto de error estructurado si falla.

### Interfaz de login rota en puerto 8080 (2025-02)
- **Síntoma**: Después de loguearse en puerto 8080, la UI se rompía con errores de consola.
- **Causa raíz**: `showApp()` llamaba a `loadPrompts()` que usaba `/api/prompts` (endpoint inexistente). El error de parseo JSON resultante no se manejaba, rompiendo la UI.
- **Solución**: Se agregaron todos los endpoints legacy que el frontend esperaba y se hizo `apiCall` resiliente.

### Endpoints Legacy Faltantes (2025-02)
- **Problema**: El frontend esperaba estos endpoints que no existían:
  - `/api/prompts` (GET/POST) → legacy_prompts_get, legacy_prompts_post
  - `/api/prompts/reset` → legacy_prompts_reset
  - `/api/prompts/refine` → legacy_prompts_refine
  - `/api/projects/fork` → fork_project
  - `/api/projects/local` → add_local_project
  - `/api/agent/responder` → agent_responder
  - `/api/agent/aprobar_plan` → agent_approve_plan
  - `/api/agent/interrupt` → agent_interrupt
- **Solución**: Todos implementados en `main.rs` v2.3.

---

## 🟢 Patrones y Decisiones de Arquitectura

### Doble Puerto (80/8080)
- Puerto 80: `state.port_80 = true`, sin auth, acceso total.
- Puerto 8080: requiere login, permisos según usuario.
- Ambos comparten el mismo `AppState` subyacente pero con `port_80` diferente.

### Migración de Datos Legacy
- `migrate_chats()` se ejecuta al inicio en `main()`.
- Es recursiva: procesa `.config/chats/` y todos sus subdirectorios.
- Migra 3 tipos de datos: chats (UUID → título-UUID), prompts.json (→ per-user), local_projects.json (→ per-user).
- Los archivos originales se renombran a `.bak` en vez de borrarse.

### Formato de Chats
- Nuevo: `<título_sanitizado>-<uuid>.json`
- Viejo: `<uuid>.json`
- Los admins tienen sus chats en `.config/chats/` directamente.
- Los usuarios normales tienen sus chats en `.config/chats/<username>/`.

### apiCall en Frontend
- Ahora es resiliente: `fetch` → `res.text()` → `JSON.parse()` con try-catch.
- Nunca lanza excepción: siempre devuelve un objeto con `status`.
- El CAPTCHA polling usa `fetch` directo (no `apiCall`) para manejo de errores más fino.

---

## 📋 Checklist Anti-Regresión

Antes de hacer deploy:
- [ ] `cargo check` limpio
- [ ] `cargo test --test integration_tests` (29 tests pasan)
- [ ] Verificar que `/api/captcha/status` devuelva 200 (no 404)
- [ ] Verificar que `/api/prompts` GET devuelva JSON con `global_current`
- [ ] Verificar que `apiCall` no lance excepciones con endpoints inexistentes
- [ ] Verificar migración: backups `.bak` creados, archivos renombrados

---

## 🧠 Lecciones para el Futuro

1. **Siempre agregar el endpoint al router**: Si existe el struct de request/response en state.rs, verificar que también haya un handler y una ruta en `build_app`.
2. **Siempre hacer `apiCall` resiliente**: Cualquier endpoint puede fallar. El frontend no debe romperse por un 404.
3. **Los tests `#[ignore]` no protegen**: Si todos los tests de integración están ignorados, no hay red de seguridad. Agregar tests unitarios de aceptación que validen estructuras JSON y lógica sin necesidad de servidor.
4. **Migración debe ser recursiva**: Si hay subdirectorios (carpetas de usuario), `fs::read_dir` solo lee un nivel. Usar recursión explícita.
5. **Legacy endpoints como puente**: Mantener compatibilidad con frontends viejos evita breaking changes silenciosos.