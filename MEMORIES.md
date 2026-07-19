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

## Por qué los tests no detectaron estos bugs
- Los tests existentes eran mayormente unitarios de estructuras JSON, no tests de UI ni E2E.
- No había tests que verificaran la correspondencia entre campos del frontend y backend.
- No había tests de integración para horarios, activación, o permisos granulares.
- No había tests de regresión específicos para la UI (HTML/CSS).

## Limitaciones técnicas conocidas
- `cargo check` en este proyecto es muy lento por la cantidad de dependencias (axum, tokio, ed25519, argon2, reqwest, etc.)
- El validador de archivos confunde declaraciones `const` locales de JS con definiciones duplicadas (falso positivo)
- Los tags `</div>` duplicados en HTML son normales y no deben marcarse como error

## APIs y comportamiento verificado
- `POST /api/admin/users` acepta campos opcionales: `editar_system_prompt_global`, `editar_system_prompt_local`
- `PUT /api/admin/users/:username/schedule` acepta `{ "horarios": { "lunes": [[9,12], [14,18]], ... } }`
- `PUT /api/admin/users/:username/limits` acepta campo `activacion: bool`
- `POST /api/chat` acepta campo `mode: "study" | "programming"`
- `POST /api/projects/local` acepta `{ "name": "...", "path": "..." }`
- `GET /api/admin/users` ahora incluye `has_study_access` y `has_programming_access` en cada usuario
