# Comparativa con otros asistentes de desarrollo

Este documento describe cómo se compara IAF con las alternativas disponibles en el mercado a junio de 2025. No pretende vender nada. Describe lo que cada herramienta hace según su código fuente, documentación oficial y reports de usuarios.

---

## Qué es IAF exactamente

Revisando el código fuente (src/main.rs, src/agent.rs, src/state.rs, Cargo.toml), IAF es un servidor HTTP escrito en Rust sobre Axum que expone una interfaz web mínima (tres archivos: index.html, app.js, style.css) y una API REST. El componente central es un bucle agéntico (src/agent.rs) que invoca la API de DeepSeek usando el modelo `deepseek-v4-pro`. El agente recibe un mensaje del usuario, lo envía a DeepSeek junto con definiciones de herramientas en formato OpenAI function calling, y ejecuta las herramientas que el modelo decide llamar. El ciclo se repite hasta que el modelo invoca `finalizar_tarea` o el usuario interrumpe manualmente.

Las herramientas disponibles, verificadas en el código (agent.rs, líneas ~130-330), son: buscar en Google mediante scraping HTTP, leer archivos del sistema de archivos local, escribir archivos con commit automático a GitHub, ejecutar comandos de PowerShell, buscar código semánticamente (la función existe aunque la implementación real usa coincidencia de texto, según indica el system prompt), forkear y clonar repositorios con GitHub CLI, leer URLs públicas, ejecutar comandos genéricos de GitHub CLI, notificar al usuario, y manejar imágenes (descargar, ver, liberar). También hay un controlador de escritorio (desktop.rs) que permite mover el ratón, hacer clic, lanzar ejecutables y abrir archivos con la aplicación predeterminada del sistema. Usa la biblioteca `rdev` para simular entrada.

El sistema mantiene sesiones de chat persistentes en disco, con historial completo y registro de pasos de auditoría. Usa VoyageAI (voyage-code-2) para generar embeddings, aunque la búsqueda de código actualmente funciona por coincidencia textual. El agente se ejecuta como una tarea asíncrona de Tokio y puede interrumpirse desde la interfaz web.

No hay paralelismo de agentes. No hay sandboxing. No hay automatización de navegador. No hay extensión para editores de código. El agente trabaja secuencialmente: piensa, decide una herramienta, la ejecuta, recibe el resultado, piensa otra vez.

---

## Comparación con otras herramientas

### Google Antigravity

Google Antigravity es un fork de VS Code lanzado por Google que integra un sistema de orquestación de múltiples agentes. Según la documentación oficial de Google y los análisis publicados en medios técnicos, permite que varios agentes trabajen en paralelo —uno puede investigar documentación mientras otro modifica código— y tiene integración nativa con Chrome para probar aplicaciones web. Soporta modelos de Google (Gemini) y permite usar modelos de terceros como Claude y GPT-4o mediante BYOM. Actualmente está en preview pública gratuita, subvencionada por Google.

Comparado con IAF, Antigravity ofrece más capacidades técnicas: paralelismo real de agentes, automatización de navegador, integración profunda con el ecosistema Google Cloud. IAF no tiene nada de esto. Sin embargo, Antigravity tiene problemas documentados por usuarios: los límites de uso incluso en modalidad de pago son restrictivos, especialmente al usar modelos Claude; la interfaz separa el editor del gestor de agentes, lo que añade complejidad; y existe el riesgo históricamente fundamentado de que Google abandone el producto (Google Code, Project IDX, Stadia, Google+, Reader, Domains y más de 290 productos cancelados). Además, al ser gratuito, las condiciones de uso permiten a Google usar los datos para entrenar sus modelos, lo que excluye su uso en entornos empresariales con requisitos de privacidad.

IAF no compite en funcionalidades con Antigravity. La diferencia práctica es que IAF no envía tu código a Google, no depende de un fork de VS Code, y no está sujeto a que Google decida cancelarlo.

### Cursor

Cursor es un fork de VS Code mantenido por Anysphere. Añade un panel de chat con capacidad de modificar múltiples archivos, ejecutar comandos en terminal y aplicar cambios mediante una interfaz de diferencias visuales. Usa Claude 3.5 Sonnet y GPT-4o como backends. El plan Pro cuesta 20 dólares al mes e incluye 500 solicitudes premium. Las solicitudes premium se consumen rápido en modo agente; al agotarse, el servicio sigue funcionando pero con modelos más lentos.

Comparado con IAF, Cursor ofrece una experiencia de edición visual integrada que IAF no tiene. La diferencia principal está en el modelo de costo y en la libertad de editor: IAF funciona desde terminal o navegador sin obligarte a usar un fork específico de VS Code. En términos de costos, los 20 dólares mensuales de Cursor son un gasto fijo independientemente del uso; IAF gasta según los tokens consumidos en la API de DeepSeek, que según los precios publicados por DeepSeek (0.14 dólares por millón de tokens de entrada, 0.28 por millón de salida para el modelo chat estándar) resulta en costos por tarea que pueden ir desde centavos hasta unos pocos dólares, dependiendo de la magnitud del trabajo.

### Windsurf

Windsurf es otro fork de VS Code, mantenido por Codeium. Su arquitectura Cascade alterna entre modo copiloto y modo agente. Mismo modelo de precio que Cursor: 20 dólares al mes con cuotas en modelos frontier. Las diferencias con Cursor son principalmente de interfaz y de ecosistema; Windsurf tiene una comunidad más pequeña. Respecto a IAF, aplican las mismas diferencias que con Cursor: IAF no requiere abandonar tu editor, y su costo es variable según consumo en lugar de una tarifa plana con límites.

