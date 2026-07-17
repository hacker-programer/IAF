// ============================================================================
// IAF — app.js — Cliente Web con Autenticación
// ============================================================================

// ---- State ----
let activeProject = null;
let currentSessionId = null;
let agentMonitorInterval = null;
let currentCaptcha = null;
let pendingMessageToSend = null;

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
        const data = await res.json();
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
    return res.json();
}

// ---- Mode Toggle ----
document.getElementById('modeProgramming').onclick = () => switchMode('programming');
document.getElementById('modeStudy').onclick = () => switchMode('study');

function switchMode(mode) {
    document.querySelectorAll('.mode-btn').forEach(b => b.classList.remove('active'));
    document.getElementById(mode === 'study' ? 'modeStudy' : 'modeProgramming').classList.add('active');
    document.getElementById('activeMode').textContent = mode === 'study' ? '📚 Estudiar' : '💻 Programar';
    studyProfileSection.classList.toggle('hidden', mode !== 'study');
    if (mode === 'study') loadStudyProfile();
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

async function editUser(username) {
    const res = await apiCall('/api/admin/users');
    const user = res.users.find(u => u.username === username);
    if (!user) return;

    document.getElementById('editUsername').textContent = username;
    document.getElementById('editPassword').value = '';
    document.getElementById('editMaxTokens').value = user.limits.max_tokens_per_day ?? 'null';
    document.getElementById('editMaxApiCalls').value = user.limits.max_api_calls_per_day ?? 'null';
    document.getElementById('editMaxSubAgents').value = user.limits.max_sub_agents;
    document.getElementById('editCanFork').checked = user.limits.can_fork_repos;
    document.getElementById('editCanExecPS').checked = user.limits.can_execute_powershell;
    document.getElementById('editCanWrite').checked = user.limits.can_write_files;
    document.getElementById('editStudyAccess').checked = user.has_study_access;
    document.getElementById('editProgAccess').checked = user.has_programming_access;

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

    const limits = {
        max_tokens_per_day: maxTokensRaw === 'null' || maxTokensRaw === '' ? null : parseInt(maxTokensRaw),
        max_api_calls_per_day: maxApiCallsRaw === 'null' || maxApiCallsRaw === '' ? null : parseInt(maxApiCallsRaw),
        max_sub_agents: parseInt(document.getElementById('editMaxSubAgents').value) || 1,
        max_projects: 2,
        allowed_tools: ['read_file', 'search_code', 'search_google'],
        can_fork_repos: document.getElementById('editCanFork').checked,
        can_execute_powershell: document.getElementById('editCanExecPS').checked,
        can_write_files: document.getElementById('editCanWrite').checked,
    };

    // Update limits
    await apiCall(`/api/admin/users/${username}/limits`, 'PUT', { limits });

    // Update access
    await apiCall(`/api/admin/users/${username}/access`, 'PUT', {
        study_access: document.getElementById('editStudyAccess').checked,
        programming_access: document.getElementById('editProgAccess').checked,
    });

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

// Create user
document.getElementById('createUserBtn').onclick = async () => {
    const username = document.getElementById('newUsername').value.trim();
    const password = document.getElementById('newPassword').value;
    if (!username || !password) return alert('Username y contraseña requeridos.');
    if (password.length < 8) return alert('La contraseña debe tener al menos 8 caracteres.');

    const res = await apiCall('/api/admin/users', 'POST', {
        username,
        password,
        is_admin: document.getElementById('newIsAdmin').checked,
        study_access: document.getElementById('newStudyAccess').checked,
        programming_access: document.getElementById('newProgAccess').checked,
        permissions: ['read_file', 'search_code'],
    });

    if (res.status === 'ok') {
        document.getElementById('newUsername').value = '';
        document.getElementById('newPassword').value = '';
        await refreshUsersTable();
    } else {
        alert('Error: ' + res.message);
    }
};

// ---- Study Profile ----
async function loadStudyProfile() {
    try {
        const res = await apiCall('/api/study/profile');
        if (res.status === 'ok') {
            const p = res.profile;
            document.getElementById('profileAge').value = p.age || '';
            document.getElementById('profileGames').value = (p.favorite_games || []).join(', ');
            document.getElementById('profileHobbies').value = (p.hobbies || []).join(', ');
            document.getElementById('profileNeuro').value = (p.neurological_conditions || []).join(', ');
            document.getElementById('studyPhase').textContent = 'Fase: ' + (p.phase || 'Exploration') + ' | Engagement: ' + ((res.engagement || 0) * 100).toFixed(0) + '%';
        }
    } catch(e) {}
}

document.getElementById('saveProfileBtn').onclick = async () => {
    const profile = {
        age: parseInt(document.getElementById('profileAge').value) || null,
        favorite_games: document.getElementById('profileGames').value.split(',').map(s => s.trim()).filter(Boolean),
        hobbies: document.getElementById('profileHobbies').value.split(',').map(s => s.trim()).filter(Boolean),
        neurological_conditions: document.getElementById('profileNeuro').value.split(',').map(s => s.trim()).filter(Boolean),
    };
    await apiCall('/api/study/profile', 'POST', profile);
    alert('Perfil guardado.');
};

// ---- Projects ----
async function loadProjects() {
    const projects = await apiCall('/api/projects');
    const list = document.getElementById('projectList');
    list.innerHTML = projects.map(p => `
        <div class="project-item ${activeProject === p.name ? 'active' : ''}" onclick="selectProject('${p.name}')">${p.name}</div>
    `).join('');
}

async function selectProject(name) {
    activeProject = name;
    document.getElementById('activeProjectName').innerText = name;
    loadProjects();
    const prompts = await apiCall('/api/prompts');
    document.getElementById('localPrompt').value = prompts.projects[name] || '';
}

// ---- Prompts ----
async function loadPrompts() {
    const prompts = await apiCall('/api/prompts');
    document.getElementById('globalPrompt').value = prompts.global_current;
    if (activeProject) document.getElementById('localPrompt').value = prompts.projects[activeProject] || '';
}

document.getElementById('savePromptsBtn').onclick = async () => {
    const global = document.getElementById('globalPrompt').value;
    const payload = { global };
    if (activeProject) payload.project_prompts = { [activeProject]: document.getElementById('localPrompt').value };
    await apiCall('/api/prompts', 'POST', payload);
    alert('Prompts guardados.');
};

document.getElementById('resetPromptBtn').onclick = async () => {
    await apiCall('/api/prompts/reset', 'POST');
    loadPrompts();
    alert('Prompt global restaurado.');
};

// ---- Fork & Clone ----
document.getElementById('forkBtn').onclick = async () => {
    const url = document.getElementById('repoUrl').value.trim();
    if (!url) return alert('Especifica la URL del repo.');
    const btn = document.getElementById('forkBtn');
    btn.innerText = 'Forking...'; btn.disabled = true;
    const res = await apiCall('/api/projects/fork', 'POST', { repo_url: url });
    btn.innerText = 'Fork & Clone'; btn.disabled = false;
    if (res.status === 'ok') { alert('Clonado correctamente.'); loadProjects(); }
    else alert('Error: ' + res.message);
};

document.getElementById('addLocalBtn').onclick = async () => {
    const name = document.getElementById('localProjName').value.trim();
    const path = document.getElementById('localProjPath').value.trim();
    if (!name || !path) return alert('Nombre y ruta requeridos.');
    const res = await apiCall('/api/projects/local', 'POST', { name, path });
    if (res.status === 'ok') { loadProjects(); }
    else alert('Error: ' + res.message);
};

// ---- Chat History ----
async function loadChatHistory() {
    try {
        const chats = await apiCall('/api/chats');
        const list = document.getElementById('chatHistoryList');
        list.innerHTML = chats.map(c => `
            <div class="project-item ${currentSessionId === c.id ? 'active' : ''}" onclick="selectChatSession('${c.id}')">${c.title}</div>
        `).join('');
    } catch(e) {}
}

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
    }
}

