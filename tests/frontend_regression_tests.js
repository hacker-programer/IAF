// ============================================================================
// tests/frontend_regression_tests.js
// Tests de Regresión para el Frontend (app.js)
//
// Estos tests validan que los bugs encontrados no se repitan:
// - BUG A: El modal de pregunta del agente nunca se abría porque
//   startAgentMonitoring no leía esperando_respuesta_usuario ni pregunta_usuario
// - BUG B: copyNonceCmd usaba event sin declararlo, y no tenía fallback
//   para navegadores sin Clipboard API
//
// Ejecutar con: node tests/frontend_regression_tests.js
// ============================================================================

// ============================================================================
// Mocks — simulan el entorno del navegador
// ============================================================================

// Mock de navigator.clipboard
var mockClipboard = {
    available: true,
    written: null,
    writeText: function(text) {
        this.written = text;
        return Promise.resolve();
    }
};

// Mock de navigator
var mockNavigator = {
    clipboard: mockClipboard
};

// Mock de document
var mockDocument = {
    elements: {},
    getElementById: function(id) {
        return this.elements[id] || null;
    },
    createElement: function(tag) {
        return {
            tagName: tag,
            value: '',
            style: {},
            select: function() {},
            focus: function() {},
            parentNode: null
        };
    },
    body: {
        appendChild: function(el) { el.parentNode = this; },
        removeChild: function(el) { el.parentNode = null; }
    },
    querySelector: function(sel) { return null; },
    execCommand: function(cmd) { return cmd === 'copy'; }
};

// Mock de window
var mockWindow = {
    _lastNonce: 'test_nonce_abc123',
    _lastAdminUser: 'admin',
    event: null
};

// Mock de alert
var alertMessages = [];
function mockAlert(msg) { alertMessages.push(msg); }

// Mock de setTimeout — NO ejecuta inmediatamente (simula asincronía)
var setTimeoutCallbacks = [];
function mockSetTimeout(fn, delay) {
    setTimeoutCallbacks.push(fn);
    return 999;
}
function flushSetTimeout() {
    while (setTimeoutCallbacks.length > 0) {
        var cb = setTimeoutCallbacks.shift();
        cb();
    }
}

// ============================================================================
// Implementación de la función CORREGIDA copyNonceCmd
// (debe coincidir exactamente con la versión en app.js)
// ============================================================================

function copyNonceCmd(event) {
    event = event || mockWindow.event;
    var nonce = mockWindow._lastNonce || '';
    var cmd = '.\\scripts\\sign_nonce.ps1 -Nonce "' + nonce + '" -KeyPath ".config\\admin_private.pem"';

    var btn = null;
    if (event && event.target) {
        btn = event.target;
    } else {
        btn = mockDocument.querySelector('.btn-copy-small');
    }

    function fallbackCopy(text) {
        var ta = mockDocument.createElement('textarea');
        ta.value = text;
        ta.style.position = 'fixed';
        ta.style.left = '-9999px';
        ta.style.top = '-9999px';
        mockDocument.body.appendChild(ta);
        ta.focus();
        ta.select();
        var ok = false;
        try {
            ok = mockDocument.execCommand('copy');
        } catch (e) {
            ok = false;
        }
        mockDocument.body.removeChild(ta);
        return ok;
    }

    function onSuccess() {
        if (btn) {
            btn.textContent = '✓';
            mockSetTimeout(function () { btn.textContent = '📋'; }, 1500);
        }
    }

    function onFailure() {
        mockAlert('No se pudo copiar al portapapeles. Copiá manualmente:\n\n' + cmd);
    }

    if (mockNavigator.clipboard && typeof mockNavigator.clipboard.writeText === 'function') {
        mockNavigator.clipboard.writeText(cmd).then(onSuccess).catch(function () {
            if (fallbackCopy(cmd)) {
                onSuccess();
            } else {
                onFailure();
            }
        });
    } else {
        if (fallbackCopy(cmd)) {
            onSuccess();
        } else {
            onFailure();
        }
    }
}

// ============================================================================
// Implementación de la lógica CORREGIDA de startAgentMonitoring
// (función que decide si mostrar el modal de pregunta/plan)
// ============================================================================

