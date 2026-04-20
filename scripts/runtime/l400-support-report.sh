#!/bin/sh
# l400-support-report.sh - Reporte y clasificación de capacidades de plataforma Linux/400

set -eu

run_dir="${L400_RUN_DIR:-/run/l400}"
output_path="${run_dir}/support-profile"
write_output=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --write)
            write_output=1
            ;;
        --output)
            shift
            output_path="${1:?missing output path}"
            ;;
        *)
            echo "ERROR: argumento no soportado: $1" >&2
            exit 1
            ;;
    esac
    shift
done

read_status_field() {
    local file="$1"
    local key="$2"
    [ -f "${file}" ] || return 1
    grep "^${key}=" "${file}" 2>/dev/null | tail -n 1 | cut -d= -f2-
}

bool_value() {
    case "${1:-}" in
        1|yes|true|TRUE|YES) echo "yes" ;;
        0|no|false|FALSE|NO) echo "no" ;;
        *) echo "unknown" ;;
    esac
}

yes_no() {
    if [ "${1}" -eq 0 ]; then
        echo "yes"
    else
        echo "no"
    fi
}

loader_status="${run_dir}/loader-status"
loader_mode="$(read_status_field "${loader_status}" mode || true)"
loader_phase="$(read_status_field "${loader_status}" phase || true)"
loader_protection_raw="$(read_status_field "${loader_status}" protection_active || true)"
loader_protection="$(bool_value "${loader_protection_raw}")"
loader_hooks="$(read_status_field "${loader_status}" attached_hooks || true)"
loader_policy_version="$(read_status_field "${loader_status}" policy_version || true)"
loader_error="$(read_status_field "${loader_status}" last_error || true)"
boot_mode="$(cat "${run_dir}/boot-mode" 2>/dev/null || true)"
expected_hooks="file_open,bprm_creds_from_file,bprm_check_security"
expected_policy_version="phase3-v1"

kernel_release="$(uname -r)"
arch="$(uname -m)"

cgroup_v2="no"
[ -f /sys/fs/cgroup/cgroup.controllers ] && cgroup_v2="yes"

btf_vmlinux="no"
[ -f /sys/kernel/btf/vmlinux ] && btf_vmlinux="yes"

bpf_lsm="no"
if [ -f /sys/kernel/security/lsm ] && grep -qw bpf /sys/kernel/security/lsm 2>/dev/null; then
    bpf_lsm="yes"
fi

lam_tbi="unknown"
case "${arch}" in
    x86_64)
        if grep -qw lam /proc/cpuinfo 2>/dev/null; then
            lam_tbi="yes"
        else
            lam_tbi="no"
        fi
        ;;
    aarch64)
        lam_tbi="unknown"
        ;;
esac

zfs_tools="no"
if command -v zfs >/dev/null 2>&1 && command -v zpool >/dev/null 2>&1; then
    zfs_tools="yes"
fi

zfs_xattr_sa="unknown"
zfs_dataset=""
if [ "${zfs_tools}" = "yes" ] && [ -d /l400 ]; then
    zfs_dataset="$(df /l400 2>/dev/null | awk 'NR==2 {print $1}')"
    if [ -n "${zfs_dataset}" ] && zfs get -H -o value xattr "${zfs_dataset}" >/tmp/l400-zfs-xattr.$$ 2>/dev/null; then
        zfs_xattr_value="$(cat /tmp/l400-zfs-xattr.$$ 2>/dev/null || true)"
        rm -f /tmp/l400-zfs-xattr.$$
        if [ "${zfs_xattr_value}" = "sa" ]; then
            zfs_xattr_sa="yes"
        else
            zfs_xattr_sa="no"
        fi
    fi
fi

loader_hooks_ok="no"
if [ "${loader_hooks:-}" = "${expected_hooks}" ]; then
    loader_hooks_ok="yes"
fi

loader_policy_ok="no"
if [ "${loader_policy_version:-}" = "${expected_policy_version}" ]; then
    loader_policy_ok="yes"
fi

phase3_enforcement_ready="no"
if [ "${loader_protection}" = "yes" ] && [ "${loader_hooks_ok}" = "yes" ] && [ "${loader_policy_ok}" = "yes" ]; then
    phase3_enforcement_ready="yes"
fi

required_for_full="yes"
[ "${cgroup_v2}" = "yes" ] || required_for_full="no"
[ "${btf_vmlinux}" = "yes" ] || required_for_full="no"
[ "${bpf_lsm}" = "yes" ] || required_for_full="no"
[ "${zfs_xattr_sa}" = "yes" ] || required_for_full="no"
[ "${phase3_enforcement_ready}" = "yes" ] || required_for_full="no"

effective_mode="degraded"
case "${loader_mode}" in
    dev)
        effective_mode="dev"
        ;;
    full)
        if [ "${required_for_full}" = "yes" ]; then
            effective_mode="full"
        else
            effective_mode="degraded"
        fi
        ;;
    degraded)
        effective_mode="degraded"
        ;;
    *)
        if [ -n "${boot_mode}" ]; then
            effective_mode="degraded"
        else
            effective_mode="dev"
        fi
        ;;
esac

report="kernel_release=${kernel_release}
arch=${arch}
boot_mode=${boot_mode:-unknown}
loader_mode=${loader_mode:-unknown}
loader_phase=${loader_phase:-unknown}
loader_protection_active=${loader_protection}
loader_hooks=${loader_hooks:-unknown}
loader_policy_version=${loader_policy_version:-unknown}
loader_hooks_ok=${loader_hooks_ok}
loader_policy_ok=${loader_policy_ok}
phase3_enforcement_ready=${phase3_enforcement_ready}
bpf_lsm=${bpf_lsm}
btf_vmlinux=${btf_vmlinux}
cgroup_v2=${cgroup_v2}
lam_tbi=${lam_tbi}
zfs_tools=${zfs_tools}
zfs_dataset=${zfs_dataset:-unknown}
zfs_xattr_sa=${zfs_xattr_sa}
required_for_full=${required_for_full}
effective_mode=${effective_mode}
loader_error=${loader_error:-none}
"

if [ "${write_output}" = "1" ]; then
    mkdir -p "$(dirname "${output_path}")"
    printf '%s' "${report}" > "${output_path}"
fi

printf '%s' "${report}"
