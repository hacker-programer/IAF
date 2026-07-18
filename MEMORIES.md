# MEMORIES.md — Lecciones Aprendidas y Limitaciones del Proyecto IAF

> Archivo de memoria persistente para minimizar llamadas innecesarias al modelo,
> cómputo redundante y llamadas repetitivas de red.

---

## 🔴 Limitaciones y Fallos Conocidos

### Google Scraping
- **Fecha**: 2025-01
- **Problema**: La búsqueda en Google (scraper.rs) no usa la API oficial; hace scraping del HTML de resultados.
- **Síntoma**: Puede disparar CAPTCHAs y dejar de funcionar sin previo aviso.
- **Mitigación**: Se usa DuckDuckGo Lite como fallback principal.

### API de Voyage
- **Fecha**: 2025-01
- **Problema**: La API de Voyage no soporta búsquedas híbridas.
- **Mitigación**: Se reemplazó search_code con búsqueda local por palabras clave (sin embeddings).

### Validación de Escritura
- **Fecha**: 2025-01
- **Problema**: El validador post-escritura en main.rs marcaba falsos positivos al detectar `const` en JS como "definiciones duplicadas".
- **Mitigación**: Ignorar warnings de validación en archivos JS. El validador está diseñado para Rust.

### Ediciones Parciales (start_line/end_line)
- **Fecha**: 2025-07
- **Problema**: Usar `write_file_with_commit` con `start_line`/`end_line` causa corrupción de archivos: código huérfano, definiciones duplicadas, funciones insertadas en medio de otras.
- **Síntoma**: Tras 3 ediciones parciales, main.rs quedó con código mezclado, funciones dentro de funciones, y braces desbalanceados.
- **Mitigación**: SIEMPRE escribir el archivo COMPLETO. Para archivos grandes (>1000 líneas), usar PowerShell para hacer reemplazos de texto y luego hacer commit con git.

### Espacio en Disco C
- **Fecha**: 2025-07
- **Problema**: C:\ puede llenarse con artifacts de cargo (cargo-target, auto-cargo-target), Docker installers, WSL MSIs, etc.
- **Síntoma**: `write_file_with_commit` falla con "os error 112" (ENOSPC).
- **Mitigación**: Limpiar `C:\Users\Fa\AppData\Local\Temp` periódicamente: remover `auto-cargo-target`, `DockerDesktopInstallers`, `wsl*.msi`, `vscode-stable-user-x64`.

### Problema de Encoding en PowerShell Replace
- **Fecha**: 2025-07
- **Problema**: `$content.Replace()` en PowerShell falla silenciosamente si el texto contiene caracteres Unicode (como `→`) que no coinciden exactamente.
- **Mitigación**: Usar patrones más cortos y específicos (ej: reemplazar solo la línea `fs::rename` en vez del bloque entero).

---

## 🟡 Bugs Corregidos (v2.4)

### Persistencia de Proyectos (2025-07)
- **Síntoma**: Los proyectos desaparecían al reiniciar el servidor.
- **Causa raíz**: `migrate_chats()` usaba `fs::rename()` para renombrar `local_projects.json` a `.bak`. En el segundo arranque, el archivo ya no existía y `initial_projects` arrancaba vacío.
- **Solución**: Cambiar `rename` por `copy`. Agregar recovery: si `.json` no existe pero `.bak` sí, restaurar. Si no existe nada, persistir desde memoria.

### Admin Creation Requería Contraseña (2025-07)
- **Síntoma**: El frontend siempre exigía contraseña al crear usuarios, incluso admins (que deben usar clave pública).
- **Causa raíz**: `createUserBtn.onclick` no distinguía entre admin y usuario normal. Siempre enviaba `password`.
- **Solución**: Modificar el frontend para detectar el checkbox "Admin" y mostrar campo de clave pública + upload .pem + generación de claves en vez de contraseña.

### Scripts .ps1 No Accesibles (2025-07)
- **Síntoma**: Los scripts `generate_keys.ps1` y `sign_nonce.ps1` existían en disco pero no eran accesibles desde la UI.
- **Causa raíz**: No había endpoint REST para servirlos ni links en el frontend.
- **Solución**: Agregar `GET /api/scripts/:name` y botones de descarga en el admin panel.

---

## 🟢 Patrones Útiles Descubiertos

### PowerShell para Ediciones Seguras
Para archivos grandes donde `write_file_with_commit` completo es impráctico:
1. Leer archivo con `Get-Content -Raw -Encoding UTF8`
2. Hacer reemplazos con `.Replace(old, new)` usando strings literales cortos
3. Guardar con `Set-Content -Path ... -Value ... -Encoding UTF8 -NoNewline`
4. Hacer commit con `git add ...; git commit -m ...; git push`
5. Verificar con `read_file` que el cambio es correcto

### Limpieza de Disco para Compilación
```powershell
Remove-Item "C:\Users\Fa\AppData\Local\Temp\auto-cargo-target" -Recurse -Force
Remove-Item "C:\Users\Fa\AppData\Local\Temp\DockerDesktopInstallers" -Recurse -Force
Remove-Item "$env:TEMP\cargo-target\debug\incremental" -Recurse -Force
```

---

## 🔵 CAPTCHA Endpoints 404 (2025-02) — Corregido en v2.3
- **Síntoma**: El frontend hacía polling cada 3s a `/api/captcha/status` y `/api/captcha/solve` que no existían.
- **Causa raíz**: `CaptchaRequest` existía en `state.rs` pero nunca se implementaron los handlers HTTP.
- **Solución**: Se agregaron `captcha_status` (GET) y `captcha_solve` (POST) en `main.rs`.

### apiCall se rompía con respuestas no-JSON (2025-02)
- **Síntoma**: `Uncaught (in promise) SyntaxError: Failed to execute 'json' on 'Response'`
- **Solución**: `apiCall` ahora lee `res.text()` primero y usa `try/catch` para parsear JSON, devolviendo objeto de error estructurado si falla.