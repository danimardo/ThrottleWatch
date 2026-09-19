# Trazas y sidecar de replay

`tools/replay.mjs` emite una traza JSON como NDJSON por `stdout`, para probar el canal IPC sin
leer sensores ni acceder a hardware. El proceso termina con un código distinto de cero cuando
la traza no contiene envolventes válidas.

```powershell
node packages/trace-fixtures/tools/replay.mjs --trace packages/trace-fixtures/intel-thermal-confirmed.json
```
