# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

### BUG-008: include_str!("src/main.rs") no encuentra archivo (error os 3)
- **Causa real**: Al corregir BUG-007 (rutas `../src/` → `src/` para `std::fs::read_to_string`), el replace global también afectó los `include_str!("../src/main.rs")` en `api_contract_tests`, dejándolos como `include_str!("src/main.rs")`. `include_str!` es relativo al archivo fuente (`tests/integration_tests.rs`), por lo que `src/main.rs` apunta a `tests/src/main.rs` que no existe.
- **Fix aplicado**: Revertir SOLO los `include_str!("src/main.rs")` de `api_contract_tests` a `include_str!("../src/main.rs")` (5 líneas: 679, 689, 696, 702, 708).
- **Regla de oro**: `include_str!` usa ruta relativa al ARCHIVO FUENTE. `std::fs::read_to_string` usa ruta relativa al CWD (raíz del proyecto). NUNCA hacer replace global de rutas que afecte ambos tipos.
- **Verificación**: Los 25 `include_str!` en `integration_tests.rs` ahora usan rutas correctas: `"../src/..."` para archivos fuente, `"../public/..."` para JS, `"exhaustive_tests.rs"` para archivos en el mismo directorio.

### BUG-007: Rutas incorrectas en integration_tests.rs (../src/ en lugar de src/)
- **Causa real**: `std::fs::read_to_string("../src/main.rs")` usa el CWD (raíz del proyecto) como base, no el directorio del archivo fuente. `../src/` desde la raíz del proyecto apunta a `C:\Users\Fa\Desktop\Auto IAF\src\` que NO existe.
- **Fix aplicado**: Reemplazar `"../src/` por `"src/` en las 11 rutas del test `archivos_fuente_principales_tienen_llaves_balanceadas`.
- **Diferencia clave**: `include_str!("../src/agent.rs")` es relativo al archivo fuente (correcto). `std::fs::read_to_string("../src/agent.rs")` es relativo al CWD (incorrecto; debe ser `src/agent.rs`).
- **Verificación**: Test `archivos_fuente_principales_tienen_llaves_balanceadas` en `integration_tests.rs`.

### BUG-006: Función sin cuerpo en exhaustive_tests.rs (línea 828)
- **Causa real**: `fn estado_agente_con_todos_los_campos_null_o_default()` estaba declarada en línea 828 sin bloque `{ }`. El compilador interpretaba el siguiente `#[test]` como parte de la función.
- **Fix aplicado**: Eliminar las líneas 827-828 (`#[test]` + declaración de función sin cuerpo).
- **Por qué no fue detectado**: El archivo ya tenía BUG-005 (unclosed delimiter), por lo que nunca compiló. Al arreglar BUG-005, el compilador encontró BUG-006.
- **Verificación**: El archivo ahora tiene 1842 líneas (2 menos). La transición entre `extension_con_numeros` y `edge008_nombre_archivo_solo_extension` es limpia.

### BUG-005: Unclosed delimiter en exhaustive_tests.rs impide compilación
- **Causa real**: `mod regression_new_bugs {` (línea 974) nunca tenía `}` de cierre. Los 7 módulos posteriores estaban anidados dentro de `regression_new_bugs` sin que este se cerrara.
- **Fix aplicado**: Insertar `}` después de la línea 1099 (después de `agent_rs_local_prompt_overridea_global`, el último test de `regression_new_bugs`).
- **Por qué no fue detectado**: El error estaba EN el archivo de tests. Si el archivo no compila, ningún test se ejecuta.
- **Solución de prevención**: Se agregó `mod test_file_integrity_tests` en `integration_tests.rs` (archivo separado) que usa `include_str!("exhaustive_tests.rs")` para verificar balance de llaves.
- **Verificación**: Tests en `tests/integration_tests.rs`.

### BUG-001: No puede analizar PDFs ni .docx
- **Causa real**: El `read_file` handler en `agent.rs` solo usaba `fs::read_to_string()`. No detectaba extensiones `.pdf` ni `.docx`.
- **Fix aplicado**: 
  1. Se agregó `fn extract_text_from_docx()` que usa `zip::ZipArchive` + `quick_xml::Reader` para parsear DOCX nativamente.
  2. Se agregó detección de extensión en `read_file`: si es `.pdf` → `pdf_extract::extract_text()`, si es `.docx` → `extract_text_from_docx()`.
  3. Dependencias agregadas en Cargo.toml: `pdf-extract = "0.7"`, `zip = "0.6"`, `quick-xml = "0.31"`.
