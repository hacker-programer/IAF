# ============================================================================
# scripts/sign_nonce.ps1 — Firma un nonce con la clave privada Ed25519
# ============================================================================
# Uso: .\scripts\sign_nonce.ps1 -Nonce "<nonce_en_base64>"
#      .\scripts\sign_nonce.ps1 -Nonce "<nonce>" -PrivateKey "hex_key"
#
# Este script:
# 1. Lee la clave privada de .config/admin_private.pem (o la recibe por parametro)
# 2. Llama al endpoint /api/auth/sign del servidor IAF
# 3. Devuelve la firma en base64 lista para usar en /api/auth/verify
#
# REQUISITO: El servidor IAF debe estar corriendo en localhost:8080

param(
    [Parameter(Mandatory=$true)]
    [string]$Nonce,
    
    [string]$PrivateKey,
    
    [string]$ServerUrl = "http://127.0.0.1:8080"
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

# Si no se pasa clave privada por parametro, leer del archivo .pem
if (-not $PrivateKey) {
    $pemPath = Join-Path $projectRoot ".config\admin_private.pem"
    
    if (-not (Test-Path $pemPath)) {
        Write-Host "ERROR: No se encontro el archivo de clave privada en:" -ForegroundColor Red
        Write-Host "       $pemPath" -ForegroundColor Red
        Write-Host ""
        Write-Host "  Ejecuta primero: .\scripts\generate_keys.ps1" -ForegroundColor Yellow
        Write-Host "  O pasa la clave con: -PrivateKey <hex_key>" -ForegroundColor Yellow
        exit 1
    }
    
    try {
        $pemContent = Get-Content -Path $pemPath -Raw
        # Extraer la clave hexadecimal del formato PEM
        if ($pemContent -match "-----BEGIN IAF ED25519 PRIVATE KEY-----\s*([a-fA-F0-9]{64})\s*-----END") {
            $PrivateKey = $Matches[1]
        } else {
            Write-Host "ERROR: El archivo .pem no tiene el formato esperado." -ForegroundColor Red
            exit 1
        }
    } catch {
        Write-Host "ERROR al leer el archivo .pem: $_" -ForegroundColor Red
        exit 1
    }
}

# Validar que la clave privada tenga 64 caracteres hex
if ($PrivateKey -notmatch '^[a-fA-F0-9]{64}$') {
    Write-Host "ERROR: La clave privada debe ser 64 caracteres hexadecimales." -ForegroundColor Red
    Write-Host "       La clave proporcionada tiene $($PrivateKey.Length) caracteres." -ForegroundColor Red
    exit 1
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  IAF — Firmador de Nonce Ed25519" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

try {
    Write-Host "[1/2] Enviando nonce al servidor para firmar..." -ForegroundColor Yellow
    
    $body = @{
        private_key = $PrivateKey
        nonce = $Nonce
    } | ConvertTo-Json
    
    $response = Invoke-RestMethod -Uri "$ServerUrl/api/auth/sign" -Method Post -Body $body -ContentType "application/json" -TimeoutSec 10
    
    if ($response.status -ne "ok") {
        Write-Host "ERROR: El servidor respondio con error: $($response.message)" -ForegroundColor Red
        exit 1
    }
    
    $signature = $response.signature
    
    Write-Host "       Nonce recibido:  $Nonce" -ForegroundColor DarkGray
    Write-Host "       Firma generada:  $signature" -ForegroundColor Green
    Write-Host ""
    
} catch {
    Write-Host "ERROR: No se pudo conectar al servidor en $ServerUrl" -ForegroundColor Red
    Write-Host "       Asegurate de que el servidor IAF este corriendo (cargo run)" -ForegroundColor Red
    Write-Host "       Error: $_" -ForegroundColor Red
    exit 1
}

Write-Host "[2/2] Firma lista para usar:" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Copia esta firma y pegala en el campo 'Firma (base64)' de la UI" -ForegroundColor White
Write-Host "  o usala directamente con curl:" -ForegroundColor White
Write-Host ""
Write-Host "  curl -X POST $ServerUrl/api/auth/verify -H 'Content-Type: application/json' -d '{""username"":""Fa"",""nonce"":""$Nonce"",""signature"":""$signature""}'" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  FIRMA: $signature" -ForegroundColor Green
Write-Host ""
