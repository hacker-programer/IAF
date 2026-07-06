let activeProject = null;
let currentCaptcha = null;
let currentSessionId = null;
let agentMonitorInterval = null;

// Fetch elements
const projectList = document.getElementById('projectList');
const activeProjectName = document.getElementById('activeProjectName');
const chatArea = document.getElementById('chatArea');
const chatInput = document.getElementById('chatInput');
const sendBtn = document.getElementById('sendBtn');
const repoUrl = document.getElementById('repoUrl');
const forkBtn = document.getElementById('forkBtn');
const globalPrompt = document.getElementById('globalPrompt');
const localPrompt = document.getElementById('localPrompt');
const savePromptsBtn = document.getElementById('savePromptsBtn');
const resetPromptBtn = document.getElementById('resetPromptBtn');

const localProjName = document.getElementById('localProjName');
const localProjPath = document.getElementById('localProjPath');
const addLocalBtn = document.getElementById('addLocalBtn');
const chatHistoryList = document.getElementById('chatHistoryList');
const newChatBtn = document.getElementById('newChatBtn');

const consoleArea = document.getElementById('consoleArea');
const interruptBtn = document.getElementById('interruptBtn');
const summarizeStepsBtn = document.getElementById('summarizeStepsBtn');

const captchaAlert = document.getElementById('captchaAlert');
const openCaptchaBtn = document.getElementById('openCaptchaBtn');
const captchaModal = document.getElementById('captchaModal');
const captchaLink = document.getElementById('captchaLink');
const captchaSolution = document.getElementById('captchaSolution');
const submitCaptchaBtn = document.getElementById('submitCaptchaBtn');
const closeCaptchaBtn = document.getElementById('closeCaptchaBtn');

// API Calls
async function apiCall(endpoint, method = 'GET', body = null) {
    const opts = {
        method,
        headers: { 'Content-Type': 'application/json' }
    };
    if (body) opts.body = JSON.stringify(body);
    const res = await fetch(`http://127.0.0.1:8080${endpoint}`, opts);
    return res.json();
}

// Load Projects
async function loadProjects() {
    const projects = await apiCall('/api/projects');
    projectList.innerHTML = '';
    projects.forEach(p => {
        const div = document.createElement('div');
        div.className = `project-item ${activeProject === p.name ? 'active' : ''}`;
        div.innerText = p.name + (p.is_local ? ' (Local)' : '');
        div.onclick = () => selectProject(p.name);
        projectList.appendChild(div);
    });
}

// Add Local Project
addLocalBtn.onclick = async () => {
    const name = localProjName.value.trim();
    const path = localProjPath.value.trim();
    if (!name || !path) return alert('Por favor ingresa nombre y ruta del proyecto.');
    
    addLocalBtn.disabled = true;
    const res = await apiCall('/api/projects/local', 'POST', { name, path });
    addLocalBtn.disabled = false;
    
    if (res.status === 'ok') {
        alert('Proyecto local agregado correctamente.');
        localProjName.value = '';
        localProjPath.value = '';
        loadProjects();
    } else {
        alert('Error: ' + res.message);
    }
};

// Select Project
async function selectProject(name) {
    activeProject = name;
    activeProjectName.innerText = name;
    loadProjects();
    
    // Load local prompt
    const prompts = await apiCall('/api/prompts');
    localPrompt.value = prompts.projects[name] || '';
}

// Load Prompts
async function loadPrompts() {
    const prompts = await apiCall('/api/prompts');
    globalPrompt.value = prompts.global_current;
    if (activeProject) {
        localPrompt.value = prompts.projects[activeProject] || '';
    }
}

// Save Prompts
savePromptsBtn.onclick = async () => {
    const prompts = await apiCall('/api/prompts');
    prompts.global_current = globalPrompt.value;
    if (activeProject) {
        prompts.projects[activeProject] = localPrompt.value;
    }
    await apiCall('/api/prompts', 'POST', prompts);
    alert('Prompts guardados correctamente.');
};

// Reset Prompt
resetPromptBtn.onclick = async () => {
    const res = await apiCall('/api/prompts/reset', 'POST');
    globalPrompt.value = res.global_current;
    alert('Prompt global restaurado al original.');
};

