# ============================================================================
# verify_encoding.ps1 — Verifica y corrige codificación UTF-8 en todo el proyecto
# ============================================================================
# Uso:
#   .\scripts\verify_encoding.ps1              # Solo verificar (reportar)
#   .\scripts\verify_encoding.ps1 -Fix         # Verificar y corregir automáticamente
#   .\scripts\verify_encoding.ps1 -Fix -Path "C:\otro\proyecto"  # Otro proyecto
# ============================================================================

param(
    [switch]$Fix,
    [string]$Path = $PSScriptRoot + "\.."
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$root = Resolve-Path $Path
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  VERIFICADOR DE CODIFICACIÓN UTF-8" -ForegroundColor Cyan
Write-Host "  Proyecto: $root" -ForegroundColor Cyan
Write-Host "  Modo: $(if ($Fix) { 'CORREGIR' } else { 'SOLO REPORTAR' })" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Extensiones de archivos de texto a verificar
$textExtensions = @(
    "*.rs", "*.js", "*.ts", "*.jsx", "*.tsx",
    "*.html", "*.htm", "*.css", "*.scss", "*.less",
    "*.txt", "*.json", "*.xml", "*.yml", "*.yaml",
    "*.md", "*.mdx", "*.toml", "*.lock",
    "*.svg", "*.ps1", "*.psm1", "*.bat", "*.sh",
    "*.py", "*.rb", "*.php", "*.java", "*.kt",
    "*.c", "*.cpp", "*.h", "*.hpp",
    "*.csv", "*.tsv", "*.env", "*.gitignore",
    "*.config", "*.ini", "*.cfg",
    "*.sql", "*.graphql", "*.proto"
)

# Directorios a excluir
$excludeDirs = @(
    "target", "node_modules", ".git", "vendor",
    "dist", "build", "__pycache__", ".venv",
    "Debug", "Release", "bin", "obj"
)

# Estadísticas
$stats = @{
    TotalFiles = 0
    BomFiles = @()
    NonUtf8Files = @()
    FixedBom = 0
    FixedNonUtf8 = 0
    Errors = @()
}

Write-Host "[1/3] Escaneando archivos..." -ForegroundColor Yellow

foreach ($ext in $textExtensions) {
    $files = Get-ChildItem -Path $root -Recurse -Filter $ext -File -ErrorAction SilentlyContinue
    foreach ($f in $files) {
        $fullPath = $f.FullName
        $skip = $false
        foreach ($excl in $excludeDirs) {
            if ($fullPath -match "\\$excl\\" -or $fullPath -match "\\$excl`$") {
                $skip = $true
                break
            }
        }
        if ($skip) { continue }
        
        $stats.TotalFiles++
        
        try {
            $bytes = [IO.File]::ReadAllBytes($fullPath)
            if ($bytes.Length -eq 0) { continue }
            
            # ---- DETECCIÓN DE BOM ----
            $hasBOM = ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF)
            
            # ---- DETECCIÓN DE UTF-16 LE/BE ----
            $isUtf16LE = ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE)
            $isUtf16BE = ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFE -and $bytes[1] -eq 0xFF)
            
            if ($isUtf16LE -or $isUtf16BE) {
                $enc = if ($isUtf16LE) { "UTF-16 LE" } else { "UTF-16 BE" }
                $stats.NonUtf8Files += "$($fullPath.Substring($root.Length)) [$enc]"
                
                if ($Fix) {
                    $text = [IO.File]::ReadAllText($fullPath, [Text.Encoding]::Unicode)
                    $utf8Bytes = [Text.Encoding]::UTF8.GetBytes($text)
                    [IO.File]::WriteAllBytes($fullPath, $utf8Bytes)
                    $stats.FixedNonUtf8++
                    Write-Host "  FIX UTF-16→UTF-8: $($f.Name)" -ForegroundColor Green
                }
                continue
            }
            
            if ($hasBOM) {
                $relPath = $fullPath.Substring($root.Length)
                $stats.BomFiles += $relPath
                
                if ($Fix) {
                    $newBytes = $bytes[3..($bytes.Length - 1)]
                    [IO.File]::WriteAllBytes($fullPath, $newBytes)
                    $stats.FixedBom++
                    Write-Host "  FIX BOM: $($f.Name)" -ForegroundColor Green
                }
                continue
            }
            
            # ---- VALIDACIÓN UTF-8 ESTRICTA ----
            # Verificar que todos los bytes forman secuencias UTF-8 válidas
            $i = 0
            $invalidUtf8 = $false
            while ($i -lt $bytes.Length) {
                $b = $bytes[$i]
                if ($b -le 0x7F) {
                    $i++
                } elseif ($b -ge 0xC2 -and $b -le 0xDF) {
                    if ($i + 1 -ge $bytes.Length) { $invalidUtf8 = $true; break }
                    $b2 = $bytes[$i + 1]
                    if ($b2 -lt 0x80 -or $b2 -gt 0xBF) { $invalidUtf8 = $true; break }
                    $i += 2
                } elseif ($b -ge 0xE0 -and $b -le 0xEF) {
                    if ($i + 2 -ge $bytes.Length) { $invalidUtf8 = $true; break }
                    $b2 = $bytes[$i + 1]
                    $b3 = $bytes[$i + 2]
                    if ($b2 -lt 0x80 -or $b2 -gt 0xBF) { $invalidUtf8 = $true; break }
                    if ($b3 -lt 0x80 -or $b3 -gt 0xBF) { $invalidUtf8 = $true; break }
                    # Check overlong sequences
                    if ($b -eq 0xE0 -and $b2 -lt 0xA0) { $invalidUtf8 = $true; break }
                    # Check surrogate pairs (invalid in UTF-8)
                    if ($b -eq 0xED -and $b2 -gt 0x9F) { $invalidUtf8 = $true; break }
                    $i += 3
                } elseif ($b -ge 0xF0 -and $b -le 0xF4) {
                    if ($i + 3 -ge $bytes.Length) { $invalidUtf8 = $true; break }
                    $b2 = $bytes[$i + 1]
                    $b3 = $bytes[$i + 2]
                    $b4 = $bytes[$i + 3]
                    if ($b2 -lt 0x80 -or $b2 -gt 0xBF) { $invalidUtf8 = $true; break }
                    if ($b3 -lt 0x80 -or $b3 -gt 0xBF) { $invalidUtf8 = $true; break }
                    if ($b4 -lt 0x80 -or $b4 -gt 0xBF) { $invalidUtf8 = $true; break }
                    if ($b -eq 0xF0 -and $b2 -lt 0x90) { $invalidUtf8 = $true; break }
                    if ($b -eq 0xF4 -and $b2 -gt 0x8F) { $invalidUtf8 = $true; break }
                    $i += 4
                } else {
                    $invalidUtf8 = $true
                    break
                }
            }
            
            if ($invalidUtf8) {
                $relPath = $fullPath.Substring($root.Length)
                $stats.NonUtf8Files += "$relPath [bytes inválidos en posición ~$i]"
                
                if ($Fix) {
                    # Intentar detectar si es Latin-1/Windows-1252 y convertirlo
                    try {
                        $text = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
                        $utf8Bytes = [Text.Encoding]::UTF8.GetBytes($text)
                        [IO.File]::WriteAllBytes($fullPath, $utf8Bytes)
                        $stats.FixedNonUtf8++
                        Write-Host "  FIX Latin-1→UTF-8: $($f.Name)" -ForegroundColor Green
                    } catch {
                        $stats.Errors += "No se pudo convertir: $relPath"
                    }
                }
            }
            
        } catch {
            $stats.Errors += "Error leyendo $($f.Name): $_"
        }
    }
}

