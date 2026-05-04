#!/bin/sh
# l400-session.sh - Lanza la TUI por defecto para sesiones interactivas del usuario qsecofr

set -eu

fallback_shell="/bin/sh"
os400_tui_bin="/usr/local/bin/os400-tui"
boot_mode=""
run_dir="${L400_RUN_DIR:-/run/l400}"

trap '' INT QUIT TSTP

export PATH="/usr/local/sbin:/usr/local/bin:/opt/l400/bin:/usr/sbin:/usr/bin:/sbin:/bin"
export L400_ROOT="${L400_ROOT:-/l400}"
export L400_LIB_PATH="${L400_LIB_PATH:-/lib/l400}"
export LIBRARY_PATH="/lib/l400${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LD_LIBRARY_PATH="/lib/l400${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

if [ -f /etc/profile.d/l400-env.sh ]; then
    # Carga el entorno Linux/400 incluso cuando busybox login invoca este script como shell.
    . /etc/profile.d/l400-env.sh
fi

if [ "$#" -gt 0 ]; then
    exec "${fallback_shell}" "$@"
fi

if [ -f "${run_dir}/boot-mode" ]; then
    boot_mode="$(cat "${run_dir}/boot-mode" 2>/dev/null || true)"
fi

if [ -z "${L400_SKIP_BOOTSTRAP:-}" ]; then
    bootstrap_bin="/usr/local/bin/l400-bootstrap"
    if [ ! -x "${bootstrap_bin}" ] && [ -x /opt/l400/bin/l400-bootstrap ]; then
        bootstrap_bin="/opt/l400/bin/l400-bootstrap"
    fi
    if [ -x "${bootstrap_bin}" ]; then
        "${bootstrap_bin}" --quiet >/dev/null 2>&1 || \
            echo "Linux/400: no se pudo inicializar el catalogo base." >&2
    fi
fi

case "${boot_mode}" in
    rescue)
        echo "=== Linux/400 Rescue Mode ==="
        echo "Opciones:"
        echo "  1) Montar /l400"
        echo "  2) Support report"
        echo "  3) Upgrade check"
        echo "  4) Restore from backup"
        echo "  5) Shell"
        echo ""
        printf "Seleccione una opción [1-5]: "
        read -r rescue_option

        case "${rescue_option}" in
            1)
                echo "Montando /l400..."
                if [ -x /usr/local/bin/l400-mount-l400 ]; then
                    /usr/local/bin/l400-mount-l400
                else
                    mount /l400 2>/dev/null || mount -a 2>/dev/null || true
                fi
                echo "Presione Enter para continuar..."
                read -r
                exec "${fallback_shell}"
                ;;
            2)
                echo "Generando support report..."
                if [ -x /usr/local/bin/l400-support-report ]; then
                    /usr/local/bin/l400-support-report
                else
                    echo "l400-support-report no disponible."
                    echo "Mostrando información básica:"
                    echo "Boot mode: ${boot_mode}"
                    cat /run/l400/boot-mode 2>/dev/null || echo "No boot-mode file"
                    echo "L400_ROOT: ${L400_ROOT:-/l400}"
                    ls -la /l400 2>/dev/null || echo "/l400 no existe o no está montado"
                fi
                echo "Presione Enter para continuar..."
                read -r
                exec "${fallback_shell}"
                ;;
            3)
                echo "Ejecutando upgrade check..."
                if [ -x /usr/local/bin/l400-upgrade-check ]; then
                    /usr/local/bin/l400-upgrade-check
                else
                    echo "l400-upgrade-check no disponible."
                fi
                echo "Presione Enter para continuar..."
                read -r
                exec "${fallback_shell}"
                ;;
            4)
                echo "Restore from backup..."
                if [ -x /usr/local/bin/l400-restore ]; then
                    /usr/local/bin/l400-restore
                else
                    echo "l400-restore no disponible."
                    echo "Puede restaurar manualmente desde /var/backups/l400/"
                fi
                echo "Presione Enter para continuar..."
                read -r
                exec "${fallback_shell}"
                ;;
            5|*)
                exec "${fallback_shell}"
                ;;
        esac
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
if [ "${TERM:-dumb}" = "dumb" ]; then
    case "${current_tty}" in
        /dev/ttyS*)
            export TERM="vt100"
            ;;
        /dev/tty*)
            export TERM="linux"
            ;;
        *)
            exec "${fallback_shell}"
            ;;
    esac
fi

if [ -n "${L400_TUI_ACTIVE:-}" ]; then
    exit 1
fi

if [ ! -x "${os400_tui_bin}" ] && [ -x /opt/l400/bin/os400-tui ]; then
    os400_tui_bin="/opt/l400/bin/os400-tui"
fi

while true; do
    stty sane 2>/dev/null || true

    if [ ! -x "${os400_tui_bin}" ]; then
        echo "Linux/400: os400-tui no está disponible; reiniciando sesión OS/400." >&2
        sleep 1
        continue
    fi

    export L400_TUI_ACTIVE=1
    if "${os400_tui_bin}"; then
        unset L400_TUI_ACTIVE
        stty sane 2>/dev/null || true
        sleep 1
        continue
    fi

    unset L400_TUI_ACTIVE
    stty sane 2>/dev/null || true
    echo "Linux/400: os400-tui terminó inesperadamente; reiniciando sesión OS/400." >&2
    sleep 1
done
