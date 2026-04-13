# Linux/400 v1 Support Matrix

## Estado de soporte RC

| Área | Estado v1 | Notas |
| --- | --- | --- |
| QEMU UEFI x86_64 | **Oficial** | perfil de validación principal |
| Instalación a disco en QEMU | **Oficial** | cubierta por `scripts/test/test_e2e_install_qemu.sh` |
| `tty1` -> TUI | **Oficial** | flujo principal |
| `ttyS0` serial | **Oficial** | depuración y recovery |
| `rescue` | **Oficial** | shell explícita |
| `c400c` | **Oficial** | compila, cataloga `*PGM`, ejecuta |
| `clc` subset v1 | **Oficial** | `PGM`, `SNDPGMMSG`, `ENDPGM` |
| `sled` para `*FILE`/`*DTAQ` | **Oficial** | backend operativo actual |
| ZFS + `xattr=sa` | **Soportado** | modo preferido para metadata/enforcement |
| eBPF LSM `full` | **Soportado** | depende de kernel/BTF/privilegios |
| Loader `degraded` / `dev` | **Oficial** | fallback explícito |
| Workloads con cgroups v2 | **Soportado** | si falla el attach real, queda registro visible en `L400_RUN_DIR` |
| Hardware físico general | **Experimental** | fuera del perfil oficial RC |
| arm64/TBI | **Experimental** | visión de arquitectura, no perfil oficial RC |

## Qué significa cada modo de operación

| Modo | Soporte RC | Uso esperado |
| --- | --- | --- |
| `full` | soportado | demos con enforcement real |
| `degraded` | soportado | hosts donde el hook no puede activarse pero se quiere conservar el runtime |
| `dev` | soportado | desarrollo local y CI manual |

## Dependencias por capacidad

| Capacidad | Requisito |
| --- | --- |
| ISO live/install | `squashfs-tools`, `grub-mkrescue`, `grub-mkstandalone`, `mtools` |
| Instalación QEMU | `expect`, `qemu-img`, `qemu-system-x86_64`, OVMF |
| Enforcement `full` | kernel `>= 6.11`, BPF LSM, BTF accesible, privilegios root |
| Metadata soportada | ZFS con `xattr=sa` |
| Workloads visibles | cgroups v2 o `L400_RUN_DIR` |

## Fuera de alcance para v1

- fidelidad completa a Berkeley DB como backend obligatorio
- soporte amplio de hardware fuera de QEMU/lab
- enforcement universal sobre todas las syscalls
- scheduler BPF propio (`sched_ext`) como requisito operativo
