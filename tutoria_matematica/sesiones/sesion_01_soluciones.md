# 📝 Soluciones — Sesión 1

> **⚠️ IMPORTANTE:** No mires las soluciones hasta que HAYAS INTENTADO resolver cada ejercicio por tu cuenta. El aprendizaje está en el intento, no en leer la respuesta.

---

## Nivel 1 (calentamiento)

### Ejercicio 1
**Calcular 1+2+3+...+50 sin sumar uno por uno.**

**Solución:**
Usamos la fórmula de Gauss: $S = \frac{n(n+1)}{2}$ con $n=50$.

$$S = \frac{50 \times 51}{2} = \frac{2550}{2} = 1275$$

**Verificación mental:** La suma debe ser un poco más grande que $25 \times 50 = 1250$ (el promedio por número es ~25.5), y 1275 es razonable. ✓

---

### Ejercicio 2
**Calcular 1+3+5+...+99 (suma de impares hasta 99).**

**Solución:**
$99 = 2n-1 \implies 2n = 100 \implies n = 50$

Hay 50 números impares del 1 al 99. La suma de los primeros $n$ impares es $n^2$.

$$S = 50^2 = 2500$$

**Verificación:** La suma de Gauss de 1 a 100 es $100 \times 101 / 2 = 5050$. Los pares son $2+4+...+100 = 2(1+2+...+50) = 2 \times 1275 = 2550$. Entonces los impares son $5050 - 2550 = 2500$. ✓

---

### Ejercicio 3
**Verificar que $1^3+2^3+3^3+4^3 = (1+2+3+4)^2$**

**Solución:**
- Lado izquierdo: $1 + 8 + 27 + 64 = 100$
- Lado derecho: $(10)^2 = 100$

Se verifica. La fórmula general es $\sum k^3 = (\sum k)^2$. ✓

---

## Nivel 2 (demostraciones)

### Ejercicio 4
**Demostrar que la suma de dos números pares SIEMPRE es par.**

**Solución (demostración directa):**

*Paso 1 — Representación:* Sean $a$ y $b$ dos números pares. Por definición, existen enteros $k$ y $m$ tales que:
$$a = 2k, \quad b = 2m$$

*Paso 2 — Suma:* $a + b = 2k + 2m = 2(k + m)$

*Paso 3 — Conclusión:* $a + b$ es $2 \times$ (un entero), por lo tanto es par. ✓

**El "por qué no otro":** Cualquier número par, por definición, es múltiplo de 2. Sumar dos múltiplos de 2 da otro múltiplo de 2. Es IMPOSIBLE que la suma sea impar porque siempre podés factorizar el 2.

---

### Ejercicio 5
**Demostrar que la suma de dos números impares SIEMPRE es par.**

**Solución:**

*Paso 1 — Representación:* Sean $a$ y $b$ impares. Existen enteros $k$ y $m$ tales que:
$$a = 2k + 1, \quad b = 2m + 1$$

*Paso 2 — Suma:* $a + b = (2k+1) + (2m+1) = 2k + 2m + 2 = 2(k + m + 1)$

*Paso 3 — Conclusión:* Es $2 \times$ (un entero), por lo tanto es par. ✓

**Intuición:** Impar + Impar = "sobran dos unos", y 1+1=2, que es par.

---

### Ejercicio 6
**Demostrar que el producto de dos números impares SIEMPRE es impar.**

**Solución:**

*Paso 1:* $a = 2k+1$, $b = 2m+1$

*Paso 2:* $a \times b = (2k+1)(2m+1) = 4km + 2k + 2m + 1 = 2(2km + k + m) + 1$

*Paso 3:* Es de la forma $2 \times$ (algo) $+ 1$, por lo tanto es impar. ✓

---

### Ejercicio 7
**Encontrar TODOS los enteros n tales que n divide a n+7. Explicar por qué son esos y no otros.**

**Solución:**

Si $n \mid n+7$, entonces existe un entero $k$ tal que:
$$n+7 = n \cdot k$$

Despejando: $7 = n(k-1)$

Por lo tanto, $n$ debe ser un divisor de 7. Los divisores de 7 son: $\pm 1, \pm 7$.

**Verificación de cada caso:**
- $n=1$: ¿$1 \mid 8$? Sí. ✓
- $n=-1$: ¿$-1 \mid 6$? Sí. ✓
- $n=7$: ¿$7 \mid 14$? Sí. ✓
- $n=-7$: ¿$-7 \mid 0$? Sí. ✓

**¿Y otros números como 2, 3, 42, 67?** Si $n=2$, entonces $2 \mid 9$ → FALSO. Si $n=3$, $3 \mid 10$ → FALSO. La ecuación $7 = n(k-1)$ fuerza a que $n$ divida a 7. Como 7 es primo, sus ÚNICOS divisores son $\pm 1, \pm 7$. Es imposible que otro número funcione.

**Respuesta final:** $n \in \{-7, -1, 1, 7\}$

---

## Nivel 3 (estilo olimpiada)

### Ejercicio 8
**Demostrar que entre 5 números enteros cualesquiera, siempre hay dos cuya diferencia es divisible entre 4.**

**Solución (Principio del Palomar):**

*Paso 1 — Identificar los "casilleros":* Al dividir cualquier entero entre 4, los posibles restos son: 0, 1, 2, 3. Hay **4 restos posibles** (4 casilleros).

*Paso 2 — Contar los "objetos":* Tenemos **5 números** (5 objetos).

*Paso 3 — Aplicar el principio:* Como hay más objetos (5) que casilleros (4), al menos dos números caen en el mismo casillero, es decir, tienen el MISMO RESTO al dividir entre 4.

*Paso 4 — Concluir:* Si dos números $a$ y $b$ tienen el mismo resto $r$ al dividir entre 4, entonces:
$$a = 4q_1 + r, \quad b = 4q_2 + r$$
$$a - b = 4(q_1 - q_2)$$

Su diferencia es múltiplo de 4. ✓

**¿Por qué con 4 números no funciona?** Contraejemplo: $\{0, 1, 2, 3\}$. Cada uno tiene un resto distinto al dividir entre 4 (0, 1, 2, 3 respectivamente). Ninguna diferencia entre dos de ellos es divisible entre 4. ¡Necesitás el quinto número para forzar la repetición!

---

## 🧠 Puntos Clave de Esta Sesión

| Concepto | Dónde se usó |
|----------|-------------|
| **Suma de Gauss** | Ejercicio 1 |
| **Suma de impares = n²** | Ejercicio 2 |
| **Suma de cubos = (suma)²** | Ejercicio 3 |
| **Representar par = 2k, impar = 2k+1** | Ejercicios 4, 5, 6 |
| **"Por qué no otro" vía divisores** | Ejercicio 7 |
| **Principio del Palomar** | Ejercicio 8 |
| **Restos al dividir (módulo)** | Ejercicio 8 |

---

## 🔍 Errores Comunes y Cómo Evitarlos

| Error | Corrección |
|-------|-----------|
| Decir "par = 2, 4, 6..." en vez de $2k$ | **Siempre** usá notación algebraica: $2k$ |
| Olvidar los divisores negativos (Ej. 7) | Preguntate: "¿n puede ser negativo?" |
| No verificar la respuesta | Reemplazá tu respuesta en el problema original |
| Olvidar el caso n=0 | El 0 es un entero perfectamente válido |
