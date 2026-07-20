"""
🔬 LABORATORIO INTERACTIVO DE NÚMEROS COMPLEJOS
===============================================
Ejecutá este archivo y experimentá con números complejos.
Python tiene soporte nativo: usá 1j en vez de i.

Ejemplo: z = 3 + 4j
"""

import cmath
import math

# ============================================================
# CLASE DIDÁCTICA: Implementamos un número complejo "a mano"
# para entender cada operación. (Python ya tiene complex, pero
# esta implementación te muestra qué pasa por dentro.)
# ============================================================

class Complejo:
    """Un número complejo a + bi, implementado con fines didácticos."""

    def __init__(self, a: float, b: float = 0.0):
        self.a = a  # parte real
        self.b = b  # parte imaginaria

    # --- Representación bonita ---
    def __repr__(self):
        if self.b >= 0:
            return f"{self.a} + {self.b}i"
        else:
            return f"{self.a} - {abs(self.b)}i"

    # --- SUMA ---
    def __add__(self, other):
        return Complejo(self.a + other.a, self.b + other.b)

    # --- RESTA ---
    def __sub__(self, other):
        return Complejo(self.a - other.a, self.b - other.b)

    # --- MULTIPLICACIÓN: (a+bi)(c+di) = (ac-bd)+(ad+bc)i ---
    def __mul__(self, other):
        real = self.a * other.a - self.b * other.b
        imag = self.a * other.b + self.b * other.a
        return Complejo(real, imag)

    # --- DIVISIÓN: multiplicar por conjugado del denominador ---
    def __truediv__(self, other):
        denom = other.a**2 + other.b**2
        if denom == 0:
            raise ZeroDivisionError("División por cero complejo")
        real = (self.a * other.a + self.b * other.b) / denom
        imag = (self.b * other.a - self.a * other.b) / denom
        return Complejo(real, imag)

    # --- CONJUGADO ---
    @property
    def conjugado(self):
        return Complejo(self.a, -self.b)

    # --- MÓDULO ---
    @property
    def modulo(self):
        return math.sqrt(self.a**2 + self.b**2)

    # --- ARGUMENTO (en radianes) ---
    @property
    def argumento(self):
        return math.atan2(self.b, self.a)

    # --- FORMA POLAR: devuelve (r, theta) ---
    def polar(self):
        return (self.modulo, self.argumento)

    # --- CREAR DESDE POLAR ---
    @classmethod
    def desde_polar(cls, r: float, theta: float):
        return cls(r * math.cos(theta), r * math.sin(theta))

    # --- POTENCIA (De Moivre) ---
    def potencia(self, n: int):
        """Eleva a la n usando De Moivre."""
        r_n = self.modulo ** n
        theta_n = self.argumento * n
        return Complejo.desde_polar(r_n, theta_n)

    # --- RAÍCES N-ÉSIMAS ---
    def raices(self, n: int):
        """Devuelve las n raíces n-ésimas."""
        r = self.modulo ** (1/n)
        theta = self.argumento
        raices = []
        for k in range(n):
            theta_k = (theta + 2 * math.pi * k) / n
            raices.append(Complejo.desde_polar(r, theta_k))
        return raices


# ============================================================
# EJERCICIOS GUIADOS
# ============================================================

