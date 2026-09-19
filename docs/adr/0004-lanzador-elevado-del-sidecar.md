# ADR-0004: Lanzador elevado bajo demanda del sidecar

- Estado: aceptado con condiciones (2026-09-19)
- Fecha: 2026-09-19
- Alcance: FR-087–FR-090, T019a, T151–T158
- Enmienda: ADR-0001, §4
- Revisores: propietario; revisión independiente 2026-09-19

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

El acceso de nivel A requiere una cuenta administradora de Windows para la instalación o reparación
de PawnIO y el registro de la tarea. Una cuenta estándar puede utilizar posteriormente el modo
estándar y el modo B/C, pero no se promete nivel A sin una instalación ya autorizada por un
administrador (R3).

La tarea:

1. Ejecutará únicamente el lanzador firmado de ThrottleWatch desde una ruta fija instalada.
2. Usará el nivel más alto para el usuario que aceptó la instalación y permanecerá detenida cuando
   no haya una sesión de diagnóstico.
3. Lanzará exclusivamente el sidecar de la ruta fija instalada cuya identidad esté cubierta por
   un manifiesto de release firmado con la clave minisign del actualizador y, cuando exista
   certificado, por una firma Authenticode válida del editor permitido. En desarrollo se exige un
   manifiesto firmado con la clave de desarrollo; nunca se desactiva la verificación. Un hash fijo
   por máquina no es el control de identidad principal: la actualización por usuario puede
   sustituir legítimamente el binario. El lanzador rechazará cualquier binario sin esas pruebas.
4. No descargará, sustituirá ni actualizará PawnIO en tiempo de ejecución.
5. Se invocará sin UAC en sesiones posteriores; instalar, actualizar o reparar PawnIO seguirá
   siendo una acción explícita que puede requerir una nueva elevación.

La configuración de la tarea queda fijada por T152: sin desencadenadores, bajo demanda, sin
instancias concurrentes, sin contraseña, con `Author = ThrottleWatch` y una descripción explícita
de que es el lanzador elevado bajo demanda del sidecar y que termina al cerrar la sesión IPC.
T152 está hecha con resultado (b): nivel A solo es viable en el flujo elevado; el usuario estándar
conserva B/C.

La comunicación entre la interfaz sin privilegios y el lanzador se hará por una tubería con nombre
creada con una ACL mínima para el usuario interactivo y el sistema. La frontera de seguridad real
es una lista cerrada de comandos, no la identidad del proceso del mismo usuario: cualquier proceso
de ese usuario puede intentar lanzar la tarea programada. Por ello el lanzador aceptará únicamente
los comandos tipados `StartSession`, `Heartbeat`, `RecheckCoverage` y `StopSession`, con los campos
definidos por contrato; no existirán comandos de escritura genérica, escritura de MSR, carga de
rutas arbitrarias ni ejecución de texto recibido por IPC (R1). Se reserva el futuro comando tipado
`ClearLimitLog`, limitado exclusivamente a limpiar los bits de registro de `0x64F` conforme a
FR-084; hoy no existe ni se acepta. No habrá ninguna otra escritura de MSR, memoria o dispositivo.
Su incorporación futura exigirá primero una prueba negativa que rechace todo registro distinto de
`0x64F` y toda operación distinta de esos bits.

La secuencia y la entrega del nonce serán (R4):

1. La UI invoca el comando Tauri sin privilegios `request_low_level_access`.
2. Rust valida el estado, genera en memoria un nonce criptográficamente aleatorio y abre una
   tubería de sesión con ACL mínima. El nombre de la tubería es aleatorio por sesión, no contiene
   el nonce y se crea con `FILE_FLAG_FIRST_PIPE_INSTANCE`; una ocupación previa provoca rechazo,
   no reutilización de una tubería existente.
3. Rust inicia la tarea programada. El lanzador se conecta a la tubería, recibe un único
   `StartSession { protocol_version, nonce, session_id }` y valida el mensaje antes de iniciar
   el sidecar. El nonce nunca pasa por línea de comandos, variables de entorno o disco.
4. El lanzador inicia el sidecar desde la ruta fija, le entrega el mismo nonce únicamente como el
   primer `hello` del canal IPC privado y reenvía solo mensajes del contrato. El sidecar debe
   devolver el nonce en cada envelope; Rust valida la coincidencia, la secuencia y el tamaño.
5. Durante la sesión solo se permiten `Heartbeat`, `RecheckCoverage` y `StopSession`; el cierre de
   la UI, EOF o la ausencia de latido detienen el sidecar y limpian el estado efímero.

El lanzador aplicará esta secuencia antes de aceptar muestras:

