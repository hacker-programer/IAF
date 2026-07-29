// ============================================================================
// IAF — app.js — Cliente Web con Autenticación
// ============================================================================

// ---- State ----
let activeProject = null;
let currentSessionId = null;
let agentMonitorInterval = null;
let currentCaptcha = null;
let pendingMessageToSend = null;
let agentQuestionShown = false;  // evita abrir el banner repetidamente
let agentPlanShown = false;      // evita abrir el modal repetidamente

// Auth state
let authToken = null;
let authUsername = null;
let authIsAdmin = false;
let authHasStudy = false;
let authHasProgramming = false;
let isPort80 = window.location.port === '80';

// Platform detection (Electron / Capacitor / Browser)
let platformType = 'browser';   // 'electron', 'capacitor', 'browser'
let platformLabel = '';         // badge text

// ---- DOM refs ----
const loginScreen = document.getElementById('loginScreen');
const appContainer = document.getElementById('appContainer');
const loginTabs = document.getElementById('loginTabs');
const loginPassword = document.getElementById('loginPassword');
const loginNonce = document.getElementById('loginNonce');
const loginError = document.getElementById('loginError');
const clientWarning = document.getElementById('clientWarning');
const userBadge = document.getElementById('userBadge');
const adminPanel = document.getElementById('adminPanel');
const studyProfileSection = document.getElementById('studyProfileSection');

// ---- Platform Detection ----

function detectPlatform() {
    // Detectar Electron: el preload.js expone window.iafClient
    if (window.iafClient && window.iafClient.isElectron) {
        platformType = 'electron';
        platformLabel = '🖥️ Electron';
        console.log('[IAF] Ejecutándose en Electron (cliente desktop)');
        return;
    }

    // Detectar Capacitor (Android WebView): expone window.Capacitor
    if (window.Capacitor && window.Capacitor.isNativePlatform && window.Capacitor.isNativePlatform()) {
        platformType = 'capacitor';
        const platform = window.Capacitor.getPlatform();
        platformLabel = platform === 'android' ? '📱 Android' : '📱 Mobile';
        console.log('[IAF] Ejecutándose en Capacitor (' + platform + ')');
        return;
    }

    // Detectar si es un dispositivo móvil táctil (Android Chrome, etc.)
    const isMobile = /Android|iPhone|iPad|iPod|webOS/i.test(navigator.userAgent)
        || ('ontouchend' in document);
    if (isMobile) {
        platformType = 'mobile-browser';
        platformLabel = '📱 Navegador Móvil';
        console.log('[IAF] Ejecutándose en navegador móvil');
        return;
    }

    platformType = 'browser';
    platformLabel = '🌐 Web';
    console.log('[IAF] Ejecutándose en navegador desktop');
}

// ---- Init ----

// ---- Helpers: Toggle Password Visibility ----
function togglePassword(fieldId) {
    const el = document.getElementById(fieldId);
    if (!el) return;
    el.type = el.type === 'password' ? 'text' : 'password';
}

/**
 * Copia el comando sign_nonce al portapapeles.
 */
function copyNonceCmd(event) {
    event = event || window.event;
    const nonce = window._lastNonce || '';
    const cmd = '.\\scripts\\sign_nonce.ps1 -Nonce "' + nonce + '" -KeyPath ".config\\admin_private.pem"';

    var btn = null;
    if (event && event.target) {
        btn = event.target;
    } else {
        btn = document.querySelector('.btn-copy-small');
    }

    function fallbackCopy(text) {
        var ta = document.createElement('textarea');
        ta.value = text;
        ta.style.position = 'fixed';
        ta.style.left = '-9999px';
        ta.style.top = '-9999px';
        document.body.appendChild(ta);
        ta.focus();
        ta.select();
        var ok = false;
        try { ok = document.execCommand('copy'); } catch (e) { ok = false; }
        document.body.removeChild(ta);
        return ok;
    }

    function onSuccess() {
        if (btn) {
            btn.textContent = '✓';
            setTimeout(function () { btn.textContent = '📋'; }, 1500);
        }
    }

    function onFailure() {
        alert('No se pudo copiar al portapapeles. Copiá manualmente:\n\n' + cmd);
    }

    if (navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
        navigator.clipboard.writeText(cmd).then(onSuccess).catch(function () {
            if (fallbackCopy(cmd)) { onSuccess(); } else { onFailure(); }
        });
    } else {
        if (fallbackCopy(cmd)) { onSuccess(); } else { onFailure(); }
    }
}

async function init() {
    detectPlatform();

    if (isPort80) {
        // Puerto 80: acceso directo como admin local
        authToken = 'admin_local';
        authUsername = 'admin_local';
        authIsAdmin = true;
        authHasStudy = true;
        authHasProgramming = true;
        showApp();
    } else {
        // Puerto 8080: login obligatorio
        loginScreen.classList.remove('hidden');
        appContainer.classList.add('hidden');
        // En Electron, intentar reconectar automáticamente
        if (platformType === 'electron' && window.iafClient) {
            try {
                const status = await window.iafClient.getStatus();
                if (status.connected) {
                    console.log('[IAF] Electron ya conectado al servidor como', status.username);
                }
            } catch(e) {}
        }
        await checkClient();
    }

    // Inicializar hamburger menu para mobile
    initMobileNav();
}

