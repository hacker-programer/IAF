# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

### BUG-027: Enter en respuesta a pregunta del agente no permitía saltos de línea
- **Síntoma**: Al responder una pregunta del agente en el textarea del banner, presionar Enter enviaba inmediatamente la respuesta sin permitir escribir múltiples líneas.
- **Causa real**: El event listener `keydown` en `#agentQuestionResponse` (app.js ~L926) capturaba cualquier `e.key === 'Enter'` y llamaba a `submitAgentResponseBtn.click()` sin verificar modificadores.
- **Fix aplicado**: Cambiado a `if (e.key === 'Enter' && (e.ctrlKey || e.metaKey))` — Ctrl+Enter (o Cmd+Enter en Mac) envía la respuesta, Enter solo inserta nueva línea. También se agregó `e.preventDefault()` para evitar comportamiento por defecto.
- **Lección**: Los textareas siempre deben permitir saltos de línea con Enter simple. El envío debe requerir un modificador (Ctrl/Cmd) o un botón explícito.

### BUG-026: System prompt de estudio no se cargaba — el agente actuaba como modo programación
- **Síntoma**: Al usar el modo estudio, el agente ignoraba completamente el prompt de estudio (`study_system_prompt.txt`) y usaba el prompt global de programación. El agente no sabía que estaba en modo estudio.
- **Causa real**: `run_agent_loop()` en `agent.rs` recibía el parámetro `mode: &str` pero **jamás lo usaba**. Siempre cargaba el `global_prompt` (modo programación). `STUDY_SYSTEM_PROMPT` existía como constante privada en `main.rs` y solo se usaba en el endpoint de preview `study_build_prompt`, nunca en el bucle real del agente.
- **Fix aplicado** (3 archivos):
  1. `src/main.rs` L51: `const STUDY_SYSTEM_PROMPT` → `pub const STUDY_SYSTEM_PROMPT` (accesible desde `agent.rs` vía `crate::`)
  2. `src/lib.rs`: Agregado `pub const STUDY_SYSTEM_PROMPT` también en la librería
  3. `src/agent.rs` L116-121: Agregado bloque `if mode == "study"` que SOBREESCRIBE `system_prompt` con `state.study_engine.build_study_system_prompt(username, crate::STUDY_SYSTEM_PROMPT)`. Esto reemplaza completamente el prompt de programación con el prompt de estudio + perfil del estudiante (edad, intereses, fase, engagement, conocimientos previos).
- **Lección**: Cuando un parámetro se pasa a una función pero no se usa, es una bandera roja. Verificar que todos los parámetros tengan un propósito real en el código.

### BUG-024: System prompt no se cargaba en la interfaz gráfica (TextArea vacío)
- **Síntoma**: El textarea `globalPrompt` siempre aparecía vacío en la UI, aunque el backend sí devolvía el prompt correctamente.
- **Causa real**: Mismatch de nombres de campo entre frontend y backend:
  - Backend `legacy_prompts_get` (main.rs:1883) devuelve `"global_current"` y `"global_default"`
  - Frontend `loadPrompts()` (app.js:606) leía `prompts.global` (campo inexistente)
  - `prompts.global` siempre era `undefined` → `''` (string vacía)
- **Fix aplicado**:
  1. `app.js:606`: Cambiado `prompts.global` → `prompts.global_current`
  2. `app.js:540`: `selectProject()` ahora también llama a `loadPrompts()` para que el prompt local se actualice al cambiar de proyecto
- **Lección**: Cuando se cambia el esquema de respuesta de un endpoint, verificar TODOS los consumidores (frontend JS, tests, etc.) para asegurar que los nombres de campo coincidan.

