# 🌀 NÚMEROS COMPLEJOS — De cero a olimpiadas

> *"El número imaginario es un maravilloso recurso del espíritu divino, casi un anfibio entre el ser y el no ser."* — Leibniz

---

## 📍 PARTE 1: ¿Por qué necesitamos números nuevos?

### El problema

Imaginate que tenés esta ecuación:

$$x^2 + 1 = 0$$

Si la resolvés: $x^2 = -1$. Pero... ¿qué número al cuadrado da $-1$?

- $1^2 = 1$ ❌
- $(-1)^2 = 1$ ❌  
- $0^2 = 0$ ❌

**¡Ningún número real funciona!** Los matemáticos dijeron: "inventemos uno". Y así nació:

$$\boxed{i = \sqrt{-1}} \quad \text{donde} \quad \boxed{i^2 = -1}$$

A $i$ se le llama la **unidad imaginaria**.

---

## 📍 PARTE 2: Forma binómica

Un número complejo $z$ se escribe como:

$$\boxed{z = a + bi}$$

Donde:
- $a = \Re(z)$ es la **parte real**
- $b = \Im(z)$ es la **parte imaginaria** (¡es un número real!)
- $i$ es la unidad imaginaria

> ⚠️ **Ojo tramposo:** La parte imaginaria es $b$, NO $bi$. Si $z = 3 + 4i$, entonces $\Im(z) = 4$, no $4i$.

### Los números reales están adentro

Si $b = 0$, tenemos $z = a$, un número real común. Los reales son un subconjunto de los complejos:

$$\mathbb{R} \subset \mathbb{C}$$

---

## 📍 PARTE 3: Operaciones básicas

Sean $z_1 = a + bi$ y $z_2 = c + di$.

### Suma y resta

$$\boxed{z_1 \pm z_2 = (a \pm c) + (b \pm d)i}$$

**Ejemplo:** $(3 + 2i) + (1 - 5i) = 4 - 3i$

> Es como sumar vectores: parte real con parte real, imaginaria con imaginaria.

### Multiplicación

$$\boxed{z_1 \cdot z_2 = (ac - bd) + (ad + bc)i}$$

**¿De dónde sale?** Distribuimos normal y usamos $i^2 = -1$:

$(a+bi)(c+di) = ac + adi + bci + bdi^2 = ac + (ad+bc)i + bd(-1) = (ac-bd) + (ad+bc)i$

**Ejemplo:** $(2 + 3i)(1 - i) = 2 - 2i + 3i - 3i^2 = 2 + i + 3 = 5 + i$

---

## 📍 PARTE 4: El conjugado

El conjugado de $z = a + bi$ se denota $\bar{z}$ y es:

$$\boxed{\bar{z} = a - bi}$$

(Se le cambia el signo a la parte imaginaria.)

### Propiedades MÁGICAS del conjugado

| Propiedad | Fórmula |
|-----------|---------|
| Suma/Resta | $\overline{z_1 \pm z_2} = \bar{z_1} \pm \bar{z_2}$ |
| Producto | $\overline{z_1 \cdot z_2} = \bar{z_1} \cdot \bar{z_2}$ |
| Cociente | $\overline{\left(\frac{z_1}{z_2}\right)} = \frac{\bar{z_1}}{\bar{z_2}}$ |
| Doble conjugado | $\bar{\bar{z}} = z$ |
| Real + conjugado | $z + \bar{z} = 2\Re(z) = 2a$ |
| Real − conjugado | $z - \bar{z} = 2i\Im(z) = 2bi$ |
| Producto clave | $\boxed{z \cdot \bar{z} = a^2 + b^2}$ (¡siempre real y ≥ 0!) |

> 🎯 El producto $z\bar{z}$ es **clave** para dividir.

---

## 📍 PARTE 5: Módulo

El módulo de $z = a + bi$ es:

$$\boxed{|z| = \sqrt{a^2 + b^2} = \sqrt{z \cdot \bar{z}}}$$

Es la **distancia al origen** en el plano complejo (¡Pitágoras!).

**Propiedades:**
- $|z| \geq 0$, y $|z| = 0 \iff z = 0$
- $|z_1 \cdot z_2| = |z_1| \cdot |z_2|$
- $\left|\frac{z_1}{z_2}\right| = \frac{|z_1|}{|z_2|}$ (si $z_2 \neq 0$)
- $|z_1 + z_2| \leq |z_1| + |z_2|$ (desigualdad triangular)

