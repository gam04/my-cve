# Política de seguridad

## Versiones soportadas

Solo la última release publicada recibe correcciones. Al no haber backend ni
servicio remoto, el alcance es el binario y el código del repositorio.

## Reportar una vulnerabilidad

**No abras un issue público.** Usá la pestaña *Security* → *Advisories* →
*Report a vulnerability* del repositorio (GitHub Private Vulnerability
Reporting), o escribí a `gam04` en GitHub si esa opción no está disponible.

Incluí, si podés:

- versión de la app y de Windows;
- pasos de reproducción y resultado esperado vs. observado;
- impacto (ejecución de código, lectura de archivos, salida a red no prevista);
- PoC o captura, si aplica.

Compromiso de respuesta: acuse de recibo en 72 h y evaluación inicial en 7 días.

## Alcance

Aplica al código de `Ficha del día` (frontend Svelte, backend Rust, configuración
Tauri y workflows). **No** aplica a vulnerabilidades en los datos de NVD, FIRST
EPSS o CISA KEV, ni a las CVEs que la app muestra: eso se reporta a cada proyecto.

## Propiedades de seguridad esperadas

- Sin telemetría ni analytics.
- Único tráfico saliente: `services.nvd.nist.gov`, `api.first.org`, `www.cisa.gov`.
- `NVD_API_KEY` se lee solo de la variable de entorno homónima; nunca se escribe
  en ajustes, caché ni logs.
- CSP sin `unsafe-inline` ni `unsafe-eval`; sin `innerHTML` con datos remotos.
- Los enlaces externos se abren con el opener del sistema, nunca dentro del WebView.

Cualquier desviación de estas propiedades se considera un defecto reportable.
