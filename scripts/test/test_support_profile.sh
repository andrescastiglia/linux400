#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
TMP_RUN="$(mktemp -d)"
trap 'rm -rf "$TMP_RUN"' EXIT

DEV_RUN="${TMP_RUN}/dev"
mkdir -p "${DEV_RUN}"
cat > "${DEV_RUN}/loader-status" <<'EOF'
mode=dev
protection_active=0
phase=fallback
attached_hooks=
policy_version=
last_error=test-dev
EOF
printf 'live\n' > "${DEV_RUN}/boot-mode"

DEV_OUTPUT="$(L400_RUN_DIR="${DEV_RUN}" sh "${ROOT_DIR}/scripts/runtime/l400-support-report.sh" --write)"
printf '%s\n' "${DEV_OUTPUT}"
grep -q '^loader_mode=dev$' <<<"${DEV_OUTPUT}"
grep -q '^effective_mode=dev$' <<<"${DEV_OUTPUT}"
grep -q '^phase3_enforcement_ready=no$' <<<"${DEV_OUTPUT}"
grep -q '^required_for_full=' <<<"${DEV_OUTPUT}"
grep -q '^effective_mode=dev$' "${DEV_RUN}/support-profile"

DEGRADED_RUN="${TMP_RUN}/degraded"
mkdir -p "${DEGRADED_RUN}"
cat > "${DEGRADED_RUN}/loader-status" <<'EOF'
mode=degraded
protection_active=0
phase=fallback
attached_hooks=
policy_version=
last_error=test-degraded
EOF
printf 'installed\n' > "${DEGRADED_RUN}/boot-mode"

DEGRADED_OUTPUT="$(L400_RUN_DIR="${DEGRADED_RUN}" sh "${ROOT_DIR}/scripts/runtime/l400-support-report.sh" --write)"
printf '%s\n' "${DEGRADED_OUTPUT}"
grep -q '^loader_mode=degraded$' <<<"${DEGRADED_OUTPUT}"
grep -q '^effective_mode=degraded$' <<<"${DEGRADED_OUTPUT}"
grep -q '^phase3_enforcement_ready=no$' <<<"${DEGRADED_OUTPUT}"
grep -q '^cgroup_v2=' <<<"${DEGRADED_OUTPUT}"
grep -q '^btf_vmlinux=' <<<"${DEGRADED_OUTPUT}"
grep -q '^effective_mode=degraded$' "${DEGRADED_RUN}/support-profile"

PHASE3_RUN="${TMP_RUN}/phase3"
mkdir -p "${PHASE3_RUN}"
cat > "${PHASE3_RUN}/loader-status" <<'EOF'
mode=full
protection_active=1
phase=active
attached_hooks=file_open,bprm_creds_from_file,bprm_check_security
policy_version=phase3-v1
last_error=none
EOF

PHASE3_OUTPUT="$(L400_RUN_DIR="${PHASE3_RUN}" sh "${ROOT_DIR}/scripts/runtime/l400-support-report.sh" --write)"
printf '%s\n' "${PHASE3_OUTPUT}"
grep -q '^loader_mode=full$' <<<"${PHASE3_OUTPUT}"
grep -q '^loader_hooks_ok=yes$' <<<"${PHASE3_OUTPUT}"
grep -q '^loader_policy_ok=yes$' <<<"${PHASE3_OUTPUT}"
grep -q '^phase3_enforcement_ready=yes$' <<<"${PHASE3_OUTPUT}"
grep -q '^phase3_enforcement_ready=yes$' "${PHASE3_RUN}/support-profile"
