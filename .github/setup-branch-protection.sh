#!/bin/bash
# .github/setup-branch-protection.sh
# Configura las reglas de protección de branches via GitHub REST API.
#
# Requiere un Personal Access Token con permisos: repo (o Administration: write en fine-grained tokens)
#
# Uso:
#   GITHUB_TOKEN=<tu_token> bash .github/setup-branch-protection.sh
#
# O exporta GITHUB_TOKEN antes de correr el script.

set -e

OWNER="andrescastiglia"
REPO="linux400"
API="https://api.github.com/repos/${OWNER}/${REPO}/branches"

if [[ -z "$GITHUB_TOKEN" ]]; then
    echo "ERROR: GITHUB_TOKEN no está definido."
    echo "Exporta tu Personal Access Token antes de correr este script:"
    echo "  export GITHUB_TOKEN=ghp_xxxx..."
    exit 1
fi

AUTH_HEADER="Authorization: Bearer ${GITHUB_TOKEN}"
ACCEPT_HEADER="Accept: application/vnd.github+json"
API_VERSION="X-GitHub-Api-Version: 2022-11-28"

put_protection() {
    local BRANCH="$1"
    local PAYLOAD="$2"
    echo ">> Configurando protección en branch: ${BRANCH}"
    RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" \
        -X PUT \
        -H "$AUTH_HEADER" \
        -H "$ACCEPT_HEADER" \
        -H "$API_VERSION" \
        -H "Content-Type: application/json" \
        -d "$PAYLOAD" \
        "${API}/${BRANCH}/protection")
    if [[ "$RESPONSE" == "200" ]]; then
        echo "   OK (HTTP ${RESPONSE})"
    else
        echo "   ERROR (HTTP ${RESPONSE}) — verifica permisos del token."
        exit 1
    fi
}

# ── main: bloquear push directo, solo PR permitido ────────────────────────────
MAIN_PAYLOAD=$(cat <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["Tests (l400)"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 0
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false
}
EOF
)
put_protection "main" "$MAIN_PAYLOAD"

# ── testing: bloquear push directo, solo PR permitido ─────────────────────────
TESTING_PAYLOAD=$(cat <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["Tests (l400)"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 0
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false
}
EOF
)
put_protection "testing" "$TESTING_PAYLOAD"

# ── develop: solo bloquear force-push y eliminación ──────────────────────────
DEVELOP_PAYLOAD=$(cat <<'EOF'
{
  "required_status_checks": null,
  "enforce_admins": false,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false
}
EOF
)
put_protection "develop" "$DEVELOP_PAYLOAD"

echo ""
echo "=== Protección de branches configurada ==="
echo "  main    → solo PR (status check: 'Tests (l400)' requerido)"
echo "  testing → solo PR (status check: 'Tests (l400)' requerido)"
echo "  develop → push directo permitido, force-push bloqueado"
