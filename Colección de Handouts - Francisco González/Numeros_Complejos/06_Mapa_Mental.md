# 🗺️ MAPA MENTAL DE NÚMEROS COMPLEJOS

```
                         ┌─────────────────────────────────┐
                         │     NÚMEROS COMPLEJOS  ℂ        │
                         │         z = a + bi              │
                         │         i² = -1                 │
                         └──────────────┬──────────────────┘
                                        │
            ┌───────────────────────────┼───────────────────────────┐
            │                           │                           │
            ▼                           ▼                           ▼
    ┌───────────────┐          ┌───────────────┐          ┌───────────────┐
    │   FORMA       │          │    FORMA      │          │    FORMA      │
    │  BINÓMICA     │          │    POLAR      │          │  EXPONENCIAL  │
    │  z = a + bi   │          │ z = r(cosθ +  │          │ z = r·e^(iθ)  │
    │               │          │     i sinθ)   │          │               │
    └───────┬───────┘          └───────┬───────┘          └───────┬───────┘
            │                          │                          │
            │    conversión            │                          │
            ├──────────────────────────┤                          │
            │  r = √(a²+b²)           │                          │
            │  θ = atan2(b,a)         │                          │
            │  a = r cosθ             │                          │
            │  b = r sinθ             │                          │
            │                          │                          │
            ▼                          ▼                          ▼
    ┌───────────────┐          ┌───────────────┐          ┌───────────────┐
    │  OPERACIONES  │          │   OPERACIONES │          │  OPERACIONES  │
    │  EN BINÓMICA  │          │   EN POLAR    │          │ EN EXPONENC.  │
    ├───────────────┤          ├───────────────┤          ├───────────────┤
    │Suma: (a+c)+   │          │               │          │               │
    │      (b+d)i   │          │Mult: r₁r₂·    │          │Mult: r₁r₂·    │
    │               │          │  (cos(θ₁+θ₂)+ │          │  e^i(θ₁+θ₂)   │
    │Resta: (a-c)+  │          │   i sin(θ₁+θ₂)│          │               │
    │       (b-d)i  │          │               │          │Div: r₁/r₂·    │
    │               │          │Div: r₁/r₂·    │          │  e^i(θ₁-θ₂)   │
    │Mult: (ac-bd)+ │          │  (cos(θ₁-θ₂)+ │          │               │
    │      (ad+bc)i │          │   i sin(θ₁-θ₂)│          │Pot: rⁿ·       │
    │               │          │               │          │  e^(inθ)      │
    │Div: mult por   │          │Pot (De Moivre)│          │               │
    │  conjugado    │          │  rⁿ(cos nθ +  │          │Raíz: r^(1/n)· │
    │               │          │   i sin nθ)   │          │  e^i(θ+2πk)/n │
    └───────────────┘          └───────────────┘          └───────────────┘
            │                          │                          │
            └──────────────────────────┼──────────────────────────┘
                                       │
                                       ▼
                         ┌─────────────────────────────────┐
                         │     HERRAMIENTAS CLAVE          │
                         ├─────────────────────────────────┤
                         │                                 │
                         │  CONJUGADO: z̄ = a - bi         │
                         │     • z·z̄ = |z|²               │
                         │     • z es real ⇔ z = z̄        │
                         │     • z imag puro ⇔ z = -z̄     │
                         │                                 │
                         │  MÓDULO: |z| = √(a²+b²)        │
                         │     • |z₁z₂| = |z₁||z₂|        │
                         │     • |z₁+z₂| ≤ |z₁|+|z₂|      │
                         │     • Si |z|=1 → z̄ = 1/z       │
                         │                                 │
                         │  ARGUMENTO: arg(z) ∈ (-π, π]    │
                         │     • arg(z₁z₂) = arg(z₁)+arg(z₂)│
                         │     • arg(zⁿ) = n·arg(z)       │
                         │                                 │
                         └─────────────────────────────────┘
                                       │
                                       ▼
                         ┌─────────────────────────────────┐
                         │   APLICACIONES EN OLIMPIADAS    │
                         ├─────────────────────────────────┤
                         │                                 │
                         │  • Raíces de la unidad           │
                         │    zⁿ=1 → z_k = e^(2πik/n)      │
                         │    Suman 0, forman polígono reg │
                         │                                 │
                         │  • Geometría con complejos       │
                         │    Rotación: mult por e^(iθ)    │
                         │    Triángulo equilátero:        │
                         │      z₁+ωz₂+ω²z₃=0             │
                         │                                 │
                         │  • Sumas trigonométricas         │
                         │    ∑cos → Re(∑e^(iθ))           │
                         │                                 │
                         │  • Desigualdades con módulos     │
                         │    |z₁|-|z₂| ≤ |z₁±z₂| ≤ |z₁|+|z₂|│
                         │                                 │
                         │  • Secuencias: z+1/z            │
                         │    Si z+1/z=1 → an=zⁿ+z⁻ⁿ       │
                         │    tiene período 6              │
                         │                                 │
                         └─────────────────────────────────┘
```

---

## 🎯 LO MÁS IMPORTANTE EN 5 PUNTOS

1. **$i^2 = -1$**. Todo lo demás se deriva de esto. Es la única regla nueva.

2. **Multiplicar = rotar + escalar**. Los complejos NO son solo "pares de números". Son transformaciones geométricas.

3. **El conjugado es tu mejor amigo**. Sirve para dividir, para ver si algo es real, y para un millón de trucos.

4. **De Moivre y Euler hacen magia con potencias**. $(1+i)^{100}$ parece imposible hasta que lo pasás a polar.

5. **Las raíces de la unidad son un arma secreta**. Cada vez que veas sumas de cosenos con ángulos equiespaciados, pensá en raíces de la unidad.

---

## 📁 ARCHIVOS DE ESTA CARPETA

| Archivo | Contenido |
|---------|-----------|
| `01_Leccion_Principal.md` | Teoría completa de 0 a olimpiada |
| `02_Ejercicios_Interactivos.py` | Laboratorio Python con gráficos |
| `02b_Test_Rapido.py` | Tests automatizados (¡que pasen todos!) |
| `03_Complejos_Rust.rs` | Implementación en Rust |
| `04_Problemas_Resueltos.md` | 10 problemas de olimpiada resueltos paso a paso |
| `05_Ejercicios_Para_Vos.md` | 18 ejercicios con soluciones ocultas |
| `06_Mapa_Mental.md` | Este archivo |

---

## 🚀 RUTA DE APRENDIZAJE SUGERIDA

```
Día 1: Leé 01_Leccion_Principal.md (partes 1-6)
       → Operaciones básicas, conjugado, módulo, división
       → Ejecutá 02_Ejercicios_Interactivos.py

Día 2: Leé 01_Leccion_Principal.md (partes 7-11)
       → Plano complejo, forma polar, Euler, De Moivre
       → Hacé ejercicios E1-E7 de 05_Ejercicios_Para_Vos.md

Día 3: Leé 01_Leccion_Principal.md (partes 12-13)
       → Raíces de la unidad, trucos de olimpiada
       → Leé 04_Problemas_Resueltos.md

Día 4: Hacé ejercicios E8-E18 de 05_Ejercicios_Para_Vos.md
       → Los de nivel 3 son desafiantes, ¡no te rindas!

Día 5: Repasá y creá tus propios problemas
       → Modificá el código de Python/Rust para experimentar
```