1. abrir la tubería segura y validar el nonce de la sesión;
2. verificar la ruta fija, el manifiesto de release firmado con minisign y, cuando exista
   certificado, la firma Authenticode/editor permitido del sidecar;
3. iniciar el sidecar con el contrato NDJSON autenticado;
4. mantener un latido y una regla de vida: desconexión de la interfaz, EOF o ausencia de latido
   durante el intervalo fijado detiene el sidecar y termina el lanzador;
5. cerrar la tubería y limpiar el estado efímero al finalizar.

La acción «Desactivar acceso avanzado» impedirá nuevas activaciones de la tarea y hará que la
interfaz vuelva a modo B/C. No detendrá ni desinstalará PawnIO, que es un controlador compartido.

## Revisión de amenazas

| Amenaza | Salvaguarda | Residuo aceptado |
|---|---|---|
| WebView o proceso local intenta obtener privilegios | La WebView no crea procesos; la tarea solo ejecuta el lanzador de ruta fija y la frontera de comandos es cerrada | Cualquier proceso del mismo usuario puede intentar lanzar la tarea; no se trata al mismo usuario como frontera de confianza |
| Sustitución del sidecar | Ruta fija, manifiesto minisign del actualizador y Authenticode cuando exista certificado, con política de versión verificada antes del arranque | La confianza de la cadena de firma del instalador queda fuera del runtime |
| Cliente no autorizado en la tubería | ACL mínima, nombre aleatorio por sesión, `FILE_FLAG_FIRST_PIPE_INSTANCE`, nonce, secuencia creciente y límite de tamaño | Cualquier proceso del mismo usuario puede intentar lanzar la tarea; la frontera cerrada de comandos limita el residuo |
| Ocupación del nombre de tubería | Nombre aleatorio por sesión y `FILE_FLAG_FIRST_PIPE_INSTANCE`; se rechaza una instancia ya ocupada | Un proceso que gane la carrera puede provocar un fallo de disponibilidad de esa sesión, acotado a reiniciar la sesión |
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
  permitir un lanzador bajo demanda si esta propuesta es aprobada; no eleva la UI.
- T153–T155 quedan desbloqueadas tras esta aceptación condicionada y deben conservar sus
  controles, firma, ACL y pruebas Windows; R7 sigue siendo condición de cierre de T154.
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

## Condiciones de aprobación (revisión R1–R7)

- **R1 (bloqueante):** la frontera del lanzador es el enum cerrado de comandos anterior; no hay
  escrituras genéricas ni se confía en que el mismo usuario sea una frontera suficiente.
- **R2 (bloqueante):** la identidad del sidecar se verifica mediante manifiesto de release firmado
  con minisign por el actualizador y Authenticode cuando exista certificado; en desarrollo se usa
  la clave de desarrollo y nunca se desactiva la verificación.
- **R3 (bloqueante):** se declara que el nivel A exige una cuenta administradora para la instalación
  o reparación inicial; el usuario estándar conserva B/C.
- **R4 (bloqueante):** la secuencia UI → Rust → lanzador → sidecar y la entrega en memoria del
  nonce están especificadas arriba.
- **R5:** el lanzador no descarga, actualiza ni elimina PawnIO.
- **R6:** la desactivación vuelve a B/C y no desinstala PawnIO.
- **R7 (cierre de T154):** el ciclo completo requiere pruebas negativas y prueba Windows del
  watchdog antes de cerrar T154; no es una condición adicional de aceptación de este ADR.

## Condiciones añadidas por la aprobación independiente (C1–C6)

- **C1:** la primera fila de amenazas reconoce que cualquier proceso del mismo usuario puede
  intentar lanzar la tarea; la frontera de comandos cerrada acota el residuo.
- **C2:** `ClearLimitLog` queda reservado como comando tipado futuro para limpiar únicamente los
  bits de log de `0x64F` (FR-084). Hoy no existe; no se permite ninguna otra escritura y su futura
  implementación requerirá pruebas negativas antes del código positivo.
- **C3:** el manifiesto minisign del actualizador es obligatorio, Authenticode se comprueba cuando
  exista certificado y el modo desarrollo usa una clave de desarrollo; nunca se omite la
  verificación.
- **C4:** cada sesión usa un nombre de tubería aleatorio y `FILE_FLAG_FIRST_PIPE_INSTANCE`; una
  ocupación del nombre es una amenaza de disponibilidad con residuo acotado a esa sesión.
- **C5:** R7 es condición de cierre de T154, no de aceptación de este ADR.
- **C6:** T152 está hecha con resultado (b) y la tarea se configura bajo demanda, sin
  desencadenadores, sin instancias concurrentes, sin contraseña, con autor y descripción.
