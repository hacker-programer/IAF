# DOCUMENTATION.md — Mapa Técnico del Proyecto IAF v3.2

> **IAF (Intelligent Agent Framework)** — Framework de agente autónomo + plataforma de enseñanza en Rust + Axum.
> Servidor HTTP doble puerto (80 auto-admin, 8080 auth), autenticación dual (password + Ed25519),
> motor de estudio con perfilado de aprendizaje, sincronización de proyectos,
> **Google APIs (Drive, Gmail, Docs, Sheets)**, **Task Scheduler**, **File Editor 3 modos**,
> cliente Electron (desktop) + Capacitor (Android) con ShellExecutor nativo, túnel Cloudflare.

---

## 📁 Estructura de Archivos

| Archivo | Líneas | Rol |
|---------|--------|-----|
| `src/main.rs` | ~3150 | Servidor HTTP doble puerto, endpoints REST (incluye Google, Tasks, FileEditor), CAPTCHA, migración de chats |
| `src/agent.rs` | ~2523 | Bucle principal del agente, 26 herramientas, extract_text_from_docx(), soporte PDF/DOCX |
| `src/auth.rs` | ~947 | Auth dual: contraseñas (argon2) + nonce Ed25519, permisos, WeeklySchedule, UserLimits |
| `src/state.rs` | ~580 | AppState (con google_auth, task_scheduler), ActiveAgentStatus, CicleState, SubAgentManager |
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
| **`src/google_drive.rs`** | ~670 | **API Google Drive v3 (list, create, rename, download, upload, delete)** |
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

## 🔑 Nuevos Módulos v3.2

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
- `GET/POST /api/google/credentials` — configurar/ver credenciales

### 3. Google Drive (`src/google_drive.rs`) — API Drive v3

Componentes clave:
- `DriveFile` (struct): id, name, mime_type, is_folder, is_drawio — líneas ~15-30
- `DriveResult` (struct): success, message, files, content, download_path — líneas ~35-50
- `GoogleDriveClient` (struct): cliente con auth — líneas ~55-60
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
- `GoogleSheetsClient` (struct): leer, escribir, crear, editar hojas — líneas ~220-230
- `read_sheet()`: leer hoja de cálculo — líneas ~235-280
- `write_sheet_range()`: escribir en rango — líneas ~285-320
- `create_spreadsheet()`: crear hoja nueva — líneas ~325-350
- `edit_sheet_lines()`: editar con 3 modos sobre filas — líneas ~355-430

Endpoints REST:
- `POST /api/google/docs/read` — leer documento
- `POST /api/google/docs/create` — crear documento
- `POST /api/google/docs/edit` — editar (mode: adicion/reemplazo/eliminacion)
- `POST /api/google/sheets/read` — leer hoja
- `POST /api/google/sheets/write` — escribir rango
- `POST /api/google/sheets/create` — crear hoja
- `POST /api/google/sheets/edit` — editar con 3 modos

### 6. Task Scheduler (`src/task_scheduler.rs`) — Planificador de Tareas

Componentes clave:
- `TaskFrequency` (enum): Once, EveryMinutes, EveryHours, EveryDays, Cron — líneas ~18-40
- `TaskAction` (enum): PowerShell, OrganizeDrive, SendEmail, AgentPrompt — líneas ~43-65
- `DriveOrganizeRule` (struct): reglas para organizar Drive — líneas ~68-80
- `TaskStatus` (enum): Active, Paused, Completed, Failed — líneas ~83-88
- `ScheduledTask` (struct): tarea completa con metadatos — líneas ~91-105
- `TaskExecutionLog` (struct): registro de ejecución — líneas ~108-115
- `TaskSchedulerStore` (struct): almacén persistente — líneas ~120-130
- `create_task()`, `update_task()`, `delete_task()`, `list_tasks()` — líneas ~160-220
- `get_due_tasks()`: tareas pendientes de ejecución — líneas ~240-260
- `start_scheduler_loop()`: loop de ejecución cada 30s — líneas ~340-390
- `execute_task()`: ejecuta la acción según el tipo — líneas ~395-460
- Soporte cron: minute, hour, day_of_month, month, day_of_week — líneas ~290-335

Endpoints REST:
- `POST /api/tasks/list` — listar tareas del usuario
- `POST /api/tasks/get` — obtener tarea por ID
- `POST /api/tasks/create` — crear tarea (todos los tipos de acción y frecuencia)
- `POST /api/tasks/update` — actualizar tarea
- `POST /api/tasks/delete` — eliminar tarea
- `POST /api/tasks/logs` — ver historial de ejecuciones

---

## 🔄 Flujo de Google OAuth2

1. Usuario configura client_id y client_secret desde Google Cloud Console
2. `POST /api/google/auth-url` → devuelve URL de autorización
3. Usuario visita la URL y autoriza
4. Google redirige a `/api/google/callback?code=...&state=...`
5. El servidor intercambia el código por access_token + refresh_token
6. Tokens se guardan en `.config/google/google_tokens.json`
7. Refresh automático cuando el token expira

---

## 📝 Modos de Edición (File Editor + Google Docs/Sheets/Drive)

Los 3 modos de edición están disponibles tanto para archivos locales como para Google Drive/Docs/Sheets:

| Modo | Parámetros | Descripción |
|------|-----------|-------------|
| **Adicion** | start_line, content | Inserta líneas después de start_line (0=principio, N=final si N>total) |
| **Reemplazo** | start_line, end_line, content | Reemplaza líneas en el rango [start, end] con el nuevo contenido |
| **Eliminacion** | start_line, end_line | Elimina líneas en el rango [start, end] |

---

## 🧪 Tests

Archivos de tests actualizados:
- `tests/exhaustive_tests.rs` (1835 líneas) — 15 módulos, 123 tests
- `tests/integration_tests.rs` (1197 líneas) — 10 módulos
- `tests/frontend_regression_tests.js` — Tests de regresión del frontend
- `src/file_editor.rs` — Tests unitarios incluidos (8 tests: adicion, reemplazo, eliminacion)

---

## 🔧 Dependencias

- `reqwest` — HTTP client para Google APIs
- `serde` / `serde_json` — Serialización
- `uuid` — IDs para tareas y estados OAuth
- `chrono` — Cálculo de tiempos para cron
- `base64` — Decodificación de emails
- `mime_guess` — Detección de MIME types
- `urlencoding` — URL encoding para OAuth y queries