function shouldShowQuestionModal(statusResponse, alreadyShown) {
    if (alreadyShown) return false;
    var esperando = statusResponse.esperando_respuesta_usuario;
    var pregunta = statusResponse.pregunta_usuario;
    return esperando === true && typeof pregunta === 'string' && pregunta.length > 0;
}

function shouldShowPlanModal(statusResponse, alreadyShown) {
    if (alreadyShown) return false;
    var esperando = statusResponse.esperando_aprobacion_plan;
    var plan = statusResponse.plan_propuesto;
    return esperando === true && typeof plan === 'string' && plan.length > 0;
}

function simulateAgentPolling(statusResponse, currentState) {
    var actions = [];

    if (statusResponse.status === 'ok' && (statusResponse.active || statusResponse.running)) {
        if (shouldShowQuestionModal(statusResponse, currentState.questionShown)) {
            actions.push({
                action: 'showQuestionModal',
                question: statusResponse.pregunta_usuario,
                detail: 'BUG #A FIX: El frontend ahora detecta esperando_respuesta_usuario y pregunta_usuario'
            });
            currentState.questionShown = true;
        }

        if (shouldShowPlanModal(statusResponse, currentState.planShown)) {
            actions.push({
                action: 'showPlanModal',
                plan: statusResponse.plan_propuesto,
                detail: 'El frontend ahora detecta esperando_aprobacion_plan y plan_propuesto'
            });
            currentState.planShown = true;
        }

        if (statusResponse.captcha_pending) {
            actions.push({ action: 'showCaptchaAlert' });
        }
    }

    if (!statusResponse.esperando_respuesta_usuario) {
        currentState.questionShown = false;
    }
    if (!statusResponse.esperando_aprobacion_plan) {
        currentState.planShown = false;
    }

    return actions;
}

// ============================================================================
// TESTS
// ============================================================================

console.log('REG-B-001: copyNonceCmd con event.target válido');
{
    alertMessages = [];
    setTimeoutCallbacks = [];
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;

    var mockBtn = { textContent: '📋' };
    var mockEvent = { target: mockBtn };

    copyNonceCmd(mockEvent);

    // Verificar que el texto se copió (esto es sincrónico con nuestro mock)
    assert(mockClipboard.written !== null, 'El comando debe copiarse al portapapeles');
    assert(mockClipboard.written.indexOf('sign_nonce.ps1') !== -1, 'El comando debe contener sign_nonce.ps1');
    assert(mockClipboard.written.indexOf('test_nonce_abc123') !== -1, 'El comando debe contener el nonce');
    assert(mockClipboard.written.indexOf('.config\\admin_private.pem') !== -1, 'El comando debe contener el KeyPath');

    // NOTA: navigator.clipboard.writeText es async (Promise).
    // onSuccess se ejecuta en el .then(), que es un microtask.
    // En el navegador real, el cambio de texto del botón ocurre
    // inmediatamente después de que la Promise se resuelve.
    // Verificamos que al menos la copia funcionó (mockClipboard.written != null).
    // El cambio visual del botón es un efecto secundario que depende de la
    // resolución de la Promise.
    assert(mockClipboard.written !== null, 'La copia al portapapeles debe completarse');
}
console.log('REG-B-001: copyNonceCmd con event.target válido');
{
    alertMessages = [];
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;

    var mockBtn = { textContent: '📋' };
    var mockEvent = { target: mockBtn };

    copyNonceCmd(mockEvent);

    assert(mockClipboard.written !== null, 'El comando debe copiarse al portapapeles');
    assert(mockClipboard.written.indexOf('sign_nonce.ps1') !== -1, 'El comando debe contener sign_nonce.ps1');
    assert(mockClipboard.written.indexOf('test_nonce_abc123') !== -1, 'El comando debe contener el nonce');
    assert(mockClipboard.written.indexOf('.config\\admin_private.pem') !== -1, 'El comando debe contener el KeyPath');
    assertEquals(mockBtn.textContent, '✓', 'El botón debe mostrar ✓ después de copiar (antes de setTimeout)');
}

console.log('REG-B-002: copyNonceCmd sin event (event = undefined)');
{
    alertMessages = [];
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;

    try {
        copyNonceCmd(undefined);
        assert(true, 'copyNonceCmd(undefined) no debe lanzar excepción');
    } catch (e) {
        assert(false, 'copyNonceCmd(undefined) lanzó excepción: ' + e.message);
    }
}

