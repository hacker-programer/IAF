# ============================================================================
# IAF Capacitor — setup_capacitor.ps1
# ============================================================================
# Script para inicializar la app Android con Capacitor.
# 
# PREREQUISITOS:
#   - Node.js 18+ instalado
#   - Android Studio instalado (para compilar APK)
#   - Java JDK 17+ (para Gradle)
#
# USO:
#   .\capacitor\setup_capacitor.ps1
#   .\capacitor\setup_capacitor.ps1 -ServerUrl "http://192.168.1.50:8080"
# ============================================================================

param(
    [string]$ServerUrl = ""
)

$ErrorActionPreference = "Stop"
$capacitorDir = Join-Path $PSScriptRoot ""

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  IAF Capacitor — Setup Android App" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# 1. Instalar dependencias npm
Write-Host "`n[1/4] Instalando dependencias npm..." -ForegroundColor Green
Push-Location $capacitorDir
npm install
Pop-Location

# 2. Inicializar Capacitor
Write-Host "`n[2/4] Inicializando proyecto Capacitor..." -ForegroundColor Green
Push-Location $capacitorDir

# Verificar si ya existe capacitor.config.ts
if (-not (Test-Path "capacitor.config.ts")) {
    Write-Host "ERROR: capacitor.config.ts no encontrado en $capacitorDir" -ForegroundColor Red
    Pop-Location
    exit 1
}

# Inicializar (crea ios/ y android/ si no existen)
npx cap init IAF com.iaf.app --web-dir=../public 2>&1 | Out-Null
Write-Host "  ✓ Proyecto Capacitor inicializado." -ForegroundColor Green

# 3. Agregar plataforma Android
Write-Host "`n[3/4] Agregando plataforma Android..." -ForegroundColor Green
npx cap add android 2>&1
Write-Host "  ✓ Plataforma Android agregada." -ForegroundColor Green

# 4. Configurar server URL si se especificó
if ($ServerUrl) {
    Write-Host "`n[4/4] Configurando server URL: $ServerUrl" -ForegroundColor Green
    
    # Modificar capacitor.config.ts para usar server URL
    $configPath = Join-Path $capacitorDir "capacitor.config.ts"
    $config = Get-Content $configPath -Raw
    
    # Reemplazar la URL comentada con la URL real
    $config = $config -replace "// url: 'http://192.168.1.X:8080',", "url: '$ServerUrl',"
    $config = $config -replace "// hostname: 'iaf-local',", "hostname: 'iaf-local',"
    
    Set-Content -Path $configPath -Value $config
    Write-Host "  ✓ Server URL configurada en capacitor.config.ts" -ForegroundColor Green
}

# 5. Sincronizar assets web con Android
Write-Host "`nSincronizando assets web con Android..." -ForegroundColor Green
npx cap sync
Write-Host "  ✓ Assets sincronizados." -ForegroundColor Green

Pop-Location

Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host "  SETUP COMPLETADO" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Para abrir en Android Studio:" -ForegroundColor Yellow
Write-Host "  cd capacitor" -ForegroundColor White
Write-Host "  npx cap open android" -ForegroundColor White
Write-Host ""
Write-Host "Para compilar APK:" -ForegroundColor Yellow
Write-Host "  cd capacitor/android" -ForegroundColor White
Write-Host "  ./gradlew assembleDebug" -ForegroundColor White
Write-Host "  # APK en: capacitor/android/app/build/outputs/apk/debug/app-debug.apk" -ForegroundColor White
Write-Host ""
Write-Host "NOTA: La app Android depende de un cliente Electron en la PC" -ForegroundColor DarkYellow
Write-Host "del usuario para ejecutar PowerShell, git y cargo. Sin el cliente" -ForegroundColor DarkYellow
Write-Host "Electron conectado, solo funcionan comandos de archivos básicos." -ForegroundColor DarkYellow
