// ============================================================================
// IAF — app.js — Cliente Web con Autenticación
// ============================================================================

// ---- State ----
let activeProject = null;
let currentSessionId = null;
let agentMonitorInterval = null;
let currentCaptcha = null;
let pendingMessageToSend = null;
let agentQuestionShown = false;  // evita abrir el modal repetidamente
let agentPlanShown = false;      // evita abrir el modal repetidamente

// Auth state
let authToken = null;
let authUsername = null;
let authIsAdmin = false;
let authHasStudy = false;
let authHasProgramming = false;
let isPort80 = window.location.port === '80' || window.location.port === '';

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

// ---- Init ----

// ---- Helpers: Toggle Password Visibility ----
function togglePassword(fieldId) {
    const el = document.getElementById(fieldId);
    if (!el) return;
    el.type = el.type === 'password' ? 'text' : 'password';
}

/**
 * Copia el comando sign_nonce al portapapeles.
 * Ahora recibe 'event' explícitamente y tiene fallback para navegadores
 * sin Clipboard API (HTTP, navegadores antiguos).
 */
function copyNonceCmd(event) {
    // Normalizar el evento (soporte cross-browser)
    event = event || window.event;
    const nonce = window._lastNonce || '';
    const cmd = '.\\scripts\\sign_nonce.ps1 -Nonce "' + nonce + '" -KeyPath ".config\\admin_private.pem"';

    // Resolver el botón que disparó el evento
    var btn = null;
    if (event && event.target) {
        btn = event.target;
    } else {
        // Fallback: buscar por clase
        btn = document.querySelector('.btn-copy-small');
    }

    /**
     * Fallback: copia usando textarea + execCommand.
     * Funciona en HTTP y navegadores sin Clipboard API.
     */
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
        try {
            ok = document.execCommand('copy');
        } catch (e) {
            ok = false;
        }
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

    // Intentar primero la API moderna
    if (navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
        navigator.clipboard.writeText(cmd).then(onSuccess).catch(function () {
            // Si falla (HTTP no seguro), usar fallback
            if (fallbackCopy(cmd)) {
                onSuccess();
            } else {
                onFailure();
            }
        });
    } else {
        // Sin Clipboard API, usar fallback directamente
        if (fallbackCopy(cmd)) {
            onSuccess();
        } else {
            onFailure();
        }
    }
}

async function init() {
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
        await checkClient();
    }
}

async function checkClient() {
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
    showApp();
}

function showApp() {
    loginScreen.classList.add('hidden');
    appContainer.classList.remove('hidden');
    userBadge.textContent = authUsername + (authIsAdmin ? ' 👑' : '');
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
        return `<span class="day-label">${day}</span><input type="text" data-day="${day}" value="${ranges}" placeholder="9-12,14-18">`;
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
    await apiCall(`/api/admin/users/${username}/limits`, 'PUT', { limits });

    // Update access (granular permissions)
    await apiCall(`/api/admin/users/${username}/access`, 'PUT', {
        modo_estudio: document.getElementById('editStudyAccess').checked,
        modo_programador: document.getElementById('editProgAccess').checked,
        editar_system_prompt_global: document.getElementById('editGlobalPromptPerm').checked,
        editar_system_prompt_local: document.getElementById('editLocalPromptPerm').checked,
    });

    // Update schedule separately
    const horarios = parseScheduleGrid();
    await apiCall(`/api/admin/users/${username}/schedule`, 'PUT', { horarios });

    // Change password if provided
    if (pwd) {
        await apiCall(`/api/admin/users/${username}/password`, 'PUT', { new_password: pwd });
    }

    document.getElementById('adminEditUserModal').classList.add('hidden');
    await refreshUsersTable();
};

