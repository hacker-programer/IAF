# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

### BUG-009: Bloque huérfano en exhaustive_tests.rs (línea 899)
- **Causa real**: Al eliminar BUG-006 (`fn estado_agente_con_todos_los_campos_null_o_default()` sin cuerpo), solo se eliminaron `#[test]` y la declaración `fn`, pero el cuerpo `{ let status = iaf::state::ActiveAgentStatus::default(); ... }` quedó como bloque huérfano en la línea 899, causando `expected item, found {`.
- **Fix aplicado**: Eliminar las 9 líneas del bloque huérfano (líneas 899-907). Archivo pasó de 1842 a 1833 líneas.
- **Lección**: Al eliminar una función, eliminar TANTO la declaración como el cuerpo. Un delete incompleto deja bloques huérfanos que el compilador rechaza.

### BUG-008: include_str!("src/main.rs") no encuentra archivo (error os 3)
- **Causa real**: Al corregir BUG-007 (rutas `../src/` → `src/` para `std::fs::read_to_string`), el replace global también afectó los `include_str!("../src/main.rs")` en `api_contract_tests`, dejándolos como `include_str!("src/main.rs")`. `include_str!` es relativo al archivo fuente (`tests/integration_tests.rs`), por lo que `src/main.rs` apunta a `tests/src/main.rs` que no existe.
- **Fix aplicado**: Revertir SOLO los `include_str!("src/main.rs")` de `api_contract_tests` a `include_str!("../src/main.rs")` (5 líneas: 679, 689, 696, 702, 708).
- **Regla de oro**: `include_str!` usa ruta relativa al ARCHIVO FUENTE. `std::fs::read_to_string` usa ruta relativa al CWD (raíz del proyecto). NUNCA hacer replace global de rutas que afecte ambos tipos.

### BUG-007: Rutas incorrectas en integration_tests.rs (../src/ en lugar de src/)
- **Causa real**: `std::fs::read_to_string("../src/main.rs")` usa el CWD (raíz del proyecto) como base, no el directorio del archivo fuente. `../src/` desde la raíz del proyecto apunta a `C:\Users\Fa\Desktop\Auto IAF\src\` que NO existe.
- **Fix aplicado**: Reemplazar `"../src/` por `"src/` en las 11 rutas del test `archivos_fuente_principales_tienen_llaves_balanceadas`.

### BUG-006: Función sin cuerpo en exhaustive_tests.rs (línea 828)
- **Causa real**: `fn estado_agente_con_todos_los_campos_null_o_default()` estaba declarada sin bloque `{ }`.
- **Fix aplicado**: Eliminar las líneas 827-828 (`#[test]` + declaración de función sin cuerpo).
- **BUG-009 fue consecuencia directa**: El cuerpo de esta función quedó huérfano.

### BUG-005: Unclosed delimiter en exhaustive_tests.rs impide compilación
- **Causa real**: `mod regression_new_bugs {` (línea 974) nunca tenía `}` de cierre.
- **Fix aplicado**: Insertar `}` después del último test del módulo (línea 1099).

### Cadena de bugs encadenados (BUG-005 → BUG-006 → BUG-009):
1. **BUG-005** ocultó todos los demás errores (el archivo no compilaba)
2. Al arreglar BUG-005, el compilador reveló **BUG-006** (fn sin cuerpo)
3. Al arreglar BUG-006 (eliminando solo declaración), quedó **BUG-009** (bloque huérfano)
4. **BUG-007** y **BUG-008** fueron errores introducidos durante las correcciones

### BUG-001: No puede analizar PDFs ni .docx
- **Fix aplicado**: `fn extract_text_from_docx()` + `pdf_extract::extract_text()` + detección de extensiones en `read_file`.
- **Verificación**: TODO presente en agent.rs. `pdftotext` NO existe.

### BUG-002: El frontend no muestra los mensajes informativos en tiempo real
- **Fix aplicado**: `info_messages` se consume SIEMPRE en app.js, no se limpia en `finalizar_tarea`.

### BUG-004: finalizar_tarea devuelve error "No se proporcionó URL"
- **Fix aplicado**: Refactorizado a ~26 líneas, usa `mensaje_final`, sin campo `"url"`.