Write-Host ""
Write-Host "[2/3] Resultados del escaneo:" -ForegroundColor Yellow
Write-Host "  Total archivos de texto escaneados: $($stats.TotalFiles)" -ForegroundColor White
Write-Host "  Archivos con BOM encontrados:      $($stats.BomFiles.Count)" -ForegroundColor $(if ($stats.BomFiles.Count -gt 0) { "Red" } else { "Green" })
Write-Host "  Archivos no UTF-8:                 $($stats.NonUtf8Files.Count)" -ForegroundColor $(if ($stats.NonUtf8Files.Count -gt 0) { "Red" } else { "Green" })
Write-Host "  Errores:                           $($stats.Errors.Count)" -ForegroundColor $(if ($stats.Errors.Count -gt 0) { "Red" } else { "Green" })

if ($stats.BomFiles.Count -gt 0) {
    Write-Host "`n  Archivos con BOM:" -ForegroundColor Red
    foreach ($f in $stats.BomFiles) { Write-Host "    $f" -ForegroundColor Red }
}

if ($stats.NonUtf8Files.Count -gt 0) {
    Write-Host "`n  Archivos no UTF-8:" -ForegroundColor Red
    foreach ($f in $stats.NonUtf8Files) { Write-Host "    $f" -ForegroundColor Red }
}

if ($stats.Errors.Count -gt 0) {
    Write-Host "`n  Errores:" -ForegroundColor Red
    foreach ($e in $stats.Errors) { Write-Host "    $e" -ForegroundColor Red }
}

if ($Fix) {
    Write-Host ""
    Write-Host "[3/3] Correcciones aplicadas:" -ForegroundColor Yellow
    Write-Host "  BOMs eliminados:  $($stats.FixedBom)" -ForegroundColor Green
    Write-Host "  No UTF-8 a UTF-8: $($stats.FixedNonUtf8)" -ForegroundColor Green
    Write-Host "  Total corregidos: $($stats.FixedBom + $stats.FixedNonUtf8)" -ForegroundColor Green
}

Write-Host ""
if ($stats.BomFiles.Count -eq 0 -and $stats.NonUtf8Files.Count -eq 0 -and $stats.Errors.Count -eq 0) {
    Write-Host "============================================" -ForegroundColor Green
    Write-Host "  ✓ VERIFICACIÓN EXITOSA" -ForegroundColor Green
    Write-Host "  Todos los archivos son UTF-8 válidos sin BOM" -ForegroundColor Green
    Write-Host "============================================" -ForegroundColor Green
    exit 0
} else {
    Write-Host "============================================" -ForegroundColor Red
    Write-Host "  ✗ SE ENCONTRARON PROBLEMAS DE CODIFICACIÓN" -ForegroundColor Red
    if (-not $Fix) {
        Write-Host "  Ejecuta con -Fix para corregirlos automáticamente" -ForegroundColor Yellow
    }
    Write-Host "============================================" -ForegroundColor Red
    exit 1
}
