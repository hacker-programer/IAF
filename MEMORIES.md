# MEMORIES.md — Memoria Persistente del Proyecto IAF

> Este archivo registra limitaciones técnicas, fallos de configuración y comportamientos de APIs
> descubiertos durante el desarrollo. Su objetivo es **minimizar llamadas innecesarias al modelo**,
> reducir cómputo redundante y evitar llamadas repetitivas de red.

---

## 🐛 Bugs Conocidos y Solucionados

### [2026-07] `git clean -fd` borraba código fuente en proyectos sin remote
- **Estado**: CORREGIDO (2026-07-04)
- **Archivo**: `src/agent.rs`, handler `write_file_with_commit`
- **Causa**: La autocuración ejecutaba `git clean -fd` y `git reset --hard origin/master` sin verificar que el remote existiera.
- **Solución**: Se agregó PASO 0 que verifica `git remote get-url origin`. Si no existe, intenta `gh repo create`. Si falla, aborta sin tocar archivos. Se eliminó `git clean -fd`.

### [2026-07] Pánico UTF-8 en truncado de pasos de auditoría
- **Estado**: CORREGIDO
- **Archivo**: `src/agent.rs`, función de generación de memoria de ejecución
- **Causa**: Indexación directa por bytes (`&s[..1500]`) en strings con caracteres multi-byte (ej: `═`)
- **Solución**: Se usa `.chars().take(1500).collect()` que es seguro para UTF-8.

### [2026-07] `return Ok(...)` en handlers de herramientas terminaba la sesión del agente
- **Estado**: CORREGIDO (2026-07-04)
- **Archivo**: `src/agent.rs`
- **Causa**: Múltiples handlers usaban `return Ok(...)` para reportar errores, pero en Rust esto retorna de `run_agent_loop()`, terminando la sesión completa.
- **Solución**: Se usan labeled blocks (`'write_handler: { ... break 'write_handler error; }`) y if/else para que los errores sean resultados de herramienta.

### [2026-07] `discover_projects` borraba proyectos locales (clear() destructivo)
- **Estado**: CORREGIDO (2026-07-04)
- **Archivo**: `src/agent.rs`, función `discover_projects`
- **Causa**: `projs.clear()` eliminaba todos los proyectos, incluyendo locales agregados manualmente.
- **Solución**: Se reemplazó `clear()` por `retain(|p| p.is_local)`, preservando proyectos locales.

### [2026-07] `check_github_cli` no usaba `working_dir`
- **Estado**: CORREGIDO (2026-07-04)
- **Archivo**: `src/agent.rs`, handler `check_github_cli`
- **Causa**: Se calculaba `working_dir` pero nunca se pasaba al `Command::new("gh")`.
- **Solución**: Se añadió `.current_dir(&working_dir)` al builder del Command.

### [2026-07] `discover_projects` llamada dos veces consecutivas en `main()`
- **Estado**: CORREGIDO (2026-07-04)
- **Archivo**: `src/main.rs`
- **Solución**: Se eliminó la segunda llamada redundante.

### [2026-07] Código huérfano/roto en `app.js`
- **Estado**: CORREGIDO (2026-07-04)
- **Archivo**: `public/app.js`
- **Causa**: Edición por rango de líneas dejó código fuera de funciones y doble handler.
- **Solución**: Reescritura completa del archivo.

### [2026-07] `DEFAULT_GLOBAL_SYSTEM_PROMPT` como `const &str` en `main.rs`
- **Estado**: CORREGIDO (2026-07-04)
- **Archivo**: `src/main.rs` → extraído a `prompts/default_system_prompt.txt`
- **Causa**: ~500 líneas de prompt embebidas que inflaban el binario.
- **Solución**: Se usa `include_str!("../prompts/default_system_prompt.txt")`.

---

## 🔧 APIs y Limitaciones Técnicas

### DeepSeek API
- **URL**: `https://api.deepseek.com/v1/chat/completions`
- **Modelo principal**: `deepseek-v4-pro` (con `thinking: {type: "enabled"}`)
- **Modelo de compresión**: `deepseek-v4-flash` (para resúmenes de contexto)
- **No soporta**: contenido multimodal `image_url` (solo texto)
- **Rate limits**: No documentados explícitamente; implementar backoff exponencial

