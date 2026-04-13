# Linux/400 v1 RC Checklist

## Build

- [ ] `./scripts/build/build_release_rc.sh`
- [ ] ISO generada en `output/`
- [ ] `BOOTX64.EFI` generado

## Smoke tests rápidos

- [ ] `./scripts/test/test_objects_v1_demo.sh`
- [ ] `./scripts/test/test_toolchain_v1_demo.sh`
- [ ] `./scripts/test/test_workload_demo.sh`
- [ ] `./scripts/test/test_loader_modes.sh`

## Smoke test de instalación

- [ ] `RUN_E2E_INSTALL=1 ./scripts/test/test_release_rc.sh`
- [ ] live boot correcto
- [ ] instalación a disco correcta
- [ ] boot instalado correcto

## Flujo visible de producto

- [ ] `tty1` entra a `os400-tui`
- [ ] `ttyS0` queda usable para depuración
- [ ] `rescue` abre shell clara
- [ ] `WRKOBJ` muestra catálogo real si existe runtime bajo `/l400`
- [ ] `WRKACTJOB` muestra jobs reales o fallback explícito

## Toolchain

- [ ] `c400c` compila y ejecuta `tests/hola_mundo.c`
- [ ] `clc` compila y ejecuta `tests/prueba.clp`
- [ ] ambos binarios quedan catalogados como `*PGM`

## Objetos y storage

- [ ] demo de objetos v1 reproducible
- [ ] PF/LF operativos
- [ ] `*DTAQ` operativa

## Loader y enforcement

- [ ] `l400-loader --mode dev --once`
- [ ] `l400-loader --mode degraded --once`
- [ ] `l400-loader --mode full --once` documentado como fail-closed
- [ ] `test_e2e_bpf.sh` / `test_e2e_zfs.sh` listos para entorno privilegiado

## Documentación

- [ ] `README.md` actualizado
- [ ] `docs/RELEASE_V1_RC.md` actualizado
- [ ] `docs/SUPPORT_MATRIX.md` actualizado