### Claude Code

Claude Code es una herramienta de línea de comandos de Anthropic. No tiene interfaz gráfica. Opera directamente sobre el sistema de archivos, ejecuta comandos, y usa los modelos de Anthropic mediante API (el usuario proporciona su propia clave). Al no tener suscripción, el costo depende enteramente del consumo de tokens.

Este es el competidor más cercano a IAF en filosofía: ambos son agentes CLI, ambos usan BYOK, ambos funcionan con cualquier editor. La diferencia fundamental está en el costo de los tokens. Según los precios oficiales de Anthropic (mayo 2025), Claude 3.5 Sonnet cuesta 3 dólares por millón de tokens de entrada y 15 dólares por millón de tokens de salida. DeepSeek, según sus precios publicados, cuesta 0.14 dólares por millón de tokens de entrada y 0.28 por millón de salida. Esto significa que una misma tarea que consuma, por ejemplo, 2 millones de tokens de entrada y 500 mil de salida costaría aproximadamente 21 dólares en Claude Code y 0.42 dólares en IAF. La diferencia es de aproximadamente 50 a 1.

Claude Code tampoco tiene interfaz visual para revisar diferencias; IAF ofrece una interfaz web mínima donde se puede ver el progreso.

### GitHub Copilot

GitHub Copilot es una extensión para VS Code y JetBrains. Su función principal es el autocompletado de código. El plan Pro cuesta 10 dólares al mes, el Enterprise 39. Tiene un chat que puede razonar sobre el código abierto, pero no ejecuta comandos ni modifica archivos autónomamente. No es un agente: no puede tomar una tarea, ejecutarla de principio a fin y hacer commit sin intervención humana constante. IAF sí puede hacer esto, porque su bucle agéntico le permite encadenar herramientas hasta completar el objetivo.

### Devin

Devin es un agente autónomo de Cognition Labs que opera en un contenedor en la nube. No se ejecuta en la máquina del usuario. Recibe tareas, trabaja de forma asíncrona durante minutos y entrega pull requests. Está orientado al mercado enterprise con precios no públicos. Su principal desventaja frente a IAF es el aislamiento: no puede acceder a hardware local, bases de datos internas ni servicios que no estén expuestos a internet. IAF, al ejecutarse localmente, puede interactuar con cualquier recurso de la máquina.

---

## Costos reales

Tomando el caso concreto de una refactorización de un proyecto mediano (varios miles de líneas, múltiples archivos), el consumo típico de tokens de entrada puede andar en el orden de 1 a 3 millones (el sistema lee archivos, el historial de chat, los resultados de herramientas) y el de salida en 300 mil a 800 mil tokens (razonamientos, código generado, mensajes).

Con DeepSeek (modelo chat estándar, precios documentados): entre 0.20 y 0.80 dólares por el input y entre 0.08 y 0.22 por el output. Total aproximado: entre 0.30 y 1 dólar por tarea grande.

Con Claude 3.5 Sonnet (precios de API de Anthropic): entre 3 y 9 dólares por el input y entre 4.5 y 12 dólares por el output. Total aproximado: entre 7.5 y 21 dólares por la misma tarea.

Con Cursor o Windsurf: 20 dólares al mes, y si la tarea consume 50-100 de las 500 solicitudes premium mensuales, estás usando entre el 10% y el 20% de tu cuota mensual en una sola tarea.

Con Google Antigravity en preview gratuita: cero dólares directos, pero tu código se usa para entrenar modelos. Cuando termine la preview, el precio de mercado estimado para un servicio similar es de 20 a 50 dólares al mes.

---

## Lo que IAF no hace (y los demás sí)

IAF carece de varias cosas que otras herramientas ofrecen. No tiene paralelismo de agentes como Antigravity. No tiene una interfaz de diferencias visuales integrada en un editor como Cursor o Windsurf. No tiene la madurez ni el respaldo corporativo de GitHub Copilot. No tiene la capacidad de razonamiento visual de Claude (el modelo de DeepSeek es solo texto). No tiene el entorno aislado de Devin. No tiene integración con servicios cloud como Amazon Q.

El controlador de escritorio (desktop.rs) es limitado: puede mover el ratón, hacer clic, y escribir espacios. No puede escribir texto arbitrario porque la implementación actual solo maneja el carácter espacio. Esto está documentado en el código fuente.

La búsqueda en Google (scraper.rs) no usa la API oficial de Google; hace scraping del HTML de resultados. Esto significa que puede disparar CAPTCHAs y dejar de funcionar sin previo aviso.

---

## Para qué sirve IAF y para qué no

IAF es útil si querés un agente que trabaje sobre tu código local, en tu máquina, usando tu propia clave de API, sin límites artificiales de requests, sin cambiar de editor, y con un costo por tarea que está consistentemente por debajo de un dólar en la mayoría de los casos.

IAF no es la mejor opción si necesitás una experiencia de edición visual pulida, paralelismo de agentes, automatización de navegador, o un producto con soporte comercial y garantías de continuidad.

---

## Nota final

Los datos de precios de APIs corresponden a los publicados por DeepSeek y Anthropic a mayo de 2025. El código de IAF está en el repositorio donde se encuentra este mismo archivo; cualquiera puede verificarlo.
