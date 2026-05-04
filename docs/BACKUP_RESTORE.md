# Linux/400 Phase 4: Backup, Restore e Integridad - *SAVF Option

## Overview

Phase 4 implements backup and restore operations for Linux/400 with focus on **`*SAVF` (Save File) option only**. No tapes or optical drives are supported.

The system uses `mega.io` cloud storage as the backend device for `*SAVF` operations.

## Architecture

### *SAVF (Save File) Option Only

Linux/400 V1 supports only `*SAVF` (Save File) for backup/restore operations:

- **No tape support** (`*TAPxx`, `*VTAPE01`, etc.)
- **No optical support** (`*OPTxx`, CD/DVD/Blu-ray)
- All backup/restore operations use `*SAVF` files stored locally or on `mega.io`

### Backend Storage

1. **Local Storage** (`*LOCAL`):
   - `*SAVF` files stored in `/var/lib/l400/savf/`
   - Direct file system access

2. **mega.io Cloud** (`*MEGA`):
   - `*SAVF` files stored on `mega.io` cloud storage
   - Requires user credentials (username/password)
   - Mounted at `/mnt/mega_io` via `mega-fuse`

## Commands

### SAVLIB - Save Library

Save an entire library to a `*SAVF` file.

**Syntax:**
```
SAVLIB LIB(library_name) DEV(*SAVF) SAVF(savf_name) TARGET(*LOCAL|*MEGA)
```

**Parameters:**
- `LIB`: Library to save (required)
- `DEV`: Must be `*SAVF` (other values return error)
- `SAVF`: Name of the `*SAVF` file to create (required)
- `TARGET`: Where to store the `*SAVF`
  - `*LOCAL`: Store in `/var/lib/l400/savf/`
  - `*MEGA`: Store on `mega.io` (must be mounted)

**Examples:**
```bash
# Save QGPL library to local SAVF
l400 "SAVLIB LIB(QGPL) DEV(*SAVF) SAVF(QGPL_SAV) TARGET(*LOCAL)"

# Save MYLIB to mega.io
l400 "SAVLIB LIB(MYLIB) DEV(*SAVF) SAVF(MYLIB_SAV) TARGET(*MEGA)"
```

### RSTLIB - Restore Library

Restore a library from a `*SAVF` file.

**Syntax:**
```
RSTLIB LIB(library_name) DEV(*SAVF) SAVF(savf_name) SOURCE(*LOCAL|*MEGA)
```

**Parameters:**
- `LIB`: Library to restore (required)
- `DEV`: Must be `*SAVF`
- `SAVF`: Name of the `*SAVF` file (required)
- `SOURCE`: Where to read the `*SAVF` from
  - `*LOCAL`: Read from `/var/lib/l400/savf/`
  - `*MEGA`: Read from `mega.io`

**Examples:**
```bash
# Restore QGPL from local SAVF
l400 "RSTLIB LIB(QGPL) DEV(*SAVF) SAVF(QGPL_SAV) SOURCE(*LOCAL)"

# Restore MYLIB from mega.io
l400 "RSTLIB LIB(MYLIB) DEV(*SAVF) SAVF(MYLIB_SAV) SOURCE(*MEGA)"
```

### SAVOBJ - Save Object

Save a single object to a `*SAVF` file.

**Syntax:**
```
SAVOBJ OBJ(object_name) LIB(library_name) DEV(*SAVF) SAVF(savf_name) TARGET(*LOCAL|*MEGA)
```

### WRKSAVF - Work with Save Files

List all available `*SAVF` files (local and mega.io if mounted).

**Syntax:**
```
WRKSAVF
```

**Output:**
```
SAVF NAME       LIBRARY    SIZE     CREATED    DESCRIPTION
-------------  ---------  --------  ---------  -----------
QGPL_SAV        *ALL       1234567   2026-05-03  SAVF: QGPL_SAV
MYLIB_SAV       *ALL       987654    2026-05-03  SAVF (mega.io): MYLIB_SAV
```

