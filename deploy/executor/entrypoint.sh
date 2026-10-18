#!/usr/bin/env bash
set -euo pipefail

marker() {
  printf '{"type":"capitainerie","event":"%s"%s}\n' "$1" "${2:-}"
}

: "${GIT_REMOTE:?}" "${BASE_BRANCH:?}" "${BRANCH:?}" "${MODEL:?}" "${MAX_TURNS:?}" "${PROMPT:?}"
: "${CAPITAINERIE_TASK_TOKEN:?}"

askpass="$(mktemp /tmp/askpass.XXXXXX)"
trap 'rm -f "$askpass"' EXIT
cat >"$askpass" <<'ASKPASS'
#!/usr/bin/env bash
case "$1" in
  Username*) printf '%s\n' "x-access-token" ;;
  *) printf '%s\n' "$CAPITAINERIE_TASK_TOKEN" ;;
esac
ASKPASS
chmod 0700 "$askpass"
export GIT_ASKPASS="$askpass"
export GIT_TERMINAL_PROMPT=0

git config --global user.name "Capitainerie Executor"
git config --global user.email "noreply@capitainerie.dev"

git clone --depth 50 --branch "$BASE_BRANCH" "$GIT_REMOTE" repo >&2
marker cloned

cd repo
git switch -c "$BRANCH" >&2

export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
claude_status=0
claude -p "$PROMPT" \
  --output-format stream-json \
  --verbose \
  --model "$MODEL" \
  --max-turns "$MAX_TURNS" \
  --allowedTools "Read,Edit,Write,Glob,Grep,Bash" || claude_status=$?

if [ -n "$(git status --porcelain)" ]; then
  first_line="${PROMPT%%$'\n'*}"
  git add -A
  git commit -q -m "capitainerie: ${first_line:0:72}" >&2
  git push origin "$BRANCH" >&2
  marker pushed ",\"sha\":\"$(git rev-parse HEAD)\""
else
  marker no_changes
fi

exit "$claude_status"