---

## 📍 PARTE 6: División

Para dividir $\frac{z_1}{z_2}$, multiplicamos numerador y denominador por $\bar{z_2}$:

$$\boxed{\frac{a+bi}{c+di} = \frac{(a+bi)(c-di)}{(c+di)(c-di)} = \frac{(ac+bd) + (bc-ad)i}{c^2 + d^2}}$$

**Ejemplo:** $\frac{1+i}{1-i} = \frac{(1+i)(1+i)}{(1-i)(1+i)} = \frac{1+2i+i^2}{1+1} = \frac{2i}{2} = i$

---

## 📍 PARTE 7: El plano complejo (diagrama de Argand)

Un número complejo $a+bi$ se representa como el punto $(a,b)$ en el plano:

```
    Im (eje imaginario)
    ^
    |
   b|        * z = a+bi
    |       /|
    |      / |
    |   r /  | b
    |    /   |
    |   /θ   |
 ---+--/-----|-----> Re (eje real)
    |/  a
   0+---------
```

- Eje horizontal: parte real
- Eje vertical: parte imaginaria
- $r = |z|$ es la distancia al origen
- $\theta$ es el **argumento**

---

## 📍 PARTE 8: Forma polar / trigonométrica

En lugar de decir "está en (a,b)", podemos decir "está a distancia $r$ con ángulo $\theta$":

$$\boxed{z = r(\cos\theta + i\sin\theta)}$$

Donde:
- $r = |z| = \sqrt{a^2+b^2}$
- $\theta = \arg(z) = \text{atan2}(b, a)$ (el ángulo)

Para convertir de polar a binómica:
- $a = r\cos\theta$
- $b = r\sin\theta$

---

## 📍 PARTE 9: Multiplicación en forma polar = ROTACIÓN + ESCALA

Esto es LO MÁS LINDO de los complejos:

$$z_1 = r_1(\cos\theta_1 + i\sin\theta_1)$$
$$z_2 = r_2(\cos\theta_2 + i\sin\theta_2)$$

$$\boxed{z_1 \cdot z_2 = r_1r_2\big(\cos(\theta_1+\theta_2) + i\sin(\theta_1+\theta_2)\big)}$$

**¡Multiplicar por un complejo = escalar y rotar!**

> 🎨 Multiplicar por $i$ es rotar 90° en sentido antihorario:
> $i = 1(\cos 90° + i\sin 90°)$, así que $i \cdot z$ rota $z$ en 90°.

---

## 📍 PARTE 10: Fórmula de Euler (la joya de la corona)

$$\boxed{e^{i\theta} = \cos\theta + i\sin\theta}$$

Esto unifica exponenciales con trigonometría. Consecuencias:

$$z = re^{i\theta}$$

$$e^{i\pi} + 1 = 0 \quad \text{(la fórmula más bella de las matemáticas)}$$

Con Euler, la multiplicación es trivial:
$$r_1e^{i\theta_1} \cdot r_2e^{i\theta_2} = r_1r_2e^{i(\theta_1+\theta_2)}$$

---

## 📍 PARTE 11: Fórmula de De Moivre (potencias)

$$\boxed{(r(\cos\theta + i\sin\theta))^n = r^n(\cos(n\theta) + i\sin(n\theta))}$$

O con Euler: $(re^{i\theta})^n = r^ne^{in\theta}$

### Ejemplo clásico

Calcular $(1+i)^{10}$:

1. $r = \sqrt{1^2+1^2} = \sqrt{2}$
2. $\theta = \frac{\pi}{4}$ (45°)
3. $(1+i)^{10} = (\sqrt{2})^{10}(\cos\frac{10\pi}{4} + i\sin\frac{10\pi}{4})$
4. $= 2^5(\cos\frac{5\pi}{2} + i\sin\frac{5\pi}{2})$
5. $= 32(\cos\frac{\pi}{2} + i\sin\frac{\pi}{2}) = 32(0 + i) = \boxed{32i}$

---

## 📍 PARTE 12: Raíces de la unidad

Las soluciones de $z^n = 1$ son:

$$\boxed{z_k = \cos\frac{2\pi k}{n} + i\sin\frac{2\pi k}{n} = e^{2\pi i k / n}, \quad k = 0, 1, ..., n-1}$$

Son $n$ puntos igualmente espaciados en el círculo unitario.

