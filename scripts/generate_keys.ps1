# ============================================================================
# scripts/generate_keys.ps1 — Genera par de claves Ed25519 y las guarda en .pem
# ============================================================================
# Uso: .\scripts\generate_keys.ps1
# 
# Este script:
# 1. Llama al endpoint /api/auth/keygen del servidor IAF
# 2. Guarda la clave PUBLICA en .config/admin_public.pem
# 3. Guarda la clave PRIVADA en .config/admin_private.pem
# 4. Muestra instrucciones para configurar users.json
#
# REQUISITO: El servidor IAF debe estar corriendo en localhost:8080

param(
    [string]$ServerUrl = "http://127.0.0.1:8080"
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir
$configDir = Join-Path $projectRoot ".config"

if (-not (Test-Path $configDir)) {
    New-Item -ItemType Directory -Force -Path $configDir | Out-Null
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  IAF — Generador de Claves Ed25519" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

try {
    Write-Host "[1/3] Solicitando par de claves al servidor..." -ForegroundColor Yellow
    $response = Invoke-RestMethod -Uri "$ServerUrl/api/auth/keygen" -Method Get -TimeoutSec 10
    
    if ($response.status -ne "ok") {
        Write-Host "ERROR: El servidor respondio con error: $($response.message)" -ForegroundColor Red
        exit 1
    }
    
    $publicKey = $response.public_key
    $privateKey = $response.private_key
    
    Write-Host "       Clave publica:  $publicKey" -ForegroundColor Green
    Write-Host "       Clave privada:  $privateKey" -ForegroundColor Green
    Write-Host ""
    
} catch {
    Write-Host "ERROR: No se pudo conectar al servidor en $ServerUrl" -ForegroundColor Red
    Write-Host "       Asegurate de que el servidor IAF este corriendo (cargo run)" -ForegroundColor Red
    Write-Host "       Error: $_" -ForegroundColor Red
    exit 1
}

try {
    Write-Host "[2/3] Guardando claves en archivos .pem..." -ForegroundColor Yellow
    
    $publicPemPath = Join-Path $configDir "admin_public.pem"
    $privatePemPath = Join-Path $configDir "admin_private.pem"
    
    $publicPemContent = @"
-----BEGIN IAF ED25519 PUBLIC KEY-----
$publicKey
-----END IAF ED25519 PUBLIC KEY-----
"@
    
    $privatePemContent = @"
-----BEGIN IAF ED25519 PRIVATE KEY-----
$privateKey
-----END IAF ED25519 PRIVATE KEY-----
"@
    
    Set-Content -Path $publicPemPath -Value $publicPemContent -Encoding UTF8
    Set-Content -Path $privatePemPath -Value $privatePemContent -Encoding UTF8
    
    Write-Host "       Clave publica ->  $publicPemPath" -ForegroundColor Green
    Write-Host "       Clave privada -> $privatePemPath" -ForegroundColor Green
    Write-Host ""
    
} catch {
    Write-Host "ERROR al guardar archivos: $_" -ForegroundColor Red
    exit 1
}

Write-Host "[3/3] Instrucciones finales:" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Listo! Claves generadas correctamente." -ForegroundColor Green
Write-Host ""
Write-Host "  Para configurar tu cuenta admin:" -ForegroundColor White
Write-Host "     1. Copia la clave publica de $publicPemPath" -ForegroundColor White
Write-Host "     2. Pegala en .config/users.json.template" -ForegroundColor White
Write-Host "     3. Renombra .config/users.json.template -> users.json" -ForegroundColor White
Write-Host "     4. La clave privada NUNCA se comparte ni se sube a git" -ForegroundColor White
Write-Host ""
Write-Host "  Para firmar nonces (autenticacion admin):" -ForegroundColor White
Write-Host "     .\scripts\sign_nonce.ps1 -Nonce '<nonce_en_base64>'" -ForegroundColor White
Write-Host ""
Write-Host "  ADVERTENCIA: La clave privada es SAGRADA." -ForegroundColor Red
Write-Host "     Si la perdes, perdes el acceso a tu cuenta admin." -ForegroundColor Red
Write-Host "     Si alguien la obtiene, puede hacerse pasar por vos." -ForegroundColor Red
Write-Host ""
