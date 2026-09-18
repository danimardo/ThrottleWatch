# Constitución de ThrottleWatch

## Principios fundamentales

### I. Evidencia antes que afirmación

La aplicación DEBE distinguir entre una observación, una inferencia y una confirmación directa. No DEBE equiparar temperatura elevada con pérdida de rendimiento ni tiempo en estado de throttling con porcentaje perdido. Toda conclusión visible DEBE incluir las señales que la sustentan, el intervalo analizado y un nivel de confianza. Si faltan datos, DEBE decirlo explícitamente.

**Motivo:** el valor del producto depende de evitar falsos diagnósticos y recomendaciones de refrigeración innecesarias.

### II. Hardware heterogéneo y degradación elegante

Ningún sensor concreto se considerará universal. El sistema DEBE descubrir capacidades en tiempo de ejecución, normalizar nombres y unidades y trabajar con valores ausentes. Una CPU no reconocida, un sensor sin valor o la falta del controlador de bajo nivel NO DEBEN bloquear la aplicación completa. La interfaz DEBE mostrar qué puede y qué no puede medir en el equipo actual.

**Motivo:** Intel, AMD, generaciones antiguas, portátiles OEM y placas base exponen datos diferentes.

### III. Causalidad prudente y factores de confusión

El motor DEBE considerar temperatura, carga, frecuencia activa y frecuencia base, potencia, límites de potencia/corriente y sus cambios, plan energético, alimentación y tiempo, incluida la ventana de turbo tras cada inicio de carga. Antes de recomendar mejor refrigeración, DEBE evaluar causas alternativas, incluida la gestión térmica del fabricante. Toda cifra de rendimiento DEBE proceder de una medición directa comparable (misma carga, mismo equipo, mismo contexto) o de un modelo físico explícito y versionado con sus entradas visibles, y DEBE expresarse como rango o tramo cuando la incertidumbre sea material. Una comparación de relojes con una referencia aprendida no es una estimación de rendimiento válida.

**Motivo:** una frecuencia baja puede deberse a reposo, PL1/PPT, modo silencioso, batería o firmware, no al calor.

### IV. Privilegio mínimo y aislamiento

La interfaz Tauri DEBE ejecutarse sin elevación. El acceso de bajo nivel DEBE limitarse al componente auxiliar estrictamente necesario, con protocolo local autenticado, comandos permitidos explícitamente y sin aceptar rutas o código arbitrario. La instalación o reparación del controlador DEBE requerir una acción consciente del usuario.

**Motivo:** leer sensores no justifica elevar toda la superficie de interfaz.

### V. Local primero y privacidad por defecto

El muestreo, diagnóstico, almacenamiento y exportación DEBEN funcionar sin Internet. No habrá telemetría remota por defecto. Los archivos exportados DEBEN permitir excluir identificadores de hardware. La política de retención DEBE ser visible y configurable.

**Motivo:** los datos de hardware y hábitos de uso pertenecen al usuario.

### VI. Visualización fiel, comprensible y accesible

Los gráficos DEBEN representar unidades, escalas, ausencia de datos e incertidumbre sin engaño. El color nunca será el único canal de estado. El resumen debe ser comprensible para una persona no experta y el detalle debe satisfacer a un usuario técnico mediante divulgación progresiva. Animaciones y efectos decorativos NO DEBEN dificultar la lectura ni degradar el rendimiento.

**Motivo:** una apariencia vistosa solo aporta valor si mejora la comprensión.

### VII. Reproducibilidad y pruebas con trazas

El motor de diagnóstico DEBE ser determinista para una configuración y una traza dadas. Cada regla DEBE tener pruebas unitarias; los escenarios completos DEBEN poder reproducirse con trazas sintéticas y grabadas, sin depender de ejecutar la prueba térmica en CI. Los cambios de umbrales DEBEN quedar versionados.

**Motivo:** el hardware real es difícil de reproducir y los errores de clasificación son costosos.

### VIII. Seguridad térmica y control del usuario

Cualquier carga guiada DEBE ser voluntaria, explicar qué hará, permitir cancelación inmediata y detenerse ante los límites de seguridad configurados, pérdida del sensor crítico o fallo del auxiliar. El programa NO DEBE modificar voltajes, límites de potencia, curvas de ventilador, BIOS ni frecuencias. La única escritura permitida en registros del procesador es la limpieza de bits de registro de estado (por ejemplo, los de razones de limitación), que no altera el funcionamiento; DEBE estar limitada a una lista cerrada de registros y bits en el componente privilegiado.

**Motivo:** la herramienta diagnostica; no toma control del hardware.

## Estándares de ingeniería

- Código, contratos, eventos de diagnóstico y migraciones de datos con versiones explícitas.
- Formato y análisis estático obligatorios en Rust, TypeScript y .NET.
- Dependencias fijadas mediante archivos de bloqueo y revisión de licencias.
- Binarios y actualizaciones firmados antes de distribución pública.
- Logs estructurados, rotados y sin datos sensibles innecesarios.
- Ninguna cadena de sensor del proveedor puede convertirse directamente en lógica de negocio; debe pasar por la capa de normalización.
- Ningún valor ausente se representará como cero.
- El sistema de diseño aprobado para el producto DEBE ser la fuente canónica de tokens, componentes y patrones de interacción. Una excepción o componente nuevo DEBE documentar su necesidad, reutilizar los tokens existentes y satisfacer las mismas exigencias de accesibilidad, localización, temas y adaptación de tamaño; no se permiten duplicados locales casi equivalentes.

## Puertas de calidad

Una característica no se considera terminada si incumple cualquiera de estas puertas:

1. Requisitos y escenarios de aceptación enlazados a pruebas.
2. Contratos compatibles o migración documentada.
3. Pruebas unitarias y de integración relevantes superadas.
4. Verificación con al menos una traza Intel, una AMD y una degradada/sin sensores; para el motor de diagnóstico, además, el corpus etiquetado con razones directas y sus copias degradadas.
5. Accesibilidad básica: teclado, foco visible, contraste y modo de movimiento reducido.
6. Consumo en reposo y crecimiento de almacenamiento dentro de los presupuestos definidos.
7. Mensajes de diagnóstico revisados para no presentar inferencias como hechos.
8. Avisos y obligaciones de licencias de terceros incorporados al paquete.
9. Las pantallas afectadas se han verificado en ambos temas, ambos idiomas y los tamaños compacto, medio y expandido definidos por el producto, incluidos sus estados de carga, vacío, degradado y error aplicables.

## Gobernanza

Esta constitución prevalece sobre decisiones locales de implementación. Toda excepción DEBE documentarse en `plan.md` con motivo, alternativa descartada, riesgo y fecha de retirada. Las enmiendas requieren actualizar la versión, registrar el cambio y comprobar los documentos de especificación afectados.

### Historial de enmiendas

- **1.1.0 (2026-09-17):** se incorpora el sistema de diseño aprobado como fuente canónica y se añade la matriz obligatoria de verificación visual y funcional.
- **1.2.0 (2026-09-18):** revisión del motor de diagnóstico: el principio III exige que las cifras de rendimiento procedan de una medición comparable o de un modelo físico explícito y considera la ventana de turbo y la gestión térmica del fabricante; el principio VIII acota la única escritura permitida en registros (limpieza de bits de estado); la puerta 4 incluye el corpus etiquetado. Documentos afectados: `spec.md`, `plan.md`, `research.md`, `data-model.md`, contratos, `ux-visual-spec.md`, `tasks.md`.

**Versión**: 1.2.0  
**Ratificada**: 2026-09-17  
**Última modificación**: 2026-09-18