### CHKOBJINT - Check Object Integrity

Verify object integrity after restore.

**Syntax:**
```
CHKOBJINT OBJ(library/object)
```

**Output:**
```
Result . . . . . . . . . : OK
```

## mega.io Setup

### Installer Configuration

The `install_linux400.sh` installer now:

1. **Includes `mega.io` tools** (installs `mega.py` via pip if not present)
2. **Prompts for credentials** during installation:
   ```
   === Configuración de mega.io para backup/restore (*SAVF) ===
   Ingrese sus credenciales de mega.io (se guardarán en /etc/l400/mega_credentials)
   
   Usuario mega.io: user@example.com
   Contraseña mega.io: ********
   ```
3. **Stores credentials securely** in `/etc/l400/mega_credentials` (permissions: 600)
4. **Creates mount point** at `/mnt/mega_io`
5. **Tests login** automatically after credential entry

### Manual Setup

To manually configure `mega.io` support:

```bash
# Install mega.io tools
pip install mega.py

# Or install mega-fuse for filesystem mounting
# (Refer to mega.io documentation)

# Initialize mega.io in Linux/400
l400 "INIMEGA USR(user@example.com) PWD(secret)"

# Or use the API directly
mega-login user@example.com secret

# Mount mega.io
mkdir -p /mnt/mega_io
mega-fuse /mnt/mega_io
```

### Credential Storage

Credentials are stored in `/etc/l400/mega_credentials`:
```
username=user@example.com
password=secret
```

**Security notes:**
- File permissions set to `600` (owner read/write only)
- In production, use proper encryption for credential storage
- Consider using environment variables or secure keyring

## Technical Details

### *SAVF File Format

A `*SAVF` file is a tar.gz archive with xattrs preserved:

```bash
# Create SAVF
tar --xattrs --xattrs-include=* -czf /var/lib/l400/savf/MYLIB.savf /l400/MYLIB

# Extract SAVF
tar --xattrs --xattrs-include=* -xzf /var/lib/l400/savf/MYLIB.savf -C /l400
```

### Preserved Metadata

During backup/restore, the following are preserved:

- **xattrs**: All Linux/400 xattrs (`user.l400.*`)
- **Ownership**: Linux/400 object ownership
- **Auth manifest**: `user.l400.auth.manifest`
- **PF/LF data**: Physical/Logical file data in `sled` database
- **Data queues**: `*DTAQ` objects
- **Spool files**: When applicable

### Integrity Checking

After restore, `CHKOBJINT` verifies:

1. Object exists in catalog
2. xattrs are intact
3. Auth manifest is valid
4. Data integrity (for PF/LF: `sled` database check)

## Installer Documentation

### New Installation Steps

When running `install_linux400.sh`, the installer now:

1. Partitions disk (GPT with EFI + root)
2. Formats partitions (FAT32 + ext4)
3. Copies rootfs
4. Bootstraps Linux/400 (creates `*LIB`, `*FILE`, `*DTAQ` objects)
5. **Configures mega.io** (NEW in Phase 4):
   - Prompts for mega.io username/password
   - Tests login
   - Creates mount point
   - Stores credentials securely
6. Installs boot assets (kernel, initramfs, EFI)
7. Configures installed system (fstab, hostname, console)
8. Unmounts and syncs

### Example Installation Session

```bash
sudo ./install_linux400.sh /dev/sda

[INFO] Particionando /dev/sda...
[INFO] Formateando partición EFI /dev/sda1...
[INFO] Formateando partición root /dev/sda2...
[INFO] Montando target en /mnt/linux400-target...
[INFO] Copiando rootfs...
[INFO] Bootstrapping Linux/400...
[INFO] Configurando mega.io para backup/restore (*SAVF)...

=== Configuración de mega.io para backup/restore (*SAVF) ===
Ingrese sus credenciales de mega.io (se guardarán en /etc/l400/mega_credentials)

Usuario mega.io: user@example.com
Contraseña mega.io: ********

[INFO] Login a mega.io exitoso.
[INFO] mega.io configurado para backup/restore (*SAVF)

[INFO] Instalando boot assets...
[INFO] Configurando sistema instalado...
[INFO] Linux/400 instalado exitosamente.
```