async function checkClient() {
    // En Electron, no mostrar advertencia de "cliente no detectado"
    if (platformType === 'electron') {
        clientWarning.innerHTML = '🖥️ <b>Cliente Electron activo</b> — comandos locales disponibles.';
        clientWarning.style.borderColor = 'var(--success)';
        clientWarning.style.color = 'var(--success)';
        clientWarning.style.background = 'rgba(16,185,129,0.1)';
        clientWarning.classList.remove('hidden');
        return;
    }

    // En Capacitor/Android, mostrar estado
    if (platformType === 'capacitor') {
        clientWarning.innerHTML = '📱 <b>App Android</b> — los comandos se ejecutan vía cliente Electron en tu PC.';
        clientWarning.style.borderColor = 'var(--accent)';
        clientWarning.style.color = 'var(--accent)';
        clientWarning.style.background = 'rgba(6,182,212,0.1)';
        clientWarning.classList.remove('hidden');
        return;
    }

    // En navegador normal, verificar si el cliente está instalado
    try {
        const res = await fetch('/api/client/check');
        const text = await res.text();
        let data;
        try { data = JSON.parse(text); } catch(e) { return; }
        if (!data.client_installed) {
            clientWarning.innerHTML = '⚠️ <b>Cliente no detectado.</b><br>' + data.instructions;
            clientWarning.classList.remove('hidden');
        }
    } catch(e) {
        // Si el endpoint falla, no bloquear
    }
}

// ---- Login Tabs ----
loginTabs.addEventListener('click', (e) => {
    if (e.target.classList.contains('login-tab')) {
        loginTabs.querySelectorAll('.login-tab').forEach(t => t.classList.remove('active'));
        e.target.classList.add('active');
        const tab = e.target.dataset.tab;
        loginPassword.classList.toggle('hidden', tab !== 'password');
        loginNonce.classList.toggle('hidden', tab !== 'nonce');
        loginError.classList.add('hidden');
    }
});

// ---- Password Login ----
document.getElementById('loginBtn').onclick = async () => {
    const username = document.getElementById('loginUser').value.trim();
    const password = document.getElementById('loginPass').value;
    if (!username || !password) return showLoginError('Usuario y contraseña requeridos.');

    try {
        const res = await apiCall('/api/auth/login', 'POST', { username, password });
        if (res.status === 'ok') {
            setAuth(res);
        } else {
            showLoginError(res.message || 'Credenciales inválidas.');
        }
    } catch(e) { showLoginError('Error de conexión.'); }
};

// ---- Nonce Login ----
document.getElementById('getChallengeBtn').onclick = async () => {
    const username = document.getElementById('nonceUser').value.trim();
    if (!username) return showLoginError('Usuario requerido.');

    try {
        const res = await apiCall('/api/auth/challenge', 'POST', { username });
        if (res.status === 'ok') {
            document.getElementById('nonceValue').value = res.nonce;
            document.getElementById('nonceLabelValue').textContent = res.nonce;
            window._lastNonce = res.nonce;
            window._lastAdminUser = document.getElementById('nonceUser').value.trim();
            document.getElementById('nonceStep1').classList.add('hidden');
            document.getElementById('nonceStep2').classList.remove('hidden');
            loginError.classList.add('hidden');
        } else {
            showLoginError(res.message);
        }
    } catch(e) { showLoginError('Error de conexión.'); }
};

document.getElementById('verifyNonceBtn').onclick = async () => {
    const username = document.getElementById('nonceUser').value.trim();
    const nonce = document.getElementById('nonceValue').value.trim();
    const signature = document.getElementById('nonceSignature').value.trim();
    if (!signature) return showLoginError('Firma requerida. Usa .\\scripts\\sign_nonce.ps1 -Nonce "' + nonce + '"');

    try {
        const res = await apiCall('/api/auth/verify', 'POST', { username, nonce, signature });
        if (res.status === 'ok') {
            setAuth(res);
        } else {
            showLoginError(res.message || 'Firma inválida.');
        }
    } catch(e) { showLoginError('Error de conexión.'); }
};

function showLoginError(msg) {
    loginError.textContent = msg;
    loginError.classList.remove('hidden');
}

function setAuth(res) {
    authToken = res.token;
    authUsername = res.username;
    authIsAdmin = res.is_admin;
    authHasStudy = res.has_study_access;
    authHasProgramming = res.has_programming_access;

    // Notificar al cliente Electron de las credenciales
    if (platformType === 'electron' && window.iafClient) {
        window.iafClient.setCredentials({
            serverUrl: window.location.origin,
            username: res.username,
            token: res.token,
        }).then(() => {
            console.log('[IAF] Credenciales enviadas al cliente Electron');
        }).catch(e => {
            console.warn('[IAF] Error notificando a Electron:', e);
        });
    }

    showApp();
}

function showApp() {
    loginScreen.classList.add('hidden');
    appContainer.classList.remove('hidden');
    userBadge.textContent = authUsername + ' ' + platformLabel + (authIsAdmin ? ' 👑' : '');
    if (authIsAdmin) adminPanel.classList.remove('hidden');
    if (!authHasProgramming) document.getElementById('modeProgramming').classList.add('hidden');
    if (!authHasStudy) document.getElementById('modeStudy').classList.add('hidden');

    loadProjects();
    loadPrompts();
    loadChatHistory();
}

// ---- Logout ----
document.getElementById('logoutBtn').onclick = async () => {
    if (authToken && authToken !== 'admin_local') {
        await apiCall('/api/auth/logout', 'POST', { token: authToken });
    }
    // Desconectar cliente Electron
    if (platformType === 'electron' && window.iafClient) {
        try { await window.iafClient.disconnect(); } catch(e) {}
    }
    authToken = null;
    authUsername = null;
    authIsAdmin = false;
    if (!isPort80) {
        appContainer.classList.add('hidden');
        loginScreen.classList.remove('hidden');
    }
};

