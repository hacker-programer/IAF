# 🎯 PROBLEMAS RESUELTOS DE OLIMPIADA

## Problema 1: Una suma que se anula

> **Problema:** Sea $\omega = e^{2\pi i / 2024}$. Calculá:
> $$S = 1 + \omega + \omega^2 + \cdots + \omega^{2023}$$

**Solución:** $\omega$ es una raíz 2024-ésima de la unidad. La suma de todas las raíces n-ésimas de la unidad es 0.

$$S = \frac{\omega^{2024} - 1}{\omega - 1} = \frac{1 - 1}{\omega - 1} = 0$$

> **Respuesta:** $\boxed{0}$

---

## Problema 2: Módulos y conjugados

> **Problema:** Si $|z| = 1$ y $z \neq 1$, demostrá que $\frac{z-1}{z+1}$ es imaginario puro.

**Solución:** Sea $w = \frac{z-1}{z+1}$. Debemos probar que $\Re(w) = 0$, o sea $w = -\bar{w}$.

Como $|z|=1$, tenemos $\bar{z} = 1/z$.

$$\bar{w} = \overline{\left(\frac{z-1}{z+1}\right)} = \frac{\bar{z}-1}{\bar{z}+1} = \frac{1/z - 1}{1/z + 1} = \frac{1-z}{1+z} = -\frac{z-1}{z+1} = -w$$

Por lo tanto $w = -\bar{w}$, así que $\Re(w) = 0$. ∎

---

## Problema 3: Ley del paralelogramo

> **Problema:** Demostrá que para todo $z_1, z_2 \in \mathbb{C}$:
> $$|z_1 + z_2|^2 + |z_1 - z_2|^2 = 2(|z_1|^2 + |z_2|^2)$$

**Solución:** Usamos $|z|^2 = z\bar{z}$.

$|z_1+z_2|^2 = (z_1+z_2)(\bar{z_1}+\bar{z_2}) = z_1\bar{z_1} + z_1\bar{z_2} + z_2\bar{z_1} + z_2\bar{z_2}$

$|z_1-z_2|^2 = (z_1-z_2)(\bar{z_1}-\bar{z_2}) = z_1\bar{z_1} - z_1\bar{z_2} - z_2\bar{z_1} + z_2\bar{z_2}$

Sumando: los términos cruzados se cancelan y queda $2z_1\bar{z_1} + 2z_2\bar{z_2} = 2(|z_1|^2 + |z_2|^2)$. ∎

---

## Problema 4: Raíces cúbicas

> **Problema:** Resolvé $z^3 = -8$ y graficá las soluciones.

**Solución:** $-8 = 8(\cos\pi + i\sin\pi) = 8e^{i\pi}$

$$z_k = \sqrt[3]{8}\left(\cos\frac{\pi + 2\pi k}{3} + i\sin\frac{\pi + 2\pi k}{3}\right), \quad k=0,1,2$$

- $k=0$: $z_0 = 2(\cos\frac{\pi}{3} + i\sin\frac{\pi}{3}) = 2(\frac{1}{2} + \frac{\sqrt{3}}{2}i) = 1 + \sqrt{3}i$
- $k=1$: $z_1 = 2(\cos\pi + i\sin\pi) = -2$
- $k=2$: $z_2 = 2(\cos\frac{5\pi}{3} + i\sin\frac{5\pi}{3}) = 2(\frac{1}{2} - \frac{\sqrt{3}}{2}i) = 1 - \sqrt{3}i$

Forman un triángulo equilátero centrado en el origen.

---

## Problema 5: Triángulo equilátero

> **Problema:** Demostrá que $z_1, z_2, z_3$ forman un triángulo equilátero si y solo si:
> $$z_1^2 + z_2^2 + z_3^2 = z_1z_2 + z_2z_3 + z_3z_1$$

**Solución:** ($\Rightarrow$) Si es equilátero, entonces $z_3 - z_1 = (z_2 - z_1)e^{\pm i\pi/3}$.
Multiplicando y expandiendo se llega a la igualdad.

($\Leftarrow$) La igualdad se reescribe como:
$(z_1 - z_2)^2 + (z_2 - z_3)^2 + (z_3 - z_1)^2 = 0$

Esto implica que las diferencias entre los vértices tienen el mismo módulo y están a 120° entre sí, lo cual solo ocurre en un triángulo equilátero.

---

## Problema 6: Fórmula trigonométrica con complejos

> **Problema:** Usando números complejos, demostrá que:
> $$\cos\frac{\pi}{7} + \cos\frac{3\pi}{7} + \cos\frac{5\pi}{7} = \frac{1}{2}$$

