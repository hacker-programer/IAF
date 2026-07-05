from pathlib import Path
import clipboard

def formatear_estructura(elemento, nivel=1) -> str:
    # Calculamos la indentación base para el nivel actual del JSON
    ind_base = " " * (nivel * 4)
    ind_contenido = " " * ((nivel + 1) * 4) # 4 espacios más adentro para el contenido
    
    if isinstance(elemento, list):
        if not elemento:
            return "[]"
        lineas = []
        for item in elemento:
            lineas.append(formatear_estructura(item, nivel + 1))
        # Unimos los elementos de la lista
        return "[\n" + ",\n".join(lineas) + f"\n{ind_base}]"
        
    elif isinstance(elemento, dict):
        if elemento.get("type") == "file":
            # Formateamos el bloque 'content' manualmente con saltos reales
            lineas_codigo = elemento["content"].splitlines()
            
            # Le sumamos la indentación a cada línea del archivo
            codigo_identado = f"\n{ind_contenido + " " * 4}".join(lineas_codigo)
            
            # Armamos el JSON de este archivo a mano para meter el salto real
            return (
                f"{ind_base}{{\n"
                f"{ind_contenido}\"type\": \"file\",\n"
                f"{ind_contenido}\"name\": \"{elemento['name']}\",\n"
                f"{ind_contenido}\"content\": \"\"\"\n{ind_contenido + " " * 4}{codigo_identado}\"\"\"\n"
                f"{ind_base}}}"
            )
        else:
            # Si es una carpeta, procesamos su contenido recursivamente aumentando el nivel
            contenido_carpeta = formatear_estructura(elemento["content"], nivel + 1)
            return (
                f"{ind_base}{{\n"
                f"{ind_contenido}\"type\": \"folder\",\n"
                f"{ind_contenido}\"name\": \"{elemento['name']}\",\n"
                f"{ind_contenido}\"content\": {contenido_carpeta.lstrip()}\n"
                f"{ind_base}}}"
            )

def recur(pa: Path) -> list:
    estructura = []
    try:
        for elemento in pa.iterdir():
            if elemento.is_file():
                try:
                    estructura.append({
                        "type": "file",
                        "name": elemento.name,
                        "content": elemento.read_text(encoding="utf-8")
                    })
                except (UnicodeDecodeError, PermissionError):
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

# Generamos el árbol crudo
arbol_datos = recur(Path("./IAF"))

# Lo formateamos con nuestro formateador personalizado
resultado_final = formatear_estructura(arbol_datos)

impri = None
copy = None

while True:
    opcion = input("Quieres imprimirlo? S/N> ").lower().strip()
    if opcion == "s":
        impri = True
        break
    elif opcion == "n":
        impri = False
        break
    print("Eso no es una opcion valida.")

while True:
    opcion = input("Quieres copiarlo al portapapeles? S/N> ").lower().strip()
    if opcion == "s":
        copy = True
        break
    elif opcion == "n":
        copy = False
        break
    print("Eso no es una opcion valida.")

if impri:
    print(resultado_final)
if copy:
    clipboard.copy(resultado_final)