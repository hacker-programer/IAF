# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

### BUG-012: Race condition en test_engine() — directorio compartido entre tests paralelos
- **Causa real**: La función `test_engine()` en `src/study.rs` usaba un directorio fijo `iaf_test_study` para TODOS los tests. Al ejecutarse `cargo test`, los tests de Rust corren en PARALELO por defecto. Un test llamaba `remove_dir_all(&tmp)` mientras otro test estaba usando ese mismo directorio para `save_profile()`, causando `os error 3` ("no puede encontrar la ruta especificada") de forma intermitente.
- **Fix aplicado**: Usar `AtomicU32` como contador global (`TEST_DIR_COUNTER`) incrementado en cada llamada a `test_engine()`, generando directorios únicos: `iaf_test_study_0`, `iaf_test_study_1`, etc. Cada test ahora tiene su propio directorio aislado, eliminando la race condition.
- **Verificación**: `test_profile_crud`, `test_knowledge_tracking`, `test_engagement`, `reg_stu007_profile_exists_on_disk_is_accurate` usan `test_engine()` y ahora son thread-safe. Los tests REG-STU-001 a REG-STU-010 ya usaban directorios hardcodeados únicos.
- **Lección**: NUNCA compartir un directorio temporal entre tests paralelos. Usar IDs únicos (atómico, UUID, o `std::thread::current().id()`) para aislar cada test.

### BUG-011: 2 tests fallaban por assertions incorrectos
- **Test 1**: `agent_rs_finalizar_tarea_no_exige_parametro_url` — delimitaba mal el schema con `"=> {"`, capturando otros tools. Reescribí para buscar `"required"` y acotar al primer `]`.
- **Test 2**: `agent_rs_read_file_tiene_manejo_errores` — buscaba `"No existe"` pero el código real usa `"Error leyendo archivo"`. Corregido.

### Cadena BUG-005→006→009→010→011→012:
1. **BUG-005**: `mod regression_new_bugs` sin `}` → insertada llave
2. **BUG-006**: `fn estado_agente...()` sin cuerpo → eliminada
3. **BUG-009**: Bloque huérfano (cuerpo de BUG-006) → eliminado
4. **BUG-007**: Rutas `"../src/"` para `read_to_string` → `"src/"`
5. **BUG-008**: `include_str!("src/main.rs")` roto → `"../src/main.rs"`
6. **BUG-010**: Módulos duplicados + match String/&str → renombrados + `.as_str()`
7. **BUG-011**: 2 tests con assertions incorrectos → reescritos
8. **BUG-012**: Race condition en `test_engine()` → `AtomicU32` para IDs únicos

### BUG-001: PDF/DOCX — `fn extract_text_from_docx()` + `pdf_extract::extract_text()`. Verificado.
### BUG-002: Mensajes en tiempo real — `info_messages` se consume SIEMPRE. Verificado.
### BUG-004: finalizar_tarea URL — `"required": ["mensaje_final"]`. Verificado.

## Por qué estos bugs no fueron detectados por tests

### BUG-012 (race condition)
- **Los tests pasaban en ejecución secuencial** (`cargo test -- --test-threads=1`), pero fallaban intermitentemente con `cargo test` normal (paralelo).
- **No había tests que verificaran thread-safety de `test_engine()`**.
- **Solución**: Aislar cada test con su propio directorio. Usar contadores atómicos para generar IDs únicos.

### La cadena BUG-005→011
- **Los errores estaban en el propio archivo de tests.** Si el archivo no compila, ningún test se ejecuta.
- **Solución**: Tests de integridad en archivo separado (`integration_tests.rs`).

## Verificación completa de bugs viejos (2025-07, tras BUG-012)

| Bug | Estado | Evidencia |
|-----|--------|-----------|
| PDF/DOCX | ✅ | `fn extract_text_from_docx`, `pdf_extract::extract_text`, `zip::ZipArchive`, `quick_xml::Reader` |
| finalizar_tarea URL | ✅ | `"required": ["mensaje_final"]`, sin `"url"` |
| System prompt local | ✅ | `load_local_prompt`, `get_project_path`, `Project Specific Prompt:` |
| Mensajes en tiempo real | ✅ | `showInfoToast`, `startAgentMonitoring`, `lastInfoMessageCount` |
| addMessage | ✅ | 1 `function addMessage` |
| Perfil estudio | ✅ | `loadStudyProfile`, `/api/study/profile`, `profile_exists_on_disk` |
| JS sintaxis | ✅ | 252 `{}`, 745 `()`, 31 `[]` — delta 0 |
| Módulos duplicados | ✅ | 15 módulos únicos |
| match String/&str | ✅ | `match ext.as_str()` |
| Race condition tests | ✅ | `AtomicU32` counter, directorios únicos por test |

## Resultados de cargo test (esperados tras BUG-012)
- lib.rs: 38/38 ✅
- main.rs: 40/40 ✅
- exhaustive_tests.rs: 123/123 ✅
- integration_tests.rs: por verificar
- **0 warnings**

## APIs y comportamiento verificado
- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD
- `cargo test` ejecuta tests en paralelo por defecto. Usar directorios únicos para cada test.
- `AtomicU32::fetch_add` es thread-safe y no requiere Mutex.

## Cambios estructurales (v3.3)
- `src/study.rs`: `test_engine()` ahora usa `AtomicU32` para IDs únicos. Agregado `use std::sync::atomic::{AtomicU32, Ordering}` en el módulo de tests.
- `tests/exhaustive_tests.rs`: 1835 líneas, 15 módulos, 123 tests.
- `tests/integration_tests.rs`: 1197 líneas, 10 módulos, 24 tests de regresión.
- `app.js`: Balanceado. Todas las funciones requeridas presentes.

## Archivos de tests (v3.3)
- `tests/exhaustive_tests.rs` (1835 líneas) — 15 módulos, 123 tests
- `tests/integration_tests.rs` (1197 líneas) — 10 módulos
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend
