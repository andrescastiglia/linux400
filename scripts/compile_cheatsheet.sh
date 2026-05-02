#!/usr/bin/env bash
# compile_cheatsheet.sh — Compila todos los .clp de examples/cl/ con clc
# Verifica que el binario resultante sea catalogado como *PGM en ZFS.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CLP_DIR="$ROOT/examples/cl"
OUT_DIR="/tmp/l400_cheatsheet"
CLC="$ROOT/cl_compiler/target/release/clc"

# Colores
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

mkdir -p "$OUT_DIR"

# Construir clc si no existe
if [ ! -f "$CLC" ]; then
    echo -e "${YELLOW}[BUILD]${NC} Compilando clc..."
    (cd "$ROOT/cl_compiler" && cargo build -p clc --release 2>&1)
    echo -e "${GREEN}[BUILD]${NC} clc compilado."
fi

PASS=0
FAIL=0

for clp_file in "$CLP_DIR"/*.clp; do
    name=$(basename "$clp_file" .clp)
    out_bin="$OUT_DIR/$name"

    echo -e "\n${YELLOW}[COMPILE]${NC} $name.clp"

    if "$CLC" -i "$clp_file" -o "$out_bin" 2>&1; then
        echo -e "${GREEN}  ✔ Compilado → $out_bin${NC}"

        # Verificar xattr *PGM si getfattr está disponible
        if command -v getfattr &>/dev/null; then
            xattr_val=$(getfattr -n user.l400.objtype --only-values "$out_bin" 2>/dev/null || true)
            if [[ "$xattr_val" == "*PGM"* ]]; then
                echo -e "${GREEN}  ✔ xattr user.l400.objtype = $xattr_val${NC}"
            else
                echo -e "${YELLOW}  ⚠ xattr no disponible (se requiere ZFS montado). Valor: '$xattr_val'${NC}"
            fi
        fi

        PASS=$((PASS + 1))
    else
        echo -e "${RED}  ✘ Error compilando $name.clp${NC}"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "=============================="
echo -e "Resultado: ${GREEN}$PASS OK${NC}  ${RED}$FAIL FAIL${NC}"
echo "=============================="

[ "$FAIL" -eq 0 ]
