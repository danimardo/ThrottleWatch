# Corpus etiquetado v1

Este directorio contiene etiquetas de verdad de referencia para el motor. Las
etiquetas describen razones observadas y no solo el resultado esperado. Cada
caso mantiene copias degradadas `level-b` y `level-c`; una copia degradada
elimina señales, nunca cambia muestras existentes ni convierte ausencias en
cero.

Casos mínimos cubiertos por T045:

- Intel híbrido con `THERMAL` confirmado.
- Intel anterior con fin de turbo sin limitación térmica.
- Portátil con DTT/DPTF y límite de potencia progresivo.
- Sobremesa con límites abiertos.
- AMD Zen 4 con equivalencia térmica permitida.
- Juego de pocos núcleos.
- Equipo que empieza caliente.
- EcoQoS/EPP con frecuencia reducida sin razón directa.
