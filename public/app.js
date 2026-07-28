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
        const createRes = await apiCall('/api/admin/users', 'POST', payload);
        if (createRes.status !== 'ok') return alert('Error creando admin: ' + createRes.message);
        const adminUsername = createRes.user.username;
        const keyRes = await apiCall('/api/auth/keygen', 'GET');
        if (keyRes.status === 'ok' && keyRes.public_key) {
            await apiCall(`/api/admin/users/${adminUsername}/access`, 'PUT', {
                modo_estudio: payload.modo_estudio,
                modo_programador: payload.modo_programador,
                editar_system_prompt_global: payload.editar_system_prompt_global,
                editar_system_prompt_local: payload.editar_system_prompt_local,
            });
            const pwd = document.getElementById('newPassword').value;
            if (pwd) {
                await apiCall(`/api/admin/users/${adminUsername}/password`, 'PUT', { new_password: pwd });
            }
            alert('Admin ' + adminUsername + ' creado.\n\nClave privada:\n' + keyRes.private_key + '\n\nClave publica:\n' + keyRes.public_key + '\n\nGUARDA ESTAS CLAVES. NO SE MOSTRARAN DE NUEVO.');
        } else {
            alert('Admin creado pero no se pudo generar claves. Crea manualmente las claves.');
        }
    } else {
        const pwd = document.getElementById('newPassword').value;
        payload.password = pwd;
        const createRes = await apiCall('/api/admin/users', 'POST', payload);
        if (createRes.status === 'ok') {
            alert('Usuario ' + username + ' creado exitosamente.');
        } else {
            alert('Error: ' + createRes.message);
        }
    }

    await refreshUsersTable();
};

function toggleAdminCreateMode() {
    const isAdmin = document.getElementById('newIsAdmin').checked;
    document.getElementById('uploadPemBtn').classList.toggle('hidden', !isAdmin);
    document.getElementById('generateKeysBtn').classList.toggle('hidden', !isAdmin);
    document.getElementById('newPublicKeyContainer').classList.toggle('hidden', !isAdmin);
}

// ---- Upload PEM file ----
document.getElementById('uploadPemBtn').onclick = () => {
    document.getElementById('pemFileInput').click();
};

document.getElementById('pemFileInput').onchange = async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    const text = await file.text();
    document.getElementById('newPublicKey').value = text.trim();
};

// ---- Generate Keys ----
document.getElementById('generateKeysBtn').onclick = async () => {
    const res = await apiCall('/api/auth/keygen', 'GET');
    if (res.status === 'ok') {
        document.getElementById('newPublicKey').value = res.public_key;
        alert('Claves generadas. Guarda la clave privada:\n\n' + res.private_key + '\n\nESTA ES LA UNICA VEZ QUE LA VERAS.');
    }
};

// ---- Download Scripts ----
function downloadScript(name) {
    const url = '/api/scripts/' + name;
    const a = document.createElement('a');
    a.href = url;
    a.download = name + '.ps1';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
}

// ============================================================================
// STUDY PROFILE — carga y guardado con IDs correctos del HTML
// ============================================================================

async function loadStudyProfile() {
    try {
        const res = await apiCall('/api/study/profile');
        if (res.status !== 'ok') return;

        const profile = res.profile || {};
        document.getElementById('profileAge').value = profile.age || '';
        document.getElementById('profileGames').value = (profile.favorite_games || []).join(', ');
        document.getElementById('profileHobbies').value = (profile.hobbies || []).join(', ');
        document.getElementById('profileNeuro').value = (profile.neurological_conditions || []).join(', ');

        const phaseDiv = document.getElementById('studyPhase');
        if (phaseDiv && res.phase) {
            const phaseNames = { Exploration: '🔍 Exploración', Exploitation: '🎯 Explotación', NotStarted: '🆕 No iniciado' };
            phaseDiv.textContent = 'Fase: ' + (phaseNames[res.phase] || res.phase);
            if (res.engagement !== undefined) {
                phaseDiv.textContent += ' | Engagement: ' + Math.round(res.engagement * 100) + '%';
            }
        }
    } catch(e) {}
}

