# ✍️ CÓMO DEMOSTRAR Y EXPLICAR — De la idea a la solución perfecta

> *"En matemáticas, el arte de hacer preguntas es más valioso que resolver problemas."* — Georg Cantor
>
> Pero cuando te toca resolver, **tenés que saber mostrar lo que pensaste.**

---

## 📍 PARTE 1: ¿Qué es una demostración?

### La idea más importante

Una demostración es un **camino lógico** que lleva de lo que sabés (hipótesis) a lo que querés probar (tesis). No es magia: es una cadena de pasos donde cada eslabón se justifica con una razón válida.

```
HIPÓTESIS  →  paso 1  →  paso 2  →  ...  →  paso n  →  TESIS
              (¿por qué?) (¿por qué?)         (¿por qué?)
```

### La regla de oro

> ⚡ **Cada afirmación que hagas debe estar justificada.**
>
> Si decís "entonces $x$ es par", tenés que poder decir POR QUÉ: ¿por la definición? ¿por un teorema? ¿por una cuenta?

---

## 📍 PARTE 2: Los 7 métodos fundamentales de demostración

### 🗡️ MÉTODO 1: Demostración Directa

**Idea:** Agarrás las hipótesis y avanzás en línea recta hacia la tesis.

**Estructura:**
```
1. Suponemos que [HIPÓTESIS] es verdadera.
2. Haciendo cuentas/lógica llegamos a...
3. ...[PASOS INTERMEDIOS]...
4. Por lo tanto [TESIS] es verdadera.
```

**Ejemplo:** Demostrar que el producto de dos números pares es par.

> **Demostración:** Sean $a$ y $b$ dos números pares. Entonces $a = 2k$ y $b = 2m$ para algunos enteros $k, m$. Su producto es $ab = (2k)(2m) = 4km = 2(2km)$. Como $2km$ es entero, $ab$ es múltiplo de 2, es decir, par. ∎

### 🔄 MÉTODO 2: Contrarrecíproco

**Idea:** En vez de probar "$P \implies Q$", probás "no $Q \implies$ no $P$". Es lo mismo lógicamente, pero a veces más fácil.

**Cuándo usarlo:** Cuando la negación de la conclusión te da más información que la hipótesis original.

**Ejemplo:** Si $n^2$ es impar, entonces $n$ es impar.

> **Demostración (contrarrecíproco):** Probamos: si $n$ es par, entonces $n^2$ es par.
>
> Si $n$ es par, $n = 2k$. Entonces $n^2 = 4k^2 = 2(2k^2)$, que es par. ∎

### 💥 MÉTODO 3: Reducción al Absurdo (Contradicción)

**Idea:** Suponés que la tesis es FALSA y llegás a algo imposible (como $1=0$). Entonces tu suposición era incorrecta y la tesis es verdadera.

**Cuándo usarlo:** Para probar que algo NO existe, o unicidad, o con infinitos.

**Ejemplo:** $\sqrt{2}$ es irracional.

> **Demostración:** Supongamos que $\sqrt{2}$ es racional: $\sqrt{2} = \frac{p}{q}$ con $p, q$ enteros coprimos y $q \neq 0$.
>
> Elevando al cuadrado: $2 = \frac{p^2}{q^2}$, o sea $p^2 = 2q^2$. Entonces $p^2$ es par, así que $p$ es par: $p = 2k$.
>
> Sustituyendo: $(2k)^2 = 2q^2 \implies 4k^2 = 2q^2 \implies 2k^2 = q^2$. Entonces $q^2$ es par, así que $q$ es par.
>
> ¡Pero $p$ y $q$ no pueden ser ambos pares porque son coprimos! Contradicción.
>
> Por lo tanto, $\sqrt{2}$ no es racional. ∎

### 🪜 MÉTODO 4: Inducción Matemática

**Idea:** Como fichas de dominó cayendo. Probás que la primera cae (caso base) y que si una cae, la siguiente también (paso inductivo).

**Estructura:**
```
1. CASO BASE: Verificamos que P(1) es verdadero.
2. HIPÓTESIS INDUCTIVA: Suponemos que P(k) es verdadero para algún k ≥ 1.
3. PASO INDUCTIVO: Demostramos que P(k+1) es verdadero usando la hipótesis.
4. CONCLUSIÓN: Por inducción, P(n) es verdadero para todo n ≥ 1.
```

