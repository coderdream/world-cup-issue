#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${BOOK_IMAGE_SERVICE_DIR:-/Volumes/System/docker/book-image-service}"
PORT="${BOOK_IMAGE_PORT:-30019}"
HEALTH_URL="http://127.0.0.1:${PORT}/health"
LOG_FILE="${APP_DIR}/watchdog.log"

cd "${APP_DIR}"

if curl -fsS --max-time 5 "${HEALTH_URL}" >/dev/null 2>&1; then
  exit 0
fi

echo "$(date '+%Y-%m-%d %H:%M:%S') restarting book image service on port ${PORT}" >>"${LOG_FILE}"

if lsof -tiTCP:"${PORT}" -sTCP:LISTEN >/tmp/book-image-service-pids.txt 2>/dev/null; then
  while IFS= read -r pid; do
    [ -n "${pid}" ] && kill "${pid}" >/dev/null 2>&1 || true
  done </tmp/book-image-service-pids.txt
  sleep 2
fi

nohup "${APP_DIR}/start.sh" >>"${APP_DIR}/service.log" 2>>"${APP_DIR}/service.err.log" &
