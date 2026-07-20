//! 🔬 NÚMEROS COMPLEJOS EN RUST
//! =============================
//! Una implementación didáctica de números complejos en Rust.
//! Ejecutá con: cargo run

use std::f64::consts::PI;
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// Un número complejo a + bi.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex {
    a: f64, // parte real
    b: f64, // parte imaginaria
}

impl Complex {
    /// Crea un nuevo número complejo.
    fn new(a: f64, b: f64) -> Self {
        Complex { a, b }
    }

    /// Unidad imaginaria i.
    fn i() -> Self {
        Complex { a: 0.0, b: 1.0 }
    }

    /// Conjugado: a - bi.
    fn conjugate(&self) -> Self {
        Complex {
            a: self.a,
            b: -self.b,
        }
    }

    /// Módulo: sqrt(a² + b²).
    fn modulus(&self) -> f64 {
        (self.a.powi(2) + self.b.powi(2)).sqrt()
    }

    /// Argumento (ángulo en radianes).
    fn argument(&self) -> f64 {
        self.b.atan2(self.a)
    }

    /// Devuelve (módulo, argumento).
    fn to_polar(&self) -> (f64, f64) {
        (self.modulus(), self.argument())
    }

    /// Crea desde coordenadas polares.
    fn from_polar(r: f64, theta: f64) -> Self {
        Complex {
            a: r * theta.cos(),
            b: r * theta.sin(),
        }
    }

    /// Potencia n-ésima usando De Moivre.
    fn pow(&self, n: i32) -> Self {
        let r_n = self.modulus().powi(n);
        let theta_n = self.argument() * n as f64;
        Complex::from_polar(r_n, theta_n)
    }

    /// Raíces n-ésimas.
    fn roots(&self, n: u32) -> Vec<Self> {
        let r = self.modulus().powf(1.0 / n as f64);
        let theta = self.argument();
        (0..n)
            .map(|k| {
                let theta_k = (theta + 2.0 * PI * k as f64) / n as f64;
                Complex::from_polar(r, theta_k)
            })
            .collect()
    }

    /// ¿Es real? (parte imaginaria aproximadamente 0)
    fn is_real(&self) -> bool {
        self.b.abs() < 1e-10
    }

    /// ¿Es imaginario puro? (parte real aproximadamente 0)
    fn is_pure_imaginary(&self) -> bool {
        self.a.abs() < 1e-10
    }
}

// --- Display bonito ---
impl fmt::Display for Complex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.b >= 0.0 {
            write!(f, "{} + {}i", self.a, self.b)
        } else {
            write!(f, "{} - {}i", self.a, -self.b)
        }
    }
}

// --- Operadores ---
impl Add for Complex {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Complex::new(self.a + other.a, self.b + other.b)
    }
}

impl Sub for Complex {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Complex::new(self.a - other.a, self.b - other.b)
    }
}

impl Mul for Complex {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Complex::new(
            self.a * other.a - self.b * other.b, // ac - bd
            self.a * other.b + self.b * other.a, // ad + bc
        )
    }
}

impl Div for Complex {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        let denom = other.a.powi(2) + other.b.powi(2);
        Complex::new(
            (self.a * other.a + self.b * other.b) / denom,
            (self.b * other.a - self.a * other.b) / denom,
        )
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════╗");
    println!("║   🔬 LABORATORIO DE COMPLEJOS EN RUST       ║");
    println!("╚══════════════════════════════════════════════╝");

    // --- Básicos ---
    let z1 = Complex::new(3.0, 2.0);
    let z2 = Complex::new(4.0, -7.0);
    println!("\n📝 Suma y resta:");
    println!("  z1 = {}", z1);
    println!("  z2 = {}", z2);
    println!("  z1 + z2 = {}", z1 + z2);
    println!("  z1 - z2 = {}", z1 - z2);

    // --- Multiplicación ---
    let z1 = Complex::new(2.0, 1.0);
    let z2 = Complex::new(3.0, -4.0);
    println!("\n📝 Multiplicación:");
    println!("  z1 = {}", z1);
    println!("  z2 = {}", z2);
    println!("  z1 * z2 = {}", z1 * z2);

    // --- División ---
    let z1 = Complex::new(5.0, 0.0);
    let z2 = Complex::new(3.0, 4.0);
    println!("\n📝 División:");
    println!("  z1 = {}", z1);
    println!("  z2 = {}", z2);
    println!("  z1 / z2 = {}", z1 / z2);