**Ejemplo:** $1 + 2 + 3 + \cdots + n = \frac{n(n+1)}{2}$

> **Demostración:**
> - **Caso base ($n=1$):** $1 = \frac{1 \cdot 2}{2} = 1$. ✓
> - **Hipótesis inductiva:** Suponemos $1 + 2 + \cdots + k = \frac{k(k+1)}{2}$.
> - **Paso inductivo:**
>   $1 + 2 + \cdots + k + (k+1) = \frac{k(k+1)}{2} + (k+1) = \frac{k(k+1) + 2(k+1)}{2} = \frac{(k+1)(k+2)}{2}$. ✓
> - **Conclusión:** La fórmula vale para todo $n \in \mathbb{N}$. ∎

### 🕊️ MÉTODO 5: Principio del Palomar (Pigeonhole)

**Idea:** Si metés $n+1$ palomas en $n$ casillas, al menos una casilla tiene 2 o más palomas.

**Cuándo usarlo:** Problemas de existencia donde algo se repite.

**Ejemplo:** En cualquier grupo de 13 personas, al menos 2 nacieron el mismo mes.

> **Demostración:** Hay 12 meses (casillas) y 13 personas (palomas). Por principio del palomar, al menos 2 personas comparten mes. ∎

### 🧊 MÉTODO 6: Invariantes

**Idea:** Encontrás una cantidad que NO cambia durante un proceso, y la usás para demostrar que cierto estado es imposible (o inevitable).

**Estructura:**
```
1. Identificás una cantidad I que depende del estado.
2. Demostrás que I no cambia con cada movimiento/jugada.
3. Calculás I en el estado inicial y en el estado deseado.
4. Si son distintos → IMPOSIBLE llegar.
```

### 🏔️ MÉTODO 7: Principio Extremal

**Idea:** Considerás el elemento "más grande", "más chico", "más a la izquierda", etc. Ese elemento extremo suele tener propiedades especiales que rompen el problema.

**Cuándo usarlo:** Problemas de geometría combinatoria, configuraciones, grafos.

---

## 📍 PARTE 3: Cómo escribir una solución DE OLIMPIADA

### La estructura perfecta

Toda solución de olimpiada tiene 4 partes:

```
┌─────────────────────────────────────────────────┐
│ 1. ENCABEZADO: "Vamos a demostrar que..."       │
│    (Le decís al lector qué vas a hacer)          │
├─────────────────────────────────────────────────┤
│ 2. ESTRATEGIA: "La idea es..." (OPCIONAL pero    │
│    suma puntos — muestra que entendés el problema)│
├─────────────────────────────────────────────────┤
│ 3. DESARROLLO: Los pasos lógicos, uno por uno,   │
│    cada uno justificado.                          │
├─────────────────────────────────────────────────┤
│ 4. CIERRE: "Por lo tanto..." (conclusión clara)  │
│    El ∎ o QED o una caja: □                      │
└─────────────────────────────────────────────────┘
```

### Antes y después — El mismo problema, dos formas de explicarlo

#### ❌ MAL (así NO se escribe):

> $n=2k+1$, $n^2=4k^2+4k+1=2(2k^2+2k)+1$ entonces es impar.

**Problemas:** No dice qué es $n$, no explica qué está demostrando, no hay conclusión, no se entiende.

#### ✅ BIEN (así SÍ se escribe):

> **Afirmación:** El cuadrado de un número impar es impar.
>
> **Demostración:** Sea $n$ un número impar. Por definición, existe un entero $k$ tal que $n = 2k + 1$.
>
> Calculamos $n^2$:
> $$n^2 = (2k+1)^2 = 4k^2 + 4k + 1 = 2(2k^2 + 2k) + 1$$
>
> Como $2k^2 + 2k$ es un entero, $n^2$ se escribe como $2m + 1$ con $m = 2k^2+2k$ entero. Por definición, $n^2$ es impar. ∎

---

## 📍 PARTE 4: Los 10 mandamientos de la explicación

### 1. DEFINÍ TUS VARIABLES
Antes de usar $n$, $x$, $k$, decí qué son: "Sea $n$ un entero positivo...", "Sea $x \in \mathbb{R}$...".

### 2. UNA IDEA POR PÁRRAFO
Cada párrafo debe transmitir UNA sola idea. Si cambiás de idea, cambiá de párrafo.

### 3. MOSTRÁ EL CAMINO, NO SOLO EL DESTINO
No escribas solo las cuentas. Decí "Vamos a probar que..." o "La estrategia es..." o "Observemos que...".

### 4. JUSTIFICÁ CADA PASO
No asumas que el lector sabe por qué $x$ es par. Decilo: "como $x^2$ es par, $x$ debe ser par (porque el cuadrado de un impar es impar)".

### 5. USÁ CONECTORES LÓGICOS
Palabras como "por lo tanto", "entonces", "así que", "en consecuencia", "sin embargo", "por otro lado", "análogamente" guían al lector.

### 6. SÉ PRECISO CON LA NOTACIÓN
$\implies$ (implica), $\iff$ (si y solo si), $\in$ (pertenece), $\forall$ (para todo), $\exists$ (existe). Usalas BIEN o no las uses.

### 7. CASOS ESPECIALES POR SEPARADO
Si tu demostración falla para $n=0$ o $x=1$, tratá esos casos APARTE al principio.

### 8. DIBUJÁ (EN GEOMETRÍA)
En geometría, un dibujo NO reemplaza la demostración pero la ACOMPAÑA. Nombrá los puntos en el dibujo igual que en el texto.

### 9. REVISÁ QUE NO HAYA SALTOS LÓGICOS
Leé tu solución como si fueras otra persona. ¿Cada paso se sigue del anterior?

### 10. CERRÁ CON UNA CONCLUSIÓN
Terminá con "Por lo tanto, [lo que querías probar]" y el ∎. El lector debe saber que terminaste.

---

## 📍 PARTE 5: Cómo se corrigen las olimpiadas (rúbrica)

En las olimpiadas, cada problema vale 7 puntos. Así se asignan:

| Puntos | Significado |
|--------|-------------|
| 0 | No hizo nada o todo mal |
| 1 | Algún avance mínimo (escribió la hipótesis, un caso trivial) |
| 2 | Avance significativo pero lejos de la solución |
| 3 | Llegó a la mitad del camino |
| 4 | Casi llega, pero hay un error importante o faltan casos |
| 5 | La solución está esencialmente bien pero falta rigor |
| 6 | Solución completa con pequeños detalles de redacción |
| 7 | Solución perfecta: clara, rigurosa, bien escrita |

> 🎯 **DATO CLAVE:** La diferencia entre 5 y 7 puntos suele ser la **calidad de la explicación**, no la matemática. ¡Explicar bien suma puntos!

---

## 📍 PARTE 6: Plantillas para cada tipo de demostración

### Plantilla: Demostración Directa
```
Afirmación: Si [HIPÓTESIS], entonces [TESIS].

Demostración: Supongamos que [HIPÓTESIS].

[AQUÍ VA EL RAZONAMIENTO...]

Por lo tanto, [TESIS]. ∎
```

### Plantilla: Contradicción
```
Afirmación: [TESIS].

Demostración: Supongamos, por el absurdo, que [NEGACIÓN DE LA TESIS].

[AQUÍ VA EL RAZONAMIENTO que lleva a una contradicción...]

Esto contradice [HECHO CONOCIDO]. Por lo tanto, nuestra suposición es falsa y [TESIS] es verdadera. ∎
```

### Plantilla: Inducción
```
Afirmación: Para todo n ≥ 1, se cumple P(n).

Demostración por inducción:
• Caso base (n = 1): [VERIFICACIÓN]. ✓
• Hipótesis inductiva: Supongamos que P(k) es verdadero para algún k ≥ 1.
• Paso inductivo: [DEMOSTRACIÓN DE P(k+1) USANDO P(k)].
• Conclusión: Por el principio de inducción, P(n) vale para todo n ≥ 1. ∎
```

