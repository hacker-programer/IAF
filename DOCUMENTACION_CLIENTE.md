# 🚀 IAF — Intelligent Agent Framework — Guía del Usuario

## ¿Qué es IAF?

IAF es un **asistente de desarrollo inteligente** que trabaja solo en tus proyectos de software. 
Le das instrucciones en lenguaje natural y él escribe código, busca en internet, ejecuta 
comandos y sube cambios a GitHub, todo de forma autónoma.

**No necesitás saber programar para usarlo.** Solo tenés que describir lo que querés hacer.

---

## ¿Qué puede hacer?

- ✍️ **Escribir y modificar código** en múltiples lenguajes (Rust, JavaScript, Python, etc.)
- 🔍 **Buscar en Google** información actualizada
- 🖥️ **Ejecutar comandos** en tu computadora
- 📦 **Subir cambios a GitHub** automáticamente
- 📸 **Analizar imágenes** (capturas de pantalla, diseños, etc.)
- 🔗 **Clonar y forkear repositorios** de GitHub
- 🖱️ **Controlar tu escritorio** (escribir texto, mover el mouse)

---

## Instalación

### Requisitos mínimos

| Componente | Mínimo |
|------------|--------|
| Procesador | 2 núcleos a 2.0 GHz |
| Memoria RAM | 4 GB |
| Sistema Operativo | Windows 10/11 |
| Rust | Instalado (via rustup) |
| Git | Instalado |
| GitHub CLI (`gh`) | Instalado y autenticado |

### Claves API necesarias

El asistente necesita estas claves para funcionar. Las configurás una sola vez:

1. **DeepSeek API Key** — Es la más importante. Se configura en el archivo `.env` del proyecto.
2. **OpenRouter API Key** — Para análisis multimodal de imágenes (opcional pero recomendado).

Consultá con el desarrollador para obtener estas claves o generá las tuyas propias en:
- DeepSeek: https://platform.deepseek.com/api_keys
- OpenRouter: https://openrouter.ai/keys

### Puesta en marcha

1. Asegurate de tener Rust, Git y GitHub CLI instalados.
2. Colocá tus claves API en el archivo `.env`.
3. Ejecutá `cargo run --release` en la carpeta del proyecto.
4. Abrí tu navegador en `http://localhost:3000`.

---

## Uso básico

### 1. La interfaz

Al abrir `http://localhost:3000` verás:

- **Panel izquierdo**: Lista de proyectos y chats anteriores.
- **Panel central**: El chat donde hablás con el asistente.
- **Panel derecho**: Consola de monitoreo (muestra qué está haciendo).

### 2. Agregar un proyecto

Tenés dos formas de agregar un proyecto:

- **Desde GitHub**: Pegá la URL de un repositorio y presioná "Fork".
- **Desde tu PC**: Ingresá el nombre y la ruta de la carpeta, y presioná "Agregar Local".

### 3. Iniciar una conversación

1. Seleccioná un proyecto de la lista.
2. Escribí tu instrucción en el chat.
3. Presioná "Enviar" o Ctrl+Enter.

El asistente comenzará a trabajar. Podés ver su progreso en la consola de monitoreo.

### 4. Interrumpir al asistente

Si el asistente está haciendo algo que no querés, presioná el botón **"Interrumpir"**. 
El asistente se detendrá de forma segura.

### 5. Reanudar conversaciones anteriores

Todas tus conversaciones se guardan automáticamente. Para continuar una anterior, 
seleccionala de la lista "Historial de Chats".

---

## Consejos para obtener mejores resultados

### Sé específico

✅ **Bueno**: "Creá una función en Rust que calcule el factorial de un número y agregale tests unitarios."

❌ **Malo**: "Hacé algo con matemáticas."

### Describí el resultado esperado

✅ **Bueno**: "Quiero que la página de login tenga un fondo azul oscuro, el logo centrado y un formulario de email/contraseña con bordes redondeados."

❌ **Malo**: "Mejorá la página de login."

### Dividí tareas grandes en pasos

Si tenés un proyecto complejo, dividilo en tareas más pequeñas:

1. "Configurá el proyecto con Rust y Axum."
2. "Agregá el endpoint de usuarios."
3. "Creá la página de registro."

---

## Solución de problemas comunes

### El asistente se queda pegado

Probá presionando "Interrumpir" y luego enviá tu mensaje de nuevo.

### Error "API key no configurada"

Revisá que el archivo `.env` tenga las claves correctas y reiniciá el servidor.

### Cambios no deseados en mi código

Todos los cambios se versionan en Git. Podés revertirlos con `git log` y `git revert`.

### El asistente no encuentra mi proyecto

Asegurate de que la carpeta del proyecto exista y tenga un repositorio Git inicializado.

---

## Preguntas frecuentes

### ¿IAF modifica archivos sin preguntar?

Sí. IAF es autónomo y modifica archivos directamente. Sin embargo, todos los cambios quedan 
registrados en Git, así que siempre podés revisar y revertir lo que hizo.

### ¿Puedo usar IAF para proyectos que no son de Rust?

Sí. Aunque IAF está optimizado para Rust, puede trabajar con JavaScript, Python, HTML, CSS 
y cualquier lenguaje de programación.

### ¿IAF consume muchos recursos?

Está diseñado para funcionar en computadoras de gama baja (4 GB de RAM, 2 núcleos). 
Si tu computadora es más potente, IAF se adapta automáticamente para aprovecharla.

### ¿Mis datos están seguros?

IAF se ejecuta 100% en tu computadora. Las únicas conexiones externas son a las APIs 
(DeepSeek, Google) y a GitHub para subir cambios.

---

## Soporte

Si encontrás algún problema, revisá:
- El archivo `DOCUMENTACION_INTERNA.md` (para detalles técnicos)
- El archivo `MEMORIES.md` (para problemas conocidos)
- Los logs en `.config/logs/`
