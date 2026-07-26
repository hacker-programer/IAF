# 📚 DOCUMENTACIÓN DEL PROYECTO
## Colección de Handouts — Francisco González

> **Propósito:** Material de estudio y entrenamiento para las Olimpiadas Nacionales de Matemática de Uruguay y competencias internacionales (IMO, Olimpiada de Mayo, Cono Sur, Iberoamericana, Rioplatense).

---

## 1. ESTRUCTURA GENERAL DEL PROYECTO

El proyecto es una colección curada de handouts (apuntes teóricos + problemas) organizados por fuente/origen. Además, contiene módulos interactivos para estudiar teoría con código (Python + Rust).

### 1.1 Directorios principales

| Directorio | Contenido | Tipo |
|------------|-----------|------|
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