### Plantilla: Contrarrecíproco
```
Afirmación: Si P, entonces Q.

Demostración: Probamos el contrarrecíproco: si NO Q, entonces NO P.

Supongamos que NO Q. [RAZONAMIENTO...]. Por lo tanto, NO P.

Así, el contrarrecíproco queda demostrado y la afirmación original es verdadera. ∎
```

---

## 📍 PARTE 7: Errores comunes que te sacan puntos

### ❌ Error 1: "Es obvio que..."
En una demostración, NADA es obvio. Si es obvio, podés justificarlo en una línea.

### ❌ Error 2: Demostrar el recíproco sin querer
Cuidado con $\implies$ vs $\iff$. A veces demostrás "si llueve, el piso se moja" pero el problema te pide "si el piso está mojado, entonces llovió".

### ❌ Error 3: Dividir por cero
Si dividís por $(x-y)$, asegurate de que $x \neq y$. Si no, tratá el caso $x=y$ por separado.

### ❌ Error 4: Usar lo que querés demostrar
No podés decir "como $A=B$ (que es lo que quiero probar), entonces...". Eso es razonamiento circular.

### ❌ Error 5: "Trivial" o "Fácil"
Nunca escribas eso en una olimpiada. Al corrector no le gusta.

### ❌ Error 6: Olvidar casos borde
$n=0$, $n=1$, triángulos degenerados, división por cero. Siempre chequiá los bordes.

### ❌ Error 7: Mezclar $\implies$ y $\iff$
$\implies$ significa "implica" (una dirección). $\iff$ significa "equivale" (ida y vuelta). Si no estás seguro, usá palabras en vez de símbolos.

---

## 📍 PARTE 8: Ejercicios de explicación

### Nivel 1: Reescribir

**E1.** La siguiente "demostración" es correcta matemáticamente pero está PÉSIMAMENTE escrita. Reescribila bien:

> "n impar n=2k+1, n^3-n = n(n-1)(n+1), uno de tres consecutivos es multiplo de 3, y como n es impar, n-1 y n+1 son pares, el producto de dos pares tiene factor 4, entonces n^3-n es multiplo de 12."

**E2.** Lo mismo con esta:

> "sup que no, entonces existe un racional q tal que q^2=3, q=p/q coprimos, p^2=3q^2, entonces 3|p, p=3k, 9k^2=3q^2, q^2=3k^2, 3|q, contradiccion con coprimos, entonces raiz(3) irracional"

### Nivel 2: Identificar errores

**E3.** ¿Qué está mal en esta "demostración" de que $1=2$?

> Sea $a=b$. Multiplicamos por $a$: $a^2 = ab$. Restamos $b^2$: $a^2-b^2 = ab-b^2$. Factorizamos: $(a-b)(a+b) = b(a-b)$. Cancelamos $(a-b)$: $a+b = b$. Como $a=b$: $b+b = b$, o sea $2b = b$. Dividimos por $b$: $2=1$.

**E4.** ¿Qué error hay en esta "demostración" de que todos los números son iguales?

> *Caso base:* Un solo número es igual a sí mismo. ✓
> *Hipótesis:* Cualquier conjunto de $k$ números son todos iguales.
> *Paso:* Tomamos $k+1$ números $\{a_1,...,a_{k+1}\}$. Por hipótesis, $\{a_1,...,a_k\}$ son iguales y $\{a_2,...,a_{k+1}\}$ son iguales. Entonces $a_1=a_2=\cdots=a_{k+1}$.

### Nivel 3: Demostrar y explicar

**E5.** Demostrá que $\sqrt{3}$ es irracional. Escribí la solución como si fuera para una olimpiada.

**E6.** Demostrá que entre dos racionales siempre hay un irracional. Explicá claramente.

**E7.** Demostrá que un número es divisible por 9 si y solo si la suma de sus dígitos es divisible por 9. Escribilo con la estructura de 4 partes.

**E8.** Demostrá que si $p$ es primo y $p \neq 2$, entonces $p$ es de la forma $4k+1$ o $4k+3$. ¿Es cierto el recíproco? Explicá.

---

*Seguí con `02_Ejercicios_Redaccion.py` para practicar identificando buenas y malas explicaciones.*
