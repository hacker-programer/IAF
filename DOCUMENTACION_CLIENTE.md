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
- 📱 **Acceder desde Android** con la app Capacitor
- 🖥️ **Cliente Electron** nativo para Windows (no necesita navegador)

---

## 📱 Clientes Disponibles

### 🖥️ Cliente Electron (Windows/Linux/Mac) — RECOMENDADO

El cliente Electron es una app de escritorio nativa que:
- **No necesita navegador** — todo está embebido en la app
- **Ejecuta comandos localmente** (PowerShell, git, cargo) igual que el viejo cliente Rust
- Se conecta automáticamente al servidor IAF
- Guarda tus credenciales de forma segura

```powershell
# Instalar
cd electron
npm install

# Iniciar
npm start
```

### 📱 Cliente Android (Capacitor)

La app Android te permite:
- Chatear con el agente desde tu celular
- Ver el historial de conversaciones
- Para ejecutar comandos (PowerShell, git, cargo), necesita que el cliente Electron esté corriendo en tu PC
- Si sos admin, el servidor ejecuta comandos directamente

```powershell
# Setup inicial
cd capacitor
.\setup_capacitor.ps1
```

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

### Regla de seguridad: ¿quién ejecuta comandos?

| Usuario | Desde navegador/Android | Desde Electron |
|---------|------------------------|----------------|
| **Admin** | El servidor ejecuta | Electron ejecuta localmente |
| **Normal** | ❌ Necesita cliente Electron en PC | Electron ejecuta localmente |

**El servidor NUNCA ejecuta comandos para usuarios no-admin.** Si sos usuario normal y querés usar IAF desde el navegador o Android, necesitás tener el cliente Electron corriendo en tu PC.

---

## 🌐 Acceso Remoto con Cloudflare Tunnel

Si necesitás acceder a IAF desde fuera de tu red local (desde el trabajo, la universidad o el celular), podés usar un túnel de Cloudflare que **solo expone el puerto 8080** (el que requiere login).

### Modo rápido (pruebas, sin dominio propio)

```powershell
.\scripts\cloudflare_tunnel.ps1 -Mode quick
```

Esto genera una URL temporal como `https://gato-aleatorio.trycloudflare.com`. Ideal para probar.

### Modo permanente (producción, con tu dominio)

```powershell
.\scripts\cloudflare_tunnel.ps1 -Mode permanent -Domain "iaf.midominio.com"
```

Luego podés ejecutar el túnel cuando quieras:

```powershell
cloudflared tunnel run iaf-tunnel
```

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
| Node.js 18+ | Para cliente Electron y Capacitor |

### Puesta en marcha

1. Asegurate de tener Rust, Git, GitHub CLI y Node.js instalados.
2. Colocá tus claves API en el archivo `.env`.
3. Ejecutá `cargo run --release` en la carpeta del proyecto.
4. Abrí tu navegador en `http://localhost:8080` **o** iniciá el cliente Electron:
   ```powershell
   cd electron
   npm install
   npm start
   ```
5. (Opcional) Para acceso remoto, ejecutá el script de túnel Cloudflare.
6. (Opcional) Para Android, ejecutá `.\capacitor\setup_capacitor.ps1`.

---

## Uso básico

### 1. La interfaz

Al abrir `http://localhost:8080` verás:

- **Panel izquierdo**: Lista de proyectos y chats anteriores.
- **Panel central**: El chat donde hablás con el asistente.
- **Panel derecho**: Consola de monitoreo (muestra qué está haciendo).

En **celulares/tablets**, el panel izquierdo se oculta y se abre con el botón ☰.

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

### 6. Modos: Programar vs Estudiar

- **💻 Programar**: El asistente escribe código y ejecuta comandos autónomamente.
- **📚 Estudiar**: El asistente te enseña de forma personalizada según tu perfil de aprendizaje.

---

## 👑 Para administradores

Los administradores pueden:
- Ver conversaciones de **todos** los usuarios (con etiqueta `@username`)
- Gestionar usuarios, permisos, límites y horarios desde el panel Admin
- Usar autenticación por firma digital Ed25519 (más segura que contraseña)

---

## ¿Cómo funciona la autenticación?

### Para usuarios normales

Ingresás tu **nombre de usuario** y **contraseña**. Simple.

### Para administradores

Los administradores usan un sistema más seguro: **firma digital**.

1. Solicitás un "desafío" (un número aleatorio)
2. Lo firmás con tu clave privada usando el script `sign_nonce.ps1`
3. El servidor verifica la firma con tu clave pública

---

## Consejos para obtener mejores resultados

### Sé específico

✅ **Bueno**: "Creá una función en Rust que calcule el factorial de un número y agregale tests unitarios."

❌ **Malo**: "Hacé algo con matemáticas."

### Dividí tareas grandes en pasos

1. "Configurá el proyecto con Rust y Axum."
2. "Agregá el endpoint de usuarios."
3. "Creá la página de registro."

---

## Solución de problemas comunes

### El asistente se queda pegado

Probá presionando "Interrumpir" y luego enviá tu mensaje de nuevo.

### Error "API key no configurada"

Revisá que el archivo `.env` tenga las claves correctas y reiniciá el servidor.

### El cliente Electron no conecta

1. Verificá que el servidor IAF esté corriendo en `127.0.0.1:8080`
2. Revisá que hayas hecho login en la UI del Electron primero
3. Las credenciales se guardan en `%APPDATA%/iaf-electron/config.json`

### Conversaciones duplicadas en el historial

Solucionado en v3.0. Si ves duplicados de antes, se limpiarán automáticamente al abrir la conversación.

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

### ¿Puedo usar IAF desde mi celular?

Sí, de dos formas:
1. **Navegador**: Si configuraste el túnel Cloudflare, accedé desde cualquier navegador
2. **App Android**: Con Capacitor, tenés una app nativa (requiere cliente Electron en PC para comandos)

### ¿Necesito el cliente Electron?

Si solo usás IAF desde la misma PC donde corre el servidor y sos admin (puerto 80), no. En cualquier otro caso, **sí** — el cliente Electron es quien ejecuta PowerShell, git y cargo en tu PC.

---

## Soporte

Si encontrás algún problema, revisá:
- El archivo `DOCUMENTACION_INTERNA.md` (para detalles técnicos)
- El archivo `MEMORIES.md` (para problemas conocidos)
- El archivo `DOCUMENTATION.md` (mapa técnico del proyecto)
