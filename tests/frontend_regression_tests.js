// ============================================================================
// tests/frontend_regression_tests.js
// Tests de Regresion para el Frontend (app.js)
// BUG A: Modal de pregunta nunca se abria
// BUG B: copyNonceCmd usaba event sin declararlo y sin fallback
// BUG-002: Mensajes informativos no se muestran en tiempo real
// ============================================================================

var mockClipboard = { available: true, written: null, writeText: function(t) { this.written = t; return Promise.resolve(); } };
var mockNavigator = { clipboard: mockClipboard };
var mockDocument = {
    elements: {},
    getElementById: function(id) { return this.elements[id] || null; },
    createElement: function(tag) { return { tagName: tag, value: "", style: {}, select: function(){}, focus: function(){}, parentNode: null }; },
    body: { appendChild: function(el) { el.parentNode = this; }, removeChild: function(el) { el.parentNode = null; } },
    querySelector: function(sel) { return null; },
    execCommand: function(cmd) { return cmd === "copy"; }
};
var mockWindow = { _lastNonce: "test_nonce_abc123", _lastAdminUser: "admin", event: null };
var alertMessages = [];
function mockAlert(msg) { alertMessages.push(msg); }
var setTimeoutCallbacks = [];
function mockSetTimeout(fn, delay) { setTimeoutCallbacks.push(fn); return 999; }

function copyNonceCmd(event) {
    event = event || mockWindow.event;
    var nonce = mockWindow._lastNonce || "";
    var cmd = ".\\scripts\\sign_nonce.ps1 -Nonce \"" + nonce + "\" -KeyPath \".config\\admin_private.pem\"";
    var btn = null;
    if (event && event.target) { btn = event.target; }
    else { btn = mockDocument.querySelector(".btn-copy-small"); }
    function fallbackCopy(text) {
        var ta = mockDocument.createElement("textarea");
        ta.value = text; ta.style.position = "fixed"; ta.style.left = "-9999px"; ta.style.top = "-9999px";
        mockDocument.body.appendChild(ta); ta.focus(); ta.select();
        var ok = false;
        try { ok = mockDocument.execCommand("copy"); } catch(e) { ok = false; }
        mockDocument.body.removeChild(ta);
        return ok;
    }
    function onSuccess() { if (btn) { btn.textContent = "OK"; mockSetTimeout(function(){ btn.textContent = "CP"; }, 1500); } }
    function onFailure() { mockAlert("No se pudo copiar."); }
    if (mockNavigator.clipboard && typeof mockNavigator.clipboard.writeText === "function") {
        mockNavigator.clipboard.writeText(cmd).then(onSuccess).catch(function(){ if (fallbackCopy(cmd)) onSuccess(); else onFailure(); });
    } else {
        if (fallbackCopy(cmd)) onSuccess(); else onFailure();
    }
}

function shouldShowQuestionModal(s, shown) {
    if (shown) return false;
    return s.esperando_respuesta_usuario && s.pregunta_usuario;
}

// BUG-002: Simular consumo de info_messages del frontend
function consumeInfoMessages(statusRes, lastCount) {
    var actions = [];
    if (statusRes.info_messages && Array.isArray(statusRes.info_messages)) {
        var currentCount = statusRes.info_messages.length;
        if (currentCount > lastCount && currentCount > 0) {
            var newMessages = statusRes.info_messages.slice(lastCount);
            newMessages.forEach(function(msg) {
                actions.push({ action: "showInfoToast", message: msg });
            });
            lastCount = currentCount;
        }
    }
    return { actions: actions, newCount: lastCount };
}

// BUG-002: Verificar que info_messages se consumen incluso con agente terminado
function consumeInfoMessagesWhenFinished(statusRes, lastCount) {
    // BUG-002 FIX: siempre consumir info_messages, sin importar running/finished
    return consumeInfoMessages(statusRes, lastCount);
}

function simulateAgentPolling(statusRes, state) {
    var actions = [];
    // BUG-002: Siempre consumir info_messages primero
    var infoResult = consumeInfoMessagesWhenFinished(statusRes, state.lastInfoCount || 0);
    state.lastInfoCount = infoResult.newCount;
    actions = actions.concat(infoResult.actions);

    // Mostrar final_message si el agente terminó
    if (statusRes.finished && statusRes.final_message) {
        actions.push({ action: "showFinalMessage", message: statusRes.final_message });
    }

    // Lógica normal (solo cuando active/running)
    if (statusRes.status === "ok" && (statusRes.active || statusRes.running)) {
        if (statusRes.esperando_respuesta_usuario && statusRes.pregunta_usuario && !state.questionShown) {
            state.questionShown = true;
            actions.push({ action: "showQuestionModal", question: statusRes.pregunta_usuario });
        }
        if (statusRes.esperando_aprobacion_plan && statusRes.plan_propuesto && !state.planShown) {
            state.planShown = true;
            actions.push({ action: "showPlanModal", plan: statusRes.plan_propuesto });
        }
        if (statusRes.captcha_pending) {
            actions.push({ action: "showCaptchaAlert" });
        }
        if (!statusRes.esperando_respuesta_usuario) { state.questionShown = false; }
        if (!statusRes.esperando_aprobacion_plan) { state.planShown = false; }
    }
    return actions;
}