// Fork & Clone
forkBtn.onclick = async () => {
    const url = repoUrl.value.trim();
    if (!url) return alert('Especifica la URL del repo de GitHub.');
    forkBtn.innerText = 'Forking & Cloning...';
    forkBtn.disabled = true;
    const res = await apiCall('/api/projects/fork', 'POST', { repo_url: url });
    forkBtn.innerText = 'Fork & Clone';
    forkBtn.disabled = false;
    if (res.status === 'ok') {
        alert('Proyecto clonado correctamente.');
        loadProjects();
    } else {
        alert('Error: ' + res.message);
    }
};

// Load Chat History
async function loadChatHistory() {
    const chats = await apiCall('/api/chats');
    chatHistoryList.innerHTML = '';
    chats.forEach(c => {
        const div = document.createElement('div');
        div.className = `project-item ${currentSessionId === c.id ? 'active' : ''}`;
        div.innerText = c.title || `Chat ${c.id.substring(0, 8)}`;
        div.onclick = () => selectChatSession(c.id);
        chatHistoryList.appendChild(div);
    });
}

// Select Chat Session
async function selectChatSession(sessionId) {
    currentSessionId = sessionId;
    loadChatHistory();
    const res = await apiCall(`/api/chats/${sessionId}`);
    if (res.status === 'ok') {
        chatArea.innerHTML = '';
        if (res.session.project_name) {
            selectProject(res.session.project_name);
        }
        res.session.messages.forEach(m => {
            addMessage(m.role, m.content);
        });

        // Cargar y mostrar los pasos históricos de auditoría que hizo el agente en este chat
        consoleArea.innerHTML = '';
        if (res.session.steps && res.session.steps.length > 0) {
            res.session.steps.forEach(step => {
                const div = document.createElement('div');
                div.className = `console-step ${step.step_type}`;
                div.innerHTML = `
                    <div class="console-step-title">${step.title}</div>
                    <div class="console-step-detail">${step.detail}</div>
                `;
                consoleArea.appendChild(div);
            });
            consoleArea.scrollTop = consoleArea.scrollHeight;
        } else {
            consoleArea.innerHTML = '<div class="console-empty">No hay registro de auditoría previo en este chat.</div>';
        }
    }
}

// New Chat
newChatBtn.onclick = () => {
    currentSessionId = null;
    chatArea.innerHTML = `
        <div class="message system-msg">
            <strong>Sistema:</strong> Nuevo chat iniciado. ¡Pregúntale al agente!
        </div>
    `;
    loadChatHistory();
};

// Summarize Steps
summarizeStepsBtn.onclick = async () => {
    if (!currentSessionId) {
        return alert("Debes seleccionar una sesión de chat activa con historial de pasos.");
    }
    summarizeStepsBtn.disabled = true;
    summarizeStepsBtn.innerText = "Resumiendo...";
    
    try {
        const res = await apiCall(`/api/chats/${currentSessionId}/summarize_steps`, 'POST');
        if (res.status === 'ok') {
            alert("Pasos resumidos exitosamente.");
            selectChatSession(currentSessionId);
        } else {
            alert("Error: " + res.message);
        }
    } catch (e) {
        alert("Error de red: " + e.message);
    } finally {
        summarizeStepsBtn.disabled = false;
        summarizeStepsBtn.innerText = "Resumir Pasos";
    }
};

// Elementos de Modales de Control
const refinePromptModal = document.getElementById('refinePromptModal');
const refinedPromptText = document.getElementById('refinedPromptText');
const refinePromptFeedback = document.getElementById('refinePromptFeedback');
const applyRefinedPromptBtn = document.getElementById('applyRefinedPromptBtn');
const reRefinePromptBtn = document.getElementById('reRefinePromptBtn');
const cancelRefinedPromptBtn = document.getElementById('cancelRefinedPromptBtn');

const agentQuestionModal = document.getElementById('agentQuestionModal');
const agentQuestionPrompt = document.getElementById('agentQuestionPrompt');
const agentQuestionResponse = document.getElementById('agentQuestionResponse');
const submitAgentResponseBtn = document.getElementById('submitAgentResponseBtn');

