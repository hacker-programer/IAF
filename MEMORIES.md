# MEMORIES.md — Memoria Persistente del Proyecto IAF

> Este archivo registra limitaciones técnicas, fallos de configuración y comportamientos de APIs
> descubiertos durante el desarrollo. Su objetivo es **minimizar llamadas innecesarias al modelo**,
> reducir cómputo redundante y evitar llamadas repetitivas de red.

---

## 🐛 Bugs Conocidos y Solucionados

### [2026-07-06] Mensaje "TRUNCADO POR EL SISTEMA" confundía al agente → reversión destructiva
- **Estado**: CORREGIDO (2026-07-06)
- **Archivo**: `src/agent.rs` (~línea 1480)
- **Causa**: El mensaje `[TRUNCADO POR EL SISTEMA. El resultado es demasiado grande...]` era interpretado por el agente como que el archivo en disco estaba corrupto. El agente respondía ejecutando `git checkout` o `git reset --hard`, perdiendo todo el progreso.
- **Solución**: Se cambió el mensaje a `[VISUALIZACIÓN PARCIAL — El archivo en disco NO está truncado. Solo se muestra una parte de la respuesta...]`. Además se agregó regla #4 en el system prompt que prohíbe explícitamente revertir cambios con git basándose en este mensaje.

### [2026-07-06] Código duplicado por ediciones parciales (start_line/end_line)
- **Estado**: CORREGIDO (2026-07-06)
- **Causa**: El agente usaba `start_line`/`end_line` en `write_file_with_commit` para hacer ediciones parciales. Tras varias ediciones secuenciales al mismo archivo, los números de línea se desactualizaban, resultando en código insertado sin eliminar el original → funciones duplicadas, constantes duplicadas.
- **Solución múltiple**:
  1. **System prompt**: Nueva sección "REGLAS OBLIGATORIAS DE EDICIÓN DE CÓDIGO" que PROHÍBE usar start_line/end_line en write_file_with_commit. El agente DEBE escribir el archivo completo.
  2. **validator.rs**: Nueva función `detect_duplicate_definitions()` que detecta definiciones duplicadas de `fn`, `struct`, `enum`, `trait`, `const`, `static`, `mod`.
  3. **Regla**: Leer siempre el archivo completo antes de editarlo.

### [2026-07-06] Doble `play_error_beep()` en write_handler
- **Estado**: CORREGIDO (2026-07-06)
- **Archivo**: `src/agent.rs`, handler `write_file_with_commit`
- **Causa**: Código duplicado accidental: dos llamadas consecutivas a `play_error_beep()` antes de `break 'write_handler`.
- **Solución**: Se eliminó la primera llamada redundante.

### [2026-07-06] Comentario "SANITIZACIÓN DE SEGURIDAD" duplicado
- **Estado**: CORREGIDO (2026-07-06)
- **Archivo**: `src/agent.rs`, handler `execute_powershell`
- **Solución**: Se eliminó la línea duplicada.

### [2026-07-06] Comentario duplicado en `compress_active_messages_if_needed`
- **Estado**: CORREGIDO (2026-07-06)
- **Archivo**: `src/agent.rs`
- **Solución**: Se eliminó la segunda ocurrencia de "Si llegamos aquí, la compresión falló o fue incompleta".

### [2026-07-06] Doble `discover_projects()` en `main()`
- **Estado**: CORREGIDO (2026-07-06)
- **Archivo**: `src/main.rs`
- **Solución**: Se eliminó la segunda llamada redundante (ya estaba corregido antes, se verificó que está limpio).

### [2026-07-06] validator.rs no detectaba definiciones duplicadas
- **Estado**: CORREGIDO (2026-07-06)
- **Archivo**: `src/validator.rs`
- **Antes**: Solo detectaba líneas duplicadas consecutivas y delimitadores no balanceados.
- **Ahora**: También detecta definiciones duplicadas de funciones, structs, enums, traits, constantes y módulos mediante `detect_duplicate_definitions()`. Esta es la defensa principal contra el patrón de error #1 del agente.

### [2026-07] `git clean -fd` borraba código fuente en proyectos sin remote
- **Estado**: CORREGIDO (2026-07-04)
- **Archivo**: `src/agent.rs`, handler `write_file_with_commit`

### [2026-07] Pánico UTF-8 en truncado de pasos de auditoría
- **Estado**: CORREGIDO (2026-07-04)

### [2026-07] `return Ok(...)` en handlers terminaba la sesión
- **Estado**: CORREGIDO (2026-07-04)

### [2026-07] `discover_projects` borraba proyectos locales
- **Estado**: CORREGIDO (2026-07-04)

---

## 🔧 APIs y Limitaciones Técnicas

### DeepSeek API
- **URL**: `https://api.deepseek.com/v1/chat/completions`
- **Modelo principal**: `deepseek-v4-pro` (con `thinking: {type: "enabled"}`)
- **Modelo de compresión**: `deepseek-v4-flash`
- **No soporta**: contenido multimodal `image_url` (solo texto)

### search_code
- La herramienta `search_code` usa búsqueda LOCAL por palabras clave (NO VoyageAI). La descripción en las tool definitions lo indica correctamente.

### GitHub CLI (gh)
- Requerido para `fork_and_clone_repo` y creación automática de repositorios en `write_file_with_commit`

---

## 📐 Decisiones de Arquitectura

### Compresión de contexto activo
- Se activa cuando `total_len > 500000` caracteres
- Usa DeepSeek Flash para resumir historial
- El system prompt nunca se comprime

### Sanitización de mensajes para API (`sanitize_messages_for_api`)
- Convierte mensajes `tool` huérfanos a `user` para evitar errores 400 de DeepSeek
- No modifica mensajes con `tool_calls` válidos

### Validación post-escritura (`validator.rs`)
- Detecta: líneas duplicadas consecutivas, definiciones duplicadas (fn/struct/enum/trait/const), delimitadores no balanceados
- Se integra en `write_file_with_commit`; resultados visibles para el agente

---

## 🚨 Patrones de Error del Agente

1. **Duplicación al editar por rango**: ⚠️ CORREGIDO — System prompt ahora prohíbe start_line/end_line en write_file_with_commit; validator.rs detecta definiciones duplicadas.
2. **Miedo al mensaje TRUNCADO**: ⚠️ CORREGIDO — Mensaje cambiado a "VISUALIZACIÓN PARCIAL" con aclaración explícita; regla #4 en system prompt.
3. **No verificar compilación entre ediciones**: ⚠️ CORREGIDO — System prompt ahora exige `cargo check` post-escritura.
4. **Código huérfano por rangos incorrectos**: ⚠️ MITIGADO — Al prohibir ediciones por rango, este problema desaparece.
5. **Timeouts en comandos pesados**: Usar parámetro `timer` en `execute_powershell` (mínimo 120s para compilación).
