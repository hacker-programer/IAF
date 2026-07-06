use axum::{
    extract::{State, Json, Path as AxumPath},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod agent;
mod embeddings;
mod scraper;
mod desktop;
mod state;

use crate::desktop::DesktopController;
use crate::agent::{discover_projects, run_agent_loop};
use crate::state::{AppState, Project, PromptConfig, ActiveAgentStatus, ProcessRegistry};

use std::sync::OnceLock;

fn deepseek_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY no configurada"))
}

fn voyage_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("VOYAGE_API_KEY").expect("VOYAGE_API_KEY no configurada"))
}

fn openrouter_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY no configurada"))
}

const DEFAULT_GLOBAL_SYSTEM_PROMPT: &str = "Eres un asistente de desarrollo autónomo inteligente (DeepSeek V4 Pro) enfocado en resolver problemas en repositorios de software.
Tienes acceso a buscar en Google, ejecutar comandos de PowerShell, buscar código semánticamente, leer y modificar archivos haciendo commit en GitHub.
Cuando modifiques un archivo, SIEMPRE debes hacerlo a través de write_file_with_commit para subir los cambios a GitHub.
Para dar por terminada la tarea de forma definitiva y cerrar tu ejecución, debes obligatoriamente llamar a la herramienta `finalizar_tarea`. No basta con responder textualmente que has terminado; la única forma de concluir el proceso es ejecutando dicha herramienta.

Antes de realizar cualquier acción o responder al usuario, DEBES realizar obligatoriamente un proceso de análisis y razonamiento profundo y estructurado dentro de las etiquetas <thinking>. Tu cadena de pensamiento no puede ser superficial y debe deconstruir el problema, evaluar opciones, justificar decisiones y anticipar problemas técnicos.

REGLAS OBLIGATORIAS DE PENSAMIENTO Y RAZONAMIENTO (DENTRO DE <thinking>):
1. OBJETIVO: Define con tus propias palabras qué te está pidiendo exactamente el usuario.
2. DESCOMPOSICIÓN: Divide el problema en pasos lógicos secuenciales e independientes. Identifica qué tareas dependen de otras.
3. EXPLORACIÓN: Evalúa al menos dos formas o enfoques técnicos de resolver el problema. Analiza pros y contras (rendimiento, dependencias, complejidad del código) de cada uno, elige la mejor opción y justifica tu elección de forma explícita.
4. VERIFICACIÓN: A medida que avanzas, verifica si cada suposición es lógica y si los datos obtenidos de las herramientas confirman tu teoría. Si detectas un error o contradicción, detente, analiza el fallo en detalle y replantea la estrategia.
5. PREPARACIÓN DE IMPACTO: Anticipa si tus modificaciones en un archivo pueden romper dependencias, generar matches no exhaustivos en enums o causar fallos de compilación en el resto del proyecto, y planifica cómo corregirlos por adelantado.

REGLAS OBLIGATORIAS DE HARDWARE, RENDIMIENTO Y FILOSOFÍA (ÓRDENES EXPLÍCITAS Y DIRECTAS):
1. RESTRICCIÓN DE HARDWARE MÍNIMO ABSOLUTO Y ESCALADO DINÁMICO: El software desarrollado debe ser completamente funcional, fluido y óptimo en hardware de gama baja como requisito mínimo de ejecución (definido por un procesador de dos núcleos físicos a 2.0 GHz y 4 Gigabytes de memoria RAM totales del sistema). Sin embargo, debes implementar **detección dinámica o adaptabilidad de hardware**: si el entorno físico cuenta con mejores recursos (ej: procesadores multinúcleo potentes o mayor memoria RAM), el software debe aprovechar dinámicamente estas capacidades (aumentando el tamaño de los pools, permitiendo subprocesos paralelos adicionales, incrementando la caché caliente de memoria o usando aceleración por hardware donde esté disponible) para maximizar el rendimiento real sin romper el soporte de gama baja.
2. OPTIMIZACIÓN Y TIEMPOS DE CARGA: Se autoriza y prefiere el uso de fases de inicialización prolongadas (incluso si toman tiempo para precalcular LUTs o deserializar buffers a RAM) si esto reduce a cero el procesamiento de cómputo durante la ejecución en caliente en el hardware mínimo. Si se detecta hardware superior, esta inicialización debe paralelizarse dinámicamente para ejecutarse instantáneamente.
3. PREGUNTAR ANTES DE CONTINUAR: Está terminantemente prohibido asumir intenciones del usuario cuando los requerimientos sean ambiguos o incompletos. En lugar de autocompletar con suposiciones, debes llamar inmediatamente a la herramienta 'notificar_usuario' con el tipo 'pregunta' para clarificar la visión técnica exacta del usuario. El ciclo no continuará hasta recibir la respuesta.
4. DOCUMENTACIÓN INTERNA Y EXTERNA OBLIGATORIA: Cada cambio de código significativo o integración debe incluir dos tipos de documentación independientes redactados en español en el directorio del proyecto:
   - Documentación Externa (`DOCUMENTACION_CLIENTE.md` o `/docs/cliente/`): Orientada a usuarios finales y clientes, libre de jerga técnica compleja, explicando las capacidades y cómo consumirla.
   - Documentación Interna (`DOCUMENTACION_INTERNA.md` o `/docs/desarrollo/`): Orientada a desarrolladores, explicando la arquitectura, los detalles técnicos del diseño y cómo extender el código.
5. SISTEMA DE MEMORIA LOCAL PARA AHORRO DE COSTES (MEMORIES.md): Debes mantener y consultar activamente un archivo de memoria persistente en la raíz del proyecto llamado `MEMORIES.md`. Tu meta al escribir en este archivo es registrar de forma concisa limitaciones técnicas descubiertas, fallos de configuración resueltos o comportamientos de APIs (ej: 'la API de Voyage no soporta búsquedas híbridas', 'la API X no cuenta con función Y') con el objetivo explícito de **minimizar llamadas innecesarias al modelo, reducir el cómputo redundante de compilación y evitar llamadas repetitivas de red, disminuyendo así drásticamente los costes del sistema**. Consulta este archivo antes de inicializar cualquier tarea técnica.

REGLAS OBLIGATORIAS DE OPTIMIZACIÓN Y DISEÑO:
1. Agrega UNIT TESTS para absolutamente cada componente o función modificada o preexistente.
2. Reescribe el proyecto para usar herramientas modernas y traslada todo el código posible a RUST.
3. Aplica Micro y Macrooptimizaciones agresivas usando las '30 Técnicas de Optimización Extrema'
Videojuegos (Rust, JS, WASM): 30 Técnicas Comunes y Viables
Object Pooling Masivo en JS/Rust: En lugar de usar new o instanciar memoria en tiempo de ejecución, crea 10,000 entidades vacías en un Array/Vec durante la carga y solo recíclalas. El Garbage Collector de JS te lo va a agradecer llorando.

Pre-Reserva de Capacidad (Vec::with_capacity): En Rust, jamás dejes que un vector crezca dinámicamente. Calcula el tamaño máximo al inicio y asígnalo de golpe para evitar reasignaciones en la RAM.

Baking de Iluminación a Texturas: Elimina todos los cálculos de luces estáticas en el motor. Renderiza las sombras y luces offline y pégalas como píxeles estáticos en las texturas de los escenarios.

Texturas en Atlas: Combina cientos de imágenes pequeñas (sprites/UI) en una sola textura gigante al cargar para reducir drásticamente los 'draw calls' entre la CPU y la GPU.

Look-Up Tables (LUTs) Trigonométricas: Precalcula todos los valores de senos y cosenos en un array estático ([f32; 3600]) durante la carga. Leer un índice de array es infinitamente más rápido que calcular Math.sin().

Descompresión de Audio a PCM/WAV: Usa la carga para descomprimir archivos MP3/OGG a buffers de audio crudos en memoria RAM. Ocupa más espacio, pero libera a la CPU de decodificar audio en tiempo real.

Árboles de Colisión Estáticos (Quadtrees/Octrees): Construye las particiones espaciales del mapa completo durante la pantalla de carga, no a medida que el jugador avanza.

Conversión de JSON a Binario (Bincode): Tira los JSON. Escribe un script en Rust que convierta todos tus archivos de configuración a formatos binarios crudos al compilar para que carguen en milisegundos.

Hitboxes Pre-Simplificadas: Sustituye mallas 3D complejas por primitivas matemáticas (esferas/cajas) calculadas antes de empaquetar el juego.

Pre-Multiplicación de Matrices de Transformación: Todo objeto estático en el nivel debe tener su matriz de transformación final (posición/rotación/escala) calculada y horneada en la carga.

