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
$capacitorDir = $PSScriptRoot
$projectRoot = Split-Path $capacitorDir -Parent

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  IAF Capacitor — Setup Android App" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# ============================================================================
# 1. Instalar dependencias npm
# ============================================================================
Write-Host "`n[1/5] Instalando dependencias npm..." -ForegroundColor Green
Push-Location $capacitorDir
npm install
Pop-Location

# ============================================================================
# 2. Inicializar Capacitor
# ============================================================================
Write-Host "`n[2/5] Inicializando proyecto Capacitor..." -ForegroundColor Green
Push-Location $capacitorDir

if (-not (Test-Path "capacitor.config.ts")) {
    Write-Host "ERROR: capacitor.config.ts no encontrado en $capacitorDir" -ForegroundColor Red
    Pop-Location
    exit 1
}

npx cap init IAF com.iaf.app --web-dir=../public 2>&1 | Out-Null
Write-Host "  ✓ Proyecto Capacitor inicializado." -ForegroundColor Green

# ============================================================================
# 3. Agregar plataforma Android
# ============================================================================
Write-Host "`n[3/5] Agregando plataforma Android..." -ForegroundColor Green
npx cap add android 2>&1
Write-Host "  ✓ Plataforma Android agregada." -ForegroundColor Green

# ============================================================================
# 3.5. Instalar plugin ShellExecutor (ejecución de comandos en Android)
# ============================================================================
Write-Host "`n[3.5/5] Instalando plugin ShellExecutor..." -ForegroundColor Green

$pluginSrcDir = Join-Path $capacitorDir "android-plugins" "src" "main" "java" "com" "iaf" "plugins"
$androidSrcDir = Join-Path $capacitorDir "android" "app" "src" "main" "java" "com" "iaf" "plugins"

if (Test-Path $pluginSrcDir) {
    New-Item -ItemType Directory -Force -Path $androidSrcDir | Out-Null
    Copy-Item -Force (Join-Path $pluginSrcDir "ShellExecutorPlugin.java") $androidSrcDir
    Write-Host "  ✓ ShellExecutorPlugin.java copiado a android/app/src/main/java/com/iaf/plugins/" -ForegroundColor Green

    # Registrar el plugin en MainActivity.java
    $mainActivityPath = Join-Path $capacitorDir "android" "app" "src" "main" "java" "com" "iaf" "app" "MainActivity.java"
    if (Test-Path $mainActivityPath) {
        $mainActivity = Get-Content $mainActivityPath -Raw
        # Verificar si ya está registrado
        if ($mainActivity -notmatch "ShellExecutorPlugin") {
            # Agregar el import
            $importLine = "import com.iaf.plugins.ShellExecutorPlugin;"
            $mainActivity = $mainActivity -replace "(package com\.iaf\.app;)", "`$1`n`n$importLine"
            # Agregar el registro en onCreate o en la lista de plugins
            # Capacitor 6 usa annotations, así que solo necesitamos el import
            Set-Content -Path $mainActivityPath -Value $mainActivity
            Write-Host "  ✓ ShellExecutorPlugin registrado en MainActivity.java" -ForegroundColor Green
        } else {
            Write-Host "  ✓ ShellExecutorPlugin ya estaba registrado." -ForegroundColor Green
        }
    }
} else {
    Write-Host "  ⚠ No se encontró android-plugins/. El plugin ShellExecutor no se instalará." -ForegroundColor Yellow
    Write-Host "    (La app funcionará pero sin ejecución local de comandos shell)" -ForegroundColor Yellow
}

# Copiar la interfaz TypeScript del plugin a la carpeta public para que app.js la use
$tsPluginSrc = Join-Path $capacitorDir "src" "plugins" "shell-executor.ts"
$publicPluginsDir = Join-Path $projectRoot "public" "plugins"
if (Test-Path $tsPluginSrc) {
    New-Item -ItemType Directory -Force -Path $publicPluginsDir | Out-Null
    Copy-Item -Force $tsPluginSrc $publicPluginsDir
    Write-Host "  ✓ shell-executor.ts copiado a public/plugins/" -ForegroundColor Green
}

# ============================================================================
# 4. Configurar server URL si se especificó
# ============================================================================
if ($ServerUrl) {
    Write-Host "`n[4/5] Configurando server URL: $ServerUrl" -ForegroundColor Green
    
    $configPath = Join-Path $capacitorDir "capacitor.config.ts"
    $config = Get-Content $configPath -Raw
    
    # Agregar server URL
    if ($config -match "// server:") {
        $config = $config -replace "// server:", "server:"
    }
    if ($config -match "// url:") {
        $config = $config -replace "// url:.*", "url: '$ServerUrl',"
    } else {
        # Agregar server block si no existe
        $serverBlock = @"
  server: {
    url: '$ServerUrl',
    cleartext: true,
  },
"@
        $config = $config -replace "(plugins: \{)", "$serverBlock`n  `$1"
    }
    
    Set-Content -Path $configPath -Value $config
    Write-Host "  ✓ Server URL configurada." -ForegroundColor Green
} else {
    Write-Host "`n[4/5] Sin server URL remota. La app cargará assets locales desde public/." -ForegroundColor Yellow
}

# ============================================================================
# 5. Sincronizar assets web con Android
# ============================================================================
Write-Host "`n[5/5] Sincronizando assets web con Android..." -ForegroundColor Green
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
Write-Host "CAPACIDADES DE LA APP ANDROID:" -ForegroundColor Cyan
Write-Host "  ✅ ShellExecutor: comandos shell nativos (ls, cat, grep, find, curl...)" -ForegroundColor Green
Write-Host "  ✅ Filesystem: leer/escribir archivos locales" -ForegroundColor Green
Write-Host "  ✅ Funciona como cliente Capacitor conectado al servidor IAF" -ForegroundColor Green
Write-Host ""
Write-Host "LIMITACIONES vs Electron (Windows):" -ForegroundColor DarkYellow
Write-Host "  ❌ Sin PowerShell (usa /system/bin/sh en su lugar)" -ForegroundColor DarkYellow
Write-Host "  ❌ Sin cargo/git/rustc (instalar Termux para desarrollo completo)" -ForegroundColor DarkYellow
Write-Host "  ⚠ Para desarrollo Rust en Android: instalar Termux + pkg install rust git" -ForegroundColor Yellow
Write-Host ""
Write-Host "USO DE LA APP:" -ForegroundColor Cyan
Write-Host "  1. Abrí la app IAF en Android" -ForegroundColor White
Write-Host "  2. Conectate a tu servidor IAF (http://TU_IP:8080)" -ForegroundColor White
Write-Host "  3. Si sos admin, usá puerto 80 para acceso directo sin cliente" -ForegroundColor White
Write-Host "  4. Si NO sos admin, necesitás un cliente Electron en PC para PowerShell/cargo" -ForegroundColor White
Write-Host "     O usá los comandos shell básicos disponibles en Android" -ForegroundColor White
