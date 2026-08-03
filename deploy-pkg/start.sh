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