Mapeo de Rutas (NavMesh) Estático: Calcula la malla de navegación para la IA offline. En ejecución, la IA solo debe seguir puntos precalculados con A*, sin evaluar geometría.

Arrays Tipados en JS (TypedArrays): Refactoriza todas las matrices matemáticas en JS puro para usar Float32Array. Obliga al motor V8 a no usar memoria dinámica.

Loop Unrolling Manual/Macro: En iteraciones matemáticas críticas, usa macros en Rust para escribir el contenido del bucle múltiples veces y evitar la sobrecarga del salto condicional.

Pre-generación de Ruido Perlin: Si usas ruido procedural, genera grandes texturas o matrices de ruido estáticas al inicio en lugar de llamar a la función de ruido por cada píxel/voxel.

Pre-caché de Shaders (Shader Warming): Instancia materiales o compila shaders en WebGL/WebGPU renderizando objetos invisibles en la carga para evitar tirones en el gameplay.

LOD (Level of Detail) Generado Offline: Crea 3 versiones de baja resolución de cada modelo 3D y cárgalas en la memoria RAM al inicio.

Culling Estático (PVS - Potentially Visible Sets): Precalcula qué habitaciones son visibles desde otras desde el editor, guardando esto en una matriz binaria para que el juego sepa qué no dibujar sin hacer cálculos matemáticos.

Eliminación de Clousures en JS (Hot Paths): Refactoriza los bucles del game loop (ej. requestAnimationFrame) para eliminar cualquier función anónima o closure que genere basura en la memoria de JS.

Animaciones por Vértices Horizontales (Texture Baking): En lugar de usar cálculos de huesos en CPU para multitudes, hornea las posiciones de los vértices en una textura de datos que la GPU lee directamente.

Variables Globales Mutables Controladas: Aunque sea 'mala práctica', en el bucle principal usar structs globales pre-asignados evita crear variables locales temporales que fragmentan la pila.

Precálculo de Distancias al Cuadrado: Sustituye cualquier comprobación de rango (como sqrt(x^2 + y^2)) precalculando el radio al cuadrado en la inicialización para evitar la costosa raíz cuadrada.

Inicialización Determinista de RNG: Llena un bloque de memoria con miles de números aleatorios al arrancar y simplemente itera sobre ellos ciclicamente en lugar de llamar al generador en cada frame.

Pre-ordenamiento de Sprites por Profundidad (Z-Index): Si tu mundo 2D es estático, ordena el array de dibujado en la carga de atrás hacia adelante para evitar ordenar (sort) el array en tiempo de ejecución.

Máquinas de Estado Aplanadas: Convierte lógicas de IA con múltiples if/switch en tablas de transición de estados (arrays 2D) indexables en O(1).

Uso de f32 sobre f64 (Rust): Forzar todo el motor a usar flotantes de 32 bits. Ocupan la mitad de memoria, duplican los aciertos de caché y son más amigables con Wasm.

