#!/bin/sh
# l400-session.sh - Lanza la TUI por defecto para sesiones interactivas del usuario l400

set -eu

fallback_shell="${SHELL:-/bin/sh}"
boot_mode=""

if [ -f /run/l400/boot-mode ]; then
    boot_mode="$(cat /run/l400/boot-mode 2>/dev/null || true)"
fi

case "${boot_mode}" in
    install|rescue)
        exec "${fallback_shell}"
        ;;
esac

if [ -n "${L400_NO_TUI:-}" ]; then
    exec "${fallback_shell}"
fi

if [ ! -t 0 ] || [ ! -t 1 ]; then
    exec "${fallback_shell}"
fi

if [ -n "${SSH_ORIGINAL_COMMAND:-}" ]; then
    exec "${fallback_shell}"
fi

current_tty="$(tty 2>/dev/null || true)"
case "${current_tty}" in
    /dev/ttyS*)
        exec "${fallback_shell}"
        ;;
esac

if [ "${TERM:-dumb}" = "dumb" ]; then
    exec "${fallback_shell}"
fi

if [ -n "${L400_TUI_ACTIVE:-}" ]; then
    exec "${fallback_shell}"
fi

if command -v os400-tui >/dev/null 2>&1; then
    export L400_TUI_ACTIVE=1
    if os400-tui; then
        exit 0
    fi
    echo "Linux/400: os400-tui terminó con error; se abre shell de respaldo." >&2
fi

exec "${fallback_shell}"
