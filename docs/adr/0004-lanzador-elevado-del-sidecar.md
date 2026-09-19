# ADR-0004: Lanzador elevado bajo demanda del sidecar

- Estado: aceptado (2026-09-19)
- Fecha: 2026-09-19
- Alcance: FR-087–FR-090, T019a, T151–T158
- Enmienda: ADR-0001, §4

## Contexto

El spike de acceso a sensores demostró que PawnIO 2.2.0 puede estar disponible para el proceso
elevado, pero todavía no demostró nivel A pleno ni acceso como usuario estándar tras reiniciar.
El sidecar necesita conservar un único `LibreHardwareMonitorLib.Computer` abierto durante su
vida para no cerrar el estado global que utilizan `RyzenSmu` e `IntelMsr`.

La interfaz de ThrottleWatch debe seguir siendo un proceso sin privilegios y no debe pedir UAC en
cada arranque. El acceso avanzado también debe poder desactivarse sin desinstalar PawnIO ni
interrumpir el diagnóstico degradado B/C.

## Decisión

Se utilizará una tarea programada de Windows bajo demanda, registrada durante la misma acción UAC
explícita que instala o repara PawnIO. No se añadirá un servicio privilegiado persistente.

La tarea:

1. Ejecutará únicamente el lanzador firmado de ThrottleWatch desde una ruta fija instalada.
2. Usará el nivel más alto para el usuario que aceptó la instalación y permanecerá detenida cuando
   no haya una sesión de diagnóstico.
3. Lanzará exclusivamente el sidecar cuya ruta y hash SHA-256 estén fijados por la instalación.
4. No descargará, sustituirá ni actualizará PawnIO en tiempo de ejecución.
5. Se invocará sin UAC en sesiones posteriores; instalar, actualizar o reparar PawnIO seguirá
   siendo una acción explícita que puede requerir una nueva elevación.

La comunicación entre la interfaz sin privilegios y el lanzador se hará por una tubería con nombre
creada con una ACL mínima para el usuario de la sesión y el sistema. La interfaz generará un nonce
criptográficamente aleatorio por sesión; el lanzador rechazará la conexión si el nonce, la versión
de protocolo, la secuencia o el tamaño no son válidos. El nonce no se almacenará en disco ni se
pasará mediante variables de entorno.

El lanzador aplicará esta secuencia antes de aceptar muestras:

1. abrir la tubería segura y validar el nonce de la sesión;
2. verificar la ruta fija y el hash del sidecar;
3. iniciar el sidecar con el contrato NDJSON autenticado;
4. mantener un latido y una regla de vida: desconexión de la interfaz, EOF o ausencia de latido
   durante el intervalo fijado detiene el sidecar y termina el lanzador;
5. cerrar la tubería y limpiar el estado efímero al finalizar.

La acción «Desactivar acceso avanzado» impedirá nuevas activaciones de la tarea y hará que la
interfaz vuelva a modo B/C. No detendrá ni desinstalará PawnIO, que es un controlador compartido.

## Revisión de amenazas

| Amenaza | Salvaguarda | Residuo aceptado |
|---|---|---|
| WebView o proceso local intenta obtener privilegios | La WebView no crea procesos; la tarea solo ejecuta el lanzador de ruta fija | Un proceso administrativo del mismo usuario podría actuar como él |
| Sustitución del sidecar | Ruta fija, ACL de instalación, firma de código y hash verificado antes del arranque | La confianza de la cadena de firma del instalador queda fuera del runtime |
| Cliente no autorizado en la tubería | ACL mínima, nonce por sesión, secuencia creciente y límite de tamaño | Un administrador local puede inspeccionar o interferir con el sistema |
| Sidecar colgado o interfaz terminada | Latido, EOF, timeout y cierre del proceso hijo | Una terminación abrupta puede dejar un proceso hasta el siguiente watchdog |
| Elevación persistente innecesaria | Tarea detenida y activación bajo demanda; no hay listener privilegiado permanente | La tarea registrada existe hasta que se desactiva o desinstala la aplicación |
| Instalador manipulado | Verificación Authenticode y hash del instalador antes de ejecutarlo | La validación del paquete publicado corresponde al pipeline de release |
| PawnIO eliminado por error | El desinstalador no lo elimina y solo muestra instrucciones | La gestión manual del controlador queda bajo responsabilidad del usuario |

## Compatibilidad y degradación

- Si PawnIO no está instalado, está por debajo de la versión mínima o no puede iniciarse, no se
  solicita UAC automáticamente: se publica `missing`, `upgradable`, `denied` o `error` y se
  continúa en B/C.
- Si el acceso avanzado falla durante una sesión, se conserva el muestreo estándar, se registra
  el cambio de cobertura y se detienen las afirmaciones que requieren nivel A.
- La ausencia de `log_clear_supported` impide declarar nivel A pleno aunque las lecturas sean
  accesibles.

## Consecuencias

- Enmienda la prohibición de ADR-0001 contra un servicio privilegiado solo en el sentido de
  permitir un lanzador bajo demanda aprobado por esta revisión; no eleva la UI.
- T153–T155 quedan habilitadas tras la aprobación de este ADR y deben conservar sus controles,
  firma, ACL y pruebas Windows.
- T152 sigue siendo una validación de hardware real independiente: debe comprobar el arranque como
  usuario estándar después del reinicio.

## Rechazo explícito de alternativas

- Elevar la UI en cada inicio: rompe FR-030 y crea UAC recurrente.
- Servicio privilegiado permanente: amplía la superficie de ataque sin ser necesario para el flujo
  bajo demanda.
- Canal IPC sin nonce o con ACL amplia: no permite atribuir la sesión ni limitar clientes.

## Evidencia y fuentes

- `docs/spikes/sensor-access.md`, entrada del 2026-09-19.
- PawnIO 2.2.0, release oficial: https://github.com/namazso/PawnIO.Setup/releases/tag/2.2.0
- Licencia del código fuente de PawnIO: https://github.com/namazso/PawnIO/blob/master/README.md
