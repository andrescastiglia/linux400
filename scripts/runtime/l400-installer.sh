#!/bin/sh
# l400-installer.sh - Instalador textual de Linux/400 para el modo boot install

set -eu

PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
export PATH

fallback_shell="/bin/sh"

trap '' INT QUIT TSTP

if [ -n "${L400_INSTALLER_ACTIVE:-}" ]; then
    exec "${fallback_shell}"
fi
export L400_INSTALLER_ACTIVE=1

if [ "$(id -u)" -ne 0 ]; then
    echo "Linux/400 installer: se requieren privilegios root." >&2
    exec "${fallback_shell}"
fi

if [ ! -t 0 ] || [ ! -t 1 ]; then
    echo "Linux/400 installer: se requiere una consola interactiva." >&2
    exec "${fallback_shell}"
fi

clear_screen() {
    printf '\033[2J\033[H'
}

prepare_terminal() {
    stty sane 2>/dev/null || true
    export TERM="${TERM:-vt100}"
}

list_disks() {
    if command -v lsblk >/dev/null 2>&1; then
        lsblk -d -P -n -o PATH,SIZE,TYPE,MODEL 2>/dev/null | while IFS= read -r line; do
            path="$(printf '%s\n' "${line}" | sed -n 's/.*PATH="\([^"]*\)".*/\1/p')"
            size="$(printf '%s\n' "${line}" | sed -n 's/.*SIZE="\([^"]*\)".*/\1/p')"
            type="$(printf '%s\n' "${line}" | sed -n 's/.*TYPE="\([^"]*\)".*/\1/p')"
            model="$(printf '%s\n' "${line}" | sed -n 's/.*MODEL="\([^"]*\)".*/\1/p')"
            [ "${type}" = "disk" ] || continue
            printf '%s\t%s\t%s\n' "${path}" "${size}" "${model}"
        done
        return 0
    fi

    for dev in /dev/vd? /dev/sd? /dev/nvme?n1 /dev/mmcblk?; do
        [ -b "${dev}" ] || continue
        printf '%s\tunknown\t-\n' "${dev}"
    done
}

show_header() {
    clear_screen
    cat <<'EOF'
============================================================
 Linux/400 Installation Manager
============================================================

Este asistente va a:
  1. Particionar el disco destino en GPT
  2. Crear EFI + root ext4
  3. Copiar el sistema Linux/400 completo
  4. Instalar el boot UEFI para arrancar sin la ISO

ATENCION: el disco elegido sera borrado por completo.
EOF
    printf '\n'
}

prompt_disk() {
    local selection=""

    while :; do
        show_header
        echo "Discos detectados:"
        list_disks || true
        printf '\n'
        printf 'Ingrese el disco destino (ej: /dev/vda) o escriba shell para salir: '
        IFS= read -r selection
        selection="$(printf '%s' "${selection}" | tr -d '[:space:]')"

        case "${selection}" in
            shell)
                exec "${fallback_shell}"
                ;;
            "")
                ;;
            /dev/*)
                if [ -b "${selection}" ]; then
                    printf '%s' "${selection}"
                    return 0
                fi
                ;;
        esac

        printf '\nDisco invalido. Presione Enter para reintentar...'
        IFS= read -r _
    done
}

confirm_install() {
    local disk="$1"
    local confirmation=""

    printf '\nDestino seleccionado: %s\n' "${disk}"
    printf 'Escriba INSTALL para continuar con el formateo: '
    IFS= read -r confirmation
    [ "${confirmation}" = "INSTALL" ]
}

finish_menu() {
    local action=""

    printf '\nInstalacion completada.\n'
    printf 'Quite la ISO antes del proximo arranque.\n'
    printf '\nOpciones: [R]eboot, [P]oweroff, [S]hell: '
    IFS= read -r action
    action="$(printf '%s' "${action}" | tr '[:lower:]' '[:upper:]')"

    case "${action}" in
        R|"")
            reboot -f || exec "${fallback_shell}"
            ;;
        P)
            poweroff -f || halt -f || exec "${fallback_shell}"
            ;;
        S)
            exec "${fallback_shell}"
            ;;
        *)
            finish_menu
            ;;
    esac
}

main() {
    local disk=""

    prepare_terminal
    disk="$(prompt_disk)"
    if ! confirm_install "${disk}"; then
        printf '\nInstalacion cancelada. Presione Enter para volver al selector...'
        IFS= read -r _
        unset L400_INSTALLER_ACTIVE
        exec "$0"
    fi

    printf '\nInstalando Linux/400 en %s...\n\n' "${disk}"
    if install-linux400 "${disk}"; then
        finish_menu
    fi

    printf '\nLa instalacion fallo. Presione Enter para abrir una shell de soporte...'
    IFS= read -r _
    exec "${fallback_shell}"
}

main "$@"
