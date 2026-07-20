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
- `GET /api/agent/steps` — NUEVO endpoint para pasos de auditoría
- `GET /api/agent/summary` — NUEVO endpoint para resumen textual del progreso

### BUG #12: Las preguntas del agente no se muestran al usuario (modal agentQuestionModal nunca se abre)
### BUG #14 (BUG-004): finalizar_tarea devuelve "No se proporcionó URL"
- **Síntoma**: La herramienta `finalizar_tarea` devuelve error "No se proporcionó URL" a pesar de que el parámetro `mensaje_final` fue proporcionado correctamente.
- **Causa**: El código de `finalizar_tarea` en `agent.rs` estaba todo en una sola línea (ilegible), sin validación de mensaje vacío, y no limpiaba `info_messages` ni `esperando_respuesta_usuario`/`esperando_aprobacion_plan` al finalizar. El error "No se proporcionó URL" en realidad provenía de la herramienta `image_fetch` (línea adyacente), pero el agente lo malinterpretaba. El código ilegible contribuyó a que este bug pasara desapercibido.
- **Fix**: Se refactorizó `finalizar_tarea` en múltiples líneas con validación de mensaje vacío, limpieza de flags (`esperando_respuesta_usuario`, `esperando_aprobacion_plan`, `info_messages`), y mejor logging.
- **Tests de regresión**: `tests/exhaustive_tests.rs` — tests `reg_bug004_*` que validan: mensaje_final sin URL, no interferencia con image_fetch, limpieza de estado al finalizar.

### BUG #15 (BUG-001): No puede analizar PDFs ni .docx
- **Síntoma**: La herramienta `read_file` solo soporta archivos de texto plano. Si el usuario intenta leer un PDF o DOCX, falla o devuelve contenido binario ilegible.
- **Causa**: `read_file` en `agent.rs` usaba exclusivamente `fs::read_to_string()`, que solo funciona con UTF-8. No había detección de extensiones ni manejo de formatos binarios.
- **Fix**: Se agregó detección de extensión (`.pdf`, `.docx`) en `read_file`. Para PDFs: se usa `pdf-extract` para extraer texto. Para DOCX: se abre el ZIP interno y se extrae texto de `word/document.xml` usando la función `extract_text_from_docx_xml()`. Se agregaron dependencias `pdf-extract = "0.7"` y `zip = "1.1"` a Cargo.toml.
- **Tests de regresión**: `tests/exhaustive_tests.rs` — tests `reg_bug001_*` que validan: aceptación de extensiones .pdf y .docx, rechazo de formatos no soportados, soporte para formatos de texto comunes.

### BUG #16 (BUG-002): El frontend no muestra los mensajes informativos en tiempo real
- **Síntoma**: Cuando el agente llama a `notificar_usuario` con tipo "informativo", el mensaje se guarda en pasos de auditoría pero el frontend nunca lo muestra al usuario.
- **Causa**: El endpoint `/api/agent/status` no incluía `info_messages`. El `ActiveAgentStatus` no tenía el campo. El frontend `startAgentMonitoring()` solo monitoreaba preguntas, planes y CAPTCHA, pero no mensajes informativos.
- **Fix**: 
  1. Se agregó `info_messages: Vec<String>` a `ActiveAgentStatus` en `state.rs`
  2. Se agregó `info_messages` a la respuesta de `get_agent_status` en `main.rs`
  3. En `agent.rs`, `notificar_usuario` tipo "informativo" ahora agrega a `info_messages` (con límite de 100)
  4. En `app.js`, `startAgentMonitoring()` ahora monitorea `info_messages` y muestra notificaciones toast + agrega al chat
  5. Se creó `showInfoToast()` para notificaciones flotantes con auto-dismiss
- **Tests de regresión**: `tests/exhaustive_tests.rs` — tests `reg_bug002_*` que validan: presencia de info_messages en estado, consumo por frontend, límite de 100 mensajes.

### BUG #17 (BUG-003): El modo estudio da resúmenes en vez de enseñar
- **Síntoma**: El agente en modo estudio responde con resúmenes, temarios o listas de temas, en lugar de enseñar paso a paso de forma interactiva.
- **Causa**: El `study_system_prompt.txt` original no era lo suficientemente enfático en PROHIBIR resúmenes. Faltaba una regla explícita que dijera "JAMÁS respondas con un resumen del tema".
- **Fix**: Se reescribió el prompt de estudio agregando:
  - **Regla de Oro**: "PROHIBIDO DAR RESÚMENES O TEMARIOS" con ejemplos de qué NO hacer y qué SÍ hacer
  - **Formato de respuesta obligatorio**: 5 pasos (concepto, explicación, pregunta, mini-ejercicio, esperar)
  - **Regla de un solo concepto por mensaje**: no abrumar al alumno
  - **"ENSEÑA, no resumas. HAZ, no listes."** como mantra repetido