document.getElementById('saveProfileBtn').onclick = async () => {
    const payload = {
        age: parseInt(document.getElementById('profileAge').value) || null,
        favorite_games: document.getElementById('profileGames').value.split(',').map(s => s.trim()).filter(Boolean),
        hobbies: document.getElementById('profileHobbies').value.split(',').map(s => s.trim()).filter(Boolean),
        neurological_conditions: document.getElementById('profileNeuro').value.split(',').map(s => s.trim()).filter(Boolean),
    };
    const res = await apiCall('/api/study/profile', 'POST', payload);
    if (res.status === 'ok') {
        showInfoToast('✅ Perfil guardado correctamente.');
        loadStudyProfile();
    } else {
        alert('Error: ' + res.message);
    }
};

// ============================================================================
// PROJECTS — carga, fork, agregar local
// ============================================================================

async function loadProjects() {
    const res = await apiCall('/api/projects');
    if (res.status !== 'ok') return;
    const list = document.getElementById('projectList');
    list.innerHTML = res.projects.map(p => `
        <div class="project-item ${activeProject === p.name ? 'active' : ''}" onclick="selectProject('${p.name}')">
            <span>${p.name}</span>
        </div>
    `).join('');
    if (activeProject && !res.projects.find(p => p.name === activeProject)) {
        activeProject = null;
        document.getElementById('activeProjectName').textContent = 'Ninguno (Global)';
    }

// ============================================================================
// PROMPTS — carga, guardado y restauración de system prompts
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
    if (globalRes.status !== 'ok') {
        alert('Error al guardar prompt global: ' + globalRes.message);
        return;
    }

    if (activeProject) {
        const localContent = document.getElementById('localPrompt').value;
        const localRes = await apiCall('/api/prompts/local', 'POST', { project_name: activeProject, content: localContent });
        if (localRes.status !== 'ok') {
            alert('Error al guardar prompt local: ' + localRes.message);
            return;
        }
        showInfoToast('✅ System prompts guardados para ' + activeProject);
    } else {
        showInfoToast('✅ System prompt global guardado.');
    }
};

document.getElementById('resetPromptBtn').onclick = async () => {
    if (!confirm('¿Restaurar el system prompt global al valor por defecto?')) return;
    const res = await apiCall('/api/prompts/global/reset', 'POST');
    if (res.status === 'ok') {
        document.getElementById('globalPrompt').value = res.content;
        showInfoToast('✅ System prompt global restaurado.');
    } else {
        alert('Error: ' + res.message);
    }
};



function selectProject(name) {
    activeProject = name;
    document.getElementById('activeProjectName').textContent = name;
    loadPrompts();
    loadProjects();
}

document.getElementById('forkBtn').onclick = async () => {
    const url = document.getElementById('repoUrl').value.trim();
    if (!url) return alert('Ingresa URL del repo o usuario/repo.');
    const res = await apiCall('/api/projects/fork', 'POST', { repo_url: url });
    if (res.status === 'ok') {
        showInfoToast('✅ Repo clonado: ' + res.project_name);
        loadProjects();
    } else {
        alert('Error: ' + res.message);
    }
};

document.getElementById('addLocalBtn').onclick = async () => {
    const name = document.getElementById('localProjName').value.trim();
    const path = document.getElementById('localProjPath').value.trim();
    if (!name || !path) return alert('Ingresa nombre y ruta.');
    const res = await apiCall('/api/projects/local', 'POST', { name, path });
    if (res.status === 'ok') {
        showInfoToast('✅ Proyecto local agregado.');
        loadProjects();
    } else {
        alert('Error: ' + res.message);
    }
};

// ============================================================================
// CHAT — historial, sesiones, nuevo chat
// ============================================================================

async function loadChatHistory() {
    const res = await apiCall('/api/chats');
    if (res.status !== 'ok') return;
    const list = document.getElementById('chatHistoryList');
    list.innerHTML = res.sessions.map(s => `
        <div class="project-item ${s.id === currentSessionId ? 'active' : ''}" onclick="selectChatSession('${s.id}')">
            <span>${s.title || 'Chat'}</span>
            <span style="font-size:10px;color:var(--text-muted);">${s.messages_count || 0} msgs</span>
        </div>
    `).join('');
}

document.getElementById('newChatBtn').onclick = () => {
    currentSessionId = null;
    document.getElementById('chatArea').innerHTML = '<div class="message system-msg"><strong>Sistema:</strong> Nuevo chat iniciado. Selecciona un proyecto y modo para empezar.</div>';
    loadChatHistory();
};

