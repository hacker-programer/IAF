#!/usr/bin/env python3
"""Fix 3: BUG-001 - PDF/DOCX support in read_file."""
import sys

with open('src/agent.rs', 'rb') as f:
    data = f.read()

LE = b'\r\n'
orig_bal = data.count(b'{') - data.count(b'}')
print(f'Original balance: {orig_bal}')

# Find read_file handler
old3 = b'                            match fs::read_to_string(&full_path) {'
if old3 not in data:
    print('[FAIL] read_file match pattern not found')
    sys.exit(1)

# Replacement: if/else chain for pdf, docx, else text
new3 = (
    b'                            // Detectar extension para formatos especiales (BUG-001)' + LE +
    b'                            let extension = full_path' + LE +
    b'                                .extension()' + LE +
    b'                                .and_then(|e| e.to_str())' + LE +
    b'                                .unwrap_or("")' + LE +
    b'                                .to_lowercase();' + LE +
    LE +
    b'                            if extension == "pdf" {' + LE +
    b'                                let pdf_path_str = full_path.to_string_lossy().to_string();' + LE +
    b'                                match std::process::Command::new("pdftotext")' + LE +
    b'                                    .args(["-layout", &pdf_path_str, "-"])' + LE +
    b'                                    .output()' + LE +
    b'                                {' + LE +
    b'                                    Ok(out) if out.status.success() => {' + LE +
    b'                                        let text = String::from_utf8_lossy(&out.stdout).to_string();' + LE +
    b'                                        if text.trim().is_empty() {' + LE +
    b'                                            "El PDF fue procesado pero no contiene texto extraible (puede ser escaneado). Prueba con analyze_images para OCR.".to_string()' + LE +
    b'                                        } else {' + LE +
    b'                                            format!("[PDF extraido: {}]\\n\\n{}", rel_path, text)' + LE +
    b'                                        }' + LE +
    b'                                    }' + LE +
    b'                                    _ => "No se pudo extraer texto del PDF. Instala pdftotext (poppler-utils) o PyPDF2 (pip install PyPDF2). Como alternativa, usa analyze_images.".to_string()' + LE +
    b'                                }' + LE +
    b'                            } else if extension == "docx" {' + LE +
    b'                                let docx_path_str = full_path.to_string_lossy().to_string();' + LE +
    b'                                let ps_script = std::format!("Add-Type -As System.IO.Compression.FileSystem; $z=[IO.Compression.ZipFile]::OpenRead(\\'{}\\'); $e=$z.GetEntry(\\'word/document.xml\\'); if($e){{$s=$e.Open();$r=[IO.StreamReader]::new($s);$x=$r.ReadToEnd();$r.Close();$s.Close();$x -replace \\'<[^>]+>\\',\\'\\'}};$z.Dispose()", docx_path_str.replace("\\'","\\'\\'"));' + LE +
    b'                                match std::process::Command::new("powershell")' + LE +
    b'                                    .args(["-NoProfile", "-Command", &ps_script])' + LE +
    b'                                    .output()' + LE +
    b'                                {' + LE +
    b'                                    Ok(out) if out.status.success() => {' + LE +
    b'                                        let text = String::from_utf8_lossy(&out.stdout).to_string();' + LE +
    b'                                        if text.trim().is_empty() {' + LE +
    b'                                            "El DOCX fue leido pero no contiene texto extraible. Instala python-docx (pip install python-docx) para mejor soporte.".to_string()' + LE +
    b'                                        } else {' + LE +
    b'                                            format!("[DOCX extraido: {}]\\n\\n{}", rel_path, text)' + LE +
    b'                                        }' + LE +
    b'                                    }' + LE +
    b'                                    _ => "No se pudo extraer texto del DOCX. Instala python-docx (pip install python-docx).".to_string()' + LE +
    b'                                }' + LE +
    b'                            } else {' + LE +
    b'                                match fs::read_to_string(&full_path) {'
)

data = data.replace(old3, new3)
print('[OK] Fix 3: read_file reemplazado')

# CRITICAL: Close the else { match ... } } block
# Original:  } else { "No hay ningun proyecto..."
# We need:   } } else { "No hay ningun proyecto..."
# Find the closing pattern after Err handler
target = b'} else {\r\n                            "No hay ning'
idx = data.find(target)
if idx > 0:
    data = data[:idx] + b'                        }\r\n' + data[idx:]
    print('[OK] Fix 3: cierre else agregado')
else:
    print('[ERROR] Fix 3: no se encontro el patron de cierre')
    print('Buscando alternativas...')
    alt = b'"No hay ning'
    idx_alt = data.find(alt)
    if idx_alt > 0:
        before = data[idx_alt-300:idx_alt]
        last_else = before.rfind(b'} else {')
        if last_else >= 0:
            insert_pos = idx_alt - 300 + last_else
            data = data[:insert_pos] + b'                        }\r\n' + data[insert_pos:]
            print('[OK] Fix 3: cierre alternativo aplicado')
        else:
            print('[ERROR] No se encontro } else { antes de "No hay ning"')
    else:
        print('[ERROR] "No hay ning" no encontrado')

bal = data.count(b'{') - data.count(b'}')
print(f'Final balance: {bal} (original: {orig_bal})')

if bal != orig_bal:
    print(f'[ERROR] Balance ALTERADO: {orig_bal} -> {bal}. Archivo NO escrito.')
    sys.exit(1)

with open('src/agent.rs', 'wb') as f:
    f.write(data)
print('[DONE] Fix 3 aplicado correctamente')
