#!/usr/bin/env python3
"""Agrega monitoreo de info_messages y showInfoToast a app.js."""
import sys

with open('public/app.js', 'rb') as f:
    data = f.read()

LE = b'\r\n'
changes = 0

# 1. Add lastInfoMessageCount before setInterval
marker1 = b"    agentMonitorInterval = setInterval(async () => {"
insert1 = b"    let lastInfoMessageCount = 0;" + LE + LE + marker1
if marker1 in data:
    data = data.replace(marker1, insert1)
    print("[OK] 1: lastInfoMessageCount")
    changes += 1
else:
    print("[FAIL] 1")

# 2. Add info_messages monitoring before resetear flag
marker2 = b"            // Si el agente ya no est"
if marker2 in data:
    insert2 = (
        b"            // BUG FIX #B (BUG-002): Mostrar mensajes informativos en tiempo real" + LE +
        b"            if (statusRes.info_messages && Array.isArray(statusRes.info_messages)) {" + LE +
        b"                const currentCount = statusRes.info_messages.length;" + LE +
        b"                if (currentCount > lastInfoMessageCount && currentCount > 0) {" + LE +
        b"                    const newMessages = statusRes.info_messages.slice(lastInfoMessageCount);" + LE +
        b"                    newMessages.forEach(function(msg) {" + LE +
        b"                        showInfoToast(msg);" + LE +
        b"                        addMessage('agent', '[i] ' + msg);" + LE +
        b"                    });" + LE +
        b"                    lastInfoMessageCount = currentCount;" + LE +
        b"                }" + LE +
        b"            }" + LE +
        b"" + LE +
        b"            // Si el agente finalizo, mostrar mensaje final" + LE +
        b"            if (statusRes.finished) {" + LE +
        b"                lastInfoMessageCount = 0;" + LE +
        b"                if (statusRes.final_message) {" + LE +
        b"                    showInfoToast('OK ' + statusRes.final_message);" + LE +
        b"                }" + LE +
        b"            }" + LE +
        b"" + LE +
        b"            // Si el agente ya no est"
    )
    data = data.replace(marker2, insert2)
    print("[OK] 2: info_messages monitoring")
    changes += 1
else:
    print("[FAIL] 2")

# 3. Add showInfoToast function after renderConsoleSteps
# renderConsoleSteps ends with: area.scrollTop = area.scrollHeight;\r\n}\r\n
marker3 = b"document.getElementById('interruptBtn').onclick"
if marker3 in data:
    insert3 = (
        b"" + LE +
        b"function showInfoToast(message) {" + LE +
        b"    var toast = document.createElement('div');" + LE +
        b"    toast.className = 'info-toast';" + LE +
        b"    toast.textContent = message;" + LE +
        b"    toast.style.cssText = 'position:fixed;bottom:20px;right:20px;background:linear-gradient(135deg,#1a1a2e,#16213e);color:#e0e0e0;padding:12px 20px;border-radius:8px;border:1px solid var(--accent,#00d4ff);box-shadow:0 4px 20px rgba(0,0,0,0.5);z-index:10000;max-width:400px;font-size:13px;animation:slideIn 0.3s ease-out;cursor:pointer;';" + LE +
        b"    toast.onclick = function() { toast.remove(); };" + LE +
        b"    document.body.appendChild(toast);" + LE +
        b"    setTimeout(function() {" + LE +
        b"        if (toast.parentNode) {" + LE +
        b"            toast.style.opacity = '0';" + LE +
        b"            toast.style.transition = 'opacity 0.3s';" + LE +
        b"            setTimeout(function() { if (toast.parentNode) toast.remove(); }, 300);" + LE +
        b"        }" + LE +
        b"    }, 8000);" + LE +
        b"}" + LE +
        b"" + LE +
        b"document.getElementById('interruptBtn').onclick"
    )
    data = data.replace(marker3, insert3)
    print("[OK] 3: showInfoToast")
    changes += 1
else:
    print("[FAIL] 3")

# 4. Reset counter on agent stop
marker4 = b"            agentQuestionShown = false;" + LE + b"            agentPlanShown = false;"
insert4 = b"            agentQuestionShown = false;" + LE + b"            agentPlanShown = false;" + LE + b"            lastInfoMessageCount = 0;"
if marker4 in data:
    data = data.replace(marker4, insert4)
    print("[OK] 4: reset counter on stop")
    changes += 1
else:
    print("[FAIL] 4")

with open('public/app.js', 'wb') as f:
    f.write(data)

print(f"\n[DONE] {changes}/4 changes applied")