async function selectChatSession(sessionId) {
    currentSessionId = sessionId;
    const res = await apiCall('/api/chats/' + sessionId);
    if (res.status !== 'ok') return;
    const chatArea = document.getElementById('chatArea');
    chatArea.innerHTML = '';
    (res.messages || []).forEach(m => {
        addMessage(m.role, m.content);
    });
    loadChatHistory();
    // Iniciar monitoreo para sesiones activas
    startAgentMonitoring();
}

// ============================================================================
// CHAT INPUT — enviar mensajes, refinar prompt
// ============================================================================

document.getElementById('sendBtn').onclick = async () => {
    const text = document.getElementById('chatInput').value.trim();
    if (!text) return;
    pendingMessageToSend = text;
    document.getElementById('chatInput').value = '';

    // Refinar prompt
    const refineRes = await apiCall('/api/refine', 'POST', { message: text, project_name: activeProject });
    if (refineRes.status === 'ok' && refineRes.refined) {
        document.getElementById('refinedPromptText').value = refineRes.refined;
        document.getElementById('refinePromptFeedback').value = '';
        document.getElementById('refinePromptModal').classList.remove('hidden');
    } else {
        // No se pudo refinar, enviar directo
        pendingMessageToSend = null;
        await sendMessageToAgent(text, 'programming');
    }
};

document.getElementById('sendDirectBtn').onclick = async () => {
    const text = document.getElementById('chatInput').value.trim();
    if (!text) return;
    document.getElementById('chatInput').value = '';
    const mode = document.getElementById('modeStudy').classList.contains('active') ? 'study' : 'programming';
    await sendMessageToAgent(text, mode);
};

// Enter en el chat input envía (Shift+Enter para nueva línea)
document.getElementById('chatInput').addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        document.getElementById('sendBtn').click();
    }
});

// ---- Refine Prompt Modal ----
document.getElementById('applyRefinedPromptBtn').onclick = async () => {
    const refined = document.getElementById('refinedPromptText').value.trim();
    document.getElementById('refinePromptModal').classList.add('hidden');
    if (refined) {
        pendingMessageToSend = null;
        await sendMessageToAgent(refined, 'programming');
    }
};

