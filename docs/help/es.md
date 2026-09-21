# Ayuda de ThrottleWatch

## Qué mide

ThrottleWatch observa temperatura, carga, frecuencia activa y potencia del paquete. No es una
prueba de rendimiento ni modifica los límites del equipo. Las cifras se muestran con la calidad
de la señal disponible y con la hora local de la interfaz.

- **Temperatura:** la temperatura del paquete de la CPU y el margen hasta el límite térmico
  observado.
- **Frecuencia activa:** una estimación de la frecuencia efectiva durante trabajo, distinta del
  multiplicador o del reloj anunciado por el fabricante.
- **Carga:** actividad agregada de los procesadores lógicos en la ventana seleccionada.
- **Potencia:** potencia del paquete cuando el hardware expone una lectura directa; una lectura
  derivada se etiqueta como tal.

## Cobertura y confianza

- **Nivel A:** hay señales directas y suficientes para confirmar una limitación en el alcance de
  la sesión.
- **Nivel B:** faltan señales avanzadas; la aplicación puede explicar patrones probables, pero no
  presenta una confirmación que requiera esas señales.
- **Nivel C:** faltan datos esenciales o el colector está desconectado; el resultado es
  insuficiente y no se inventan cifras.

El nivel es por sesión. Si el acceso avanzado falla a mitad de una sesión, las muestras
posteriores se degradan a B/C y las conclusiones de A no se extienden a ese tramo.

## Interpretar el resultado

Una limitación térmica confirmada combina temperatura cercana al límite con una frecuencia que
cede en la misma ventana. Una limitación de potencia muestra potencia sostenida en el techo
efectivo sin alcanzar el límite térmico. Una gestión del fabricante puede parecer una limitación
de potencia; por eso se presenta como hipótesis de plataforma cuando faltan señales directas.

«Mejorar la refrigeración» es una recomendación prudente, no una promesa de porcentaje. El
potencial cuantificado solo aparece con nivel A mantenido durante toda la ventana necesaria.

## Prueba guiada

La prueba guiada observa fases de reposo, calentamiento, carga y recuperación. No genera una
carga de CPU integrada en la versión actual. Se puede detener con `Ctrl+Shift+X`; alcanzar el
límite térmico nunca es por sí solo una orden de parada. Si se oculta o cierra la ventana, la
prueba se cancela de forma segura.

## Acceso avanzado y privacidad

PawnIO es un controlador compartido instalado por máquina y no se elimina al desinstalar
ThrottleWatch. El nivel A puede requerir una cuenta administradora durante la instalación; el
uso diario de la interfaz permanece sin privilegios. El diagnóstico B/C sigue disponible si se
rechaza UAC o el proveedor no está accesible.

Las sesiones se guardan localmente. La exportación anónima elimina los identificadores definidos
por el contrato y vuelve a validar el documento antes de escribirlo. El registro detallado es
opcional y vuelve al nivel normal al reiniciar o después de 24 horas.

## Atajos

`Ctrl+1…Ctrl+6` navega por las pantallas, `Ctrl+,` abre Ajustes, `Ctrl+E` exporta la sesión,
`F1` abre esta ayuda, `Ctrl+Shift+X` detiene la prueba guiada y `Esc` cierra diálogos o tooltips.