### BUG-018: Study mode — obsesión con escribir archivos .md en vez de enseñar
- **Causa real**: El `study_system_prompt.txt` tenía reglas contra escribir archivos, pero no eran lo suficientemente prominentes. El `default_system_prompt.txt` contiene "DOCUMENTACIÓN INTERNA Y EXTERNA OBLIGATORIA" que entraba en conflicto. Además, el prompt de estudio no mencionaba la transparencia de razonamiento ni el testing de métodos de aprendizaje.
- **Fix aplicado**: `study_system_prompt.txt` completamente reescrito:
  1. **REGLA DE ORO #1** (la primera, más prominente): PROHIBIDO ABSOLUTAMENTE crear archivos .md, guías, tutoriales, READMEs, o documentos de estudio. Solo se permite crear juegos educativos interactivos (.html, .rs, .py).
  2. **REGLA DE ORO #3**: TRANSPARENCIA DE RAZONAMIENTO — si el alumno pregunta cómo razona el agente, DEBE mostrar su <thinking>.
  3. Instrucciones explícitas sobre testing de métodos de aprendizaje (exploración vs explotación).
  4. "ENSEÑA EN EL CHAT, NO EN ARCHIVOS" repetido múltiples veces.
- **Lección**: Las reglas más importantes deben estar PRIMERO en el prompt y ser reforzadas con ejemplos negativos concretos.

### BUG-017: Study mode — ignoraba preguntas sobre formato de razonamiento y métodos
- **Causa real**: El prompt de estudio no mencionaba en absoluto que el alumno puede preguntar sobre el formato de razonamiento (<thinking>) ni sobre los métodos de aprendizaje (exploración/explotación).
- **Fix aplicado**: Agregada REGLA DE ORO #3 (transparencia) y sección específica sobre fases de aprendizaje.
- **Lección**: Todas las funcionalidades del sistema deben estar documentadas en el prompt para que el agente las conozca y las explique.

### BUG-016: Respuesta a pregunta del agente no se guardaba en el chat
- **Causa real**: 
  1. Frontend: `submitAgentResponseBtn.onclick` no llamaba a `addMessage('user', respuesta)` → la respuesta no aparecía en el chat.
  2. Backend: `agent_responder` solo seteaba `respuesta_usuario` y `esperando_respuesta_usuario = false`, pero NUNCA guardaba la respuesta en el archivo de sesión JSON.
- **Fix aplicado**:
  1. Frontend: `addMessage('user', respuesta)` antes de enviar al backend.
  2. Backend: `agent_responder` ahora busca el archivo de chat por `session_id`, agrega la pregunta del agente (si no estaba guardada) y la respuesta del usuario, y persiste.
  3. Se creó `find_chat_file_by_session_id_inner()` en main.rs (duplicada de `agent.rs` porque la original es privada).
- **Lección**: Cuando el frontend y backend manejan el mismo dato, ambos deben persistirlo. No confiar en que uno solo lo haga.

### BUG-015: No mostraba la auditoría al recargar la página y entrar a un chat
- **Causa real**: `selectChatSession()` cargaba los mensajes del chat pero NO:
  1. Cargaba `session.steps` desde la sesión para mostrarlos en el panel de auditoría.
  2. Llamaba a `startAgentMonitoring()` para iniciar el polling.
- **Fix aplicado**: `selectChatSession()` ahora:
  1. Si `res.session.steps` existe, llama a `renderConsoleSteps(res.session.steps)`.
  2. Llama a `startAgentMonitoring()` al final.
- **Lección**: Cada vez que se agrega una nueva funcionalidad (auditoría, monitoreo), hay que verificar TODOS los puntos de entrada del frontend (nuevo chat, chat existente, recarga).

### BUG-014: Mensajes en tiempo real no se mostraban sin recargar la página
- **Causa real**: `startAgentMonitoring()` solo se llamaba después de `sendMessageToAgent()` (enviar un mensaje nuevo). Si el usuario recargaba la página y entraba a un chat existente mediante `selectChatSession()`, el monitoreo NUNCA se iniciaba.
- **Fix aplicado**: `selectChatSession()` ahora llama a `startAgentMonitoring()`.
- **Lección**: El monitoreo debe iniciarse desde TODOS los puntos de entrada: nuevo mensaje Y selección de chat existente.

