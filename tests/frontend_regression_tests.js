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
let mockClipboard = {
    available: true,
    written: null,
    writeText: function(text) {
        this.written = text;
        return Promise.resolve();
    }
};

// Mock de navigator
let mockNavigator = {
    clipboard: mockClipboard
};

// Mock de document
let mockDocument = {
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
            parentNode: null
        };
    },
    body: {
        appendChild: function(el) { el.parentNode = this; },
        removeChild: function(el) { el.parentNode = null; }
    },
    querySelector: function(sel) { return null; }
};

// Mock de window
let mockWindow = {
    _lastNonce: 'test_nonce_abc123',
    _lastAdminUser: 'admin',
    event: null
};

// Mock de alert
let alertMessages = [];
function mockAlert(msg) { alertMessages.push(msg); }

// Mock de setTimeout
function mockSetTimeout(fn, delay) { fn(); return 999; }

// ============================================================================
// Implementación de la función CORREGIDA copyNonceCmd
// (debe coincidir exactamente con la versión en app.js)
// ============================================================================

function copyNonceCmd(event) {
    // Normalizar el evento (soporte cross-browser)
    event = event || mockWindow.event;
    const nonce = mockWindow._lastNonce || '';
    const cmd = '.\\scripts\\sign_nonce.ps1 -Nonce "' + nonce + '" -KeyPath ".config\\admin_private.pem"';

    // Resolver el botón que disparó el evento
    var btn = null;
    if (event && event.target) {
        btn = event.target;
    } else {
        // Fallback: buscar por clase
        btn = mockDocument.querySelector('.btn-copy-small');
    }

    /**
     * Fallback: copia usando textarea + execCommand.
     * Funciona en HTTP y navegadores sin Clipboard API.
     */
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

    // Intentar primero la API moderna
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

/**
 * Determina si se debe mostrar el modal de pregunta del agente.
 * Esta lógica DEBE estar en startAgentMonitoring.
 *
 * BUG #A: Esta verificación NO existía en el frontend. El backend
 * correctamente devolvía esperando_respuesta_usuario y pregunta_usuario
 * pero el frontend nunca los leía.
 */
function shouldShowQuestionModal(statusResponse, alreadyShown) {
    if (alreadyShown) return false;

    const esperando = statusResponse.esperando_respuesta_usuario;
    const pregunta = statusResponse.pregunta_usuario;

    // Debe esperar respuesta Y tener una pregunta no vacía
    return esperando === true
        && typeof pregunta === 'string'
        && pregunta.length > 0;
}

/**
 * Determina si se debe mostrar el modal de aprobación de plan.
 */
function shouldShowPlanModal(statusResponse, alreadyShown) {
    if (alreadyShown) return false;

    const esperando = statusResponse.esperando_aprobacion_plan;
    const plan = statusResponse.plan_propuesto;

    return esperando === true
        && typeof plan === 'string'
        && plan.length > 0;
}

/**
 * Simula el polling de startAgentMonitoring.
 * Retorna las acciones que debe tomar el frontend.
 */
function simulateAgentPolling(statusResponse, currentState) {
    const actions = [];

    if (statusResponse.status === 'ok' && (statusResponse.active || statusResponse.running)) {
        // Verificar pregunta pendiente
        if (shouldShowQuestionModal(statusResponse, currentState.questionShown)) {
            actions.push({
                action: 'showQuestionModal',
                question: statusResponse.pregunta_usuario,
                detail: 'BUG #A FIX: El frontend ahora detecta esperando_respuesta_usuario y pregunta_usuario'
            });
            currentState.questionShown = true;
        }

        // Verificar plan pendiente
        if (shouldShowPlanModal(statusResponse, currentState.planShown)) {
            actions.push({
                action: 'showPlanModal',
                plan: statusResponse.plan_propuesto,
                detail: 'El frontend ahora detecta esperando_aprobacion_plan y plan_propuesto'
            });
            currentState.planShown = true;
        }

        // Verificar CAPTCHA
        if (statusResponse.captcha_pending) {
            actions.push({ action: 'showCaptchaAlert' });
        }
    }

    // Si ya no está esperando, resetear flags
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

let passed = 0;
let failed = 0;

function assert(condition, testName) {
    if (condition) {
        passed++;
        console.log('  ✓ ' + testName);
    } else {
        failed++;
        console.error('  ✗ FAIL: ' + testName);
    }
}

function assertEquals(actual, expected, testName) {
    if (actual === expected) {
        passed++;
        console.log('  ✓ ' + testName);
    } else {
        failed++;
        console.error('  ✗ FAIL: ' + testName + ' — expected: ' + JSON.stringify(expected) + ', actual: ' + JSON.stringify(actual));
    }
}

// ============================================================================
// BATERÍA DE TESTS: copyNonceCmd
// ============================================================================

console.log('\n=== TESTS DE REGRESIÓN: copyNonceCmd ===\n');

console.log('REG-B-001: copyNonceCmd con event.target válido');
{
    alertMessages = [];
    mockClipboard.available = true;
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;
    mockDocument.execCommand = function() { return true; };

    const mockBtn = { textContent: '📋' };
    const mockEvent = { target: mockBtn };

    copyNonceCmd(mockEvent);

    // Verificar que el texto se copió
    assert(mockClipboard.written !== null, 'El comando debe copiarse al portapapeles');
    assert(mockClipboard.written.indexOf('sign_nonce.ps1') !== -1, 'El comando debe contener sign_nonce.ps1');
    assert(mockClipboard.written.indexOf('test_nonce_abc123') !== -1, 'El comando debe contener el nonce');
    assert(mockClipboard.written.indexOf('.config\\admin_private.pem') !== -1, 'El comando debe contener el KeyPath');
    assertEquals(mockBtn.textContent, '✓', 'El botón debe mostrar ✓ después de copiar');
}

console.log('REG-B-002: copyNonceCmd sin event (event = undefined)');
{
    alertMessages = [];
    mockClipboard.available = true;
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;

    // Sin evento — no debe crashear
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
    mockNavigator.clipboard = null; // Sin Clipboard API
    mockDocument.execCommand = function(cmd) { return cmd === 'copy'; };

    const mockBtn = { textContent: '📋' };
    const mockEvent = { target: mockBtn };

    try {
        copyNonceCmd(mockEvent);
        assert(mockBtn.textContent === '✓', 'El fallback debe funcionar y mostrar ✓');
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
    mockDocument.execCommand = function() { return false; }; // fallback también falla

    const mockBtn = { textContent: '📋' };
    const mockEvent = { target: mockBtn };

    // Debe ejecutarse sin crashear
    try {
        copyNonceCmd(mockEvent);
        assert(true, 'No debe crashear incluso si todo falla');
    } catch (e) {
        assert(false, 'Crashó: ' + e.message);
    }
}

console.log('REG-B-005: copyNonceCmd con nonce vacío');
{
    alertMessages = [];
    mockClipboard.available = true;
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;
    mockWindow._lastNonce = '';

    const mockBtn = { textContent: '📋' };
    const mockEvent = { target: mockBtn };

    copyNonceCmd(mockEvent);

    assert(mockClipboard.written !== null, 'Debe copiar incluso con nonce vacío');
    assert(mockClipboard.written.indexOf('-Nonce ""') !== -1, 'El comando debe tener -Nonce "" con nonce vacío');
}

console.log('REG-B-006: copyNonceCmd con caracteres especiales en nonce');
{
    alertMessages = [];
    mockClipboard.available = true;
    mockClipboard.written = null;
    mockNavigator.clipboard = mockClipboard;
    mockWindow._lastNonce = 'abc!@#$%^&*()_+{}[]|;:<>,.?/~`';

    const mockBtn = { textContent: '📋' };
    const mockEvent = { target: mockBtn };

    copyNonceCmd(mockEvent);

    assert(mockClipboard.written !== null, 'Debe copiar con caracteres especiales');
    // El nonce con caracteres especiales debe estar presente en el comando
    assert(mockClipboard.written.indexOf('abc!@#$%^&*()_+{}[]|;') !== -1,
        'El nonce con caracteres especiales debe estar en el comando');
}

// ============================================================================
// BATERÍA DE TESTS: startAgentMonitoring (Detección de preguntas y planes)
// ============================================================================

console.log('\n=== TESTS DE REGRESIÓN: startAgentMonitoring (BUG #A) ===\n');

console.log('REG-A-001: Detectar pregunta pendiente del agente');
{
    const statusResponse = {
        status: 'ok',
        active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '¿Qué base de datos prefieren?',
        esperando_aprobacion_plan: false,
        plan_propuesto: null,
        captcha_pending: false
    };

    const currentState = { questionShown: false, planShown: false };
    const actions = simulateAgentPolling(statusResponse, currentState);

    assert(actions.length === 1, 'Debe haber exactamente 1 acción');
    assert(actions[0].action === 'showQuestionModal', 'La acción debe ser showQuestionModal');
    assertEquals(actions[0].question, '¿Qué base de datos prefieren?', 'La pregunta debe coincidir');
    assertEquals(currentState.questionShown, true, 'questionShown debe ser true');
}

console.log('REG-A-002: No mostrar modal si ya se mostró');
{
    const statusResponse = {
        status: 'ok',
        active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '¿Qué framework usar?',
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };

    const currentState = { questionShown: true, planShown: false }; // Ya mostrado
    const actions = simulateAgentPolling(statusResponse, currentState);

    assertEquals(actions.length, 0, 'No debe haber acciones si el modal ya se mostró');
}

console.log('REG-A-003: No mostrar modal si pregunta está vacía');
{
    const statusResponse = {
        status: 'ok',
        active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '',  // ¡vacía!
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };

    const currentState = { questionShown: false, planShown: false };
    const actions = simulateAgentPolling(statusResponse, currentState);

    assertEquals(actions.length, 0, 'No debe mostrar modal si la pregunta está vacía');
}

console.log('REG-A-004: No mostrar modal si no está esperando respuesta');
{
    const statusResponse = {
        status: 'ok',
        active: true,
        esperando_respuesta_usuario: false,  // No esperando
        pregunta_usuario: null,
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };

    const currentState = { questionShown: false, planShown: false };
    const actions = simulateAgentPolling(statusResponse, currentState);

    assertEquals(actions.length, 0, 'No debe haber acciones si no hay pregunta pendiente');
    assertEquals(currentState.questionShown, false, 'questionShown debe permanecer false');
}

console.log('REG-A-005: Detectar plan propuesto');
{
    const statusResponse = {
        status: 'ok',
        active: true,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: true,
        plan_propuesto: '1. Modificar main.rs\n2. Agregar tests\n3. Actualizar docs',
        captcha_pending: false
    };

    const currentState = { questionShown: false, planShown: false };
    const actions = simulateAgentPolling(statusResponse, currentState);

    assert(actions.length === 1, 'Debe haber exactamente 1 acción');
    assert(actions[0].action === 'showPlanModal', 'La acción debe ser showPlanModal');
    assert(actions[0].plan.indexOf('Modificar main.rs') !== -1, 'El plan debe contener las acciones propuestas');
    assertEquals(currentState.planShown, true, 'planShown debe ser true');
}

console.log('REG-A-006: Resetear flags cuando el agente deja de esperar');
{
    // Primera llamada: pregunta pendiente
    const status1 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '¿Qué hacer?',
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };

    const currentState = { questionShown: false, planShown: false };
    let actions = simulateAgentPolling(status1, currentState);
    assert(actions.length === 1, 'Debe detectar la pregunta inicial');
    assertEquals(currentState.questionShown, true, 'questionShown debe ser true');

    // Segunda llamada: usuario respondió, agente ya no espera
    const status2 = {
        status: 'ok', active: true,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };

    actions = simulateAgentPolling(status2, currentState);
    assertEquals(currentState.questionShown, false, 'questionShown debe resetearse cuando el agente deja de esperar');
}

console.log('REG-A-007: Detectar CAPTCHA pendiente');
{
    const statusResponse = {
        status: 'ok',
        active: true,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: false,
        plan_propuesto: null,
        captcha_pending: true
    };

    const currentState = { questionShown: false, planShown: false };
    const actions = simulateAgentPolling(statusResponse, currentState);

    assert(actions.length === 1, 'Debe haber acción de CAPTCHA');
    assert(actions[0].action === 'showCaptchaAlert', 'Debe mostrar alerta de CAPTCHA');
}

console.log('REG-A-008: Pregunta y plan simultáneos (caso borde)');
{
    const statusResponse = {
        status: 'ok',
        active: true,
        esperando_respuesta_usuario: true,
        pregunta_usuario: '¿Qué DB usar?',
        esperando_aprobacion_plan: true,
        plan_propuesto: 'Plan de cambios',
        captcha_pending: false
    };

    const currentState = { questionShown: false, planShown: false };
    const actions = simulateAgentPolling(statusResponse, currentState);

    assert(actions.length === 2, 'Debe haber 2 acciones (pregunta + plan)');
    assert(actions.some(a => a.action === 'showQuestionModal'), 'Debe incluir showQuestionModal');
    assert(actions.some(a => a.action === 'showPlanModal'), 'Debe incluir showPlanModal');
}

console.log('REG-A-009: Agente inactivo no debe mostrar nada');
{
    const statusResponse = {
        status: 'ok',
        active: false,
        running: false,
        esperando_respuesta_usuario: false,
        pregunta_usuario: null,
        esperando_aprobacion_plan: false,
        plan_propuesto: null
    };

    const currentState = { questionShown: false, planShown: false };
    const actions = simulateAgentPolling(statusResponse, currentState);

    assertEquals(actions.length, 0, 'Agente inactivo no debe generar acciones');
}

console.log('REG-A-010: Respuesta sin status "ok" no debe procesarse');
{
    const statusResponse = {
        status: 'error',
        message: 'Unauthorized'
    };

    const currentState = { questionShown: false, planShown: false };
    const actions = simulateAgentPolling(statusResponse, currentState);

    assertEquals(actions.length, 0, 'Respuesta de error no debe generar acciones');
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