- **Verificación**: Tests en `exhaustive_tests.rs` y `integration_tests.rs` (módulo `regression_bugs_tests`).

### BUG-002: El frontend no muestra los mensajes informativos en tiempo real
- **Causa real**: 
  1. `startAgentMonitoring()` en `app.js` solo consumía `info_messages` cuando `active || running` era true.
  2. Cuando el agente terminaba (`running=false`), el frontend iba al `else` y nunca veía los últimos mensajes.
  3. `finalizar_tarea` en `agent.rs` hacía `info_messages.clear()` borrando mensajes pendientes.
- **Fix aplicado**:
  1. `app.js`: El consumo de `info_messages` se mueve ANTES del chequeo `active || running`.
  2. `agent.rs`: `finalizar_tarea` YA NO hace `info_messages.clear()`.
  3. `state.rs`: `ActiveAgentStatus` tiene campo `info_messages: Vec<String>`.
  4. `main.rs`: `get_agent_status` incluye `info_messages` y `final_message` en la respuesta JSON.
- **Verificación**: Tests en `exhaustive_tests.rs` y `integration_tests.rs`.

### BUG-004: finalizar_tarea devuelve error "No se proporcionó URL"
- **Causa real**: El handler de `finalizar_tarea` estaba en una sola línea (ilegible), y el agente confundía el error de `image_fetch` con `finalizar_tarea`.
- **Fix aplicado**: `finalizar_tarea` refactorizado a múltiples líneas (~25 líneas), con validación de `mensaje_final` vacío, sin referencia a `url`, sin `info_messages.clear()`.
- **Verificación**: Tests en `exhaustive_tests.rs` y `integration_tests.rs`.

### BUG: addMessage duplicada en app.js (no se puede empezar conversación)
- **Causa real**: `addMessage` definida DOS VECES, la primera incompleta (solo `const div = document.createElement('div');` sin cuerpo).
- **Fix**: Eliminadas líneas duplicadas, dejando una sola definición completa.
- **Verificación**: Test `app_js_add_message_definida_una_sola_vez` y `addmessage_app_js_definida_una_sola_vez`.

### BUG: No carga el perfil en modo estudio en el frontend
- **Fix**: `StudyEngine` usa rutas correctas: `.config/data/<username>/profile.json`, `learnings.json`, `teachingMethod.json`.
- **Verificación**: Tests en `integration_tests.rs` y `exhaustive_tests.rs`.

### BUG: No ve el system prompt local ni el perfil ni el directorio del proyecto
- **Fix**: `agent.rs` carga `local_prompt` desde `state.prompts.projects` y `global_prompt` desde `state.prompts.global_current`. Usa `get_project_path()` para el directorio.
- **Verificación**: Tests en `exhaustive_tests.rs` y `integration_tests.rs`.

## Por qué estos bugs no fueron detectados por tests (Lección 2025-2026)

### BUG-008 (include_str! paths)
- **Causa raíz**: Un replace global de rutas que no distinguió entre `include_str!` (relativo al archivo fuente) y `std::fs::read_to_string` (relativo al CWD).
- **Lección**: NUNCA hacer replace global de rutas en archivos de tests. Siempre hacer replaces quirúrgicos acotados a cada tipo de macro/función.

### BUG-007 (rutas incorrectas)
- `include_str!` usa rutas relativas al archivo fuente, pero `std::fs::read_to_string` usa el CWD.
- **No hay forma de que el compilador detecte esto**: `std::fs::read_to_string` es runtime, no se evalúa en compilación.
- **Solución**: Los tests ahora usan `include_str!` para archivos fuente conocidos y solo usan `std::fs::read_to_string` con rutas corregidas.

### BUG-006 (función sin cuerpo)
- **El archivo nunca compiló debido a BUG-005**, por lo que BUG-006 estaba oculto.
- **Lección**: Los errores de sintaxis se encadenan. Arreglar uno revela el siguiente. Siempre verificar balance de llaves ANTES de intentar compilar.

### BUG-005 (unclosed delimiter)
- **El error estaba en el propio archivo de tests. Si el archivo no compila, ningún test se ejecuta.**
- **Solución**: Tests de integridad en archivo separado (`integration_tests.rs`) que verifican `exhaustive_tests.rs` como texto.

### Lección general: Tests SIMULADOS vs REALES
- Los tests simulados (crear JSON y validar contra sí mismo) NO detectan bugs reales.
- Los tests REALES deben usar:
  - `include_str!` para verificar código fuente
  - Conteo de ocurrencias (`matches()`)
  - Verificación de posiciones relativas (`find()` / `rfind()`)
  - Serialización/deserialización real con `serde_json`
  - Creación de archivos reales en disco

