#!/bin/sh
set -eu

l400_root="${L400_ROOT:-/l400}"
run_dir="${L400_RUN_DIR:-/run/l400}"

echo "=== l400-upgrade-check ==="
echo "l400_root=${l400_root}"

if [ ! -d "${l400_root}" ]; then
    echo "status=fail"
    echo "reason=/l400 missing"
    exit 1
fi

metadata_version="$(cat "${l400_root}/.metadata-version" 2>/dev/null || echo 1)"
echo "metadata_version=${metadata_version}"

if command -v getfattr >/dev/null 2>&1; then
    if getfattr -n user.l400.objtype "${l400_root}/QSYS" >/dev/null 2>&1; then
        echo "xattrs_present=yes"
    else
        echo "xattrs_present=no"
    fi
else
    echo "xattrs_present=unknown"
fi

mkdir -p "${run_dir}"
if command -v l400-support-report >/dev/null 2>&1; then
    l400-support-report --write >/dev/null || true
elif [ -x "$(dirname "$0")/l400-support-report.sh" ]; then
    "$(dirname "$0")/l400-support-report.sh" --write >/dev/null || true
fi

if [ -f "${run_dir}/support-profile" ]; then
    grep '^l400_root_persistent=' "${run_dir}/support-profile" || true
    grep '^effective_mode=' "${run_dir}/support-profile" || true
    grep '^kernel_release=' "${run_dir}/support-profile" || true
fi

echo "backup_recommended=rsync -aX ${l400_root}/ /backup/l400/"
echo "status=ok"
