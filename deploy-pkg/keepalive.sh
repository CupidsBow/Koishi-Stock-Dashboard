#!/usr/bin/env bash
set -e

PORT=3000
HEALTH_URL="http://localhost:${PORT}/api/health"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$SCRIPT_DIR/log"
mkdir -p "$LOG_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'
log()  { echo -e "${BLUE}[$(date +%H:%M:%S)]${NC} $*"; }
ok()   { echo -e "${GREEN}[$(date +%H:%M:%S)] ✓${NC} $*"; }
warn() { echo -e "${YELLOW}[$(date +%H:%M:%S)] ⚠${NC} $*"; }
err()  { echo -e "${RED}[$(date +%H:%M:%S)] ✗${NC} $*"; }

is_alive() {
  curl -s --max-time 5 "$HEALTH_URL" >/dev/null 2>&1
}

CHECK_INTERVAL="${1:-10}"
log "Keepalive started — checking every ${CHECK_INTERVAL}s on port ${PORT}"
echo ""

while true; do
  if is_alive; then
    ok "Backend healthy"
  else
    warn "Backend DOWN — restarting..."
    fuser -k "${PORT}/tcp" 2>/dev/null || true
    sleep 1
    cd "$SCRIPT_DIR"
    export STATIC_DIR=dist
    nohup ./stock-backend >> "$LOG_DIR/app.log" 2>&1 &
    for i in $(seq 1 15); do
      sleep 2
      is_alive && break
    done
    is_alive && ok "Restarted" || err "Failed — see $LOG_DIR/app.log"
  fi
  echo ""
  sleep "$CHECK_INTERVAL"
done