def ejercicios():
    print("=" * 60)
    print("🧪 EJERCICIOS INTERACTIVOS DE NÚMEROS COMPLEJOS")
    print("=" * 60)

    # --- Ejercicio 1: Suma y resta ---
    print("\n📝 EJERCICIO 1: Suma y resta")
    z1 = Complejo(3, 2)
    z2 = Complejo(4, -7)
    print(f"  z1 = {z1}")
    print(f"  z2 = {z2}")
    print(f"  z1 + z2 = {z1 + z2}")
    print(f"  z1 - z2 = {z1 - z2}")

    # --- Ejercicio 2: Multiplicación ---
    print("\n📝 EJERCICIO 2: Multiplicación")
    z1 = Complejo(2, 1)
    z2 = Complejo(3, -4)
    print(f"  z1 = {z1}")
    print(f"  z2 = {z2}")
    print(f"  z1 * z2 = {z1 * z2}")
    # Verificar: (2+i)(3-4i) = 6 - 8i + 3i -4i² = 6 -5i +4 = 10 -5i ✓

    # --- Ejercicio 3: División ---
    print("\n📝 EJERCICIO 3: División")
    z1 = Complejo(5, 0)
    z2 = Complejo(3, 4)
    print(f"  z1 = {z1}")
    print(f"  z2 = {z2}")
    print(f"  z1 / z2 = {z1 / z2}")
    # 5/(3+4i) = 5(3-4i)/25 = (15-20i)/25 = 0.6 - 0.8i

    # --- Ejercicio 4: Conjugado y módulo ---
    print("\n📝 EJERCICIO 4: Conjugado y módulo")
    z = Complejo(-3, 4)
    print(f"  z = {z}")
    print(f"  conjugado = {z.conjugado}")
    print(f"  |z| = {z.modulo}")  # √(9+16) = 5
    print(f"  z * conjugado = {z * z.conjugado}")  # = |z|² = 25

    # --- Ejercicio 5: Potencias con De Moivre ---
    print("\n📝 EJERCICIO 5: (1+i)^10 usando De Moivre")
    z = Complejo(1, 1)
    print(f"  z = {z}")
    print(f"  |z| = {z.modulo} = √2")
    print(f"  arg(z) = {z.argumento} rad = {math.degrees(z.argumento):.1f}°")
    resultado = z.potencia(10)
    print(f"  z^10 = {resultado}")
    print(f"  ¡Debe dar 32i! ✓" if abs(resultado.a) < 1e-10 and abs(resultado.b - 32) < 1e-10 else "  Revisar...")

    # --- Ejercicio 6: Raíces cúbicas de -8 ---
    print("\n📝 EJERCICIO 6: Raíces cúbicas de -8")
    z = Complejo(-8, 0)
    raices = z.raices(3)
    print(f"  Las 3 raíces cúbicas de -8 son:")
    for k, r in enumerate(raices):
        print(f"    ω{k} = {r}  (en polar: r={r.modulo:.3f}, θ={math.degrees(r.argumento):.1f}°)")

    # --- Ejercicio 7: Raíces de la unidad ---
    print("\n📝 EJERCICIO 7: Raíces quintas de la unidad")
    uno = Complejo(1, 0)
    raices_unidad = uno.raices(5)
    print(f"  Las 5 raíces quintas de 1:")
    suma = Complejo(0, 0)
    for k, r in enumerate(raices_unidad):
        print(f"    ζ{k} = {r}")
        suma = suma + r
    print(f"  Suma de todas las raíces = {suma}  (¡debe ser 0!)")

    # --- Ejercicio 8: Magia de Euler ---
    print("\n📝 EJERCICIO 8: e^(iπ) + 1 = 0")
    z = Complejo.desde_polar(1, math.pi)
    print(f"  e^(iπ) = cos(π) + i·sin(π) = {z}")
    resultado = z + Complejo(1, 0)
    print(f"  e^(iπ) + 1 = {resultado}")
    print(f"  ¡La fórmula más bella! ✓")

    # --- Ejercicio 9: Multiplicar por i = rotar 90° ---
    print("\n📝 EJERCICIO 9: Multiplicar por i rota 90°")
    z = Complejo(2, 1)
    i = Complejo(0, 1)
    print(f"  z = {z}")
    print(f"  i*z = {i * z}  (rotado 90° antihorario)")
    print(f"  i²*z = {i * i * z}  (rotado 180°)")
    print(f"  i³*z = {i * i * i * z}  (rotado 270°)")
    print(f"  i⁴*z = {i * i * i * i * z}  (vuelta completa)")

    # --- Ejercicio 10: Demostración con |z|=1 ---
    print("\n📝 EJERCICIO 10: Si |z|=1, entonces (z-1)/(z+1) es imaginario puro")
    # Tomamos z = e^(iθ)
    for angulo in [math.pi/4, math.pi/3, math.pi/6]:
        z = Complejo.desde_polar(1, angulo)
        w = (z - Complejo(1, 0)) / (z + Complejo(1, 0))
        print(f"  θ={math.degrees(angulo):.0f}°: z={z}, (z-1)/(z+1) = {w}")
        print(f"      Parte real ≈ {w.a:.10f} (debe ser ~0, imaginario puro ✓)")


# ============================================================
# VISUALIZACIÓN (si matplotlib está instalado)
# ============================================================

