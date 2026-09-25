# Kit de sondeo de hardware

Paquete portable para recoger, en una máquina que **no** es la de desarrollo, lo que hace falta
para completar la matriz de hardware (T019, T109) y para decidir T028b2 en AMD
(`docs/spikes/amd-pm-table.md`).

## Qué hace y qué no hace

**Hace:** lee identificación de la CPU, versión de Windows, plan de energía, estado de PawnIO,
ejecuta el sondeo de acceso de bajo nivel, recoge 10 s de contadores por procesador lógico y,
en AMD, vuelca **cruda** la tabla PM del SMU.

**No hace:** no instala nada, no toca el registro, no cambia configuración, no envía nada por red.
Escribe solo en su propia carpeta.

**No recoge** números de serie, nombres de usuario, nombre del equipo ni rutas del sistema.

## Preparar el paquete (en la máquina de desarrollo)

```powershell
cd docs\spikes\tools\hardware-probe
.\build-kit.ps1
```

Deja `throttlewatch-probe-kit.zip` (~70 MB, lleva el runtime de .NET dentro, así que la máquina
destino no necesita instalar nada).

## Ejecutarlo (en la máquina que se quiere sondear)

1. Copiar y descomprimir el zip.
2. **Recomendado:** instalar PawnIO antes, si se va a medir nivel A. Sin PawnIO el informe sigue
   valiendo, pero solo como fila de nivel B/C.
3. Abrir PowerShell **como administrador** (sin elevar no hay nivel A ni tabla PM) y ejecutar:

```powershell
powershell -ExecutionPolicy Bypass -File .\run-probe.ps1
```

Para ver el comportamiento bajo carga, lanzar en paralelo una carga multihilo y usar
`-Seconds 60`.

4. Devolver el `.zip` que deja el guion (`informe-<cpu>-<fecha>.zip`).

## Qué mirar en el resultado

`RESUMEN.txt` lo dice de un vistazo. Lo importante:

| Campo | Qué significa |
|---|---|
| `estado` del probe | `available` = nivel A alcanzable; `denied`/`missing` = solo B/C |
| `resultado` de la tabla PM | `PM_TABLE_DUMPED` = volcado bueno; `PM_TABLE_UNAVAILABLE` = sin acceso (falta elevar o PawnIO); `PM_TABLE_SIZE_IMPLAUSIBLE` = la CPU no expone una tabla usable; `NOT_AMD` = normal en Intel |
| `version tabla` | La clave para T028b2: decide si existe mapa de campos para esa familia |

## Advertencia sobre el volcado de la tabla PM

El volcado **no interpreta ni un solo campo**: guarda las palabras crudas tal cual. Es
deliberado — el mapa de qué posición es THM, PPT, TDC o EDC es justo lo que T028b2 no tiene
todavía. Con el volcado guardado, ese mapa se puede contrastar más tarde y sin volver a pisar la
máquina.

Su camino de error está probado (2026-09-25, Ryzen 5 2600X sin elevar: devuelve
`PM_TABLE_UNAVAILABLE` sin romperse). **El camino bueno, con una tabla legible de verdad, no se ha
podido probar todavía**: ninguna máquina disponible tiene una familia cubierta.

## Máquinas que interesan

| Objetivo | CPU |
|---|---|
| T028b2 sin problema de licencia | Cualquier **APU** Ryzen: 4000U/G, 5000U/G, 6000, 7040, 8000G |
| T028b2 vía `ryzen_monitor` (AGPL, exige decisión) | **Zen3**: 5800X, 5900X, 5950X · **Zen2**: 3600, 3700X, 3800X, 3900X, 3950X |
| Fila «Intel anterior» + «sobremesa con límites abiertos» | Intel serie K de 8.ª–11.ª generación |
| **No sirven** | Ryzen 7000/9000, EPYC, Threadripper (sin mapa de campos público) |