**Ejemplo:** Las raíces cúbicas de 1 ($n=3$):
- $k=0$: $1$
- $k=1$: $\cos\frac{2\pi}{3} + i\sin\frac{2\pi}{3} = -\frac{1}{2} + \frac{\sqrt{3}}{2}i = \omega$
- $k=2$: $\cos\frac{4\pi}{3} + i\sin\frac{4\pi}{3} = -\frac{1}{2} - \frac{\sqrt{3}}{2}i = \omega^2$

Y se cumple: $\boxed{1 + \omega + \omega^2 = 0}$

---

## 📍 PARTE 13: Trucos de olimpiada

### Truco 1: Si $|z|=1$, entonces $\bar{z} = \frac{1}{z}$

Porque $z\bar{z} = |z|^2 = 1$, así que $\bar{z} = 1/z$.

### Truco 2: Suma de vértices de un polígono regular = 0

Si $z_1, z_2, ..., z_n$ son las raíces n-ésimas de la unidad, entonces:

$$\sum_{k=0}^{n-1} z_k = 0$$

### Truco 3: $z$ es real $\iff z = \bar{z}$

Si un número complejo es igual a su conjugado, entonces es real.

### Truco 4: $z$ es imaginario puro $\iff z = -\bar{z}$

### Truco 5: Desigualdades con módulos

- $|z_1| - |z_2| \leq |z_1 + z_2| \leq |z_1| + |z_2|$
- $|z_1| - |z_2| \leq |z_1 - z_2| \leq |z_1| + |z_2|$

### Truco 6: Rotaciones en el plano

Multiplicar por $e^{i\theta}$ rota un punto $\theta$ radianes alrededor del origen. Para rotar alrededor de otro punto $c$:

$$z' = c + (z - c)e^{i\theta}$$

### Truco 7: Condición para triángulo equilátero

Si $z_1, z_2, z_3$ son los vértices de un triángulo equilátero (orientado positivamente):

$$z_1 + \omega z_2 + \omega^2 z_3 = 0$$

Donde $\omega = e^{2\pi i/3}$.

---

## 📍 PARTE 14: Tabla resumen de fórmulas clave

| Concepto | Fórmula |
|----------|---------|
| Definición | $i^2 = -1$, $z = a + bi$ |
| Conjugado | $\bar{z} = a - bi$ |
| Módulo | $|z| = \sqrt{a^2+b^2}$ |
| Producto | $(a+bi)(c+di) = (ac-bd) + (ad+bc)i$ |
| División | $\frac{z_1}{z_2} = \frac{z_1\bar{z_2}}{|z_2|^2}$ |
| Forma polar | $z = r(\cos\theta + i\sin\theta)$ |
| Euler | $z = re^{i\theta}$ |
| De Moivre | $z^n = r^n(\cos n\theta + i\sin n\theta)$ |
| Raíces n-ésimas | $z_k = \sqrt[n]{r}\left(\cos\frac{\theta+2\pi k}{n} + i\sin\frac{\theta+2\pi k}{n}\right)$ |
| Raíces de unidad | $\zeta_k = e^{2\pi i k / n}, k=0,...,n-1$ |

---

## 🧪 EJERCICIOS

### Nivel 1: Básico
1. Calculá $(3+2i) + (4-7i)$
2. Calculá $(2+i)(3-4i)$
3. Calculá $\frac{5}{3+4i}$
4. Hallá el conjugado y el módulo de $z = -3 + 4i$
5. Resolvé $z^2 = i$

### Nivel 2: Intermedio
6. Encontrá todas las raíces cúbicas de $-8$
7. Demostrá que $|z_1 + z_2|^2 + |z_1 - z_2|^2 = 2(|z_1|^2 + |z_2|^2)$ (Ley del paralelogramo)
8. Si $|z|=1$ y $z \neq 1$, demostrá que $\frac{z-1}{z+1}$ es imaginario puro
9. Calculá $(1+\sqrt{3}i)^{2024}$

### Nivel 3: Olimpiada
10. Si $z_1, z_2, z_3$ son complejos con $|z_1|=|z_2|=|z_3|=1$ y $z_1+z_2+z_3=0$, demostrá que forman un triángulo equilátero
11. Resolvé en $\mathbb{C}$: $z^4 + z^2 + 1 = 0$
12. Demostrá que $\cos\frac{\pi}{7} + \cos\frac{3\pi}{7} + \cos\frac{5\pi}{7} = \frac{1}{2}$ (¡usando complejos!)

---

*¡Seguí con `02_Ejercicios_Interactivos.py` para practicar con código!*
