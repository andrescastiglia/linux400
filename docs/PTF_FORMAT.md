# Linux/400 PTF Package Format

## Overview

PTF (Program Temporary Fix) packages in Linux/400 are versioned maintenance packages that can be applied, rolled back, and audited. This document defines the package format for V1 (no tapes - SERVICE option only).

## Package Structure

A PTF package is a directory or archive with the following structure:

```
ptf-XXXXXX-vYYYYMMDD/
├── manifest.toml          # Required: Package metadata
├── files/                 # Optional: Files to install
│   ├── binary1
│   ├── binary2
│   └── ...
├── scripts/               # Optional: Pre/post scripts
│   ├── pre-check.sh
│   ├── pre-apply.sh
│   ├── post-apply.sh
│   ├── pre-rollback.sh
│   └── post-rollback.sh
└── checksum.sha256      # Required: Checksums for all files
```

## manifest.toml Format

```toml
[package]
id = "PTF0001"                    # Unique PTF identifier
name = "Fix authority checks"
version = "0.2.1"                 # Target version after apply
release_date = "2026-05-03"
origin_version = "0.2.0"          # Version this PTF applies to
target_profile = "full"            # Target platform profile: full, degraded, dev
description = "Fixes authority checks in DLTOBJ"

[files]
# Files to install, with destination paths
binary1 = { source = "files/l400-bootstrap", dest = "/usr/local/bin/l400-bootstrap", mode = "755" }
binary2 = { source = "files/os400-tui", dest = "/usr/local/bin/os400-tui", mode = "755" }

[scripts]
pre_check = "scripts/pre-check.sh"      # Check prerequisites
pre_apply = "scripts/pre-apply.sh"    # Backup originals
post_apply = "scripts/post-apply.sh" # Verify installation
pre_rollback = "scripts/pre-rollback.sh"
post_rollback = "scripts/post-rollback.sh"

[rollback]
# Files to restore on rollback (automatically generated if not present)
restore_backup = true
backup_dir = "/var/backups/l400/ptf"
```

## PTF Server (SERVICE Option)

Since there are no tapes in V1, PTFs are served via:

1. **Local filesystem**: `/var/cache/l400/ptf/` - Downloaded PTFs
2. **HTTP/HTTPS server**: `ptf-server` daemon that serves PTFs from local cache
3. **Manual install**: Place PTF directory in `/var/cache/l400/ptf/` and apply with `APYPTF`

### PTF Server Daemon (`l400-ptf-server`)

A simple HTTP server that:
- Serves PTF packages from `/var/cache/l400/ptf/`
- Lists available PTFs at `GET /ptf/list`
- Serves PTF package at `GET /ptf/{ptf_id}`
- Validates checksums before serving

## Compilation Mode to Generate PTFs

To create a PTF package, use the `l400-ptf-create` tool:

```bash
# Create a PTF from current build
l400-ptf-create \
    --id PTF0001 \
    --name "Fix authority checks" \
    --origin-version 0.2.0 \
    --target-version 0.2.1 \
    --files l400-bootstrap:/usr/local/bin/l400-bootstrap \
    --files os400-tui:/usr/local/bin/os400-tui \
    --output /var/cache/l400/ptf/ptf-PTF0001-v20260503.tar.gz
```

This tool:
1. Collects specified files
2. Generates `manifest.toml`
3. Runs pre-check scripts
4. Creates checksum file
5. Packages everything into a tar.gz archive

## Apply Process

1. `APYPTF PTF(PTF0001)` runs `l400-upgrade-check` as precheck
2. Downloads PTF from server (or uses local cache)
3. Validates manifest and checksums
4. Runs `pre-check.sh` (if present)
5. Runs `pre-apply.sh` (backup originals)
6. Installs files to destinations
7. Runs `post-apply.sh` (verify)
8. Records apply in audit log: `/var/log/l400/ptf-audit.log`
9. Updates `/l400` metadata version

## Rollback Process

1. `APYPTF PTF(PTF0001) OPTION(*ROLLBACK)` 
2. Reads audit log to find applied PTF
3. Runs `pre-rollback.sh`
4. Restores files from backup in `/var/backups/l400/ptf/`
5. Runs `post-rollback.sh`
6. Records rollback in audit log
7. Reverts `/l400` metadata version

## Audit Log Format

Located at `/var/log/l400/ptf-audit.log`:

```
2026-05-03T10:30:00Z APPLY PTF0001 user=qsecofr build=4e5df62 success
2026-05-03T10:35:00Z ROLLBACK PTF0001 user=qsecofr build=4e5df62 success
```

## Commands

### DSPPTF - Display PTFs

Lists applied and pending PTFs:

```bash
DSPPTF                              # List all PTFs
DSPPTF PTF(PTF0001)                # Show details for specific PTF
DSPPTF OPTION(*APPLIED)              # List applied PTFs only
DSPPTF OPTION(*PENDING)              # List pending PTFs only
```

### APYPTF - Apply or Rollback PTF

```bash
APYPTF PTF(PTF0001) OPTION(*APPLY) CONFIRM(*YES)
APYPTF PTF(PTF0001) OPTION(*ROLLBACK) CONFIRM(*YES)
APYPTF PTF(PTF0001) OPTION(*CHECK)        # Dry run
```

## Directory Locations

- PTF cache: `/var/cache/l400/ptf/`
- PTF backups: `/var/backups/l400/ptf/`
- Audit log: `/var/log/l400/ptf-audit.log`
- PTF server config: `/etc/l400/ptf-server.toml`

## V1 Limitations (No Tapes)

- No tape support (SERVICE option only)
- PTFs served via filesystem or HTTP server
- No PTF groups (individual PTFs only)
- No automatic dependency resolution between PTFs