- **Tests de regresión**: `tests/exhaustive_tests.rs` — tests `reg_bug003_*` que validan: presencia de frases anti-resumen en el prompt, lecciones interactivas vs temarios pasivos.

## Por qué estos bugs no fueron detectados por tests (Lección 2025)
- **BUG-001 (PDF/DOCX)**: Los tests solo probaban archivos `.txt`, `.rs`, `.md`. No había tests con extensiones `.pdf` o `.docx`.
- **BUG-002 (info_messages)**: No había tests de contrato API que verificaran que el campo `info_messages` estuviera en la respuesta de `/api/agent/status`. Tampoco había tests que simularan el polling del frontend.
- **BUG-003 (resúmenes)**: Los tests no validaban el contenido semántico del prompt de estudio. Solo verificaban estructura, no reglas de comportamiento.
- **BUG-004 (finalizar_tarea)**: El código estaba en una sola línea, lo que hacía difícil leerlo y testearlo. No había tests que verificaran el flujo completo de finalización de tarea.

## Nuevas dependencias agregadas
- `pdf-extract = "0.7"` — extracción de texto de PDFs
- `zip = "1.1"` — lectura de archivos DOCX (formato ZIP con XML interno)

## Cambios estructurales en el estado del agente
- `ActiveAgentStatus` ahora tiene `info_messages: Vec<String>` (máx 100 mensajes)
- `get_agent_status` ahora incluye `info_messages` en la respuesta JSON
- `notificar_usuario` tipo "informativo" escribe en `info_messages` además de `steps`
- **Causa**: `startAgentMonitoring()` en `app.js` hacía polling a `/api/agent/status` pero solo leía `statusRes.active` y `statusRes.captcha_pending`. NUNCA leía `statusRes.esperando_respuesta_usuario` ni `statusRes.pregunta_usuario`. El modal `agentQuestionModal` estaba definido en el HTML pero nunca se abría programáticamente.
- **Fix**: Se agregó lógica en `startAgentMonitoring()` que revisa `esperando_respuesta_usuario` y `pregunta_usuario` y abre el modal. También se agregó detección de `esperando_aprobacion_plan` y `plan_propuesto` para abrir `agentPlanModal`. Se usan flags `agentQuestionShown` y `agentPlanShown` para evitar abrir el modal repetidamente durante el polling. También se agregaron los campos `esperando_aprobacion_plan` y `plan_propuesto` al endpoint `get_agent_status` (que no estaban en la respuesta JSON).
- **Tests de regresión**: `tests/frontend_regression_tests.js` — 10 tests (A-001 a A-010) que validan: detección de pregunta, no re-mostrar, pregunta vacía, sin pregunta, detección de plan, reset de flags, CAPTCHA, pregunta+plan simultáneos, agente inactivo, respuesta de error.
- **Lección**: El contrato API frontend-backend debe verificarse en ambos lados. Si el backend devuelve un campo pero el frontend no lo consume, es un bug tan grave como si el backend no lo devolviera.

### BUG #13: copyNonceCmd no copia nada — usa event sin declararlo y no tiene fallback para HTTP
- **Síntoma**: El botón 📋 en la pantalla de login nonce no copiaba el comando al portapapeles.
- **Causa doble**:
  1. `function copyNonceCmd()` usaba `event.target` sin declarar `event` como parámetro. En strict mode esto causa `ReferenceError`.
  2. `navigator.clipboard.writeText()` solo funciona en HTTPS o localhost. En HTTP (puerto 8080) la Promise se rechaza. No había fallback con `document.execCommand('copy')`.
- **Fix**: `copyNonceCmd(event)` ahora recibe `event` explícitamente. Se agregó `fallbackCopy()` usando `textarea + execCommand('copy')` para navegadores sin Clipboard API o HTTP. Si ambos fallan, muestra un alert con el comando para copia manual.
- **Tests de regresión**: `tests/frontend_regression_tests.js` — 6 tests (B-001 a B-006) que validan: copia con event, sin event, sin clipboard API (fallback), fallback que falla, nonce vacío, caracteres especiales en nonce.
- **Lección**: Las funciones que responden a eventos DOM deben declarar `event` como parámetro. El Clipboard API requiere un contexto seguro; siempre debe haber fallback.