## Verificación de bugs viejos (estado actual — verificado 2025-07)

| Bug | Estado | Verificación exhaustiva en código fuente |
|-----|--------|------------------------------------------|
| PDF/DOCX | ✅ | `fn extract_text_from_docx`, `pdf_extract::extract_text`, `zip::ZipArchive`, `quick_xml::Reader`, `ext == "pdf"`, `ext == "docx"` — TODO presente en agent.rs. `pdftotext` NO existe. |
| finalizar_tarea URL | ✅ | Handler ocupa ~25 líneas, usa `mensaje_final`, NO contiene `"url"`, NO contiene `info_messages.clear()`. |
| System prompt local | ✅ | `load_local_prompt` en agent.rs, `get_project_path`, `Project Specific Prompt:`. `fn load_global_prompt` y `fn load_local_prompt` en state.rs, `globalPrompt.json` y `localPrompt.json`. |
| Mensajes en tiempo real | ✅ | `showInfoToast` en app.js, `startAgentMonitoring`, `lastInfoMessageCount`. `info_messages: Vec<String>` en state.rs. `info_messages` en main.rs. |
| addMessage duplicada | ✅ | Exactamente 1 `function addMessage` en app.js. `sendMessageToAgent`, `function init`, `init()` — todo presente. |
| Perfil estudio | ✅ | `function loadStudyProfile` y `/api/study/profile` en app.js. `/api/study/profile` y `study_get_profile` en main.rs. `profile_exists_on_disk` y `profile.json` en study.rs. |
| JS frontend sintaxis | ✅ | Balanceado: 252 `{}`, 745 `()`, 31 `[]` — delta 0 en todos. |

## APIs y comportamiento verificado
- `POST /api/chat` spawnea el agente en `tokio::spawn` después de guardar el mensaje
- `GET /api/agent/status` devuelve `{"status":"ok","active":bool,"finished":bool,"final_message":...,"info_messages":[...]}`
- `POST /api/agent/responder` acepta `{"respuesta":"..."}` y limpia `esperando_respuesta_usuario`
- `POST /api/agent/aprobar_plan` acepta `{"aprobar":bool}` y limpia `esperando_aprobacion_plan`
- `Path::extension()` para `.gitignore` devuelve `None` (el `.` inicial es parte del stem, no extensión)
- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD

## Cambios estructurales (v2.9)
- `lib.rs`: `pub mod utils; pub mod state; pub mod auth; pub mod study; pub mod desktop; pub mod sync;`
- `state.rs`: `ActiveAgentStatus` tiene `info_messages: Vec<String>`, `finished: bool`, `final_message: Option<String>`
- `agent.rs`: `extract_text_from_docx()`, `pdf_extract::extract_text()`, `finalizar_tarea` refactorizado (~25 líneas), `load_local_prompt()`, `get_project_path()`
- `app.js`: `startAgentMonitoring()` consume `info_messages` SIEMPRE. `addMessage` definida 1 vez. Balanceado: 252 `{}`, 745 `()`, 31 `[]`.
- `tests/exhaustive_tests.rs`: BUG-005 y BUG-006 arreglados. 1842 líneas. Todos los módulos cerrados correctamente.
- `tests/integration_tests.rs`: ~1197 líneas. Módulos: `study_engine_tests`, `sanitize_filename_tests`, `active_agent_status_tests`, `docx_tests`, `user_store_tests`, `cicle_phase_tests`, `chat_session_tests`, `api_contract_tests` (rutas corregidas, BUG-008), `test_file_integrity_tests`, `regression_bugs_tests` (20 tests de verificación de código fuente).

## Dependencias agregadas
- `pdf-extract = "0.7"` — extracción de texto de PDFs
- `zip = "0.6"` — lectura de archivos DOCX
- `quick-xml = "0.31"` — parseo rápido de XML en DOCX

## Archivos de tests (v2.9)
- `tests/exhaustive_tests.rs` (1842 líneas) — Source code verification, regresión, integración, estrés, inyección de fallos, casos límite, smoke tests, e2e tests
- `tests/integration_tests.rs` (~1197 líneas) — StudyEngine con disco, UserStore, sanitize_filename, ActiveAgentStatus, DOCX, CiclePhase, ChatSession, contrato API, integridad de archivos (balance de llaves), regresión de bugs viejos (20 tests)
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend (JS, Node)
