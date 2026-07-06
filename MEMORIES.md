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
- **NOTA**: Actualmente `semantic_code_search()` usa búsqueda local por palabras clave, NO embeddings de Voyage. La función `get_voyage_embeddings()` existe pero no se usa.

### search_code
- **Importante**: La herramienta `search_code` usa búsqueda LOCAL por palabras clave (NO VoyageAI embeddings). La descripción en el system prompt dice "VoyageAI" pero es incorrecta.

### GitHub CLI (gh)
- Requerido para `fork_and_clone_repo` y para la creación automática de repositorios en `write_file_with_commit`
- Si no está instalado, `write_file_with_commit` falla en proyectos sin remote (pero no borra archivos)

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

## 🚨 Patrones de Error del Agente

1. **Bucles de compilación**: El agente puede hacer 40+ commits para lograr que un proyecto compile
2. **Explosión de pasos**: 2177 pasos de auditoría para 63 mensajes (ratio 34:1)
3. **Repetición de código**: El agente a veces reescribe código que ya existe
4. **Amnesia de contexto**: Después de compresión, puede perder detalles importantes
