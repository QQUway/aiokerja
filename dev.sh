#!/usr/bin/env bash
# Local development without building the backend/frontend images.
# Postgres still runs in Docker (it needs the pgvector extension);
# the backend runs as a native cargo binary and the frontend as the Vite
# dev server, so both hot-reload.
#
#   ./dev.sh up      start postgres + backend + frontend
#   ./dev.sh down    stop backend + frontend (keeps postgres running)
#   ./dev.sh nuke    stop everything incl. postgres and drop the DB volume
#   ./dev.sh status  show what is up
#   ./dev.sh logs    tail backend + frontend logs
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUN_DIR="${TMPDIR:-/tmp}/workassistant-dev"
mkdir -p "$RUN_DIR"

# Native backend must reach postgres on the host port, not the compose
# service name. The root .env is picked up by dotenvy, so override it here.
export DATABASE_URL="${DEV_DATABASE_URL:-postgres://user:password@localhost:5432/workassistant}"
BACKEND_PORT="${BACKEND_PORT:-8080}"
FRONTEND_PORT="${FRONTEND_PORT:-5173}"

pidfile() { echo "$RUN_DIR/$1.pid"; }

stop_one() {
  local name="$1" file
  file="$(pidfile "$name")"
  if [[ -f "$file" ]]; then
    local pid
    pid="$(cat "$file")"
    if kill -0 "$pid" 2>/dev/null; then
      # Kill the whole process group so npm/cargo children go too.
      kill -- "-$pid" 2>/dev/null || kill "$pid" 2>/dev/null || true
      echo "stopped $name (pid $pid)"
    fi
    rm -f "$file"
  fi
}

start_backend() {
  if [[ -f "$(pidfile backend)" ]] && kill -0 "$(cat "$(pidfile backend)")" 2>/dev/null; then
    echo "backend already running"
    return
  fi
  echo "building backend (debug)…"
  ( cd "$ROOT/backend" && cargo build )
  echo "starting backend on :$BACKEND_PORT"
  ( cd "$ROOT/backend" && setsid nohup ./target/debug/work-assistant-backend \
      >"$RUN_DIR/backend.log" 2>&1 < /dev/null & echo $! >"$(pidfile backend)" )
}

start_frontend() {
  if [[ -f "$(pidfile frontend)" ]] && kill -0 "$(cat "$(pidfile frontend)")" 2>/dev/null; then
    echo "frontend already running"
    return
  fi
  [[ -d "$ROOT/frontend/node_modules" ]] || ( cd "$ROOT/frontend" && npm install )
  echo "starting frontend on :$FRONTEND_PORT"
  ( cd "$ROOT/frontend" && setsid nohup npm run dev \
      >"$RUN_DIR/frontend.log" 2>&1 < /dev/null & echo $! >"$(pidfile frontend)" )
}

wait_http() {
  local url="$1" tries="${2:-30}"
  for ((i = 0; i < tries; i++)); do
    if curl -fsS -m 2 -o /dev/null "$url" 2>/dev/null; then return 0; fi
    sleep 1
  done
  return 1
}

cmd_up() {
  echo "starting postgres container…"
  ( cd "$ROOT" && docker compose up -d postgres >/dev/null )
  wait_http "http://127.0.0.1:$BACKEND_PORT/api/health" 1 || true
  start_backend
  start_frontend

  if wait_http "http://127.0.0.1:$BACKEND_PORT/api/health" 40; then
    echo "backend  ok  http://127.0.0.1:$BACKEND_PORT/api/health"
  else
    echo "backend  FAILED — see $RUN_DIR/backend.log"
  fi
  if wait_http "http://localhost:$FRONTEND_PORT/" 40; then
    echo "frontend ok  http://localhost:$FRONTEND_PORT/"
  else
    echo "frontend FAILED — see $RUN_DIR/frontend.log"
  fi
  echo
  echo "open http://localhost:$FRONTEND_PORT/"
  echo "logs: ./dev.sh logs    stop: ./dev.sh down"
}

cmd_down() {
  stop_one frontend
  stop_one backend
}

cmd_nuke() {
  cmd_down
  echo "stopping postgres and dropping volume…"
  ( cd "$ROOT" && docker compose down -v >/dev/null )
}

cmd_status() {
  local code
  code="$(curl -s -m 2 -o /dev/null -w '%{http_code}' "http://127.0.0.1:$BACKEND_PORT/api/health" || true)"
  echo "backend  :$BACKEND_PORT  ${code:-down}"
  code="$(curl -s -m 2 -o /dev/null -w '%{http_code}' "http://localhost:$FRONTEND_PORT/" || true)"
  echo "frontend :$FRONTEND_PORT  ${code:-down}"
  ( cd "$ROOT" && docker compose ps postgres --format 'postgres {{.Status}}' )
}

cmd_logs() {
  echo "== backend ($RUN_DIR/backend.log) =="
  tail -n 40 "$RUN_DIR/backend.log" 2>/dev/null || echo "(no log)"
  echo
  echo "== frontend ($RUN_DIR/frontend.log) =="
  tail -n 20 "$RUN_DIR/frontend.log" 2>/dev/null || echo "(no log)"
}

case "${1:-up}" in
  up) cmd_up ;;
  down) cmd_down ;;
  nuke) cmd_nuke ;;
  status) cmd_status ;;
  logs) cmd_logs ;;
  *)
    echo "usage: ./dev.sh [up|down|nuke|status|logs]" >&2
    exit 2
    ;;
esac