// ---- Auth headers for all API calls ----
async function apiCall(endpoint, method = 'GET', body = null) {
    const opts = { method, headers: { 'Content-Type': 'application/json' } };
    if (authToken && authToken !== 'admin_local') {
        opts.headers['Authorization'] = 'Bearer ' + authToken;
    }
    if (body) opts.body = JSON.stringify(body);
    const res = await fetch(endpoint, opts);
    const text = await res.text();
    try {
        return JSON.parse(text);
    } catch (e) {
        if (text.length === 0) {
            console.warn('apiCall: respuesta vacía de ' + endpoint + ' (HTTP ' + res.status + ')');
        } else {
            console.warn('apiCall: respuesta no-JSON de ' + endpoint + ' (HTTP ' + res.status + '):', text.substring(0, 200));
        }
        return { status: 'error', message: 'Respuesta inválida del servidor (HTTP ' + res.status + ')' };
    }
}

// ---- Mode Toggle ----
document.getElementById('modeProgramming').onclick = () => switchMode('programming');
document.getElementById('modeStudy').onclick = () => switchMode('study');

function switchMode(mode) {
    document.querySelectorAll('.mode-btn').forEach(b => b.classList.remove('active'));
    document.getElementById(mode === 'study' ? 'modeStudy' : 'modeProgramming').classList.add('active');
    document.getElementById('activeMode').textContent = mode === 'study' ? '📚 Estudiar' : '💻 Programar';
    studyProfileSection.classList.toggle('hidden', mode !== 'study');
    if (mode === 'study') {
        loadStudyProfile();
    }
}

// ---- Admin Panel ----
document.getElementById('adminUsersBtn').onclick = openAdminUsers;
document.getElementById('adminPromptsBtn').onclick = () => {
    document.querySelector('.config-section').scrollIntoView({ behavior: 'smooth' });
};

async function openAdminUsers() {
    const modal = document.getElementById('adminUsersModal');
    modal.classList.remove('hidden');
    await refreshUsersTable();
}

async function refreshUsersTable() {
    const tbody = document.getElementById('usersTableBody');
    try {
        const res = await apiCall('/api/admin/users');
        if (res.status !== 'ok') return;
        tbody.innerHTML = res.users.map(u => `
            <tr style="border-bottom:1px solid var(--border-color);">
                <td style="padding:6px;">${u.username}${u.is_admin ? ' 👑' : ''}</td>
                <td>${u.is_admin ? '✅' : '❌'}</td>
                <td>${u.has_study_access ? '✅' : '❌'}</td>
                <td>${u.has_programming_access ? '✅' : '❌'}</td>
                <td><button class="btn btn-warning btn-sm" onclick="editUser('${u.username}')">Editar</button></td>
            </tr>
        `).join('');
    } catch(e) {}
}

// ============================================================================
// EDIT USER - with schedule, activation, granular permissions
// ============================================================================

const DAY_NAMES = ['lunes', 'martes', 'miercoles', 'jueves', 'viernes', 'sabado', 'domingo'];

function buildScheduleGrid(schedule) {
    const grid = document.getElementById('editScheduleGrid');
    grid.innerHTML = DAY_NAMES.map(day => {
        const ranges = (schedule && schedule.horarios && schedule.horarios[day])
            ? schedule.horarios[day].map(r => r.join('-')).join(',')
            : '';
        return '<span class="day-label">' + day + '</span><input type="text" data-day="' + day + '" value="' + ranges + '" placeholder="9-12,14-18">';
    }).join('');
}

function parseScheduleGrid() {
    const horarios = {};
    document.querySelectorAll('#editScheduleGrid input[type="text"]').forEach(input => {
        const day = input.dataset.day;
        const raw = input.value.trim();
        if (!raw) { horarios[day] = []; return; }
        const ranges = raw.split(',').map(s => s.trim()).filter(Boolean).map(rangeStr => {
            const parts = rangeStr.split('-').map(Number);
            if (parts.length === 2 && !isNaN(parts[0]) && !isNaN(parts[1])) {
                return [parts[0], parts[1]];
            }
            return null;
        }).filter(Boolean);
        horarios[day] = ranges;
    });
    return horarios;
}

async function editUser(username) {
    const res = await apiCall('/api/admin/users');
    const user = res.users.find(u => u.username === username);
    if (!user) return;

    document.getElementById('editUsername').textContent = username;
    document.getElementById('editPassword').value = '';

    // Limits
    const lim = user.limits || {};
    document.getElementById('editMaxTokens').value = lim.max_tokens_per_day ?? 0;
    document.getElementById('editMaxApiCalls').value = lim.max_api_calls_per_day ?? 0;
    document.getElementById('editMaxIterations').value = lim.limite_iteraciones ?? 0;
    document.getElementById('editMaxSubAgents').value = lim.max_sub_agents ?? 1;

    // Activation
    document.getElementById('editActivacion').checked = lim.activacion !== false;

    // Permission toggles
    document.getElementById('editCanFork').checked = lim.can_fork_repos || false;
    document.getElementById('editCanExecPS').checked = lim.can_execute_powershell || false;
    document.getElementById('editCanWrite').checked = lim.can_write_files || false;
    document.getElementById('editCanSearchGoogle').checked = (lim.allowed_tools || []).includes('search_google');
    document.getElementById('editStudyAccess').checked = user.has_study_access || false;
    document.getElementById('editProgAccess').checked = user.has_programming_access || false;
    document.getElementById('editGlobalPromptPerm').checked = user.editar_system_prompt_global || false;
    document.getElementById('editLocalPromptPerm').checked = user.editar_system_prompt_local || false;

    // Schedule
    buildScheduleGrid(lim.horarios || {});

    document.getElementById('adminEditUserModal').classList.remove('hidden');
}

document.getElementById('closeAdminUsersBtn').onclick = () => {
    document.getElementById('adminUsersModal').classList.add('hidden');
};

document.getElementById('closeEditUserBtn').onclick = () => {
    document.getElementById('adminEditUserModal').classList.add('hidden');
};

