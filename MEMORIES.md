# MEMORIES.md — Limitaciones y Descubrimientos Técnicos

## [2026-07-17] Sistema de Autenticación Dual Implementado
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
