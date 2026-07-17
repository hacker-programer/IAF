# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v2.0

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos y cliente de ejecución remota.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~680 | Servidor HTTP doble puerto, endpoints REST, migración de chats |
| `src/auth.rs` | ~580 | Auth dual: contraseñas (argon2) + nonce Ed25519, UserStore, SessionStore |
| `src/state.rs` | ~400 | AppState con todos los stores (StudyEngine, SyncStore, clientes) |
| `src/study.rs` | ~570 | Motor de estudio: perfiles, knowledge base, hipótesis, engagement |
| `src/sync.rs` | ~280 | Sincronización de proyectos entre amigos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor para ejecución remota |
| `src/agent.rs` | 2088 | Bucle principal del agente, herramientas |
| `src/validator.rs` | 508 | Validación post-escritura |
| `src/scraper.rs` | 170 | Búsqueda web DuckDuckGo Lite |
| `src/sub_agent.rs` | 520 | Sub-agentes paralelos |
| `src/desktop.rs` | 165 | Control de mouse/teclado |
| `client/Cargo.toml` | 15 | Cliente binario independiente |
| `client/src/main.rs` | ~350 | Ejecutor local (files, PowerShell, git, cargo) |
| `prompts/study_system_prompt.txt` | 60 | System prompt del modo estudio |
| `tests/integration_tests.rs` | ~250 | Tests de integración y aceptación |

---

## 🔐 Autenticación Dual

| Método | Usuarios | Endpoint |
|--------|----------|----------|
| **Username + Password (argon2id)** | Usuarios normales | `POST /api/auth/login` |
| **Ed25519 Challenge-Response** | Solo admins | `POST /api/auth/challenge` → `POST /api/auth/verify` |

### Estructura UserAccount (`src/auth.rs` línea ~40)

```
username, public_key?, password_hash?, is_admin, permissions[],
limits: { max_tokens_per_day, max_api_calls_per_day, allowed_tools[],
          max_sub_agents, max_projects, can_fork_repos,
          can_execute_powershell, can_write_files },
has_study_access, has_programming_access, created_at, key_updated_at
```

---

## 🌐 Doble Puerto

| Puerto | Auth | Acceso |
|--------|------|--------|
| **80** | Ninguno (auto-admin) | Total. Ejecuta localmente. Chats en `.config/chats/` |
| **8080** | Requiere login | Según permisos. Chats en `.config/chats/<username>/` |

---

## 📚 Modo Estudio

### Fases

1. **Exploración** — Perfilado del usuario (edad, neurología, juegos, hobbies, YouTubers). Prueba métodos de enseñanza.
2. **Explotación** — Método optimizado encontrado. Mide rendimiento continuo.

### API de Estudio

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/study/profile` | Obtener perfil de aprendizaje |
| `POST` | `/api/study/profile` | Guardar perfil |
| `GET` | `/api/study/knowledge` | Obtener knowledge base |
| `POST` | `/api/study/projects` | Crear proyecto de estudio |
| `GET` | `/api/study/projects` | Listar proyectos del usuario |
| `POST` | `/api/study/projects/:id/members` | Agregar miembro |
| `POST` | `/api/study/build-prompt` | Construir system prompt personalizado |

---

## 🔄 Sincronización

| Método | Ruta | Descripción |
|--------|------|-------------|
| `POST` | `/api/sync/process` | Procesar manifiesto de sync |
| `POST` | `/api/sync/push` | Subir nueva versión de archivo |
| `GET` | `/api/sync/history/:project_id/*path` | Historial de versiones |

---

## 🖥️ Cliente de Ejecución Remota

| Método | Ruta | Descripción |
|--------|------|-------------|
| `POST` | `/api/client/connect` | Registrar cliente |
| `POST` | `/api/client/heartbeat` | Heartbeat cada 30s |
| `POST` | `/api/client/poll` | Polling de trabajo pendiente |
| `POST` | `/api/client/response` | Enviar resultado de ejecución |

El cliente se ejecuta: `iaf-client.exe <server_url> <username> <token>`

---

## 📝 Chats

- **Formato**: `<titulo_sanitizado>-<UUID>.json`
- **Admin/Port80**: `.config/chats/`
- **Usuarios**: `.config/chats/<username>/`
- **Migración automática** al iniciar el servidor

---

## 👑 Admin Endpoints

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/admin/users` | Listar usuarios |
| `POST` | `/api/admin/users` | Crear usuario |
| `PUT` | `/api/admin/users/:username/limits` | Actualizar límites |
| `PUT` | `/api/admin/users/:username/access` | study_access, programming_access |
| `PUT` | `/api/admin/users/:username/password` | Cambiar contraseña |
| `DELETE` | `/api/admin/users/:username` | Eliminar usuario |

---

## 🧪 Tests

```bash
# Unit tests
cargo test --lib

# Integration tests (requieren servidor)
cargo test --test integration_tests -- --ignored

# Todos los tests
cargo test
```