    // --- Conjugado y módulo ---
    let z = Complex::new(-3.0, 4.0);
    println!("\n📝 Conjugado y módulo:");
    println!("  z = {}", z);
    println!("  conjugado = {}", z.conjugate());
    println!("  |z| = {:.4}", z.modulus());
    println!("  z * conjugado = {} = |z|² = {}", z * z.conjugate(), z.modulus().powi(2));

    // --- De Moivre: (1+i)^10 ---
    let z = Complex::new(1.0, 1.0);
    println!("\n📝 De Moivre: (1+i)^10");
    println!("  z = {}", z);
    println!("  |z| = {:.4} = √2", z.modulus());
    println!("  arg(z) = {:.4} rad = {:.1}°", z.argument(), z.argument().to_degrees());
    let resultado = z.pow(10);
    println!("  z^10 = {}", resultado);
    println!("  ¡Debe ser 32i! ✓");

    // --- Raíces cúbicas de -8 ---
    let z = Complex::new(-8.0, 0.0);
    println!("\n📝 Raíces cúbicas de -8:");
    let raices = z.roots(3);
    for (k, r) in raices.iter().enumerate() {
        println!("  ω{} = {}  (|ω|={:.3}, arg={:.1}°)",
            k, r, r.modulus(), r.argument().to_degrees());
    }

    // --- Raíces quintas de la unidad ---
    let uno = Complex::new(1.0, 0.0);
    println!("\n📝 Raíces quintas de la unidad:");
    let raices_5 = uno.roots(5);
    let mut suma = Complex::new(0.0, 0.0);
    for (k, r) in raices_5.iter().enumerate() {
        println!("  ζ{} = {}", k, r);
        suma = suma + *r;
    }
    println!("  Suma = {}  (¡debe ser ~0!) ✓", suma);

    // --- Fórmula de Euler ---
    println!("\n📝 e^(iπ) + 1 = 0:");
    let e_ipi = Complex::from_polar(1.0, PI);
    println!("  e^(iπ) = {}", e_ipi);
    println!("  e^(iπ) + 1 = {}", e_ipi + Complex::new(1.0, 0.0));

    // --- Rotación por i ---
    println!("\n📝 Multiplicar por i = rotar 90°:");
    let z = Complex::new(2.0, 1.0);
    let i = Complex::i();
    println!("  z = {}", z);
    println!("  i*z   = {}  (90°)", z * i);
    println!("  i²*z  = {}  (180°)", z * i * i);
    println!("  i³*z  = {}  (270°)", z * i * i * i);
    println!("  i⁴*z  = {}  (360° = vuelta completa)", z * i * i * i * i);

    // --- Truco: |z|=1 => conjugado = 1/z ---
    println!("\n📝 Truco olimpiada: |z|=1 => conjugado = 1/z:");
    let z_unit = Complex::from_polar(1.0, PI / 4.0); // e^(iπ/4)
    println!("  z = {}  (|z|={})", z_unit, z_unit.modulus());
    println!("  conjugado = {}", z_unit.conjugate());
    println!("  1/z = {}", Complex::new(1.0,0.0) / z_unit);
    println!("  ¡Son iguales! ✓");

    // --- Triángulo equilátero ---
    println!("\n📝 Condición triángulo equilátero:");
    let omega = Complex::from_polar(1.0, 2.0 * PI / 3.0); // ω = e^(2πi/3)
    println!("  ω = e^(2πi/3) = {}", omega);
    // Tres puntos en un triángulo equilátero centrado en el origen
    let z1 = Complex::new(1.0, 0.0);
    let z2 = Complex::from_polar(1.0, 2.0 * PI / 3.0);
    let z3 = Complex::from_polar(1.0, 4.0 * PI / 3.0);
    let cond = z1 + omega * z2 + omega * omega * z3;
    println!("  z1={}, z2={}, z3={}", z1, z2, z3);
    println!("  z1 + ω·z2 + ω²·z3 = {}  (¡debe ser ~0!) ✓", cond);

    println!("\n✅ ¡Listo! Modificá el código y experimentá.");
    println!("   Probá: Complex::new(tu_valor, tu_valor).roots(lo_que_quieras)");
}
