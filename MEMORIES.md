# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

### BUG-001: No puede analizar PDFs ni .docx
- **Causa real**: El `read_file` handler en `agent.rs` solo usaba `fs::read_to_string()`. No detectaba extensiones `.pdf` ni `.docx`.
- **Fix aplicado**: 
  1. Se agregó `fn extract_text_from_docx()` que usa `zip::ZipArchive` + `quick_xml::Reader` para parsear DOCX nativamente.
  2. Se agregó detección de extensión en `read_file`: si es `.pdf` → `pdf_extract::extract_text()`, si es `.docx` → `extract_text_from_docx()`.
  3. Dependencias agregadas en Cargo.toml: `pdf-extract = "0.7"`, `zip = "0.6"`, `quick-xml = "0.31"`.
- **Verificación**: Tests `agent_rs_contiene_extract_text_from_docx`, `agent_rs_usa_pdf_extract_nativo_no_pdftotext`, `agent_rs_read_file_detecta_extension_pdf_docx` en `exhaustive_tests.rs`.

### BUG-002: El frontend no muestra los mensajes informativos en tiempo real
- **Causa real**: 
  1. `startAgentMonitoring()` en `app.js` solo consumía `info_messages` cuando `active || running` era true.
  2. Cuando el agente terminaba (`running=false`), el frontend iba al `else` y nunca veía los últimos mensajes.
  3. `finalizar_tarea` en `agent.rs` hacía `info_messages.clear()` borrando mensajes pendientes.
- **Fix aplicado**:
  1. `app.js`: El consumo de `info_messages` se mueve ANTES del chequeo `active || running`, para que se consuman SIEMPRE.
  2. `agent.rs`: `finalizar_tarea` YA NO hace `info_messages.clear()`. Los mensajes persisten para que el frontend los consuma.
  3. `state.rs`: `ActiveAgentStatus` tiene campo `info_messages: Vec<String>`.
  4. `main.rs`: `get_agent_status` incluye `info_messages` y `final_message` en la respuesta JSON.
- **Verificación**: Tests `app_js_muestra_info_messages_incluso_con_agente_terminado`, `agent_rs_notificar_usuario_push_info_messages`, `agent_rs_finalizar_tarea_no_limpia_info_messages` en `exhaustive_tests.rs`.

### BUG-004: finalizar_tarea devuelve error "No se proporcionó URL"
- **Causa real**: El handler de `finalizar_tarea` estaba en una sola línea (ilegible), y el agente a veces confundía el error de `image_fetch` ("No se proporcionó URL") con `finalizar_tarea`.
- **Fix aplicado**: `finalizar_tarea` refactorizado a múltiples líneas, con validación de `mensaje_final` vacío, y sin referencia a `url`. Ahora limpia correctamente flags (`esperando_respuesta_usuario`, `esperando_aprobacion_plan`) y NO limpia `info_messages` (BUG-002).
- **Verificación**: Tests `agent_rs_finalizar_tarea_refactorizado_multilinea`, `agent_rs_finalizar_tarea_usa_mensaje_final_no_url`, `finalizar_tarea_mensaje_vacio_usa_default` en `exhaustive_tests.rs`.

### BUG: addMessage duplicada en app.js (no se puede empezar conversación)
- **Causa real**: La función `addMessage` estaba definida DOS VECES en `app.js` (líneas 829-830 y 831-837). La primera definición estaba incompleta (solo `const div = document.createElement('div');` sin cierre de llave ni cuerpo completo). Esto causaba error de sintaxis JS y rompía todo el flujo de chat.
- **Fix**: Eliminadas las líneas duplicadas (829-830), dejando una sola definición completa de `addMessage`.
- **Verificación**: Test `app_js_add_message_definida_una_sola_vez` en `exhaustive_tests.rs`.

### BUG: No carga el perfil en modo estudio en el frontend (arreglado previamente)
- **Fix**: `StudyEngine` usa rutas correctas: `.config/data/<username>/profile.json`, `learnings.json`, `teachingMethod.json`.
- **Verificación**: Tests en `integration_tests.rs` — `study_engine_nuevo_carga_perfiles_desde_disco`, `study_engine_save_profile_crea_archivo_en_disco`.

### BUG: No ve el system prompt local ni el perfil ni el directorio del proyecto (arreglado previamente)
- **Fix**: `agent.rs` `run_agent_loop` carga `local_prompt` desde `state.prompts.projects` y `global_prompt` desde `state.prompts.global_current`.
- El directorio del proyecto se obtiene con `get_project_path()`.
- **Verificación**: Tests `agent_rs_recibe_project_name_y_local_prompt`, `agent_rs_usa_get_project_path`, `agent_rs_local_prompt_overridea_global` en `exhaustive_tests.rs`.

