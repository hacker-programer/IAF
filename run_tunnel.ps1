# run_tunnel.ps1 — Inicia el túnel Cloudflare IAF (solo puerto 8080)
# Simplemente ejecuta: .\run_tunnel.ps1
# Dominio: https://iaf.mujerbonitauy.com

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ConfigFile = Join-Path $ScriptDir "scripts\cloudflared_config.yml"

if (-not (Test-Path $ConfigFile)) {
    Write-Host "[ERROR] No se encuentra el archivo de configuración: $ConfigFile" -ForegroundColor Red
    exit 1
}

Write-Host "Iniciando túnel IAF -> iaf.mujerbonitauy.com (puerto 8080)..." -ForegroundColor Cyan
Write-Host "Config: $ConfigFile" -ForegroundColor Gray
Write-Host "Presiona Ctrl+C para detener." -ForegroundColor Gray
Write-Host ""

cloudflared tunnel --config "$ConfigFile" run IAF
