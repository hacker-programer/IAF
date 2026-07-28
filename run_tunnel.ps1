# run_tunnel.ps1 — Inicia el túnel Cloudflare IAF (solo puerto 8080)
# Simplemente ejecuta: .\run_tunnel.ps1
# Dominio: https://iaf.mujerbonitauy.com

Write-Host "Iniciando túnel IAF -> iaf.mujerbonitauy.com (puerto 8080)..." -ForegroundColor Cyan
Write-Host "Presiona Ctrl+C para detener." -ForegroundColor Gray
cloudflared tunnel run IAF
