#!/usr/bin/env bash
set -e

# ── Config ────────────────────────────────────────────────────────────────
PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
BACKEND_DIR="$PROJECT_ROOT/stock-backend"
FRONTEND_DIR="$PROJECT_ROOT/frontend"
OUTPUT_DIR="$PROJECT_ROOT/deploy-pkg"
ARCHIVE_NAME="stock-dashboard.tar.gz"

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log()  { echo -e "${BLUE}[$(date +%H:%M:%S)]${NC} $*"; }
ok()   { echo -e "${GREEN}[$(date +%H:%M:%S)] ✓${NC} $*"; }
err()  { echo -e "${RED}[$(date +%H:%M:%S)] ✗${NC} $*"; }

# ── Step 1: Build backend ─────────────────────────────────────────────────
log "Building Rust backend (release)..."
cd "$BACKEND_DIR"
cargo build --release
ok "Backend built"

# ── Step 2: Build frontend ────────────────────────────────────────────────
log "Building frontend..."
cd "$FRONTEND_DIR"
if [ ! -d "node_modules" ]; then
  log "Installing frontend dependencies..."
  npm ci
fi
npm run build
ok "Frontend built"

# ── Step 3: Assemble deploy package ───────────────────────────────────────
cd "$PROJECT_ROOT"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

cp "$BACKEND_DIR/target/release/stock-backend" "$OUTPUT_DIR/"
cp -r "$FRONTEND_DIR/dist" "$OUTPUT_DIR/"
cp "$BACKEND_DIR/.env" "$OUTPUT_DIR/.env.example"

cat > "$OUTPUT_DIR/keepalive.sh" << 'KEEPALIVE'
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
KEEPALIVE
chmod +x "$OUTPUT_DIR/keepalive.sh"

cat > "$OUTPUT_DIR/start.sh" << 'SCRIPT'
#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

if [ ! -f .env ]; then
  echo "[ERROR] 请先从 .env.example 复制 .env 并填写配置"
  exit 1
fi

export STATIC_DIR=dist
echo "Starting stock-dashboard on port 3000..."
exec ./stock-backend
SCRIPT
chmod +x "$OUTPUT_DIR/start.sh"

ok "Deploy package assembled: $OUTPUT_DIR"

# ── Step 4: Create tarball ────────────────────────────────────────────────
log "Creating archive..."
cd "$PROJECT_ROOT"
tar czf "$ARCHIVE_NAME" --transform 's,^,stock-dashboard/,' deploy-pkg/
ok "Archive created: $PROJECT_ROOT/$ARCHIVE_NAME ($(du -h "$ARCHIVE_NAME" | cut -f1))"

# ── Summary ───────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  Build complete!${NC}"
echo ""
echo "  Archive: $PROJECT_ROOT/$ARCHIVE_NAME"
echo ""
echo -e "${BLUE}  Deploy to target server:${NC}"
echo ""
echo "    scp $ARCHIVE_NAME user@server:/opt/"
echo "    ssh user@server"
echo "    cd /opt && tar xzf $ARCHIVE_NAME"
echo "    cd stock-dashboard/deploy-pkg"
echo "    cp .env.example .env  # edit as needed"
echo "    ./start.sh"
echo -e "${GREEN}════════════════════════════════════════════════${NC}"