#!/usr/bin/env bash
set -euo pipefail

APP_DIR="/Volumes/System/docker/book-image-service"
LABEL="com.coderdream.book-image-service"
CRON_MARKER="${APP_DIR}/watchdog.sh"
CRON_LINE="* * * * * ${APP_DIR}/watchdog.sh"
UID_VALUE="$(id -u)"

cd "${APP_DIR}"
chmod +x start.sh watchdog.sh

launchctl bootout "gui/${UID_VALUE}/${LABEL}" >/dev/null 2>&1 || true
launchctl disable "gui/${UID_VALUE}/${LABEL}" >/dev/null 2>&1 || true
rm -f "${HOME}/Library/LaunchAgents/${LABEL}.plist"

tmp_cron="$(mktemp)"
if crontab -l >"${tmp_cron}" 2>/dev/null; then
  grep -vF "${CRON_MARKER}" "${tmp_cron}" >"${tmp_cron}.new" || true
else
  : >"${tmp_cron}.new"
fi
echo "${CRON_LINE}" >>"${tmp_cron}.new"
crontab "${tmp_cron}.new"
rm -f "${tmp_cron}" "${tmp_cron}.new"

"${APP_DIR}/watchdog.sh"
sleep 5

echo "CRONTAB"
crontab -l | grep -F "${CRON_MARKER}" || true
echo "LISTEN"
lsof -nP -iTCP:30019 -sTCP:LISTEN || true
echo "HEALTH"
curl -sS http://127.0.0.1:30019/health || true
echo
echo "WATCHDOG LOG"
tail -40 "${APP_DIR}/watchdog.log" 2>/dev/null || true
