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

---

## 🔐 Seguridad: Puertos y Acceso

IAF usa **dos puertos** con niveles de seguridad distintos:

| Puerto | Acceso | Autenticación | Ubicación |
|--------|--------|---------------|-----------|
| **80** | Admin local | ❌ Sin autenticación | Solo red local (127.0.0.1) |
| **8080** | Usuarios | ✅ Login obligatorio | Local + túnel Cloudflare |

- **Puerto 80**: Acceso directo como administrador. **SOLO debe usarse en tu red local de confianza.** No requiere contraseña porque se asume que quien está en tu PC eres tú.
- **Puerto 8080**: Acceso para todos los usuarios (incluyendo administradores). **Siempre requiere iniciar sesión** con usuario y contraseña, o con firma digital (Ed25519) para administradores.

> ⚠️ **Importante**: Nunca expongas el puerto 80 a internet. Cualquiera que acceda a él tendrá control total del sistema sin necesidad de contraseña.

---

## 🌐 Acceso Remoto con Cloudflare Tunnel

Si necesitás acceder a IAF desde fuera de tu red local (desde el trabajo, la universidad o el celular), podés usar un túnel de Cloudflare que **solo expone el puerto 8080** (el que requiere login).

### ¿Qué es Cloudflare Tunnel?

Es un servicio gratuito de Cloudflare que crea un "puente" seguro entre tu PC e internet, sin necesidad de abrir puertos en tu router ni configurar IPs públicas.

### Modo rápido (pruebas, sin dominio propio)

```powershell
.\scripts\cloudflare_tunnel.ps1 -Mode quick
```

Esto genera una URL temporal como `https://gato-aleatorio.trycloudflare.com`. Ideal para probar.

### Modo permanente (producción, con tu dominio)

```powershell
.\scripts\cloudflare_tunnel.ps1 -Mode permanent -Domain "iaf.midominio.com"
```

Esto configura un túnel con nombre, enruta tu dominio y genera el archivo de configuración `scripts\cloudflared_config.yml`.

Luego podés ejecutar el túnel cuando quieras:

```powershell
cloudflared tunnel run iaf-tunnel
```

O instalarlo como servicio de Windows (se inicia solo al prender la PC):

```powershell
cloudflared service install
```

### Seguridad del túnel

- ✅ Solo expone el puerto **8080** (NUNCA el puerto 80)
- ✅ El puerto 8080 **siempre requiere login**
- ✅ Los administradores necesitan firma digital Ed25519 (más segura que una contraseña)
- ✅ Cloudflare proporciona protección DDoS y certificado SSL automático

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

### Requisitos para túnel (opcional)

| Componente | Mínimo |
|------------|--------|
| cloudflared | Instalado (`winget install Cloudflare.cloudflared`) |
| Dominio Cloudflare | Solo para modo permanente |

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
4. Abrí tu navegador en `http://localhost:8080`.
5. (Opcional) Para acceso remoto, ejecutá el script de túnel Cloudflare.

---

## Uso básico

### 1. La interfaz

Al abrir `http://localhost:8080` verás:

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

## ¿Cómo funciona la autenticación?

### Para usuarios normales

Ingresás tu **nombre de usuario** y **contraseña**. Simple.

### Para administradores

Los administradores usan un sistema más seguro: **firma digital**.

1. Solicitás un "desafío" (un número aleatorio)
2. Lo firmás con tu clave privada usando el script `sign_nonce.ps1`
3. El servidor verifica la firma con tu clave pública

Esto significa que:
- No hay contraseña que pueda ser robada
- Un atacante necesitaría tu archivo de clave privada (que solo está en tu PC)
- Es el mismo sistema que usan las criptomonedas

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

### El túnel Cloudflare no funciona

1. Verificá que `cloudflared` esté instalado: `cloudflared --version`
2. Verificá que el servidor IAF esté corriendo en `127.0.0.1:8080`
3. Para modo permanente, verificá que tu dominio esté configurado en Cloudflare
4. Revisá que no haya firewall bloqueando `cloudflared`

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
(DeepSeek, Google, OpenRouter) y a GitHub para subir cambios.

### ¿Puedo usar IAF desde mi celular?

Sí, si configuraste el túnel Cloudflare, podés acceder desde cualquier navegador web a 
tu dominio (ej: `https://iaf.midominio.com`). Necesitarás tu usuario y contraseña.

---

## Soporte

Si encontrás algún problema, revisá:
- El archivo `DOCUMENTACION_INTERNA.md` (para detalles técnicos)
- El archivo `MEMORIES.md` (para problemas conocidos)
- El archivo `DOCUMENTATION.md` (mapa técnico del proyecto)