def visualizar():
    """Grafica números complejos en el plano de Argand."""
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print("\n⚠️  matplotlib no está instalado. Instalalo con: pip install matplotlib")
        return

    fig, axes = plt.subplots(1, 3, figsize=(16, 5))

    # --- Gráfico 1: Raíces quintas de la unidad ---
    ax = axes[0]
    uno = Complejo(1, 0)
    raices = uno.raices(5)
    xs = [r.a for r in raices]
    ys = [r.b for r in raices]

    # Círculo unitario
    circulo = plt.Circle((0, 0), 1, fill=False, color='lightblue', linestyle='--')
    ax.add_patch(circulo)
    ax.scatter(xs, ys, c='red', s=100, zorder=5)
    for k, (x, y) in enumerate(zip(xs, ys)):
        ax.annotate(f'ζ{k}', (x, y), textcoords="offset points", xytext=(10, 10), fontsize=12)
    ax.axhline(y=0, color='gray', alpha=0.5)
    ax.axvline(x=0, color='gray', alpha=0.5)
    ax.set_aspect('equal')
    ax.set_xlim(-1.5, 1.5)
    ax.set_ylim(-1.5, 1.5)
    ax.set_title("Raíces quintas de la unidad")
    ax.set_xlabel("Re")
    ax.set_ylabel("Im")

    # --- Gráfico 2: Rotación multiplicando por i ---
    ax = axes[1]
    z = Complejo(2, 1)
    puntos = [z]
    i = Complejo(0, 1)
    for _ in range(3):
        puntos.append(puntos[-1] * i)

    for k, p in enumerate(puntos):
        ax.arrow(0, 0, p.a, p.b, head_width=0.15, head_length=0.15,
                 fc=f'C{k}', ec=f'C{k}', alpha=0.7, width=0.05)
        ax.annotate(f'i^{k}·z', (p.a, p.b), textcoords="offset points",
                    xytext=(10, 5), color=f'C{k}', fontsize=11)
    ax.axhline(y=0, color='gray', alpha=0.5)
    ax.axvline(x=0, color='gray', alpha=0.5)
    ax.set_aspect('equal')
    ax.set_xlim(-3, 3)
    ax.set_ylim(-3, 3)
    ax.set_title("Multiplicar por i = rotar 90°")
    ax.set_xlabel("Re")
    ax.set_ylabel("Im")

    # --- Gráfico 3: Suma de complejos (regla del paralelogramo) ---
    ax = axes[2]
    z1 = Complejo(2, 1)
    z2 = Complejo(1, 3)
    suma = z1 + z2

    # Flechas desde el origen
    ax.arrow(0, 0, z1.a, z1.b, head_width=0.1, head_length=0.1, fc='blue', ec='blue', alpha=0.6, width=0.03)
    ax.arrow(0, 0, z2.a, z2.b, head_width=0.1, head_length=0.1, fc='red', ec='red', alpha=0.6, width=0.03)
    ax.arrow(0, 0, suma.a, suma.b, head_width=0.1, head_length=0.1, fc='purple', ec='purple', alpha=0.8, width=0.05)

    # Paralelogramo
    ax.plot([z1.a, suma.a], [z1.b, suma.b], 'green', linestyle='--', alpha=0.5)
    ax.plot([z2.a, suma.a], [z2.b, suma.b], 'green', linestyle='--', alpha=0.5)

    ax.annotate('z1', (z1.a, z1.b), textcoords="offset points", xytext=(10, 5), color='blue')
    ax.annotate('z2', (z2.a, z2.b), textcoords="offset points", xytext=(10, 5), color='red')
    ax.annotate('z1+z2', (suma.a, suma.b), textcoords="offset points", xytext=(10, 5), color='purple')
    ax.axhline(y=0, color='gray', alpha=0.5)
    ax.axvline(x=0, color='gray', alpha=0.5)
    ax.set_aspect('equal')
    ax.set_xlim(-0.5, 4)
    ax.set_ylim(-0.5, 4.5)
    ax.set_title("Suma = regla del paralelogramo")
    ax.set_xlabel("Re")
    ax.set_ylabel("Im")

    plt.tight_layout()
    plt.savefig('Numeros_Complejos/plano_complejo.png', dpi=150, bbox_inches='tight')
    print("\n📊 ¡Gráfico guardado como 'Numeros_Complejos/plano_complejo.png'!")
    plt.show()


# ============================================================
# PROGRAMA PRINCIPAL
# ============================================================

if __name__ == "__main__":
    ejercicios()
    print("\n" + "=" * 60)
    resp = input("\n¿Querés ver las visualizaciones? (s/n): ")
    if resp.lower().startswith('s'):
        visualizar()
    print("\n✅ ¡Listo! Ahora explorá el código y modificá los valores.")
    print("   Probá crear tus propios complejos y experimentar.")
