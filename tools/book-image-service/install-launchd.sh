#!/usr/bin/env bash
set -euo pipefail

APP_DIR="/Volumes/System/docker/book-image-service"
LABEL="com.coderdream.book-image-service"
PLIST_NAME="${LABEL}.plist"
AGENT_DIR="${HOME}/Library/LaunchAgents"
AGENT_PLIST="${AGENT_DIR}/${PLIST_NAME}"
UID_VALUE="$(id -u)"

cd "${APP_DIR}"
chmod +x start.sh
mkdir -p "${AGENT_DIR}"
cp "${PLIST_NAME}" "${AGENT_PLIST}"
plutil -lint "${AGENT_PLIST}"

launchctl bootout "gui/${UID_VALUE}/${LABEL}" >/dev/null 2>&1 || true
pkill -f "uvicorn app:app.*30019" >/dev/null 2>&1 || true
sleep 2

launchctl bootstrap "gui/${UID_VALUE}" "${AGENT_PLIST}"
launchctl enable "gui/${UID_VALUE}/${LABEL}"
sleep 8

launchctl print "gui/${UID_VALUE}/${LABEL}" | sed -n "1,120p"
echo "LISTEN"
lsof -nP -iTCP:30019 -sTCP:LISTEN || true
echo "HEALTH"
curl -sS http://127.0.0.1:30019/health || true
echo
echo "STDERR"
tail -80 service.err.log 2>/dev/null || true
echo "STDOUT"
tail -80 service.log 2>/dev/null || true
