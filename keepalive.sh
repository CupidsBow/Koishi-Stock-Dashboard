#!/usr/bin/env bash
set -e

# ── Configuration ──────────────────────────────────────────────────────────
BACKEND_DIR="stock-backend"
FRONTEND_DIR="frontend"
BACKEND_PORT=3000
FRONTEND_PORT=5173

BACKEND_HEALTH="http://localhost:${BACKEND_PORT}/api/health"
FRONTEND_HEALTH="http://localhost:${FRONTEND_PORT}"

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# ── Log directory (project-root /log) ─────────────────────────────────────
LOG_DIR="$SCRIPT_DIR/log"
mkdir -p "$LOG_DIR"

# ── Helpers ────────────────────────────────────────────────────────────────

log()  { echo -e "${BLUE}[$(date +%H:%M:%S)]${NC} $*"; }
ok()   { echo -e "${GREEN}[$(date +%H:%M:%S)] ✓${NC} $*"; }
warn() { echo -e "${YELLOW}[$(date +%H:%M:%S)] ⚠${NC} $*"; }
err()  { echo -e "${RED}[$(date +%H:%M:%S)] ✗${NC} $*"; }

port_listening() {
  curl -s --max-time 5 "$1" >/dev/null 2>&1
}

start_backend() {
  log "Starting backend..."
  cd "$SCRIPT_DIR/$BACKEND_DIR"
  nohup cargo run --release >> "$LOG_DIR/backend.log" 2>&1 &
  echo $! > "$LOG_DIR/backend.pid"
  cd "$SCRIPT_DIR"
}

start_frontend() {
  log "Starting frontend dev server..."
  cd "$SCRIPT_DIR/$FRONTEND_DIR"
  nohup npm run dev >> "$LOG_DIR/frontend.log" 2>&1 &
  echo $! > "$LOG_DIR/frontend.pid"
  cd "$SCRIPT_DIR"
}

# ── Main loop ──────────────────────────────────────────────────────────────

CHECK_INTERVAL="${1:-10}"

log "Keepalive started — checking every ${CHECK_INTERVAL}s"
log "Logs: $LOG_DIR/"
log "Backend health:  ${BACKEND_HEALTH}"
log "Frontend health: ${FRONTEND_HEALTH}"
echo ""

while true; do
  # ── Check backend ──────────────────────────────────────────────────────
  if port_listening "$BACKEND_HEALTH"; then
    ok "Backend is healthy"
  else
    warn "Backend is DOWN — attempting restart..."
    if [ -f "$LOG_DIR/backend.pid" ]; then
      kill "$(cat "$LOG_DIR/backend.pid")" 2>/dev/null || true
      rm -f "$LOG_DIR/backend.pid"
    fi
    fuser -k "${BACKEND_PORT}/tcp" 2>/dev/null || true
    sleep 1
    start_backend
    for i in $(seq 1 15); do
      sleep 2
      if port_listening "$BACKEND_HEALTH"; then
        ok "Backend restarted successfully"
        break
      fi
    done
    if ! port_listening "$BACKEND_HEALTH"; then
      err "Backend failed to restart — check $LOG_DIR/backend.log"
    fi
  fi

  # ── Check frontend ─────────────────────────────────────────────────────
  if port_listening "$FRONTEND_HEALTH"; then
    ok "Frontend is healthy"
  else
    warn "Frontend is DOWN — attempting restart..."
    if [ -f "$LOG_DIR/frontend.pid" ]; then
      kill "$(cat "$LOG_DIR/frontend.pid")" 2>/dev/null || true
      rm -f "$LOG_DIR/frontend.pid"
    fi
    fuser -k "${FRONTEND_PORT}/tcp" 2>/dev/null || true
    sleep 1
    start_frontend
    for i in $(seq 1 10); do
      sleep 2
      if port_listening "$FRONTEND_HEALTH"; then
        ok "Frontend restarted successfully"
        break
      fi
    done
    if ! port_listening "$FRONTEND_HEALTH"; then
      err "Frontend failed to restart — check $LOG_DIR/frontend.log"
    fi
  fi

  echo ""
  sleep "$CHECK_INTERVAL"
done