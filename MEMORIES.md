# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025)

### BUG #1: Crear usuario no permite configurar permisos granulares
- **Causa**: El JS frontend (`app.js`) hardcodeaba `permissions: ['read_file', 'search_code']` en el payload de creación. No se enviaban `editar_system_prompt_global` ni `editar_system_prompt_local`.
- **Fix**: Se agregaron checkboxes en el modal de crear usuario para todos los permisos granulares: modo_estudio, modo_programador, editar_system_prompt_global, editar_system_prompt_local, can_fork, can_exec_ps, can_write, can_search_google. El payload ahora envía todos los campos.
- **Lección**: Siempre mapear UI ↔ backend completo. No hardcodear payloads.

### BUG #2: No se pueden administrar horarios ni activación
- **Causa**: El backend tiene `WeeklySchedule` y endpoint `PUT /api/admin/users/:user/schedule`, pero el frontend NO tenía UI. El edit user modal solo tenía límites básicos sin campo `activacion` ni editor de horarios.
- **Fix**: Se agregó editor de horarios por día (formato "9-12,14-18") y toggle de activación en el modal de editar usuario.
- **Lección**: Si el backend soporta una feature, el frontend debe exponerla.

### BUG #3: Campo de contraseña parece texto
- **Causa**: El CSS solo estilizaba `input[type="text"]` pero no `input[type="password"]`, causando renderizado inconsistente entre navegadores.
- **Fix**: Se agregó `input[type="password"]` a todas las reglas CSS donde aparecía `input[type="text"]`, más estilos específicos para campos password (monospace, letter-spacing).
- **Lección**: Siempre incluir `input[type="password"]` en el reset CSS.

### BUG #4: Botón guardar prompts sugiere que solo guarda el global
- **Causa**: En el HTML, el `<button id="savePromptsBtn">` estaba entre el textarea Global y el textarea Local, visualmente asociado solo al global.
- **Fix**: Se movió el botón después de AMBOS textareas y se renombró a "Guardar Ambos".
- **Lección**: La posición visual de los botones debe reflejar su ámbito de acción.

### BUG #5: No hay botón de añadir carpeta local
- **Causa**: El botón y endpoint SÍ existen en el código fuente actual. Probablemente el HTML servido no era la versión más reciente (posible caché del navegador o build viejo).
- **Fix**: Verificado que el endpoint `POST /api/projects/local` (línea 1761 de main.rs) y el botón `#addLocalBtn` existen. Se agregó mensaje de éxito al agregar.
- **Lección**: Siempre verificar que la versión servida coincida con el código fuente.

### BUG #6: Modo estudio no envía mensajes
- **Causa**: El backend `/api/chat` recibe el campo `mode` pero no hace nada distinto con él para modo estudio. El frontend correctamente envía `mode: 'study'`. También faltaba verificación de permisos `has_study_access()` en el backend.
- **Fix**: El frontend ya verifica `authHasStudy` antes de mostrar el botón. El backend recibe `mode` en `ChatInput`. Se agregó verificación de permiso de estudio/programación en el endpoint `/api/chat` cuando se especifica `mode`. Ahora rechaza con 403 si el usuario no tiene el permiso correspondiente.
- **Lección**: Validar permisos tanto en frontend (UX) como en backend (seguridad).

### BUG #7: Permisos modo_programador y modo_estudio siempre desactivados en UI de admin
- **Causa**: El endpoint `GET /api/admin/users` serializa `UserAccount` directamente con serde. `UserAccount` tiene campos `modo_estudio` y `modo_programador`, pero el frontend (`app.js`) espera `has_study_access` y `has_programming_access` (que son métodos calculados, no campos serializables: `has_study_access()` devuelve `is_admin || admin || modo_estudio`). Como los métodos no se serializan, el JSON de respuesta no incluía esos campos, el frontend recibía `undefined`, y los checkboxes/íconos siempre mostraban estado falso.
- **Fix**: Se modificó `admin_list_users` en `main.rs` para transformar la lista de usuarios agregando los campos calculados `has_study_access` y `has_programming_access` a cada objeto antes de serializar.
- **Lección**: Cuando el frontend y backend usan nombres de campo diferentes, verificar que el backend serialice lo que el frontend espera. Los métodos en Rust no se serializan a JSON.

### BUG #8: Nonce muestra placeholder `<nonce>` en vez del nonce real
- **Causa**: El label HTML era estático: `Nonce (firmalo con: .\scripts\sign_nonce.ps1 -Nonce "&lt;nonce&gt;")`. El JS solo actualizaba el textarea con el nonce, nunca el label. Además no había botón de copiar comando.
- **Fix**: Se agregó `<span id="nonceLabelValue">` en el label para que JS lo actualice dinámicamente. Se agregó botón 📋 para copiar el comando completo al portapapeles. El JS ahora actualiza `nonceLabelValue.textContent` al recibir el challenge y almacena `_lastNonce` y `_lastAdminUser` para la función `copyNonceCmd()`.
- **Lección**: Los placeholders estáticos en HTML deben ser reemplazados con spans dinámicos cuando el valor viene del backend.