### BUG-013: Métodos `get_user_projects` y `build_study_system_prompt` perdidos de study.rs
- **Causa real**: Al aplicar el fix de BUG-012 con `write_file_with_commit`, el contenido de `study.rs` se truncó en memoria (el `read_file` devolvió contenido incompleto por el tamaño del archivo). Esto eliminó dos métodos públicos que `main.rs` necesitaba: `get_user_projects` y `build_study_system_prompt`. El error solo se detectó al ejecutar `cargo test` (no al compilar `study.rs` solo, ya que los métodos estaban en `impl StudyEngine` y el archivo cerraba correctamente).
- **Fix aplicado**: Restaurar ambos métodos desde el commit `7b9a273`:
  1. `get_user_projects(&self, username) -> Vec<StudyProject>` insertado después de `list_user_projects` (L583). Devuelve los proyectos completos donde el usuario es miembro.
  2. `build_study_system_prompt(&self, username, base_prompt) -> String` insertado después de `detect_disengagement` (L515). Construye el system prompt del modo estudio con perfil, knowledge base y engagement.
- **Lección**: `write_file_with_commit` con archivos grandes (>900 líneas) puede truncar si `read_file` no devuelve el contenido completo. Verificar SIEMPRE que los métodos requeridos por otros archivos sigan presentes después de la edición.

### BUG-012: Race condition en test_engine() — directorio compartido entre tests paralelos
- **Causa real**: La función `test_engine()` usaba un directorio fijo `iaf_test_study` para TODOS los tests. `cargo test` ejecuta tests en PARALELO. Un test llamaba `remove_dir_all(&tmp)` mientras otro usaba `save_profile()`, causando `os error 3` intermitente.
- **Fix aplicado**: `AtomicU32` como contador global (`TEST_DIR_COUNTER`) generando directorios únicos: `iaf_test_study_0`, `iaf_test_study_1`, etc.
- **Lección**: NUNCA compartir un directorio temporal entre tests paralelos. Usar IDs atómicos únicos.

### BUG-011: 2 tests fallaban por assertions incorrectos
- **Test 1**: `agent_rs_finalizar_tarea_no_exige_parametro_url` — delimitaba mal el schema con `"=> {"`. Reescribí para buscar `"required"` y acotar al primer `]`.
- **Test 2**: `agent_rs_read_file_tiene_manejo_errores` — buscaba `"No existe"` pero el código real usa `"Error leyendo archivo"`. Corregido.

### Cadena completa BUG-005→006→009→007→008→010→011→012→013 (9 bugs):
1. **BUG-005**: `mod regression_new_bugs` sin `}` → insertada llave
2. **BUG-006**: `fn estado_agente...()` sin cuerpo → eliminada
3. **BUG-009**: Bloque huérfano (cuerpo de BUG-006) → eliminado
4. **BUG-007**: Rutas `"../src/"` para `read_to_string` → `"src/"`
5. **BUG-008**: `include_str!("src/main.rs")` roto → `"../src/main.rs"`
6. **BUG-010**: Módulos duplicados + match String/&str → renombrados + `.as_str()`
7. **BUG-011**: 2 tests con assertions incorrectos → reescritos
8. **BUG-012**: Race condition en `test_engine()` → `AtomicU32`
9. **BUG-013**: Métodos `get_user_projects` + `build_study_system_prompt` perdidos → restaurados

### BUG-001: PDF/DOCX — `fn extract_text_from_docx()` + `pdf_extract::extract_text()`. Verificado.
### BUG-002: Mensajes en tiempo real — `info_messages` se consume SIEMPRE. Verificado.
### BUG-004: finalizar_tarea URL — `"required": ["mensaje_final"]`. Verificado.

## Por qué estos bugs no fueron detectados por tests

### BUG-026 (system prompt estudio)
- `run_agent_loop` recibía `mode: &str` pero ningún test verificaba que el prompt cambiara según el modo.
- **Solución**: Agregar test que verifique que cuando `mode == "study"`, `system_prompt` contiene `STUDY_SYSTEM_PROMPT`.

### BUG-027 (Enter en textarea)
- No había test de frontend que simulase eventos de teclado en el textarea de respuesta.
- **Solución**: Agregar test de regresión en `frontend_regression_tests.js`.

