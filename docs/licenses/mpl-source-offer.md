# Código fuente cubierto por MPL-2.0

Este registro acompaña a las distribuciones de ThrottleWatch que incluyen
`LibreHardwareMonitorLib` 0.9.6 en el sidecar `SensorAgent.exe`.

## Corresponding Source

- Componente: `LibreHardwareMonitorLib` 0.9.6.
- Licencia: Mozilla Public License 2.0.
- Versión fijada: `apps/sensor-agent/SensorAgent.csproj` y
  `apps/sensor-agent/packages.lock.json`.
- Código fuente y texto de licencia: repositorio oficial de LibreHardwareMonitor,
  etiqueta correspondiente a la versión 0.9.6.
- Copia de la licencia MPL-2.0: `docs/licenses/rust-third-party.html` y el aviso
  incluido en el paquete publicado.

La forma preferida para modificar el componente es su código fuente upstream.
El proceso de publicación debe conservar esta referencia, los avisos de copyright
y la versión exacta usada para construir el binario. El SBOM CycloneDX y los
informes de licencias se regeneran desde los manifiestos bloqueados mediante:

```text
node scripts/generate-sbom.mjs
```

No se distribuye una copia modificada del componente en este repositorio. Si una
release incorpora cambios locales en archivos cubiertos por MPL-2.0, la release
debe adjuntar esos archivos modificados y este registro debe actualizarse con el
commit o archivo fuente correspondiente antes de publicarse.

## Checklist de publicación

- [ ] Confirmar que la versión del paquete coincide con los manifiestos bloqueados.
- [ ] Incluir `THIRD-PARTY-NOTICES`, `docs/licenses/` y el SBOM en el artefacto.
- [ ] Verificar que el enlace de Corresponding Source sigue disponible.
- [ ] Si existen modificaciones locales, adjuntar sus fuentes y avisos.
