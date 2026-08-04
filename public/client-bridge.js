// ============================================================================
// IAF Capacitor Client Bridge — client-bridge.js
// ============================================================================
//
// Puente entre el servidor IAF y la app Capacitor Android.
// Implementa el protocolo cliente-servidor (polling) para que Android
// ejecute comandos LOCALMENTE sin depender de una PC.
//
// Flujo:
//   1. connect() → registra el cliente Android en el servidor
//   2. poll()    → cada 2s pregunta si hay comandos pendientes
//   3. execute() → ejecuta localmente con ShellExecutor (plugin nativo)
//   4. respond() → envía el resultado al servidor
//   5. heartbeat() → cada 30s avisa que sigue vivo
//
// Uso: El servidor IAF delega herramientas (execute_powershell,
// write_file, read_file) a este cliente, que las ejecuta en Android.
// ============================================================================

(function() {
    'use strict';

    // Solo ejecutar en Capacitor (Android)
    if (!window.Capacitor || !window.Capacitor.isNativePlatform ||
        !window.Capacitor.isNativePlatform()) {
        console.log('[IAF Bridge] No es Capacitor nativo. Omitiendo.');
        return;
    }

    console.log('[IAF Bridge] Iniciando cliente Capacitor...');

    // ========================================================================
    // Configuración
    // ========================================================================
    const CONFIG = {
        pollIntervalMs: 2000,       // polling cada 2s
        heartbeatIntervalMs: 30000, // heartbeat cada 30s
        serverUrl: window.location.origin, // misma URL que la UI
        clientId: null,
        authToken: null,
        username: null,
        running: false,
    };

    // ========================================================================
    // ShellExecutor — Plugin Capacitor nativo
    // ========================================================================
    const ShellExecutor = {
        available: false,

        async init() {
            try {
                // Verificar si el plugin está disponible
                if (window.Capacitor && window.Capacitor.Plugins &&
                    window.Capacitor.Plugins.ShellExecutor) {
                    this.available = true;
                    const info = await window.Capacitor.Plugins.ShellExecutor.info();
                    console.log('[IAF Bridge] ShellExecutor disponible:', info);
                } else {
                    console.warn('[IAF Bridge] ShellExecutor no disponible. Solo operaciones de archivos.');
                }
            } catch(e) {
                console.warn('[IAF Bridge] Error init ShellExecutor:', e.message);
            }
        },

        async execute(command, workdir, timeout) {
            if (!this.available) {
                return {
                    exitCode: -1,
                    stdout: '',
                    stderr: 'ShellExecutor no disponible en este dispositivo.',
                    elapsedMs: 0,
                    timedOut: false,
                };
            }
            return await window.Capacitor.Plugins.ShellExecutor.execute({
                command: command,
                workdir: workdir || '/sdcard',
                timeout: timeout || 30,
            });
        },

        async which(command) {
            if (!this.available) return { found: false };
            return await window.Capacitor.Plugins.ShellExecutor.which({ command });
        },
    };

    // ========================================================================
    // Filesystem — Operaciones de archivos
    // ========================================================================
    const FileOps = {
        async readFile(path) {
            try {
                const result = await window.Capacitor.Plugins.Filesystem.readFile({
                    path: path,
                    encoding: 'utf8',
                });
                return { ok: true, content: result.data };
            } catch(e) {
                return { ok: false, error: e.message };
            }
        },

        async writeFile(path, content) {
            try {
                await window.Capacitor.Plugins.Filesystem.writeFile({
                    path: path,
                    data: content,
                    encoding: 'utf8',
                });
                return { ok: true };
            } catch(e) {
                return { ok: false, error: e.message };
            }
        },

        async fileExists(path) {
            try {
                const result = await window.Capacitor.Plugins.Filesystem.stat({ path });
                return { exists: true, metadata: result };
            } catch(e) {
                return { exists: false };
            }
        },
    };

    // ========================================================================
    // API HTTP (comunicación con el servidor)
    // ========================================================================
    async function apiCall(endpoint, method, body) {
        const url = CONFIG.serverUrl + endpoint;
        const headers = { 'Content-Type': 'application/json' };
        if (CONFIG.authToken) {
            headers['Authorization'] = 'Bearer ' + CONFIG.authToken;
        }

        const options = { method, headers };
        if (body) {
            options.body = JSON.stringify(body);
        }

        const res = await fetch(url, options);
        return await res.json();
    }

    // ========================================================================
    // Protocolo Cliente-Servidor
    // ========================================================================

    async function connect(username, token) {
        CONFIG.username = username;
        CONFIG.authToken = token;

        const res = await apiCall('/api/client/connect', 'POST', {
            username: username,
            token: token,
            host_info: 'Android/Capacitor ' + (navigator.userAgent.match(/Android [\d.]+/) || [''])[0],
        });

        if (res.status === 'ok') {
            CONFIG.clientId = res.client_id;
            console.log('[IAF Bridge] Conectado. Client ID:', CONFIG.clientId);
            return true;
        } else {
            console.error('[IAF Bridge] Error al conectar:', res);
            return false;
        }
    }

    async function poll() {
        if (!CONFIG.clientId) return [];

        try {
            const res = await apiCall('/api/client/poll', 'POST', {
                client_id: CONFIG.clientId,
                token: CONFIG.authToken,
            });

            if (res.pending_requests && res.pending_requests.length > 0) {
                console.log('[IAF Bridge]', res.pending_requests.length, 'comandos pendientes');
                return res.pending_requests;
            }
        } catch(e) {
            console.error('[IAF Bridge] Error en poll:', e.message);
        }

        return [];
    }

    async function heartbeat() {
        if (!CONFIG.clientId) return;
        try {
            await apiCall('/api/client/heartbeat', 'POST', {
                client_id: CONFIG.clientId,
                token: CONFIG.authToken,
            });
        } catch(e) {
            console.error('[IAF Bridge] Error heartbeat:', e.message);
        }
    }

    async function respond(requestId, status, result, error) {
        if (!CONFIG.clientId) return;
        try {
            await apiCall('/api/client/response', 'POST', {
                client_id: CONFIG.clientId,
                token: CONFIG.authToken,
                response: {
                    request_id: requestId,
                    status: status,
                    result: result || {},
                    error: error || null,
                    timestamp: Date.now(),
                },
            });
        } catch(e) {
            console.error('[IAF Bridge] Error respond:', e.message);
        }
    }

    // ========================================================================
    // Ejecución de comandos delegados
    // ========================================================================

    async function executeRequest(req) {
        const { request_id, action, params } = req;
        console.log('[IAF Bridge] Ejecutando:', action, '| ID:', request_id);

        try {
            switch (action) {
                case 'execute_powershell': {
                    const command = params.command || '';
                    const workdir = params.work_dir || '/sdcard';
                    const timeout = params.timeout_secs || 30;

                    // Adaptar comando PowerShell a shell de Android
                    const adaptedCommand = adaptCommandToAndroidShell(command);

                    const result = await ShellExecutor.execute(adaptedCommand, workdir, timeout);
                    await respond(request_id, result.exitCode === 0 ? 'ok' : 'error',
                        {
                            stdout: result.stdout,
                            stderr: result.stderr,
                            exit_code: result.exitCode,
                            pid: 0,  // Android no expone PID
                        },
                        result.timedOut ? 'Timeout: el comando excedió ' + timeout + 's' : null
                    );
                    break;
                }

                case 'read_file': {
                    const path = params.path || '';
                    const fileResult = await FileOps.readFile(path);
                    if (fileResult.ok) {
                        let content = fileResult.content;
                        if (params.start_line || params.end_line) {
                            const lines = content.split('\n');
                            const start = (params.start_line || 1) - 1;
                            const end = params.end_line || lines.length;
                            content = lines.slice(start, end).join('\n');
                        }
                        await respond(request_id, 'ok', { content, lines: content.split('\n').length });
                    } else {
                        await respond(request_id, 'error', {}, fileResult.error);
                    }
                    break;
                }

                case 'write_file': {
                    const path = params.path || '';
                    const content = params.content || '';
                    const fileResult = await FileOps.writeFile(path, content);
                    if (fileResult.ok) {
                        await respond(request_id, 'ok', { written: true, path });
                    } else {
                        await respond(request_id, 'error', {}, fileResult.error);
                    }
                    break;
                }

                case 'file_exists': {
                    const path = params.path || '';
                    const existsResult = await FileOps.fileExists(path);
                    await respond(request_id, 'ok', existsResult);
                    break;
                }

                case 'list_directory': {
                    // Usar shell para listar directorio
                    const path = params.path || '/sdcard';
                    const result = await ShellExecutor.execute('ls -la "' + path + '"', '/', 10);
                    await respond(request_id, 'ok', {
                        raw: result.stdout,
                        error: result.stderr,
                    });
                    break;
                }

                case 'search_code': {
                    const query = params.query || '';
                    const filePattern = params.file_pattern || '*.rs';
                    const path = params.path || '/sdcard';
                    const cmd = 'grep -r --include="' + filePattern + '" "' + query + '" "' + path + '" 2>/dev/null || echo "No matches"';
                    const result = await ShellExecutor.execute(cmd, '/', 15);
                    await respond(request_id, 'ok', { matches: result.stdout });
                    break;
                }

                default:
                    await respond(request_id, 'error', {},
                        'Acción no soportada: ' + action);
            }
        } catch(e) {
            console.error('[IAF Bridge] Error ejecutando', action, ':', e.message);
            await respond(request_id, 'error', {},
                'Error interno: ' + e.message);
        }
    }

    /**
     * Adapta un comando PowerShell a shell POSIX de Android.
     * Traduce cmdlets comunes a equivalentes Unix.
     */
    function adaptCommandToAndroidShell(command) {
        let adapted = command;

        // Reemplazos comunes PowerShell → Unix
        const replacements = [
            ['Get-Content', 'cat'],
            ['Set-Content', 'cat > '],  // aproximado
            ['Get-ChildItem', 'ls'],
            ['Select-Object', 'head'],  // aproximado
            ['Select-String', 'grep'],
            ['New-Item -ItemType Directory -Force -Path', 'mkdir -p'],
            ['Remove-Item', 'rm -rf'],
            ['Copy-Item', 'cp -r'],
            ['Move-Item', 'mv'],
            ['Write-Host', 'echo'],
            ['Write-Output', 'echo'],
            ['Rename-Item', 'mv'],
            ['Test-Path', 'test -e'],
            ['Out-Null', '> /dev/null'],
            ['$LASTEXITCODE', '$?'],
            ['tar.exe', 'tar'],
            ['ForEach-Object', 'while read line; do'],  // muy aproximado
        ];

        for (const [psCmd, unixCmd] of replacements) {
            adapted = adapted.replace(new RegExp(psCmd.replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&'), 'gi'), unixCmd);
        }

        // Si el comando empieza con "powershell " o "pwsh ", eliminarlo
        adapted = adapted.replace(/^powershell\s+/i, '');
        adapted = adapted.replace(/^pwsh\s+/i, '');

        // Si empieza con .\script.ps1, convertirlo a sh script.sh
        adapted = adapted.replace(/^\.\\(.+)\.ps1\b/, 'sh ./$1.sh');

        // Reemplazar rutas de Windows a Unix
        adapted = adapted.replace(/C:\\/gi, '/sdcard/');
        adapted = adapted.replace(/\\/g, '/');

        console.log('[IAF Bridge] Comando adaptado:', adapted.substring(0, 100));
        return adapted;
    }

    // ========================================================================
    // Bucle principal
    // ========================================================================

    async function mainLoop() {
        CONFIG.running = true;
        console.log('[IAF Bridge] Bucle principal iniciado.');

        let lastHeartbeat = Date.now();

        while (CONFIG.running) {
            try {
                // Polling de comandos pendientes
                const requests = await poll();

                if (requests.length > 0) {
                    // Ejecutar todos los comandos pendientes en paralelo
                    await Promise.all(requests.map(req => executeRequest(req)));
                }

                // Heartbeat periódico
                if (Date.now() - lastHeartbeat > CONFIG.heartbeatIntervalMs) {
                    await heartbeat();
                    lastHeartbeat = Date.now();
                }
            } catch(e) {
                console.error('[IAF Bridge] Error en main loop:', e.message);
            }

            // Esperar antes del próximo poll
            await new Promise(resolve => setTimeout(resolve, CONFIG.pollIntervalMs));
        }
    }

    // ========================================================================
    // Inicialización
    // ========================================================================

    async function start(authToken, username) {
        console.log('[IAF Bridge] Iniciando con usuario:', username);

        // Inicializar ShellExecutor
        await ShellExecutor.init();

        // Conectar al servidor
        const connected = await connect(username, authToken);
        if (!connected) {
            console.error('[IAF Bridge] No se pudo conectar al servidor.');
            return false;
        }

        // Iniciar bucle principal en segundo plano
        mainLoop().catch(e => console.error('[IAF Bridge] Error fatal:', e));

        return true;
    }

    function stop() {
        CONFIG.running = false;
        console.log('[IAF Bridge] Detenido.');
    }

    // Exponer API global
    window.IAFCapacitorBridge = {
        start,
        stop,
        ShellExecutor,
        FileOps,
        isRunning: () => CONFIG.running,
        getClientId: () => CONFIG.clientId,
    };

    console.log('[IAF Bridge] API expuesta en window.IAFCapacitorBridge');
    console.log('[IAF Bridge] Esperando autenticación para iniciar...');
})();
