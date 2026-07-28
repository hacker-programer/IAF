<# 
.SYNOPSIS
    Configura y ejecuta un túnel Cloudflare para exponer el puerto 8080 de IAF.
    SOLO expone el puerto 8080 (autenticación requerida), NUNCA el puerto 80 (admin local).

.DESCRIPCIÓN
    Este script tiene dos modos:
    - MODO RÁPIDO:    Crea un túnel efímero con URL de trycloudflare.com (para pruebas)
    - MODO PERMANENTE: Configura un túnel nombrado con dominio propio (para producción)
    
    El puerto 80 NUNCA se expone. Solo se tunela 127.0.0.1:8080.

.PARAMETER Mode
    "quick" para túnel rápido, "permanent" para túnel permanente con dominio propio.

.PARAMETER TunnelName
    Nombre del túnel (solo modo permanente). Default: "iaf-tunnel"

.PARAMETER Domain
    Dominio configurado en Cloudflare (solo modo permanente). Ej: "iaf.midominio.com"

.EJEMPLO
    # Túnel rápido (pruebas)
    .\cloudflare_tunnel.ps1 -Mode quick

    # Túnel permanente
    .\cloudflare_tunnel.ps1 -Mode permanent -TunnelName "iaf-prod" -Domain "iaf.midominio.com"
#>

param(
    [ValidateSet("quick", "permanent")]
    [string]$Mode = "quick",
    [string]$TunnelName = "iaf-tunnel",
    [string]$Domain = ""
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path "$ScriptDir\.."

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
    Write-Host "  2. Descargar de: https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/"
    Write-Host "  3. Chocolatey: choco install cloudflared"
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
# MODO RÁPIDO
# ============================================================================
if ($Mode -eq "quick") {
    Write-Host "[MODO RÁPIDO] Iniciando túnel efímero (trycloudflare.com)..." -ForegroundColor Cyan
    Write-Host "[INFO] Se generará una URL temporal del tipo https://xxx.trycloudflare.com" -ForegroundColor Gray
    Write-Host "[INFO] Presiona Ctrl+C para detener el túnel." -ForegroundColor Gray
    Write-Host ""
    
    # Ejecutar cloudflared tunnel --url
    & cloudflared tunnel --url http://127.0.0.1:8080 --no-autoupdate
    
    exit 0
}

# ============================================================================
# MODO PERMANENTE
# ============================================================================
Write-Host "[MODO PERMANENTE] Configurando túnel '$TunnelName'..." -ForegroundColor Cyan
Write-Host ""

# Verificar dominio
if ([string]::IsNullOrWhiteSpace($Domain)) {
    Write-Host "[ERROR] Debes especificar un dominio con -Domain para el modo permanente." -ForegroundColor Red
    Write-Host "        Ejemplo: -Domain 'iaf.midominio.com'" -ForegroundColor Red
    exit 1
}

# Autenticación con Cloudflare (si no está autenticado)
Write-Host "[PASO 1/4] Verificando autenticación con Cloudflare..." -ForegroundColor Yellow
$loginCheck = & cloudflared tunnel list 2>&1
if ($LASTEXITCODE -ne 0 -or $loginCheck -match "You have no tunnels") {
    # Puede que no esté autenticado
    Write-Host "[INFO] No hay túneles o no estás autenticado." -ForegroundColor Yellow
    Write-Host "[INFO] Se abrirá el navegador para autenticarte con Cloudflare..." -ForegroundColor Yellow
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
Write-Host "  cloudflared tunnel run $TunnelName" -ForegroundColor White
Write-Host ""
Write-Host "Para instalar como servicio de Windows (inicio automático):" -ForegroundColor Cyan
Write-Host "  cloudflared service install" -ForegroundColor White
Write-Host ""
Write-Host "Para ver el estado de los túneles:" -ForegroundColor Cyan
Write-Host "  cloudflared tunnel list" -ForegroundColor White
Write-Host ""
Write-Host "ADVERTENCIA: El puerto 80 (admin local sin auth) NO está expuesto por el túnel." -ForegroundColor Yellow
Write-Host "             Accede localmente a http://localhost:80 solo desde tu red de confianza." -ForegroundColor Yellow
Write-Host ""