document.getElementById('reRefinePromptBtn').onclick = async () => {
    const btn = document.getElementById('reRefinePromptBtn');
    btn.innerText = 'Refinando...'; btn.disabled = true;
    const original = pendingMessageToSend;
    const feedback = document.getElementById('refinePromptFeedback').value.trim();
    const refineRes = await apiCall('/api/refine', 'POST', {
        message: original,
        project_name: activeProject,
        feedback: feedback || undefined
    });
    if (refineRes.status === 'ok' && refineRes.refined) {
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

// ============================================================================
// AGENT MONITORING — polling con manejo de mensajes informativos
// ============================================================================

function startAgentMonitoring() {
    if (agentMonitorInterval) clearInterval(agentMonitorInterval);
    updateConsoleStatus('🔍 Agente iniciando...');

    agentMonitorInterval = setInterval(async () => {
        try {
            const res = await apiCall('/api/agent/status');
            if (res.status !== 'ok') return;

            // Mostrar/ocultar botón de interrumpir según estado
            const interruptBtn = document.getElementById('interruptBtn');
            if (interruptBtn) {
                if (res.running) {
                    interruptBtn.classList.remove('hidden');
                } else if (res.finished || res.interrupted || !res.active) {
                    interruptBtn.classList.add('hidden');
                }
            }

            // Mostrar thinking en la consola de auditoría
            if (res.thinking_content && Array.isArray(res.thinking_content)) {
                const thinking = res.thinking_content.join('\n');
                if (thinking.trim()) {
                    updateConsoleThinking(thinking);
                }
            }

            // Mostrar steps en la consola
            if (res.steps && Array.isArray(res.steps)) {
                renderConsoleSteps(res.steps);
            }

            // Mostrar mensajes informativos en el chat
            if (res.info_messages && Array.isArray(res.info_messages)) {
                // Solo mostrar los mensajes nuevos (no duplicados)
                const shownKey = '_shownInfoMsgs';
                if (!window[shownKey]) window[shownKey] = new Set();
                res.info_messages.forEach(msg => {
                    if (!window[shownKey].has(msg)) {
                        window[shownKey].add(msg);
                        // BUG-002 FIX: Distinguir notificaciones [NOTIF] de respuestas de texto normales
                        if (msg.startsWith('[NOTIF] ')) {
                            addMessage('agent', 'ℹ️ ' + msg.slice(8), 'info-msg');
                        } else {
                            // Respuesta de texto normal del agente (sin clase extra)
                            addMessage('agent', msg);
                        }
                    }
                });
                // Limpiar el set si crece demasiado
                if (window[shownKey].size > 200) window[shownKey] = new Set();
            }

            // Mostrar pregunta del agente (banner inline, ya no es modal)
            if (res.esperando_respuesta_usuario && res.pregunta_usuario && !agentQuestionShown) {
                agentQuestionShown = true;
                showAgentQuestionBanner(res.pregunta_usuario);
            }

            // Mostrar plan del agente (modal)
            if (res.esperando_aprobacion_plan && res.plan_propuesto && !agentPlanShown) {
                agentPlanShown = true;
                showAgentPlanModal(res.plan_propuesto);
            }

            // Agente terminó (finalizar_tarea)
            if (res.finished) {
                clearInterval(agentMonitorInterval);
                agentMonitorInterval = null;
                if (res.final_message) {
                    addMessage('agent', '✅ ' + res.final_message, 'final-msg');
                }
                // Recargar historial
                loadChatHistory();
                updateConsoleStatus('✅ Agente finalizado.');
                // Ocultar botón de interrumpir
                const ib = document.getElementById('interruptBtn');
                if (ib) ib.classList.add('hidden');
            }

            // Interrupción
            if (res.interrupted) {
                updateConsoleStatus('⚠️ Agente interrumpido.');
                const ib = document.getElementById('interruptBtn');
                if (ib) ib.classList.add('hidden');
            }
        } catch(e) {
            console.error('[IAF] Error en monitoreo del agente:', e);
        }
    }, 1000);
}

// ---- Interrupt Button ----
document.getElementById('interruptBtn').onclick = async () => {
    const res = await apiCall('/api/agent/interrupt', 'POST');
    if (res.status === 'ok') {
        showInfoToast('⏸️ Agente interrumpido.');
    }
};

// ---- Agent Answer (responder a pregunta, banner inline) ----
async function responderAgente(respuesta) {
    // Mostrar la respuesta del usuario en el chat
    addMessage('user', respuesta);
    document.getElementById('agentQuestionBanner').classList.add('hidden');
    agentQuestionShown = false;
    await apiCall('/api/agent/responder', 'POST', { respuesta });
}

// ---- Agent Approve Plan ----
async function aprobarPlan(aprobado, feedback) {
    document.getElementById('agentPlanModal').classList.add('hidden');
    agentPlanShown = false;
    await apiCall('/api/agent/aprobar_plan', 'POST', { aprobado, feedback: feedback || '' });
}

// ============================================================================
// UI HELPERS — modales, toasts, consola, mensajes
// ============================================================================

// BUG-025 FIX: El modal agentQuestionModal fue ELIMINADO del HTML y reemplazado
// por el banner inline #agentQuestionBanner. Se renombró la función y se usan
// los IDs correctos: agentQuestionBanner, agentQuestionPrompt, agentQuestionResponse.
function showAgentQuestionBanner(pregunta) {
    const banner = document.getElementById('agentQuestionBanner');
    document.getElementById('agentQuestionPrompt').textContent = pregunta;
    document.getElementById('agentQuestionResponse').value = '';
    banner.classList.remove('hidden');
    document.getElementById('agentQuestionResponse').focus();
}

// BUG-025 FIX: Los IDs agentAnswerSendBtn/agentAnswerInput ya no existen.
// Se usan submitAgentResponseBtn/agentQuestionResponse que están en el HTML.
document.getElementById('submitAgentResponseBtn').onclick = () => {
    const respuesta = document.getElementById('agentQuestionResponse').value.trim();
    if (!respuesta) return;
    responderAgente(respuesta);
};

// BUG-026 FIX: Ctrl+Enter (o Cmd+Enter en Mac) para enviar la respuesta.
// Enter solo inserta nueva línea, permitiendo respuestas multi-línea.
document.getElementById('agentQuestionResponse').addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        document.getElementById('submitAgentResponseBtn').click();
    }
});

