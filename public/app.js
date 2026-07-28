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
        if (createRes.status === 'ok') {
            document.getElementById('newUsername').value = '';
            document.getElementById('newIsAdmin').checked = false;
            showInfoToast('✅ Admin creado exitosamente.');
            await refreshUsersTable();
        } else {
            alert('Error: ' + (createRes.message || 'Error al crear admin.'));
        }
    } else {
        const pwd = document.getElementById('newPassword').value;
        if (!pwd) return alert('Password requerido para usuarios normales.');
        payload.password = pwd;
        const createRes = await apiCall('/api/admin/users', 'POST', payload);
        if (createRes.status === 'ok') {
            document.getElementById('newUsername').value = '';
            document.getElementById('newPassword').value = '';
            document.getElementById('newPasswordConfirm').value = '';
            showInfoToast('✅ Usuario creado exitosamente.');
            await refreshUsersTable();
        } else {
            alert('Error: ' + (createRes.message || 'Error al crear usuario.'));
        }
    }
};

// ============================================================================
// PROYECTOS
// ============================================================================

async function loadProjects() {
    try {
        const res = await apiCall('/api/projects');
        const list = document.getElementById('projectList');
        if (!res.projects || !Array.isArray(res.projects)) return;
        const projects = res.projects;
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

function selectProject(name) {
    activeProject = name;
    document.getElementById('activeProjectName').innerText = name;
    loadProjects();
    loadPrompts(); // BUG-024 FIX: Reload prompts when switching projects so local prompt updates
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
// ============================================================================
// PROMPTS — alineado con IDs del HTML: globalPrompt, localPrompt,
//           savePromptsBtn (guarda ambos), resetPromptBtn (restaura global)
// ============================================================================

async function loadPrompts() {
    try {
        const prompts = await apiCall('/api/prompts');
        if (prompts.status === 'ok') {
            // BUG-024 FIX: Backend returns global_current, not global
            document.getElementById('globalPrompt').value = prompts.global_current || '';
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
    // FIX: El backend solo acepta POST en /api/prompts/global (no PUT)
    const globalRes = await apiCall('/api/prompts/global', 'POST', { content: globalContent });
    if (globalRes.status !== 'ok') {
        alert('Error al guardar prompt global: ' + globalRes.message);
        return;
    }

    if (activeProject) {
        const localContent = document.getElementById('localPrompt').value;
        // FIX: /api/prompts/local usa POST y recibe { project_name, content } en el body (no PUT a /api/prompts/projects/...)
        const localRes = await apiCall('/api/prompts/local', 'POST', { project_name: activeProject, content: localContent });
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
                        addMessage('agent', 'ℹ️ ' + msg, 'info-msg');
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

// Enter en el input de respuesta del banner inline
document.getElementById('agentQuestionResponse').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
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