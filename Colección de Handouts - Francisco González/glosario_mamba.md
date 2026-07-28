# Glosario Mamba — Símbolos y conceptos

## Símbolos básicos

| Símbolo | Nombre | ¿Qué es? | Analogía |
|---------|--------|----------|----------|
| `x_t` | Input / Token | Vector de números que representa la palabra que entra en el paso `t`. | "La palabra que estás leyendo ahora mismo." |
| `h_t` | Estado oculto | Vector que guarda la **memoria comprimida** de todo lo visto hasta el paso `t`. | "Tu bloc de notas mental con espacio limitado." |
| `y_t` | Salida | Vector que produce el modelo en el paso `t` (predicción del siguiente token). | "Lo que el modelo 'dice' después de leer." |
| `A` | Matriz de transición | Matriz (HiPPO) que controla cómo evoluciona `h` por sí solo, **sin input**. Es diagonal en Mamba. | "La inercia de tu memoria: qué tan rápido se te olvidan las cosas." |
| `B` | Matriz de entrada | Controla **cuánto del input** `x_t` se inyecta al estado. En Mamba, **depende de `x_t`**. | "Qué tan fuerte te pega la palabra nueva." |
| `C` | Matriz de salida | Controla **qué parte del estado** se usa para producir la salida. En Mamba, **depende de `x_t`**. | "Qué recuerdo usás para responder." |
| `Δ` (delta) | Paso de tiempo | Escalar que controla **cuánto peso** tiene el token actual. En Mamba, **depende de `x_t`**. | "Qué tan importante es esta palabra." |

## Símbolos discretizados (con barrita)

| Símbolo | Fórmula | ¿Qué es? |
|---------|---------|----------|
| `Ā` | `exp(Δ_t · A)` | Versión discreta de `A`. Controla cuánto del estado viejo se conserva. |
| `B̄` | `(Ā − I) · A⁻¹ · B_t` | Versión discreta de `B`. Controla cuánto del input nuevo entra al estado. |

## Funciones aprendibles

| Símbolo | Definición | ¿Qué hace? |
|---------|-----------|------------|
| `s_B(x)` | `W_B @ x + b_B` | Capa lineal que produce `B_t` a partir del input. |
| `s_C(x)` | `W_C @ x + b_C` | Capa lineal que produce `C_t` a partir del input. |
| `s_Δ(x)` | `softplus(W_Δ @ x + b_Δ)` | Capa lineal + softplus que produce `Δ_t` (siempre positivo). |

## Notación matemática

| Símbolo | Significado |
|---------|-------------|
| `@` | Multiplicación de matrices. Igual que `×` o `·`. |
| `'` (comilla) | Derivada respecto al tiempo. `h'(t)` = "qué tan rápido cambia `h`". |
| `exp()` | Función exponencial: `e^x`. |
| `A⁻¹` | Matriz inversa de `A`. |
| `I` | Matriz identidad (unos en la diagonal, ceros en el resto). |
| `softplus(x)` | `ln(1 + e^x)`. Versión suave de ReLU que siempre da positivo. |

## Ecuación de estado (versión discreta)

```
h_t = Ā · h_{t-1} + B̄_t · x_t
```

> El estado nuevo = (lo que quedó del estado viejo) + (lo que entra del input nuevo)

## Ecuación de salida

```
y_t = C_t · h_t
```

> La salida = (lo que extraigo del estado actual)
