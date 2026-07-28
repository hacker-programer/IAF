<# 
.SYNOPSIS
    Ejecuta el túnel Cloudflare para exponer el puerto 8080 de IAF.
    SOLO expone el puerto 8080 (autenticación requerida), NUNCA el puerto 80 (admin local).

.DESCRIPCIÓN
    Tres modos de operación:
    - MODO RUN:       Ejecuta el túnel ya configurado "IAF" -> iaf.mujerbonitauy.com (por defecto)
    - MODO QUICK:     Crea un túnel efímero con URL de trycloudflare.com (para pruebas)
    - MODO SETUP:     Configura un nuevo túnel desde cero con dominio propio

    El puerto 80 NUNCA se expone. Solo se tunela 127.0.0.1:8080.

.PARAMETER Mode
    "run" (defecto), "quick" o "setup".

.PARAMETER TunnelName
    Nombre del túnel. Default: "IAF"

.PARAMETER Domain
    Dominio configurado en Cloudflare. Default: "iaf.mujerbonitauy.com"

.EJEMPLO
    # Simplemente ejecutar el túnel IAF (lo más común)
    .\cloudflare_tunnel.ps1

    # Ejecutar el túnel IAF explícitamente
    .\cloudflare_tunnel.ps1 -Mode run

    # Túnel rápido de prueba
    .\cloudflare_tunnel.ps1 -Mode quick

    # Configurar un túnel nuevo desde cero
    .\cloudflare_tunnel.ps1 -Mode setup -TunnelName "mi-tunel" -Domain "iaf.midominio.com"
#>

param(
    [ValidateSet("run", "quick", "setup")]
    [string]$Mode = "run",
    [string]$TunnelName = "IAF",
    [string]$Domain = "iaf.mujerbonitauy.com"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Túnel Cloudflare para IAF (Puerto 8080)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# ============================================================================
# VERIFICACIÓN DE cloudflared
# ============================================================================
$cloudflared = Get-Command cloudflared -ErrorAction SilentlyContinue
if (-not $cloudflared) {
    Write-Host "[ERROR] cloudflared no está instalado." -ForegroundColor Red
    Write-Host ""
    Write-Host "Opciones de instalación:"
    Write-Host "  1. winget install Cloudflare.cloudflared"
    Write-Host "  2. choco install cloudflared"
    exit 1
}
Write-Host "[OK] cloudflared detectado: $($cloudflared.Source)" -ForegroundColor Green

# ============================================================================
# VERIFICACIÓN DEL SERVIDOR IAF
# ============================================================================
Write-Host ""
Write-Host "[INFO] Verificando que IAF esté corriendo en 127.0.0.1:8080..." -ForegroundColor Yellow
try {
    $test = Invoke-WebRequest -Uri "http://127.0.0.1:8080" -TimeoutSec 3 -ErrorAction Stop
    Write-Host "[OK] Puerto 8080 responde (status: $($test.StatusCode))" -ForegroundColor Green
} catch {
    Write-Host "[ADVERTENCIA] No se pudo contactar 127.0.0.1:8080: $_" -ForegroundColor Yellow
    Write-Host "            Asegúrate de que el servidor IAF esté corriendo antes de iniciar el túnel." -ForegroundColor Yellow
}

# ============================================================================
# ADVERTENCIA DE SEGURIDAD
# ============================================================================
Write-Host ""
Write-Host "╔══════════════════════════════════════════════════════════════╗" -ForegroundColor Yellow
Write-Host "║  ADVERTENCIA DE SEGURIDAD                                    ║" -ForegroundColor Yellow
Write-Host "╠══════════════════════════════════════════════════════════════╣" -ForegroundColor Yellow
Write-Host "║  Este túnel SOLO expone el puerto 8080 (requiere login).     ║" -ForegroundColor Yellow
Write-Host "║  El puerto 80 (admin sin auth) NUNCA se expone.              ║" -ForegroundColor Yellow
Write-Host "║  Asegúrate de tener contraseñas fuertes en todos los users.  ║" -ForegroundColor Yellow
Write-Host "╚══════════════════════════════════════════════════════════════╝" -ForegroundColor Yellow
Write-Host ""

# ============================================================================
# MODO RUN — Simplemente ejecuta el túnel existente
# ============================================================================
if ($Mode -eq "run") {
    Write-Host "[MODO RUN] Ejecutando túnel '$TunnelName'..." -ForegroundColor Cyan
    Write-Host "[INFO] Dominio: https://$Domain" -ForegroundColor Gray
    Write-Host "[INFO] Servicio: http://127.0.0.1:8080 (SOLO puerto 8080)" -ForegroundColor Gray
    Write-Host "[INFO] Presiona Ctrl+C para detener el túnel." -ForegroundColor Gray
    Write-Host ""
    
    # Verificar que el túnel existe
    $tunnelList = & cloudflared tunnel list 2>&1 | Out-String
    if ($tunnelList -notmatch $TunnelName) {
        Write-Host "[ERROR] El túnel '$TunnelName' no existe." -ForegroundColor Red
        Write-Host "        Créalo primero con: .\cloudflare_tunnel.ps1 -Mode setup" -ForegroundColor Red
        Write-Host "        O usa el modo rápido: .\cloudflare_tunnel.ps1 -Mode quick" -ForegroundColor Red
        exit 1
    }
    
    & cloudflared tunnel run $TunnelName
    exit 0
}

# ============================================================================
# MODO QUICK — Túnel efímero con trycloudflare.com
# ============================================================================
if ($Mode -eq "quick") {
    Write-Host "[MODO QUICK] Iniciando túnel efímero (trycloudflare.com)..." -ForegroundColor Cyan
    Write-Host "[INFO] Se generará una URL temporal del tipo https://xxx.trycloudflare.com" -ForegroundColor Gray
    Write-Host "[INFO] Presiona Ctrl+C para detener el túnel." -ForegroundColor Gray
    Write-Host ""
    
    & cloudflared tunnel --url http://127.0.0.1:8080 --no-autoupdate
    exit 0
}

# ============================================================================
# MODO SETUP — Configurar túnel desde cero
# ============================================================================
Write-Host "[MODO SETUP] Configurando túnel '$TunnelName'..." -ForegroundColor Cyan
Write-Host ""

# Verificar dominio
if ([string]::IsNullOrWhiteSpace($Domain)) {
    Write-Host "[ERROR] Debes especificar un dominio con -Domain para el modo setup." -ForegroundColor Red
    Write-Host "        Ejemplo: -Domain 'iaf.midominio.com'" -ForegroundColor Red
    exit 1
}

# Autenticación con Cloudflare
Write-Host "[PASO 1/4] Verificando autenticación con Cloudflare..." -ForegroundColor Yellow
$loginCheck = & cloudflared tunnel list 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "[INFO] No estás autenticado. Se abrirá el navegador..." -ForegroundColor Yellow
    & cloudflared tunnel login
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Falló la autenticación con Cloudflare." -ForegroundColor Red
        exit 1
    }
}