// BUG-025 FIX: agentPlanText → agentPlanContent (ID real en HTML).
// agentPlanFeedback no existe en HTML, se eliminó su referencia.
function showAgentPlanModal(plan) {
    const modal = document.getElementById('agentPlanModal');
    document.getElementById('agentPlanContent').textContent = plan;
    modal.classList.remove('hidden');
}

// BUG-025 FIX: agentPlanApproveBtn/agentPlanRejectBtn → approvePlanBtn/rejectPlanBtn (IDs reales en HTML).
// Se eliminó agentPlanFeedback (no existe en el HTML del modal).
document.getElementById('approvePlanBtn').onclick = () => {
    aprobarPlan(true, '');
};

document.getElementById('rejectPlanBtn').onclick = () => {
    aprobarPlan(false, '');
};

function addMessage(role, text, extraClass) {
    const chatArea = document.getElementById('chatArea');
    const div = document.createElement('div');
    div.classList.add('message');
    if (role === 'user') {
        div.classList.add('user-msg');
    } else if (role === 'agent') {
        div.classList.add('agent-msg');
    }
    if (extraClass) div.classList.add(extraClass);

    // Renderizar markdown para mensajes del agente, texto plano para usuario
    if (role === 'agent' && typeof marked !== 'undefined') {
        try {
            div.innerHTML = marked.parse(text);
        } catch(e) {
            div.textContent = text;
        }
    } else {
        div.textContent = text;
    }

    chatArea.appendChild(div);
    chatArea.scrollTop = chatArea.scrollHeight;
}
// Toast informativo
function showInfoToast(msg) {
    const toast = document.getElementById('infoToast');
    if (!toast) {
        // Crear toast si no existe
        const t = document.createElement('div');
        t.id = 'infoToast';
        t.className = 'info-toast';
        t.textContent = msg;
        document.body.appendChild(t);
        setTimeout(() => { t.classList.add('show'); }, 10);
        setTimeout(() => {
            t.classList.remove('show');
            setTimeout(() => { t.remove(); }, 300);
        }, 3000);
        return;
    }
    toast.textContent = msg;
    toast.classList.add('show');
    setTimeout(() => { toast.classList.remove('show'); }, 3000);
}

// ============================================================================
// ============================================================================
// CONSOLE DE AUDITORÍA — renderizado de steps y thinking
// ============================================================================

function renderConsoleSteps(steps) {
    const consoleArea = document.getElementById('consoleArea');
    if (!consoleArea) return;
    if (!steps || steps.length === 0) {
        // Si no hay pasos, dejar el estado actual (no sobreescribir con vacío)
        const emptyEl = consoleArea.querySelector('.console-empty');
        if (emptyEl && window._agentRunning) {
            emptyEl.textContent = '🔍 Agente ejecutándose, esperando primer paso...';
            emptyEl.className = 'console-step';
        }
        return;
    }

    // Evitar re-renderizar si no hay cambios
    const hash = steps.length + '-' + (steps[steps.length - 1]?.title || '');
    if (consoleArea.dataset.stepsHash === hash) return;
    consoleArea.dataset.stepsHash = hash;

    let html = '';
    steps.forEach((step, i) => {
        const icon = step.step_type === 'thinking' ? '🧠' :
                     step.step_type === 'tool_call' ? '🔧' :
                     step.step_type === 'tool_result' ? '📋' :
                     step.step_type === 'info' ? 'ℹ️' : '📌';
        html += `<div class="console-step">
            <span class="console-step-icon">${icon}</span>
            <span class="console-step-title">#${i + 1} ${step.title}</span>
        </div>`;
    });
    consoleArea.innerHTML = html;
    consoleArea.scrollTop = consoleArea.scrollHeight;
}

function updateConsoleThinking(thinking) {
    const consoleArea = document.getElementById('consoleArea');
    if (!consoleArea) return;

    // Buscar si ya hay un div de thinking
    let thinkDiv = consoleArea.querySelector('.console-thinking');
    if (!thinkDiv) {
        thinkDiv = document.createElement('div');
        thinkDiv.className = 'console-thinking';
        consoleArea.appendChild(thinkDiv);
    }
    // Mostrar solo las últimas 500 caracteres para no saturar
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
// INIT
// ============================================================================
init();