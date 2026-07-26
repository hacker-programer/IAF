# MEMORIES.md — Registro de Bugs, Limitaciones y Lecciones Aprendidas

## Bugs Corregidos (Sesión 2025-2026)

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
| JS sintaxis | ✅ | 252 `{}`, 745 `()`, 31 `[]` — delta 0 |
| Módulos duplicados | ✅ | 15 módulos únicos |
| match String/&str | ✅ | `match ext.as_str()` |
| Race condition tests | ✅ | `AtomicU32` counter |
| Métodos study.rs | ✅ | `get_user_projects`, `build_study_system_prompt` restaurados |

## APIs y comportamiento verificado
- `include_str!` es relativo al archivo fuente; `std::fs::read_to_string` es relativo al CWD
- `cargo test` ejecuta tests en paralelo por defecto
- `write_file_with_commit` con archivos grandes puede truncar el contenido
- Verificar métodos cross-file después de cada edición

## Cambios estructurales (v3.4)
- `src/study.rs`: 973 líneas. `get_user_projects` (L583), `build_study_system_prompt` (L515), `test_engine()` con `AtomicU32`.
- `tests/exhaustive_tests.rs`: 1835 líneas, 15 módulos, 123 tests.
- `tests/integration_tests.rs`: 1197 líneas, 10 módulos, 24 tests de regresión.
- `app.js`: Balanceado.

## Archivos de tests (v3.4)
- `tests/exhaustive_tests.rs` (1835 líneas) — 15 módulos, 123 tests
- `tests/integration_tests.rs` (1197 líneas) — 10 módulos
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend
