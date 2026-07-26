# 🏆 Sesión 1 — Las Bases que Nadie Te Explicó

> **Tutor: IAF | Alumno: Francisco González | Fecha: Julio 2026**
> **Objetivo:** Entender las 4 áreas, aprender las 10 fórmulas que necesitás sí o sí, y dominar el arte de demostrar "por qué X y no Z".

---

## 📍 PARTE 1: El Mapa del Tesoro (Las 4 Áreas)

Imaginate que las olimpiadas son como un juego con 4 mundos:

| Área | De qué trata | Ejemplo de problema |
|------|-------------|---------------------|
| 🔢 **Teoría de Números** | Propiedades de los números enteros: divisibilidad, primos, restos | "¿En qué dígito termina 7²⁰²⁴?" |
| 🧩 **Combinatoria** | Contar cosas sin enumerarlas una por una | "¿De cuántas formas puedo elegir 3 personas de un grupo de 10?" |
| 📐 **Geometría** | Figuras, ángulos, triángulos, círculos | "Demostrar que tal punto está sobre tal recta" |
| 📈 **Álgebra** | Ecuaciones, desigualdades, manipulación simbólica | "Si x + y = 10, ¿cuál es el valor máximo de xy?" |

**Vos dijiste:** *"No sé qué es eso, solo entiendo medianamente geometría y álgebra."*

Esto es normal. Teoría de Números y Combinatoria suenan a nombres raros, pero son habilidades que YA USÁS.

---

## 📍 PARTE 2: Las 10 Fórmulas Que Tenés Que Saber (Aunque No Te Las Sepas)

### 🔴 ERROR QUE COMETISTE: Tuviste que sumar 1+2+...+10 y 1+2+...+7 para "acordarte" de la fórmula.

**La fórmula de Gauss (suma de naturales consecutivos):**

$$\sum_{k=1}^{n} k = 1 + 2 + 3 + ... + n = \frac{n(n+1)}{2}$$

**Demostración VISUAL** (para que nunca más la olvides):

```
Imaginá que escribís los números y abajo los mismos al revés:
  1   +   2   +   3   + ... + (n-1) +   n
  n   + (n-1) + (n-2) + ... +   2   +   1
─────────────────────────────────────────
(n+1) + (n+1) + (n+1) + ... + (n+1) + (n+1)   ← n veces

Suma total = n(n+1)
Pero como escribimos todo dos veces, la verdadera suma es n(n+1)/2 ✓
```

**⚠️ El truco**: NO memorices la fórmula como "n por n+1 sobre 2". Entendé el truco de dar vuelta los números. Si lo entendés, nunca más te vas a trabar en un examen.

---

Aquí van las 10 fórmulas esenciales. **No tenés que memorizarlas como loro. Tenés que entender de dónde vienen.**

### 1. Suma de Gauss
$$1+2+...+n = \frac{n(n+1)}{2}$$

### 2. Suma de los primeros impares
$$1+3+5+...+(2n-1) = n^2$$

**Truco visual:** Los impares forman cuadrados:
```
●         ● ●       ● ● ●
          ● ●       ● ● ●
                    ● ● ●
1=1²      1+3=2²    1+3+5=3²
```

### 3. Suma de los primeros pares
$$2+4+6+...+2n = n(n+1)$$

(Es simplemente 2 × la suma de Gauss)

### 4. Suma de cuadrados
$$1^2+2^2+3^2+...+n^2 = \frac{n(n+1)(2n+1)}{6}$$

### 5. Suma de cubos
$$1^3+2^3+3^3+...+n^3 = \left(\frac{n(n+1)}{2}\right)^2 = (1+2+...+n)^2$$

**¡Esto es hermoso!** La suma de los cubos es el cuadrado de la suma de Gauss.

### 6. Productos notables
- $(a+b)^2 = a^2 + 2ab + b^2$
- $(a-b)^2 = a^2 - 2ab + b^2$
- $(a+b)(a-b) = a^2 - b^2$

### 7. Diferencia de potencias
$$a^n - b^n = (a-b)(a^{n-1} + a^{n-2}b + ... + ab^{n-2} + b^{n-1})$$

(Ej: $a^3 - b^3 = (a-b)(a^2 + ab + b^2)$)

### 8. Números pares e impares
- Par: $2k$
- Impar: $2k+1$ (o $2k-1$)

**REGLA DE ORO:** Cuando un problema dice "número par", escribí $2k$. No digas "bueno, 2, 4, 6...".

### 9. Divisibilidad básica
$a \mid b$ significa "$a$ divide a $b$", es decir $b = a \cdot k$ para algún entero $k$.
- Un número es divisible entre 3 si la suma de sus dígitos es divisible entre 3
- Un número es divisible entre 9 si la suma de sus dígitos es divisible entre 9

### 10. Principio fundamental del conteo
Si tenés $m$ formas de hacer algo y $n$ formas de hacer otra cosa, tenés $m \times n$ formas de hacer ambas.

---

## 📍 PARTE 3: Cómo Demostrar "Por Qué X y No Z"

### 🔴 PROBLEMA QUE DIJISTE: *"Me cuesta horrores explicar por qué X y no Z o 42 o 67"*

Este es EL problema central. Vamos a resolverlo con 3 técnicas:

---

### Técnica 1: **ACOTAR** (encerrar entre paredes)

**Filosofía:** Si podés demostrar que tu respuesta es mayor que A y menor que B, y solo hay un número entre A y B... ¡ese es tu número!

**Ejemplo:** "Encontrar todos los enteros n tales que n² < 2n + 35"