### BUG-018/017 (study mode)
- Tests de integración no incluyen pruebas del system prompt porque el prompt es texto libre.
- **Solución**: Agregar test que verifique que el prompt de estudio contiene las reglas clave (prohibición .md, transparencia).

### BUG-016 (respuesta no guardada)
- No había test que simulara el flujo completo: agente pregunta → usuario responde → verificar que la respuesta aparece en la sesión.
- **Solución**: Agregar test de integración para el flujo `notificar_usuario(pregunta) → agent_responder`.

### BUG-015/014 (monitoreo al recargar)
- Los tests de frontend no cubren el escenario "recargar página y seleccionar chat existente".
- **Solución**: Agregar test de regresión que verifique `selectChatSession` llama a `startAgentMonitoring`.

### BUG-013 (métodos perdidos)
- `cargo check` de `study.rs` no detecta métodos faltantes porque `impl StudyEngine` cerraba correctamente.
- `cargo test` de `lib.rs` (unit tests) no usa `main.rs`, por lo que pasaba.
- Solo `cargo test` del binario (`--bin iaf`) detectó el error porque `main.rs` llama a los métodos.
- **Solución**: Los tests en `integration_tests.rs` deberían verificar la presencia de métodos clave en `study.rs` mediante `include_str!`.

### BUG-012 (race condition)
- Los tests pasaban en ejecución secuencial (`--test-threads=1`) pero fallaban intermitentemente en paralelo.
- **Solución**: Aislar cada test con directorios únicos usando `AtomicU32`.

### La cadena BUG-005→011
- Los errores estaban en el propio archivo de tests. Si no compila, ningún test se ejecuta.
- **Solución**: Tests de integridad en archivo separado (`integration_tests.rs`).

## Verificación completa de bugs viejos (2025-07)

| Bug | Estado | Evidencia |
|-----|--------|-----------|
| PDF/DOCX | ✅ | `fn extract_text_from_docx`, `pdf_extract::extract_text`, `zip::ZipArchive`, `quick_xml::Reader` |
| finalizar_tarea URL | ✅ | `"required": ["mensaje_final"]`, sin `"url"` |
| System prompt local | ✅ | `load_local_prompt`, `get_project_path`, `Project Specific Prompt:` |
| Mensajes en tiempo real | ✅ | `showInfoToast`, `startAgentMonitoring`, `lastInfoMessageCount` |
| addMessage | ✅ | 1 `function addMessage` |
| Perfil estudio | ✅ | `loadStudyProfile`, `/api/study/profile`, `profile_exists_on_disk` |
| JS sintaxis | ✅ | Llaves y paréntesis balanceados |
| Módulos duplicados | ✅ | 15 módulos únicos |
| match String/&str | ✅ | `match ext.as_str()` |
| Race condition tests | ✅ | `AtomicU32` counter |
| Métodos study.rs | ✅ | `get_user_projects`, `build_study_system_prompt` restaurados |

## APIs y comportamiento verificado
- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD
- `cargo test` ejecuta tests en paralelo por defecto
- `write_file_with_commit` con archivos grandes puede truncar el contenido
- Verificar métodos cross-file después de cada edición
- `node --check` valida sintaxis JS sin ejecutar
- Un parámetro que se recibe pero no se usa es una bandera roja de bug

## Cambios estructurales (v3.6 — BUG-026 + BUG-027)
- `src/main.rs`: `STUDY_SYSTEM_PROMPT` ahora es `pub const` (L51)
- `src/lib.rs`: Agregado `pub const STUDY_SYSTEM_PROMPT` para acceso desde librería
- `src/agent.rs`: ~2418 líneas. Bloque `if mode == "study"` en L116-121 que reemplaza system prompt con estudio + perfil
- `public/app.js`: ~1066 líneas. Fix BUG-027: Ctrl+Enter envía respuesta, Enter inserta nueva línea
- `prompts/study_system_prompt.txt`: Prompt pedagógico completo con 4 reglas de oro

## Archivos de tests (v3.5)
- `tests/exhaustive_tests.rs` (1835 líneas) — 15 módulos, 123 tests
- `tests/integration_tests.rs` (1197 líneas) — 10 módulos
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend