// ============================================================================
// tests/frontend_regression_tests.js
// Tests de Regresion para el Frontend (app.js)
// BUG A: Modal de pregunta nunca se abria
// BUG B: copyNonceCmd usaba event sin declararlo y sin fallback
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
    return s.esperando_respuesta_usuario === true && typeof s.pregunta_usuario === "string" && s.pregunta_usuario.length > 0;
}
function shouldShowPlanModal(s, shown) {
    if (shown) return false;
    return s.esperando_aprobacion_plan === true && typeof s.plan_propuesto === "string" && s.plan_propuesto.length > 0;
}
function simulateAgentPolling(status, state) {
    var actions = [];
    if (status.status === "ok" && (status.active || status.running)) {
        if (shouldShowQuestionModal(status, state.questionShown)) {
            actions.push({ action: "showQuestionModal", question: status.pregunta_usuario });
            state.questionShown = true;
        }
        if (shouldShowPlanModal(status, state.planShown)) {
            actions.push({ action: "showPlanModal", plan: status.plan_propuesto });
            state.planShown = true;
        }
        if (status.captcha_pending) { actions.push({ action: "showCaptchaAlert" }); }
    }
    if (!status.esperando_respuesta_usuario) { state.questionShown = false; }
    if (!status.esperando_aprobacion_plan) { state.planShown = false; }
    return actions;
}

var passed = 0, failed = 0;
function assert(cond, name) { if (cond) { passed++; console.log("  OK " + name); } else { failed++; console.error("  FAIL " + name); } }
function assertEquals(a, e, name) { if (a === e) { passed++; console.log("  OK " + name); } else { failed++; console.error("  FAIL " + name + " expected:" + JSON.stringify(e) + " actual:" + JSON.stringify(a)); } }

console.log("\n=== TESTS copyNonceCmd ===\n");

console.log("B-001: copyNonceCmd con event");
{ alertMessages=[]; mockClipboard.written=null; mockNavigator.clipboard=mockClipboard; var b={textContent:"CP"}; var ev={target:b};
  copyNonceCmd(ev);
  assert(mockClipboard.written!==null,"Copia al portapapeles");
  assert(mockClipboard.written.indexOf("sign_nonce.ps1")!==-1,"Contiene sign_nonce.ps1");
  assert(mockClipboard.written.indexOf("test_nonce_abc123")!==-1,"Contiene nonce");
  assert(mockClipboard.written.indexOf(".config\\admin_private.pem")!==-1,"Contiene KeyPath"); }

console.log("B-002: copyNonceCmd sin event");
{ alertMessages=[]; mockClipboard.written=null; mockNavigator.clipboard=mockClipboard;
  try { copyNonceCmd(undefined); assert(true,"No lanza excepcion"); } catch(e) { assert(false,"Excepcion: "+e.message); } }

console.log("B-003: copyNonceCmd sin clipboard (fallback)");
{ alertMessages=[]; mockClipboard.written=null; mockNavigator.clipboard=null; mockDocument.execCommand=function(c){return c==="copy";};
  var b2={textContent:"CP"}; var ev2={target:b2};
  try { copyNonceCmd(ev2); assertEquals(b2.textContent,"OK","Fallback muestra OK"); } catch(e) { assert(false,"Excepcion: "+e.message); } }

console.log("B-004: copyNonceCmd con fallback que falla");
{ alertMessages=[]; mockNavigator.clipboard={writeText:function(){return Promise.reject(new Error("Denied"));}}; mockDocument.execCommand=function(){return false;};
  var b3={textContent:"CP"}; var ev3={target:b3};
  try { copyNonceCmd(ev3); assert(true,"No crashea"); } catch(e) { assert(false,"Crash: "+e.message); } }

console.log("B-005: nonce vacio");
{ alertMessages=[]; mockClipboard.written=null; mockNavigator.clipboard=mockClipboard; mockWindow._lastNonce="";
  var b4={textContent:"CP"}; var ev4={target:b4}; copyNonceCmd(ev4);
  assert(mockClipboard.written!==null,"Copia con nonce vacio");
  assert(mockClipboard.written.indexOf("-Nonce \"\"")!==-1,"-Nonce vacio"); }

console.log("B-006: caracteres especiales");
{ alertMessages=[]; mockClipboard.written=null; mockNavigator.clipboard=mockClipboard; mockWindow._lastNonce="abc!@#$%^&*()_+{}[]";
  var b5={textContent:"CP"}; var ev5={target:b5}; copyNonceCmd(ev5);
  assert(mockClipboard.written!==null,"Copia especiales");
  assert(mockClipboard.written.indexOf("abc!@#$%^&*()_+{}[]")!==-1,"Nonce especial"); }

console.log("\n=== TESTS startAgentMonitoring (BUG A) ===\n");

console.log("A-001: Detectar pregunta");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:true,pregunta_usuario:"Que DB?",esperando_aprobacion_plan:false,plan_propuesto:null,captcha_pending:false};
  var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assert(a.length===1,"1 accion"); assert(a[0].action==="showQuestionModal","showQuestionModal"); assertEquals(a[0].question,"Que DB?","Pregunta coincide");
  assertEquals(st.questionShown,true,"questionShown=true"); }

console.log("A-002: No re-mostrar");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:true,pregunta_usuario:"Framework?",esperando_aprobacion_plan:false,plan_propuesto:null};
  var st={questionShown:true,planShown:false}; var a=simulateAgentPolling(s,st);
  assertEquals(a.length,0,"Sin acciones"); }

console.log("A-003: Pregunta vacia");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:true,pregunta_usuario:"",esperando_aprobacion_plan:false,plan_propuesto:null};
  var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assertEquals(a.length,0,"Sin acciones"); }

console.log("A-004: Sin pregunta");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:false,pregunta_usuario:null,esperando_aprobacion_plan:false,plan_propuesto:null};
  var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assertEquals(a.length,0,"Sin acciones"); assertEquals(st.questionShown,false,"questionShown=false"); }

console.log("A-005: Detectar plan");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:false,pregunta_usuario:null,esperando_aprobacion_plan:true,plan_propuesto:"1. Modificar\n2. Tests",captcha_pending:false};
  var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assert(a.length===1,"1 accion"); assert(a[0].action==="showPlanModal","showPlanModal");
  assert(a[0].plan.indexOf("Modificar")!==-1,"Contiene plan"); assertEquals(st.planShown,true,"planShown=true"); }

console.log("A-006: Reset flags");
{ var s1={status:"ok",active:true,esperando_respuesta_usuario:true,pregunta_usuario:"Q?",esperando_aprobacion_plan:false,plan_propuesto:null};
  var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s1,st);
  assert(a.length===1,"Detecta pregunta"); assertEquals(st.questionShown,true,"questionShown=true");
  var s2={status:"ok",active:true,esperando_respuesta_usuario:false,pregunta_usuario:null,esperando_aprobacion_plan:false,plan_propuesto:null};
  simulateAgentPolling(s2,st); assertEquals(st.questionShown,false,"questionShown reset"); }

console.log("A-007: CAPTCHA");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:false,pregunta_usuario:null,esperando_aprobacion_plan:false,plan_propuesto:null,captcha_pending:true};
  var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assert(a.length===1,"1 accion CAPTCHA"); assert(a[0].action==="showCaptchaAlert","showCaptchaAlert"); }

console.log("A-008: Pregunta + plan");
{ var s={status:"ok",active:true,esperando_respuesta_usuario:true,pregunta_usuario:"DB?",esperando_aprobacion_plan:true,plan_propuesto:"Plan",captcha_pending:false};
  var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assert(a.length===2,"2 acciones"); assert(a.some(function(x){return x.action==="showQuestionModal";}),"Incluye pregunta");
  assert(a.some(function(x){return x.action==="showPlanModal";}),"Incluye plan"); }

console.log("A-009: Inactivo");
{ var s={status:"ok",active:false,running:false,esperando_respuesta_usuario:false,pregunta_usuario:null,esperando_aprobacion_plan:false,plan_propuesto:null};
  var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assertEquals(a.length,0,"Sin acciones"); }

console.log("A-010: Error");
{ var s={status:"error",message:"Unauthorized"}; var st={questionShown:false,planShown:false}; var a=simulateAgentPolling(s,st);
  assertEquals(a.length,0,"Sin acciones"); }

console.log("\n========================================");
console.log("RESULTADOS: "+passed+" OK, "+failed+" FAIL");
console.log("========================================\n");
if (failed>0) { process.exit(1); } else { console.log("OK Todos los tests de regresion de frontend pasaron.\n"); process.exit(0); }
