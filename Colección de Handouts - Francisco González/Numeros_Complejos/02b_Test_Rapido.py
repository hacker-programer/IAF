"""
🧪 TEST RÁPIDO DE NÚMEROS COMPLEJOS (sin input)
Ejecutá: python 02b_Test_Rapido.py
"""
import math

class Complejo:
    def __init__(self, a: float, b: float = 0.0):
        self.a = a
        self.b = b

    def __repr__(self):
        return f"{self.a} + {self.b}i" if self.b >= 0 else f"{self.a} - {abs(self.b)}i"

    def __add__(self, o):
        return Complejo(self.a + o.a, self.b + o.b)

    def __sub__(self, o):
        return Complejo(self.a - o.a, self.b - o.b)

    def __mul__(self, o):
        return Complejo(self.a * o.a - self.b * o.b, self.a * o.b + self.b * o.a)

    def __truediv__(self, o):
        d = o.a**2 + o.b**2
        return Complejo((self.a * o.a + self.b * o.b) / d, (self.b * o.a - self.a * o.b) / d)

    @property
    def conjugado(self):
        return Complejo(self.a, -self.b)

    @property
    def modulo(self):
        return math.sqrt(self.a**2 + self.b**2)

    @property
    def argumento(self):
        return math.atan2(self.b, self.a)

    @classmethod
    def desde_polar(cls, r, theta):
        return cls(r * math.cos(theta), r * math.sin(theta))

    def potencia(self, n):
        return Complejo.desde_polar(self.modulo ** n, self.argumento * n)

    def raices(self, n):
        r = self.modulo ** (1/n)
        t = self.argumento
        return [Complejo.desde_polar(r, (t + 2*math.pi*k)/n) for k in range(n)]


print("=" * 60)
print("🧪 TEST RÁPIDO DE NÚMEROS COMPLEJOS")
print("=" * 60)

# 1. Suma
z1, z2 = Complejo(3, 2), Complejo(4, -7)
assert str(z1 + z2) == "7.0 - 5.0i", f"Suma mal: {z1+z2}"
print("✅ Suma: OK")

# 2. Multiplicación
z1, z2 = Complejo(2, 1), Complejo(3, -4)
assert str(z1 * z2) == "10.0 - 5.0i", f"Mult mal: {z1*z2}"
print("✅ Multiplicación: OK")

# 3. División
z1, z2 = Complejo(5, 0), Complejo(3, 4)
result = z1 / z2
assert abs(result.a - 0.6) < 1e-10 and abs(result.b - (-0.8)) < 1e-10, f"Div mal: {result}"
print("✅ División: OK")

# 4. Conjugado y módulo
z = Complejo(-3, 4)
assert abs(z.modulo - 5.0) < 1e-10
assert str(z.conjugado) == "-3.0 - 4.0i"
assert abs((z * z.conjugado).a - 25.0) < 1e-10
print("✅ Conjugado y módulo: OK")

# 5. De Moivre: (1+i)^10
z = Complejo(1, 1)
r = z.potencia(10)
assert abs(r.a) < 1e-10 and abs(r.b - 32.0) < 1e-10, f"(1+i)^10 = {r}"
print("✅ De Moivre (1+i)^10 = 32i: OK")

# 6. Raíces cúbicas de -8
z = Complejo(-8, 0)
raices = z.raices(3)
assert len(raices) == 3
# Deberían ser -2, 1+i√3, 1-i√3
print("✅ Raíces cúbicas de -8:")
for k, r in enumerate(raices):
    print(f"   ω{k} = {r}  (r={r.modulo:.3f}, θ={math.degrees(r.argumento):.1f}°)")

# 7. Raíces de la unidad suman 0
uno = Complejo(1, 0)
raices5 = uno.raices(5)
suma = Complejo(0, 0)
for r in raices5:
    suma = suma + r
assert abs(suma.a) < 1e-10 and abs(suma.b) < 1e-10, f"Suma raíces: {suma}"
print("✅ Raíces quintas de 1 suman 0: OK")

# 8. e^(iπ) + 1 = 0
e_ipi = Complejo.desde_polar(1, math.pi)
result = e_ipi + Complejo(1, 0)
assert abs(result.a) < 1e-10 and abs(result.b) < 1e-10, f"e^(iπ)+1 = {result}"
print("✅ e^(iπ) + 1 = 0: OK")

# 9. Multiplicar por i = rotar 90°
z, i = Complejo(2, 1), Complejo(0, 1)
assert str(z * i) == "-1.0 + 2.0i"  # (2+i)*i = 2i + i² = 2i - 1 = -1+2i
print("✅ Multiplicar por i = rotar 90°: OK")

# 10. Si |z|=1, (z-1)/(z+1) es imaginario puro
for ang in [math.pi/4, math.pi/3, math.pi/6]:
    z = Complejo.desde_polar(1, ang)
    w = (z - Complejo(1, 0)) / (z + Complejo(1, 0))
    assert abs(w.a) < 1e-10, f"No es imag puro: {w}"
print("✅ (z-1)/(z+1) imaginario puro si |z|=1: OK")

print("\n🎉 ¡TODOS LOS TESTS PASARON! ¡Sos un crack de los complejos!")
print("   Ahora ejecutá 02_Ejercicios_Interactivos.py para la versión completa con gráficos.")