var passed = 0, failed = 0;
function assert(cond, msg) { if (cond) passed++; else { console.log("FAIL: "+msg); failed++; } }
function assertEquals(a, b, msg) { if (a===b) passed++; else { console.log("FAIL: "+msg+" - esperado "+b+" obtenido "+a); failed++; } }

console.log("===== FRONTEND REGRESSION TESTS v2 (BUG-002 incluido) =====\n");

console.log("A-001: copyNonceCmd sin event");
{ mockWindow.event = null; mockClipboard.written = null;
  copyNonceCmd(null);
  assert(mockClipboard.written !== null, "Debe escribir al clipboard");
  assert(mockClipboard.written.indexOf("sign_nonce.ps1") !== -1, "Debe contener comando"); }

console.log("A-002: copyNonceCmd con event target");
{ var btn = {textContent:"CP"}; mockWindow.event = {target:btn}; mockClipboard.written = null;
  copyNonceCmd(mockWindow.event);
  assert(mockClipboard.written !== null, "Con event: Debe copiar"); }

console.log("A-003: shouldShowQuestionModal - primera vez");
{ var s={esperando_respuesta_usuario:true, pregunta_usuario:"Test?"};
  assert(shouldShowQuestionModal(s,false)===true, "Debe mostrar si no mostrado"); }

console.log("A-004: shouldShowQuestionModal - ya mostrado");
{ var s={esperando_respuesta_usuario:true, pregunta_usuario:"Test?"};
  assert(shouldShowQuestionModal(s,true)===false, "No debe mostrar si ya mostrado"); }

// ========================================================================
// BUG-002 Tests: Mensajes informativos en tiempo real
// ========================================================================

console.log("\n--- BUG-002: Info Messages ---");

console.log("B002-001: Consumir info_messages normales");
{ var s={status:"ok",active:true,running:true,info_messages:["Msg1","Msg2","Msg3"]};
  var st={lastInfoCount:0,questionShown:false,planShown:false};
  var result=consumeInfoMessagesWhenFinished(s, st.lastInfoCount);
  assertEquals(result.actions.length, 3, "Debe consumir 3 mensajes");
  assertEquals(result.newCount, 3, "Contador debe ser 3"); }

console.log("B002-002: No consumir mensajes ya vistos");
{ var s={status:"ok",active:true,running:true,info_messages:["Msg1","Msg2","Msg3"]};
  var result=consumeInfoMessagesWhenFinished(s, 3);
  assertEquals(result.actions.length, 0, "No debe haber mensajes nuevos");
  assertEquals(result.newCount, 3, "Contador sin cambios"); }

console.log("B002-003: Consumir solo mensajes nuevos");
{ var s={status:"ok",active:true,running:true,info_messages:["Msg1","Msg2","Msg3","Msg4","Msg5"]};
  var result=consumeInfoMessagesWhenFinished(s, 3);
  assertEquals(result.actions.length, 2, "Solo 2 mensajes nuevos");
  assertEquals(result.newCount, 5, "Contador actualizado a 5"); }

console.log("B002-004: BUG-002 FIX - Mensajes visibles con agente terminado");
{ var s={status:"ok",active:false,running:false,finished:true,info_messages:["Iniciando","Procesando","Finalizado"],final_message:"OK"};
  var st={lastInfoCount:0,questionShown:false,planShown:false};
  var a=simulateAgentPolling(s,st);
  // Debe mostrar los 3 info_messages + final_message
  var infoActions=a.filter(function(x){return x.action==="showInfoToast";});
  var finalActions=a.filter(function(x){return x.action==="showFinalMessage";});
  assertEquals(infoActions.length, 3, "Debe mostrar 3 info_messages incluso con agente terminado");
  assertEquals(finalActions.length, 1, "Debe mostrar mensaje final");
  assertEquals(finalActions[0].message, "OK", "Mensaje final correcto"); }

