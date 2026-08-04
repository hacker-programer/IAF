// ============================================================================
// IAF Electron Client — main.js
// ============================================================================
//
// Reemplaza al cliente Rust (client/src/main.rs).
// Se ejecuta en Windows/Linux/Mac como app desktop Electron.
//
// Responsabilidades:
//   1. Abre ventana BrowserWindow cargando la UI web del servidor IAF
//   2. Implementa el protocolo cliente-servidor (connect → poll → execute → respond)
//   3. Ejecuta comandos localmente: PowerShell, git, cargo, operaciones de archivos
//   4. Heartbeat periódico
//
// El servidor NUNCA ejecuta comandos para usuarios no-admin.
// Solo admin (puerto 80 o nonce verificado) ejecuta en el servidor.
//
// Para Android/Capacitor: este mismo cliente Electron en la PC del usuario
// es quien ejecuta los comandos solicitados desde la app Android.
// ============================================================================

const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const fs = require('fs');
const { execSync, exec, spawn } = require('child_process');
const crypto = require('crypto');

// ============================================================================
// Seguridad: validación de entrada para prevenir inyección de comandos
// ============================================================================

/// Solo permite caracteres seguros en subcomandos (estrictos)
const SAFE_SUBCMD_REGEX = /^[a-zA-Z0-9_\-.]+$/;