const agentPlanModal = document.getElementById('agentPlanModal');
const agentPlanContent = document.getElementById('agentPlanContent');
const approvePlanBtn = document.getElementById('approvePlanBtn');
const rejectPlanBtn = document.getElementById('rejectPlanBtn');

let pendingMessageToSend = null;

// Send Message to Agent - Interceptado para Refinamiento
sendBtn.onclick = async () => {
    const text = chatInput.value.trim();
    if (!text) return;
    
    sendBtn.disabled = true;
    if (refinePromptFeedback) refinePromptFeedback.value = '';
    
    // Llamar al endpoint de refinamiento
    try {
        const refineRes = await apiCall('/api/prompts/refine', 'POST', { 
            prompt: text,
            session_id: currentSessionId,
            project_name: activeProject
        });
        if (refineRes.status === 'ok') {
            pendingMessageToSend = text;
            refinedPromptText.value = refineRes.refined;
            refinePromptModal.classList.remove('hidden');
        } else {
            // Si falla el refinador, enviar mensaje original
            chatInput.value = '';
            await sendMessageToAgent(text);
        }
    } catch(e) {
        chatInput.value = '';
        await sendMessageToAgent(text);
    } finally {
        sendBtn.disabled = false;
    }
};
// Debounce helper
let sendDebounceTimer = null;
const SEND_DEBOUNCE_MS = 500;

// Send Message to Agent - Interceptado para Refinamiento (con debounce)
sendBtn.onclick = async () => {
    if (sendDebounceTimer) {
        clearTimeout(sendDebounceTimer);
    }
    sendDebounceTimer = setTimeout(async () => {
        sendDebounceTimer = null;
        const text = chatInput.value.trim();
        if (!text) return;
        
        sendBtn.disabled = true;
        if (refinePromptFeedback) refinePromptFeedback.value = '';
        
        // Llamar al endpoint de refinamiento
        try {
            const refineRes = await apiCall('/api/prompts/refine', 'POST', { 
                prompt: text,
                session_id: currentSessionId,
                project_name: activeProject
            });
            if (refineRes.status === 'ok') {
                pendingMessageToSend = text;
                refinedPromptText.value = refineRes.refined;
                refinePromptModal.classList.remove('hidden');
            } else {
                // Si falla el refinador, enviar mensaje original
                chatInput.value = '';
                await sendMessageToAgent(text);
            }
        } catch(e) {
            chatInput.value = '';
            await sendMessageToAgent(text);
        } finally {
            sendBtn.disabled = false;
        }
    }, SEND_DEBOUNCE_MS);
};
        });
        if (refineRes.status === 'ok') {
            refinedPromptText.value = refineRes.refined;
            if (refinePromptFeedback) refinePromptFeedback.value = '';
        } else {
            alert('Error en el re-refinamiento: ' + refineRes.message);
        }
    } catch(e) {
        alert('Error en el re-refinamiento de red.');
    }
    
    reRefinePromptBtn.innerText = 'Re-Refinar con Instrucción';
    reRefinePromptBtn.disabled = false;
};

cancelRefinedPromptBtn.onclick = async () => {
    refinePromptModal.classList.add('hidden');
    if (pendingMessageToSend) {
        chatInput.value = '';
        await sendMessageToAgent(pendingMessageToSend);
    }
};

async function sendMessageToAgent(text) {
    addMessage('user', text);
    sendBtn.disabled = true;
    
    const res = await apiCall('/api/chat', 'POST', {
        message: text,
        project_name: activeProject,
        session_id: currentSessionId
    });
    
    sendBtn.disabled = false;
    
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
    chatArea.appendChild(div);
    chatArea.scrollTop = chatArea.scrollHeight;
}

