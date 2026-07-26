# 🧠 MEMORIES.md — Registro de conocimiento del proyecto

> **Propósito:** Evitar repetir errores, recordar configuraciones, y minimizar llamadas innecesarias.
> **Actualizado:** Julio 2025

---

## LECCIONES TÉCNICAS

### Sobre el formato de módulos
- El formato de `Numeros_Complejos/` funciona: lección principal → ejercicios interactivos → tests → problemas resueltos → ejercicios para el alumno → mapa mental
- Los módulos deben ser autocontenidos (cada archivo puede usarse solo)
- El orden natural de aprendizaje: teoría primero, práctica después, problemas desafiantes al final

### Sobre Python y Rust
- Python es mejor para prototipado rápido y visualizaciones interactivas
- Rust es mejor para implementaciones "de producción" que corren rápido y sin errores
- Ambos lenguajes pueden coexistir en el mismo módulo con fines didácticos

### Sobre los PDFs externos
- Los handouts de HKUST (Excalibur) son cortos (1-3 páginas) y enfocados en UN solo teorema/técnica → útiles como referencia rápida
- Los handouts de IMSC son más largos y profundos → útiles para estudio a fondo
- Los handouts de OMA están en español rioplatense → más accesibles
- Los handouts de MOP USA están en inglés y son muy avanzados

### Sobre las competencias de Francisco
- Olimpiada de Mayo 2024: 8vo puesto nacional, 9 puntos (nivel 1)
- Necesita: teoría sistemática + práctica de redacción de soluciones

---

## COSAS QUE NO FUNCIONAN / LIMITACIONES

- `union.py` tiene dependencias pesadas (pytesseract, pypdf) que requieren instalación. No usar en flujo principal.
- Los PDFs escaneados (imágenes) no se pueden extraer sin OCR.
- No todos los handouts están en español; muchos están en inglés.

---

## PLAN DE ACCIÓN (próximos pasos)

1. Crear módulo `Como_demostrar/` con técnicas de demostración
2. Crear `Plan_de_estudio.md` con ruta de aprendizaje
3. Expandir módulos a otros temas: Álgebra (desigualdades), Combinatoria (invariantes), Geometría, Teoría de Números