/// Permite caracteres adicionales en argumentos (commit messages, URLs, paths)
const SAFE_ARGS_REGEX = /^[a-zA-Z0-9_\-\s.\/\\:+=,@~#%!^*()<>'"\[\]{}?&]+$/;

function validateSafeArg(value, context) {
    if (typeof value !== 'string' || value.length === 0) return true;
    if (value.length > 2000) {
        console.error(`[IAF-Electron] BLOQUEO: ${context} excede 2000 caracteres: ${value.substring(0,80)}...`);
        return false;
    }
    // Usar regex más permisivo para argumentos, estricto para subcomandos
    var regex = context.includes('subcommand') ? SAFE_SUBCMD_REGEX : SAFE_ARGS_REGEX;
    if (!regex.test(value)) {
        console.error(`[IAF-Electron] BLOQUEO: ${context} contiene caracteres no permitidos: ${value.substring(0,100)}`);
        return false;
    }
    // Bloquear shell metacharacters críticos incluso si el regex los permite
    if (/[;|`$]/.test(value) && (context.includes('subcommand') || value.length < 500)) {
        console.error(`[IAF-Electron] BLOQUEO: ${context} contiene shell metacharacters: ${value.substring(0,100)}`);
        return false;
    }
    return true;
}
// ============================================================================

// Guardar credenciales en %APPDATA%/iaf-electron/config.json
const CONFIG_DIR = path.join(app.getPath('userData'));
const CONFIG_PATH = path.join(CONFIG_DIR, 'config.json');

let config = {
    serverUrl: 'http://127.0.0.1:8080',
    username: '',
    token: '',
    clientId: null,
};

function loadConfig() {
    try {
        if (fs.existsSync(CONFIG_PATH)) {
            const data = fs.readFileSync(CONFIG_PATH, 'utf-8');
            config = { ...config, ...JSON.parse(data) };
            console.log('[IAF-Electron] Configuración cargada:', config.serverUrl, config.username);
        }
    } catch (e) {
        console.error('[IAF-Electron] Error cargando config:', e.message);
    }
}

function saveConfig() {
    try {
        if (!fs.existsSync(CONFIG_DIR)) fs.mkdirSync(CONFIG_DIR, { recursive: true });
        fs.writeFileSync(CONFIG_PATH, JSON.stringify(config, null, 2));
        console.log('[IAF-Electron] Configuración guardada.');
    } catch (e) {
        console.error('[IAF-Electron] Error guardando config:', e.message);
    }
}

// ============================================================================
// Cliente HTTP (fetch wrapper)
// ============================================================================

const fetch = require('node-fetch');

async function apiCall(endpoint, method = 'GET', body = null) {
    const url = config.serverUrl + endpoint;
    const opts = {
        method,
        headers: { 'Content-Type': 'application/json' },
    };
    if (body) opts.body = JSON.stringify(body);
    const res = await fetch(url, opts);
    return res.json();
}

// ============================================================================
// Ejecutores de comandos (equivalente a las funciones Rust en client/src/main.rs)
// ============================================================================

function executeReadFile(params) {
    const filePath = params.path;
    const startLine = params.start_line || null;
    const endLine = params.end_line || null;

    if (!fs.existsSync(filePath)) {
        throw new Error(`Archivo no encontrado: ${filePath}`);
    }

    const content = fs.readFileSync(filePath, 'utf-8');
    const lines = content.split('\n');
    const totalLines = lines.length;

    let selected, range;
    if (startLine && endLine) {
        const s = Math.max(0, startLine - 1);
        const e = Math.min(totalLines, endLine);
        selected = lines.slice(s, e).join('\n');
        range = `${startLine}-${endLine}`;
    } else if (startLine) {
        const s = Math.max(0, startLine - 1);
        selected = lines.slice(s).join('\n');
        range = `${startLine}-${totalLines}`;
    } else {
        selected = content;
        range = `1-${totalLines}`;
    }

    return { content: selected, lines: range, total_lines: totalLines, path: filePath };
}

function executeWriteFile(params) {
    const filePath = params.path;
    const content = params.content;

    const dir = path.dirname(filePath);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

    fs.writeFileSync(filePath, content, 'utf-8');

    const hash = crypto.createHash('sha256').update(content).digest('hex');
    return { status: 'ok', path: filePath, sha256: hash, bytes_written: content.length };
}

function executePowerShell(params) {
    const command = params.command;
    const workDir = params.work_dir || process.cwd();

    // FIX #3: Usar Base64 encoding para evitar escapes frágiles.
    // PowerShell acepta -EncodedCommand con Base64 UTF-16LE.
    const utf16le = Buffer.from(command, 'utf16le');
    const base64Cmd = utf16le.toString('base64');

    try {
        const result = execSync(`powershell -NoProfile -EncodedCommand ${base64Cmd}`, {
            cwd: workDir,
            maxBuffer: 10 * 1024 * 1024, // 10 MB
            timeout: (params.timeout_secs || 120) * 1000,
            encoding: 'utf-8',
        });
        return { stdout: result, stderr: '', exit_code: 0 };
    } catch (e) {
        return {
            stdout: e.stdout || '',
            stderr: e.stderr || e.message,
            exit_code: e.status || 1,
        };
    }
}

function executeListDirectory(params) {
    const dirPath = params.path;
    const recursive = params.recursive || false;
    const pattern = params.pattern || null;

    if (!fs.existsSync(dirPath)) {
        throw new Error(`Directorio no encontrado: ${dirPath}`);
    }

    function walkDir(dir, depth = 0, maxDepth = 10) {
        const entries = [];
        const items = fs.readdirSync(dir, { withFileTypes: true });
        for (const item of items) {
            const fullPath = path.join(dir, item.name);
            if (pattern && !item.name.includes(pattern)) continue;
            const stat = fs.statSync(fullPath);
            entries.push({
                name: item.name,
                path: fullPath,
                is_dir: item.isDirectory(),
                size_bytes: stat.size,
                modified: Math.floor(stat.mtimeMs / 1000),
                depth: depth,
            });
            if (recursive && item.isDirectory() && depth < maxDepth) {
                entries.push(...walkDir(fullPath, depth + 1, maxDepth));
            }
        }
        return entries;
    }

    return { entries: walkDir(dirPath) };
}

function executeFileExists(params) {
    return { exists: fs.existsSync(params.path) };
}

function executeFileMetadata(params) {
    const stat = fs.statSync(params.path);
    return {
        size_bytes: stat.size,
        is_dir: stat.isDirectory(),
        is_file: stat.isFile(),
        readonly: (stat.mode & 0o222) === 0,
        modified: Math.floor(stat.mtimeMs / 1000),
        created: Math.floor(stat.birthtimeMs / 1000),
    };
}

function executeGit(params) {
    const subcommand = (params.subcommand || '').trim();
    const args = (params.args || '').trim();
    const workDir = params.work_dir || process.cwd();

    // FIX #2: Validar que subcommand y args solo contengan caracteres seguros
    if (subcommand && !validateSafeArg(subcommand, 'git subcommand')) {
        throw new Error(`Comando git bloqueado por seguridad: subcommand contiene caracteres no permitidos`);
    }
    if (args && !validateSafeArg(args, 'git args')) {
        throw new Error(`Comando git bloqueado por seguridad: args contienen caracteres no permitidos`);
    }

    try {
        const cmd = ['git', subcommand].filter(Boolean).join(' ');
        const fullCmd = args ? `${cmd} ${args}` : cmd;
        const result = execSync(fullCmd, {
            cwd: workDir,
            maxBuffer: 5 * 1024 * 1024,
            timeout: 60 * 1000,
            encoding: 'utf-8',
            windowsHide: true,
        });
        return { stdout: result, stderr: '', exit_code: 0 };
    } catch (e) {
        return {
            stdout: e.stdout || '',
            stderr: e.stderr || e.message,
            exit_code: e.status || 1,
        };
    }
}

function executeCargo(params) {
    const subcommand = (params.subcommand || '').trim();
    const args = (params.args || '').trim();
    const workDir = params.work_dir || process.cwd();

    // FIX #2: Validar que subcommand y args solo contengan caracteres seguros
    if (subcommand && !validateSafeArg(subcommand, 'cargo subcommand')) {
        throw new Error(`Comando cargo bloqueado por seguridad: subcommand contiene caracteres no permitidos`);
    }
    if (args && !validateSafeArg(args, 'cargo args')) {
        throw new Error(`Comando cargo bloqueado por seguridad: args contienen caracteres no permitidos`);
    }

    try {
        const cmd = ['cargo', subcommand].filter(Boolean).join(' ');
        const fullCmd = args ? `${cmd} ${args}` : cmd;
        const result = execSync(fullCmd, {
            cwd: workDir,
            maxBuffer: 10 * 1024 * 1024,
            timeout: 300 * 1000, // 5 minutos para builds
            encoding: 'utf-8',
            windowsHide: true,
        });
        return { stdout: result, stderr: '', exit_code: 0 };
    } catch (e) {
        return {
            stdout: e.stdout || '',
            stderr: e.stderr || e.message,
            exit_code: e.status || 1,
        };
    }
}

function executeSearchCode(params) {
    const query = params.query.toLowerCase();
    const searchPath = params.path || process.cwd();
    const filePattern = params.file_pattern || '*.rs,*.js,*.ts,*.html,*.css,*.json,*.md,*.txt,*.toml,*.ps1';
    const patterns = filePattern.split(',').map(p => p.trim());

    const results = [];

    function searchDir(dir, depth = 0) {
        if (depth > 10 || results.length >= 200) return;
        try {
            const items = fs.readdirSync(dir, { withFileTypes: true });
            for (const item of items) {
                if (results.length >= 200) return;
                const fullPath = path.join(dir, item.name);

                // Saltar node_modules, target, .git
                if (item.isDirectory()) {
                    const skip = ['node_modules', 'target', '.git', 'dist', '.config',
                                  'capacitor', '.cloudflared', '__pycache__'];
                    if (skip.includes(item.name)) continue;
                    searchDir(fullPath, depth + 1);
                } else if (item.isFile()) {
                    const matchesPattern = patterns.some(p =>
                        p === '*' || item.name.endsWith(p.replace('*', ''))
                    );
                    if (!matchesPattern) continue;

                    try {
                        const content = fs.readFileSync(fullPath, 'utf-8');
                        const lines = content.split('\n');
                        for (let i = 0; i < lines.length; i++) {
                            if (results.length >= 200) break;
                            if (lines[i].toLowerCase().includes(query)) {
                                results.push({
                                    file: fullPath,
                                    line: i + 1,
                                    content: lines[i].trim().substring(0, 200),
                                });
                            }
                        }
                    } catch (e) {
                        // Skip binary files
                    }
                }
            }
        } catch (e) {
            // Skip inaccessible directories
        }
    }

    searchDir(searchPath);
    return { query, matches: results.length, results };
}

// ============================================================================
// Despachador de acciones
// ============================================================================

function executeRequest(req) {
    const now = Math.floor(Date.now() / 1000);

    try {
        let result;
        switch (req.action) {
            case 'read_file':          result = executeReadFile(req.params); break;
            case 'write_file':         result = executeWriteFile(req.params); break;
            case 'execute_powershell': result = executePowerShell(req.params); break;
            case 'list_directory':     result = executeListDirectory(req.params); break;
            case 'file_exists':        result = executeFileExists(req.params); break;
            case 'file_metadata':      result = executeFileMetadata(req.params); break;
            case 'git_operation':      result = executeGit(req.params); break;
            case 'cargo_operation':    result = executeCargo(req.params); break;
            case 'search_code':        result = executeSearchCode(req.params); break;
            default:
                return {
                    request_id: req.request_id,
                    status: 'error',
                    result: {},
                    error: `Acción desconocida: ${req.action}`,
                    timestamp: now,
                };
        }
        return {
            request_id: req.request_id,
            status: 'ok',
            result: result,
            error: null,
            timestamp: now,
        };
    } catch (e) {
        return {
            request_id: req.request_id,
            status: 'error',
            result: {},
            error: e.message,
            timestamp: now,
        };
    }
}

// ============================================================================
// Loop del Cliente (connect → poll → execute → respond)
// ============================================================================

let pollInterval = null;
let heartbeatInterval = null;
let isConnected = false;

async function connectToServer() {
    if (!config.token || !config.username) {
        console.log('[IAF-Electron] Sin credenciales. Esperando login en la UI...');
        return;
    }

    try {
        const hostInfo = `${require('os').userInfo().username}@${require('os').hostname()}`;
        const resp = await apiCall('/api/client/connect', 'POST', {
            username: config.username,
            token: config.token,
            host_info: hostInfo,
        });

        if (resp.status === 'ok') {
            config.clientId = resp.client_id;
            isConnected = true;
            console.log(`[IAF-Electron] ✅ Conectado como cliente ${config.clientId}`);

            // Enviar estado de conexión a la UI
            if (mainWindow) {
                mainWindow.webContents.send('client-status', {
                    connected: true,
                    clientId: config.clientId,
                });
            }

            // Iniciar poll loop
            startPolling();
            startHeartbeat();
        } else {
            console.error('[IAF-Electron] ❌ Error de conexión:', resp.message);
            isConnected = false;
            // Reintentar en 10 segundos
            setTimeout(connectToServer, 10000);
        }
    } catch (e) {
        console.error('[IAF-Electron] Error conectando:', e.message);
        isConnected = false;
        setTimeout(connectToServer, 10000);
    }
}

function startPolling() {
    if (pollInterval) clearInterval(pollInterval);

    pollInterval = setInterval(async () => {
        if (!isConnected || !config.clientId) return;

        try {
            const resp = await apiCall('/api/client/poll', 'POST', {
                client_id: config.clientId,
                token: config.token,
            });

            if (resp.status !== 'ok') {
                console.warn('[IAF-Electron] ⚠️ Error en poll:', resp);
                return;
            }

            const requests = resp.pending_requests || [];
            for (const req of requests) {
                console.log(`[IAF-Electron] 📋 Ejecutando: ${req.action} (${req.request_id})`);
                const response = executeRequest(req);

                await apiCall('/api/client/response', 'POST', {
                    client_id: config.clientId,
                    token: config.token,
                    response: response,
                });
            }
        } catch (e) {
            console.error('[IAF-Electron] Error en poll:', e.message);
        }
    }, 2000); // Poll cada 2 segundos (igual que el cliente Rust)
}

function startHeartbeat() {
    if (heartbeatInterval) clearInterval(heartbeatInterval);

    heartbeatInterval = setInterval(async () => {
        if (!isConnected || !config.clientId) return;

        try {
            await apiCall('/api/client/heartbeat', 'POST', {
                client_id: config.clientId,
                token: config.token,
            });
        } catch (e) {
            console.error('[IAF-Electron] Error en heartbeat:', e.message);
        }
    }, 30000); // Cada 30 segundos
}

function disconnectFromServer() {
    isConnected = false;
    if (pollInterval) clearInterval(pollInterval);
    if (heartbeatInterval) clearInterval(heartbeatInterval);
    config.clientId = null;
    console.log('[IAF-Electron] Desconectado del servidor.');
}

// ============================================================================
// Ventana Principal
// ============================================================================

let mainWindow = null;

function createWindow() {
    mainWindow = new BrowserWindow({
        width: 1400,
        height: 900,
        minWidth: 800,
        minHeight: 600,
        title: 'IAF — Intelligent Agent Framework',
        icon: path.join(__dirname, 'icon.ico'),
        webPreferences: {
            preload: path.join(__dirname, 'preload.js'),
            nodeIntegration: false,
            contextIsolation: true,
        },
        autoHideMenuBar: true,
    });

    const serverUrl = config.serverUrl || 'http://127.0.0.1:8080';
    mainWindow.loadURL(serverUrl);

    mainWindow.on('closed', () => {
        mainWindow = null;
    });

    if (process.argv.includes('--dev')) {
        mainWindow.webContents.openDevTools();
    }
}

// ============================================================================
// IPC Handlers (comunicación entre main process y renderer/preload)
// ============================================================================

ipcMain.handle('execute-local', async (event, action, params) => {
    const req = {
        request_id: crypto.randomUUID(),
        action: action,
        params: params,
    };
    return executeRequest(req);
});

ipcMain.handle('set-credentials', async (event, credentials) => {
    config.serverUrl = credentials.serverUrl || config.serverUrl;
    config.username = credentials.username;
    config.token = credentials.token;
    saveConfig();
    console.log('[IAF-Electron] Credenciales actualizadas para:', config.username);

    disconnectFromServer();
    await connectToServer();
    return { status: 'ok' };
});

ipcMain.handle('get-client-status', async () => {
    return {
        connected: isConnected,
        clientId: config.clientId,
        serverUrl: config.serverUrl,
        username: config.username,
    };
});

ipcMain.handle('disconnect-client', async () => {
    disconnectFromServer();
    return { status: 'ok' };
});

// ============================================================================
// App Lifecycle
// ============================================================================

app.whenReady().then(() => {
    loadConfig();
    createWindow();

    // Conectar inmediatamente si hay token guardado (sin esperar 2s).
    // La UI hará polling con getStatus() hasta que conecte.
    if (config.token) {
        connectToServer();
    }

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) {
            createWindow();
        }
    });
});
        }
    });
});

app.on('window-all-closed', () => {
    disconnectFromServer();
    if (process.platform !== 'darwin') {
        app.quit();
    }
});

app.on('before-quit', () => {
    disconnectFromServer();
});

console.log('[IAF-Electron] Iniciado. Esperando conexión al servidor...');