*Paso 1:* Planteás: $n^2 - 2n - 35 < 0$
*Paso 2:* Factorizás: $(n-7)(n+5) < 0$
*Paso 3:* Esto ocurre SOLO cuando $-5 < n < 7$
*Paso 4:* Como n es entero: $n \in \{-4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 6\}$

**¡Acabás de demostrar por qué esos y NINGÚN OTRO!** Porque el producto $(n-7)(n+5)$ solo es negativo en ese intervalo.

---

### Técnica 2: **CASOS** (partir el universo en pedazos)

**Filosofía:** Dividí todas las posibilidades en casos exhaustivos y resolvé cada uno.

**Ejemplo:** "Encontrar todos los enteros n tales que n² − n es divisible entre 6"

*Paso 1:* Todo entero n es de la forma $6k$, $6k+1$, $6k+2$, $6k+3$, $6k+4$, o $6k+5$
*Paso 2:* Evaluás $n^2 - n = n(n-1)$ en cada caso:

| Caso | $n(n-1)$ | ¿Divisible entre 6? |
|------|----------|---------------------|
| $n=6k$ | $6k(6k-1)$ | ✅ (tiene factor 6) |
| $n=6k+1$ | $(6k+1)(6k)$ | ✅ |
| $n=6k+2$ | $(6k+2)(6k+1)$ | ✅ (factor 2 y 3) |
| ... | ... | ... |

*Paso 3:* **Todos los enteros** cumplen la propiedad. Demostrado exhaustivamente.

**🧠 CLAVE:** La palabra "exhaustivo" significa que NO DEJASTE NINGÚN CASO AFUERA. Si cubriste todos los casos posibles, demostraste que solo esos (todos) funcionan.

---

### Técnica 3: **UNICIDAD POR CONTRADICCIÓN** (si hay otro, algo explota)

**Filosofía:** Suponé que hay OTRA solución además de la que encontraste, y demostrá que eso lleva a un absurdo.

**Ejemplo:** "Demostrar que $x=3$ es la ÚNICA solución de $\sqrt{x+1} = 2$"

*Paso 1:* Encontrás que $x=3$ funciona (verificación directa).
*Paso 2:* Suponés que existe OTRO valor $y \neq 3$ que también funciona.
*Paso 3:* $\sqrt{y+1} = 2 \implies y+1 = 4 \implies y = 3$
*Paso 4:* ¡Contradicción! Porque supusimos $y \neq 3$ pero llegamos a $y = 3$.

**Conclusión:** $x=3$ es la única solución. ✓

---

## 📍 PARTE 4: Checklist Anti-Errores Boludos

### 🔴 ERRORES QUE COMETISTE:
- $9 \times 9 = 91$ (en vez de 81)
- No contar un caso extra
- Pusiste 224 cuando era 225

**Solución: El método de los 3 pasos**

```
1. RESOLVÉ → Hacé el problema normalmente
2. VERIFICÁ → Reemplazá tu respuesta en el problema original
3. CONTÁ CASOS → Preguntate: "¿y si n=0? ¿y si n=1? ¿y si es negativo?"
```

**Truco para multiplicaciones:** $9 \times 9$ es $9^2$. Los cuadrados del 1 al 10 DEBEN ser automáticos:
$$1, 4, 9, 16, 25, 36, 49, 64, 81, 100$$

Regla mnemotécnica: "81" rima con "9×9", no hay forma de confundirlo con 91.

---

## 📍 PARTE 5: Ejercicios Para Esta Semana

### Nivel 1 (calentamiento)
1. Calcular $1+2+3+...+50$ sin sumar uno por uno.
2. Calcular $1+3+5+...+99$ (suma de impares hasta 99).
3. Verificar que $1^3+2^3+3^3+4^3 = (1+2+3+4)^2$

### Nivel 2 (demostraciones)
4. Demostrar que la suma de dos números pares SIEMPRE es par.
5. Demostrar que la suma de dos números impares SIEMPRE es par.
6. Demostrar que el producto de dos números impares SIEMPRE es impar.
7. Encontrar TODOS los enteros n tales que n divide a n+7. Explicar por qué son esos y no otros.

### Nivel 3 (estilo olimpiada)
8. Demostrar que entre 5 números enteros cualesquiera, siempre hay dos cuya diferencia es divisible entre 4.
   **Pista:** ¿Cuántos restos posibles hay al dividir entre 4?

---

## 📚 Material Que YA Tenés (y deberías revisar)

| Archivo | Para qué sirve |
|---------|---------------|
| `Cómo demostrar cosas.pdf` | LEE ESTO PRIMERO. Cubre EXACTAMENTE tu problema de "no sé explicar" |
| `Todo el teorico de olimpiadas.pdf` | Tu diccionario de teoremas. No lo leas de corrido, consultalo cuando necesites |
| `Talleres IMO 2025 URUGUAY/` | Handouts de entrenadores uruguayos. ¡Oro puro! |
| `Temarios/Temario de Olimpiadas por nivel.pdf` | El mapa de lo que tenés que saber |
| `Pruebas anteriores/` | Practicá con pruebas reales |

---

**⚠️ Tarea para la próxima sesión:** Traeme UN problema de las pruebas anteriores que no hayas podido resolver. Lo destripamos juntos.

**📊 Tu perfil actual según lo que me contaste:**
- ✅ Geometría y Álgebra: comprensión mediana
- ❌ Teoría de Números: sin conocimiento formal
- ❌ Combinatoria: sin conocimiento formal
- ❌ Demostraciones rigurosas: punto más débil
- ⚠️ Errores de cálculo: mejorar con verificación

**Próxima sesión:** Teoría de Números desde cero (divisibilidad, congruencias, MCD) + práctica de demostraciones con el método de casos.
