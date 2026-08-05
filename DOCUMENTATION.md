# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v3.3

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos,
> **Google APIs (Drive, Gmail, Docs, Sheets) con herramientas nativas del agente**, **Task Scheduler**, **File Editor 3 modos**,
> cliente Electron (desktop) + Capacitor (Android) con ShellExecutor nativo, túnel Cloudflare.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~3196 | Servidor HTTP doble puerto, endpoints REST (incluye Google, Tasks, FileEditor), CAPTCHA, migración de chats |
| `src/agent.rs` | ~2776 | Bucle principal del agente, **32 herramientas** (incluye 6 de Google Drive), extract_text_from_docx(), soporte PDF/DOCX |
| `src/auth.rs` | ~947 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~600 | AppState (con google_auth, **google_drive**, task_scheduler), ActiveAgentStatus, CicleState, SubAgentManager |
| `src/study.rs` | ~973 | Motor de estudio: UserLearningProfile, UserKnowledgeBase, StudyEngine |
| `src/sync.rs` | ~280 | Sincronización de proyectos (push/pull/conflictos) |
| `src/client_protocol.rs` | ~180 | Protocolo cliente-servidor (ClientAction, ClientRequest, ClientResponse, ConnectedClient) |
| `src/validator.rs` | ~508 | Validación post-escritura (líneas duplicadas, delimitadores, errores Rust) |
| `src/scraper.rs` | ~170 | Búsqueda web DuckDuckGo Lite |
| `src/sub_agent.rs` | ~520 | Sub-agentes paralelos (máx 8, permisos por Patrón Composite) |
| `src/desktop.rs` | ~165 | Control de mouse/teclado (rdev) |
| `src/lib.rs` | ~22 | Librería pública + `pub const STUDY_SYSTEM_PROMPT` |
| `src/utils.rs` | ~72 | sanitize_filename() |
| **`src/file_editor.rs`** | ~260 | **Editor de archivos con 3 modos: Adicion, Reemplazo, Eliminacion** |
| **`src/google_auth.rs`** | ~340 | **Autenticación OAuth2 para Google APIs (token, refresh, estados)** |
| **`src/google_drive.rs`** | ~666 | **API Google Drive v3 (list, create, rename, download, upload, delete, read_content, update_content). Draw.io soportado.** |
| **`src/google_gmail.rs`** | ~340 | **API Gmail v1 (listar, leer, enviar, archivar, papelera)** |
| **`src/google_docs.rs`** | ~530 | **API Google Docs v1 + Sheets v4 (leer, crear, editar con 3 modos)** |
| **`src/task_scheduler.rs`** | ~460 | **Planificador de tareas cron-like (one-time, recurrentes, cron)** |
| `electron/main.js` | ~350 | Cliente Electron (connect→poll→execute→respond) |
| `capacitor/` | - | Cliente Android Capacitor |
| `public/index.html` | ~314 | Interfaz web SPA |
| `public/app.js` | ~1322 | Lógica frontend (chat, auth, admin, study) |
| `prompts/default_system_prompt.txt` | - | System prompt global por defecto |
| `prompts/study_system_prompt.txt` | - | System prompt para modo estudio |

---

## 🔑 Novedades v3.3 — Google Drive Tools para el Agente

### Herramientas del agente para Google Drive

El agente ahora tiene **6 herramientas nativas** para interactuar con Google Drive (antes solo existían como endpoints REST, ahora el agente puede usarlas directamente):

| Herramienta | Descripción | Requiere auth Google |
|-------------|-------------|---------------------|
| `google_drive_list` | Listar archivos/carpetas con query, parent_id, max_results | ✅ |
| `google_drive_download` | Descargar archivo → DOCX/XLSX/PPTX/.drawio | ✅ |
| `google_drive_read_content` | Leer contenido textual sin descargar (incluye XML de .drawio) | ✅ |
| `google_drive_metadata` | Obtener metadatos (nombre, MIME, si es drawio, etc.) | ✅ |
| `google_drive_create_folder` | Crear carpeta en Drive | ✅ |
| `google_drive_upload` | Subir archivo local a Drive | ✅ |

### Draw.io en Google Drive

Los archivos `.drawio` almacenados en Google Drive son totalmente soportados:
- `google_drive_list` los marca como `[DRAW.IO]`
- `google_drive_download` los descarga como archivo `.drawio` (XML editable)
- `google_drive_read_content` lee el XML directamente sin descargar
- `google_drive_metadata` reporta `is_drawio: true`

El agente puede usar `read_file` local sobre el `.drawio` descargado para ver/editar el XML.

### Cambios técnicos