document.getElementById('newChatBtn').onclick = () => {
    currentSessionId = null;
    document.getElementById('chatArea').innerHTML = '<div class="message system-msg"><strong>Sistema:</strong> Nuevo chat iniciado.</div>';
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

function addMessage(role, text) {
    const div = document.createElement('div');
    div.className = `message ${role}-msg`;
    div.innerHTML = `<strong>${role === 'user' ? 'Tú' : 'Agente'}:</strong> ${text.replace(/\n/g, '<br>')}`;
    document.getElementById('chatArea').appendChild(div);
    document.getElementById('chatArea').scrollTop = document.getElementById('chatArea').scrollHeight;
}

// ---- Agent Monitor ----
function startAgentMonitoring() {
    document.getElementById('interruptBtn').classList.remove('hidden');
    document.getElementById('consoleArea').innerHTML = '';
    if (agentMonitorInterval) clearInterval(agentMonitorInterval);

    agentMonitorInterval = setInterval(async () => {
        const status = await apiCall('/api/agent/status');
        const consoleArea = document.getElementById('consoleArea');
        consoleArea.innerHTML = '';
        if (!status.steps || status.steps.length === 0) {
            consoleArea.innerHTML = '<div class="console-empty">El agente se está preparando...</div>';
        }
        (status.steps || []).forEach(step => {
            const div = document.createElement('div');
            div.className = `console-step ${step.step_type}`;
            div.innerHTML = `<div class="console-step-title">${step.title}</div><div class="console-step-detail">${step.detail}</div>`;
            consoleArea.appendChild(div);
        });
        consoleArea.scrollTop = consoleArea.scrollHeight;

        if (currentSessionId) {
            try {
                const chatRes = await apiCall(`/api/chats/${currentSessionId}`);
                if (chatRes.status === 'ok') {
                    const msgCount = document.getElementById('chatArea').querySelectorAll('.message').length;
                    if (chatRes.session.messages.length > msgCount) {
                        document.getElementById('chatArea').innerHTML = '';
                        chatRes.session.messages.forEach(m => addMessage(m.role, m.content));
                    }
                }
            } catch(e) {}
        }

        if (status.esperando_respuesta_usuario) {
            document.getElementById('agentQuestionPrompt').innerText = status.pregunta_usuario;
            document.getElementById('agentQuestionModal').classList.remove('hidden');
        } else {
            document.getElementById('agentQuestionModal').classList.add('hidden');
        }

        if (status.esperando_aprobacion_plan) {
            document.getElementById('agentPlanContent').innerText = status.plan_propuesto;
            document.getElementById('agentPlanModal').classList.remove('hidden');
        } else {
            document.getElementById('agentPlanModal').classList.add('hidden');
        }

        if (!status.running) {
            clearInterval(agentMonitorInterval);
            document.getElementById('interruptBtn').classList.add('hidden');
            if (currentSessionId) selectChatSession(currentSessionId);
        }
    }, 1000);
}

document.getElementById('submitAgentResponseBtn').onclick = async () => {
    const resp = document.getElementById('agentQuestionResponse').value.trim();
    if (!resp) return alert('Ingresa una respuesta.');
    await apiCall('/api/agent/responder', 'POST', { respuesta: resp });
    document.getElementById('agentQuestionResponse').value = '';
    document.getElementById('agentQuestionModal').classList.add('hidden');
};

document.getElementById('approvePlanBtn').onclick = async () => {
    await apiCall('/api/agent/aprobar_plan', 'POST', { aprobado: true });
    document.getElementById('agentPlanModal').classList.add('hidden');
};

document.getElementById('rejectPlanBtn').onclick = async () => {
    await apiCall('/api/agent/aprobar_plan', 'POST', { aprobado: false });
    document.getElementById('agentPlanModal').classList.add('hidden');
};

document.getElementById('interruptBtn').onclick = async () => {
    await apiCall('/api/agent/interrupt', 'POST');
};

// ---- CAPTCHA Polling ----
setInterval(async () => {
    try {
        const captcha = await apiCall('/api/captcha/status');
        if (captcha && captcha.url) {
            currentCaptcha = captcha;
            document.getElementById('captchaAlert').classList.remove('hidden');
            document.getElementById('captchaLink').href = captcha.url;
        } else {
            document.getElementById('captchaAlert').classList.add('hidden');
        }
    } catch(e) {}
}, 3000);

document.getElementById('openCaptchaBtn').onclick = () => document.getElementById('captchaModal').classList.remove('hidden');
document.getElementById('closeCaptchaBtn').onclick = () => document.getElementById('captchaModal').classList.add('hidden');
document.getElementById('submitCaptchaBtn').onclick = async () => {
    const sol = document.getElementById('captchaSolution').value.trim();
    if (!sol) return;
    await apiCall('/api/captcha/solve', 'POST', { id: currentCaptcha.id, solved_content: sol });
    document.getElementById('captchaModal').classList.add('hidden');
    document.getElementById('captchaAlert').classList.add('hidden');
};

// ---- Start ----
init();
