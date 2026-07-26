# 📚 DOCUMENTACIÓN DEL PROYECTO
## Colección de Handouts — Francisco González

> **Propósito:** Material de estudio y entrenamiento para las Olimpiadas Nacionales de Matemática de Uruguay y competencias internacionales (IMO, Olimpiada de Mayo, Cono Sur, Iberoamericana, Rioplatense).

---

## 1. ESTRUCTURA GENERAL DEL PROYECTO

El proyecto es una colección curada de handouts (apuntes teóricos + problemas) organizados por fuente/origen. Además, contiene módulos interactivos para estudiar teoría con código (Python + Rust).

### 1.1 Directorios principales

| Directorio | Contenido | Tipo |
|------------|-----------|------|
| `Numeros_Complejos/` | Módulo interactivo completo: teoría, ejercicios, código Python y Rust, mapa mental | Módulo propio (creado) |
| `CAMPAMENTO BERKELEY.pdf/` | Handouts del campamento Berkeley Math Circle | Handouts externos |
| `Excalibur de las olimpiadas - HKUST/` | ~40 handouts de HKUST (Hong Kong) cubriendo todos los temas | Handouts externos |
| `IMSC/` | Handouts del IMSC (International Math Summer Camp) + mocks + competencia final | Handouts externos |
| `Material Cantu/` | Material del entrenador Carlos Cantú (México): álgebra, geometría, TN, ecuaciones funcionales | Handouts externos |
| `Materiales Talleres General - Uruguay/` | Material de talleres en Uruguay organizado por temas (Cono, IMO, Seminarios) | Handouts externos |
| `MOP USA/` | Material del Mathematical Olympiad Program de USA (geometría) | Handouts externos |
| `Más Importante (Resumen) - Olimpiada Matemática Baja California/` | Resúmenes por tema y nivel (Básico/Avanzado) | Handouts externos |
| `OMA/` | Apuntes de la Olimpiada Matemática Argentina (Nivel 3) | Handouts externos |
| `Pruebas anteriores/` | Pruebas de años anteriores (niveles primaria) | Pruebas |
| `Talleres IMO 2025 URUGUAY/` | Handouts de entrenamiento IMO 2025 Uruguay | Handouts externos |
| `Temarios/` | Temarios y programas de estudio por nivel | Planes de estudio |
| `Temas en general_ Largo/` | Handouts largos organizados por tema (Álgebra/TN, Combi, Geo) | Handouts externos |

### 1.2 Archivos raíz

| Archivo | Descripción |
|---------|-------------|
| `union.py` | Script para extraer texto de PDFs, DOCX, imágenes y unificar en una estructura JSON |
| `Todo el teorico de olimpiadas.pdf` | Compendio masivo de teoría |
| `Como_rendir_una_prueba.pdf` | Guía sobre cómo rendir pruebas de olimpiada |
| `Cómo demostrar cosas.pdf` | Guía sobre técnicas de demostración |
| `Programa de Evan Chen.pdf` | Programa de estudio de Evan Chen (entrenador IMO) |

---

## 2. EL MÓDULO `Numeros_Complejos/`

Es el **módulo modelo** que demuestra el formato ideal de estudio. Sirve como plantilla para crear nuevos módulos de otros temas.

### 2.1 Archivos y su función

| # | Archivo | Contenido | Tipo |
|---|---------|-----------|------|
| 1 | `01_Leccion_Principal.md` | Teoría completa desde cero hasta nivel olimpiada: 14 partes que cubren definición, forma binómica, operaciones, conjugado, módulo, plano complejo, forma polar, Euler, De Moivre, raíces de la unidad, trucos de olimpiada y tabla resumen | Markdown |
| 2 | `02_Ejercicios_Interactivos.py` | Clase `Complejo` implementada didácticamente + ejercicios guiados con salida por consola + visualización con matplotlib | Python |
| 3 | `02b_Test_Rapido.py` | Tests automatizados (sin input del usuario) que validan 10 propiedades fundamentales. La clase `Complejo` está reimplementada para ser autocontenida | Python |
| 4 | `03_Complejos_Rust.rs` | Struct `Complex` con todos los operadores (Add, Sub, Mul, Div), métodos (conjugate, modulus, argument, to_polar, from_polar, pow, roots, is_real, is_pure_imaginary) y un `main()` que ejecuta todos los ejercicios | Rust |
| 5 | `04_Problemas_Resueltos.md` | 10 problemas de olimpiada resueltos paso a paso con explicaciones detalladas | Markdown |
| 6 | `05_Ejercicios_Para_Vos.md` | 18 ejercicios en 3 niveles (fundamentos, intermedio, olimpiada) con soluciones ocultas en tags `<details>` | Markdown |
| 7 | `06_Mapa_Mental.md` | Mapa visual ASCII + resumen de 5 puntos clave + ruta de aprendizaje de 5 días | Markdown |
| 8 | `__pycache__/` | Caché de Python (ignorar) | Auto-generado |

### 2.2 Estructura de `01_Leccion_Principal.md`

Se compone de 14 partes:
- **Partes 1-2:** Motivación, definición de i, forma binómica
- **Partes 3-6:** Operaciones básicas, conjugado, módulo, división
- **Partes 7-9:** Plano complejo, forma polar, multiplicación como rotación+escala
- **Partes 10-12:** Fórmula de Euler, De Moivre, raíces de la unidad
- **Partes 13-14:** Trucos de olimpiada, tabla resumen de fórmulas