## Por qué estos bugs no fueron detectados por tests (Lección 2025-2026)

### BUG-001 (PDF/DOCX)
- **Los tests existentes solo probaban extensiones `.txt`, `.rs`, `.md`.**
- No había tests que verificaran que `read_file` detectara extensiones `.pdf` o `.docx`.
- No había tests que verificaran que `fn extract_text_from_docx` existiera en el código.
- **Solución**: Se agregaron tests de verificación de código fuente (`include_str!`) que validan la presencia de `fn extract_text_from_docx`, `pdf_extract::extract_text`, `zip::ZipArchive`, `quick_xml::Reader`.

### BUG-002 (Mensajes informativos)
- **No había tests que verificaran el contrato API frontend ↔ backend para `info_messages`.**
- Los tests no simulaban el polling del frontend ni la race condition de agente terminado.
- **Solución**: Se agregó test `app_js_muestra_info_messages_incluso_con_agente_terminado` que verifica que `info_messages` se consumen ANTES del chequeo `active || running`.

### BUG-004 (finalizar_tarea URL)
- **El código estaba en una sola línea, imposible de testear unitariamente.**
- No había tests que verificaran el bloque de `finalizar_tarea` completo.
- **Solución**: Se agregaron tests que verifican que `finalizar_tarea` tiene más de 10 líneas (refactorizado), que NO contiene `"url"`, y que maneja correctamente mensajes vacíos.

### BUG: addMessage duplicada
- **No había tests que contaran las definiciones de funciones en app.js.**
- El validador detectaba "DEFINICIÓN DUPLICADA" pero se consideraba falso positivo.
- **Solución**: Se agregó test `app_js_add_message_definida_una_sola_vez` que cuenta `matches("function addMessage")`.

## Lección: Tests SIMULADOS vs REALES
- Los tests simulados (crear JSON y validar contra sí mismo) NO detectan bugs reales.
- Los tests REALES deben usar:
  - `include_str!` para verificar código fuente
  - Conteo de ocurrencias (`matches()`)
  - Verificación de posiciones relativas (`find()` / `rfind()`)
  - Serialización/deserialización real con `serde_json`
  - Creación de archivos reales en disco
  - Llamadas a funciones reales del sistema

## APIs y comportamiento verificado
- `POST /api/chat` spawnea el agente en `tokio::spawn` después de guardar el mensaje
- `GET /api/agent/status` devuelve `{"status":"ok","active":bool,"finished":bool,"final_message":...,"info_messages":[...]}`
- `POST /api/agent/responder` acepta `{"respuesta":"..."}` y limpia `esperando_respuesta_usuario`
- `POST /api/agent/aprobar_plan` acepta `{"aprobar":bool}` y limpia `esperando_aprobacion_plan`
- `GET /api/agent/steps` devuelve pasos de auditoría
- `GET /api/agent/summary` devuelve resumen textual
- `Path::extension()` para `.gitignore` devuelve `None` (el `.` inicial es parte del stem, no extensión)
- `Path::extension()` para `.gitignore` devuelve `None` (el `.` inicial es parte del stem, no extensión)

## Cambios estructurales (v2.6)
- `lib.rs` ahora expone: `pub mod utils; pub mod state; pub mod auth; pub mod study; pub mod desktop; pub mod sync;`
- `state.rs`: `ActiveAgentStatus` tiene `info_messages: Vec<String>`, `finished: bool`, `final_message: Option<String>`
- `agent.rs`: `extract_text_from_docx()` para DOCX nativo, `pdf_extract::extract_text()` para PDF nativo, `finalizar_tarea` refactorizado a multi-línea
- `app.js`: `startAgentMonitoring()` consume `info_messages` SIEMPRE, sin importar `running`/`finished`. `addMessage` sin duplicados.

## Dependencias agregadas
- `pdf-extract = "0.7"` — extracción de texto de PDFs
- `zip = "0.6"` — lectura de archivos DOCX (formato ZIP con XML interno)
- `quick-xml = "0.31"` — parseo rápido del XML dentro de DOCX

## Archivos de tests (v2.6)
- `tests/exhaustive_tests.rs` — Tests de verificación de código fuente (include_str!), regresión, integración, estrés, inyección de fallos, casos límite, smoke tests, y tests de nuevos bugs descubiertos
- `tests/integration_tests.rs` — Tests reales: StudyEngine con disco, UserStore con contraseñas, sanitize_filename, ActiveAgentStatus, DOCX, CiclePhase, ChatSession, contrato API
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend (JS, ejecutar con Node)
