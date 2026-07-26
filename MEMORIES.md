# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

### BUG-010: Módulos duplicados y match String vs &str (4 errores)
- **Causa real**: Al cerrar BUG-005 (`mod regression_new_bugs`), los módulos `stress_tests` (L618 y L1312) y `fault_injection_tests` (L681 y L1367) —que estaban en scopes diferentes— colisionaron en el scope padre. También `match ext` fallaba porque `ext` era `String` (de `to_lowercase()`) y los brazos usaban literales `&str`.
- **Fix aplicado**:
  1. Renombrar `mod stress_tests` en L1312 → `mod stress_tests_extended` (tests de carga masiva: 100k mensajes, 10k archivos, JSON parse)
  2. Renombrar `mod fault_injection_tests` en L1367 → `mod fault_injection_tests_extended` (tests de caracteres especiales, unicode, null bytes)
  3. Cambiar `match ext {` → `match ext.as_str() {` en L1427 para que `String` haga match contra `&str`
- **Verificación**: 15 módulos únicos, sin duplicados. `match ext.as_str()` presente.

### Cadena completa de bugs encadenados en exhaustive_tests.rs:
**BUG-005** (unclosed delimiter) → **BUG-006** (fn sin cuerpo) → **BUG-009** (bloque huérfano) → **BUG-010** (módulos duplicados + match String/&str)
Cada bug ocultaba al siguiente. Al arreglar uno, el compilador revelaba el siguiente.

### BUG-009: Bloque huérfano en exhaustive_tests.rs (línea 899)
- **Causa**: Al eliminar BUG-006, el cuerpo de la función quedó huérfano (`{ let status = ... }`).
- **Fix**: Eliminar 9 líneas del bloque huérfano. Archivo: 1833 líneas.

### BUG-008: include_str!("src/main.rs") no encuentra archivo (error os 3)
- **Causa**: Replace global de BUG-007 rompió `include_str!` en `api_contract_tests`.
- **Fix**: Revertir 5 `include_str!("src/main.rs")` → `include_str!("../src/main.rs")`.
- **Regla**: `include_str!` = relativo al archivo fuente. `std::fs::read_to_string` = relativo al CWD.

### BUG-007: Rutas incorrectas en integration_tests.rs
- **Causa**: `std::fs::read_to_string("../src/")` usa CWD (raíz), no el directorio del archivo fuente.
- **Fix**: Cambiar `"../src/` → `"src/"` en 11 rutas.

### BUG-006: Función sin cuerpo (línea 828)
- `fn estado_agente_con_todos_los_campos_null_o_default()` sin `{ }`.
- **Fix**: Eliminar líneas 827-828.

### BUG-005: Unclosed delimiter (línea 974)
- `mod regression_new_bugs {` nunca se cerraba con `}`.
- **Fix**: Insertar `}` después del último test del módulo (L1099).

### BUG-001: No puede analizar PDFs ni .docx
- **Fix**: `fn extract_text_from_docx()` + `pdf_extract::extract_text()` + detección de extensiones.

### BUG-002: Frontend no muestra mensajes informativos en tiempo real
- **Fix**: `info_messages` se consume SIEMPRE en app.js, no se limpia en `finalizar_tarea`.

### BUG-004: finalizar_tarea devuelve error "No se proporcionó URL"
- **Fix**: Refactorizado a ~15 líneas, usa `mensaje_final`, sin campo `"url"`.

### BUG: addMessage duplicada en app.js, perfil estudio, system prompt local
- **Fix**: Todos arreglados y verificados en código fuente.

## Por qué estos bugs no fueron detectados por tests

### La cadena BUG-005→006→009→010
- **Los errores estaban en el propio archivo de tests.** Si el archivo no compila, ningún test se ejecuta.
- **Solución**: Tests de integridad en archivo separado (`integration_tests.rs`) que verifican `exhaustive_tests.rs` como texto.
- **Lección**: Los errores de sintaxis se encadenan. Arreglar uno revela el siguiente. Siempre verificar balance de llaves ANTES de compilar.

### BUG-008/007 (rutas incorrectas)
- `include_str!` es relativo al archivo fuente. `std::fs::read_to_string` es relativo al CWD.
- **Lección**: NUNCA hacer replace global de rutas.

## Verificación de bugs viejos (estado actual — verificado 2025-07)

| Bug | Estado | Verificación |
|-----|--------|-------------|
| PDF/DOCX | ✅ | `fn extract_text_from_docx`, `pdf_extract::extract_text`, `zip::ZipArchive`, `quick_xml::Reader`, `ext == "pdf"/"docx"` — TODO presente. `pdftotext` NO existe |
| finalizar_tarea URL | ✅ | ~15 líneas, `mensaje_final`, sin `"url"`, sin `info_messages.clear()` |
| System prompt local | ✅ | `load_local_prompt`, `get_project_path`, `Project Specific Prompt:`, `fn load_global_prompt`, `fn load_local_prompt`, `globalPrompt.json`, `localPrompt.json` |
| Mensajes en tiempo real | ✅ | `showInfoToast`, `startAgentMonitoring`, `lastInfoMessageCount`, `info_messages: Vec<String>` |
| addMessage | ✅ | 1 `function addMessage`, `sendMessageToAgent`, `function init`, `init()` |
| Perfil estudio | ✅ | `loadStudyProfile`, `/api/study/profile`, `study_get_profile`, `profile_exists_on_disk`, `profile.json` |
| JS sintaxis | ✅ | Balanceado: 252 `{}`, 745 `()`, 31 `[]` — delta 0 |
| Módulos duplicados | ✅ | 15 módulos únicos en exhaustive_tests.rs, 0 duplicados |
| match String/&str | ✅ | `match ext.as_str()` en fault_injection_tests_extended |

## APIs y comportamiento verificado
- `POST /api/chat` spawnea el agente en `tokio::spawn`
- `GET /api/agent/status` devuelve `{"status":"ok","active":bool,"finished":bool,"final_message":...,"info_messages":[...]}`
- `Path::extension()` para `.gitignore` devuelve `None`
- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD

## Cambios estructurales (v3.1)
- `tests/exhaustive_tests.rs`: 1833 líneas. 15 módulos únicos: `source_code_verification_tests`, `regression_tests`, `integration_tests`, `stress_tests`, `fault_injection_tests`, `edge_case_tests`, `smoke_tests`, `regression_new_bugs`, `user_requested_test_names`, `additional_regression_tests`, `stress_tests_extended`, `fault_injection_tests_extended`, `additional_edge_case_tests`, `e2e_tests`, `regression_historical`
- `tests/integration_tests.rs`: 1197 líneas. 10 módulos. 25 `include_str!` con rutas correctas.
- `app.js`: Balanceado. `addMessage` 1 vez. `startAgentMonitoring` consume `info_messages` SIEMPRE.
- `agent.rs`: `extract_text_from_docx()`, `pdf_extract::extract_text()`, `finalizar_tarea` ~15 líneas.

## Archivos de tests (v3.1)
- `tests/exhaustive_tests.rs` (1833 líneas) — 15 módulos: source code verification, regresión, integración, estrés, inyección de fallos, casos límite, smoke, e2e, + variantes extended
- `tests/integration_tests.rs` (1197 líneas) — StudyEngine, UserStore, sanitize_filename, ActiveAgentStatus, DOCX, CiclePhase, ChatSession, contrato API, integridad de archivos, regresión de bugs viejos
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend (JS, Node)