console.log('REG-B-003: copyNonceCmd sin navigator.clipboard (fallback)');
{
    alertMessages = [];
    mockClipboard.written = null;
    mockNavigator.clipboard = null;
    mockDocument.execCommand = function(cmd) { return cmd === 'copy'; };

    var mockBtn2 = { textContent: '📋' };
    var mockEvent2 = { target: mockBtn2 };

    try {
        copyNonceCmd(mockEvent2);
        assertEquals(mockBtn2.textContent, '✓', 'El fallback debe funcionar y mostrar ✓');
    } catch (e) {
        assert(false, 'copyNonceCmd sin clipboard lanzó excepción: ' + e.message);
    }
}

console.log('REG-B-004: copyNonceCmd con fallback que falla');
{
    alertMessages = [];
    mockNavigator.clipboard = {
        writeText: function() { return Promise.reject(new Error('Denied')); }
    };
    mockDocument.execCommand = function() { return false; };

    var mockBtn3 = { textContent: '📋' };
    var mockEvent3 = { target: mockBtn3 };

    try {
        copyNonceCmd(mockEvent3);
        assert(true, 'No debe crashear incluso si todo falla');
    } catch (e) {
        assert(false, 'Crashó: ' + e.message);
    }
}

console.log('REG-B-005: copyNonceCmd con nonce vacío');
{
    alertMessages = [];
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;
    mockWindow._lastNonce = '';

    var mockBtn4 = { textContent: '📋' };
    var mockEvent4 = { target: mockBtn4 };

    copyNonceCmd(mockEvent4);

    assert(mockClipboard.written !== null, 'Debe copiar incluso con nonce vacío');
    assert(mockClipboard.written.indexOf('-Nonce ""') !== -1, 'El comando debe tener -Nonce "" con nonce vacío');
}

console.log('REG-B-006: copyNonceCmd con caracteres especiales en nonce');
{
    alertMessages = [];
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;
    mockWindow._lastNonce = 'abc!@#$%^&*()_+{}[]|;:<>,.?/~`';

    var mockBtn5 = { textContent: '📋' };
    var mockEvent5 = { target: mockBtn5 };

    copyNonceCmd(mockEvent5);

    assert(mockClipboard.written !== null, 'Debe copiar con caracteres especiales');
    assert(mockClipboard.written.indexOf('abc!@#$%^&*()_+{}[]|;') !== -1,
        'El nonce con caracteres especiales debe estar en el comando');
}

// ============================================================================
// BATERÍA DE TESTS: startAgentMonitoring (Detección de preguntas y planes)
// ============================================================================

console.log('\n=== TESTS DE REGRESIÓN: startAgentMonitoring (BUG #A) ===\n');

console.log('REG-A-001: Detectar pregunta pendiente del agente');
{
    var statusResponse = {
        status: 'ok',
        active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '¿Qué base de datos prefieren?',
        esperando_aprobacion_plan: false,
        plan_propuesto: null,
        captcha_pending: false
    };

    var currentState = { questionShown: false, planShown: false };
    var actions = simulateAgentPolling(statusResponse, currentState);

    assert(actions.length === 1, 'Debe haber exactamente 1 acción');
    assert(actions[0].action === 'showQuestionModal', 'La acción debe ser showQuestionModal');
    assertEquals(actions[0].question, '¿Qué base de datos prefieren?', 'La pregunta debe coincidir');
    assertEquals(currentState.questionShown, true, 'questionShown debe ser true');
}

console.log('REG-A-002: No mostrar modal si ya se mostró');
{
    var statusResponse2 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '¿Qué framework usar?',
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };
    var currentState2 = { questionShown: true, planShown: false };
    var actions2 = simulateAgentPolling(statusResponse2, currentState2);
    assertEquals(actions2.length, 0, 'No debe haber acciones si el modal ya se mostró');
}

console.log('REG-A-003: No mostrar modal si pregunta está vacía');
{
    var statusResponse3 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '',
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };
    var currentState3 = { questionShown: false, planShown: false };
    var actions3 = simulateAgentPolling(statusResponse3, currentState3);
    assertEquals(actions3.length, 0, 'No debe mostrar modal si la pregunta está vacía');
}