document.getElementById('deleteUserBtn').onclick = async () => {
    const username = document.getElementById('editUsername').textContent;
    if (!confirm(`¿Eliminar permanentemente a ${username}?`)) return;
    await apiCall(`/api/admin/users/${username}`, 'DELETE');
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
        username,
        is_admin: isAdmin,
        modo_estudio: document.getElementById('newStudyAccess').checked,
        modo_programador: document.getElementById('newProgAccess').checked,
        editar_system_prompt_global: document.getElementById('newEditGlobalPrompt').checked,
        editar_system_prompt_local: document.getElementById('newEditLocalPrompt').checked,
        permissions: allowedTools,
    };

    if (isAdmin) {
        const publicKey = document.getElementById('newPublicKey').value.trim();
        if (!publicKey || publicKey.length < 64) {
            return alert('Para crear un admin se requiere la clave pública (64 caracteres hex). Generá un par de claves con el botón "Generar Par de Claves".');
        }
        payload.public_key = publicKey;
    } else {
        const password = document.getElementById('newPassword').value;
        payload.password = password;
    }

    try {
        const res = await apiCall('/api/admin/users', 'POST', payload);
        if (res.status === 'ok') {
            document.getElementById('newUsername').value = '';
            document.getElementById('newPassword').value = '';
            document.getElementById('newPasswordConfirm').value = '';
            document.getElementById('newPublicKey').value = '';
            await refreshUsersTable();
        } else {
            alert('Error: ' + (res.message || 'No se pudo crear el usuario.'));
        }
    } catch(e) {
        alert('Error de conexión al crear usuario.');
    }
};

// ============================================================================
// KEY GENERATION
// ============================================================================

let currentServerKey = null;