document.getElementById('saveEditUserBtn').onclick = async () => {
    const username = document.getElementById('editUsername').textContent;
    const pwd = document.getElementById('editPassword').value.trim();
    const maxTokensRaw = document.getElementById('editMaxTokens').value.trim();
    const maxApiCallsRaw = document.getElementById('editMaxApiCalls').value.trim();
    const maxIterRaw = document.getElementById('editMaxIterations').value.trim();
    const maxSub = parseInt(document.getElementById('editMaxSubAgents').value) || 1;
    const activacion = document.getElementById('editActivacion').checked;

    // Build allowed_tools
    const allowedTools = ['read_file', 'search_code'];
    if (document.getElementById('editCanSearchGoogle').checked) allowedTools.push('search_google');

    const limits = {
        activacion: activacion,
        max_tokens_per_day: maxTokensRaw === '' || maxTokensRaw === '0' ? 0 : parseInt(maxTokensRaw),
        max_api_calls_per_day: maxApiCallsRaw === '' || maxApiCallsRaw === '0' ? 0 : parseInt(maxApiCallsRaw),
        limite_iteraciones: maxIterRaw === '' || maxIterRaw === '0' ? 0 : parseInt(maxIterRaw),
        max_sub_agents: maxSub,
        max_projects: 2,
        allowed_tools: allowedTools,
        can_fork_repos: document.getElementById('editCanFork').checked,
        can_execute_powershell: document.getElementById('editCanExecPS').checked,
        can_write_files: document.getElementById('editCanWrite').checked,
        horarios: { horarios: parseScheduleGrid() },
    };

    // Update limits
    await apiCall('/api/admin/users/' + username + '/limits', 'PUT', { limits });

    // Update access (granular permissions)
    await apiCall('/api/admin/users/' + username + '/access', 'PUT', {
        modo_estudio: document.getElementById('editStudyAccess').checked,
        modo_programador: document.getElementById('editProgAccess').checked,
        editar_system_prompt_global: document.getElementById('editGlobalPromptPerm').checked,
        editar_system_prompt_local: document.getElementById('editLocalPromptPerm').checked,
    });

    // Update schedule separately
    const horarios = parseScheduleGrid();
    await apiCall('/api/admin/users/' + username + '/schedule', 'PUT', { horarios });

    // Change password if provided
    if (pwd) {
        await apiCall('/api/admin/users/' + username + '/password', 'PUT', { new_password: pwd });
    }

    document.getElementById('adminEditUserModal').classList.add('hidden');
    await refreshUsersTable();
};

document.getElementById('deleteUserBtn').onclick = async () => {
    const username = document.getElementById('editUsername').textContent;
    if (!confirm('¿Eliminar permanentemente a ' + username + '?')) return;
    await apiCall('/api/admin/users/' + username, 'DELETE');
    document.getElementById('adminEditUserModal').classList.add('hidden');
    await refreshUsersTable();
};

// ============================================================================
// CREATE USER - with granular permissions
// ============================================================================

document.getElementById('createUserBtn').onclick = async () => {
    const username = document.getElementById('newUsername').value.trim();
    const isAdmin = document.getElementById('newIsAdmin').checked;

    if (!username) return alert('Username requerido.');

    // Validar confirmacion de contrasena
    if (!isAdmin) {
        const pwd = document.getElementById('newPassword').value;
        const pwdConfirm = document.getElementById('newPasswordConfirm').value;
        if (!pwd) return alert('La contrasena es requerida para usuarios no-admin.');
        if (pwd !== pwdConfirm) return alert('Las contrasenas no coinciden.');
    }

    // Build allowed_tools from checkboxes
    const allowedTools = ['read_file', 'search_code'];
    if (document.getElementById('newCanSearchGoogle').checked) allowedTools.push('search_google');

    const payload = {
        username: username,
        is_admin: isAdmin,
        modo_estudio: document.getElementById('newStudyAccess').checked,
        modo_programador: document.getElementById('newProgAccess').checked,
        editar_system_prompt_global: document.getElementById('newEditGlobalPrompt').checked,
        editar_system_prompt_local: document.getElementById('newEditLocalPrompt').checked,
        permissions: allowedTools,
    };

    if (isAdmin) {
        const createRes = await apiCall('/api/admin/users', 'POST', payload);
        if (createRes.status === 'ok') {
            if (createRes.user && createRes.user.public_key) {
                showInfoToast('✅ Admin ' + username + ' creado. Clave pública generada.');
            } else {
                showInfoToast('✅ Admin ' + username + ' creado.');
            }
            document.getElementById('newUsername').value = '';
            document.getElementById('newPassword').value = '';
            document.getElementById('newPasswordConfirm').value = '';
            document.getElementById('newPublicKey').value = '';
            await refreshUsersTable();
        } else {
            alert('Error: ' + (createRes.message || 'No se pudo crear el usuario.'));
        }
    } else {
        const pwd = document.getElementById('newPassword').value;
        if (!pwd) return alert('La contrasena es requerida.');
        payload.password = pwd;
        const createRes = await apiCall('/api/admin/users', 'POST', payload);
        if (createRes.status === 'ok') {
            showInfoToast('✅ Usuario ' + username + ' creado.');
            document.getElementById('newUsername').value = '';
            document.getElementById('newPassword').value = '';
            document.getElementById('newPasswordConfirm').value = '';
            await refreshUsersTable();
        } else {
            alert('Error: ' + (createRes.message || 'No se pudo crear el usuario.'));
        }
    }
};

function toggleAdminCreateMode() {
    const isAdmin = document.getElementById('newIsAdmin').checked;
    document.getElementById('newPublicKeyContainer').classList.toggle('hidden', !isAdmin);
    document.getElementById('uploadPemBtn').classList.toggle('hidden', !isAdmin);
    document.getElementById('generateKeysBtn').classList.toggle('hidden', !isAdmin);
    // Mostrar/ocultar password fields
    const pwdInputs = [document.getElementById('newPassword'), document.getElementById('newPasswordConfirm')];
    pwdInputs.forEach(el => {
        if (el) el.parentElement.style.display = isAdmin ? 'none' : '';
    });
}

