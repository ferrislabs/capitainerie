#!/bin/sh
set -eu

HTML_DIR=/usr/share/nginx/html
SRC_DIR=/usr/local/src/capitainerie

rm -rf -- "${HTML_DIR:?}/"* "${HTML_DIR:?}/".[!.]* "${HTML_DIR:?}/"..?* 2>/dev/null || true
cp -r "$SRC_DIR"/. "$HTML_DIR"

if [ -f "$SRC_DIR/config.json" ]; then
  API_URL="${API_URL:-}" OIDC_ISSUER_URL="${OIDC_ISSUER_URL:-}" OIDC_CLIENT_ID="${OIDC_CLIENT_ID:-}" OIDC_RESOURCE="${OIDC_RESOURCE:-}" TRACES_URL="${TRACES_URL:-}" \
    envsubst '${API_URL} ${OIDC_ISSUER_URL} ${OIDC_CLIENT_ID} ${OIDC_RESOURCE} ${TRACES_URL}' \
    < "$SRC_DIR/config.json" > "$HTML_DIR/config.json"
fi

exec "$@"
