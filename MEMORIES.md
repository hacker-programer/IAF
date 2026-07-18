# MEMORIES.md — Limitaciones y Descubrimientos Técnicos

## [2026-07-18] Compilación Exitosa y MiniMax M3
- **agent.rs compila** con 26 herramientas incluyendo no_sync, reportar_fallo, y MiniMax M3
- **MiniMax M3** usa providers: `{"order": ["DeepInfra"], "allow_fallbacks": true}` via OpenRouter
- **Todas las referencias a Qwen eliminadas** (descripción, handlers, comentarios)
- Target dir: `C:\Users\Fa\AppData\Local\Temp\cargo-target` (por permisos)
- **0 errores, 71 warnings** (todos de dead_code — normales en proyecto en desarrollo)

## [2026-07-18] Sistema de Permisos y Límites Completo
- **Permisos booleanos**: `admin`, `modo_programador`, `modo_estudio`, `editar_system_prompt_global`, `editar_system_prompt_local`
- **Límites por horario**: WeeklySchedule con HashMap días → Vec<(hora_inicio, hora_fin)>
- **Límites detallados**: peticiones_por_minuto/hora, limite_iteraciones, limite_tokens_entrada/salida, activacion
- `UserAccount::has_study_access()` y `has_programming_access()` como métodos
- `UserAccount::can_edit_global_prompt()` y `can_edit_local_prompt()`

## [2026-07-18] Sistema de Ciclos (CicleState)
- Modo programación dividido en 6 ciclos: Implementacion → Optimizacion → BusquedaBugs → Reduccion → SegundaBusquedaBugs → Terminar
- Estado guardado en `.config/data/<username>/<project>/cicle.json`
- `CiclePhase::next()` para avanzar automáticamente
- Endpoints: `GET /api/cicles/:project_name`, `PUT /api/cicles/:project_name`

## [2026-07-18] System Prompts por Usuario y Proyecto
- **Global**: `.config/data/<username>/globalPrompt.json`
- **Local**: `.config/data/<username>/<project>/localPrompt.json`
- Endpoints: `GET/POST /api/prompts/global`, `POST /api/prompts/global/reset`, `GET /api/prompts/local?project=X`, `POST /api/prompts/local`
- `AppState::load_global_prompt(&username)` y `AppState::load_local_prompt(&username, &project)`

## [2026-07-18] Herramientas del Agente (26 total)
- **no_sync**: Configurar sincronización selectiva con Patrón Composite (include/exclude patterns)
- **reportar_fallo**: Reportar fallos internos de IAF a `.config/fallos_reportados.json`
- **analyze_images**: Migrado a MiniMax M3 via OpenRouter con providers DeepInfra
- MiniMax M3 tiene mayor ventana de contexto y soporta audio/video nativamente

## [2026-07-17] Sistema de Autenticación Dual
- **Contraseñas**: argon2id (OWASP recommended) para usuarios normales
- **Nonce**: Ed25519 challenge-response para admins
- **Coexistencia**: UserAccount tiene `password_hash` y `public_key` como Option
- **Puerto 80**: auto-admin, sin auth. Puerto 8080: requiere login siempre

## [2026-07-17] Sistema de Estudio Completo
- **Fase Exploración**: Perfilado (edad, neurología, juegos, hobbies, YouTubers)
- **Fase Explotación**: Método optimizado tras 3+ hipótesis efectivas
- **Knowledge Base**: Semi-global (compartida entre proyectos, local del usuario)
- **Engagement**: Calculado por gaps entre respuestas del usuario
- **Principio**: NUNCA hacer el código por el alumno. Forjar autonomía.

## [2026-07-17] Cliente-Servidor
- El servidor NUNCA ejecuta comandos de usuarios normales
- Cliente binario separado (`iaf-client`) hace toda la ejecución local
- Protocolo: connect → poll → execute → respond → loop

## [2026-07-17] Chats
- Formato: `<titulo_sanitizado>-<UUID>.json`
- Usuarios normales: `.config/chats/<username>/`
- Admin/Port80: `.config/chats/`
- Migración automática de chats viejos al iniciar

## [2026-07-18] Ediciones parciales de archivos (start_line/end_line) — CRÍTICO
- **PROBLEMA**: write_file_with_commit con start_line/end_line CORROMPE archivos sistemáticamente
- **Síntomas**: delimitadores desbalanceados, líneas duplicadas, cierres `]` y `)` perdidos
- **Causa**: desfase de una línea entre el rango especificado y el contenido real; no hay forma confiable de predecir el número exacto de línea
- **SOLUCIÓN**: Usar PowerShell para modificar archivos in-memory con ArrayList, verificando antes de guardar
- **Alternativa segura**: Escribir archivo completo sin start_line/end_line

## [2026-07-08] Falsos positivos en validator.rs
- `detect_duplicate_definitions` marca como duplicados métodos con mismo nombre en distintos `impl` blocks
- Solución aplicada: `extract_impl_struct_name()` y `extract_def_name_with_context()`
- Aún hay falsos positivos residuales (no bloquean compilación)

## [2026-07-08] Google Search siempre fallaba
- Causa: Google bloquea scrapers
- Solución: DuckDuckGo Lite como fuente principal

## [2026-07-08] Truncado arbitrario de tool results
- Solución: ToolResultStore con IDs y paginación

## [2026-07-08] Sin capacidad de paralelismo
- Solución: SubAgentManager + sub_agent.rs
