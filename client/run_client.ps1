# Script de PowerShell para ejecutar el Servidor Agent-First (IAF) con auto-reinicio y diagnóstico de errores
Clear-Host
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   Iniciador Resiliente del Cleinte Agent-First (IAF)    " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$env:CARGO_TARGET_DIR = "C:\Users\Fa\AppData\Local\Temp\cargo-client-target"

while ($true) {
    Write-Host "`n[$(Get-Date -Format 'HH:mm:ss')] Iniciando Cliente (cargo run --release)..." -ForegroundColor Green
    
    # Ejecutar el binario
    cargo run --release
    
    # Capturar el código de salida del proceso anterior
    $exitCode = $LASTEXITCODE
    
    Write-Host "`n[$(Get-Date -Format 'HH:mm:ss')] El proceso del cliente ha terminado." -ForegroundColor Yellow
    Write-Host "Código de salida (Exit Code): $exitCode" -ForegroundColor Yellow
    
    if ($exitCode -eq 0) {
        Write-Host "El cliente terminó limpiamente (Exit Code 0). Reiniciando automáticamente en 2 segundos..." -ForegroundColor Cyan
        Start-Sleep -Seconds 2
    } else {
        # Si terminó con código de error o Ctrl+C
        Write-Host "El cliente terminó con un código de error/interrupción ($exitCode)." -ForegroundColor Red
        return
    }
}
