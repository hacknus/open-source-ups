#!/bin/sh
set -eu

token_file="${CODEX_WS_TOKEN_FILE:-/workspace/.devcontainer/codex-ws-token}"
server_port="${CODEX_APP_SERVER_PORT:-4500}"

umask 077

if [ -n "${CODEX_WS_TOKEN:-}" ]; then
  printf '%s\n' "$CODEX_WS_TOKEN" > "$token_file"
elif [ ! -s "$token_file" ]; then
  openssl rand -hex 32 > "$token_file"
fi

chmod 600 "$token_file"

if ! codex login status >/dev/null 2>&1; then
  echo "Codex is not logged in. Run:" >&2
  echo "docker compose -f .devcontainer/compose.yaml run --rm app codex login --device-auth" >&2
  echo "Waiting for login before starting the app server..." >&2
fi

while ! codex login status >/dev/null 2>&1; do
  sleep 5
done

exec codex app-server \
  --listen "ws://0.0.0.0:${server_port}" \
  --ws-auth capability-token \
  --ws-token-file "$token_file"