1. **`src/google_drive.rs`**: `GoogleDriveClient` ahora deriva `Clone` para integrarse en `AppState`.
2. **`src/state.rs`**: `AppState` tiene nuevo campo `google_drive: GoogleDriveClient`.
3. **`src/main.rs`**: Inicializa `GoogleDriveClient` desde `GoogleAuthStore` al crear `AppState`.
4. **`src/agent.rs`**: 6 nuevas definiciones de herramientas + 6 nuevos handlers en el dispatch.

---

## 🔑 Módulos v3.2

### 1. File Editor (`src/file_editor.rs`) — 3 Modos de Edición

Componentes clave:
- `EditMode` (enum): `Adicion`, `Reemplazo`, `Eliminacion` — líneas ~20-30
- `EditResult` (struct): resultado con `success`, `message`, `lines_before`, `lines_after`, `preview` — líneas ~38-50
- `edit_file()`: aplica el modo de edición al archivo — líneas ~60-180
- `read_file_full()`: lee archivo completo — líneas ~185-190
- `read_file_range()`: lee rango de líneas — líneas ~195-215

Endpoints REST:
- `POST /api/file-editor/read` — leer archivo completo
- `POST /api/file-editor/read-range` — leer rango de líneas
- `POST /api/file-editor/edit` — editar con modo (adicion/reemplazo/eliminacion)

### 2. Google Auth (`src/google_auth.rs`) — OAuth2

Componentes clave:
- `GoogleCredentials` (struct): client_id, client_secret, redirect_uri — líneas ~20-30
- `GoogleToken` (struct): access_token, refresh_token, expires_at — líneas ~35-45
- `GoogleAuthStore` (struct): almacén de credenciales y tokens — líneas ~60-70
- `generate_auth_url()`: URL de autorización OAuth2 — líneas ~120-145
- `exchange_code()`: intercambia código por token — líneas ~150-185
- `get_valid_token()`: token válido (refresh automático) — líneas ~190-210
- `refresh_token()`: refresca token expirado — líneas ~220-255
- `ALL_SCOPES`: [DRIVE, GMAIL, DOCS, SHEETS] — línea ~340

Endpoints REST:
- `POST /api/google/auth-url` — generar URL OAuth2
- `GET /api/google/callback` — callback OAuth2
- `GET /api/google/token-status` — verificar token
- `POST /api/google/revoke` — revocar token

### 3. Google Drive (`src/google_drive.rs`) — API Drive v3

Componentes clave:
- `DriveFile` (struct): id, name, mime_type, is_folder, is_drawio — líneas ~15-30
- `DriveResult` (struct): success, message, files, content, download_path — líneas ~35-50
- `GoogleDriveClient` (struct): cliente con auth (Clone) — líneas ~55-60
- `list_files()`: listar archivos con query — líneas ~70-120
- `get_file_metadata()`: metadatos de archivo — líneas ~125-155
- `download_file()`: descargar (exporta Google Docs a DOCX/XLSX) — líneas ~160-230
- `upload_file()`: subir archivo (multipart) — líneas ~235-320
- `create_folder()`: crear carpeta — líneas ~325-360
- `rename_file()`: renombrar — líneas ~365-400
- `trash_file()` / `delete_permanently()`: eliminar — líneas ~405-460
- `read_file_content()`: leer contenido textual — líneas ~465-510
- `update_file_content()`: actualizar contenido — líneas ~515-555
- Draw.io: soportado como archivo normal de Drive (`.drawio` = XML editable)

Endpoints REST:
- `POST /api/google/drive/list` — listar archivos
- `GET /api/google/drive/file/:file_id` — metadatos
- `POST /api/google/drive/download` — descargar
- `POST /api/google/drive/upload` — subir
- `POST /api/google/drive/create-folder` — crear carpeta
- `POST /api/google/drive/rename` — renombrar
- `POST /api/google/drive/delete` — eliminar (papelera o permanente)
- `POST /api/google/drive/read-content` — leer contenido
- `POST /api/google/drive/update-content` — actualizar contenido

### 4. Google Gmail (`src/google_gmail.rs`) — API Gmail v1

Componentes clave:
- `GmailMessage` (struct): id, subject, from, to, body_text, labels — líneas ~12-25
- `GmailResult` (struct): success, messages, total_results — líneas ~28-35
- `GmailClient` (struct): cliente con auth — líneas ~38-42
- `list_emails()`: listar inbox con query — líneas ~48-95
- `send_email()`: enviar email (RFC 2822 + base64) — líneas ~130-170
- `archive_email()` / `trash_email()`: archivar/papelera — líneas ~175-220