document.getElementById('generateKeysBtn').onclick = async () => {
    const res = await apiCall('/api/auth/keygen');
    if (res.status === 'ok') {
        document.getElementById('newPublicKey').value = res.public_key;
        alert('⚠️ GUARDA ESTA CLAVE PRIVADA. ES LA ÚNICA VEZ QUE LA VERÁS:\n\n' + res.private_key);
    }
};

document.getElementById('uploadPemBtn').onclick = () => {
    document.getElementById('pemFileInput').click();
};

document.getElementById('pemFileInput').onchange = async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const text = await file.text();
    // Extract public key from PEM
    const pubMatch = text.match(/-----BEGIN PUBLIC KEY-----[\s\S]*?-----END PUBLIC KEY-----/);
    if (pubMatch) {
        document.getElementById('newPublicKey').value = pubMatch[0].replace(/-----.*?-----/g, '').replace(/\s/g, '');
        showInfoToast('✅ Clave pública extraída del .pem');
    } else {
        alert('No se encontró una clave pública en el archivo .pem');
    }
};

// ============================================================================
// PROJECTS
// ============================================================================

async function loadProjects() {
    try {
        const res = await apiCall('/api/projects');
        if (res.status === 'ok' && res.projects) {
            const list = document.getElementById('projectList');
            list.innerHTML = res.projects.map(p => {
                const isActive = activeProject === p.name ? ' active' : '';
                return '<div class="project-item' + isActive + '" onclick="selectProject(\'' + p.name + '\')">' + p.name + '</div>';
            }).join('');
        }
    } catch(e) {}
}

function selectProject(name) {
    activeProject = name;
    document.getElementById('activeProjectName').innerText = name;
    loadProjects();
    loadPrompts();
}

document.getElementById('forkBtn').onclick = async () => {
    const url = document.getElementById('repoUrl').value.trim();
    if (!url) return alert('URL del repositorio requerida.');
    const btn = document.getElementById('forkBtn');
    btn.disabled = true;
    btn.textContent = 'Forkeando...';
    try {
        const res = await apiCall('/api/projects/fork', 'POST', { repo_url: url });
        if (res.status === 'ok') {
            document.getElementById('repoUrl').value = '';
            showInfoToast('✅ Repo forkeado: ' + res.name);
            await loadProjects();
        } else {
            alert('Error: ' + (res.message || 'No se pudo forkear.'));
        }
    } catch(e) { alert('Error de conexión.'); }
    btn.disabled = false;
    btn.textContent = 'Fork & Clone';
};

document.getElementById('addLocalBtn').onclick = async () => {
    const name = document.getElementById('localProjName').value.trim();
    const path = document.getElementById('localProjPath').value.trim();
    if (!name || !path) return alert('Nombre y ruta requeridos.');
    const btn = document.getElementById('addLocalBtn');
    btn.disabled = true;
    btn.textContent = 'Agregando...';
    try {
        const res = await apiCall('/api/projects/local', 'POST', { name: name, path: path });
        if (res.status === 'ok') {
            document.getElementById('localProjName').value = '';
            document.getElementById('localProjPath').value = '';
            showInfoToast('✅ Proyecto local agregado: ' + name);
            await loadProjects();
        } else {
            alert('Error: ' + (res.message || 'No se pudo agregar el proyecto.'));
        }
    } catch(e) { alert('Error de conexión al agregar proyecto.'); }
    btn.disabled = false;
    btn.textContent = 'Agregar Carpeta';
};

// ============================================================================
// PROMPTS
// ============================================================================

async function loadPrompts() {
    try {
        const prompts = await apiCall('/api/prompts');
        if (prompts.status === 'ok') {
            document.getElementById('globalPrompt').value = prompts.global_current || '';
            if (activeProject && prompts.projects && prompts.projects[activeProject]) {
                document.getElementById('localPrompt').value = prompts.projects[activeProject];
            } else {
                document.getElementById('localPrompt').value = '';
            }
        }
    } catch(e) {}
}

document.getElementById('savePromptsBtn').onclick = async () => {
    const globalContent = document.getElementById('globalPrompt').value;
    const globalRes = await apiCall('/api/prompts/global', 'POST', { content: globalContent });
    if (activeProject && document.getElementById('localPrompt').value.trim()) {
        const localContent = document.getElementById('localPrompt').value;
        await apiCall('/api/prompts/local', 'POST', { project_name: activeProject, content: localContent });
    }
    if (globalRes.status === 'ok') {
        showInfoToast('✅ Prompts guardados.');
    } else {
        alert('Error al guardar prompt global: ' + (globalRes.message || ''));
    }
};

document.getElementById('resetPromptBtn').onclick = async () => {
    if (!confirm('¿Restaurar el prompt global al valor por defecto?')) return;
    const res = await apiCall('/api/prompts/global/reset', 'POST');
    if (res.status === 'ok') {
        document.getElementById('globalPrompt').value = res.content || '';
        showInfoToast('🔄 Prompt global restaurado.');
    } else {
        alert('Error: ' + (res.message || 'No se pudo restaurar.'));
    }
};

// ============================================================================
// STUDY PROFILE
// ============================================================================