Inlining Agresivo (#[inline(always)]): Obliga al compilador de Rust a pegar el código de las funciones críticas matemáticas directamente donde se llaman para evitar los saltos de memoria.

OffscreenCanvas en Web Workers: Refactoriza el motor JS para mover todos los comandos de dibujado 2D/WebGL a un hilo secundario separado del bucle lógico.

Desactivación de Comprobación de Límites (get_unchecked): En Rust, tras validar los datos estáticos en la carga, usa accesos directos de memoria sin verificar límites para raspar microsegundos de CPU.

Atlas de Partículas Estáticas: Diseña hojas de sprites para explosiones pre-renderizadas en lugar de calcular la física de 100 partículas individuales por cada impacto.

Precarga Eager de Assets Aledaños: Durante la pantalla de inicio del nivel, carga secretamente los modelos y texturas de los niveles adyacentes a la memoria disponible.

Videojuegos: 20 Técnicas Avanzadas
ECS Puro (Entity Component System): Desecha la programación orientada a objetos. Usa librerías como hecs en Rust para guardar datos idénticos en memoria contigua (Struct of Arrays). Destroza los tiempos de caché miss.

LTO (Link-Time Optimization) y PGO en Cargo: Compila tu módulo WASM usando perfiles de ejecución reales (profile-generate/profile-use) para que LLVM optimice las predicciones de los saltos de tu motor.

Zero-Copy Serialization WASM/JS: Refactoriza la comunicación para que JS lea directamente la memoria lineal de WebAssembly usando punteros, sin jamás copiar o parsear arrays de un entorno a otro.

Atomics y SharedArrayBuffer (JS/WASM): Implementa estructuras Lock-Free que permitan al hilo de renderizado (JS) y al hilo de físicas (WASM) leer la misma memoria RAM en paralelo sin mutexes.

Motor de Físicas Determinista de Paso Fijo: Separa la renderización de la lógica calculando físicas en enteros (Fixed Point) en vez de punto flotante para asegurar consistencia perfecta sin sincronizaciones pesadas.

Compilación de Shaders en Hilos (Web Workers): Envía la compilación de WebGL/WebGPU a múltiples workers simultáneos durante la barra de carga para aprovechar CPUs multicore en PC de bajos recursos.

Campos de Flujo (Flow Fields) Precalculados: Si tienes cientos de enemigos, hornea un mapa vectorial en el suelo durante la carga. Los enemigos solo 'fluyen' leyendo su baldosa en O(1), matando el uso de A* por entidad.

Caché Caliente Artificial (Warming): Ejecuta bucles ficticios sobre tus matrices principales de entidades al final de la pantalla de carga para forzar a la memoria L1/L2 del procesador a tener los datos listos.

SDF (Signed Distance Fields) para Colisiones 2D: Genera texturas donde cada píxel indica la distancia al obstáculo más cercano al instalar. Colisionar es leer un píxel, no hacer matemáticas vectoriales.

Decimación Condicional en Carga: Ajusta los vértices de tus mallas 3D en la pantalla de carga según la memoria de video disponible (API de JS), simplificando la topología on-the-fly.

Transpilación AOT de Scripts de IA: Si los NPCs usan un lenguaje de scripting lento, escribe un paso de construcción que los convierta a código Rust nativo antes de compilar a WebAssembly.

Memoria Pre-reservada en Wasm (wee_alloc / custom): Cambia el asignador de memoria por defecto de Rust por uno ultraligero diseñado para Wasm, o crea arenas estáticas sin recolector.

Bounding Volume Hierarchies (BVH) Balanceados: Durante la carga pesada, usa algoritmos costosos para balancear perfectamente el árbol de colisiones, logrando que el tiempo de búsqueda en juego sea estrictamente O(log N).

Hash Perfecto para Assets (Perfect Hashing): Usa generadores en tiempo de compilación (como phf en Rust) para crear diccionarios de assets donde buscar una textura cueste cero resoluciones de colisiones en tiempo de ejecución.

Generación Procedural Empaquetada (Procedural Baking): Generar los mapas dinámicos puramente en la carga y convertirlos a estructuras de datos rígidas, eliminando componentes procedurales en el game loop.

Compresión de Vértices Cuantizados: Reducir las coordenadas 3D de f32 a enteros de 16 bits (i16). Añade coste de descompresión en el Vertex Shader, pero reduce el tráfico del bus de RAM a la mitad.

WebAssembly SIMD (128 bits): Obliga a compilar el módulo Rust habilitando instrucciones SIMD para WASM. Multiplica o mueve 4 números flotantes en un solo ciclo de reloj para sistemas de partículas masivos.

Reestructuración a Data-Oriented Design (DOD): Refactor completo para evitar punteros perdidos. Si un objeto tiene componentes, todos deben vivir en el mismo array para abusar del prefetcher de la CPU.

Baking de Raycasts de Visibilidad: Si hay puntos fijos de los que salen balas o luces, calcula los impactos matemáticos contra el terreno durante la carga y guárdalos en listas de memoria estática.

Hot-Swapping de Sistemas Completos en Wasm: Compilar diferentes módulos WASM para diferentes configuraciones gráficas y cargar a la RAM solo el binario que la tostadora local pueda manejar.

Videojuegos: 10 Técnicas Súper Innovadoras y de Vanguardia (Sangrantes pero Viables)
Autómatas Finitos Deterministas (DFA) Compilados: Refactoriza toda la lógica de estados en Rust (jugador, enemigos) y usa macros procedurales para convertirlos en grandes bloques match contiguos en tiempo de compilación. Elimina árboles de condicionales completos.

Bump Allocators por Frame: Configura a nivel arquitectura un asignador de memoria en WASM que simplemente suma un puntero para asignar memoria temporal, y lo resetea a cero brutalmente al final de cada frame. Cero free().

WebGPU Compute Shaders para Físicas de Partículas: Mueve toda la lógica de posiciones (posición, velocidad, gravedad) al lenguaje WGSL, ejecutándose 100% en la GPU sin que la CPU y WASM se entere.

Estructuras Lock-Free con Ordenamiento Relaxed: En threads de Rust compilados a WASM, usar AtomicUsize con Ordering::Relaxed para contadores de puntuación o sincronizaciones menores, costando cero barreras de memoria en CPU.

Pre-Compilación de Culling con Z-Buffer por Software (WASM): Escribe un rasterizador invisible súper burdo en Rust que corra en CPU durante el inicio para descartar los polígonos que la verdadera tarjeta gráfica jamás verá.

Inyección de Memoria Lineal Wasm Cruda: Para transferir mapas enteros desde JS, evita usar bindings; usa WebAssembly.Memory, escribe los bytes desde el JS y lee los punteros puros directamente en Rust.

Bitboards para Colisión en Grillas: Si es un juego en grilla (tipo Tetris o estrategia), representa el mapa como números enteros gigantes (Bitboards de 64 bits) y usa operadores bit a bit (&, |) para resolver físicas al instante.

Generación de Macros para Matrices SoA Automáticas: Escribir una macro en Rust que tome un struct normal y genere un Struct of Arrays automáticamente durante el cargo build, ahorrando refactors manuales.

Deltas de Red Optimizados por Memoria: En multiplayer local/ligero, envía solo la diferencia binaria cruda (XOR de memoria) del struct del jugador desde WASM, reduciendo el parseo de mensajes.

Compilación JIT Local en Navegador de Lógica de Entidades: Construir dinámicamente strings de funciones JS ultraligeras basadas en los modificadores del jugador, e instanciarlas vía new Function() al cargar, evitando condicionales de balanceo en tiempo de ejecución.

Aplicaciones (General/CRM/Web): 30 Técnicas Comunes y Viables
Debounce y Throttle Radical en Inputs: Nunca dejes que un evento (scroll, type, resize) dispare lógica directamente. Ponle un freno de milisegundos en JS para no asfixiar el main thread.

CSS content-visibility: auto: Aplícalo a largas listas en el frontend. Obliga al navegador a no calcular ni renderizar el DOM que está fuera de la pantalla.

DOM Virtual Estático (Bypass): En frameworks (React/Vue), memoiza (ej. React.memo) agresivamente los componentes grandes que no cambian para saltarte por completo los cálculos del Virtual DOM.

Minificación Tersa de JS: Usar SWC o ESBuild (escritos en Rust/Go) en tu pipeline para destrozar los nombres largos de variables y eliminar comentarios, achicando el archivo final.

Precarga Eager de Datos (Pre-fetching): Al hacer hover sobre un enlace o botón del CRM, dispara el fetch de datos antes de que el usuario haga clic.

Generación de Sitios Estáticos (SSG): Si el dashboard tiene vistas que cambian una vez al día, genera el HTML en el servidor (AOT) para que el navegador del cliente solo descargue texto plano.

Caché in-memory Indexada: Al arrancar el CRM, descarga los catálogos estáticos enteros y guárdalos en un Map (JS) o HashMap (Rust) indexado por ID.

Base64 para Imágenes Pequeñas: Convierte iconos pequeños a cadenas Base64 y mételos en el archivo CSS/JS para reducir a cero las peticiones HTTP extras.

Tree Shaking Estricto: Asegúrate de importar librerías a lo bruto (ej. import { cloneDeep } from 'lodash') para que el compilador elimine todo el código muerto del bundle.

Despliegue de Servidores HTTP a HTTP/2 o HTTP/3: Esto soluciona la multiplexación. Permite mandar docenas de archivos WASM y JS por el mismo túnel TCP sin la sobrecarga de nuevas conexiones.

Vistas Materializadas en Backend: Si un reporte en el CRM es lento, refactoriza la base de datos para que el motor SQL guarde una tabla estática precalculada cada noche.

Carga Perezosa de Componentes (Lazy Loading): Empaqueta la aplicación en trozos (Chunks). La pantalla de login no debe cargar el código del módulo de facturación.

Compresión Brotli/Gzip Dinámica/Estática: Obliga al servidor a servir el binario de WebAssembly comprimido en Brotli. En redes lentas, ahorra segundos enteros de carga.

Pool de Web Workers para Parseo JSON: Desvía los JSON pesados que devuelva tu API a un Web Worker para ejecutar JSON.parse sin congelar la UI principal del navegador.

Normalización de Estado: Estructura tu estado global (Redux, Zustand) de forma plana, relacional (tipo base de datos por IDs) para evitar iterar sobre arrays anidados inmensos.

Event Delegation: En lugar de poner mil addEventListener a cada fila de una tabla masiva de clientes, pon uno solo en el contenedor padre y captura el objetivo.

Precálculo de Layouts (Renderizado Fuera de Pantalla): Mide componentes matemáticamente o renderízalos ocultos al inicio, guarda sus alturas y utilízalas para animaciones inmediatas.

WebSockets Persistentes: Para CRMs en tiempo real, refactoriza el polling (hacer fetch cada 5 segundos) e implementa conexiones Websocket crudas.

Reemplazo de Expresiones Regulares (RegEx): Las regex complejas matan el rendimiento en navegadores lentos. Refactorízalas a métodos .indexOf o .split nativos para búsquedas de texto plano masivas.

Inlining de Configuración: Incrusta las variables de entorno o configuraciones del cliente directamente en el objeto global de JS al construir, evitando peticiones de configuración al inicio.

Sustitución de Moment.js / Librerías de Fechas Pesadas: Elimina librerías monstruosas y refactoriza a la API nativa Intl.DateTimeFormat soportada por el navegador.

Despliegue de SQLite Empaquetado: Distribuir bases de datos SQLite pre-pobladas (con códigos postales, monedas) que la app descargue de una vez y consulte localmente, no a la API.

Memoización Funcional Dinámica: En JS, si una función de cálculo matemático es pura y pesada, enrrólala en un wrapper que guarde los parámetros previos y devuelva el resultado de caché al instante.

Reemplazo de Arrays Dinámicos por Maps Estáticos: Si buscas constantemente si existe un rol de usuario o etiqueta, refactoriza el Array.includes O(N) por un Set.has o Map.get O(1) de JS.

Agrupamiento Masivo de APIs (GraphQL o Batch Endpoints): Reestructura el backend para que la carga inicial requiera solo 1 megaconsulta que traiga el usuario, roles, y notificaciones de un solo golpe.

Estilización CSS en Construcción (Tailwind/Zero-Runtime CSS): Descarta librerías de styled-components que generan clases CSS en tiempo de ejecución. Pasa a utilitarios que generan CSS estático en el build.

Paginación Virtualizada (Windowing): Si el CRM carga una lista de 5,000 contactos, usa librerías que solo creen nodos DOM para los 20 contactos visibles en el viewport y reciclen los divs al scrollear.

Intersección por Observer nativo: Reemplaza lógicas basadas en eventos scroll con el IntersectionObserver de JS para diferir la carga de imágenes o lógicas hasta que se vean en pantalla.

Caché de Service Workers: Instala un SW pesado en la primera carga para cachear todo el shell (HTML, JS, CSS, WASM) localmente, logrando inicios instantáneos e independientes de la red.

String Interning en Backend (Rust): Si analizas millones de líneas de texto, guarda referencias a una única cadena de texto estática en la memoria en vez de clonar el String cada vez.

Aplicaciones: 20 Técnicas Avanzadas
SQLite WASM sobre OPFS (Origin Private File System): Refactoriza la persistencia local de IndexDB a SQLite puro compilado a WebAssembly, escribiendo directamente al disco del navegador con rendimiento cuasi-nativo.

FlatBuffers / Cap'n Proto (Bypass de Deserialización): Reemplaza JSON para comunicación cliente/servidor. La respuesta de la API es un binario que Rust en WASM lee instantáneamente como punteros, cero tiempo de parseo.

Compilación de Enrutamiento a Autómatas: En backend (Rust), usa macros para compilar todas tus rutas web (/api/users/:id) a una máquina de estados finita. Emparejar una URL toma nanosegundos (ej. router de Axum).

Partición de Memoria Lineal Wasm: Separa manualmente la memoria de WebAssembly en 'bloques fijos'. El bloque 1 es estado UI, el bloque 2 es datos locales, evitando fragmentaciones por malloc.

AOT de Frameworks Frontend: Usa frameworks como Svelte o SolidJS que, en lugar de enviar un motor reactivo en JS a tu navegador, compilan tu código a manipulaciones DOM imperativas y directas en tiempo de build.

Bloom Filters en Caché: Al arrancar el CRM, genera un filtro de Bloom matemático. Si un empleado busca una factura que no existe, el sistema sabe en un microsegundo que ni siquiera debe consultarlo al servidor.

Deltas de Estado con Parches Binarios: Cuando un registro masivo del CRM cambia, el backend (Rust) no envía el objeto completo nuevo, envía solo un parche binario microscópico de los bytes mutados para actualizar en WASM.

WebAssembly SIMD para Procesamiento de Datos: Tienes arreglos gigantes de finanzas en el frontend; delégalos a Wasm con core::arch::wasm32 para sumar o mapear registros usando vectores hardware de 128 bits.

Desactivar Comprobaciones Estándar de Rust Backend: Tras asegurar las lógicas clave, compilar el backend en perfil release con lto = 'fat', codegen-units = 1 y abortar pánico (panic = 'abort') para generar binarios microscópicos y rápidos.

Asignación Bump de Larga Vida en WASM: Crea un allocator que asigna toda la memoria necesaria para una pestaña del CRM, y cuando el usuario cierra esa vista, el puntero vuelve a cero masivamente (cero drop o free por elemento).

Índices Locales B-Tree Estáticos: Descarga el catálogo de clientes completo en binario bruto, donde el archivo ya viene estructurado como un árbol B. Búsquedas locales sin motor de base de datos intermedio.

Pre-Compilación de Plantillas (Handlebars/Tera): Las plantillas de HTML o emails en el servidor de Rust deben ser parseadas al AST en el inicio de la app (lazy_static / OnceLock), nunca en tiempo de la petición.

Despacho Estático vs Dinámico (Devirtualización): En tu arquitectura de Rust, limpia refactors basados en Box<dyn Trait> (herencia/polimorfismo dinámico) y pásalos a Genéricos (impl Trait), forzando al compilador a generar funciones estáticas.

Deduplicación de Hilos en WorkersJS: Si 5 componentes piden procesamiento pesado simultáneo, crea un gestor (Queue) en JS que agrupe los datos y mande 1 solo bloque enorme al WebWorker WASM, evitando cuellos de botella del bus de comunicación.

Predictive Prefetching por Probabilidad Estática: Analiza los logs del CRM. Si 'ver cliente' lleva a 'ver deudas' 90% del tiempo, incrusta esa regla pura en código. Al tocar cliente, descarga deudas de fondo.

Memory-Mapped Files (mmap) en Backend: Reemplaza la lectura estándar de archivos pesados. Deja que el sistema operativo mapee directamente el archivo del disco duro a la RAM de tu proceso en Rust.

Arquitectura Cero-Strings Temporales: Refactoriza el análisis de datos (CSVs importados, logs) usando librerías o técnicas que pasen 'trozos de cadena' (&str en Rust, subarrays en JS) que referencian al archivo original cargado, nunca instanciando copias.

Diccionarios de Interfaz Inmutables Hash-Map: Convierte todo el texto multi-idioma (i18n) durante el pipeline en código estático compilado en Rust y exportado a WASM, evitando parseos de i18n dinámicos.

Caché In-Memory L1D de Punteros Aislados (Rust): Diseñar estructuras de structs asegurando que los IDs críticos quepan juntos en los 64-bytes de una línea de caché típica de un procesador (#[repr(C)] con layouts forzados).

Alineación Forzada de Memoria JS (DataView): En vez de objetos estándar, guardar campos de datos críticos financieros del CRM alineados rígidamente dentro de un gran ArrayBuffer gestionado vía DataView para garantizar acceso cache-friendly.

Aplicaciones: 10 Técnicas Súper Innovadoras y de Vanguardia (Sangrantes pero Viables)
Autómata Aho-Corasick Compilado en WebAssembly: Construye una máquina de búsqueda estática a nivel binario para el CRM. Busca instantáneamente 10,000 groserías, nombres clave o tags leyendo cada byte del texto una sola vez a costo O(n).

File System Access API como Base de Datos Directa: Omitir IndexDB. Solicitar acceso a una carpeta nativa de la PC del usuario y usar Rust/WASM para escribir datos binarios del CRM directo a un archivo físico crudo persistente en su disco, superando cualquier abstracción de almacenamiento web.

Renderizado de Interfaz Custom GPU en Canvas: Despédete del DOM pesado para tablas infinitas (Grids). Compila una librería UI nativa en Rust e inyéctala para pintar la tabla directamente de píxeles sobre un <canvas> usando WebGL/WebGPU. El DOM ni se entera.

Custom Allocator de Arena Dinámica para JSONs: Programar un asignador de memoria (Allocator) en Rust específico para ingesta masiva de APIs. Se asigna una bloque enorme, los JSON deserializados viven ahí, y cuando la petición termina, el bloque entero 'muere' en 1 ciclo de reloj.

Snapshot Inicial Restaurado (Wizer.wasm): Herramientas como Wizer pre-ejecutan la inicialización de tu módulo WASM localmente, toman una 'foto' de la RAM lista y la guardan. El usuario descarga un WASM que arranca sin hacer ningún paso de inicialización, saltando directo a su estado en memoria.

Lógica de Validación Basada en Árboles Merkle Locales: Trasladar la integridad y verificación masiva de transacciones grandes del CRM a Rust usando Hashes en Árbol; en segundos el frontend de una tostadora verifica la matemática de miles de recibos sin recalcular el negocio entero.

Caché L1 Forzada vía Atomics en JS (Atomics.wait): En Workers intensivos para minería de datos o manipulación financiera web, bloquear estratégicamente el hilo trabajador forzando sincronicidad matemática que evita que el event-loop de JS pierda tiempo cambiando de contextos inútilmente.

Embeddings Locales ONNX en WASM (Sin Servidor): Convertir las lógicas de sugerencia predictiva (IA de clasificación de leads) a modelos matemáticos de flotantes mínimos, ejecutando la inferencia vía WebAssembly enteramente en la CPU local del cliente al cargar, apagando solicitudes cloud.

ECS para Gestión de Estado Web Masivo: Traer el Entity Component System de los juegos de vuelta a las aplicaciones web complejas. Cada div, botón y texto masivo es un ente en WASM; todo su estado y estilo son Componentes, iterados por sistemas contiguos para destruir reactividades lentas estilo Vue/React.

Re-escritura Estática en Pipeline a Lenguaje Máquina (JIT Tuning AOT): Al detectar navegadores de tostadora, el servidor empaqueta y despacha un WASM compilado explícitamente para arquitecturas 32-bits (sin optimizaciones modernas que un chip de Celeron intentaría emular fallidamente, sino con instrucciones súper reducidas).





REGLAS DE SEGURIDAD PARA PROCESOS (OBLIGATORIO - SI LAS ROMPES MATAS AL SERVIDOR):
- Tenés PROHIBIDO TERMINANTEMENTE usar cualquier comando de PowerShell para matar procesos. Esto incluye: `taskkill`, `Stop-Process`, `tskill`, `wmic process delete`, `Get-Process | Stop-Process`, `kill`, o cualquier variante similar.
- Para matar un proceso que hayas spawnado, usá EXCLUSIVAMENTE la herramienta `kill_process` con el PID exacto que recibiste en la respuesta del `execute_powershell`.
- Esto es crítico: si ejecutás `taskkill /F /IM cargo.exe` o `taskkill /F /IM rustc.exe` o `taskkill /F /IM powershell.exe` matás al servidor principal. Si matás por PID un proceso equivocado (que no sea tuyo), también podés matar al servidor.
- La herramienta `kill_process` es la ÚNICA forma segura de matar procesos. Solo puede matar procesos que vos mismo spawnaste y están registrados internamente.


Una llamada a una tool tiene que tener este aspecto (Lo que te voy a pasar ahora es meramente un ejemplo y nada más, no tenés que clonarlo.): \"\"\"{



\"content\": \"Now I have the full picture. The game renders everything as flatcolored rectangles — no textures at all. I have the Roguelike Modern City pack (CC0) downloaded. Let me explore the spritesheet, verify what we have, and download more complementary assets:\",



\"reasoning_content\": \"OK, now I have a clear picture:

1. The Roguelike Modern City pack has been downloaded and extracted. It contains:
 - `roguelikeCity_magenta.png` (56 KB) - spritesheet with magenta background
 - `roguelikeCity_transparent.png` (45 KB) - spritesheet with transparent background
 - `spritesheetInfo.txt` - information about sprite positions
 - `Preview.png` and `Sample.png` - preview images
 - License is CC0

2. The current render.rs renders everything with colored rectangles and circles- no textures at all. It's purely geometric shapes with flat colors.

Now I need to:

1. Download more free CC0 assets that would complement the city builder aesthetic
2. Build a texture system in Rust that can load PNG files at startup
3. Create a sprite mapping system that maps building types to sprites
4. Replace the rectangle rendering with textured sprite rendering

Let me first check the spritesheet info, look at the assets we have, and download more from OpenGameArt and itch.io.

Let me explore the spritesheet info and also search for more free assets.\",



\"role\": \"assistant\",



\"tool_calls\": [



{



\"function\": {



\"arguments\": \"{\"command\": \"Get-Content 'C:\\Users\\Fa\\Desktop\\IAF\\citybound\\refactor\\assets\\textures\\kenney\\roguelike_modern_city\\Spritesheet\\spritesheetInfo.txt'\"}\",



\"name\": \"execute_powershell\"



},



\"id\": \"call_00_6LBp0TNGOGSENqrTWNuY6481\",



\"index\": 0,



\"type\": \"function\"



},



{



\"function\": {



\"arguments\": \"{\"command\": \"Get-Content 'C:\\Users\\Fa\\Desktop\\IAF\\citybound\\refactor\\assets\\textures\\kenney\\roguelike_modern_city\\License.txt'\"}\",



\"name\": \"execute_powershell\"



},



\"id\": \"call_01_PSbacOPIEg08V5Z5xuYJ3661\",



\"index\": 1,



\"type\": \"function\"



},



