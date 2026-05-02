# l400-loader

## Objetivo

`l400-loader` es el cargador privilegiado del programa eBPF LSM. Su objetivo es adjuntar la politica kernel, publicar el estado efectivo del sistema y permitir que Linux/400 opere de forma explicita en modo `full`, `degraded` o `dev`.

Tambien funciona como pieza de soporte: si el host no cumple los requisitos, debe dejar claro que el sistema esta operando sin enforcement kernel completo.

## Nivel de avance

Estado: **medio-alto**.

Ya soporta modos operativos, carga/adjunta eBPF cuando el entorno lo permite y persiste estado para reportes y TUI. El flujo esta cubierto por smoke tests de loader.

Para plena funcionalidad faltan:

- integracion de servicio de arranque instalada y monitoreo continuo;
- comandos operativos para ver, reiniciar o diagnosticar la politica activa;
- mensajes de soporte mas completos para fallos de BTF, permisos, kernel o artefacto ausente;
- cobertura e2e sistematica en perfil `full`;
- estrategia de actualizacion/PTF del artefacto eBPF con rollback.

## Modos y estado publicado

`l400-loader` publica su estado en:

```text
${L400_RUN_DIR:-/run/l400}/loader-status
```

Campos relevantes:

| Campo | Significado |
| --- | --- |
| `mode` | `full`, `degraded` o `dev`. |
| `protection_active` | `1` si los hooks estan activos. |
| `phase` | `starting`, `active`, `fallback`, `stopped`, etc. |
| `attached_hooks` | Hooks LSM adjuntados. |
| `policy_version` | Version de contrato esperada por userspace. |
| `last_error` | Error de carga o adjunte si aplica. |

Modos:

- `full`: requiere enforcement activo o falla.
- `degraded`: intenta activar enforcement; si falla, continua sin proteccion kernel y lo reporta.
- `dev`: tolera entorno incompleto para desarrollo local.

Diagnostico rapido:

```bash
cat "${L400_RUN_DIR:-/run/l400}/loader-status"
l400-support-report --write
cargo run -p l400-loader -- --mode dev --once
cargo run -p l400-loader -- --mode degraded --once
```