async function loadStudyProfile() {
    try {
        const res = await apiCall('/api/study/profile');
        if (res.status === 'ok' && res.profile) {
            const p = res.profile;
            document.getElementById('profileAge').value = p.age || '';
            document.getElementById('profileGames').value = (p.favorite_games || []).join(', ');
            document.getElementById('profileHobbies').value = (p.hobbies || []).join(', ');
            document.getElementById('profileNeuro').value = (p.neurological_conditions || []).join(', ');

            const phaseMap = { 'NotStarted': 'No iniciado', 'Exploration': '🔍 Exploración', 'Exploitation': '🎯 Explotación' };
            const phaseEl = document.getElementById('studyPhase');
            phaseEl.innerHTML = '<b>Fase:</b> ' + (phaseMap[p.phase] || p.phase || 'No definida');

            if (res.engagement) {
                phaseEl.innerHTML += ' | <b>Engagement:</b> ' + Math.round(res.engagement * 100) + '%';
            }
        }
    } catch(e) {}
}

document.getElementById('saveProfileBtn').onclick = async () => {
    const age = parseInt(document.getElementById('profileAge').value) || null;
    const games = document.getElementById('profileGames').value.split(',').map(s => s.trim()).filter(Boolean);
    const hobbies = document.getElementById('profileHobbies').value.split(',').map(s => s.trim()).filter(Boolean);
    const neuro = document.getElementById('profileNeuro').value.split(',').map(s => s.trim()).filter(Boolean);

    try {
        const res = await apiCall('/api/study/profile', 'POST', {
            age: age,
            favorite_games: games,
            hobbies: hobbies,
            neurological_conditions: neuro,
        });
        if (res.status === 'ok') {
            showInfoToast('✅ Perfil guardado.');
        }
    } catch(e) {
        alert('Error de conexión al guardar perfil.');
    }
};

// ============================================================================
// CHAT HISTORY — con deduplicación y etiqueta de usuario para admin
// ============================================================================

async function loadChatHistory() {
    try {
        const res = await apiCall('/api/chats');
        const list = document.getElementById('chatHistoryList');
        if (res.chats && Array.isArray(res.chats)) {
            // DEDUP: usar un Set para evitar IDs duplicados (defensa en profundidad)
            const seen = new Set();
            const unique = [];
            for (let i = 0; i < res.chats.length; i++) {
                const c = res.chats[i];
                if (!seen.has(c.id)) {
                    seen.add(c.id);
                    unique.push(c);
                }
            }
            list.innerHTML = unique.map(c => {
                const isActive = currentSessionId === c.id ? ' active' : '';
                // Mostrar etiqueta de usuario para admins
                const userLabel = (authIsAdmin && c.username && c.username !== authUsername)
                    ? ' <span style="font-size:10px;color:var(--text-muted);">(@' + c.username + ')</span>'
                    : '';
                return '<div class="project-item' + isActive + '" onclick="selectChatSession(\'' + c.id + '\')">' + c.title + userLabel + '</div>';
            }).join('');
        }
    } catch(e) {}
}

async function selectChatSession(id) {
    currentSessionId = id;
    loadChatHistory();
    const res = await apiCall('/api/chats/' + id);
    if (res.status === 'ok') {
        const chatArea = document.getElementById('chatArea');
        chatArea.innerHTML = '';
        res.session.messages.forEach(m => addMessage(m.role, m.content));
        window._shownInfoMsgs = new Set();
        if (res.session.project_name) {
            document.getElementById('activeProjectName').innerText = res.session.project_name;
            activeProject = res.session.project_name;
            loadProjects();
        }
        if (res.session.steps && Array.isArray(res.session.steps) && res.session.steps.length > 0) {
            renderConsoleSteps(res.session.steps);
        }
        startAgentMonitoring();
    }
}

document.getElementById('newChatBtn').onclick = () => {
    currentSessionId = null;
    window._shownInfoMsgs = new Set();
    document.getElementById('chatArea').innerHTML = '<div class="message system-msg"><strong>Sistema:</strong> Nuevo chat iniciado.</div>';
    if (agentMonitorInterval) {
        clearInterval(agentMonitorInterval);
        agentMonitorInterval = null;
    }
    document.getElementById('interruptBtn').classList.add('hidden');
    const consoleArea = document.getElementById('consoleArea');
    if (consoleArea) consoleArea.innerHTML = '';
    agentQuestionShown = false;
    agentPlanShown = false;
    loadChatHistory();
};

// ---- Send Message ----
const SEND_DEBOUNCE_MS = 500;
let sendTimeout;

document.getElementById('sendBtn').onclick = () => {
    const text = document.getElementById('chatInput').value.trim();
    if (!text) return;
    document.getElementById('sendBtn').disabled = true;
    clearTimeout(sendTimeout);
    sendTimeout = setTimeout(async () => {
        try {
            const mode = document.getElementById('modeStudy').classList.contains('active') ? 'study' : 'programming';
            const refineRes = await apiCall('/api/prompts/refine', 'POST', {
                prompt: text, session_id: currentSessionId, project_name: activeProject
            });
            if (refineRes.status === 'ok') {
                pendingMessageToSend = text;
                document.getElementById('refinedPromptText').value = refineRes.refined;
                document.getElementById('refinePromptModal').classList.remove('hidden');
            } else {
                document.getElementById('chatInput').value = '';
                await sendMessageToAgent(text, mode);
            }
        } catch(e) {
            document.getElementById('chatInput').value = '';
            await sendMessageToAgent(text, 'programming');
        }
        document.getElementById('sendBtn').disabled = false;
    }, SEND_DEBOUNCE_MS);
};

document.getElementById('sendDirectBtn').onclick = async () => {
    const text = document.getElementById('chatInput').value.trim();
    if (!text) return;
    const btn = document.getElementById('sendDirectBtn');
    btn.disabled = true;
    document.getElementById('chatInput').value = '';
    const mode = document.getElementById('modeStudy').classList.contains('active') ? 'study' : 'programming';
    await sendMessageToAgent(text, mode);
    btn.disabled = false;
};

