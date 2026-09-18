# Mockup integral de ThrottleWatch

Prototipo navegable de alta fidelidad que reúne las pantallas del sistema de diseño dentro de una única ventana simulada. Sirve para revisar cómo se sentirá la aplicación terminada antes de conectar Tauri, Rust, sensores o persistencia.

## Ejecutar

```powershell
Set-Location F:\Apps\ThrottleWatch\design\mockup
npm install
npm run dev
```

Para verificar una entrega:

```powershell
npm run check
npm run build
npm run preview
```

## Alcance

- Navegación real entre Ahora, Análisis, CPU, Sesiones, Diagnóstico guiado y Ajustes.
- Onboarding inicial simulado.
- Shell de escritorio con barra de título propia y diseño adaptable.
- Tema claro/oscuro y vista compacta mediante el tamaño de la ventana.
- Datos ficticios coherentes para mostrar todos los destinos.
- Reutilización directa de `../components`, `../examples`, `../icons`, `../illustrations` y `../tokens`.
- Piezas de la revisión de 2026-09-18: franja de contexto y panel de cobertura en `Ahora`, tarjetas de novedades, diálogo de exportación (`Ctrl+E`), pregunta de primera X al pulsar cerrar, barras globales de «prueba en curso» y «colector caído», barra inferior con «Más» en ancho compacto y atajos `Ctrl+1…6`, `Ctrl+,`, `Ctrl+Shift+X`. La barra «Mockup navegable» tiene chips para simular esos estados.

No implementa sensores, IPC, SQLite, bandeja, notificaciones, actualizaciones ni controles reales de ventana. Los botones que dependen de Tauri muestran comportamiento simulado o no producen efectos externos.