**Solución:** Sea $\zeta = e^{i\pi/7}$. Entonces $\zeta^7 = e^{i\pi} = -1$, o sea $\zeta^7 + 1 = 0$.

Las raíces de $z^7 + 1 = 0$ son $e^{i\pi(2k+1)/7}$ para $k=0,...,6$.

Consideremos $S = \zeta + \zeta^3 + \zeta^5$ (los de exponente impar). Queremos $\Re(S)$.

Por simetría de las raíces: $\zeta + \zeta^3 + \zeta^5 + \zeta^7 + \zeta^9 + \zeta^{11} + \zeta^{13} = 0$

Pero $\zeta^7 = -1$, $\zeta^9 = \zeta^2 \cdot \zeta^7 = -\zeta^2$, etc. Reagrupando:

$S + (-\zeta^2 - \zeta^4 - \zeta^6 - 1) = 0$, así que $S = 1 + \zeta^2 + \zeta^4 + \zeta^6$.

Ahora, $\zeta^2 + \zeta^4 + \zeta^6 = \bar{S}$ (porque $\zeta^6 = \bar{\zeta}$, etc.).

Entonces $S = 1 + \bar{S}$, de donde $S - \bar{S} = 1$, pero $S - \bar{S} = 2i\Im(S)$... 

Alternativamente: $S + \bar{S} = S + (S - 1) = 2S - 1$. Pero $S + \bar{S} = 2\Re(S)$.

Por otro lado, la suma de todas las raíces es 0:
$S + \bar{S} + (-1) = 0$, así que $2\Re(S) - 1 = 0$, y $\Re(S) = \frac{1}{2}$. ∎

---

## Problema 7: Desigualdad con módulos

> **Problema:** Si $|z| \leq 1$, demostrá que $|z^2 + 2z + 3| \leq 6$.

**Solución:** Usamos desigualdad triangular:
$|z^2 + 2z + 3| \leq |z^2| + |2z| + |3| = |z|^2 + 2|z| + 3$

Como $|z| \leq 1$, la función $f(t) = t^2 + 2t + 3$ es creciente en $[0,1]$, así que el máximo está en $t=1$:
$|z|^2 + 2|z| + 3 \leq 1 + 2 + 3 = 6$. ∎

---

## Problema 8: Sistema de ecuaciones

> **Problema:** Resolvé el sistema:
> $$\begin{cases} z + w = 2 \\ zw = 2 \end{cases}$$

**Solución:** $z$ y $w$ son raíces de $t^2 - 2t + 2 = 0$.

$t = \frac{2 \pm \sqrt{4-8}}{2} = \frac{2 \pm 2i}{2} = 1 \pm i$

> **Respuesta:** $\{z,w\} = \{1+i, 1-i\}$

---

## Problema 9: Una suma trigonométrica

> **Problema:** Calculá $\cos\frac{2\pi}{5} + \cos\frac{4\pi}{5} + \cos\frac{6\pi}{5} + \cos\frac{8\pi}{5}$

**Solución:** Las raíces quintas de la unidad son $\omega^k = e^{2\pi i k/5}$, $k=0,1,2,3,4$.

$\cos\frac{2\pi}{5} + \cos\frac{4\pi}{5} + \cos\frac{6\pi}{5} + \cos\frac{8\pi}{5} = \Re(\omega + \omega^2 + \omega^3 + \omega^4)$

Como $1 + \omega + \omega^2 + \omega^3 + \omega^4 = 0$: $\omega + \omega^2 + \omega^3 + \omega^4 = -1$.

> **Respuesta:** $\boxed{-1}$

---

## Problema 10: Sistema con módulos

> **Problema:** Encontrá todos los $z \in \mathbb{C}$ tales que $|z| = 1$ y $|z-1| = \sqrt{2}$.

**Solución:** $|z|=1$ significa que $z$ está en el círculo unitario. 
$|z-1|^2 = 2$ significa $(z-1)(\bar{z}-1) = 2$, o sea $z\bar{z} - z - \bar{z} + 1 = 2$.

Como $z\bar{z} = |z|^2 = 1$: $1 - (z+\bar{z}) + 1 = 2$, así que $z+\bar{z} = 0$.

Por lo tanto $\Re(z) = 0$, y como $|z|=1$, $z = \pm i$. Verificamos:
- $z=i$: $|i-1| = | -1+i| = \sqrt{2}$ ✓
- $z=-i$: $|-i-1| = |-1-i| = \sqrt{2}$ ✓

> **Respuesta:** $\boxed{z = i \text{ o } z = -i}$
