#!/usr/bin/env bash
# Declares the demo organization's realm and clients in FerrisKey, then
# registers the organization in Capitainerie so the API trusts its issuer.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TF_DIR="${REPO_ROOT}/deploy/ferriskey/terraform"
FERRISKEY_URL="${FERRISKEY_URL:-http://localhost:4110}"
ORGANIZATION_NAME="${ORGANIZATION_NAME:-Capitainerie Demo}"

for tool in terraform docker curl jq; do
    command -v "${tool}" >/dev/null 2>&1 || { echo "❌ ${tool} is required" >&2; exit 1; }
done

echo "🏛  realm and clients (${TF_DIR})"
terraform -chdir="${TF_DIR}" init -input=false -no-color >/dev/null
TF_VAR_ferriskey_url="${FERRISKEY_URL}" \
    terraform -chdir="${TF_DIR}" apply -auto-approve -input=false -no-color | tail -1

ISSUER="$(terraform -chdir="${TF_DIR}" output -raw issuer)"
REALM="$(terraform -chdir="${TF_DIR}" output -raw realm)"
CONSOLE_CLIENT_ID="$(terraform -chdir="${TF_DIR}" output -raw console_client_id)"
API_CLIENT_UUID="$(terraform -chdir="${TF_DIR}" output -raw api_client_uuid)"
API_RESOURCE="$(terraform -chdir="${TF_DIR}" output -raw api_resource)"
MCP_CLIENT_ID="$(terraform -chdir="${TF_DIR}" output -raw mcp_client_id)"
AGENT_CLIENTS="$(terraform -chdir="${TF_DIR}" output -json agent_clients)"
ADMIN_ROLE_UUID="$(terraform -chdir="${TF_DIR}" output -raw admin_role_uuid)"
ADMIN_USERNAME="${ADMIN_USERNAME:-demo}"

# What provider 0.1.0 cannot declare: token exchange, resource ownership and
# exchange policies, through the admin API. Every call below converges.
ADMIN_TOKEN="$(curl -fsS -X POST "${FERRISKEY_URL}/realms/master/protocol/openid-connect/token" \
    -d grant_type=password -d client_id=admin-cli \
    -d "username=${FERRISKEY_ADMIN_USER:-admin}" -d "password=${FERRISKEY_ADMIN_PASSWORD:-admin}" \
    | jq -r .access_token)"
admin() {
    local method="$1" path="$2" body="${3:-}"
    curl -fsS -X "${method}" "${FERRISKEY_URL}/realms/${REALM}${path}" \
        -H "Authorization: Bearer ${ADMIN_TOKEN}" -H 'content-type: application/json' \
        ${body:+-d "${body}"}
}
ensure_policy() {
    local client_uuid="$1" target="$2" impersonation="$3" delegation="$4"
    if admin GET "/clients/${client_uuid}/token-exchange-policies" \
        | jq -e --arg t "${target}" '(if type == "array" then . else .data end)[] | select(.target_audience == $t)' >/dev/null; then
        return
    fi
    admin POST "/clients/${client_uuid}/token-exchange-policies" \
        "{\"target_audience\":\"${target}\",\"allow_impersonation\":${impersonation},\"allow_delegation\":${delegation}}" >/dev/null
}

echo "🔁 token exchange: ${API_RESOURCE} owned by capitainerie-api, api -> agents -> ${MCP_CLIENT_ID}"
admin PUT /settings "{\"allowed_resources\":[\"${API_RESOURCE}\"]}" >/dev/null
admin PUT /resource-owners "{\"owners\":[{\"uri\":\"${API_RESOURCE}\",\"client_id\":\"${API_CLIENT_UUID}\"}]}" >/dev/null
admin PATCH "/clients/${API_CLIENT_UUID}" '{"token_exchange_enabled":true}' >/dev/null
for agent in $(jq -r 'keys[]' <<<"${AGENT_CLIENTS}"); do
    agent_uuid="$(jq -r --arg a "${agent}" '.[$a]' <<<"${AGENT_CLIENTS}")"
    admin PATCH "/clients/${agent_uuid}" '{"token_exchange_enabled":true}' >/dev/null
    # The API turns the user's token into one addressed to the agent...
    ensure_policy "${API_CLIENT_UUID}" "${agent}" true false
    # ...which the agent exchanges, as itself, for a delegated MCP token.
    ensure_policy "${agent_uuid}" "${MCP_CLIENT_ID}" false true
done

# The role is declared above but granted here: the provider has no resource
# for the grant that converges with a user created by self-registration.
# Assigning a role the user already holds answers 500, hence the check first.
admin_user_id="$(admin GET "/users" \
    | jq -r --arg u "${ADMIN_USERNAME}" '(if type == "array" then . else .data end)[]? | select(.username == $u) | .id' \
    | head -1)"
if [ -n "${admin_user_id}" ]; then
    echo "👑 ${ADMIN_USERNAME} -> capitainerie-admin"
    if ! admin GET "/users/${admin_user_id}/roles" \
        | jq -e '(if type == "array" then . else .data end)[]? | select(.name == "capitainerie-admin")' >/dev/null; then
        admin POST "/users/${admin_user_id}/roles/${ADMIN_ROLE_UUID}" >/dev/null
    fi
else
    echo "ℹ️  no user ${ADMIN_USERNAME} yet: register it, then run this script again to grant capitainerie-admin"
fi

client_secret() {
    admin GET "/clients/$1/client-secret" | jq -r '.data.client_secret // .client_secret'
}
API_SECRET="$(client_secret "${API_CLIENT_UUID}")"
AGENT_CREDENTIALS=""
for agent in $(jq -r 'keys[]' <<<"${AGENT_CLIENTS}"); do
    secret="$(client_secret "$(jq -r --arg a "${agent}" '.[$a]' <<<"${AGENT_CLIENTS}")")"
    AGENT_CREDENTIALS="${AGENT_CREDENTIALS}${AGENT_CREDENTIALS:+,}${agent}=${secret}"
done

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
     VITE_OIDC_RESOURCE=${API_RESOURCE}

   API and worker (.env) — secrets are printed, not written: they belong in
   whatever holds your secrets, not in the working tree:
     EXCHANGE_CLIENT_ID=capitainerie-api
     EXCHANGE_CLIENT_SECRET=${API_SECRET}
     AGENT_CREDENTIALS=${AGENT_CREDENTIALS}
     DELEGATION_KEY=<openssl rand -base64 32, the same for the API and the worker>
     SECRETS_KEY=<openssl rand -base64 32, distinct from DELEGATION_KEY, the same for the API and the worker>

   Create your account from the console's login page ("Register"): FerrisKey
   has no admin API to set a password, so self-registration is the way in.
SUMMARY