### BUG: addMessage duplicada en app.js
- **Fix**: 1 sola definición de `addMessage`.

### BUG: No carga el perfil en modo estudio en el frontend
- **Fix**: `StudyEngine` con rutas correctas, `loadStudyProfile` en app.js.

### BUG: No ve el system prompt local ni el perfil ni el directorio del proyecto
- **Fix**: `load_local_prompt()`, `get_project_path()`, `Project Specific Prompt:` en agent.rs.

## Por qué estos bugs no fueron detectados por tests (Lección 2025-2026)

### BUG-009/006/005 (errores encadenados en tests)
- **Los errores estaban en el propio archivo de tests. Si el archivo no compila, ningún test se ejecuta.**
- **Solución**: Tests de integridad en archivo separado (`integration_tests.rs`) que verifican `exhaustive_tests.rs` como texto.
- **Lección**: Siempre verificar balance de llaves y estructura ANTES de intentar compilar. Los errores de sintaxis se encadenan.

### BUG-008/007 (rutas incorrectas)
- `include_str!` = relativo al archivo fuente. `std::fs::read_to_string` = relativo al CWD.
- **Lección**: NUNCA hacer replace global de rutas. Hacer replaces quirúrgicos.

## Verificación de bugs viejos (estado actual — verificado 2025-07)

| Bug | Estado | Verificación exhaustiva |
|-----|--------|-------------------------|
| PDF/DOCX | ✅ | `fn extract_text_from_docx`, `pdf_extract::extract_text`, `zip::ZipArchive`, `quick_xml::Reader`, `ext == "pdf"`, `ext == "docx"` — TODO presente |
| finalizar_tarea URL | ✅ | ~26 líneas, usa `mensaje_final`, sin `"url"`, sin `info_messages.clear()` |
| System prompt local | ✅ | `load_local_prompt`, `get_project_path`, `Project Specific Prompt:`, `fn load_global_prompt`, `fn load_local_prompt`, `globalPrompt.json`, `localPrompt.json` |
| Mensajes en tiempo real | ✅ | `showInfoToast`, `startAgentMonitoring`, `lastInfoMessageCount`, `info_messages: Vec<String>` |
| addMessage duplicada | ✅ | 1 `function addMessage`, `sendMessageToAgent`, `function init`, `init()` |
| Perfil estudio | ✅ | `loadStudyProfile`, `/api/study/profile`, `study_get_profile`, `profile_exists_on_disk`, `profile.json` |
| JS frontend sintaxis | ✅ | Balanceado: 252 `{}`, 745 `()`, 31 `[]` — delta 0 |

## APIs y comportamiento verificado
- `POST /api/chat` spawnea el agente en `tokio::spawn`
- `GET /api/agent/status` devuelve `{"status":"ok","active":bool,"finished":bool,"final_message":...,"info_messages":[...]}`
- `Path::extension()` para `.gitignore` devuelve `None`
- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD

## Cambios estructurales (v3.0)
- `tests/exhaustive_tests.rs`: 1833 líneas. BUG-005, BUG-006, BUG-009 arreglados. 15 módulos de tests.
- `tests/integration_tests.rs`: 1197 líneas. 10 módulos incluyendo `test_file_integrity_tests` (4 tests) y `regression_bugs_tests` (20 tests). 25 `include_str!` con rutas correctas.
- `app.js`: Balanceado. `addMessage` 1 vez. `startAgentMonitoring` consume `info_messages` SIEMPRE.
- `agent.rs`: `extract_text_from_docx()`, `pdf_extract::extract_text()`, `finalizar_tarea` ~26 líneas.

## Archivos de tests (v3.0)
- `tests/exhaustive_tests.rs` (1833 líneas) — Source code verification, regresión, integración, estrés, inyección de fallos, casos límite, smoke tests, e2e tests
- `tests/integration_tests.rs` (1197 líneas) — StudyEngine, UserStore, sanitize_filename, ActiveAgentStatus, DOCX, CiclePhase, ChatSession, contrato API, integridad de archivos, regresión de bugs viejos
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend (JS, Node)