document.getElementById('applyRefinedPromptBtn').onclick = async () => {
    document.getElementById('refinePromptModal').classList.add('hidden');
    if (pendingMessageToSend) {
        const finalText = document.getElementById('refinedPromptText').value.trim() || pendingMessageToSend;
        document.getElementById('chatInput').value = '';
        const mode = document.getElementById('modeStudy').classList.contains('active') ? 'study' : 'programming';
        await sendMessageToAgent(finalText, mode);
        pendingMessageToSend = null;
    }
};

document.getElementById('reRefinePromptBtn').onclick = async () => {
    const currentText = document.getElementById('refinedPromptText').value.trim();
    const feedback = document.getElementById('refinePromptFeedback').value.trim();
    if (!currentText) return;
    const btn = document.getElementById('reRefinePromptBtn');
    btn.disabled = true; btn.innerText = 'Re-Refinando...';
    const refineRes = await apiCall('/api/prompts/refine', 'POST', {
        prompt: currentText, feedback: feedback, session_id: currentSessionId, project_name: activeProject
    });
    if (refineRes.status === 'ok') {
        document.getElementById('refinedPromptText').value = refineRes.refined;
        document.getElementById('refinePromptFeedback').value = '';
    } else { alert('Error: ' + refineRes.message); }
    btn.innerText = 'Re-Refinar'; btn.disabled = false;
};

document.getElementById('cancelRefinedPromptBtn').onclick = async () => {
    document.getElementById('refinePromptModal').classList.add('hidden');
    if (pendingMessageToSend) {
        document.getElementById('chatInput').value = '';
        await sendMessageToAgent(pendingMessageToSend, 'programming');
        pendingMessageToSend = null;
    }
};

async function sendMessageToAgent(text, mode) {
    addMessage('user', text);
    agentQuestionShown = false;
    agentPlanShown = false;
    const res = await apiCall('/api/chat', 'POST', {
        message: text, project_name: activeProject,
        session_id: currentSessionId, mode: mode
    });
    if (res.status === 'ok') {
        currentSessionId = res.session_id;
        loadChatHistory();
        startAgentMonitoring();
    } else {
        addMessage('agent', 'Error: ' + res.message);
    }
}

// ============================================================================
// AGENT MONITORING
// ============================================================================

function startAgentMonitoring() {
    if (agentMonitorInterval) clearInterval(agentMonitorInterval);
    updateConsoleStatus('🔍 Agente iniciando...');

    agentMonitorInterval = setInterval(async () => {
        try {
            const res = await apiCall('/api/agent/status');
            if (res.status !== 'ok') return;

            const interruptBtn = document.getElementById('interruptBtn');
            if (interruptBtn) {
                if (res.running) { interruptBtn.classList.remove('hidden'); }
                else if (res.finished || res.interrupted || !res.active) { interruptBtn.classList.add('hidden'); }
            }

            if (res.thinking_content && Array.isArray(res.thinking_content)) {
                const thinking = res.thinking_content.join('\n');
                if (thinking.trim()) { updateConsoleThinking(thinking); }
            }

            if (res.steps && Array.isArray(res.steps)) {
                renderConsoleSteps(res.steps);
            }

            if (res.info_messages && Array.isArray(res.info_messages)) {
                if (!window._shownInfoMsgs) window._shownInfoMsgs = new Set();
                res.info_messages.forEach(msg => {
                    if (!window._shownInfoMsgs.has(msg)) {
                        window._shownInfoMsgs.add(msg);
                        if (msg.startsWith('[NOTIF] ')) {
                            addMessage('agent', 'ℹ️ ' + msg.slice(8), 'info-msg');
                        } else {
                            addMessage('agent', msg);
                        }
                    }
                });
                if (window._shownInfoMsgs.size > 200) window._shownInfoMsgs = new Set();
            }

            if (res.esperando_respuesta_usuario && res.pregunta_usuario && !agentQuestionShown) {
                agentQuestionShown = true;
                showAgentQuestionBanner(res.pregunta_usuario);
            }

            if (res.esperando_aprobacion_plan && res.plan_propuesto && !agentPlanShown) {
                agentPlanShown = true;
                showAgentPlanModal(res.plan_propuesto);
            }

            if (res.finished) {
                clearInterval(agentMonitorInterval);
                agentMonitorInterval = null;
                if (res.final_message) {
                    addMessage('agent', '✅ ' + res.final_message, 'final-msg');
                }
                loadChatHistory();
                updateConsoleStatus('✅ Agente finalizado.');
                const ib = document.getElementById('interruptBtn');
                if (ib) ib.classList.add('hidden');
            }

            if (res.interrupted) {
                updateConsoleStatus('⚠️ Agente interrumpido.');
                const ib = document.getElementById('interruptBtn');
                if (ib) ib.classList.add('hidden');
            }
        } catch(e) { console.error('[IAF] Error en monitoreo del agente:', e); }
    }, 1000);
}

// ---- Interrupt Button ----
document.getElementById('interruptBtn').onclick = async () => {
    const res = await apiCall('/api/agent/interrupt', 'POST');
    if (res.status === 'ok') {
        showInfoToast('⏸️ Agente interrumpido.');
    }
};

// ---- Agent Answer ----
async function responderAgente(respuesta) {
    addMessage('user', respuesta);
    document.getElementById('agentQuestionBanner').classList.add('hidden');
    agentQuestionShown = false;
    await apiCall('/api/agent/responder', 'POST', { respuesta: respuesta });
}

// ---- Agent Approve Plan ----
async function aprobarPlan(aprobado, feedback) {
    document.getElementById('agentPlanModal').classList.add('hidden');
    agentPlanShown = false;
    await apiCall('/api/agent/aprobar_plan', 'POST', { aprobado: aprobado, feedback: feedback || '' });
}

// ============================================================================
// UI HELPERS
// ============================================================================

