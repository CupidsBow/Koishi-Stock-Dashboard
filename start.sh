#!/usr/bin/env bash
set -e

# ── Configuration ──────────────────────────────────────────────────────────
BACKEND_DIR="stock-backend"
FRONTEND_DIR="frontend"
BACKEND_PORT=3000

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

cleanup() {
  echo ""
  echo -e "${BLUE}Shutting down...${NC}"
  # Kill frontend dev server (Vite)
  if [ -n "$FRONTEND_PID" ] && kill -0 "$FRONTEND_PID" 2>/dev/null; then
    kill "$FRONTEND_PID" 2>/dev/null
    echo -e "${GREEN}✓${NC} Frontend stopped"
  fi
  # Kill backend (cargo run spawns child processes)
  if [ -n "$BACKEND_PID" ] && kill -0 "$BACKEND_PID" 2>/dev/null; then
    kill "$BACKEND_PID" 2>/dev/null
    echo -e "${GREEN}✓${NC} Backend stopped"
  fi
  exit 0
}

trap cleanup SIGINT SIGTERM

# ── Check prerequisites ────────────────────────────────────────────────────
command -v cargo >/dev/null 2>&1 || { echo -e "${RED}Error: cargo not found. Install Rust first.${NC}"; exit 1; }
command -v npm  >/dev/null 2>&1 || { echo -e "${RED}Error: npm not found. Install Node.js first.${NC}"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# ── Install frontend deps if needed ─────────────────────────────────────────
if [ ! -d "$FRONTEND_DIR/node_modules" ]; then
  echo -e "${BLUE}Installing frontend dependencies...${NC}"
  npm --prefix "$FRONTEND_DIR" install
fi

# ── Start backend ───────────────────────────────────────────────────────────
echo -e "${BLUE}Starting Rust backend on port ${BACKEND_PORT}...${NC}"
(cd "$BACKEND_DIR" && cargo run) &
BACKEND_PID=$!

# ── Wait for backend to be ready ────────────────────────────────────────────
echo -n "Waiting for backend"
for i in $(seq 1 30); do
  if curl -s "http://localhost:$BACKEND_PORT/api/search?keyword=000001" >/dev/null 2>&1; then
    echo ""
    echo -e "${GREEN}✓${NC} Backend is ready"
    break
  fi
  echo -n "."
  sleep 1
done
echo ""

# ── Start frontend dev server ───────────────────────────────────────────────
echo -e "${BLUE}Starting Vite frontend dev server...${NC}"
npm --prefix "$FRONTEND_DIR" run dev &
FRONTEND_PID=$!

echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  Stock Dashboard is starting up${NC}"
echo -e "${GREEN}  Backend:  http://localhost:${BACKEND_PORT}${NC}"
echo -e "${GREEN}  Frontend: http://localhost:5173${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "Press ${RED}Ctrl+C${NC} to stop both servers"
echo ""

# ── Wait for either process to exit ─────────────────────────────────────────
wait "$BACKEND_PID" "$FRONTEND_PID"