Endpoints REST:
- `POST /api/google/gmail/list` — listar emails
- `POST /api/google/gmail/send` — enviar email
- `POST /api/google/gmail/archive` — archivar
- `POST /api/google/gmail/trash` — papelera

### 5. Google Docs & Sheets (`src/google_docs.rs`) — APIs Docs v1 + Sheets v4

Componentes clave:
- `GoogleDocContent` (struct): document_id, title, full_text, lines — líneas ~15-25
- `GoogleDocsClient` (struct): leer, crear, editar documentos — líneas ~30-40
- `read_document()`: leer Google Doc completo — líneas ~45-80
- `create_document()`: crear Google Doc — líneas ~85-105
- `edit_document()`: editar con 3 modos (Adicion/Reemplazo/Eliminacion) — líneas ~110-190
- `GoogleSheetData` (struct): spreadsheet_id, title, values — líneas ~200-215
- `read_sheet()`: leer hoja de cálculo — líneas ~220-260
- `write_sheet()`: escribir en hoja — líneas ~265-310

Endpoints REST:
- `POST /api/google/docs/read` — leer documento
- `POST /api/google/docs/create` — crear documento
- `POST /api/google/docs/edit` — editar documento
- `POST /api/google/sheets/read` — leer hoja
- `POST /api/google/sheets/write` — escribir hoja

### 6. Task Scheduler (`src/task_scheduler.rs`)

Componentes clave:
- `ScheduledTask` (struct): id, name, schedule (cron/one-time/recurring), action — líneas ~15-40
- `TaskSchedulerStore` (struct): almacén de tareas — líneas ~45-55
- `add_task()`: agregar tarea — líneas ~60-80
- `list_tasks()`: listar tareas — líneas ~85-100
- `remove_task()`: eliminar tarea — líneas ~105-120
- `execute_due_tasks()`: ejecutar tareas pendientes — líneas ~140-200

Endpoints REST:
- `POST /api/tasks/add` — agregar tarea
- `GET /api/tasks/list` — listar tareas
- `POST /api/tasks/remove` — eliminar tarea
- `POST /api/tasks/run-due` — ejecutar pendientes

---

## 🔧 Herramientas del Agente (32 herramientas en v3.3)

| # | Herramienta | Categoría |
|---|-------------|-----------|
| 1 | `search_google` | Búsqueda web |
| 2 | `read_file` | Archivos (soporta PDF/DOCX) |
| 3 | `write_file_with_commit` | Archivos + Git |
| 4 | `execute_powershell` | Sistema |
| 5 | `search_code` | Búsqueda en código |
| 6 | `fork_and_clone_repo` | GitHub |
| 7 | `read_url` | Web |
| 8 | `check_github_cli` | GitHub |
| 9 | `notificar_usuario` | Comunicación |
| 10 | `finalizar_tarea` | Control |
| 11 | `image_fetch` | Multimedia |
| 12 | `image_view` | Multimedia |
| 13 | `image_release` | Multimedia |
| 14 | `git_resolve_divergence` | Git |
| 15 | `analyze_images` | Multimedia (Qwen2.5-VL) |
| 16 | `kill_process` | Sistema |
| 17 | `fetch_tool_result` | Memoria |
| 18 | `release_tool_result` | Memoria |
| 19 | `spawn_sub_agent` | Sub-agentes |
| 20 | `check_sub_agent` | Sub-agentes |
| 21 | `kill_sub_agent` | Sub-agentes |
| 22 | `no_sync` | Sincronización |
| 23 | `reportar_fallo` | Sistema |
| **24** | **`google_drive_list`** | **🆕 Google Drive** |
| **25** | **`google_drive_download`** | **🆕 Google Drive** |
| **26** | **`google_drive_read_content`** | **🆕 Google Drive** |
| **27** | **`google_drive_metadata`** | **🆕 Google Drive** |
| **28** | **`google_drive_create_folder`** | **🆕 Google Drive** |
| **29** | **`google_drive_upload`** | **🆕 Google Drive** |
| 30-32 | (reservadas para Google Docs/Gmail tools) | Futuro |

---

## 🔄 Flujo de Google Drive con el Agente

```
1. Admin configura credenciales Google Cloud en panel de admin
2. Usuario vincula cuenta Google → /api/google/auth-url → OAuth2 callback
3. Agente usa google_drive_list → ve archivos/carpetas de Drive
4. Agente usa google_drive_read_content → lee .drawio XML sin descargar
5. Agente usa google_drive_download → descarga archivos al proyecto local
6. Agente usa read_file local → manipula PDF/DOCX/.drawio descargados
7. Agente usa google_drive_upload → sube resultados de vuelta a Drive
```