// ---- Generate Keys locally ----
document.getElementById('generateKeysBtn').onclick = async () => {
    const btn = document.getElementById('generateKeysBtn');
    btn.disabled = true;
    btn.textContent = 'Generando...';
    try {
        const res = await apiCall('/api/auth/generate-keypair', 'GET');
        if (res.status === 'ok') {
            document.getElementById('newPublicKey').value = res.public_key;
            // Create blob and download private key
            const blob = new Blob([res.private_key], { type: 'application/x-pem-file' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'admin_private.pem';
            a.click();
            URL.revokeObjectURL(url);
        } else {
            alert('Error: ' + res.message);
        }
    } catch(e) {
        alert('Error al generar claves.');
    }
    btn.disabled = false;
    btn.textContent = '🔑 Generar Par de Claves';
};

// ---- Upload PEM file ----
document.getElementById('uploadPemBtn').onclick = () => {
    document.getElementById('pemFileInput').click();
};

document.getElementById('pemFileInput').onchange = async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const text = await file.text();
    try {
        const res = await apiCall('/api/auth/upload-public-key', 'POST', { pem_content: text });
        if (res.status === 'ok') {
            document.getElementById('newPublicKey').value = res.public_key;
        } else {
            alert('Error: ' + res.message);
        }
    } catch(e) {
        alert('Error al procesar el archivo PEM.');
    }
};
// ============================================================================
// PROJECTS
// ============================================================================

async function loadProjects() {
    try {
        const res = await apiCall('/api/projects');
        const list = document.getElementById('projectList');
        if (!list) return;

        // Soporta ambos formatos: {projects: [...]} (nuevo) o array pelado (legacy)
        const projects = Array.isArray(res) ? res : (res.projects || []);
        
        if (projects.length > 0) {
            list.innerHTML = projects.map(p => `
                <div class="project-item ${activeProject === p.name ? 'active' : ''}" onclick="selectProject('${p.name}')">${p.name}</div>
            `).join('');
        } else {
            list.innerHTML = '<div class="console-empty">No hay proyectos. Agregá uno local o forkeá un repo.</div>';
        }
    } catch(e) {
        console.error('[IAF] Error cargando proyectos:', e);
        const list = document.getElementById('projectList');
        if (list) list.innerHTML = '<div class="console-empty" style="color:var(--danger);">Error al cargar proyectos. Revisá la consola.</div>';
    }
}


// ---- Fork & Clone ----
document.getElementById('forkBtn').onclick = async () => {
    const url = document.getElementById('repoUrl').value.trim();
    if (!url) return alert('Ingresá una URL de GitHub o usuario/repo.');

    const btn = document.getElementById('forkBtn');
    btn.disabled = true;
    btn.textContent = 'Forkeando...';

    try {
        const res = await apiCall('/api/projects/fork', 'POST', { repo_url: url });
        if (res.status === 'ok') {
            document.getElementById('repoUrl').value = '';
            showInfoToast('✅ Repo forkeado y clonado: ' + (res.project ? res.project.name : url));
            await loadProjects();
        } else {
            alert('Error: ' + (res.message || 'No se pudo forkear el repo.'));
        }
    } catch(e) {
        alert('Error de conexión al forkear.');
    }
    btn.disabled = false;
    btn.textContent = 'Fork & Clone';
};

// ---- Agregar Carpeta Local ----
document.getElementById('addLocalBtn').onclick = async () => {
    const name = document.getElementById('localProjName').value.trim();
    const path = document.getElementById('localProjPath').value.trim();
    if (!name) return alert('Ingresá un nombre para el proyecto.');
    if (!path) return alert('Ingresá la ruta absoluta de la carpeta.');

    const btn = document.getElementById('addLocalBtn');
    btn.disabled = true;
    btn.textContent = 'Agregando...';

    try {
        const res = await apiCall('/api/projects/local', 'POST', { name, path });
        if (res.status === 'ok') {
            document.getElementById('localProjName').value = '';
            document.getElementById('localProjPath').value = '';
            showInfoToast('✅ Proyecto local agregado: ' + name);
            await loadProjects();
        } else {
            alert('Error: ' + (res.message || 'No se pudo agregar el proyecto.'));
        }
    } catch(e) {
        alert('Error de conexión al agregar proyecto.');
    }
    btn.disabled = false;
    btn.textContent = 'Agregar Carpeta';
};
// ============================================================================
// PROMPTS — alineado con IDs del HTML: globalPrompt, localPrompt,
//           savePromptsBtn (guarda ambos), resetPromptBtn (restaura global)
// ============================================================================

async function loadPrompts() {
    try {
        const prompts = await apiCall('/api/prompts');
        if (prompts.status === 'ok') {
            document.getElementById('globalPrompt').value = prompts.global || '';
            if (activeProject && prompts.projects && prompts.projects[activeProject]) {
                document.getElementById('localPrompt').value = prompts.projects[activeProject];
            } else {
                document.getElementById('localPrompt').value = '';
            }
        }
    } catch(e) {}
}

// savePromptsBtn: guarda ambos prompts (global y local) en una sola acción
document.getElementById('savePromptsBtn').onclick = async () => {
    const globalContent = document.getElementById('globalPrompt').value;
    const globalRes = await apiCall('/api/prompts/global', 'PUT', { content: globalContent });
    if (globalRes.status !== 'ok') {
        alert('Error al guardar prompt global: ' + globalRes.message);
        return;
    }

    if (activeProject) {
        const localContent = document.getElementById('localPrompt').value;
        const localRes = await apiCall(`/api/prompts/projects/${activeProject}`, 'PUT', { content: localContent });
        if (localRes.status !== 'ok') {
            alert('Error al guardar prompt local: ' + localRes.message);
            return;
        }
        alert('System prompts global y local guardados para ' + activeProject + '.');
    } else {
        alert('System prompt global guardado.');
    }
};

// resetPromptBtn: restaura el prompt global al valor por defecto
document.getElementById('resetPromptBtn').onclick = async () => {
    if (!confirm('¿Restaurar el system prompt global al valor por defecto?')) return;
    const res = await apiCall('/api/prompts/global/reset', 'POST');
    if (res.status === 'ok') {
        document.getElementById('globalPrompt').value = res.content;
        alert('System prompt global restaurado.');
    } else {
        alert('Error: ' + res.message);
    }
};

// ============================================================================
// STUDY PROFILE — carga desde /api/study/profile y rellena el formulario
// ============================================================================

async function loadStudyProfile() {
    try {
        const res = await apiCall('/api/study/profile');
        if (res.status === 'ok' && res.profile) {
            const p = res.profile;

            // Rellenar campos del formulario de perfil
            const ageEl = document.getElementById('profileAge');
            const gamesEl = document.getElementById('profileGames');
            const hobbiesEl = document.getElementById('profileHobbies');
            const neuroEl = document.getElementById('profileNeuro');
            const phaseEl = document.getElementById('studyPhase');

            if (ageEl && p.age) ageEl.value = p.age;
            if (gamesEl && p.favorite_games) gamesEl.value = p.favorite_games.join(', ');
            if (hobbiesEl && p.hobbies) hobbiesEl.value = p.hobbies.join(', ');
            if (neuroEl && p.neurological_conditions) neuroEl.value = p.neurological_conditions.join(', ');

            // Mostrar info de la fase y resumen de aprendizaje
            if (phaseEl) {
                const phaseMap = { 'NotStarted': 'No iniciado', 'Exploration': '🔍 Exploración', 'Exploitation': '🎯 Explotación' };
                const phaseName = phaseMap[p.phase] || p.phase || 'No definida';
                const summary = p.learning_style_summary || '';
                const engagement = res.engagement ? (' | Engagement: ' + Math.round(res.engagement * 100) + '%') : '';
                phaseEl.innerHTML = '<b>Fase:</b> ' + phaseName + engagement +
                    (summary ? '<br><b>Resumen:</b> ' + summary : '') +
                    '<br><b>Usuario:</b> ' + (p.username || authUsername || '?');
            }

            // Mostrar en el chat que el perfil se cargó correctamente
            console.log('[IAF] Perfil de estudio cargado:', p.username, 'fase:', p.phase);
        } else {
            // Perfil no encontrado, mostrar estado inicial
            const phaseEl = document.getElementById('studyPhase');
            if (phaseEl) phaseEl.innerHTML = '<b>Fase:</b> No iniciado — Completá el perfil y guardalo.';
        }
    } catch(e) {
        console.error('[IAF] Error cargando perfil de estudio:', e);
        const phaseEl = document.getElementById('studyPhase');
        if (phaseEl) phaseEl.innerHTML = '<b>Error:</b> No se pudo cargar el perfil.';
    }
}

// ---- Save Study Profile ----
document.getElementById('saveProfileBtn').onclick = async () => {
    const ageRaw = document.getElementById('profileAge').value.trim();
    const gamesRaw = document.getElementById('profileGames').value.trim();
    const hobbiesRaw = document.getElementById('profileHobbies').value.trim();
    const neuroRaw = document.getElementById('profileNeuro').value.trim();

    const payload = {};
    if (ageRaw) payload.age = parseInt(ageRaw);
    if (gamesRaw) payload.favorite_games = gamesRaw.split(',').map(s => s.trim()).filter(Boolean);
    if (hobbiesRaw) payload.hobbies = hobbiesRaw.split(',').map(s => s.trim()).filter(Boolean);
    if (neuroRaw) payload.neurological_conditions = neuroRaw.split(',').map(s => s.trim()).filter(Boolean);

    try {
        const res = await apiCall('/api/study/profile', 'POST', payload);
        if (res.status === 'ok') {
            showInfoToast('✅ Perfil de estudio guardado correctamente.');
            loadStudyProfile(); // Recargar para mostrar datos actualizados
        } else {
            alert('Error al guardar perfil: ' + (res.message || 'Desconocido'));
        }
    } catch(e) {
        alert('Error de conexión al guardar perfil.');
    }
};

// ============================================================================
// CHAT HISTORY
// ============================================================================

async function loadChatHistory() {
    try {
        const res = await apiCall('/api/chats');
        const list = document.getElementById('chatHistoryList');
        if (res.chats && Array.isArray(res.chats)) {
            list.innerHTML = res.chats.map(c => `
                <div class="project-item ${currentSessionId === c.id ? 'active' : ''}" onclick="selectChatSession('${c.id}')">${c.title}</div>
            `).join('');
        }
    } catch(e) {}
}

// BUG-014 + BUG-015 FIX: Al seleccionar un chat existente (ej: tras recargar la página),
// se inicia el monitoreo del agente y se cargan los steps de auditoría desde la sesión.
async function selectChatSession(id) {
    currentSessionId = id;
    loadChatHistory();
    const res = await apiCall(`/api/chats/${id}`);
    if (res.status === 'ok') {
        const chatArea = document.getElementById('chatArea');
        chatArea.innerHTML = '';
        res.session.messages.forEach(m => addMessage(m.role, m.content));
        if (res.session.project_name) {
            activeProject = res.session.project_name;
            document.getElementById('activeProjectName').innerText = activeProject;
            loadProjects();
        }
        // BUG-015 FIX: Cargar steps de auditoría desde la sesión persistida
        if (res.session.steps && Array.isArray(res.session.steps) && res.session.steps.length > 0) {
            renderConsoleSteps(res.session.steps);
        }
        // BUG-014 FIX: Iniciar monitoreo del agente al entrar a un chat existente
        startAgentMonitoring();
    }
}

// BUG-019 FIX: Al crear un nuevo chat, limpiar TODO el estado anterior
document.getElementById('newChatBtn').onclick = () => {
    currentSessionId = null;
    document.getElementById('chatArea').innerHTML = '<div class="message system-msg"><strong>Sistema:</strong> Nuevo chat iniciado.</div>';
    // BUG-019: Detener monitoreo y limpiar auditoría
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
        prompt: currentText, feedback, session_id: currentSessionId, project_name: activeProject
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
    // Resetear flags de modales para la nueva sesión del agente
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

function addMessage(role, text, extraClass) {
    const div = document.createElement('div');
    var cssClass = 'message ' + role + '-msg';
    if (extraClass) cssClass += ' ' + extraClass;
    div.className = cssClass;
    div.innerHTML = '<strong>' + (role === 'user' ? 'Tú' : 'Agente') + ':</strong> ' + text.replace(/\n/g, '<br>');
    document.getElementById('chatArea').appendChild(div);
    document.getElementById('chatArea').scrollTop = document.getElementById('chatArea').scrollHeight;
}

// ---- Agent Monitoring (Console) ----
// BUG-021 FIX: Mensajes informativos se muestran EN EL CHAT en tiempo real
// (no solo en la auditoría). Polling reducido a 1s para mejor respuesta.
async function startAgentMonitoring() {
    if (agentMonitorInterval) clearInterval(agentMonitorInterval);
    document.getElementById('interruptBtn').classList.remove('hidden');
    let lastInfoMessageCount = 0;
    let lastSessionId = null;

    agentMonitorInterval = setInterval(async () => {
        const statusRes = await apiCall('/api/agent/status');

        // Reiniciar contador solo cuando cambia la sesión
        if (statusRes.current_session_id && statusRes.current_session_id !== lastSessionId) {
            lastSessionId = statusRes.current_session_id;
            lastInfoMessageCount = 0;
        }

        // BUG-021 FIX: Consumir info_messages SIEMPRE y mostrarlos en el CHAT
        // (antes solo aparecían en la consola de auditoría)
        if (statusRes.info_messages && Array.isArray(statusRes.info_messages)) {
            const currentCount = statusRes.info_messages.length;
            if (currentCount > lastInfoMessageCount && currentCount > 0) {
                const newMessages = statusRes.info_messages.slice(lastInfoMessageCount);
                newMessages.forEach(function(msg) {
                    // Toast flotante (notificación visual inmediata)
                    showInfoToast(msg);
                    // Mensaje en el chat con estilo distintivo (fondo azul semitransparente)
                    addMessage('agent', 'ℹ️ ' + msg, 'info-msg');
                });
                lastInfoMessageCount = currentCount;
            }
        }

        // Mostrar mensaje final si el agente terminó
        if (statusRes.finished && statusRes.final_message) {
            addMessage('agent', '✅ ' + statusRes.final_message, 'final-msg');
            showInfoToast('✅ Tarea completada: ' + statusRes.final_message);
            // Limpiar el intervalo después de mostrar el mensaje final
            setTimeout(() => {
                if (agentMonitorInterval) {
                    clearInterval(agentMonitorInterval);
                    agentMonitorInterval = null;
                    document.getElementById('interruptBtn').classList.add('hidden');
                }
            }, 2000);
        }

        // El resto de la lógica solo aplica cuando el agente está activo
        if (statusRes.status === 'ok' && (statusRes.active || statusRes.running)) {
            // Actualizar pasos de auditoría en la consola
            const stepsRes = await apiCall('/api/agent/steps');
            if (stepsRes.status === 'ok' && stepsRes.steps) {
                renderConsoleSteps(stepsRes.steps);
            }

            // Mostrar pregunta del agente al usuario
            if (statusRes.esperando_respuesta_usuario && statusRes.pregunta_usuario && !agentQuestionShown) {
                agentQuestionShown = true;
                document.getElementById('agentQuestionPrompt').textContent = statusRes.pregunta_usuario;
                document.getElementById('agentQuestionResponse').value = '';
                document.getElementById('agentQuestionModal').classList.remove('hidden');
            }

            // Mostrar plan de cambios propuesto
            if (statusRes.esperando_aprobacion_plan && statusRes.plan_propuesto && !agentPlanShown) {
                agentPlanShown = true;
                document.getElementById('agentPlanContent').textContent = statusRes.plan_propuesto;
                document.getElementById('agentPlanModal').classList.remove('hidden');
            }

            // Alerta de CAPTCHA
            if (statusRes.captcha_pending) {
                document.getElementById('captchaAlert').classList.remove('hidden');
            }

            // Si el agente ya no está esperando respuesta, resetear flag
            if (!statusRes.esperando_respuesta_usuario) {
                agentQuestionShown = false;
            }
            if (!statusRes.esperando_aprobacion_plan) {
                agentPlanShown = false;
            }
        } else if (!statusRes.finished) {
            // Agente detenido pero no finalizado (posiblemente interrumpido)
            document.getElementById('interruptBtn').classList.add('hidden');
            agentQuestionShown = false;
            agentPlanShown = false;
        }
    }, 1000); // BUG-021: reducido de 1500ms a 1000ms para mejor tiempo real
}

function renderConsoleSteps(steps) {
    const area = document.getElementById('consoleArea');
    if (!area) return;
    area.innerHTML = steps.map(function(s) {
        return `<div class="console-step">
            <div class="console-step-title">[${s.step_type || ''}] ${s.title || ''}</div>
            <div class="console-step-detail">${s.detail || ''}</div>
        </div>`;
    }).join('');
    area.scrollTop = area.scrollHeight;
}


function showInfoToast(message) {
    var toast = document.createElement('div');
    toast.className = 'info-toast';
    toast.textContent = message;
    toast.style.cssText = 'position:fixed;bottom:20px;right:20px;background:linear-gradient(135deg,#1a1a2e,#16213e);color:#e0e0e0;padding:12px 20px;border-radius:8px;border:1px solid var(--accent,#00d4ff);box-shadow:0 4px 20px rgba(0,0,0,0.5);z-index:10000;max-width:400px;font-size:13px;animation:slideIn 0.3s ease-out;cursor:pointer;';
    toast.onclick = function() { toast.remove(); };
    document.body.appendChild(toast);
    setTimeout(function() {
        if (toast.parentNode) {
            toast.style.opacity = '0';
            toast.style.transition = 'opacity 0.3s';
            setTimeout(function() { if (toast.parentNode) toast.remove(); }, 300);
        }
    }, 8000);
}

document.getElementById('interruptBtn').onclick = async () => {
    await apiCall('/api/agent/interrupt', 'POST');
    document.getElementById('interruptBtn').classList.add('hidden');
    if (agentMonitorInterval) clearInterval(agentMonitorInterval);
};

document.getElementById('summarizeStepsBtn').onclick = async () => {
    const res = await apiCall('/api/agent/summary');
    if (res.status === 'ok') {
        const area = document.getElementById('consoleArea');
        area.innerHTML = `<div class="console-step"><div class="console-step-title">📋 Resumen</div><div class="console-step-detail">${res.summary}</div></div>`;
    }
};

// ---- CAPTCHA ----
document.getElementById('openCaptchaBtn').onclick = async () => {
    const res = await apiCall('/api/captcha/status');
    if (res.status === 'ok' && res.url) {
        document.getElementById('captchaLink').href = res.url;
        document.getElementById('captchaModal').classList.remove('hidden');
    }
};

document.getElementById('submitCaptchaBtn').onclick = async () => {
    const solution = document.getElementById('captchaSolution').value.trim();
    if (!solution) return;
    const res = await apiCall('/api/captcha/solve', 'POST', { id: currentCaptcha, solved_content: solution });
    if (res.status === 'ok') {
        document.getElementById('captchaModal').classList.add('hidden');
        document.getElementById('captchaAlert').classList.add('hidden');
    } else { alert('Error: ' + res.message); }
};

document.getElementById('closeCaptchaBtn').onclick = () => {
    document.getElementById('captchaModal').classList.add('hidden');
};

// ---- Agent Question Modal ----
document.getElementById('submitAgentResponseBtn').onclick = async () => {
    const respuesta = document.getElementById('agentQuestionResponse').value.trim();
    if (!respuesta) return;
    addMessage('user', respuesta);
    await apiCall('/api/agent/responder', 'POST', { respuesta });
    document.getElementById('agentQuestionModal').classList.add('hidden');
    agentQuestionShown = false;
};

// ---- Agent Plan Modal ----
document.getElementById('approvePlanBtn').onclick = async () => {
    await apiCall('/api/agent/aprobar_plan', 'POST', { aprobado: true });
    document.getElementById('agentPlanModal').classList.add('hidden');
    agentPlanShown = false;
};

document.getElementById('rejectPlanBtn').onclick = async () => {
    await apiCall('/api/agent/aprobar_plan', 'POST', { aprobado: false });
    document.getElementById('agentPlanModal').classList.add('hidden');
    agentPlanShown = false;
};

// ---- Helper: Download script ----
function downloadScript(name) {
    var url = '/api/scripts/' + name;
    fetch(url)
        .then(function(r) { return r.blob(); })
        .then(function(blob) {
            var a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = name + '.ps1';
            a.click();
            URL.revokeObjectURL(a.href);
        })
        .catch(function() { alert('Error al descargar el script.'); });
}

// ---- Helper: Download PEM ----
function downloadPem(type, content) {
    var blob = new Blob([content], { type: 'application/x-pem-file' });
    var a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = type === 'private' ? 'admin_private.pem' : 'admin_public.pem';
    a.click();
    URL.revokeObjectURL(a.href);
}

// ---- Toggle Admin Create Mode ----
function toggleAdminCreateMode() {
    var isAdmin = document.getElementById('newIsAdmin').checked;
    document.getElementById('newPassword').parentElement.parentElement.style.display = isAdmin ? 'none' : '';
    document.getElementById('newPasswordConfirm').parentElement.parentElement.style.display = isAdmin ? 'none' : '';
    document.getElementById('newPublicKeyContainer').classList.toggle('hidden', !isAdmin);
    document.getElementById('uploadPemBtn').classList.toggle('hidden', !isAdmin);
    document.getElementById('generateKeysBtn').classList.toggle('hidden', !isAdmin);
    if (isAdmin) {
        document.getElementById('newStudyAccess').checked = true;
        document.getElementById('newProgAccess').checked = true;
        document.getElementById('newEditGlobalPrompt').checked = true;
        document.getElementById('newEditLocalPrompt').checked = true;
        document.getElementById('newCanFork').checked = true;
        document.getElementById('newCanExecPS').checked = true;
        document.getElementById('newCanWrite').checked = true;
        document.getElementById('newCanSearchGoogle').checked = true;
    }
}

// ---- Close Keygen Modal ----
document.getElementById('closeKeygenBtn').onclick = function() {
    document.getElementById('keygenModal').classList.add('hidden');
};

// ---- Init ----
init();
