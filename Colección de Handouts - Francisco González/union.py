from pathlib import Path
import clipboard
import pytesseract
from PIL import Image
import pypdf
import docx

def formatear_estructura(elemento, nivel=1) -> str:
    ind_base = " " * (nivel * 4)
    ind_contenido = " " * ((nivel + 1) * 4)
    
    if isinstance(elemento, list):
        if not elemento:
            return "[]"
        lineas = []
        for item in elemento:
            lineas.append(formatear_estructura(item, nivel + 1))
        return "[\n" + ",\n".join(lineas) + f"\n{ind_base}]"
        
    elif isinstance(elemento, dict):
        if elemento.get("type") == "file":
            # Si el contenido es None o vacío, evitamos que .splitlines() falle
            contenido_seguro = elemento.get("content") or ""
            lineas_codigo = contenido_seguro.splitlines()
            
            codigo_identado = f"\n{ind_contenido + ' ' * 4}".join(lineas_codigo)
            
            return (
                f"{ind_base}{{\n"
                f"{ind_contenido}\"type\": \"file\",\n"
                f"{ind_contenido}\"name\": \"{elemento['name']}\",\n"
                f"{ind_contenido}\"content\": \"\"\"\n{ind_contenido + ' ' * 4}{codigo_identado}\"\"\"\n"
                f"{ind_base}}}"
            )
        else:
            contenido_carpeta = formatear_estructura(elemento["content"], nivel + 1)
            return (
                f"{ind_base}{{\n"
                f"{ind_contenido}\"type\": \"folder\",\n"
                f"{ind_contenido}\"name\": \"{elemento['name']}\",\n"
                f"{ind_contenido}\"content\": {contenido_carpeta.lstrip()}\n"
                f"{ind_base}}}"
            )

def extraer_texto(archivo: Path) -> str:
    """Intenta exprimir el texto de lo que sea que le tires."""
    ext = archivo.suffix.lower()
    texto = ""
    
    if ext == '.pdf':
        reader = pypdf.PdfReader(archivo)
        for page in reader.pages:
            extraido = page.extract_text()
            if extraido:
                texto += extraido + "\n"
    elif ext == '.docx':
        doc = docx.Document(archivo)
        for para in doc.paragraphs:
            texto += para.text + "\n"
    elif ext in ['.jpg', '.jpeg', '.png']:
        # Si no instalaste Tesseract en el OS, esto fallará miserablemente.
        # Descomenta y ajusta la siguiente línea si usas Windows y Tesseract no está en tu PATH:
        # pytesseract.pytesseract.tesseract_cmd = r'C:\Program Files\Tesseract-OCR\tesseract.exe'
        texto = pytesseract.image_to_string(Image.open(archivo))
    else:
        # Asumimos que es texto plano y rezamos
        texto = archivo.read_text(encoding="utf-8")
        
    return texto

def recur(pa: Path) -> list:
    estructura = []
    try:
        for elemento in pa.iterdir():
            if elemento.is_file():
                try:
                    contenido = extraer_texto(elemento)
                    estructura.append({
                        "type": "file",
                        "name": elemento.name,
                        "content": contenido
                    })
                except (UnicodeDecodeError, PermissionError) as e:
                    # Ignoramos silenciosamente los archivos que no podemos leer, como te gusta.
                    continue
                except Exception as e:
                    # Capturamos otros errores (como un PDF encriptado o falta de Tesseract)
                    # para que todo el script no se caiga por un solo archivo rebelde.
                    continue
            elif elemento.is_dir():
                estructura.append({
                    "type": "folder",
                    "name": elemento.name,
                    "content": recur(elemento)
                })
    except PermissionError:
        pass
    return estructura

arbol_datos = recur(Path("."))
resultado_final = formatear_estructura(arbol_datos)

impri = None
copy = None

while True:
    opcion = input("¿Quieres imprimirlo? S/N> ").lower().strip()
    if opcion == "s":
        impri = True
        break
    elif opcion == "n":
        impri = False
        break
    print("Eso no es una opción válida.")

while True:
    opcion = input("¿Quieres copiarlo al portapapeles? S/N> ").lower().strip()
    if opcion == "s":
        copy = True
        break
    elif opcion == "n":
        copy = False
        break
    print("Eso no es una opción válida.")

if impri:
    print(resultado_final)
if copy:
    clipboard.copy(resultado_final)
    print("\nCopiado al portapapeles. Úsalo con sabiduría.")