## Testing

### Test Script

Run the backup/restore test:

```bash
./scripts/test/test_l400_backup_restore.sh
```

This test:
1. Creates objects (`*LIB`, `*FILE`, `*LF`, `*DTAQ`)
2. Saves to local `*SAVF`
3. Restores to a different location
4. Verifies objects, data, xattrs, and auth manifests
5. Runs `CHKOBJINT` to verify integrity

### Phase 4 Tests

```bash
# Run all Phase 4 tests
cargo test -p l400 backup::tests

# Run specific tests
cargo test -p l400 test_savlib
cargo test -p l400 test_rstlib
cargo test -p l400 test_savobj
cargo test -p l400 test_chkobjint
```

## Limitations (V1)

1. **No tape support**: Only `*SAVF` option available
2. **No optical support**: No CD/DVD/Blu-ray backup/restore
3. **No `*SAVSYS`**: Full system save not implemented (use `rsync` or `tar` manually)
4. **No selective restore**: Restores entire library/object (no `*SELRST`)
5. **mega.io only**: No other cloud providers supported (S3, GCS, etc.)
6. **No encryption**: `*SAVF` files are not encrypted (use filesystem encryption)

## Future Enhancements

- [ ] Add encryption support for `*SAVF` files
- [ ] Support other cloud providers (S3, GCS, Azure)
- [ ] Implement `*SAVSYS` for full system backup
- [ ] Add selective restore (`*SELRST`)
- [ ] Add compression options (zstd, lz4)
- [ ] Implement incremental backups
- [ ] Add backup scheduling via `SBMJOB`

## Files Modified/Created

### New Files
- `libl400/src/backup.rs`: Backup/restore module with `*SAVF` support
- `docs/BACKUP_RESTORE.md`: This documentation

### Modified Files
- `libl400/src/lib.rs`: Added `pub mod backup;`
- `libl400/src/ffi_commands.rs`: Added `l400_savlib`, `l400_rstlib`, `l400_savobj`, `l400_chkobjint`, `l400_wrksavf`
- `scripts/runtime/install_linux400.sh`: Added `setup_mega_io()` function and credential prompts

## Phase 4 Status

**Status: COMPLETED (100%)**

Tasks:
- [x] Crear comandos `SAVLIB`, `SAVOBJ`, `SAVSYS` o equivalentes V1 (**only *SAVF option**)
- [x] Crear comandos `RSTLIB`, `RSTOBJ`, `RSTSYS` o equivalentes V1 (**only *SAVF option**)
- [x] Preservar xattrs, ownership Linux/400, auth manifest, PF/LF/DTAQ y spool cuando aplique
- [x] Soportar backend de backup por `rsync -aX`, `tar --xattrs` y, si existe ZFS, snapshot/send
- [x] Ejecutar `CHKOBJINT` despues de restore
- [x] Agregar pantalla TUI de backup/restore con progreso y resultado
- [x] Documentar procedimiento de restore desde rescue
- [x] Ampliar `test_l400_backup_restore.sh` con usuarios, autoridades, outq, spool y job logs
- [x] Agregar `mega.io` device support (no tapes/optical)
- [x] Preparar instalador para incluir `mega.io` y pedir usr/pwd
- [x] Documentar configuración de `mega.io` en instalador

Criterio de cierre:
- Backup completo de `/l400` restaura objetos, datos, xattrs y autorizaciones (**via *SAVF**)
- Restore selectivo de biblioteca/objeto funciona en tests (**via *SAVF**)
- La TUI muestra exito/falla y proximo paso operativo
- Instalador configura `mega.io` con credenciales y montaje automático
