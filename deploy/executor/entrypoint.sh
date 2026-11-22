#!/usr/bin/env bash
set -euo pipefail

marker() {
  printf '{"type":"capitainerie","event":"%s"%s}\n' "$1" "${2:-}"
}

: "${GIT_REMOTE:?}" "${BASE_BRANCH:?}" "${BRANCH:?}" "${MODEL:?}" "${MAX_TURNS:?}" "${PROMPT:?}"
: "${CAPITAINERIE_TASK_TOKEN:?}"
harness="${CAPITAINERIE_HARNESS:-claude-code}"
case "$harness" in
  claude-code) ;;
  codex) : "${CAPITAINERIE_OPENAI_BASE_URL:?}" "${CODEX_API_KEY:?}" "${CODEX_HOME:?}" ;;
  *)
    echo "unknown CAPITAINERIE_HARNESS: $harness" >&2
    exit 2
    ;;
esac
mode="${CAPITAINERIE_MODE:-change}"
case "$mode" in
  change | audit) ;;
  *)
    echo "unknown CAPITAINERIE_MODE: $mode" >&2
    exit 2
    ;;
esac

REPORT_MAX_BYTES=1048576
REPORT_CHUNK_CHARS=4096
REPORT_NOTE=$'\n\n_[rapport tronque a 1 Mio]_\n'

# Sends the report on stdout as base64 chunks, then its SHA-256 and size;
# a report above REPORT_MAX_BYTES is cut and ends with a note.
emit_report() {
  local source="$1" sent truncated=false size keep
  [ -s "$source" ] || return 1
  sent="$(mktemp /tmp/report.XXXXXX)"
  size="$(wc -c <"$source" | tr -d ' ')"
  if [ "$size" -gt "$REPORT_MAX_BYTES" ]; then
    keep=$((REPORT_MAX_BYTES - $(printf '%s' "$REPORT_NOTE" | LC_ALL=C wc -c | tr -d ' ')))
    head -c "$keep" "$source" >"$sent"
    printf '%s' "$REPORT_NOTE" >>"$sent"
    truncated=true
  else
    cat "$source" >"$sent"
  fi
  base64 -w0 <"$sent" | fold -w "$REPORT_CHUNK_CHARS" | while IFS= read -r chunk || [ -n "$chunk" ]; do
    marker report_chunk ",\"data\":\"$chunk\""
  done
  marker report_end ",\"sha256\":\"$(sha256sum "$sent" | cut -d' ' -f1)\",\"bytes\":$(wc -c <"$sent" | tr -d ' '),\"truncated\":$truncated"
  rm -f "$sent"
}

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

write_codex_config() {
  mkdir -p "$CODEX_HOME"
  cat >"$CODEX_HOME/config.toml" <<CODEX_CONFIG
model_provider = "capitainerie"
web_search = "disabled"
check_for_update_on_startup = false

[model_providers.capitainerie]
name = "capitainerie"
base_url = "$CAPITAINERIE_OPENAI_BASE_URL"
env_key = "CODEX_API_KEY"
wire_api = "responses"

[features]
image_generation = false
api_key_model_discovery = false

[analytics]
enabled = false
CODEX_CONFIG
}

run_agent() {
  local run_mode="$1"
  case "$harness" in
    claude-code)
      if [ "$run_mode" = audit ]; then
        claude -p "$PROMPT" \
          --output-format stream-json \
          --verbose \
          --model "$MODEL" \
          --max-turns "$MAX_TURNS" \
          --allowedTools "Read,Glob,Grep,Bash,Write" \
          --disallowedTools "Edit,MultiEdit,NotebookEdit"
      else
        claude -p "$PROMPT" \
          --output-format stream-json \
          --verbose \
          --model "$MODEL" \
          --max-turns "$MAX_TURNS" \
          --allowedTools "Read,Edit,Write,Glob,Grep,Bash"
      fi
      ;;
    codex)
      codex exec --json --skip-git-repo-check --ephemeral \
        --dangerously-bypass-approvals-and-sandbox -m "$MODEL" -- "$PROMPT"
      ;;
  esac
}

cd repo
if [ "$harness" = claude-code ]; then
  export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
else
  write_codex_config
fi
agent_status=0

if [ "$mode" = audit ]; then
  : "${REPORT_PATH:?}"
  mkdir -p "$(dirname "$REPORT_PATH")"
  rm -f "$REPORT_PATH"
  # Read-only: no Edit, and nothing is ever committed or pushed (the proxy
  # refuses a push of this run anyway). Write is kept for the report only.
  run_agent audit || agent_status=$?
  echo "$harness exited with $agent_status" >&2
  emit_report "$REPORT_PATH" || echo "no report at $REPORT_PATH" >&2
  # The worker judges the audit from the report and the summary it read;
  # a stop of Claude Code is in its stream.
  marker audit_done
  exit 0
fi

git switch -c "$BRANCH" >&2
run_agent change || agent_status=$?

if [ -n "$(git status --porcelain)" ]; then
  first_line="${PROMPT%%$'\n'*}"
  git add -A
  git commit -q -m "capitainerie: ${first_line:0:72}" >&2
  git push origin "$BRANCH" >&2
  marker pushed ",\"sha\":\"$(git rev-parse HEAD)\""
else
  marker no_changes
fi

exit "$agent_status"
