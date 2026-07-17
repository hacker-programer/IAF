# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo en Rust + Axum + DeepSeek API.
> Servidor HTTP que orquesta un agente de desarrollo de software con herramientas,
> sub-agentes paralelos, autenticación criptográfica y almacenamiento de resultados con IDs.

---

## 📁 Estructura de Archivos Fuente

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | 1265 | Servidor HTTP (Axum), endpoints REST, inicialización, auth endpoints |
| `src/agent.rs` | 2088 | Bucle principal del agente, herramientas, loop de ejecución |
| `src/state.rs` | 647 | Estructuras de datos compartidas (AppState, ToolResultStore, SubAgentManager) |
| `src/auth.rs` | 565 | Autenticación Ed25519 challenge-response, gestión de usuarios, sesiones |
| `src/validator.rs` | 508 | Validación post-escritura (duplicados, delimitadores, contexto impl) |
| `src/scraper.rs` | 170 | Búsqueda web vía DuckDuckGo Lite + fallback Google |
| `src/sub_agent.rs` | 520 | Ejecución paralela de sub-agentes con restricciones de path |
| `src/desktop.rs` | 165 | Control de mouse/teclado (rdev), lanzamiento de ejecutables |
| `prompts/default_system_prompt.txt` | 517 | System prompt global del agente (reglas, técnicas de optimización) |
| `.config/users.json` | (gitignored) | Usuarios, claves públicas, límites, permisos |

---

## 🔐 Sistema de Autenticación (`src/auth.rs`)

### Flujo Challenge-Response (Ed25519)

```
Cliente                          Servidor
  │                                 │
  ├─ POST /api/auth/challenge ────>│  (1) Solicita nonce
  │   { "username": "Fa" }          │
  │                                 │
  │<── { "nonce": "base64..." } ───┤  (2) Servidor genera 32 bytes aleatorios
  │                                 │     Almacena en ChallengeStore (TTL 5 min)
  │                                 │
  │  (El cliente firma el nonce     │
  │   con su clave privada Ed25519) │
  │                                 │
  ├─ POST /api/auth/verify ───────>│  (3) Envía firma
  │   { "username","nonce",         │
  │     "signature":"base64..." }   │     Verifica con clave pública almacenada
  │                                 │     Consume el nonce (anti-replay)
  │<── { "token":"iaf_...", ... } ──┤  (4) Sesión creada (TTL 24h)
```

### Estructuras Clave

| Estructura | Línea | Descripción |
|-----------|-------|-------------|
| `UserAccount` | ~40 | `username, public_key (hex 64), is_admin, permissions[], limits, created_at` |
| `UserLimits` | ~63 | `max_tokens_per_day, max_api_calls_per_day, allowed_tools[], max_sub_agents, max_projects, can_fork_repos, can_execute_powershell, can_write_files` |
| `UserStore` | ~129 | Carga/guarda users.json. CRUD de usuarios con validación de clave pública |
| `ChallengeStore` | ~306 | Nonces efímeros (TTL 5 min). generate_challenge() y verify_challenge() con anti-replay |
| `SessionStore` | ~443 | Tokens de sesión (TTL 24h). create_session(), validate_token(), revoke_token() |
| `generate_keypair()` | ~537 | Genera par Ed25519 → (private_hex, public_hex) |
| `sign_message()` | ~550 | Firma bytes con clave privada Ed25519 |

### Endpoints de Auth

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| `GET` | `/api/auth/keygen` | No | Genera un par de claves Ed25519 (setup inicial) |
| `POST` | `/api/auth/challenge` | No | Solicita un nonce (challenge) para firmar |
| `POST` | `/api/auth/verify` | No | Verifica la firma del challenge → retorna token |
| `POST` | `/api/auth/logout` | No | Invalida un token de sesión |

### Endpoints Admin (requieren token en header `Authorization: Bearer <token>`)

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/admin/users` | Listar todos los usuarios |
| `POST` | `/api/admin/users` | Crear nuevo usuario |
| `PUT` | `/api/admin/users/:username/limits` | Actualizar límites de un usuario |
| `PUT` | `/api/admin/users/:username/permissions` | Actualizar permisos |
| `PUT` | `/api/admin/users/:username/key` | Cambiar clave pública |
| `DELETE` | `/api/admin/users/:username` | Eliminar usuario |

### Configuración Inicial

1. Llamar a `GET /api/auth/keygen` para generar un par de claves
2. Copiar `public_key` en `.config/users.json` (usar `.config/users.json.template` como base)
3. Guardar `private_key` en un lugar seguro (variable de entorno o archivo protegido)
4. Iniciar el servidor

---

## 🧩 Estructuras de Datos Principales (`src/state.rs`)

| Estructura | Línea aprox. | Descripción |
|-----------|-------------|-------------|
| `Project` | ~13 | `name: String, path: String, is_local: bool` — Proyecto registrado |
| `PromptConfig` | ~19 | `global_default, global_current: String, projects: HashMap<String, String>` |
| `ChatMessage` | ~33 | `role: String, content: String, timestamp: u64` |