### 2.3 Estructura de la clase `Complejo` (Python, línea 21-92 de `02_Ejercicios_Interactivos.py`)

```python
class Complejo:
    __init__(self, a: float, b: float)     # Constructor
    __repr__(self)                          # Representación bonita
    __add__(self, other)                    # Suma
    __sub__(self, other)                    # Resta
    __mul__(self, other)                    # Multiplicación: (ac-bd)+(ad+bc)i
    __truediv__(self, other)               # División: multiplicar por conjugado
    conjugado (property)                    # a - bi
    modulo (property)                       # sqrt(a² + b²)
    argumento (property)                    # atan2(b, a)
    polar(self)                             # (modulo, argumento)
    desde_polar(cls, r, theta)             # Constructor desde polar
    potencia(self, n)                       # De Moivre
    raices(self, n)                         # Raíces n-ésimas
```

### 2.4 Estructura del struct `Complex` (Rust, líneas 13-91 de `03_Complejos_Rust.rs`)

```rust
struct Complex { a: f64, b: f64 }
impl Complex {
    fn new(a, b)           // Constructor
    fn i()                 // Unidad imaginaria
    fn conjugate()         // Conjugado
    fn modulus()           // Módulo
    fn argument()          // Argumento
    fn to_polar()          // (módulo, argumento)
    fn from_polar(r, theta)// Desde polar
    fn pow(n)              // De Moivre
    fn roots(n)            // Raíces n-ésimas
    fn is_real()           // ¿Es real?
    fn is_pure_imaginary() // ¿Es imaginario puro?
}
// Traits: Display, Add, Sub, Mul, Div
```

---

## 3. TEMAS CUBIERTOS POR LOS HANDOUTS

### 3.1 Álgebra
- Desigualdades (Jensen, Muirhead, Rearrangement, pqr/uvw method, smoothing)
- Polinomios (raíces, factorización, teorema fundamental)
- Ecuaciones funcionales (estrategias: substitución, Cauchy, inyectividad/surjectividad)
- Sucesiones y recurrencias
- Números complejos (módulo completo propio)
- Lagrange Interpolation
- Inducción de Cauchy

### 3.2 Combinatoria
- Principios básicos (aditivo, multiplicativo, inclusiones-exclusiones)
- Biyecciones
- Coloreo
- Grafos
- Ramsey
- Probabilistic Method
- Juegos (game theory)
- Invariantes y monovariantes
- Configuraciones discretas
- Tiling
- Funciones generatrices

### 3.3 Geometría
- Teoremas clásicos (Ptolomeo, Menelao, Ceva, Pascal, La Hire)
- Potencia de un punto, ejes radicales, coaxilidad
- Cuadriláteros armónicos
- Inversión
- Homotecia
- Coordenadas baricéntricas
- Números complejos en geometría
- Trigonometría avanzada
- Vectores
- Área method
- Casey's Theorem
- Línea OI, Recta de Euler, puntos notables

### 3.4 Teoría de Números
- Divisibilidad y congruencias
- Aritmética modular
- Teorema chino del resto
- Teorema de Fermat, Euler, Wilson
- Órdenes y raíces primitivas
- LTE (Lifting The Exponent)
- Ecuaciones diofánticas
- Ecuación de Pell
- Zsigmondy's Theorem
- Funciones aritméticas
- Representación en base n
- Suma de dígitos
- Método de descenso infinito
- Cuadrados perfectos
- Factorización en teoría de números
- Finite fields

---

## 4. HERRAMIENTAS DEL PROYECTO

### 4.1 `union.py` (raíz)
Script que recorre recursivamente un directorio, extrae texto de cada archivo (PDF, DOCX, imágenes con OCR, texto plano) y genera una estructura JSON unificada. Opcionalmente imprime o copia al portapapeles.

**Dependencias:** `pypdf`, `python-docx`, `pytesseract`, `Pillow`, `clipboard`

### 4.2 Módulos Python
Usan solo biblioteca estándar + `matplotlib` (opcional para gráficos) + `cmath`/`math`.

### 4.3 Módulos Rust
`03_Complejos_Rust.rs` usa solo `std` (sin dependencias externas). Se ejecuta con `cargo run` o `rustc`.

---

## 5. PERFIL DEL ESTUDIANTE

**Francisco González:**
- 8vo puesto en Olimpiada de Mayo 2024 (9 puntos, nivel 1 — compitiendo con chicos de 13 años)
- Mención de honor en otra competencia
- Se prepara para las Olimpiadas Nacionales de Matemática de Uruguay
- Fortalezas: intuición matemática, resolución de problemas
- Áreas a mejorar: conocimiento sistemático de teoría, redacción de demostraciones y explicaciones

---

## 6. PLAN DE TRABAJO

### Fase 1: Crear sistema de estudio ✓
- [x] `Numeros_Complejos/` — módulo completo
- [ ] `Como_demostrar/` — técnicas de demostración y explicación
- [ ] Más módulos temáticos en el mismo formato

### Fase 2: Expandir a todos los temas
Crear módulos interactivos para cada área:
- Álgebra: Desigualdades, Polinomios, Ecuaciones funcionales
- Combinatoria: Invariantes, Grafos, Conteo
- Geometría: Teoremas clásicos, Inversión, Baricéntricas
- Teoría de Números: Divisibilidad, Congruencias, LTE

---

*Última actualización: Julio 2025*
