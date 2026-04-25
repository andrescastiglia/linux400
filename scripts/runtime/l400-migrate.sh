#!/bin/sh
set -eu

l400_root="${L400_ROOT:-/l400}"
target_version="${L400_METADATA_VERSION:-1}"
version_file="${l400_root}/.metadata-version"

if [ ! -d "${l400_root}" ]; then
    echo "ERROR: ${l400_root} no existe" >&2
    exit 1
fi

current_version="$(cat "${version_file}" 2>/dev/null || echo 0)"
echo "=== l400-migrate ${current_version} -> ${target_version} ==="

if [ "${current_version}" = "${target_version}" ]; then
    echo "metadata_version=${current_version}"
    echo "status=already-current"
    exit 0
fi

if [ "${current_version}" -gt "${target_version}" ] 2>/dev/null; then
    echo "ERROR: downgrade de metadata no soportado (${current_version} > ${target_version})" >&2
    exit 2
fi

tmp_file="${version_file}.$$"
printf '%s\n' "${target_version}" > "${tmp_file}"
mv "${tmp_file}" "${version_file}"

if command -v l400-bootstrap >/dev/null 2>&1; then
    l400-bootstrap --quiet || true
fi

echo "metadata_version=${target_version}"
echo "status=migrated"