### BUG #9: Crear usuario no pide confirmar contraseña
- **Causa**: El formulario de crear usuario solo tenía un campo de contraseña (`newPassword`), sin campo de confirmación.
- **Fix**: Se agregó `newPasswordConfirm` con su toggle de visibilidad. El JS ahora valida que ambas contraseñas coincidan antes de enviar (solo para usuarios no-admin, ya que los admin usan nonce).
- **Lección**: Siempre incluir confirmación de contraseña en formularios de creación de usuarios.

### BUG #10: No hay toggle de visibilidad en campos de contraseña
- **Causa**: Los `<input type="password">` no tenían botón para mostrar/ocultar la contraseña.
- **Fix**: Se agregaron botones 👁️ con clase `toggle-password` en los tres campos: `loginPass`, `newPassword`, `newPasswordConfirm`. La función `togglePassword(fieldId)` cambia el `type` entre "password" y "text".
- **Lección**: UX básica de formularios de auth debe incluir toggle de visibilidad.

### BUG #11: Ningún modo funciona - muestra "iniciando agente" pero no hace nada
- **Causa Triple**:
  1. `chat_endpoint` solo guardaba el mensaje en disco y devolvía `{ status: "ok" }`. NUNCA spawneaba el agente. El agente (`run_agent_loop` en `agent.rs`) jamás se ejecutaba.
  2. `/api/agent/status` devolvía `{ running: bool, ... }` sin wrapper `status: "ok"`, y sin campo `active`. El frontend chequeaba `statusRes.status === 'ok' && statusRes.active` — ambas condiciones siempre daban `false`.
  3. Faltaban los endpoints `/api/agent/steps` y `/api/agent/summary` que el frontend llamaba.
- **Fix**:
  1. `chat_endpoint` ahora spawnea el agente en `tokio::spawn` después de guardar el mensaje. Al terminar, guarda la respuesta del agente en la sesión y actualiza los steps de auditoría.
  2. `get_agent_status` ahora devuelve `{ "status": "ok", "active": status.running, ... }`.
  3. Se agregaron `agent_steps` y `agent_summary` como nuevos handlers, y sus rutas en `build_app()`.
- **Lección**: Un endpoint de chat que solo guarda mensajes no es un agente. El agente debe spawnearse. El contrato API entre frontend y backend debe coincidir exactamente en nombres de campos.

## Por qué los tests no detectaron estos bugs
- Los tests existentes eran mayormente unitarios de estructuras JSON, no tests de UI ni E2E.
- No había tests que verificaran la correspondencia entre campos del frontend y backend.
- No había tests de integración para horarios, activación, o permisos granulares.
- No había tests de regresión específicos para la UI (HTML/CSS).

## Limitaciones técnicas conocidas
- `cargo check` en este proyecto es muy lento por la cantidad de dependencias (axum, tokio, ed25519, argon2, reqwest, etc.)
- El validador de archivos confunde declaraciones `const` locales de JS con definiciones duplicadas (falso positivo)
- Los tags `</div>` duplicados en HTML son normales y no deben marcarse como error
- **CRÍTICO**: La herramienta `write_file_with_commit` con `start_line`/`end_line` es PROPENSA A DUPLICAR CÓDIGO. Siempre que sea posible, usar scripts externos (Python/PowerShell) para ediciones complejas y luego commitear con git CLI.
- `git reset --hard` + `write_file_with_commit` parcial deja residuos de ediciones anteriores. Usar `git stash` + `git reset --hard` para limpieza completa.

## APIs y comportamiento verificado
- `POST /api/admin/users` acepta campos opcionales: `editar_system_prompt_global`, `editar_system_prompt_local`
- `PUT /api/admin/users/:username/schedule` acepta `{ "horarios": { "lunes": [[9,12], [14,18]], ... } }`
- `PUT /api/admin/users/:username/limits` acepta campo `activacion: bool`
- `POST /api/chat` acepta campo `mode: "study" | "programming"` y ahora SPAWNEA el agente en background
- `POST /api/projects/local` acepta `{ "name": "...", "path": "..." }`
- `GET /api/admin/users` ahora incluye `has_study_access` y `has_programming_access` en cada usuario
- `GET /api/agent/status` ahora devuelve `{ "status": "ok", "active": bool, ... }`
- `GET /api/agent/steps` — NUEVO endpoint para pasos de auditoría
- `GET /api/agent/summary` — NUEVO endpoint para resumen textual del progreso