### VoyageAI API
- **URL**: `https://api.voyageai.com/v1/embeddings`
- **Modelo**: `voyage-code-2`
- **Uso**: Embeddings para búsqueda de código semántica
- **NOTA**: El módulo `embeddings.rs` fue ELIMINADO. La búsqueda de código es puramente local (coincidencia de palabras clave).

### search_code
- **Importante**: La herramienta `search_code` usa búsqueda LOCAL por palabras clave (NO VoyageAI embeddings).

### GitHub CLI (gh)
- Requerido para `fork_and_clone_repo` y para la creación automática de repositorios en `write_file_with_commit`
- Si no está instalado, `write_file_with_commit` falla en proyectos sin remote (pero no borra archivos)

---

## 📐 Decisiones de Arquitectura

### Compresión de contexto activo
- Se activa cuando `total_len > 500000` caracteres
- Usa DeepSeek Flash para resumir historial
- El system prompt nunca se comprime

### Sanitización de mensajes para API
- Convierte mensajes `tool` huérfanos a `user` para evitar errores 400 de DeepSeek
- No modifica mensajes con `tool_calls` válidos

### Escritura en disco
- `save_chat_steps_to_disk()` escribe el archivo JSON completo en cada paso — ineficiente
- `debug_messages.json` se escribe en cada iteración — ineficiente
- Optimizar con escritura por lotes o rate-limiting

### Validación post-escritura (NUEVO — Julio 2026)
- **Archivo**: `src/validator.rs`
- **Propósito**: Detectar errores comunes después de que el agente modifica archivos.
- Detecta: líneas duplicadas consecutivas, delimitadores no balanceados (`{}`, `()`, `[]`), errores de sintaxis Rust (`rustfmt --check`) y JS (`node --check`).
- Se integra en `write_file_with_commit` en `agent.rs`.
- Los resultados se muestran como advertencias al modelo para que pueda autocorregirse.

### Optimización de rendimiento (Julio 2026)
- **`scraper.rs`**: Regex de limpieza HTML ahora usa `OnceLock<Regex>` (compilada una sola vez).
- **`desktop.rs`**: `type_text` reemplazó match de ~100 brazos por `HashMap<char, (Key, bool)>` precomputado.
- **`state.rs`**: `get_parent_pid` reemplazó `wmic` (obsoleto) por `Get-CimInstance`.

---

## 🚨 Patrones de Error del Agente

1. **Bucles de compilación**: El agente puede hacer 40+ commits para lograr que un proyecto compile
2. **Explosión de pasos**: 2177 pasos de auditoría para 63 mensajes (ratio 34:1)
3. **Repetición de código**: El agente a veces reescribe código que ya existe
4. **Amnesia de contexto**: Después de compresión, puede perder detalles importantes

### NUEVOS PATRONES DESCUBIERTOS (Julio 2026):

5. **Duplicación al editar por rango**: Cuando el agente usa `write_file_with_commit` con `start_line`/`end_line`, frecuentemente inserta código sin eliminar el original, creando duplicados.
   - **Mitigación**: `validator.rs` detecta líneas duplicadas consecutivas y advierte al modelo.

6. **Código huérfano por rangos incorrectos**: Al editar rangos que no respetan límites de funciones/clases, quedan fragmentos de código fuera de lugar.
   - **Mitigación**: `validator.rs` detecta delimitadores no balanceados.

7. **Timeouts en comandos pesados**: `cargo build`, `cargo check` y `Get-ChildItem -Recurse` frecuentemente exceden 30s.
   - **Mitigación**: Usar el parámetro `timer` en `execute_powershell` (mínimo 120s para compilación).

8. **No verificar compilación entre ediciones**: El agente tiende a hacer múltiples ediciones consecutivas sin verificar que compilan.
   - **Recomendación**: En el system prompt, instruir al agente a intercalar `cargo check` entre ediciones.
