#!/bin/sh
# l400-console-autologin.sh - Login automático para consola Linux/400

tty_name="$(/bin/busybox tty 2>/dev/null || true)"
login_user="l400"

case "${tty_name}" in
    /dev/ttyS*)
        login_user="root"
        ;;
esac

exec /bin/busybox login -f "${login_user}"