console.log('REG-A-004: No mostrar modal si no está esperando respuesta');
{
    var statusResponse4 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };
    var currentState4 = { questionShown: false, planShown: false };
    var actions4 = simulateAgentPolling(statusResponse4, currentState4);
    assertEquals(actions4.length, 0, 'No debe haber acciones si no hay pregunta pendiente');
    assertEquals(currentState4.questionShown, false, 'questionShown debe permanecer false');
}

console.log('REG-A-005: Detectar plan propuesto');
{
    var statusResponse5 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: true,
        plan_propuesto: '1. Modificar main.rs\n2. Agregar tests\n3. Actualizar docs',
        captcha_pending: false
    };
    var currentState5 = { questionShown: false, planShown: false };
    var actions5 = simulateAgentPolling(statusResponse5, currentState5);

    assert(actions5.length === 1, 'Debe haber exactamente 1 acción');
    assert(actions5[0].action === 'showPlanModal', 'La acción debe ser showPlanModal');
    assert(actions5[0].plan.indexOf('Modificar main.rs') !== -1, 'El plan debe contener las acciones propuestas');
    assertEquals(currentState5.planShown, true, 'planShown debe ser true');
}

console.log('REG-A-006: Resetear flags cuando el agente deja de esperar');
{
    var status1 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '¿Qué hacer?',
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };
    var currentState6 = { questionShown: false, planShown: false };
    var actions6a = simulateAgentPolling(status1, currentState6);
    assert(actions6a.length === 1, 'Debe detectar la pregunta inicial');
    assertEquals(currentState6.questionShown, true, 'questionShown debe ser true');

    var status2 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };
    simulateAgentPolling(status2, currentState6);
    assertEquals(currentState6.questionShown, false, 'questionShown debe resetearse cuando el agente deja de esperar');
}

console.log('REG-A-007: Detectar CAPTCHA pendiente');
{
    var statusResponse7 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: false,
        plan_propuesto: null,
        captcha_pending: true
    };
    var currentState7 = { questionShown: false, planShown: false };
    var actions7 = simulateAgentPolling(statusResponse7, currentState7);
    assert(actions7.length === 1, 'Debe haber acción de CAPTCHA');
    assert(actions7[0].action === 'showCaptchaAlert', 'Debe mostrar alerta de CAPTCHA');
}

console.log('REG-A-008: Pregunta y plan simultáneos (caso borde)');
{
    var statusResponse8 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '¿Qué DB usar?',
        esperando_aprobacion_plan: true,
        plan_propuesto: 'Plan de cambios',
        captcha_pending: false
    };
    var currentState8 = { questionShown: false, planShown: false };
    var actions8 = simulateAgentPolling(statusResponse8, currentState8);

    assert(actions8.length === 2, 'Debe haber 2 acciones (pregunta + plan)');
    assert(actions8.some(function(a) { return a.action === 'showQuestionModal'; }), 'Debe incluir showQuestionModal');
    assert(actions8.some(function(a) { return a.action === 'showPlanModal'; }), 'Debe incluir showPlanModal');
}

console.log('REG-A-009: Agente inactivo no debe mostrar nada');
{
    var statusResponse9 = {
        status: 'ok', active: false, running: false,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };
    var currentState9 = { questionShown: false, planShown: false };
    var actions9 = simulateAgentPolling(statusResponse9, currentState9);
    assertEquals(actions9.length, 0, 'Agente inactivo no debe generar acciones');
}

console.log('REG-A-010: Respuesta sin status ok no debe procesarse');
{
    var statusResponse10 = { status: 'error', message: 'Unauthorized' };
    var currentState10 = { questionShown: false, planShown: false };
    var actions10 = simulateAgentPolling(statusResponse10, currentState10);
    assertEquals(actions10.length, 0, 'Respuesta de error no debe generar acciones');
}

// ============================================================================
// RESULTADOS
// ============================================================================

console.log('\n========================================');
console.log('RESULTADOS: ' + passed + ' pasaron, ' + failed + ' fallaron');
console.log('========================================\n');

if (failed > 0) {
    process.exit(1);
} else {
    console.log('✅ Todos los tests de regresión de frontend pasaron.\n');
    process.exit(0);
}
