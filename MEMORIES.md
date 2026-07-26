# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

### BUG-011: 2 tests fallaban por assertions incorrectos (no bugs reales en el código)
- **Test 1: `agent_rs_finalizar_tarea_no_exige_parametro_url`**
  - **Causa**: El test buscaba `"finalizar_tarea"` y delimitaba el schema con `"=> {"`, pero `"=> {"` capturaba handlers de otros tools incluyendo `image_fetch` que sí tiene `"url"` en required. El `schema_block` era demasiado grande y contenía `"url"` de otros schemas.
  - **Fix**: Reescribir el test para buscar `"required"` después de `"finalizar_tarea"` y acotar al primer `]` (fin del array required). Así solo verifica el contenido de `"required": ["mensaje_final"]`.
- **Test 2: `agent_rs_read_file_tiene_manejo_errores`**
  - **Causa**: El test buscaba la frase `"No existe"` en el handler de `read_file`, pero el código real usa `"Error leyendo archivo"` (de `format!("Error leyendo archivo: {}", e)`).
  - **Fix**: Cambiar el assert de `"No existe"` a `"Error leyendo archivo"`.
- **Lección**: Los tests que verifican strings en código fuente pueden romperse si el código cambia su wording. Usar frases que sean estables o verificar múltiples variantes.

### BUG-010: Módulos duplicados y match String vs &str (4 errores)
- **Causa real**: Al cerrar BUG-005 (`mod regression_new_bugs`), los módulos `stress_tests` (L618 y L1312) y `fault_injection_tests` (L681 y L1367) colisionaron. También `match ext` fallaba porque `ext` era `String` y los brazos usaban `&str`.
- **Fix**: Renombrar a `stress_tests_extended`, `fault_injection_tests_extended`, y `match ext.as_str()`.

### Cadena completa de bugs encadenados en exhaustive_tests.rs:
**BUG-005** → **BUG-006** → **BUG-009** → **BUG-010** → **BUG-011**
Cada bug ocultaba al siguiente. Al arreglar uno, el compilador o los tests revelaban el siguiente.

### BUG-005: Unclosed delimiter (línea 974)
- `mod regression_new_bugs {` nunca se cerraba con `}`. Insertada llave.

### BUG-006: Función sin cuerpo (línea 828)
- `fn estado_agente_con_todos_los_campos_null_o_default()` sin `{ }`. Eliminada.

### BUG-009: Bloque huérfano (línea 899)
- Cuerpo de BUG-006 quedó huérfano. Eliminado.

### BUG-007: Rutas incorrectas en integration_tests.rs
- `std::fs::read_to_string("../src/")` → `"src/"`.

### BUG-008: include_str!("src/main.rs") roto
- Replace global de BUG-007 rompió `include_str!` en `api_contract_tests`. Revertido.

### BUG-001: PDF/DOCX
- **Fix**: `fn extract_text_from_docx()` + `pdf_extract::extract_text()` + detección de extensiones. Verificado en agent.rs.

### BUG-002: Mensajes informativos en tiempo real
- **Fix**: `info_messages` se consume SIEMPRE en app.js, no se limpia en `finalizar_tarea`. Verificado.

### BUG-004: finalizar_tarea error URL
- **Fix**: `"required": ["mensaje_final"]`, sin `"url"`. Verificado en schema.

### BUGs: addMessage duplicada, perfil estudio, system prompt local
- **Fix**: Todos arreglados y verificados en código fuente.

## Por qué estos bugs no fueron detectados por tests

### La cadena BUG-005→006→009→010→011
- **Los errores estaban en el propio archivo de tests.** Si el archivo no compila, ningún test se ejecuta.
- **Solución**: Tests de integridad en archivo separado (`integration_tests.rs`) que verifican `exhaustive_tests.rs` como texto.
- **Lección**: Los errores de sintaxis se encadenan. Arreglar uno revela el siguiente. Siempre verificar balance de llaves ANTES de compilar.

### BUG-011 (asserts incorrectos)
- Tests que verifican strings literales en código fuente son frágiles. Si el código cambia su wording, el test falla sin que haya un bug real.
- **Solución**: Acotar las búsquedas de strings al contexto correcto (ej: delimitar por `[`/`]` en lugar de `"=> {"`) y usar frases estables.

## Verificación completa de bugs viejos (2025-07, tras arreglar BUG-011)

| Bug | Estado | Evidencia en código fuente |
|-----|--------|---------------------------|
| PDF/DOCX | ✅ | `fn extract_text_from_docx`, `pdf_extract::extract_text`, `zip::ZipArchive`, `quick_xml::Reader`, `ext == "pdf"`, `ext == "docx"` |
| finalizar_tarea URL | ✅ | `"required": ["mensaje_final"]`, sin `"url"` en required |
| System prompt local | ✅ | `load_local_prompt`, `get_project_path`, `Project Specific Prompt:`, `fn load_global_prompt`, `fn load_local_prompt` |
| Mensajes en tiempo real | ✅ | `showInfoToast`, `startAgentMonitoring`, `lastInfoMessageCount`, `info_messages: Vec<String>` |
| addMessage | ✅ | 1 `function addMessage`, `sendMessageToAgent`, `function init`, `init()` |
| Perfil estudio | ✅ | `loadStudyProfile`, `/api/study/profile`, `study_get_profile`, `profile_exists_on_disk` |
| JS sintaxis | ✅ | 252 `{}`, 745 `()`, 31 `[]` — delta 0 |
| Módulos duplicados | ✅ | 15 módulos únicos en exhaustive_tests.rs |
| match String/&str | ✅ | `match ext.as_str()` |
| Tests pasan | ✅ | 121 de 123 → ahora 123/123 (BUG-011 corregido) |

## Resultados de cargo test (tras BUG-011)
- lib.rs: 38/38 pasan
- main.rs: 40/40 pasan
- exhaustive_tests.rs: 123/123 pasan (0 failures)
- integration_tests.rs: por verificar (requiere compilación)
- **0 warnings**

## APIs y comportamiento verificado
- `POST /api/chat` spawnea el agente en `tokio::spawn`
- `GET /api/agent/status` devuelve `{"status":"ok","active":bool,"finished":bool,"final_message":...,"info_messages":[...]}`
- `Path::extension()` para `.gitignore` devuelve `None`
- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD

## Cambios estructurales (v3.2)
- `tests/exhaustive_tests.rs`: 1835 líneas. 15 módulos únicos. BUG-011 corregido: test `agent_rs_finalizar_tarea_no_exige_parametro_url` reescrito con delimitación correcta, test `agent_rs_read_file_tiene_manejo_errores` con assert actualizado.
- `tests/integration_tests.rs`: 1197 líneas. 10 módulos.
- `app.js`: Balanceado. `addMessage` 1 vez.
- `agent.rs`: `extract_text_from_docx()`, `pdf_extract::extract_text()`, `finalizar_tarea` ~15 líneas, `read_file` con `"Error leyendo archivo"`.

## Archivos de tests (v3.2)
- `tests/exhaustive_tests.rs` (1835 líneas) — 15 módulos, 123 tests
- `tests/integration_tests.rs` (1197 líneas) — 10 módulos, incluyendo `test_file_integrity_tests` y `regression_bugs_tests`
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend (JS, Node)
