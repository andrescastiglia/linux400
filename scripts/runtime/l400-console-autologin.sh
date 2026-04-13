#!/bin/sh
# l400-console-autologin.sh - Login automático para consola Linux/400

tty_name="$(/bin/busybox tty 2>/dev/null || true)"
login_user="l400"
boot_mode=""
run_dir="${L400_RUN_DIR:-/run/l400}"

if [ -f "${run_dir}/boot-mode" ]; then
    boot_mode="$(cat "${run_dir}/boot-mode" 2>/dev/null || true)"
fi

case "${tty_name}" in
    /dev/ttyS*)
        login_user="root"
        ;;
esac

case "${boot_mode}" in
    rescue)
        login_user="root"
        ;;
esac

exec /bin/busybox login -f "${login_user}"
