#!/usr/bin/env bash
# Declares the demo organization's realm and clients in FerrisKey, then
# registers the organization in Capitainerie so the API trusts its issuer.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TF_DIR="${REPO_ROOT}/deploy/ferriskey/terraform"
FERRISKEY_URL="${FERRISKEY_URL:-http://localhost:4110}"
ORGANIZATION_NAME="${ORGANIZATION_NAME:-Capitainerie Demo}"

for tool in terraform docker; do
    command -v "${tool}" >/dev/null 2>&1 || { echo "❌ ${tool} is required" >&2; exit 1; }
done

echo "🏛  realm and clients (${TF_DIR})"
terraform -chdir="${TF_DIR}" init -input=false -no-color >/dev/null
TF_VAR_ferriskey_url="${FERRISKEY_URL}" \
    terraform -chdir="${TF_DIR}" apply -auto-approve -input=false -no-color | tail -1

ISSUER="$(terraform -chdir="${TF_DIR}" output -raw issuer)"
REALM="$(terraform -chdir="${TF_DIR}" output -raw realm)"
CONSOLE_CLIENT_ID="$(terraform -chdir="${TF_DIR}" output -raw console_client_id)"

echo "🏢 organization ${REALM} -> ${ISSUER}"
docker compose -f "${REPO_ROOT}/docker-compose.yml" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U capitainerie -d capitainerie \
    -v slug="${REALM}" -v name="${ORGANIZATION_NAME}" -v issuer="${ISSUER}" >/dev/null <<'SQL'
INSERT INTO organizations (id, slug, name, issuer_url)
VALUES (gen_random_uuid(), :'slug', :'name', :'issuer')
ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name, issuer_url = EXCLUDED.issuer_url;
SQL

cat <<SUMMARY

✅ ready

   Console (apps/console/.env):
     VITE_OIDC_ISSUER_URL=${ISSUER}
     VITE_OIDC_CLIENT_ID=${CONSOLE_CLIENT_ID}

   Create your account from the console's login page ("Register"): FerrisKey
   has no admin API to set a password, so self-registration is the way in.
SUMMARY