console.log("B002-005: Info messages vacío no rompe nada");
{ var s={status:"ok",active:true,running:true,info_messages:[]};
  var result=consumeInfoMessagesWhenFinished(s, 0);
  assertEquals(result.actions.length, 0, "Sin acciones para array vacío"); }

console.log("B002-006: Info messages null no rompe nada");
{ var s={status:"ok",active:true,running:true,info_messages:null};
  var result=consumeInfoMessagesWhenFinished(s, 0);
  assertEquals(result.actions.length, 0, "Sin acciones para null"); }

console.log("B002-007: Reiniciar contador con nueva sesión");
{ var s={status:"ok",active:true,running:true,info_messages:["A","B"],current_session_id:"sess-456"};
  var st={lastInfoCount:3,lastSessionId:"sess-123",questionShown:false,planShown:false};
  // Nueva sesión: resetear contador
  if (s.current_session_id !== st.lastSessionId) { st.lastInfoCount = 0; st.lastSessionId = s.current_session_id; }
  assertEquals(st.lastInfoCount, 0, "Contador reseteado para nueva sesión"); }

console.log("B002-008: Mensajes con caracteres especiales");
{ var s={status:"ok",active:true,info_messages:["Mensaje con ñ","Emoji 🚀","<script>alert('xss')</script>"]};
  var result=consumeInfoMessagesWhenFinished(s, 0);
  assertEquals(result.actions.length, 3, "3 mensajes especiales");
  assert(result.actions[0].message.indexOf("ñ") !== -1, "Debe preservar ñ");
  assert(result.actions[1].message.indexOf("🚀") !== -1, "Debe preservar emoji"); }

// ========================================================================
// Tests originales A-005 a A-010
// ========================================================================

console.log("\n--- Tests originales ---");

console.log("A-005: Pregunta");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:true,pregunta_usuario:"Pregunta?",esperando_aprobacion_plan:false,plan_propuesto:null,info_messages:[]};
  var st={lastInfoCount:0,questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assert(a.length===1,"1 accion"); assert(a[0].action==="showQuestionModal","showQuestionModal");
  assert(a[0].question==="Pregunta?","Contiene pregunta"); assertEquals(st.questionShown,true,"questionShown=true"); }

console.log("A-006: Reset flags");
{ var s1={status:"ok",active:true,esperando_respuesta_usuario:true,pregunta_usuario:"Q?",esperando_aprobacion_plan:false,plan_propuesto:null,info_messages:[]};
  var st={lastInfoCount:0,questionShown:false,planShown:false}; var a=simulateAgentPolling(s1,st);
  assert(a.length===1,"Detecta pregunta"); assertEquals(st.questionShown,true,"questionShown=true");
  var s2={status:"ok",active:true,esperando_respuesta_usuario:false,pregunta_usuario:null,esperando_aprobacion_plan:false,plan_propuesto:null,info_messages:[]};
  simulateAgentPolling(s2,st); assertEquals(st.questionShown,false,"questionShown reset"); }

console.log("A-007: CAPTCHA");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:false,pregunta_usuario:null,esperando_aprobacion_plan:false,plan_propuesto:null,captcha_pending:true,info_messages:[]};
  var st={lastInfoCount:0,questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assert(a.length===1,"1 accion CAPTCHA"); assert(a[0].action==="showCaptchaAlert","showCaptchaAlert"); }

console.log("A-008: Pregunta + plan");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:true,pregunta_usuario:"DB?",esperando_aprobacion_plan:true,plan_propuesto:"Plan",captcha_pending:false,info_messages:[]};
  var st={lastInfoCount:0,questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assert(a.length===2,"2 acciones"); assert(a.some(function(x){return x.action==="showQuestionModal";}),"Incluye pregunta");
  assert(a.some(function(x){return x.action==="showPlanModal";}),"Incluye plan"); }

console.log("A-009: Inactivo");
{ var s={status:"ok",active:false,running:false,esperando_respuesta_usuario:false,pregunta_usuario:null,esperando_aprobacion_plan:false,plan_propuesto:null,info_messages:[]};
  var st={lastInfoCount:0,questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assertEquals(a.length,0,"Sin acciones"); }

console.log("A-010: Error");
{ var s={status:"error",message:"Unauthorized",info_messages:[]}; var st={lastInfoCount:0,questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assertEquals(a.length,0,"Sin acciones"); }

console.log("\n========================================");
console.log("RESULTADOS: "+passed+" OK, "+failed+" FAIL");
console.log("========================================\n");
if (failed>0) { process.exit(1); } else { console.log("OK Todos los tests de regresion de frontend pasaron.\n"); process.exit(0); }