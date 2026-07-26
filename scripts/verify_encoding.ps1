# verify_encoding.ps1 - Verifica codificacion UTF-8 en el proyecto IAF
# Uso: .\scripts\verify_encoding.ps1 [-Fix] [-Path ruta]
param([switch]$Fix, [string]$Path = "")

if ($Path -eq "") { $Path = Split-Path -Parent $PSScriptRoot }
$root = Resolve-Path $Path
$ErrorActionPreference = "Continue"

Write-Host "============================================"
Write-Host " VERIFICADOR UTF-8 - $root"
Write-Host "============================================"

$extensions = @("*.rs","*.js","*.html","*.css","*.txt","*.json","*.md","*.toml","*.lock","*.svg","*.ps1","*.yml")
$exclude = @("target","node_modules",".git","citybound","vendor","dist","build")
$bomCount = 0; $badCount = 0; $totalCount = 0; $fixedCount = 0

function is-excluded($p) {
    foreach ($e in $exclude) { if ($p -match "\\$e\\" -or $p -match "\\$e$") { return $true } }
    return $false
}

function check-utf8($bytes) {
    $i = 0
    while ($i -lt $bytes.Length) {
        $b = $bytes[$i]
        if ($b -le 0x7F) { $i++; continue }
        if ($b -ge 0xC2 -and $b -le 0xDF) {
            if ($i+1 -ge $bytes.Length) { return $false }
            if ($bytes[$i+1] -lt 0x80 -or $bytes[$i+1] -gt 0xBF) { return $false }
            $i += 2; continue
        }
        if ($b -ge 0xE0 -and $b -le 0xEF) {
            if ($i+2 -ge $bytes.Length) { return $false }
            if ($bytes[$i+1] -lt 0x80 -or $bytes[$i+1] -gt 0xBF) { return $false }
            if ($bytes[$i+2] -lt 0x80 -or $bytes[$i+2] -gt 0xBF) { return $false }
            if ($b -eq 0xE0 -and $bytes[$i+1] -lt 0xA0) { return $false }
            if ($b -eq 0xED -and $bytes[$i+1] -gt 0x9F) { return $false }
            $i += 3; continue
        }
        if ($b -ge 0xF0 -and $b -le 0xF4) {
            if ($i+3 -ge $bytes.Length) { return $false }
            if ($bytes[$i+1] -lt 0x80 -or $bytes[$i+1] -gt 0xBF) { return $false }
            if ($bytes[$i+2] -lt 0x80 -or $bytes[$i+2] -gt 0xBF) { return $false }
            if ($bytes[$i+3] -lt 0x80 -or $bytes[$i+3] -gt 0xBF) { return $false }
            if ($b -eq 0xF0 -and $bytes[$i+1] -lt 0x90) { return $false }
            if ($b -eq 0xF4 -and $bytes[$i+1] -gt 0x8F) { return $false }
            $i += 4; continue
        }
        return $false
    }
    return $true
}

foreach ($ext in $extensions) {
    $files = Get-ChildItem -Path $root -Recurse -Filter $ext -File -ErrorAction SilentlyContinue
    foreach ($f in $files) {
        if (is-excluded $f.FullName) { continue }
        $totalCount++
        try {
            $bytes = [IO.File]::ReadAllBytes($f.FullName)
            if ($bytes.Length -eq 0) { continue }

            # BOM check
            if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
                $rel = $f.FullName.Substring($root.Length)
                Write-Host "[BOM] $rel"
                $bomCount++
                if ($Fix) {
                    $newBytes = $bytes[3..($bytes.Length-1)]
                    [IO.File]::WriteAllBytes($f.FullName, $newBytes)
                    Write-Host "  -> CORREGIDO"
                    $fixedCount++
                }
                continue
            }

            # UTF-16 check
            if ($bytes.Length -ge 2) {
                if (($bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) -or ($bytes[0] -eq 0xFE -and $bytes[1] -eq 0xFF)) {
                    $rel = $f.FullName.Substring($root.Length)
                    Write-Host "[UTF-16] $rel"
                    $badCount++
                    if ($Fix) {
                        $enc = if ($bytes[0] -eq 0xFF) { [Text.Encoding]::Unicode } else { [Text.Encoding]::BigEndianUnicode }
                        $text = [IO.File]::ReadAllText($f.FullName, $enc)
                        [IO.File]::WriteAllText($f.FullName, $text, [Text.Encoding]::UTF8)
                        Write-Host "  -> CORREGIDO"
                        $fixedCount++
                    }
                    continue
                }
            }

            # UTF-8 validity check
            if (-not (check-utf8 $bytes)) {
                $rel = $f.FullName.Substring($root.Length)
                Write-Host "[NO-UTF8] $rel"
                $badCount++
                if ($Fix) {
                    try {
                        $text = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
                        [IO.File]::WriteAllText($f.FullName, $text, [Text.Encoding]::UTF8)
                        Write-Host "  -> CORREGIDO (Latin-1 asumido)"
                        $fixedCount++
                    } catch {
                        Write-Host "  -> ERROR: No se pudo convertir"
                    }
                }
            }
        } catch {
            Write-Host "[ERROR] $($f.Name): $_"
        }
    }
}

Write-Host ""
Write-Host "=== RESUMEN ==="
Write-Host "Archivos escaneados: $totalCount"
Write-Host "Con BOM:            $bomCount"
Write-Host "No UTF-8:           $badCount"
if ($Fix) { Write-Host "Corregidos:         $fixedCount" }

if ($bomCount -eq 0 -and $badCount -eq 0) {
    Write-Host ""
    Write-Host "VERIFICACION EXITOSA - Todos los archivos UTF-8 sin BOM"
    exit 0
} else {
    Write-Host ""
    Write-Host "Hay problemas de codificacion."
    if (-not $Fix) { Write-Host "Ejecuta con -Fix para corregirlos." }
    exit 1
}