# Crear túnel (si no existe)
Write-Host "[PASO 2/4] Creando/verificando túnel '$TunnelName'..." -ForegroundColor Yellow
$tunnelList = & cloudflared tunnel list 2>&1 | Out-String
if ($tunnelList -match $TunnelName) {
    Write-Host "[OK] El túnel '$TunnelName' ya existe." -ForegroundColor Green
    # Extraer tunnel ID
    $tunnelId = ($tunnelList | Select-String -Pattern "$TunnelName\s+([a-f0-9\-]+)").Matches.Groups[1].Value
} else {
    Write-Host "[INFO] Creando nuevo túnel '$TunnelName'..." -ForegroundColor Yellow
    $createOutput = & cloudflared tunnel create $TunnelName 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] No se pudo crear el túnel: $createOutput" -ForegroundColor Red
        exit 1
    }
    Write-Host "[OK] Túnel creado exitosamente." -ForegroundColor Green
    # Extraer tunnel ID del output
    $tunnelId = ($createOutput | Select-String -Pattern "Created tunnel.*id\s+([a-f0-9\-]+)").Matches.Groups[1].Value
}

# Configurar DNS
Write-Host "[PASO 3/4] Configurando DNS: $Domain -> túnel '$TunnelName'..." -ForegroundColor Yellow
$dnsOutput = & cloudflared tunnel route dns $TunnelName $Domain 2>&1
if ($LASTEXITCODE -ne 0) {
    if ($dnsOutput -match "already exists") {
        Write-Host "[OK] El registro DNS ya existe para $Domain" -ForegroundColor Green
    } else {
        Write-Host "[ERROR] Falló la configuración DNS: $dnsOutput" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "[OK] Registro DNS creado para $Domain" -ForegroundColor Green
}

# Generar config.yml
Write-Host "[PASO 4/4] Generando archivo de configuración..." -ForegroundColor Yellow
$credentialsFile = "$env:USERPROFILE\.cloudflared\$TunnelName.json"
$configPath = "$ScriptDir\cloudflared_config.yml"

$configContent = @"
# Configuración de túnel Cloudflare para IAF
# Generado: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
# ATENCIÓN: Solo expone el puerto 8080 (autenticación requerida)

tunnel: $TunnelName
credentials-file: $credentialsFile

# Reglas de enrutamiento (ingress)
# Orden: primera coincidencia gana
ingress:
  # Ruta principal: solo puerto 8080
  - hostname: $Domain
    service: http://127.0.0.1:8080
  
  # Regla final obligatoria: rechazar todo lo demás
  - service: http_status:404
"@

Set-Content -Path $configPath -Value $configContent -Encoding UTF8
Write-Host "[OK] Configuración guardada en: $configPath" -ForegroundColor Green

# Instrucciones finales
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  CONFIGURACIÓN COMPLETA" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Túnel:     $TunnelName" 
Write-Host "Dominio:   https://$Domain"
Write-Host "Servicio:  http://127.0.0.1:8080 (SOLO puerto 8080)"
Write-Host ""

Write-Host "Para ejecutar el túnel ahora:" -ForegroundColor Cyan
Write-Host "  .\cloudflare_tunnel.ps1" -ForegroundColor White
Write-Host "  (o simplemente: cloudflared tunnel run $TunnelName)" -ForegroundColor White
Write-Host ""
Write-Host "Para instalar como servicio de Windows (inicio automático):" -ForegroundColor Cyan
Write-Host "  cloudflared service install" -ForegroundColor White
Write-Host ""
Write-Host "ADVERTENCIA: El puerto 80 (admin local sin auth) NO está expuesto por el túnel." -ForegroundColor Yellow
Write-Host "             Accede localmente a http://localhost:80 solo desde tu red de confianza." -ForegroundColor Yellow
Write-Host ""
