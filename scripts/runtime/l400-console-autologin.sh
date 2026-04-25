#!/bin/sh
# l400-console-autologin.sh - Login automático para consola Linux/400

login_user="qsecofr"
boot_mode=""
run_dir="${L400_RUN_DIR:-/run/l400}"

if [ -f "${run_dir}/boot-mode" ]; then
    boot_mode="$(cat "${run_dir}/boot-mode" 2>/dev/null || true)"
fi

case "${boot_mode}" in
    install)
        stty sane 2>/dev/null || true
        export TERM="${TERM:-vt100}"
        exec /usr/local/bin/l400-installer
        ;;
    rescue)
        login_user="qsecofr"
        ;;
esac

exec /bin/busybox login -f "${login_user}"