// Agent Auditor / Monitor
function startAgentMonitoring() {
    interruptBtn.classList.remove('hidden');
    consoleArea.innerHTML = '';
    
    if (agentMonitorInterval) clearInterval(agentMonitorInterval);
    
    agentMonitorInterval = setInterval(async () => {
        const status = await apiCall('/api/agent/status');
        
        // Render step by step
        consoleArea.innerHTML = '';
        if (status.steps.length === 0) {
            consoleArea.innerHTML = '<div class="console-empty">El agente se está preparando...</div>';
        }
        
        status.steps.forEach(step => {
            const div = document.createElement('div');
            div.className = `console-step ${step.step_type}`;
            div.innerHTML = `
                <div class="console-step-title">${step.title}</div>
                <div class="console-step-detail">${step.detail}</div>
            `;
            consoleArea.appendChild(div);
        });
        consoleArea.scrollTop = consoleArea.scrollHeight;
        
        // Cargar mensajes en tiempo real si hay nuevos
        if (currentSessionId) {
            try {
                const chatRes = await apiCall(`/api/chats/${currentSessionId}`);
                if (chatRes.status === 'ok') {
                    const currentMessageCount = chatArea.querySelectorAll('.message').length;
                    if (chatRes.session.messages.length > currentMessageCount) {
                        chatArea.innerHTML = '';
                        chatRes.session.messages.forEach(m => {
                            addMessage(m.role, m.content);
                        });
                    }
                }
            } catch (e) {
                console.error("Error al recargar mensajes en tiempo real:", e);
            }
        }
        
        // Manejar Preguntas Pausadas
        if (status.esperando_respuesta_usuario) {
            agentQuestionPrompt.innerText = status.pregunta_usuario;
            agentQuestionModal.classList.remove('hidden');
        } else {
            agentQuestionModal.classList.add('hidden');
        }

        // Manejar Plan de Cambios Pausado
        if (status.esperando_aprobacion_plan) {
            agentPlanContent.innerText = status.plan_propuesto;
            agentPlanModal.classList.remove('hidden');
        } else {
            agentPlanModal.classList.add('hidden');
        }
        
        if (!status.running) {
            clearInterval(agentMonitorInterval);
            interruptBtn.classList.add('hidden');
            // Recargar el chat para ver la respuesta final escrita
            if (currentSessionId) {
                selectChatSession(currentSessionId);
            }
        }
    }, 1000);
}

// Responder Pregunta
submitAgentResponseBtn.onclick = async () => {
    const resp = agentQuestionResponse.value.trim();
    if (!resp) return alert('Por favor ingresa una respuesta para el agente.');
    
    submitAgentResponseBtn.disabled = true;
    const res = await apiCall('/api/agent/responder', 'POST', { respuesta: resp });
    submitAgentResponseBtn.disabled = false;
    
    if (res.status === 'ok') {
        agentQuestionResponse.value = '';
        agentQuestionModal.classList.add('hidden');
    }
};

// Aprobar Plan
approvePlanBtn.onclick = async () => {
    approvePlanBtn.disabled = true;
    await apiCall('/api/agent/aprobar_plan', 'POST', { aprobado: true });
    approvePlanBtn.disabled = false;
    agentPlanModal.classList.add('hidden');
};

rejectPlanBtn.onclick = async () => {
    rejectPlanBtn.disabled = true;
    await apiCall('/api/agent/aprobar_plan', 'POST', { aprobado: false });
    rejectPlanBtn.disabled = false;
    agentPlanModal.classList.add('hidden');
};

// Interrupt Agent
interruptBtn.onclick = async () => {
    const res = await apiCall('/api/agent/interrupt', 'POST');
    alert(res.message || 'Se envió señal de interrupción.');
};

// CAPTCHA Polling
setInterval(async () => {
    const captcha = await apiCall('/api/captcha/status');
    if (captcha) {
        currentCaptcha = captcha;
        captchaAlert.classList.remove('hidden');
        captchaLink.href = captcha.url;
    } else {
        captchaAlert.classList.add('hidden');
    }
}, 3000);

openCaptchaBtn.onclick = () => {
    captchaModal.classList.remove('hidden');
};

submitCaptchaBtn.onclick = async () => {
    const sol = captchaSolution.value.trim();
    if (!sol) return alert('Por favor pega el contenido resuelto del CAPTCHA.');
    const res = await apiCall('/api/captcha/solve', 'POST', {
        id: currentCaptcha.id,
        solved_content: sol
    });
    if (res.status === 'ok') {
        captchaModal.classList.add('hidden');
        captchaAlert.classList.add('hidden');
        captchaSolution.value = '';
    } else {
        alert('Error al enviar la solución.');
    }
};

closeCaptchaBtn.onclick = () => {
    captchaModal.classList.add('hidden');
};

// Init
loadProjects();
loadPrompts();
loadChatHistory();