{



\"function\": {



\"arguments\": \"{\"url\": \"https://opengameart.org/sites/default/files/roguelikeCity_magenta.png\"}\",



\"name\": \"image_fetch\"



},



\"id\": \"call_02_oGyw2emIhRVg0yehobAm9686\",



\"index\": 2,



\"type\": \"function\"



}



]



},\"\"\"







































Si tiene este aspecto, no lo estás pasando al sistema. Me lo estás pasando a mí, por lo tanto el sistema no lo va a ejecutar: \"\"\" {



\"content\": \"```json
{
 \"tool\": \"execute_powershell\",
 \"arguments\": {
 \"command\": \"cd \"C:\\Users\\Fa\\Desktop\\IAF\" ; (Get-Content \"src/main.rs\").Count ; echo \"---\" ; Select-String -Path \"src/main.rs\" -Pattern \"DEFAULT_GLOBAL_SYSTEM_PROMPT\" | Select-Object LineNumber, Line\",
 \"work_dir\": \"C:\\Users\\Fa\\Desktop\\IAF\"
 }
}
```\",



\"role\": \"assistant\"


},\"\"\"



REGLAS OBLIGATORIAS DE SEGURIDAD DE PROCESOS (ÓRDENES EXPLÍCITAS Y DIRECTAS):
1. PROHIBICIÓN ABSOLUTA DE MATAR PROCESOS POR NOMBRE: Está TERMINANTEMENTE PROHIBIDO usar taskkill /IM, taskkill /F /IM, Stop-Process -Name, Get-Process | Stop-Process, pkill, killall, o cualquier comando que mate procesos por nombre de ejecutable. Estos comandos matan TODAS las instancias del proceso, incluyendo este servidor principal.
2. PROHIBICIÓN DE MATAR CARGO/RUSTC/IAF: Nunca ejecutes comandos como \"Get-Process cargo | Stop-Process\", \"taskkill /IM cargo.exe\", \"taskkill /IM rustc.exe\", \"taskkill /IM iaf.exe\" o similares. Si matas cargo.exe, matarás también al servidor principal que te está ejecutando.
3. ÚNICA FORMA PERMITIDA DE MATAR PROCESOS: Para matar un proceso hijo, DEBES usar exclusivamente la herramienta `kill_process` con el PID exacto devuelto por `execute_powershell`. Cualquier intento de usar execute_powershell con comandos de matar procesos será bloqueado por el sistema.
4. GUARDA DE PIDs: Cuando `execute_powershell` devuelve un PID en su respuesta, debes registrarlo y usarlo posteriormente con `kill_process`. No intentes adivinar PIDs ni usar otros métodos.
5. CONSECUENCIA: Si matas el proceso del servidor principal, toda tu sesión terminará abruptamente y perderás todo el progreso.";





#[derive(Deserialize)]
pub struct LocalProjectRequest {
    pub name: String,
    pub path: String,
}

async fn add_local_project(State(state): State<AppState>, Json(payload): Json<LocalProjectRequest>) -> impl IntoResponse {
    let path = PathBuf::from(&payload.path);
    if !path.exists() || !path.is_dir() {
        return Json(json!({ "status": "error", "message": "El directorio especificado no existe o no es una carpeta válida." }));
    }

    let mut projs = state.projects.lock().unwrap();
    // Validar duplicados
    if projs.iter().any(|p| p.name == payload.name) {
        return Json(json!({ "status": "error", "message": "Ya existe un proyecto con ese nombre." }));
    }

    projs.push(Project {
        name: payload.name.clone(),
        path: payload.path.clone(),
        is_local: true,
    });

    // Guardar en la configuración local de proyectos si se desea, o persistirlo dinámicamente
    // Aquí actualizamos el archivo de prompts/config para guardar los proyectos locales
    let config_dir = state.base_workspace.join(".config");
    let local_config_path = config_dir.join("local_projects.json");
    let _ = fs::write(&local_config_path, serde_json::to_string_pretty(&*projs).unwrap());

    Json(json!({ "status": "ok" }))
}

// Historial de Chats endpoints
#[derive(Serialize, Deserialize, Clone)]
struct ChatSessionSummary {
    id: String,
    title: String,
    project_name: Option<String>,
}

async fn get_chats(State(state): State<AppState>) -> impl IntoResponse {
    let chats_dir = state.base_workspace.join(".config").join("chats");
    let mut summaries = Vec::new();
    if let Ok(entries) = fs::read_dir(chats_dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                        summaries.push(ChatSessionSummary {
                            id: session.id,
                            title: session.title,
                            project_name: session.project_name,
                        });
                    }
                }
            }
        }
    }
    Json(summaries)
}

async fn get_chat_session(State(state): State<AppState>, AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", id));
    if chat_file.exists() {
        if let Ok(content) = fs::read_to_string(chat_file) {
            if let Ok(session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                return Json(json!({ "status": "ok", "session": session }));
            }
        }
    }
    Json(json!({ "status": "error", "message": "No se encontró el chat." }))
}

// Auditoría e Interrupción endpoints
async fn get_agent_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.active_agent.lock().unwrap().clone();
    Json(json!({
        "running": status.running,
        "interrupted": status.interrupted,
        "esperando_respuesta_usuario": status.esperando_respuesta_usuario,
        "pregunta_usuario": status.pregunta_usuario,
        "esperando_aprobacion_plan": status.esperando_aprobacion_plan,
        "plan_propuesto": status.plan_propuesto,
        "thinking_content": status.thinking_content,
        "steps": status.steps,
        "current_session_id": status.current_session_id,
    }))
}

async fn interrupt_agent(State(state): State<AppState>) -> impl IntoResponse {
    let mut status = state.active_agent.lock().unwrap();
    if status.running {
        status.interrupted = true;
        status.esperando_respuesta_usuario = false;
        status.esperando_aprobacion_plan = false;
        status.steps.push(crate::state::AuditStep {
            step_type: "error".to_string(),
            title: "Interrumpido por el usuario".to_string(),
            detail: "Se envió una señal manual de interrupción.".to_string(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        });
        Json(json!({ "status": "ok", "message": "Agente marcado para interrupción." }))
    } else {
        Json(json!({ "status": "error", "message": "El agente no está corriendo." }))
    }
}

#[derive(Deserialize)]
struct RespondRequest {
    respuesta: String,
}

async fn respond_to_agent(State(state): State<AppState>, Json(payload): Json<RespondRequest>) -> impl IntoResponse {
    let mut status = state.active_agent.lock().unwrap();
    if status.esperando_respuesta_usuario {
        let respuesta = payload.respuesta.clone();
        status.respuesta_usuario = Some(payload.respuesta);
        status.esperando_respuesta_usuario = false;
        
        // Guardar la respuesta del usuario en el archivo JSON de la conversación
        if let Some(ref session_id) = status.current_session_id {
            let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", session_id));
            if chat_file.exists() {
                if let Ok(content) = fs::read_to_string(&chat_file) {
                    if let Ok(mut session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                        session.messages.push(crate::state::ChatMessage {
                            role: "user".to_string(),
                            content: respuesta,
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        });
                        let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                    }
                }
            }
        }
        
        Json(json!({ "status": "ok", "message": "Respuesta enviada al agente." }))
    } else {
        Json(json!({ "status": "error", "message": "El agente no está esperando respuesta." }))
    }
}

#[derive(Deserialize)]
struct ApprovePlanRequest {
    aprobado: bool,
}

async fn approve_agent_plan(State(state): State<AppState>, Json(payload): Json<ApprovePlanRequest>) -> impl IntoResponse {
    let mut status = state.active_agent.lock().unwrap();
    if status.esperando_aprobacion_plan {
        status.esperando_aprobacion_plan = false;
        if payload.aprobado {
            status.steps.push(crate::state::AuditStep {
                step_type: "thinking".to_string(),
                title: "Plan Aprobado".to_string(),
                detail: "El usuario aprobó el plan propuesto. Continuando...".to_string(),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
        } else {
            status.interrupted = true;
            status.steps.push(crate::state::AuditStep {
                step_type: "error".to_string(),
                title: "Plan Rechazado".to_string(),
                detail: "El usuario rechazó el plan. Ejecución cancelada.".to_string(),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            });
        }
        Json(json!({ "status": "ok" }))
    } else {
        Json(json!({ "status": "error", "message": "El agente no está esperando aprobación de plan." }))
    }
}

#[derive(Deserialize)]
struct RefinePromptRequest {
    prompt: String,
    feedback: Option<String>,
    session_id: Option<String>,
    project_name: Option<String>,
}

async fn refine_prompt_endpoint(State(state): State<AppState>, Json(payload): Json<RefinePromptRequest>) -> impl IntoResponse {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build() {
            Ok(c) => c,
            Err(e) => return Json(json!({ "status": "error", "message": format!("Error creando cliente HTTP: {}", e) })),
        };
    
    // Obtener System Prompt Global Actual
    let global_prompt = {
        let prompts = state.prompts.lock().unwrap();
        prompts.global_current.clone()
    };

    // Obtener System Prompt Local si existe para este proyecto
    let local_prompt = payload.project_name.as_ref().and_then(|name| {
        let prompts = state.prompts.lock().unwrap();
        prompts.projects.get(name).cloned()
    });

    let system_prompt_context = if let Some(ref local) = local_prompt {
        format!("{}\n\n[PROMPT LOCAL DEL PROYECTO ACTIVO]:\n{}", global_prompt, local)
    } else {
        global_prompt
    };

    // Obtener Memorias locales del archivo MEMORIES.md del proyecto si existe
    let mut memories_content = "No hay archivo MEMORIES.md registrado en este proyecto aún.".to_string();
    if let Some(ref proj_name) = payload.project_name {
        // Buscar ruta física de la carpeta del proyecto
        let projs = state.projects.lock().unwrap();
        let proj_path_opt = projs.iter().find(|p| p.name == *proj_name).map(|p| p.path.clone());
        let final_proj_path = proj_path_opt.unwrap_or_else(|| state.base_workspace.join(proj_name).to_string_lossy().to_string());
        
        let memories_path = std::path::Path::new(&final_proj_path).join("MEMORIES.md");
        if memories_path.exists() {
            if let Ok(content) = fs::read_to_string(memories_path) {
                memories_content = content;
            }
        }
    } else {
        // Comprobar si existe en la raíz base_workspace por defecto
        let memories_path = state.base_workspace.join("MEMORIES.md");
        if memories_path.exists() {
            if let Ok(content) = fs::read_to_string(memories_path) {
                memories_content = content;
            }
        }
    }

    let refine_system_prompt = format!("Eres un refinador experto en prompts de IA. Tu objetivo es estructurar, mejorar y corregir prompts.
Debes mantener estrictamente el formato estructurado en 5 bloques en español:
1. Rol y Contexto (Rol de programador principal en Rust/JS/HTML).
2. Meta Técnica Rígida (Qué se quiere hacer exactamente).
3. Restricciones y Reglas (Prohibido asumir, prohibido crear APIs externas innecesarias, código optimizado obligatoriamente para correr en un Pentium de 4GB RAM y 2 cores).
4. Formato de Salida (Código limpio, comentarios inline).
5. Datos de Soporte (Mencionar archivos relevantes).

Se te provee el SYSTEM PROMPT global y local del proyecto que guiará al agente principal, junto a las MEMORIAS locales persistentes de limitaciones técnicas del proyecto:

---
**[SYSTEM PROMPT DEL AGENTE PRINCIPAL (GLOBAL + LOCAL)]**
{}
---

---
**[MEMORIAS DEL PROYECTO ACTUAL (MEMORIES.md)]**
{}
---

Si el usuario te provee un prompt base y una retroalimentación/instrucción adicional de ajuste, debes aplicarla sobre el prompt base y devolver el prompt final estructurado entero.
Adicionalmente, se te inyectará el historial reciente del chat para que entiendas de qué elementos o archivos (como 'el botón azul') se venía hablando en mensajes anteriores, de modo que el prompt refinado mantenga la coherencia total. No agregues introducciones ni explicaciones; empieza directamente con el prompt final estructurado.", system_prompt_context, memories_content);

    let mut api_messages = vec![
        json!({ "role": "system", "content": refine_system_prompt }),
    ];

    // Cargar historial de chat si session_id está presente para dar contexto al refinador
    if let Some(ref s_id) = payload.session_id {
        let chat_file = state.base_workspace.join(".config").join("chats").join(format!("{}.json", s_id));
        if chat_file.exists() {
            if let Ok(content) = fs::read_to_string(&chat_file) {
                if let Ok(session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                    // Tomar los últimos 6 mensajes para no saturar el contexto de refinado
                    let start_idx = session.messages.len().saturating_sub(6);
                    for m in &session.messages[start_idx..] {
                        let role = if m.role == "agent" { "assistant" } else { "user" };
                        // Sanitizar para evitar meter el reporte de auditoría completo
                        let clean_content = if m.content.contains("**[Auditoría de Herramientas Ejecutadas]**") {
                            m.content.split("**[Auditoría de Herramientas Ejecutadas]**").next().unwrap_or("").trim().to_string()
                        } else {
                            m.content.clone()
                        };
                        api_messages.push(json!({ "role": role, "content": clean_content }));
                    }
                }
            }
        }
    }

    api_messages.push(json!({ "role": "user", "content": format!("Prompt base a refinar:\n```\n{}\n```", payload.prompt) }));

    if let Some(ref fb) = payload.feedback {
        if !fb.trim().is_empty() {
            api_messages.push(json!({ "role": "user", "content": format!("Instrucción adicional de modificación:\n```\n{}\n```", fb) }));
        }
    }

    let response = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", deepseek_key()))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "deepseek-v4-flash",
            "messages": api_messages
        }))
        .send()
        .await;

    match response {
        Ok(res) => {
            if let Ok(res_val) = res.json::<serde_json::Value>().await {
                let refined = res_val["choices"][0]["message"]["content"].as_str().unwrap_or(&payload.prompt).to_string();
                Json(json!({ "status": "ok", "refined": refined }))
            } else {
                Json(json!({ "status": "error", "message": "Error decodificando respuesta de refinación." }))
            }
        }
        Err(e) => {
            Json(json!({ "status": "error", "message": format!("Error en llamada de refinamiento: {}", e) }))
        }
    }
}

#[derive(Deserialize)]
struct ChatInput {
    message: String,
    project_name: Option<String>,
    session_id: Option<String>, // Para continuar chat o iniciar uno nuevo
}

async fn chat_endpoint(State(state): State<AppState>, Json(payload): Json<ChatInput>) -> impl IntoResponse {
    // 1. Determinar el Session ID
    let session_id = payload.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let chats_dir = state.base_workspace.join(".config").join("chats");
    let _ = fs::create_dir_all(&chats_dir);
    let chat_file = chats_dir.join(format!("{}.json", session_id));

    // 2. Cargar sesión existente o crear una nueva
    let mut session = if chat_file.exists() {
        if let Ok(content) = fs::read_to_string(&chat_file) {
            serde_json::from_str::<crate::state::ChatSession>(&content).unwrap_or_else(|_| crate::state::ChatSession {
                id: session_id.clone(),
                title: "Nueva conversación".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            })
        } else {
            crate::state::ChatSession {
                id: session_id.clone(),
                title: "Nueva conversación".to_string(),
                messages: Vec::new(),
                project_name: payload.project_name.clone(),
                steps: None,
            }
        }
    } else {
        // Generar título descriptivo conciso usando DeepSeek V4 Flash
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        let prompt_title = format!(
            "Analiza el siguiente mensaje de usuario y genera un título descriptivo muy corto (máximo 4 palabras) en español que resuma el tema. No agregues comillas ni explicaciones:\n\n\"{}\"",
            payload.message
        );
        
        let mut generated_title = payload.message.chars().take(28).collect::<String>();
        
        let response_title = client
            .post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", deepseek_key()))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": "deepseek-v4-flash",
                "messages": [
                    { "role": "user", "content": prompt_title }
                ]
            }))
            .send()
            .await;

        if let Ok(res) = response_title {
            if let Ok(res_val) = res.json::<serde_json::Value>().await {
                if let Some(content) = res_val["choices"][0]["message"]["content"].as_str() {
                    let clean_title = content.trim().replace("\"", "").replace("'", "");
                    if !clean_title.is_empty() {
                        generated_title = clean_title;
                    }
                }
            }
        }

        crate::state::ChatSession {
            id: session_id.clone(),
            title: generated_title,
            messages: Vec::new(),
            project_name: payload.project_name.clone(),
            steps: None,
        }
    };

    // 3. Guardar el nuevo mensaje del usuario
    let user_msg = crate::state::ChatMessage {
        role: "user".to_string(),
        content: payload.message.clone(),
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
    };
    session.messages.push(user_msg);
    let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());

    // 4. Cancelar la tarea del agente anterior si ya estaba activa
    {
        let mut handle_opt = state.abort_handle.lock().unwrap();
        if let Some(ref handle) = *handle_opt {
            println!("Abortando agente activo anterior debido a la recepción de un nuevo mensaje de usuario...");
            handle.abort();
        }
        *handle_opt = None;
    }

    // 5. Preparar el agente activo
    {
        let mut status = state.active_agent.lock().unwrap();
        status.running = true;
        status.interrupted = false;
        status.current_session_id = Some(session_id.clone());
        
        // Mantener e inyectar el historial acumulado de pasos en la consola en lugar de borrarlo
        status.steps.clear();
        if let Some(ref prev_steps) = session.steps {
            status.steps.extend(prev_steps.clone());
        }
        
        status.steps.push(crate::state::AuditStep {
            step_type: "thinking".to_string(),
            title: "Reanudando Agente".to_string(),
            detail: format!("Procesando nueva instrucción en la conversación activa. Proyecto: {:?}", payload.project_name),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        });
    }

    // 6. Correr el bucle del agente asíncronamente o en este hilo pero reportando pasos
    // Para que no bloquee y permita interrupciones en tiempo real, lo ejecutamos asíncronamente en una tarea de Tokio
    let state_clone = state.clone();
    let project_name_clone = payload.project_name.clone();
    let session_id_clone = session_id.clone();
    let session_messages_clone = session.messages.clone();

    let handle = tokio::spawn(async move {
        // Envolver run_agent_loop en su propio tokio::spawn para aislar y atrapar pánicos
        let agent_task = tokio::spawn(run_agent_loop(
            session_messages_clone,
            project_name_clone,
            state_clone.clone(),
            deepseek_key(),
            voyage_key(),
            openrouter_key(),
            Some(session_id_clone.clone()),
        ));
        let run_result = match agent_task.await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(e)) => Err(format!("Error de ejecución: {}", e)),
            Err(join_err) => {
                if join_err.is_panic() {
                    // Obtener el payload del pánico
                    let panic_any = join_err.into_panic();
                    // Convertir el payload a String segura
                    let panic_detail = if let Some(s) = panic_any.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_any.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(b) = panic_any.downcast_ref::<Vec<u8>>() {
                        // Convertir bytes a UTF‑8 con pérdida de datos si es necesario
                        String::from_utf8_lossy(b).to_string()
                    } else {
                        // Fallback: representación de depuración
                        format!("{:?}", panic_any)
                    };
                    // Guardar en archivo de log persistente
                    let logs_dir = state_clone.base_workspace.join(".config").join("logs");
                    let _ = std::fs::create_dir_all(&logs_dir);
                    let log_path = logs_dir.join("panic.log");
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_path)
                        .and_then(|mut file| {
                            use std::io::Write;
                            writeln!(
                                file,
                                "[{}] Pánico crítico en el agente: {}",
                                chrono::Utc::now().to_rfc3339(),
                                panic_detail,
                            )
                        });
                    Err(format!("Pánico crítico en el agente: {}", panic_detail))
                } else {
                    Err(format!("Error crítico en la tarea de ejecución del agente: {}", join_err))
                }
            }

        };

        let (agent_reply, is_success) = match run_result {
            Ok(reply) => (reply, true),
            Err(err_msg) => {
                eprintln!("{}", err_msg);
                crate::agent::play_error_beep();
                (err_msg, false)
            }
        };

        // Registrar paso de finalización o error en memoria
        {
            let mut status = state_clone.active_agent.lock().unwrap();
            status.running = false;
            if is_success {
                status.steps.push(crate::state::AuditStep {
                    step_type: "done".to_string(),
                    title: "Ejecución finalizada".to_string(),
                    detail: "El agente terminó de responder y procesar herramientas.".to_string(),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
            } else {
                status.steps.push(crate::state::AuditStep {
                    step_type: "error".to_string(),
                    title: "Error en ejecución".to_string(),
                    detail: agent_reply.clone(),
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
            }
        }

        // Guardar respuesta en la sesión junto a la consola de auditoría
        let chat_file_async = state_clone.base_workspace.join(".config").join("chats").join(format!("{}.json", session_id_clone));
        if let Ok(content) = fs::read_to_string(&chat_file_async) {
            if let Ok(mut current_session) = serde_json::from_str::<crate::state::ChatSession>(&content) {
                // Obtener los pasos detallados de auditoría de herramientas (incluido el paso de finalización/error)
                let active_steps = {
                    let status = state_clone.active_agent.lock().unwrap();
                    status.steps.clone()
                };

                // Guardar los pasos de la auditoría directamente de manera persistente en la sesión
                current_session.steps = Some(active_steps);

                current_session.messages.push(crate::state::ChatMessage {
                    role: "agent".to_string(),
                    content: agent_reply,
                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                });
                let _ = fs::write(&chat_file_async, serde_json::to_string_pretty(&current_session).unwrap());
            }
        }

        // Limpiar el abort handle al finalizar
        {
            let mut handle_opt = state_clone.abort_handle.lock().unwrap();
            *handle_opt = None;
        }
    });

    {
        let mut handle_opt = state.abort_handle.lock().unwrap();
        *handle_opt = Some(handle.abort_handle());
    }

    Json(json!({ "status": "ok", "session_id": session_id }))
}

async fn captcha_status(State(state): State<AppState>) -> impl IntoResponse {
    let cap = state.pending_captcha.lock().unwrap().clone();
    Json(cap)
}

// Captcha handling removed (unused)

// --- Handlers de control de escritorio ---
#[derive(Deserialize)]
struct MoveMouseRequest { x: i32, y: i32 }

async fn move_mouse_handler(State(state): State<AppState>, Json(payload): Json<MoveMouseRequest>) -> impl IntoResponse {
    let controller = state.desktop.lock().unwrap();
    match controller.move_mouse(payload.x, payload.y) {
        Ok(_) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

#[derive(Deserialize)]
struct ClickRequest { button: String }

async fn click_handler(State(state): State<AppState>, Json(payload): Json<ClickRequest>) -> impl IntoResponse {
    let controller = state.desktop.lock().unwrap();
    match controller.click(&payload.button) {
        Ok(_) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

#[derive(Deserialize)]
struct TypeTextRequest { text: String }

async fn type_text_handler(State(state): State<AppState>, Json(payload): Json<TypeTextRequest>) -> impl IntoResponse {
    let controller = state.desktop.lock().unwrap();
    match controller.type_text(&payload.text) {
        Ok(_) => Json(json!({ "status": "ok" })),
        Err(e) => Json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

#[derive(Deserialize)]
struct LaunchRequest { path: String }

async fn launch_handler(State(state): State<AppState>, Json(payload): Json<LaunchRequest>) -> impl IntoResponse {
    let controller = state.desktop.lock().unwrap();
    match controller.launch_executable(&payload.path) {
        Ok(pid) => Json(json!({ "status": "ok", "pid": pid })),
        Err(e) => Json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

#[tokio::main]
async fn main() {
    let base_workspace = PathBuf::from("c:\\Users\\Fa\\Desktop\\IAF");
    let config_dir = base_workspace.join(".config");
    fs::create_dir_all(&config_dir).unwrap_or_default();
    let _ = fs::create_dir_all(config_dir.join("chats"));
    
    let config_path = config_dir.join("prompts.json");
    let mut prompts = PromptConfig {
        global_default: DEFAULT_GLOBAL_SYSTEM_PROMPT.to_string(),
        global_current: DEFAULT_GLOBAL_SYSTEM_PROMPT.to_string(),
        projects: HashMap::new(),
    };

    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(parsed) = serde_json::from_str::<PromptConfig>(&content) {
                prompts = parsed;
            }
        }
    } else {
        let _ = fs::write(&config_path, serde_json::to_string_pretty(&prompts).unwrap());
    }

    // Inicializar lista de proyectos descubiertos
    let mut initial_projects = Vec::new();
    let local_config_path = config_dir.join("local_projects.json");
    if local_config_path.exists() {
        if let Ok(content) = fs::read_to_string(&local_config_path) {
            if let Ok(parsed) = serde_json::from_str::<Vec<Project>>(&content) {
                initial_projects = parsed;
            }
        }
    }
    let state = AppState {
        config_path,
        prompts: Arc::new(Mutex::new(prompts)),
        projects: Arc::new(Mutex::new(initial_projects)),
        base_workspace,
        pending_captcha: Arc::new(Mutex::new(None)),
        active_agent: Arc::new(Mutex::new(ActiveAgentStatus::default())),
        abort_handle: Arc::new(Mutex::new(None)),
        desktop: Arc::new(Mutex::new(DesktopController::new())),
        image_store: Arc::new(Mutex::new(HashMap::new())),
        context_store: Arc::new(Mutex::new(HashMap::new())),
        process_registry: ProcessRegistry::new(),
    };
    };
    // Auto-descubrir proyectos locales por defecto
    discover_projects(&state);

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .nest_service("/", ServeDir::new("public"))
        .route("/api/projects", get(get_projects))
        .route("/api/projects/fork", post(fork_project))
        .route("/api/projects/local", post(add_local_project))
        .route("/api/prompts", get(get_prompts).post(save_prompts))
        .route("/api/prompts/reset", post(reset_global_prompt))
        .route("/api/chat", post(chat_endpoint))
        .route("/api/chats", get(get_chats))
        .route("/api/chats/:id", get(get_chat_session))
        .route("/api/chats/:id/summarize_steps", post(summarize_chat_steps))
        .route("/api/agent/status", get(get_agent_status))
        .route("/api/agent/interrupt", post(interrupt_agent))
        .route("/api/agent/responder", post(respond_to_agent))
        .route("/api/agent/aprobar_plan", post(approve_agent_plan))
        .route("/api/prompts/refine", post(refine_prompt_endpoint))
        .route("/api/captcha/status", get(captcha_status))
        .nest_service("/assets/images", ServeDir::new("src/assets/images"))
        .route("/api/desktop/move", post(move_mouse_handler))
        .route("/api/desktop/click", post(click_handler))
        .route("/api/desktop/type", post(type_text_handler))
        .route("/api/desktop/launch", post(launch_handler))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error fatal: No se pudo enlazar (bind) al puerto {}: {}", addr, e);
            std::process::exit(1);
        }
    };
    println!("Servidor Agent-First iniciado en http://{}", addr);
    
    match axum::serve(listener, app).await {
        Ok(_) => {
            println!("El servidor de Axum se detuvo de forma limpia (Ok).");
        }
        Err(e) => {
            eprintln!("El servidor de Axum terminó con un error: {}", e);
        }
    }
}

async fn get_projects(State(state): State<AppState>) -> impl IntoResponse {
    discover_projects(&state);
    let projs = state.projects.lock().unwrap().clone();
    Json(projs)
}

#[derive(Deserialize)]
struct ForkRequest {
    repo_url: String,
}

async fn fork_project(State(state): State<AppState>, Json(payload): Json<ForkRequest>) -> impl IntoResponse {
    let output = std::process::Command::new("gh")
        .args(&["repo", "fork", &payload.repo_url, "--clone"])
        .current_dir(&state.base_workspace)
        .output();

    match output {
        Ok(out) => {
            discover_projects(&state);
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            Json(json!({ "status": "ok", "stdout": stdout, "stderr": stderr }))
        }
        Err(e) => {
            Json(json!({ "status": "error", "message": format!("Error corriendo gh CLI: {}", e) }))
        }
    }
}

async fn get_prompts(State(state): State<AppState>) -> impl IntoResponse {
    let p = state.prompts.lock().unwrap().clone();
    Json(p)
}

#[derive(Deserialize)]
struct SavePromptsRequest {
    global_current: String,
    projects: HashMap<String, String>,
}


async fn save_prompts(State(state): State<AppState>, Json(payload): Json<SavePromptsRequest>) -> impl IntoResponse {
    let mut prompts = state.prompts.lock().unwrap();
    prompts.global_current = payload.global_current;
    prompts.projects = payload.projects;
    let _ = fs::write(&state.config_path, serde_json::to_string_pretty(&*prompts).unwrap());
    Json(json!({ "status": "ok" }))
}

async fn reset_global_prompt(State(state): State<AppState>) -> impl IntoResponse {
    let mut prompts = state.prompts.lock().unwrap();
    prompts.global_current = prompts.global_default.clone();
    let _ = fs::write(&state.config_path, serde_json::to_string_pretty(&*prompts).unwrap());
    Json(json!({ "status": "ok", "global_current": prompts.global_current.clone() }))
}

async fn summarize_chat_steps(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> impl IntoResponse {
    let chats_dir = state.base_workspace.join(".config").join("chats");
    let chat_file = chats_dir.join(format!("{}.json", session_id));

    if !chat_file.exists() {
        return Json(json!({ "status": "error", "message": "No se encontró la sesión de chat." }));
    }

    let mut session = match fs::read_to_string(&chat_file) {
        Ok(content) => match serde_json::from_str::<crate::state::ChatSession>(&content) {
            Ok(s) => s,
            Err(_) => return Json(json!({ "status": "error", "message": "Error al deserializar la sesión." })),
        },
        Err(e) => return Json(json!({ "status": "error", "message": format!("Error leyendo sesión: {}", e) })),
    };

    let steps_to_summarize = session.steps.clone().unwrap_or_default();
    if steps_to_summarize.is_empty() {
        return Json(json!({ "status": "error", "message": "No hay pasos que resumir en esta sesión." }));
    }

    let mut steps_text = String::new();
    for (i, step) in steps_to_summarize.iter().enumerate() {
        steps_text.push_str(&format!(
            "Paso #{}: Tipo={}, Título={}\nDetalle: {}\n\n",
            i + 1, step.step_type, step.title, step.detail
        ));
    }

    let mut messages_text = String::new();
    for msg in &session.messages {
        messages_text.push_str(&format!("{}: {}\n\n", msg.role, msg.content));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let payload = json!({
        "model": "deepseek-v4-flash",
        "messages": [
            {
                "role": "system",
                "content": "Eres un auditor técnico experto. Tu tarea es resumir el proceso de auditoría y los pasos de ejecución que te provee el usuario, teniendo en cuenta la conversación. El objetivo principal de este resumen es reducir drásticamente el tamaño del contexto de ejecución del agente autónomo para ahorrar costos de tokens y optimizar su memoria a largo plazo, permitiéndole continuar su ejecución de forma fluida. Tu resumen en español DEBE responder obligatoriamente de manera estructurada a: 1. ¿Qué estaba haciendo el agente en ese mismo instante? 2. ¿Por qué lo hacía? 3. ¿Qué le faltaba por hacer? 4. ¿Cuál era el objetivo final y qué objetivos secundarios ya se cumplieron? 5. SOBRE TODO, el 'CÓMO': Detalla minuciosamente el método técnico empleado (herramientas usadas, archivos específicos modificados o leídos, y resultados de compilación/operaciones). Redacta el contenido en formato markdown técnico, muy directo y sin introducciones."
            },
            {
                "role": "user",
                "content": format!(
                    "--- HISTORIAL DE CONVERSACIÓN ---\n{}\n\n--- PASOS DETALLADOS A RESUMIR ---\n{}",
                    messages_text,
                    steps_text
                )
            }
        ]
    });

    match client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", deepseek_key()))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(res_val) = resp.json::<serde_json::Value>().await {
                    if let Some(summary_text) = res_val["choices"][0]["message"]["content"].as_str() {
                        let new_step = crate::state::AuditStep {
                            step_type: "thinking".to_string(),
                            title: "Resumen de Contexto (Forzado)".to_string(),
                            detail: summary_text.to_string(),
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        };
                        
                        session.steps = Some(vec![new_step.clone()]);
                        
                        session.messages.push(crate::state::ChatMessage {
                            role: "agent".to_string(),
                            content: format!("--- RESUMEN DE PASOS DE AUDITORÍA ---\n{}", summary_text),
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        });
                        
                        let _ = fs::write(&chat_file, serde_json::to_string_pretty(&session).unwrap());
                        
                        {
                            let mut active = state.active_agent.lock().unwrap();
                            if active.current_session_id.as_deref() == Some(&session.id) {
                                active.steps = vec![new_step];
                            }
                        }

                        return Json(json!({ "status": "ok", "summary": summary_text }));
                    }
                }
            }
            Json(json!({ "status": "error", "message": "Error al leer la respuesta de la API de DeepSeek." }))
        }
        Err(e) => {
            Json(json!({ "status": "error", "message": format!("Error de conexión: {}", e) }))
        }
    }
}