function showAgentQuestionBanner(pregunta) {
    const banner = document.getElementById('agentQuestionBanner');
    document.getElementById('agentQuestionPrompt').textContent = pregunta;
    document.getElementById('agentQuestionResponse').value = '';
    banner.classList.remove('hidden');
    document.getElementById('agentQuestionResponse').focus();
}

document.getElementById('submitAgentResponseBtn').onclick = () => {
    const respuesta = document.getElementById('agentQuestionResponse').value.trim();
    if (!respuesta) return;
    responderAgente(respuesta);
};

document.getElementById('agentQuestionResponse').addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        document.getElementById('submitAgentResponseBtn').click();
    }
});

function showAgentPlanModal(plan) {
    const modal = document.getElementById('agentPlanModal');
    document.getElementById('agentPlanContent').textContent = plan;
    modal.classList.remove('hidden');
}

document.getElementById('approvePlanBtn').onclick = () => { aprobarPlan(true, ''); };
document.getElementById('rejectPlanBtn').onclick = () => { aprobarPlan(false, ''); };

function addMessage(role, text, extraClass) {
    const chatArea = document.getElementById('chatArea');
    const div = document.createElement('div');
    div.classList.add('message');
    if (role === 'user') { div.classList.add('user-msg'); }
    else if (role === 'agent') { div.classList.add('agent-msg'); }
    if (extraClass) div.classList.add(extraClass);

    if (role === 'agent' && typeof marked !== 'undefined') {
        try { div.innerHTML = marked.parse(text); }
        catch(e) { div.textContent = text; }
    } else {
        div.textContent = text;
    }

    chatArea.appendChild(div);
    chatArea.scrollTop = chatArea.scrollHeight;
}

function showInfoToast(msg) {
    const toast = document.getElementById('infoToast');
    if (!toast) {
        const t = document.createElement('div');
        t.id = 'infoToast';
        t.className = 'info-toast';
        t.textContent = msg;
        document.body.appendChild(t);
        setTimeout(() => { t.classList.add('show'); }, 10);
        setTimeout(() => { t.classList.remove('show'); setTimeout(() => { t.remove(); }, 300); }, 3000);
        return;
    }
    toast.textContent = msg;
    toast.classList.add('show');
    setTimeout(() => { toast.classList.remove('show'); }, 3000);
}

// ============================================================================
// CONSOLE DE AUDITORÍA
// ============================================================================

function renderConsoleSteps(steps) {
    const consoleArea = document.getElementById('consoleArea');
    if (!consoleArea) return;
    if (!steps || steps.length === 0) {
        const emptyEl = consoleArea.querySelector('.console-empty');
        if (emptyEl && window._agentRunning) {
            emptyEl.textContent = '🔍 Agente ejecutándose, esperando primer paso...';
            emptyEl.className = 'console-step';
        }
        return;
    }

    const lastStep = steps[steps.length - 1];
    const hash = steps.length + '-' + (lastStep ? (lastStep.title || '') : '');
    if (consoleArea.dataset.stepsHash === hash) return;
    consoleArea.dataset.stepsHash = hash;

    let html = '';
    steps.forEach((step, i) => {
        const icon = step.step_type === 'thinking' ? '🧠' :
                     step.step_type === 'tool_call' ? '🔧' :
                     step.step_type === 'tool_result' ? '📋' :
                     step.step_type === 'info' ? 'ℹ️' : '📌';
        html += '<div class="console-step"><span class="console-step-icon">' + icon + '</span><span class="console-step-title">#' + (i + 1) + ' ' + step.title + '</span></div>';
    });
    consoleArea.innerHTML = html;
    consoleArea.scrollTop = consoleArea.scrollHeight;
}

function updateConsoleThinking(thinking) {
    const consoleArea = document.getElementById('consoleArea');
    if (!consoleArea) return;
    let thinkDiv = consoleArea.querySelector('.console-thinking');
    if (!thinkDiv) {
        thinkDiv = document.createElement('div');
        thinkDiv.className = 'console-thinking';
        consoleArea.appendChild(thinkDiv);
    }
    const short = thinking.length > 500 ? '...' + thinking.slice(-500) : thinking;
    thinkDiv.textContent = '🧠 Thinking: ' + short;
    consoleArea.scrollTop = consoleArea.scrollHeight;
}

function updateConsoleStatus(status) {
    const consoleArea = document.getElementById('consoleArea');
    if (!consoleArea) return;
    const div = document.createElement('div');
    div.className = 'console-status';
    div.textContent = status;
    consoleArea.appendChild(div);
    consoleArea.scrollTop = consoleArea.scrollHeight;
}

// ============================================================================
// MOBILE HAMBURGER MENU
// ============================================================================

function initMobileNav() {
    const hamburgerBtn = document.getElementById('hamburgerBtn');
    const sidebar = document.querySelector('.sidebar');
    const overlay = document.getElementById('sidebarOverlay');

    if (!hamburgerBtn || !sidebar || !overlay) return;

    function openSidebar() {
        sidebar.style.transform = 'translateX(0)';
        overlay.classList.remove('hidden');
        overlay.style.display = 'block';
        document.body.style.overflow = 'hidden';
    }

    function closeSidebar() {
        sidebar.style.transform = 'translateX(-100%)';
        overlay.classList.add('hidden');
        overlay.style.display = 'none';
        document.body.style.overflow = '';
    }

    hamburgerBtn.onclick = openSidebar;
    overlay.onclick = closeSidebar;

    // Cerrar sidebar al seleccionar un proyecto, chat, o cambiar modo
    sidebar.addEventListener('click', (e) => {
        if (e.target.closest('.project-item') ||
            e.target.closest('.mode-btn') ||
            e.target.closest('#newChatBtn') ||
            e.target.closest('#logoutBtn')) {
            setTimeout(closeSidebar, 150);
        }
    });
}

// ============================================================================
// INIT
// ============================================================================
init();
